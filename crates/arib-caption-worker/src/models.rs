use serde::Serialize;

use crate::ARIB_TTML_NAMESPACE;
use crate::caption::ruby::TtmlRubyBinding;
use crate::native_b24;

use crate::resource::TtmlResourceMetadata;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InputKind {
    MpegTs,
    M2ts,
    Tlv,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct InputProbe {
    pub(crate) kind: InputKind,
    pub(crate) packet_size: Option<usize>,
    pub(crate) sync_offset: Option<usize>,
    pub(crate) confidence: usize,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) struct B24Track {
    pub(crate) service_id: u16,
    pub(crate) pmt_pid: u16,
    pub(crate) caption_pid: u16,
    pub(crate) component_tag: u8,
    pub(crate) caption_pids: Vec<u16>,
    pub(crate) language: Option<String>,
    pub(crate) service_name: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct DataTracks {
    pub(crate) pmt_pid: u16,
    pub(crate) pids: Vec<u16>,
    pub(crate) caption_pids: Vec<u16>,
    pub(crate) superimpose_pids: Vec<u16>,
}

impl DataTracks {
    pub(crate) fn retain_default_caption_tracks(&mut self) {
        if !self.caption_pids.is_empty() {
            self.pids.retain(|pid| self.caption_pids.contains(pid));
        }
    }

    pub(crate) fn component_kind(&self, pid: u16) -> &'static str {
        if self.caption_pids.contains(&pid) {
            "caption"
        } else if self.superimpose_pids.contains(&pid) {
            "superimpose"
        } else {
            "candidate"
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct CaptionTrackInspection {
    pub(crate) label: String,
    pub(crate) detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) track_id: Option<u16>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct InputInspection {
    pub(crate) bytes: u64,
    pub(crate) container: String,
    pub(crate) route_code: &'static str,
    pub(crate) route: String,
    pub(crate) service: String,
    pub(crate) tracks: Vec<CaptionTrackInspection>,
    pub(crate) broadcast: BroadcastMetadata,
}

#[derive(Debug, Default, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct BroadcastMetadata {
    pub(crate) network_name: Option<String>,
    pub(crate) programme_name: Option<String>,
    pub(crate) programme_description: Option<String>,
    pub(crate) broadcast_time_utc: Option<String>,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct B24DecodeSummary {
    pub bytes_read: u64,
    pub pes_packets: u64,
    pub captions: u64,
    pub regions: u64,
    pub characters: u64,
    pub drcs_glyphs: u64,
    pub decoder_errors: u64,
    #[serde(default)]
    pub features: CaptionFeatureSummary,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct CaptionFeatureSummary {
    pub ruby: bool,
    pub drcs: bool,
    pub position: bool,
    pub color: bool,
    pub gaiji: bool,
    pub accessibility: bool,
    #[serde(default)]
    pub observed_counts: std::collections::BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub feature_details: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub complete: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureState {
    #[default]
    Unknown,
    Present,
    Absent,
}

impl CaptionFeatureSummary {
    pub fn state(&self, feature: &str) -> FeatureState {
        let present = match feature {
            "ruby" => self.ruby,
            "drcs" => self.drcs,
            "position" => self.position,
            "color" => self.color,
            "gaiji" => self.gaiji,
            "accessibility" => self.accessibility,
            _ => false,
        };
        if present {
            FeatureState::Present
        } else if self.complete {
            FeatureState::Absent
        } else {
            FeatureState::Unknown
        }
    }

    fn mark(&mut self, feature: &str, present: bool) {
        if present {
            *self.observed_counts.entry(feature.to_string()).or_default() += 1;
        }
    }

    fn mark_count(&mut self, feature: &str, count: usize) {
        if count > 0 {
            *self.observed_counts.entry(feature.to_string()).or_default() += count as u64;
        }
    }

    fn mark_detail_flag(&mut self, feature: &str, detail: &str, present: bool) {
        if !present {
            return;
        }
        let details = self
            .feature_details
            .entry(feature.to_owned())
            .or_insert_with(|| serde_json::json!({}));
        details
            .as_object_mut()
            .expect("feature details are always JSON objects")
            .insert(detail.to_owned(), serde_json::Value::Bool(true));
    }

    pub(crate) fn details(&self, feature: &str) -> Option<&serde_json::Value> {
        self.feature_details.get(feature)
    }
}

impl CaptionFeatureSummary {
    fn observe_accessibility(&mut self, evidence: &crate::caption_features::AccessibilityEvidence) {
        self.observe_accessibility_excluding(evidence, &[]);
    }

    fn observe_accessibility_excluding(
        &mut self,
        evidence: &crate::caption_features::AccessibilityEvidence,
        excluded_ranges: &[std::ops::Range<usize>],
    ) {
        if evidence.ranges.is_empty() {
            return;
        }
        self.accessibility = true;
        let count = evidence
            .cue_ranges
            .iter()
            .filter(|ranges| {
                !ranges.iter().any(|range| {
                    excluded_ranges
                        .iter()
                        .any(|excluded| range.start < excluded.end && excluded.start < range.end)
                })
            })
            .count();
        self.mark_count("accessibility", count);
        self.mark_detail_flag("accessibility", "textCue", true);
        self.mark_detail_flag(
            "accessibility",
            "leadingAnnotation",
            evidence.leading_annotation,
        );
        self.mark_detail_flag("accessibility", "musicCue", evidence.music_cue);
        self.mark_detail_flag(
            "accessibility",
            "narrationDelimiter",
            evidence.narration_delimiter,
        );
    }

    pub(crate) fn observe_b24_scene(&mut self, scene: &native_b24::CaptionScene) {
        let multiple_regions = scene.regions.len() > 1;
        let explicit_geometry = scene.regions.iter().any(|region| {
            region.x != 0
                || region.y != 0
                || region.width != scene.plane_width
                || region.height != scene.plane_height
        });
        if multiple_regions || explicit_geometry {
            self.position = true;
            self.mark("position", true);
            self.mark_detail_flag("position", "multipleRegions", multiple_regions);
            self.mark_detail_flag("position", "explicitGeometry", explicit_geometry);
        }
        let foreground = scene
            .characters
            .iter()
            .any(|character| character.text_color & 0x00ff_ffff != 0x00ff_ffff);
        let background = scene
            .characters
            .iter()
            .any(|character| character.back_color & 0x00ff_ffff != 0);
        let stroke = scene
            .characters
            .iter()
            .any(|character| character.stroke_color & 0x00ff_ffff != 0);
        if foreground || background || stroke {
            self.color = true;
            self.mark("color", true);
            self.mark_detail_flag("color", "foreground", foreground);
            self.mark_detail_flag("color", "background", background);
            self.mark_detail_flag("color", "stroke", stroke);
        }
        if scene.regions.iter().any(|region| region.is_ruby) {
            self.ruby = true;
            self.mark("ruby", true);
            self.mark_detail_flag("ruby", "boundAnnotation", true);
        }
        let referenced_drcs_codes = scene
            .regions
            .iter()
            .filter_map(|region| {
                let start = region.first_character as usize;
                let end = start.saturating_add(region.character_count as usize);
                scene.characters.get(start..end)
            })
            .flatten()
            .filter_map(|character| (character.drcs_code != 0).then_some(character.drcs_code))
            .collect::<std::collections::BTreeSet<_>>();
        if !referenced_drcs_codes.is_empty() {
            self.drcs = true;
            self.mark("drcs", true);
            self.mark_detail_flag("drcs", "referenced", true);
            self.mark_detail_flag(
                "drcs",
                "resourceBacked",
                scene
                    .drcs_glyphs
                    .iter()
                    .any(|glyph| referenced_drcs_codes.contains(&glyph.drcs_code)),
            );
        }
        let gaiji_count = scene
            .characters
            .iter()
            .filter(|character| b24_character_is_gaiji_source(character))
            .count();
        if gaiji_count > 0 {
            self.gaiji = true;
            self.mark_count("gaiji", gaiji_count);
            self.mark_detail_flag("gaiji", "aribAdditionalSymbol", true);
        }
        for characters in scene.regions.iter().filter_map(|region| {
            let start = region.first_character as usize;
            let end = start.saturating_add(region.character_count as usize);
            scene.characters.get(start..end)
        }) {
            let text = characters
                .iter()
                .map(|character| character.utf8.as_str())
                .collect::<String>();
            self.observe_accessibility(&crate::caption_features::accessibility_evidence(&text));
        }
    }

    pub(crate) fn observe_ttml(&mut self, caption: &TtmlCaption) {
        if !caption.ruby_bindings.is_empty() {
            self.ruby = true;
            self.mark("ruby", true);
            self.mark_detail_flag("ruby", "boundAnnotation", true);
        }
        let explicit_geometry = caption
            .source_layout
            .as_ref()
            .is_some_and(|layout| layout.explicit_origin || layout.explicit_extent);
        let vertical_writing = caption
            .style
            .writing_mode
            .as_deref()
            .is_some_and(|mode| matches!(mode, "vertical-rl" | "vertical-lr"));
        let explicit_direction = caption.style.direction.is_some();
        let explicit_alignment =
            caption.style.text_align.is_some() || caption.style.display_align.is_some();
        if explicit_geometry
            || caption.style.writing_mode.is_some()
            || explicit_direction
            || explicit_alignment
        {
            self.position = true;
            self.mark("position", true);
            self.mark_detail_flag("position", "explicitGeometry", explicit_geometry);
            self.mark_detail_flag("position", "verticalWriting", vertical_writing);
            self.mark_detail_flag("position", "explicitDirection", explicit_direction);
            self.mark_detail_flag("position", "explicitAlignment", explicit_alignment);
        }
        let inline_runs = caption
            .rich_body
            .as_deref()
            .map(|body| crate::parse_ttml_inline_runs(body, &caption.style))
            .unwrap_or_default();
        let styles = std::iter::once(&caption.style)
            .chain(inline_runs.iter().map(|run| &run.style))
            .chain(inline_runs.iter().filter_map(|run| run.ruby_style.as_ref()));
        let mut foreground = false;
        let mut background = false;
        let mut stroke = false;
        for style in styles {
            foreground |= style
                .color
                .as_deref()
                .is_some_and(ttml_foreground_is_material);
            background |= style
                .background_color
                .as_deref()
                .is_some_and(ttml_background_is_material);
            stroke |= style
                .text_outline
                .as_deref()
                .is_some_and(ttml_outline_is_material);
        }
        if foreground || background || stroke {
            self.color = true;
            self.mark("color", true);
            self.mark_detail_flag("color", "foreground", foreground);
            self.mark_detail_flag("color", "background", background);
            self.mark_detail_flag("color", "stroke", stroke);
        }
        let drcs_count = ttml_font_resource_character_count(
            &caption.text,
            &caption.style,
            caption.rich_body.as_deref(),
        );
        if drcs_count > 0 {
            self.drcs = true;
            self.mark_count("drcs", drcs_count);
            self.mark_detail_flag("drcs", "referenced", true);
            self.mark_detail_flag("drcs", "resourceBacked", true);
        }
        let gaiji_count = crate::caption_features::gaiji_ranges(&caption.text).len();
        if gaiji_count > 0 {
            self.gaiji = true;
            self.mark_count("gaiji", gaiji_count);
            self.mark_detail_flag("gaiji", "aribAdditionalSymbol", true);
        }
        let semantic_ranges = caption
            .accessibility_cues
            .iter()
            .map(|cue| cue.start..cue.end)
            .collect::<Vec<_>>();
        if !semantic_ranges.is_empty() {
            self.accessibility = true;
            self.mark_count("accessibility", semantic_ranges.len());
            self.mark_detail_flag("accessibility", "semanticRole", true);
        }
        self.observe_accessibility_excluding(
            &crate::caption_features::accessibility_evidence(&caption.text),
            &semantic_ranges,
        );
    }
}

fn compact_ttml_color(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn ttml_foreground_is_material(value: &str) -> bool {
    !matches!(
        compact_ttml_color(value).as_str(),
        "white"
            | "#fff"
            | "#ffff"
            | "#ffffff"
            | "#ffffffff"
            | "rgb(255,255,255)"
            | "rgba(255,255,255,1)"
            | "rgba(255,255,255,1.0)"
    )
}

fn ttml_background_is_material(value: &str) -> bool {
    let compact = compact_ttml_color(value);
    let transparent_hex = compact
        .strip_prefix('#')
        .is_some_and(|hex| match hex.len() {
            4 => hex.ends_with('0'),
            8 => hex.ends_with("00"),
            _ => false,
        });
    let transparent_rgba =
        compact.starts_with("rgba(") && (compact.ends_with(",0)") || compact.ends_with(",0.0)"));
    compact != "transparent" && !transparent_hex && !transparent_rgba
}

fn ttml_outline_is_material(value: &str) -> bool {
    if value.trim().eq_ignore_ascii_case("none") {
        return false;
    }
    let widths = value.split_whitespace().filter_map(|token| {
        ["px", "em", "c", "%"]
            .into_iter()
            .find_map(|unit| token.strip_suffix(unit))
            .and_then(|number| number.parse::<f64>().ok())
    });
    let mut found_width = false;
    for width in widths {
        found_width = true;
        if width > 0.0 {
            return true;
        }
    }
    !found_width
        && value
            .trim()
            .parse::<f64>()
            .map_or(true, |width| width > 0.0)
}

pub(crate) fn b24_character_is_gaiji_source(character: &native_b24::CaptionCharacter) -> bool {
    character.pua_codepoint != 0
        && crate::arib_symbols::is_arib_additional_symbol_codepoint(character.pua_codepoint)
}

#[cfg(test)]
mod feature_tests {
    use super::*;

    fn b24_character() -> native_b24::CaptionCharacter {
        native_b24::CaptionCharacter {
            kind: 0,
            codepoint: '字' as u32,
            pua_codepoint: 0,
            drcs_code: 0,
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
            utf8: "字".into(),
        }
    }

    #[test]
    fn b24_material_features_are_observed_from_used_caption_content() {
        let mut character = b24_character();
        character.drcs_code = 7;
        character.text_color = 0xff00_ff00;
        let scene = native_b24::CaptionScene {
            pts_ms: 0,
            wait_duration_ms: 1_000,
            plane_width: 960,
            plane_height: 540,
            regions: vec![native_b24::CaptionRegion {
                x: 100,
                y: 200,
                width: 36,
                height: 36,
                is_ruby: true,
                first_character: 0,
                character_count: 1,
            }],
            characters: vec![character],
            drcs_glyphs: Vec::new(),
            rendered_image: None,
        };
        let mut features = CaptionFeatureSummary::default();

        features.observe_b24_scene(&scene);

        assert!(features.ruby);
        assert!(features.drcs);
        assert!(features.position);
        assert!(features.color);
        assert_eq!(features.details("ruby").unwrap()["boundAnnotation"], true);
        assert_eq!(features.details("drcs").unwrap()["referenced"], true);
        assert_eq!(
            features.details("position").unwrap()["explicitGeometry"],
            true
        );
        assert_eq!(features.details("color").unwrap()["foreground"], true);
    }

    #[test]
    fn default_b24_presentation_is_not_a_material_feature() {
        let scene = native_b24::CaptionScene {
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
            characters: vec![b24_character()],
            drcs_glyphs: Vec::new(),
            rendered_image: None,
        };
        let mut features = CaptionFeatureSummary::default();

        features.observe_b24_scene(&scene);

        assert!(!features.ruby);
        assert!(!features.drcs);
        assert!(!features.position);
        assert!(!features.color);
        assert!(features.feature_details.is_empty());
        assert_eq!(features.state("ruby"), FeatureState::Unknown);
        features.complete = true;
        assert_eq!(features.state("ruby"), FeatureState::Absent);
    }

    #[test]
    fn present_feature_does_not_regress_when_stream_completes() {
        let mut features = CaptionFeatureSummary {
            ruby: true,
            ..Default::default()
        };
        features.observed_counts.insert("ruby".into(), 3);

        features.complete = true;

        assert_eq!(features.state("ruby"), FeatureState::Present);
        assert_eq!(features.observed_counts["ruby"], 3);
    }

    #[test]
    fn b24_gaiji_and_accessibility_use_shared_material_classifiers() {
        let mut character = b24_character();
        character.pua_codepoint = '➡' as u32;
        character.utf8 = "♪".into();
        let scene = native_b24::CaptionScene {
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
            characters: vec![character],
            drcs_glyphs: Vec::new(),
            rendered_image: None,
        };
        let mut features = CaptionFeatureSummary::default();

        features.observe_b24_scene(&scene);

        assert!(features.gaiji);
        assert!(features.accessibility);
        assert_eq!(features.observed_counts["gaiji"], 1);
        assert_eq!(features.observed_counts["accessibility"], 1);
        assert_eq!(
            features.details("gaiji").unwrap()["aribAdditionalSymbol"],
            true
        );
        assert_eq!(features.details("accessibility").unwrap()["textCue"], true);
        assert_eq!(features.details("accessibility").unwrap()["musicCue"], true);
    }

    #[test]
    fn b24_accessibility_respects_independent_region_text_boundaries() {
        let characters = ["本", "文", "（", "シ", "ン", "ジ", "）", "台", "詞"]
            .into_iter()
            .map(|text| {
                let mut character = b24_character();
                character.utf8 = text.into();
                character
            })
            .collect();
        let scene = native_b24::CaptionScene {
            pts_ms: 0,
            wait_duration_ms: 1_000,
            plane_width: 960,
            plane_height: 540,
            regions: vec![
                native_b24::CaptionRegion {
                    x: 0,
                    y: 0,
                    width: 960,
                    height: 270,
                    is_ruby: false,
                    first_character: 0,
                    character_count: 2,
                },
                native_b24::CaptionRegion {
                    x: 0,
                    y: 270,
                    width: 960,
                    height: 270,
                    is_ruby: false,
                    first_character: 2,
                    character_count: 7,
                },
            ],
            characters,
            drcs_glyphs: Vec::new(),
            rendered_image: None,
        };
        let mut features = CaptionFeatureSummary::default();

        features.observe_b24_scene(&scene);

        assert!(features.accessibility);
        assert_eq!(features.observed_counts["accessibility"], 1);
        assert_eq!(
            features.details("accessibility").unwrap()["leadingAnnotation"],
            true
        );
    }

    #[test]
    fn paired_narration_delimiters_count_as_one_accessibility_cue() {
        let characters = ["＜", "語", "り", "＞"]
            .into_iter()
            .map(|text| {
                let mut character = b24_character();
                character.utf8 = text.into();
                character
            })
            .collect();
        let scene = native_b24::CaptionScene {
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
                character_count: 4,
            }],
            characters,
            drcs_glyphs: Vec::new(),
            rendered_image: None,
        };
        let mut features = CaptionFeatureSummary::default();

        features.observe_b24_scene(&scene);

        assert_eq!(features.observed_counts["accessibility"], 1);
        assert_eq!(
            features.details("accessibility").unwrap()["narrationDelimiter"],
            true
        );
    }

    #[test]
    fn ttml_gaiji_and_accessibility_are_source_text_facts() {
        let caption = crate::parse_ttml_captions(
            "<tt><body><p begin='0s' end='1s'>➡♪〜本文</p></body></tt>",
            0,
        )
        .remove(0);
        let mut features = CaptionFeatureSummary::default();

        features.observe_ttml(&caption);

        assert!(features.gaiji);
        assert!(features.accessibility);
        assert_eq!(features.observed_counts["gaiji"], 1);
        assert_eq!(features.observed_counts["accessibility"], 1);
        assert_eq!(
            features.details("gaiji").unwrap()["aribAdditionalSymbol"],
            true
        );
        assert_eq!(features.details("accessibility").unwrap()["textCue"], true);
        assert_eq!(features.details("accessibility").unwrap()["musicCue"], true);
    }

    #[test]
    fn ttml_semantic_accessibility_roles_are_source_facts() {
        let caption = crate::parse_ttml_captions(
            "<tt xmlns:ttm='http://www.w3.org/ns/ttml#metadata'><body><p begin='0s' end='1s'><span ttm:role='sound'>ドア音</span></p></body></tt>",
            0,
        )
        .remove(0);
        let mut features = CaptionFeatureSummary::default();

        features.observe_ttml(&caption);

        assert!(features.accessibility);
        assert_eq!(features.observed_counts["accessibility"], 1);
        assert_eq!(
            features.details("accessibility").unwrap()["semanticRole"],
            true
        );
    }

    #[test]
    fn semantic_role_and_text_cue_overlap_count_once() {
        let caption = crate::parse_ttml_captions(
            "<tt xmlns:ttm='http://www.w3.org/ns/ttml#metadata'><body><p begin='0s' end='1s'><span ttm:role='music'>♪〜</span>音楽</p></body></tt>",
            0,
        )
        .remove(0);
        let mut features = CaptionFeatureSummary::default();

        features.observe_ttml(&caption);

        assert_eq!(features.observed_counts["accessibility"], 1);
        let details = features.details("accessibility").unwrap();
        assert_eq!(details["semanticRole"], true);
        assert_eq!(details["musicCue"], true);
    }

    #[test]
    fn ttml_default_colors_and_transparent_background_are_not_material() {
        for color in ["white", "#FFF", "#FFFFFF", "#FFFFFFFF"] {
            for background in ["transparent", "#00000000", "#FFFFFF00"] {
                let xml = format!(
                    "<tt><body><p begin='0s' end='1s' tts:color='{color}' tts:backgroundColor='{background}' tts:textOutline='none'>本文</p></body></tt>"
                );
                let caption = crate::parse_ttml_captions(&xml, 0).remove(0);
                let mut features = CaptionFeatureSummary::default();

                features.observe_ttml(&caption);

                assert!(
                    !features.color,
                    "{color} with {background} should be default presentation"
                );
            }
        }
    }

    #[test]
    fn ttml_fallback_coordinates_are_not_material_but_explicit_geometry_is() {
        let ordinary =
            crate::parse_ttml_captions("<tt><body><p begin='0s' end='1s'>本文</p></body></tt>", 0)
                .remove(0);
        assert_eq!((ordinary.x, ordinary.y), (960, 920));
        let mut ordinary_features = CaptionFeatureSummary::default();
        ordinary_features.observe_ttml(&ordinary);
        assert!(!ordinary_features.position);

        let explicit = crate::parse_ttml_captions(
            "<tt><head><layout><region xml:id='r' tts:origin='960px 920px'/></layout></head><body><p region='r' begin='0s' end='1s'>本文</p></body></tt>",
            0,
        )
        .remove(0);
        assert_eq!((explicit.x, explicit.y), (960, 920));
        let mut explicit_features = CaptionFeatureSummary::default();
        explicit_features.observe_ttml(&explicit);
        assert!(explicit_features.position);
        assert_eq!(
            explicit_features.details("position").unwrap()["explicitGeometry"],
            true
        );
    }

    #[test]
    fn ttml_alignment_outline_and_inline_color_are_material_features() {
        for attribute in ["tts:textAlign='center'", "tts:displayAlign='after'"] {
            let xml = format!("<tt><body><p begin='0s' end='1s' {attribute}>本文</p></body></tt>");
            let caption = crate::parse_ttml_captions(&xml, 0).remove(0);
            let mut features = CaptionFeatureSummary::default();
            features.observe_ttml(&caption);
            assert!(features.position, "{attribute} should be material layout");
            assert_eq!(
                features.details("position").unwrap()["explicitAlignment"],
                true
            );
        }

        let vertical = crate::parse_ttml_captions(
            "<tt><body><p begin='0s' end='1s' tts:writingMode='tbrl'>本文</p></body></tt>",
            0,
        )
        .remove(0);
        let mut vertical_features = CaptionFeatureSummary::default();
        vertical_features.observe_ttml(&vertical);
        assert_eq!(
            vertical_features.details("position").unwrap()["verticalWriting"],
            true
        );

        for body in [
            "<p begin='0s' end='1s' tts:textOutline='2px #000000'>本文</p>",
            "<p begin='0s' end='1s'><span tts:color='#00FFFF'>本文</span></p>",
        ] {
            let xml = format!("<tt><body>{body}</body></tt>");
            let caption = crate::parse_ttml_captions(&xml, 0).remove(0);
            let mut features = CaptionFeatureSummary::default();
            features.observe_ttml(&caption);
            assert!(features.color, "{body} should be material color");
            let detail = if body.contains("textOutline") {
                "stroke"
            } else {
                "foreground"
            };
            assert_eq!(features.details("color").unwrap()[detail], true);
        }
    }

    #[test]
    fn ttml_drcs_feature_includes_used_resource_fonts() {
        let ordinary = crate::parse_ttml_captions(
            r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body><p begin='0s' end='1s' arib-tt:font-face='subt://9'>字</p></body></tt>"#,
            0,
        )
        .remove(0);
        let mut ordinary_features = CaptionFeatureSummary::default();
        ordinary_features.observe_ttml(&ordinary);
        assert!(ordinary_features.drcs);
        assert_eq!(
            ordinary_features.details("drcs").unwrap()["referenced"],
            true
        );
        assert_eq!(
            ordinary_features.details("drcs").unwrap()["resourceBacked"],
            true
        );
        assert!(ordinary.drcs_uses.is_empty());

        let referenced = crate::parse_ttml_captions(
            r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body><p begin='0s' end='1s' arib-tt:font-face='subt://9'>&#xE000;</p></body></tt>"#,
            0,
        )
        .remove(0);
        assert_eq!(referenced.text, "\u{e000}");
        assert_eq!(referenced.style.font_resource.as_deref(), Some("subt://9"));
        let mut features = CaptionFeatureSummary::default();
        features.observe_ttml(&referenced);
        assert!(features.drcs);
        assert_eq!(features.observed_counts["drcs"], 1);
        assert_eq!(features.details("drcs").unwrap()["referenced"], true);
        assert_eq!(features.details("drcs").unwrap()["resourceBacked"], true);
        assert_eq!(referenced.drcs_uses[0].source_codepoint, 0xe000);
        assert_eq!(referenced.drcs_uses[0].resource_index, 9);

        let unreferenced = crate::parse_ttml_captions(
            "<tt><body><p begin='0s' end='1s'>&#xE000;</p></body></tt>",
            0,
        )
        .remove(0);
        let mut without_reference = CaptionFeatureSummary::default();
        without_reference.observe_ttml(&unreferenced);
        assert!(!without_reference.drcs);
    }

    #[test]
    fn ttml_drcs_mapping_classifies_private_and_replacement_characters() {
        assert_eq!(ttml_drcs_kind('\u{e000}'), Some(TtmlDrcsKind::PrivateUse));
        assert_eq!(ttml_drcs_kind('\u{f0000}'), Some(TtmlDrcsKind::PrivateUse));
        assert_eq!(
            ttml_drcs_kind('\u{fffc}'),
            Some(TtmlDrcsKind::ObjectReplacement)
        );
        assert_eq!(ttml_drcs_kind('\u{fffd}'), Some(TtmlDrcsKind::Replacement));
        assert_eq!(ttml_drcs_kind('字'), None);
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlCaption {
    pub(crate) start_ms: i64,
    pub(crate) end_ms: i64,
    pub(crate) text: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: Option<i32>,
    pub(crate) height: Option<i32>,
    pub(crate) style: TtmlCaptionStyle,
    pub(crate) rich_body: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) drcs_uses: Vec<TtmlDrcsUse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) ruby_bindings: Vec<TtmlRubyBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) accessibility_cues: Vec<TtmlAccessibilityCue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_layout: Option<TtmlSourceLayout>,
    pub(crate) source: Option<TtmlCaptionSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TtmlAccessibilityCue {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) roles: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TtmlDrcsKind {
    PrivateUse,
    ObjectReplacement,
    Replacement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TtmlDrcsUse {
    pub(crate) run_index: usize,
    pub(crate) character_index: usize,
    pub(crate) source_codepoint: u32,
    pub(crate) resource_index: u32,
    pub(crate) kind: TtmlDrcsKind,
}

pub(crate) fn ttml_drcs_mapping_key(
    source: Option<&TtmlCaptionSource>,
    resource_index: u32,
    source_codepoint: u32,
) -> Option<String> {
    let index = u8::try_from(resource_index).ok()?;
    let resource = source?
        .resources
        .iter()
        .find(|resource| resource.index == index)?;
    (!resource.content_sha256.is_empty())
        .then(|| crate::resource::b62_drcs_mapping_key(&resource.content_sha256, source_codepoint))
}

pub(crate) fn ttml_drcs_kind(character: char) -> Option<TtmlDrcsKind> {
    match character as u32 {
        0xe000..=0xf8ff | 0xf0000..=0xffffd | 0x100000..=0x10fffd => Some(TtmlDrcsKind::PrivateUse),
        0xfffc => Some(TtmlDrcsKind::ObjectReplacement),
        0xfffd => Some(TtmlDrcsKind::Replacement),
        _ => None,
    }
}

pub(crate) fn ttml_drcs_uses(
    text: &str,
    style: &TtmlCaptionStyle,
    rich_body: Option<&str>,
) -> Vec<TtmlDrcsUse> {
    ttml_resolved_runs(text, style, rich_body)
        .iter()
        .enumerate()
        .flat_map(|(run_index, run)| {
            let resource_index = run
                .style
                .font_resource
                .as_deref()
                .and_then(subt_resource_index);
            run.text
                .chars()
                .enumerate()
                .filter_map(move |(character_index, character)| {
                    Some(TtmlDrcsUse {
                        run_index,
                        character_index,
                        source_codepoint: character as u32,
                        resource_index: resource_index?,
                        kind: ttml_drcs_kind(character)?,
                    })
                })
        })
        .collect()
}

fn ttml_resolved_runs(
    text: &str,
    style: &TtmlCaptionStyle,
    rich_body: Option<&str>,
) -> Vec<crate::TtmlInlineRun> {
    let mut runs = rich_body
        .and_then(|body| {
            let prefix = format!(
                "<body xmlns:tts='http://www.w3.org/ns/ttml#styling' xmlns:ttm='http://www.w3.org/ns/ttml#metadata' xmlns:arib='https://resubwinny.dev/ns/arib' xmlns:arib-tt='{ARIB_TTML_NAMESPACE}'>"
            );
            let wrapped = format!("{prefix}{body}</body>");
            let document = roxmltree::Document::parse(&wrapped).ok()?;
            Some(
                document
                    .descendants()
                    .filter(|node| node.is_text())
                    .filter_map(|node| {
                        let text = node.text()?.to_owned();
                        if text.is_empty() {
                            return None;
                        }
                        let mut run_style = style.clone();
                        if let Some(font_resource) = node.ancestors().find_map(|ancestor| {
                            ancestor.attributes().find_map(|attribute| {
                                (attribute.name() == "font-face"
                                    && attribute.namespace() == Some(ARIB_TTML_NAMESPACE))
                                .then(|| attribute.value().to_owned())
                            })
                        }) {
                            run_style.font_resource = Some(font_resource);
                        }
                        Some(crate::TtmlInlineRun {
                            text,
                            style: run_style,
                            ..Default::default()
                        })
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or_default();
    if runs.is_empty() {
        runs.push(crate::TtmlInlineRun {
            text: text.to_owned(),
            style: style.clone(),
            ..Default::default()
        });
    }
    runs
}

pub(crate) fn ttml_font_resource_character_count(
    text: &str,
    style: &TtmlCaptionStyle,
    rich_body: Option<&str>,
) -> usize {
    ttml_resolved_runs(text, style, rich_body)
        .iter()
        .filter(|run| {
            run.style
                .font_resource
                .as_deref()
                .and_then(subt_resource_index)
                .is_some()
        })
        .map(|run| run.text.chars().count())
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TtmlSourcePlaneBasis {
    Declared,
    Inferred,
    LegacyLogical2k,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TtmlSourceLayout {
    pub(crate) plane_width: i32,
    pub(crate) plane_height: i32,
    pub(crate) plane_basis: TtmlSourcePlaneBasis,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: Option<i32>,
    pub(crate) height: Option<i32>,
    #[serde(skip)]
    pub(crate) explicit_origin: bool,
    #[serde(skip)]
    pub(crate) explicit_extent: bool,
    pub(crate) style: TtmlCaptionStyle,
    pub(crate) rich_body: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub(crate) struct TtmlCaptionStyle {
    pub(crate) color: Option<String>,
    pub(crate) background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) background_scope: Option<TtmlBackgroundScope>,
    pub(crate) font_size: Option<String>,
    pub(crate) font_family: Option<String>,
    pub(crate) font_style: Option<String>,
    pub(crate) font_weight: Option<String>,
    pub(crate) writing_mode: Option<String>,
    pub(crate) direction: Option<String>,
    pub(crate) text_align: Option<String>,
    pub(crate) text_outline: Option<String>,
    pub(crate) line_height: Option<String>,
    pub(crate) letter_spacing: Option<String>,
    pub(crate) opacity: Option<String>,
    pub(crate) display_align: Option<String>,
    pub(crate) background_image: Option<String>,
    pub(crate) font_resource: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TtmlBackgroundScope {
    Region,
    Block,
    Inline,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub(crate) struct RationalTimestamp {
    pub(crate) value: i64,
    pub(crate) timescale: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // Exactly one route is compiled into a worker backend.
pub(crate) enum TlvTimelineBasis {
    MptPresentationNtp,
    LibaribTlvNormalizedPts,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlCaptionSource {
    pub(crate) route: &'static str,
    pub(crate) source_offset: u64,
    pub(crate) mmpt_packet_id: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mpu_sequence_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mmtp_sequence_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) presentation_ntp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) normalized_pts: Option<RationalTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reference_start_pts: Option<RationalTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reference_start_ntp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reference_start_time_leap_indicator: Option<u8>,
    pub(crate) timeline_basis: TlvTimelineBasis,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) track_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) component_tag: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timing_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operation_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) display_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) compression_type: Option<u8>,
    pub(crate) random_access: bool,
    pub(crate) discontinuity: bool,
    pub(crate) discontinuity_reasons: u32,
    pub(crate) xml_encoding: String,
    pub(crate) resources: Vec<TtmlResourceMetadata>,
    pub(crate) resources_complete: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct TtmlResourceScope {
    pub(crate) packet_id: u16,
    pub(crate) mpu_sequence_number: u32,
}

impl TtmlResourceScope {
    pub(crate) fn key(&self) -> String {
        format!("packet:{}:mpu:{}", self.packet_id, self.mpu_sequence_number)
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlResourceAssociation {
    pub(crate) status: &'static str,
    pub(crate) scope: Option<TtmlResourceScope>,
    pub(crate) scope_key: Option<String>,
    pub(crate) resource_record_key: Option<String>,
    pub(crate) resource_data_type: Option<u8>,
    pub(crate) resource_byte_length: Option<usize>,
    pub(crate) resource_format_hint: Option<&'static str>,
    pub(crate) resource_format_validation: Option<&'static str>,
    pub(crate) resource_width: Option<u32>,
    pub(crate) resource_height: Option<u32>,
    pub(crate) resource_preview_available: Option<bool>,
    pub(crate) reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlResourceReference {
    pub(crate) uri: String,
    pub(crate) resource_index: u32,
    pub(crate) usage: &'static str,
    pub(crate) extraction: String,
    pub(crate) association: TtmlResourceAssociation,
    pub(crate) source: Option<TtmlCaptionSource>,
}

pub(crate) fn subt_resource_index(value: &str) -> Option<u32> {
    let index = value.strip_prefix("subt://")?;
    (!index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| index.parse::<u32>().ok())
        .flatten()
}

pub(crate) fn ttml_resource_references(caption: &TtmlCaption) -> Vec<TtmlResourceReference> {
    let scope = caption.source.as_ref().and_then(|source| {
        source
            .mpu_sequence_number
            .map(|mpu_sequence_number| TtmlResourceScope {
                packet_id: source.mmpt_packet_id,
                mpu_sequence_number,
            })
    });
    let scope_key = scope.as_ref().map(TtmlResourceScope::key);
    [
        (caption.style.background_image.as_deref(), "background-image"),
        (caption.style.font_resource.as_deref(), "font-face"),
    ]
    .into_iter()
    .filter_map(|(uri, usage)| {
        let uri = uri?;
        let resource_index = subt_resource_index(uri)?;
        let metadata = u8::try_from(resource_index).ok().and_then(|index| {
            caption
                .source
                .as_ref()
                .and_then(|source| source.resources.iter().find(|item| item.index == index))
        });
        let evidence_match = metadata.is_some();
        let resource_record_key = scope_key.as_ref().zip(metadata).map(|(scope_key, metadata)| {
            format!("stpp-resource:{}:subsample:{}", scope_key, metadata.index)
        });
        Some(TtmlResourceReference {
            uri: uri.to_owned(),
            resource_index,
            usage,
            extraction: if evidence_match {
                "same-MPU resource evidence retained; semantic decode is not enabled".into()
            } else {
                "unresolved: archive reference only; no matching same-MPU resource evidence".into()
            },
            association: TtmlResourceAssociation {
                status: if evidence_match { "same-mpu-evidence" } else { "unresolved" },
                scope: scope.clone(),
                scope_key: scope_key.clone(),
                resource_record_key,
                resource_data_type: metadata.map(|item| item.data_type),
                resource_byte_length: metadata.map(|item| item.byte_length),
                resource_format_hint: metadata.and_then(|item| item.format_hint),
                resource_format_validation: metadata.map(|item| item.format_validation),
                resource_width: metadata.and_then(|item| item.width),
                resource_height: metadata.and_then(|item| item.height),
                resource_preview_available: metadata.map(|item| item.preview_available),
                reason: if evidence_match {
                    "same-MPU resource evidence matched by subsample index; resource bytes remain raw"
                } else if caption.source.as_ref().map(|source| source.resources_complete).unwrap_or(false) {
                    "resource map was complete but the referenced subsample index was absent"
                } else {
                    "subt:// indices are scoped to the current MPU resource map; resource evidence is incomplete"
                },
            },
            source: caption.source.clone(),
        })
    })
    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XmlTextEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    ShiftJis,
    EucJp,
    Iso2022Jp,
}

impl XmlTextEncoding {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::ShiftJis => "Shift_JIS",
            Self::EucJp => "EUC-JP",
            Self::Iso2022Jp => "ISO-2022-JP",
        }
    }

    pub(crate) fn closing_tag(self) -> &'static [u8] {
        match self {
            Self::Utf16Le => b"<\0/\0t\0t\0>\0",
            Self::Utf16Be => b"\0<\0/\0t\0t\0>",
            _ => b"</tt>",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DecodedTtmlDocument {
    pub(crate) xml: String,
    pub(crate) encoding: XmlTextEncoding,
}
