use super::*;

#[allow(
    clippy::too_many_arguments,
    reason = "the glyph hot path keeps drawing state explicit and allocation-free"
)]
pub(super) fn draw_character(
    canvas: &mut [u8],
    font: &Font,
    character: char,
    x: i32,
    baseline: i32,
    font_size: f32,
    foreground: [u8; 4],
    outline: Option<TextOutline>,
    missing_glyph_count: &mut usize,
) -> f32 {
    let glyph_index = font.lookup_glyph_index(character);
    if glyph_index == 0 && character != '\0' {
        *missing_glyph_count += 1;
        return font_size * 0.5;
    }
    let (metrics, bitmap) = font.rasterize_indexed(glyph_index, font_size);
    if bitmap.len() > MAX_TEXT_PIXELS {
        return 0.0;
    }
    let glyph_x = x + metrics.xmin;
    let glyph_y = baseline - metrics.height as i32 - metrics.ymin;
    if let Some(outline) = outline.filter(|outline| outline.color[3] > 0) {
        for offset_y in -outline.radius..=outline.radius {
            for offset_x in -outline.radius..=outline.radius {
                if offset_x == 0 && offset_y == 0
                    || offset_x * offset_x + offset_y * offset_y > outline.radius * outline.radius
                {
                    continue;
                }
                blend_glyph(
                    canvas,
                    glyph_x + offset_x,
                    glyph_y + offset_y,
                    metrics.width,
                    metrics.height,
                    &bitmap,
                    outline.color,
                );
            }
        }
    }
    blend_glyph(
        canvas,
        glyph_x,
        glyph_y,
        metrics.width,
        metrics.height,
        &bitmap,
        foreground,
    );
    metrics.advance_width
}

pub(super) fn measure_text(font: &Font, text: &str, font_size: f32, letter_spacing: f32) -> f32 {
    text.chars()
        .map(|character| {
            let glyph = font.lookup_glyph_index(character);
            if glyph == 0 {
                font_size * 0.5
            } else {
                font.metrics_indexed(glyph, font_size).advance_width + letter_spacing
            }
        })
        .sum()
}
