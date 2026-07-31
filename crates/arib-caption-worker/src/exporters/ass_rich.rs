use crate::{TtmlCaptionStyle, TtmlInlineRun, ass_font_size_from_ttml, parse_ttml_inline_runs};

const ASS_ARIB_FONT: &[u8] =
    include_bytes!("../../../../third_party/rounded-mplus-1m-arib/rounded-mplus-1m-arib.ttf");

pub(crate) fn bundled_ass_font() -> &'static [u8] {
    ASS_ARIB_FONT
}

pub(crate) type AssInlineRun = TtmlInlineRun;

pub(crate) fn parse_ass_inline_runs(
    body: &str,
    base_style: &TtmlCaptionStyle,
) -> Vec<AssInlineRun> {
    parse_ttml_inline_runs(body, base_style)
}

pub(crate) fn run_advance(run: &AssInlineRun, fallback_size: i32) -> f32 {
    text_advance(&run.text, &run.style, fallback_size)
}

pub(crate) fn run_ink_bounds(run: &AssInlineRun, fallback_size: i32) -> (f32, f32) {
    let size = run
        .style
        .font_size
        .as_deref()
        .and_then(ass_font_size_from_ttml)
        .unwrap_or(fallback_size) as f32;
    let (_, left, right) = ass_text_ink_bounds(&run.text, size);
    (left, right)
}

pub(crate) fn text_advance(text: &str, style: &TtmlCaptionStyle, fallback_size: i32) -> f32 {
    let size = style
        .font_size
        .as_deref()
        .and_then(ass_font_size_from_ttml)
        .unwrap_or(fallback_size) as f32;
    let spacing = style
        .letter_spacing
        .as_deref()
        .and_then(|value| value.trim().strip_suffix("px"))
        .and_then(|value| value.trim().parse::<f32>().ok())
        .unwrap_or(0.0);
    let (advance, count) = text.chars().filter(|character| *character != '\n').fold(
        (0.0, 0_usize),
        |(advance, count), character| {
            (
                advance + ass_glyph_advance(character, size),
                count.saturating_add(1),
            )
        },
    );
    advance + spacing * count.saturating_sub(1) as f32
}

/// Measures the font explicitly selected by the ASS exporter. The fallback is
/// retained only for malformed fonts and unsupported glyphs.
pub(crate) fn ass_glyph_advance(character: char, font_size: f32) -> f32 {
    let Ok(face) = ttf_parser::Face::parse(ASS_ARIB_FONT, 0) else {
        return fallback_glyph_advance(character, font_size);
    };
    let Some(glyph) = face.glyph_index(character) else {
        return fallback_glyph_advance(character, font_size);
    };
    let Some(advance) = face.glyph_hor_advance(glyph) else {
        return fallback_glyph_advance(character, font_size);
    };
    f32::from(advance) * ass_font_scale(&face, font_size)
}

pub(crate) fn ass_text_ink_bounds(text: &str, font_size: f32) -> (f32, f32, f32) {
    let Ok(face) = ttf_parser::Face::parse(ASS_ARIB_FONT, 0) else {
        let advance = text
            .chars()
            .map(|character| fallback_glyph_advance(character, font_size))
            .sum::<f32>();
        return (advance, 0.0, advance);
    };
    let scale = ass_font_scale(&face, font_size);
    let mut cursor = 0.0_f32;
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for character in text.chars() {
        let Some(glyph) = face.glyph_index(character) else {
            let advance = fallback_glyph_advance(character, font_size);
            left = left.min(cursor);
            right = right.max(cursor + advance);
            cursor += advance;
            continue;
        };
        let advance = face
            .glyph_hor_advance(glyph)
            .map(|advance| f32::from(advance) * scale)
            .unwrap_or_else(|| fallback_glyph_advance(character, font_size));
        if let Some(bounds) = face.glyph_bounding_box(glyph) {
            left = left.min(cursor + f32::from(bounds.x_min) * scale);
            right = right.max(cursor + f32::from(bounds.x_max) * scale);
        } else {
            left = left.min(cursor);
            right = right.max(cursor + advance);
        }
        cursor += advance;
    }
    if left.is_finite() && right.is_finite() {
        (cursor, left, right)
    } else {
        (cursor, 0.0, cursor)
    }
}

fn ass_font_scale(face: &ttf_parser::Face<'_>, font_size: f32) -> f32 {
    let metric_height = i32::from(face.ascender()) - i32::from(face.descender());
    if metric_height > 0 {
        font_size / metric_height as f32
    } else {
        font_size / f32::from(face.units_per_em())
    }
}

fn fallback_glyph_advance(character: char, font_size: f32) -> f32 {
    if character.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&character) {
        font_size * 0.5
    } else {
        font_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RubyPlacement;

    #[test]
    fn preserves_inline_colour_and_associates_standard_ruby() {
        let base = TtmlCaptionStyle {
            color: Some("#FFFFFF".into()),
            font_size: Some("72px".into()),
            ..TtmlCaptionStyle::default()
        };
        let runs = parse_ass_inline_runs(
            "<span tts:color='#00FFFF'>字幕</span><ruby><span tts:ruby='base'>漢</span><rt><span tts:ruby='text' tts:color='#FFFF00'>かん</span></rt></ruby>",
            &base,
        );
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].style.color.as_deref(), Some("#00FFFF"));
        assert_eq!(runs[1].ruby_text.as_deref(), Some("かん"));
        assert_eq!(
            runs[1]
                .ruby_style
                .as_ref()
                .and_then(|style| style.color.as_deref()),
            Some("#FFFF00")
        );
    }

    #[test]
    fn ruby_without_an_explicit_size_uses_the_half_size_default() {
        let base = TtmlCaptionStyle {
            font_size: Some("72px".into()),
            ..TtmlCaptionStyle::default()
        };
        let runs = parse_ass_inline_runs(
            "<ruby><span tts:ruby='base'>日</span><span tts:ruby='base'>本</span><rt><span tts:ruby='text'>にほん</span></rt></ruby>",
            &base,
        );
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1].ruby_text.as_deref(), Some("にほん"));
        assert_eq!(runs[1].ruby_group_base_count, 2);
        assert_eq!(
            runs[1]
                .ruby_style
                .as_ref()
                .and_then(|style| style.font_size.as_deref()),
            None
        );
    }

    #[test]
    fn preserves_explicit_below_placement_on_the_resolved_base_range() {
        let base = TtmlCaptionStyle {
            font_size: Some("72px".into()),
            ..TtmlCaptionStyle::default()
        };
        let runs = parse_ass_inline_runs(
            "<ruby><span tts:ruby='base'>放送</span><rt><span tts:ruby='text' tts:rubyPosition='after'>ほうそう</span></rt></ruby>",
            &base,
        );
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "放送");
        assert_eq!(runs[0].ruby_text.as_deref(), Some("ほうそう"));
        assert_eq!(runs[0].ruby_group_base_count, 1);
        assert_eq!(runs[0].ruby_placement, Some(RubyPlacement::Below));
    }
}
