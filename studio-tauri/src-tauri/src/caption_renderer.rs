use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use fontdue::{Font, FontSettings};
use serde_json::Value;
use std::{collections::BTreeSet, sync::Arc};

const MAX_LAYERS: usize = 128;
const MAX_PIXELS: u64 = 33_177_600;
const MAX_IMAGE_BYTES: usize = 128 * 1024 * 1024;
const TTML_PLANE_WIDTH: u32 = 1920;
const TTML_PLANE_HEIGHT: u32 = 1080;
const MAX_TEXT_PIXELS: usize = 2_000_000;

#[derive(Debug, Clone)]
pub(crate) struct CaptionPlaneFrame {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Arc<[u8]>,
    pub(crate) layer_count: usize,
    pub(crate) mode: &'static str,
    pub(crate) missing_glyph_count: usize,
    pub(crate) rendered_ruby_count: usize,
}

impl CaptionPlaneFrame {
    pub(crate) fn png_base64(&self) -> Option<String> {
        encode_png(self.width, self.height, &self.pixels).map(|png| BASE64.encode(png))
    }
}

struct StyledRun {
    text: String,
    id: Option<String>,
    ruby_target_id: Option<String>,
    color: [u8; 4],
    background: [u8; 4],
    font_size: f32,
    letter_spacing: f32,
    outline: Option<TextOutline>,
    ruby_text: Option<String>,
    ruby_style: Option<RubyStyle>,
    ruby_base: bool,
    ruby_group_base_count: usize,
    text_combine: bool,
}

#[derive(Clone, Copy)]
struct TextOutline {
    radius: i32,
    color: [u8; 4],
}

/// Presentation properties belonging to an explicitly associated ruby
/// annotation. Keeping these separate from the base run prevents annotation
/// spans from leaking their font metrics into the base text path.
#[derive(Clone, Copy)]
struct RubyStyle {
    color: [u8; 4],
    font_size: f32,
    letter_spacing: f32,
    outline: Option<TextOutline>,
}

fn default_ruby_style(run: &StyledRun) -> RubyStyle {
    RubyStyle {
        color: run.color,
        font_size: (run.font_size * 0.5).clamp(6.0, 80.0),
        letter_spacing: (run.letter_spacing * 0.5).clamp(-10.0, 40.0),
        outline: run.outline,
    }
}

fn ruby_style(run: &StyledRun) -> RubyStyle {
    run.ruby_style.unwrap_or_else(|| default_ruby_style(run))
}

pub(crate) fn compose(intervals: &[Value]) -> Option<CaptionPlaneFrame> {
    // Caption gaps are the common case while scrubbing. Do not initialize the
    // embedded font or allocate a full 1920x1080 RGBA plane merely to discover
    // that there is nothing to draw.
    if intervals.is_empty() {
        return None;
    }
    let mut layers = Vec::new();
    let mut canvas_width = 0_u32;
    let mut canvas_height = 0_u32;
    for value in intervals.iter().take(MAX_LAYERS) {
        // ARIB images are positioned within a defined caption plane.  Keeping
        // that plane is essential: a cropped bitmap may look acceptable at
        // 2K but moves and shrinks relative to a 4K/8K video surface.
        canvas_width = canvas_width.max(
            value
                .get("plane_width")
                .or_else(|| value.get("planeWidth"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0),
        );
        canvas_height = canvas_height.max(
            value
                .get("plane_height")
                .or_else(|| value.get("planeHeight"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0),
        );
        let Some(image) = value.get("rendered_image") else {
            continue;
        };
        let Some((pixels, width, height)) = decode_rendered_image(image) else {
            continue;
        };
        let x = image
            .get("dst_x")
            .or_else(|| image.get("dstX"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let y = image
            .get("dst_y")
            .or_else(|| image.get("dstY"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if x < 0 || y < 0 {
            continue;
        }
        let x = u32::try_from(x).ok()?;
        let y = u32::try_from(y).ok()?;
        canvas_width = canvas_width.max(x.saturating_add(width));
        canvas_height = canvas_height.max(y.saturating_add(height));
        layers.push((x, y, width, height, pixels));
    }
    if layers.is_empty() {
        return compose_ttml_horizontal(intervals);
    }
    if canvas_width == 0
        || canvas_height == 0
        || u64::from(canvas_width) * u64::from(canvas_height) > MAX_PIXELS
    {
        return None;
    }
    let mut canvas = vec![0_u8; canvas_width as usize * canvas_height as usize * 4];
    for (x, y, width, height, pixels) in &layers {
        blend_layer(
            &mut canvas,
            canvas_width,
            canvas_height,
            *x,
            *y,
            *width,
            *height,
            pixels,
        );
    }
    Some(CaptionPlaneFrame {
        width: canvas_width,
        height: canvas_height,
        pixels: canvas.into(),
        layer_count: layers.len(),
        mode: "b24-native-rgba",
        missing_glyph_count: 0,
        rendered_ruby_count: 0,
    })
}

fn compose_ttml_horizontal(intervals: &[Value]) -> Option<CaptionPlaneFrame> {
    let font = Font::from_bytes(
        include_bytes!("../../../third_party/rounded-mplus-1m-arib/rounded-mplus-1m-arib.ttf")
            as &[u8],
        FontSettings::default(),
    )
    .ok()?;
    let mut canvas = vec![0_u8; TTML_PLANE_WIDTH as usize * TTML_PLANE_HEIGHT as usize * 4];
    let mut layer_count = 0;
    let mut missing_glyph_count = 0;
    let mut has_vertical = false;
    let mut rendered_ruby_count = 0;
    let mut rendered_region_backgrounds = BTreeSet::new();
    for interval in intervals.iter().take(MAX_LAYERS) {
        let Some(text) = caption_text(interval) else {
            continue;
        };
        let style = interval.get("style").unwrap_or(interval);
        let vertical = style_value(style, "writing_mode")
            .or_else(|| style_value(style, "writingMode"))
            .is_some_and(|mode| mode.starts_with("vertical"));
        let (x, y) = interval_position(interval, style);
        let (width, height) = interval_extent(interval, style);
        let opacity = parse_opacity(style_value(style, "opacity"));
        let background = apply_opacity(
            parse_rgba(
                style_value(style, "background_color")
                    .or_else(|| style_value(style, "backgroundColor"))
                    .unwrap_or("#00000000"),
            ),
            opacity,
        );
        let background_scope = style_value(style, "background_scope")
            .or_else(|| style_value(style, "backgroundScope"));
        // TTML's two-axis fontSize is horizontal then vertical. The native
        // rasterizer is height-based, so `36px 72px` must remain 72px tall
        // instead of becoming a visibly undersized 36px glyph.
        let font_size = parse_font_height(
            style_value(style, "font_size").or_else(|| style_value(style, "fontSize")),
        )
        .unwrap_or(42.0)
        .clamp(8.0, 160.0);
        let letter_spacing = parse_px(
            style_value(style, "letter_spacing").or_else(|| style_value(style, "letterSpacing")),
        )
        .unwrap_or(0.0)
        .clamp(-20.0, 80.0);
        let foreground = apply_opacity(
            parse_rgba(style_value(style, "color").unwrap_or("#FFFFFFFF")),
            opacity,
        );
        let outline_value =
            style_value(style, "text_outline").or_else(|| style_value(style, "textOutline"));
        let outline = parse_text_outline(outline_value, opacity).or_else(|| {
            // ARIB-TTML broadcasts commonly rely on the receiver's rounded
            // caption presentation baseline rather than repeating an outline
            // declaration on every span. Keep an explicit `none` authoritative.
            (!outline_value.is_some_and(|value| value.trim().eq_ignore_ascii_case("none"))
                && is_arib_rounded_caption(interval, style))
            .then_some(TextOutline {
                radius: 2,
                color: [0, 0, 0, foreground[3]],
            })
        });
        let line_height = parse_px(
            style_value(style, "line_height").or_else(|| style_value(style, "lineHeight")),
        )
        .unwrap_or(font_size)
        .clamp(8.0, 240.0);
        let runs = styled_runs(
            interval,
            text,
            foreground,
            font_size,
            letter_spacing,
            outline,
            opacity,
        );
        if width > 0
            && height > 0
            && background[3] > 0
            && background_scope == Some("region")
            && rendered_region_backgrounds.insert((x, y, width, height, background))
        {
            fill_rect(
                &mut canvas,
                x,
                y,
                width,
                height,
                background,
            );
        }
        let drawn = if vertical {
            has_vertical = true;
            let (drawn, ruby_count) = draw_vertical_text(
                &mut canvas,
                &font,
                &runs,
                x,
                y,
                width,
                height,
                style_value(style, "writing_mode")
                    .or_else(|| style_value(style, "writingMode"))
                    .unwrap_or_default(),
                &mut missing_glyph_count,
            );
            rendered_ruby_count += ruby_count;
            drawn
        } else {
            let (drawn, ruby_count) = draw_horizontal_text(
                &mut canvas,
                &font,
                &runs,
                x,
                y,
                width,
                height,
                style_value(style, "direction").is_some_and(|direction| direction == "rtl"),
                style_value(style, "text_align").or_else(|| style_value(style, "textAlign")),
                style_value(style, "display_align").or_else(|| style_value(style, "displayAlign")),
                line_height,
                &mut missing_glyph_count,
            );
            rendered_ruby_count += ruby_count;
            drawn
        };
        if drawn {
            layer_count += 1;
        }
    }
    if layer_count == 0 {
        return None;
    }
    Some(CaptionPlaneFrame {
        width: TTML_PLANE_WIDTH,
        height: TTML_PLANE_HEIGHT,
        pixels: canvas.into(),
        layer_count,
        mode: if has_vertical && rendered_ruby_count > 0 {
            "ttml-vertical-ruby-basic-native"
        } else if has_vertical {
            "ttml-vertical-basic-native"
        } else if rendered_ruby_count > 0 {
            "ttml-horizontal-ruby-basic-native"
        } else {
            "ttml-horizontal-native"
        },
        missing_glyph_count,
        rendered_ruby_count,
    })
}

fn decode_rendered_image(image: &Value) -> Option<(Vec<u8>, u32, u32)> {
    if let Some(encoded) = image
        .get("rgba_base64")
        .or_else(|| image.get("rgbaBase64"))
        .and_then(Value::as_str)
    {
        if encoded.len() > MAX_IMAGE_BYTES.saturating_mul(4).div_ceil(3) {
            return None;
        }
        let width = image
            .get("width")?
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())?;
        let height = image
            .get("height")?
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())?;
        let packed_stride = u64::from(width).checked_mul(4)?;
        let stride = image
            .get("stride")
            .and_then(Value::as_u64)
            .unwrap_or(packed_stride);
        let expected = stride
            .checked_mul(u64::from(height))
            .and_then(|value| usize::try_from(value).ok())?;
        if width == 0 || height == 0 || stride < packed_stride || expected > MAX_IMAGE_BYTES {
            return None;
        }
        let rgba = BASE64.decode(encoded).ok()?;
        if rgba.len() < expected || rgba.len() > MAX_IMAGE_BYTES {
            return None;
        }
        let row_bytes = usize::try_from(packed_stride).ok()?;
        let stride = usize::try_from(stride).ok()?;
        let mut packed = Vec::with_capacity(row_bytes.checked_mul(height as usize)?);
        for row in 0..height as usize {
            let start = row.checked_mul(stride)?;
            packed.extend_from_slice(rgba.get(start..start + row_bytes)?);
        }
        return Some((packed, width, height));
    }
    let encoded = image
        .get("png_base64")
        .or_else(|| image.get("pngBase64"))
        .and_then(Value::as_str)?;
    let bytes = BASE64.decode(encoded).ok()?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }
    decode_png(&bytes)
}

mod layout;
#[cfg(test)]
use layout::{
    VerticalGlyphOrientation, horizontal_lines, rotate_bitmap_clockwise, text_combine_digit_count,
    vertical_glyph_orientation, vertical_presentation_form,
};
use layout::{draw_horizontal_text, draw_vertical_text};
mod rich_text;
use rich_text::styled_runs;
mod glyph;
use glyph::{draw_character, measure_text};
mod style;
use style::{
    apply_opacity, caption_text, interval_extent, interval_position, is_arib_rounded_caption,
    parse_font_height, parse_opacity, parse_px, parse_rgba, parse_text_outline, style_value,
};
mod bitmap;
use bitmap::{blend_glyph, blend_layer, decode_png, encode_png, fill_rect};
#[cfg(test)]
#[path = "caption_renderer/tests.rs"]
mod tests;
