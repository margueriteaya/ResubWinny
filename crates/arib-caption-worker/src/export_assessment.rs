use crate::{
    CaptionFeatureSummary, ConversionOptions, TtmlCaption, native_b24, ttml_drcs_mapping_key,
};
use serde::Serialize;
use std::{fmt, io};

const CAPABILITIES: &str = include_str!("../../../shared/format_capabilities.json");

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportConflict {
    pub(crate) issue_code: String,
    pub(crate) formats: Vec<String>,
    pub(crate) feature: String,
    pub(crate) logical_track: String,
    pub(crate) available_actions: Vec<String>,
}

impl fmt::Display for ExportConflict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "selected formats cannot preserve {}",
            self.feature
        )
    }
}

impl std::error::Error for ExportConflict {}

fn selected_formats(options: &ConversionOptions) -> Vec<&'static str> {
    let mut formats = Vec::new();
    if options.keep_ass {
        formats.push("ASS");
    }
    if options.ttml {
        formats.push("TTML");
    }
    if options.srt {
        formats.push("SRT");
    }
    if options.webvtt {
        formats.push("WebVTT");
    }
    if options.archive {
        formats.push("JSON");
    }
    if options.raw {
        formats.push("Raw Data");
    }
    formats
}

fn unsupported_formats(options: &ConversionOptions, feature: &str) -> Vec<String> {
    let capabilities: serde_json::Value = serde_json::from_str(CAPABILITIES)
        .expect("shared format capability contract must be valid JSON");
    selected_formats(options)
        .into_iter()
        .filter(|format| capabilities[*format][feature] == "unsupported")
        .map(str::to_owned)
        .collect()
}

fn export_conflict(
    formats: Vec<String>,
    feature: &str,
    issue_code: &str,
    offer_drcs_mapping: bool,
    offer_compatible_format: bool,
) -> io::Result<()> {
    if formats.is_empty() {
        return Ok(());
    }
    let mut available_actions = vec![
        format!("disable_preservation:{feature}"),
        "remove_format".into(),
    ];
    if offer_compatible_format {
        available_actions.push("choose_compatible_format".into());
    }
    if feature == "drcs" && offer_drcs_mapping {
        available_actions.insert(0, "open_drcs_mapping".into());
    }
    Err(io::Error::other(ExportConflict {
        issue_code: issue_code.into(),
        formats,
        feature: feature.into(),
        logical_track: std::env::var("RESUBWINNY_LOGICAL_TRACK")
            .unwrap_or_else(|_| "logical-track:default".into()),
        available_actions,
    }))
}

fn conflict(options: &ConversionOptions, feature: &str) -> io::Result<()> {
    let formats = unsupported_formats(options, feature);
    export_conflict(
        formats,
        feature,
        "format_cannot_preserve_feature",
        false,
        true,
    )
}

pub(crate) fn assess_b24_scene(
    options: &ConversionOptions,
    scene: &native_b24::CaptionScene,
) -> io::Result<()> {
    let mut facts = CaptionFeatureSummary::default();
    facts.observe_b24_scene(scene);
    assess_facts(options, &facts)?;
    if options.preserve_drcs {
        let unresolved = scene.regions.iter().any(|region| {
            let start = region.first_character as usize;
            let end = start.saturating_add(region.character_count as usize);
            scene.characters.get(start..end).is_some_and(|characters| {
                characters.iter().any(|character| {
                    character.kind == 1
                        && character.drcs_code != 0
                        && character.utf8.is_empty()
                        && (options.drcs_mode != crate::DrcsMode::UseUserMapping
                            || options
                                .drcs_replacements
                                .get(&character.drcs_code)
                                .is_none_or(String::is_empty))
                        && scene
                            .drcs_glyphs
                            .iter()
                            .find(|glyph| glyph.drcs_code == character.drcs_code)
                            .is_none_or(|glyph| glyph.alternative_text.is_empty())
                })
            })
        });
        if unresolved {
            let formats = selected_formats(options)
                .into_iter()
                .filter(|format| matches!(*format, "SRT" | "WebVTT"))
                .map(str::to_owned)
                .collect();
            export_conflict(formats, "drcs", "unresolved_drcs_text_target", true, true)?;
        }
    }
    Ok(())
}

pub(crate) fn assess_ttml_caption(
    options: &ConversionOptions,
    caption: &TtmlCaption,
) -> io::Result<()> {
    let mut facts = CaptionFeatureSummary::default();
    facts.observe_ttml(caption);
    assess_facts(options, &facts)?;
    if options.preserve_drcs && !caption.drcs_uses.is_empty() {
        let unresolved = caption.drcs_uses.iter().filter(|drcs_use| {
            options.drcs_mode != crate::DrcsMode::UseUserMapping
                || ttml_drcs_mapping_key(
                    caption.source.as_ref(),
                    drcs_use.resource_index,
                    drcs_use.source_codepoint,
                )
                .as_ref()
                .and_then(|key| options.ttml_drcs_replacements.get(key))
                .is_none_or(String::is_empty)
        });
        let unresolved = unresolved.collect::<Vec<_>>();
        if unresolved.is_empty() {
            return Ok(());
        }
        let formats = selected_formats(options)
            .into_iter()
            .filter(|format| matches!(*format, "ASS" | "TTML" | "SRT" | "WebVTT"))
            .map(str::to_owned)
            .collect();
        // The scoped mapping contract is usable by the CLI and by mappings
        // already persisted by Studio. Do not advertise the dictionary action
        // until the B62 report path can surface these identities and glyphs.
        export_conflict(formats, "drcs", "unresolved_drcs_text_target", false, false)?;
    }
    Ok(())
}

fn assess_facts(options: &ConversionOptions, facts: &CaptionFeatureSummary) -> io::Result<()> {
    for (feature, present, preserve) in [
        ("position", facts.position, options.preserve_position),
        ("color", facts.color, options.preserve_color),
        ("ruby", facts.ruby, options.preserve_ruby),
    ] {
        if present && preserve {
            conflict(options, feature)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attach_b62_resource(caption: &mut TtmlCaption, bytes: &[u8]) -> String {
        let digest = crate::resource::resource_sha256(bytes);
        caption.source = Some(crate::TtmlCaptionSource {
            route: "test",
            source_offset: 0,
            mmpt_packet_id: 1,
            mpu_sequence_number: Some(1),
            mmtp_sequence_number: None,
            presentation_ntp: None,
            normalized_pts: None,
            reference_start_pts: None,
            reference_start_ntp: None,
            reference_start_time_leap_indicator: None,
            timeline_basis: crate::TlvTimelineBasis::MptPresentationNtp,
            track_id: None,
            component_tag: None,
            timing_mode: None,
            operation_mode: None,
            display_mode: None,
            compression_type: None,
            random_access: false,
            discontinuity: false,
            discontinuity_reasons: 0,
            xml_encoding: "UTF-8".into(),
            resources: vec![crate::TtmlResourceMetadata {
                index: 9,
                data_type: 1,
                byte_length: bytes.len(),
                content_sha256: digest.clone(),
                format_hint: Some("woff2"),
                format_validation: "header-validated",
                width: None,
                height: None,
                preview_available: false,
            }],
            resources_complete: true,
        });
        crate::resource::b62_drcs_mapping_key(&digest, 0xe000)
    }

    fn unresolved_drcs_scene() -> native_b24::CaptionScene {
        native_b24::CaptionScene {
            pts_ms: 0,
            wait_duration_ms: 1_000,
            plane_width: 960,
            plane_height: 540,
            regions: vec![native_b24::CaptionRegion {
                x: 0,
                y: 0,
                width: 960,
                height: 540,
                is_ruby: false,
                first_character: 0,
                character_count: 1,
            }],
            characters: vec![native_b24::CaptionCharacter {
                kind: 1,
                codepoint: 0,
                pua_codepoint: 0,
                drcs_code: 7,
                x: 0,
                y: 0,
                width: 36,
                height: 36,
                horizontal_spacing: 0,
                vertical_spacing: 0,
                horizontal_scale: 1.0,
                vertical_scale: 1.0,
                text_color: 0xffff_ffff,
                back_color: 0,
                stroke_color: 0,
                style: 0,
                enclosure_style: 0,
                utf8: String::new(),
            }],
            drcs_glyphs: Vec::new(),
            rendered_image: None,
        }
    }

    #[test]
    fn webvtt_position_requires_an_explicit_drop_with_the_current_text_exporter() {
        let facts = CaptionFeatureSummary {
            position: true,
            ..Default::default()
        };
        let mut options = ConversionOptions {
            webvtt: true,
            ..Default::default()
        };
        let error = assess_facts(&options, &facts).unwrap_err();
        let conflict = error
            .get_ref()
            .unwrap()
            .downcast_ref::<ExportConflict>()
            .unwrap();
        assert_eq!(conflict.formats, ["WebVTT"]);
        assert_eq!(conflict.feature, "position");
        options.preserve_position = false;
        assert!(assess_facts(&options, &facts).is_ok());
        options.preserve_position = true;
        assert!(assess_facts(&options, &CaptionFeatureSummary::default()).is_ok());
    }

    #[test]
    fn srt_ruby_is_a_conflict_but_ass_approximation_is_not() {
        let facts = CaptionFeatureSummary {
            ruby: true,
            ..Default::default()
        };
        let mut options = ConversionOptions::default();
        assert!(assess_facts(&options, &facts).is_ok());
        options.srt = true;
        assert!(assess_facts(&options, &facts).is_err());
    }

    #[test]
    fn user_selected_drop_is_not_a_conflict() {
        let facts = CaptionFeatureSummary {
            ruby: true,
            ..Default::default()
        };
        let options = ConversionOptions {
            srt: true,
            preserve_ruby: false,
            ..Default::default()
        };
        assert!(assess_facts(&options, &facts).is_ok());
    }

    #[test]
    fn unresolved_drcs_conflicts_only_with_selected_text_targets() {
        let scene = unresolved_drcs_scene();
        let mut options = ConversionOptions::default();
        assert!(assess_b24_scene(&options, &scene).is_ok());

        options.webvtt = true;
        let error = assess_b24_scene(&options, &scene).unwrap_err();
        let conflict = error
            .get_ref()
            .and_then(|error| error.downcast_ref::<ExportConflict>())
            .unwrap();
        assert_eq!(conflict.issue_code, "unresolved_drcs_text_target");
        assert_eq!(conflict.formats, ["WebVTT"]);
        assert_eq!(conflict.available_actions[0], "open_drcs_mapping");

        options.srt = true;
        options.ttml = true;
        options.archive = true;
        options.raw = true;
        let error = assess_b24_scene(&options, &scene).unwrap_err();
        let conflict = error
            .get_ref()
            .and_then(|error| error.downcast_ref::<ExportConflict>())
            .unwrap();
        assert_eq!(conflict.formats, ["SRT", "WebVTT"]);

        options.srt = false;
        options.webvtt = false;
        options.keep_ass = false;
        assert!(assess_b24_scene(&options, &scene).is_ok());
    }

    #[test]
    fn mapped_or_disabled_drcs_is_not_a_text_target_conflict() {
        let scene = unresolved_drcs_scene();
        let mut options = ConversionOptions {
            srt: true,
            webvtt: true,
            preserve_drcs: false,
            ..Default::default()
        };
        assert!(assess_b24_scene(&options, &scene).is_ok());

        options.preserve_drcs = true;
        options.drcs_mode = crate::DrcsMode::UseUserMapping;
        options.drcs_replacements.insert(7, "字".into());
        assert!(assess_b24_scene(&options, &scene).is_ok());
    }

    #[test]
    fn b62_drcs_conflicts_only_for_resource_backed_unmapped_characters() {
        let ordinary = crate::parse_ttml_captions(
            r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body><p begin='0s' end='1s' arib-tt:font-face='subt://9'>字</p></body></tt>"#,
            0,
        )
        .remove(0);
        let mut unresolved = crate::parse_ttml_captions(
            r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body><p begin='0s' end='1s' arib-tt:font-face='subt://9'>&#xE000;</p></body></tt>"#,
            0,
        )
        .remove(0);
        let mapping_key = attach_b62_resource(&mut unresolved, b"font-resource-a");
        let mut options = ConversionOptions {
            srt: true,
            webvtt: true,
            ttml: true,
            archive: true,
            raw: true,
            preserve_position: false,
            preserve_color: false,
            preserve_ruby: false,
            ..Default::default()
        };

        assert!(assess_ttml_caption(&options, &ordinary).is_ok());
        let error = assess_ttml_caption(&options, &unresolved).unwrap_err();
        let conflict = error
            .get_ref()
            .and_then(|error| error.downcast_ref::<ExportConflict>())
            .unwrap();
        assert_eq!(conflict.issue_code, "unresolved_drcs_text_target");
        assert_eq!(conflict.formats, ["ASS", "TTML", "SRT", "WebVTT"]);
        assert!(
            !conflict
                .available_actions
                .iter()
                .any(|action| action == "open_drcs_mapping")
        );
        assert!(
            !conflict
                .available_actions
                .iter()
                .any(|action| action == "choose_compatible_format")
        );

        options.drcs_mode = crate::DrcsMode::UseUserMapping;
        options.drcs_replacements.insert(0xe000, "字".into());
        assert!(assess_ttml_caption(&options, &unresolved).is_err());
        options
            .ttml_drcs_replacements
            .insert(mapping_key, "映".into());
        assert!(assess_ttml_caption(&options, &unresolved).is_ok());

        let mut same_codepoint_other_resource = unresolved.clone();
        attach_b62_resource(&mut same_codepoint_other_resource, b"font-resource-b");
        assert!(assess_ttml_caption(&options, &same_codepoint_other_resource).is_err());

        options.preserve_drcs = false;
        options.drcs_replacements.clear();
        assert!(assess_ttml_caption(&options, &unresolved).is_ok());
    }
}
