use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::{RubyLayoutBox, RubyPlacement, RubyWritingMode};

use super::ass_rich::ass_text_ink_bounds;

pub(crate) const RUBY_FONT_FAMILY: &str = "Rounded M+ 1m for ARIB";

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RubyGlyphMetrics {
    pub(crate) advance: f32,
    pub(crate) ink_start: f32,
    pub(crate) ink_end: f32,
}

impl RubyGlyphMetrics {
    fn ink_width(self) -> f32 {
        (self.ink_end - self.ink_start).max(0.0)
    }

    fn ink_offset_from_advance_center(self) -> f32 {
        (self.ink_start + self.ink_end) * 0.5 - self.advance * 0.5
    }
}

pub(crate) trait RubyGlyphMetricsProvider {
    fn measure(&self, text: &str, font_size: i32) -> RubyGlyphMetrics;
}

pub(crate) struct BundledAssGlyphMetrics;

impl RubyGlyphMetricsProvider for BundledAssGlyphMetrics {
    fn measure(&self, text: &str, font_size: i32) -> RubyGlyphMetrics {
        let (advance, ink_start, ink_end) = ass_text_ink_bounds(text, font_size.max(1) as f32);
        RubyGlyphMetrics {
            advance,
            ink_start,
            ink_end,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RubyLayoutRequest<'a> {
    pub(crate) text: &'a str,
    pub(crate) container: RubyLayoutBox,
    pub(crate) preferred_font_size: i32,
    pub(crate) minimum_font_size: i32,
    pub(crate) placement: RubyPlacement,
    pub(crate) writing_mode: RubyWritingMode,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct RubyGlyphPlacement {
    pub(crate) text: String,
    pub(crate) anchor_x: i32,
    pub(crate) anchor_y: i32,
    pub(crate) slot: RubyLayoutBox,
    pub(crate) advance: f32,
    pub(crate) ink_start: f32,
    pub(crate) ink_end: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct RubyLayoutPlan {
    pub(crate) container: RubyLayoutBox,
    pub(crate) font_family: String,
    pub(crate) font_size: i32,
    pub(crate) writing_mode: RubyWritingMode,
    pub(crate) placement: RubyPlacement,
    pub(crate) glyphs: Vec<RubyGlyphPlacement>,
    pub(crate) degraded: bool,
}

pub(crate) fn layout_ruby(
    request: &RubyLayoutRequest<'_>,
    metrics_provider: &impl RubyGlyphMetricsProvider,
) -> Option<RubyLayoutPlan> {
    let graphemes = request
        .text
        .graphemes(true)
        .filter(|grapheme| !grapheme.is_empty())
        .collect::<Vec<_>>();
    if graphemes.is_empty() {
        return None;
    }

    let primary_extent = match request.writing_mode {
        RubyWritingMode::HorizontalTb => request.container.width,
        RubyWritingMode::VerticalRl | RubyWritingMode::VerticalLr => request.container.height,
    }
    .max(1);
    let slot_extents = distributed_extents(primary_extent, graphemes.len());
    let preferred = request.preferred_font_size.max(1);
    let minimum = request.minimum_font_size.clamp(1, preferred);
    let mut font_size = preferred;
    let mut measured = measure_graphemes(&graphemes, font_size, metrics_provider);
    while font_size > minimum && !ink_fits_slots(&measured, &slot_extents) {
        font_size -= 1;
        measured = measure_graphemes(&graphemes, font_size, metrics_provider);
    }
    let fits = ink_fits_slots(&measured, &slot_extents);

    let mut primary_cursor = match request.writing_mode {
        RubyWritingMode::HorizontalTb => request.container.x,
        RubyWritingMode::VerticalRl | RubyWritingMode::VerticalLr => request.container.y,
    };
    let mut glyphs = graphemes
        .into_iter()
        .zip(measured)
        .zip(slot_extents)
        .map(|((text, metrics), slot_extent)| {
            let slot_center = primary_cursor as f32 + slot_extent as f32 * 0.5;
            let anchor_primary =
                (slot_center - metrics.ink_offset_from_advance_center()).round() as i32;
            let placement = match request.writing_mode {
                RubyWritingMode::HorizontalTb => RubyGlyphPlacement {
                    text: text.to_owned(),
                    anchor_x: anchor_primary,
                    anchor_y: request.container.y,
                    slot: RubyLayoutBox {
                        x: primary_cursor,
                        y: request.container.y,
                        width: slot_extent,
                        height: request.container.height,
                    },
                    advance: metrics.advance,
                    ink_start: anchor_primary as f32 - metrics.advance * 0.5 + metrics.ink_start,
                    ink_end: anchor_primary as f32 - metrics.advance * 0.5 + metrics.ink_end,
                },
                RubyWritingMode::VerticalRl | RubyWritingMode::VerticalLr => RubyGlyphPlacement {
                    text: text.to_owned(),
                    anchor_x: request.container.x,
                    anchor_y: anchor_primary,
                    slot: RubyLayoutBox {
                        x: request.container.x,
                        y: primary_cursor,
                        width: request.container.width,
                        height: slot_extent,
                    },
                    advance: metrics.advance,
                    ink_start: anchor_primary as f32 - metrics.advance * 0.5 + metrics.ink_start,
                    ink_end: anchor_primary as f32 - metrics.advance * 0.5 + metrics.ink_end,
                },
            };
            primary_cursor = primary_cursor.saturating_add(slot_extent);
            placement
        })
        .collect::<Vec<_>>();
    recenter_visible_ink(&mut glyphs, request.container, request.writing_mode);

    Some(RubyLayoutPlan {
        container: request.container,
        font_family: RUBY_FONT_FAMILY.to_owned(),
        font_size,
        writing_mode: request.writing_mode,
        placement: request.placement,
        glyphs,
        degraded: font_size != preferred || !fits,
    })
}

fn recenter_visible_ink(
    glyphs: &mut [RubyGlyphPlacement],
    container: RubyLayoutBox,
    writing_mode: RubyWritingMode,
) {
    let Some(ink_start) = glyphs.iter().map(|glyph| glyph.ink_start).reduce(f32::min) else {
        return;
    };
    let Some(ink_end) = glyphs.iter().map(|glyph| glyph.ink_end).reduce(f32::max) else {
        return;
    };
    let container_center = match writing_mode {
        RubyWritingMode::HorizontalTb => container.x as f32 + container.width as f32 * 0.5,
        RubyWritingMode::VerticalRl | RubyWritingMode::VerticalLr => {
            container.y as f32 + container.height as f32 * 0.5
        }
    };
    let shift = (container_center - (ink_start + ink_end) * 0.5).round() as i32;
    if shift == 0 {
        return;
    }
    for glyph in glyphs {
        match writing_mode {
            RubyWritingMode::HorizontalTb => glyph.anchor_x = glyph.anchor_x.saturating_add(shift),
            RubyWritingMode::VerticalRl | RubyWritingMode::VerticalLr => {
                glyph.anchor_y = glyph.anchor_y.saturating_add(shift)
            }
        }
        glyph.ink_start += shift as f32;
        glyph.ink_end += shift as f32;
    }
}

fn measure_graphemes(
    graphemes: &[&str],
    font_size: i32,
    metrics_provider: &impl RubyGlyphMetricsProvider,
) -> Vec<RubyGlyphMetrics> {
    graphemes
        .iter()
        .map(|grapheme| metrics_provider.measure(grapheme, font_size))
        .collect()
}

fn distributed_extents(total: i32, count: usize) -> Vec<i32> {
    let total = total.max(1) as i64;
    let count = count.max(1) as i64;
    (0..count)
        .map(|index| {
            let start = index * total / count;
            let end = (index + 1) * total / count;
            (end - start) as i32
        })
        .collect()
}

fn ink_fits_slots(metrics: &[RubyGlyphMetrics], slot_extents: &[i32]) -> bool {
    metrics
        .iter()
        .zip(slot_extents)
        .all(|(metrics, slot)| metrics.ink_width() <= (*slot).max(0) as f32 + 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SquareMetrics;

    impl RubyGlyphMetricsProvider for SquareMetrics {
        fn measure(&self, _text: &str, font_size: i32) -> RubyGlyphMetrics {
            RubyGlyphMetrics {
                advance: font_size as f32,
                ink_start: 0.0,
                ink_end: font_size as f32,
            }
        }
    }

    #[test]
    fn horizontal_slots_exactly_cover_the_base_box_without_ink_overlap() {
        let plan = layout_ruby(
            &RubyLayoutRequest {
                text: "じゅしん",
                container: RubyLayoutBox {
                    x: 100,
                    y: 80,
                    width: 81,
                    height: 20,
                },
                preferred_font_size: 18,
                minimum_font_size: 6,
                placement: RubyPlacement::Above,
                writing_mode: RubyWritingMode::HorizontalTb,
            },
            &SquareMetrics,
        )
        .expect("layout");
        assert_eq!(
            plan.glyphs
                .iter()
                .map(|glyph| glyph.slot.width)
                .sum::<i32>(),
            81
        );
        assert_eq!(plan.glyphs.first().unwrap().slot.x, 100);
        assert_eq!(plan.glyphs.last().unwrap().slot.right(), 181);
        assert!(
            plan.glyphs
                .windows(2)
                .all(|pair| pair[0].ink_end <= pair[1].ink_start)
        );
    }

    #[test]
    fn integer_font_fallback_prevents_glyph_ink_overlap() {
        let plan = layout_ruby(
            &RubyLayoutRequest {
                text: "ささ",
                container: RubyLayoutBox {
                    x: 0,
                    y: 0,
                    width: 20,
                    height: 20,
                },
                preferred_font_size: 18,
                minimum_font_size: 6,
                placement: RubyPlacement::Above,
                writing_mode: RubyWritingMode::HorizontalTb,
            },
            &SquareMetrics,
        )
        .expect("layout");
        assert_eq!(plan.font_size, 10);
        assert!(plan.degraded);
        assert!(plan.glyphs[0].ink_end <= plan.glyphs[1].ink_start);
    }

    #[test]
    fn vertical_layout_transposes_the_primary_axis() {
        let plan = layout_ruby(
            &RubyLayoutRequest {
                text: "かな",
                container: RubyLayoutBox {
                    x: 300,
                    y: 100,
                    width: 20,
                    height: 60,
                },
                preferred_font_size: 18,
                minimum_font_size: 6,
                placement: RubyPlacement::Above,
                writing_mode: RubyWritingMode::VerticalRl,
            },
            &SquareMetrics,
        )
        .expect("layout");
        assert_eq!(plan.glyphs[0].slot.y, 100);
        assert_eq!(plan.glyphs[1].slot.y, 130);
        assert_eq!(plan.glyphs[1].slot.bottom(), 160);
        assert_eq!(plan.glyphs[0].anchor_x, plan.glyphs[1].anchor_x);
    }
}
