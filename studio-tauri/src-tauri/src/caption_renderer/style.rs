use super::*;

pub(super) fn caption_text(value: &Value) -> Option<&str> {
    ["text", "caption", "content"]
        .into_iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
        .filter(|text| !text.trim().is_empty())
}

pub(super) fn style_value<'a>(style: &'a Value, key: &str) -> Option<&'a str> {
    style.get(key).and_then(Value::as_str)
}

pub(super) fn interval_position(interval: &Value, style: &Value) -> (i32, i32) {
    let x = interval
        .get("x")
        .or_else(|| interval.get("left"))
        .and_then(Value::as_i64)
        .or_else(|| {
            style_value(style, "origin").and_then(|value| parse_pair(value).map(|pair| pair.0))
        })
        .unwrap_or(0);
    let y = interval
        .get("y")
        .or_else(|| interval.get("top"))
        .and_then(Value::as_i64)
        .or_else(|| {
            style_value(style, "origin").and_then(|value| parse_pair(value).map(|pair| pair.1))
        })
        .unwrap_or(0);
    (
        x.clamp(0, TTML_PLANE_WIDTH as i64) as i32,
        y.clamp(0, TTML_PLANE_HEIGHT as i64) as i32,
    )
}

pub(super) fn interval_extent(interval: &Value, style: &Value) -> (i32, i32) {
    let width = interval
        .get("width")
        .and_then(Value::as_i64)
        .or_else(|| {
            style_value(style, "extent").and_then(|value| parse_pair(value).map(|pair| pair.0))
        })
        .unwrap_or(0);
    let height = interval
        .get("height")
        .and_then(Value::as_i64)
        .or_else(|| {
            style_value(style, "extent").and_then(|value| parse_pair(value).map(|pair| pair.1))
        })
        .unwrap_or(0);
    (
        width.clamp(0, TTML_PLANE_WIDTH as i64) as i32,
        height.clamp(0, TTML_PLANE_HEIGHT as i64) as i32,
    )
}

fn parse_pair(value: &str) -> Option<(i64, i64)> {
    let mut values = value
        .split_whitespace()
        .filter_map(|part| parse_px(Some(part)))
        .map(|value| value.round() as i64);
    Some((values.next()?, values.next()?))
}
pub(super) fn parse_px(value: Option<&str>) -> Option<f32> {
    value?
        .split_whitespace()
        .next()?
        .trim_end_matches("px")
        .parse()
        .ok()
}

pub(super) fn parse_font_height(value: Option<&str>) -> Option<f32> {
    let value = value?;
    let mut dimensions = value
        .split_whitespace()
        .filter_map(|part| parse_px(Some(part)));
    let horizontal = dimensions.next()?;
    Some(dimensions.next().unwrap_or(horizontal))
}

pub(super) fn is_arib_rounded_caption(interval: &Value, style: &Value) -> bool {
    let family = style_value(style, "font_family")
        .or_else(|| style_value(style, "fontFamily"))
        .or_else(|| {
            interval
                .get("rich_body")
                .or_else(|| interval.get("richBody"))
                .and_then(Value::as_str)
        })
        .unwrap_or_default();
    family.contains("丸ゴシック") || family.to_ascii_lowercase().contains("rounded m+")
}
pub(super) fn parse_rgba(value: &str) -> [u8; 4] {
    let value = value.trim();
    let named = match value.to_ascii_lowercase().as_str() {
        "black" => Some([0, 0, 0, 255]),
        "white" => Some([255, 255, 255, 255]),
        "red" => Some([255, 0, 0, 255]),
        "green" => Some([0, 255, 0, 255]),
        "blue" => Some([0, 0, 255, 255]),
        "yellow" => Some([255, 255, 0, 255]),
        "cyan" => Some([0, 255, 255, 255]),
        "magenta" => Some([255, 0, 255, 255]),
        "transparent" => Some([0, 0, 0, 0]),
        _ => None,
    };
    if let Some(named) = named {
        return named;
    }
    let hex = value.trim_start_matches('#');
    let byte =
        |offset| u8::from_str_radix(hex.get(offset..offset + 2).unwrap_or("00"), 16).unwrap_or(0);
    match hex.len() {
        6 => [byte(0), byte(2), byte(4), 255],
        8 => [byte(0), byte(2), byte(4), byte(6)],
        _ => [255, 255, 255, 255],
    }
}

pub(super) fn parse_opacity(value: Option<&str>) -> f32 {
    let Some(value) = value else { return 1.0 };
    let value = value.trim();
    let parsed = if let Some(percent) = value.strip_suffix('%') {
        percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|number| number / 100.0)
    } else {
        value.parse::<f32>().ok()
    };
    parsed.unwrap_or(1.0).clamp(0.0, 1.0)
}

pub(super) fn apply_opacity(mut color: [u8; 4], opacity: f32) -> [u8; 4] {
    color[3] = (f32::from(color[3]) * opacity.clamp(0.0, 1.0)).round() as u8;
    color
}

pub(super) fn parse_text_outline(value: Option<&str>, opacity: f32) -> Option<TextOutline> {
    let value = value?.trim();
    if value.eq_ignore_ascii_case("none") {
        return None;
    }
    let thickness = value
        .split_whitespace()
        .find_map(|part| part.strip_suffix("px")?.trim().parse::<f32>().ok())?;
    if thickness <= 0.0 {
        return None;
    }
    let radius = thickness
        .round()
        .clamp(1.0, 4.0) as i32;
    let color = value
        .split_whitespace()
        .find_map(parse_ttml_outline_color)?;
    Some(TextOutline {
        radius,
        color: apply_opacity(color, opacity),
    })
}

/// Keep the outline grammar deliberately narrower than CSS: TTML's named
/// colours and full RGB/RGBA hex are accepted, while malformed tokens remain
/// metadata rather than turning into an invented white outline.
fn parse_ttml_outline_color(value: &str) -> Option<[u8; 4]> {
    let normalized = value.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "black"
            | "white"
            | "red"
            | "green"
            | "blue"
            | "yellow"
            | "cyan"
            | "magenta"
            | "transparent"
    ) {
        return Some(parse_rgba(&normalized));
    }
    let hex = normalized.strip_prefix('#')?;
    (hex.len() == 6 || hex.len() == 8)
        .then_some(())
        .filter(|_| hex.chars().all(|character| character.is_ascii_hexdigit()))?;
    Some(parse_rgba(&normalized))
}
