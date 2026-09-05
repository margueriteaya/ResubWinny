use crate::{RegionInterval, TtmlCaption};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaptionTiming {
    pub(crate) begin_ms: i64,
    pub(crate) end_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaptionRegion {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: Option<i32>,
    pub(crate) height: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptionRoute {
    B24,
    AribTtml,
}

/// A zero-copy semantic boundary over the route-specific faithful payloads.
///
/// Parsers and exporters keep their B24/ARIB-TTML models. Cross-route
/// consumers use this closed adapter for semantics that are genuinely shared,
/// without copying DRCS pixels or MMT resource evidence and without pretending
/// either transport is the other.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CaptionCueRef<'a> {
    B24(&'a RegionInterval),
    AribTtml(&'a TtmlCaption),
}

impl CaptionCueRef<'_> {
    pub(crate) fn timing(self) -> CaptionTiming {
        match self {
            Self::B24(interval) => CaptionTiming {
                begin_ms: interval.begin_ms,
                end_ms: interval.end_ms,
            },
            Self::AribTtml(caption) => CaptionTiming {
                begin_ms: caption.start_ms,
                end_ms: caption.end_ms,
            },
        }
    }

    pub(crate) fn region(self) -> CaptionRegion {
        match self {
            Self::B24(interval) => CaptionRegion {
                x: interval.region.x,
                y: interval.region.y,
                width: Some(interval.region.width),
                height: Some(interval.region.height),
            },
            Self::AribTtml(caption) => CaptionRegion {
                x: caption.x,
                y: caption.y,
                width: caption.width,
                height: caption.height,
            },
        }
    }

    pub(crate) fn route(self) -> CaptionRoute {
        match self {
            Self::B24(_) => CaptionRoute::B24,
            Self::AribTtml(_) => CaptionRoute::AribTtml,
        }
    }

    /// Plain semantic text. Route-specific markup, ruby placement and style
    /// remain in the faithful payload and are intentionally not flattened.
    #[allow(dead_code)]
    pub(crate) fn plain_text(self) -> String {
        match self {
            Self::B24(interval) => interval
                .characters
                .iter()
                .map(|character| character.utf8.as_str())
                .collect(),
            Self::AribTtml(caption) => caption.text.clone(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn ruby_count(self) -> usize {
        match self {
            Self::B24(interval) => usize::from(interval.ruby_binding.is_some()),
            Self::AribTtml(caption) => caption.ruby_bindings.len(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn has_drcs(self) -> bool {
        match self {
            Self::B24(interval) => {
                !interval.drcs_glyphs.is_empty()
                    || interval
                        .characters
                        .iter()
                        .any(|character| character.drcs_code != 0)
            }
            Self::AribTtml(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TtmlCaptionStyle, native_b24};

    #[test]
    fn route_specific_payloads_share_timing_region_and_route_semantics() {
        let interval = RegionInterval {
            begin_ms: 1_000,
            end_ms: 2_000,
            wait_duration_ms: 1_000,
            plane_width: 960,
            plane_height: 540,
            source_pid: Some(0x120),
            region: native_b24::CaptionRegion {
                x: 10,
                y: 20,
                width: 30,
                height: 40,
                is_ruby: false,
                first_character: 0,
                character_count: 1,
            },
            characters: vec![native_b24::CaptionCharacter {
                kind: 0,
                codepoint: '字' as u32,
                pua_codepoint: 0,
                drcs_code: 0,
                x: 10,
                y: 20,
                width: 30,
                height: 40,
                horizontal_spacing: 0,
                vertical_spacing: 0,
                horizontal_scale: 1.0,
                vertical_scale: 1.0,
                text_color: 0,
                back_color: 0,
                stroke_color: 0,
                style: 0,
                enclosure_style: 0,
                utf8: "字".into(),
            }],
            drcs_glyphs: Vec::new(),
            ruby_binding: None,
        };
        let ttml = TtmlCaption {
            start_ms: 1_000,
            end_ms: 2_000,
            text: "字".into(),
            x: 10,
            y: 20,
            width: Some(30),
            height: Some(40),
            style: TtmlCaptionStyle::default(),
            rich_body: None,
            drcs_uses: Vec::new(),
            ruby_bindings: Vec::new(),
            source_layout: None,
            source: None,
        };
        let b24 = CaptionCueRef::B24(&interval);
        let arib_ttml = CaptionCueRef::AribTtml(&ttml);
        assert_eq!(b24.timing(), arib_ttml.timing());
        assert_eq!(b24.region(), arib_ttml.region());
        assert_eq!(b24.plain_text(), arib_ttml.plain_text());
        assert_eq!(b24.ruby_count(), arib_ttml.ruby_count());
        assert!(!b24.has_drcs());
        assert!(!arib_ttml.has_drcs());
        assert_eq!(b24.route(), CaptionRoute::B24);
        assert_eq!(arib_ttml.route(), CaptionRoute::AribTtml);
    }
}
