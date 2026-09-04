use crate::{CaptionFeatureSummary, ConversionOptions, TtmlCaption, native_b24};
use serde::Serialize;
use std::{fmt, io};

const CAPABILITIES: &str = include_str!("../../../shared/format_capabilities.json");

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportConflict {
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

fn conflict(options: &ConversionOptions, feature: &str) -> io::Result<()> {
    let formats = unsupported_formats(options, feature);
    if formats.is_empty() {
        return Ok(());
    }
    Err(io::Error::other(ExportConflict {
        formats,
        feature: feature.into(),
        logical_track: std::env::var("RESUBWINNY_LOGICAL_TRACK")
            .unwrap_or_else(|_| "logical-track:default".into()),
        available_actions: vec![
            format!("disable_preservation:{feature}"),
            "remove_format".into(),
            "choose_compatible_format".into(),
        ],
    }))
}

pub(crate) fn assess_b24_scene(
    options: &ConversionOptions,
    scene: &native_b24::CaptionScene,
) -> io::Result<()> {
    let mut facts = CaptionFeatureSummary::default();
    facts.observe_b24_scene(scene);
    assess_facts(options, &facts)
}

pub(crate) fn assess_ttml_caption(
    options: &ConversionOptions,
    caption: &TtmlCaption,
) -> io::Result<()> {
    let mut facts = CaptionFeatureSummary::default();
    facts.observe_ttml(caption);
    assess_facts(options, &facts)
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
}
