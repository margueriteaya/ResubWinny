use crate::*;
use roxmltree::{Document, Node, NodeType};

pub(crate) fn attribute(tag: &str, name: &str) -> Option<String> {
    for quote in ['\"', '\''] {
        let marker = format!("{name}={quote}");
        if let Some(start) = tag.find(&marker).map(|index| index + marker.len())
            && let Some(end) = tag[start..].find(quote).map(|index| index + start)
        {
            return Some(tag[start..end].to_owned());
        }
    }
    None
}

pub(crate) fn ttml_time_ms(value: &str) -> Option<i64> {
    let value = value.trim();
    for (suffix, multiplier) in [
        ("ms", 1.0),
        ("h", 3_600_000.0),
        ("m", 60_000.0),
        ("s", 1_000.0),
    ] {
        if let Some(number) = value.strip_suffix(suffix) {
            let milliseconds = number.trim().parse::<f64>().ok()? * multiplier;
            return milliseconds
                .is_finite()
                .then_some(milliseconds.round() as i64);
        }
    }
    let (hours, rest) = value.split_once(':')?;
    let (minutes, seconds) = rest.split_once(':')?;
    let seconds = seconds.parse::<f64>().ok()?;
    Some(
        ((hours.parse::<f64>().ok()? * 3600.0 + minutes.parse::<f64>().ok()? * 60.0 + seconds)
            * 1000.0)
            .round() as i64,
    )
}

pub(crate) fn ttml_plain_text(value: &str) -> String {
    let mut output = value.replace("<br/>", "\n").replace("<br />", "\n");
    let mut stripped = String::new();
    let mut inside_tag = false;
    for ch in output.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => stripped.push(ch),
            _ => {}
        }
    }
    output = stripped;
    output
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_owned()
}

pub(crate) fn safe_ttml_inline_body(value: &str) -> Option<String> {
    let mut remaining = value;
    let mut stack = Vec::new();
    while let Some(start) = remaining.find('<') {
        let end = remaining[start..].find('>')?;
        let tag = remaining[start + 1..start + end].trim();
        if tag.starts_with('!') || tag.starts_with('?') {
            return None;
        }
        let closing = tag.starts_with('/');
        let name = tag
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split_whitespace()
            .next()?;
        if !matches!(name, "span" | "ruby" | "rt" | "br") {
            return None;
        }
        let self_closing = tag.ends_with('/') || name == "br";
        if closing {
            if stack.pop().as_deref() != Some(name) {
                return None;
            }
        } else if !self_closing {
            stack.push(name.to_owned());
        }
        remaining = &remaining[start + end + 1..];
    }
    stack.is_empty().then(|| value.to_owned())
}

pub(crate) fn remove_xml_attribute(tag: &str, name: &str) -> String {
    for quote in ['"', '\''] {
        let marker = format!("{name}={quote}");
        if let Some(attribute_start) = tag.find(&marker) {
            let start = tag[..attribute_start]
                .rfind(char::is_whitespace)
                .unwrap_or(attribute_start);
            let value_start = attribute_start + marker.len();
            if let Some(value_end) = tag[value_start..].find(quote) {
                let end = value_start + value_end + 1;
                return format!("{}{}", &tag[..start], &tag[end..]);
            }
        }
    }
    tag.to_owned()
}

pub(crate) fn ttml_style_attributes(style: &TtmlCaptionStyle) -> String {
    let mut output = String::new();
    for (name, value) in [
        ("color", style.color.as_deref()),
        ("backgroundColor", style.background_color.as_deref()),
        ("fontSize", style.font_size.as_deref()),
        ("fontFamily", style.font_family.as_deref()),
        ("fontStyle", style.font_style.as_deref()),
        ("fontWeight", style.font_weight.as_deref()),
        ("writingMode", style.writing_mode.as_deref()),
        ("textAlign", style.text_align.as_deref()),
        ("textOutline", style.text_outline.as_deref()),
        ("lineHeight", style.line_height.as_deref()),
        ("letterSpacing", style.letter_spacing.as_deref()),
        ("opacity", style.opacity.as_deref()),
        ("displayAlign", style.display_align.as_deref()),
    ] {
        if let Some(value) = value {
            output.push_str(&format!(" tts:{name}=\"{}\"", xml_escape(value)));
        }
    }
    output
}

pub(crate) fn expand_ttml_inline_style_references(
    body: &str,
    definitions: &BTreeMap<String, TtmlCaptionStyle>,
) -> String {
    let mut output = String::new();
    let mut remaining = body;
    while let Some(start) = remaining.find("<span") {
        output.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        let Some(end) = remaining.find('>') else {
            output.push_str(remaining);
            break;
        };
        let tag = &remaining[..end + 1];
        let mut style = TtmlCaptionStyle::default();
        ttml_apply_style(&mut style, tag, definitions);
        let preserved = remove_xml_attribute(tag, "style");
        let closing = if preserved.ends_with("/>") { "/>" } else { ">" };
        let opening = preserved.trim_end_matches(closing);
        output.push_str(opening);
        output.push_str(&ttml_style_attributes(&style));
        output.push_str(closing);
        remaining = &remaining[end + 1..];
    }
    output.push_str(remaining);
    output
}

pub(crate) fn ttml_first_span_tag(body: &str) -> Option<&str> {
    let start = body.find("<span")?;
    let end = body[start..].find('>')?;
    Some(&body[start..start + end + 1])
}

pub(crate) fn ttml_tag_with_xml_id<'a>(xml: &'a str, element: &str, id: &str) -> Option<&'a str> {
    let marker = format!("<{element}");
    let mut remaining = xml;
    while let Some(offset) = remaining.find(&marker) {
        remaining = &remaining[offset..];
        let end = remaining.find('>')?;
        let tag = &remaining[..end + 1];
        if attribute(tag, "xml:id").as_deref() == Some(id) {
            return Some(tag);
        }
        remaining = &remaining[end + 1..];
    }
    None
}

const LOGICAL_DISPLAY_WIDTH: i32 = 1920;
const LOGICAL_DISPLAY_HEIGHT: i32 = 1080;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TtmlDisplayPlane {
    source_width: i32,
    source_height: i32,
    basis: TtmlSourcePlaneBasis,
}

impl TtmlDisplayPlane {
    const fn logical() -> Self {
        Self {
            source_width: LOGICAL_DISPLAY_WIDTH,
            source_height: LOGICAL_DISPLAY_HEIGHT,
            basis: TtmlSourcePlaneBasis::LegacyLogical2k,
        }
    }

    fn horizontal_scale(self) -> f64 {
        f64::from(LOGICAL_DISPLAY_WIDTH) / f64::from(self.source_width)
    }

    fn vertical_scale(self) -> f64 {
        f64::from(LOGICAL_DISPLAY_HEIGHT) / f64::from(self.source_height)
    }

    fn text_scale(self) -> f64 {
        self.horizontal_scale().min(self.vertical_scale())
    }
}

fn ttml_root_tag(xml: &str) -> Option<&str> {
    let mut remaining = xml;
    while let Some(offset) = remaining.find("<tt") {
        remaining = &remaining[offset..];
        let boundary = remaining.as_bytes().get(3).copied();
        if boundary.is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/')) {
            let end = remaining.find('>')?;
            return Some(&remaining[..=end]);
        }
        remaining = &remaining[3..];
    }
    None
}

fn ttml_display_plane(xml: &str) -> TtmlDisplayPlane {
    ttml_display_plane_with_root(xml, ttml_root_tag(xml))
}

fn ttml_display_plane_with_root(xml: &str, root: Option<&str>) -> TtmlDisplayPlane {
    let Some(root) = root else {
        return TtmlDisplayPlane::logical();
    };
    if let Some(extent) = attribute(root, "tts:extent").or_else(|| attribute(root, "extent")) {
        let mut values = extent.split_whitespace();
        let width = values.next().and_then(ttml_pixel_length);
        let height = values.next().and_then(ttml_pixel_length);
        return match (width, height) {
            (Some(width), Some(height)) if width > 0 && height > 0 => TtmlDisplayPlane {
                source_width: width,
                source_height: height,
                basis: TtmlSourcePlaneBasis::Declared,
            },
            _ => TtmlDisplayPlane::logical(),
        };
    }
    inferred_ttml_display_plane(xml).unwrap_or_else(TtmlDisplayPlane::logical)
}

/// Some recorded ARIB-TTML documents omit the root display extent while still
/// using a canonical 4K/8K pixel coordinate space. Region extent is layout
/// capacity and may be clipped beyond the display edge, so `origin + extent`
/// is only a gross sanity bound; it must not promote a 4K document to 8K.
fn inferred_ttml_display_plane(xml: &str) -> Option<TtmlDisplayPlane> {
    let mut max_horizontal_component = 0_i32;
    let mut max_vertical_component = 0_i32;
    let mut max_right = 0_i32;
    let mut max_bottom = 0_i32;
    let mut remaining = xml;
    while let Some(start) = remaining.find('<') {
        remaining = &remaining[start..];
        let end = remaining.find('>')?;
        let tag = &remaining[..=end];
        remaining = &remaining[end + 1..];
        let (Some(origin), Some(extent)) =
            (attribute(tag, "tts:origin"), attribute(tag, "tts:extent"))
        else {
            // XML declarations, the root, and non-layout tags are not evidence
            // about the display plane. Continue until a complete geometry pair.
            continue;
        };
        let mut origin_values = origin.split_whitespace();
        let mut extent_values = extent.split_whitespace();
        let (Some(x), Some(y), Some(width), Some(height)) = (
            origin_values.next().and_then(ttml_nonnegative_pixel_length),
            origin_values.next().and_then(ttml_nonnegative_pixel_length),
            extent_values.next().and_then(ttml_pixel_length),
            extent_values.next().and_then(ttml_pixel_length),
        ) else {
            continue;
        };
        max_horizontal_component = max_horizontal_component.max(x).max(width);
        max_vertical_component = max_vertical_component.max(y).max(height);
        max_right = max_right.max(x.saturating_add(width));
        max_bottom = max_bottom.max(y.saturating_add(height));
    }
    if max_horizontal_component <= LOGICAL_DISPLAY_WIDTH
        && max_vertical_component <= LOGICAL_DISPLAY_HEIGHT
    {
        return None;
    }
    if max_right > 7680 || max_bottom > 4320 {
        return None;
    }
    [(3840, 2160), (7680, 4320)]
        .into_iter()
        .find(|(width, height)| {
            max_horizontal_component <= *width && max_vertical_component <= *height
        })
        .map(|(source_width, source_height)| TtmlDisplayPlane {
            source_width,
            source_height,
            basis: TtmlSourcePlaneBasis::Inferred,
        })
}

fn ttml_nonnegative_pixel_length(value: &str) -> Option<i32> {
    let pixels = value
        .trim()
        .strip_suffix("px")?
        .trim()
        .parse::<f64>()
        .ok()?;
    (pixels.is_finite() && pixels >= 0.0 && pixels <= f64::from(i32::MAX))
        .then_some(pixels.round() as i32)
}

fn ttml_pixel_length(value: &str) -> Option<i32> {
    let pixels = value
        .trim()
        .strip_suffix("px")?
        .trim()
        .parse::<f64>()
        .ok()?;
    (pixels.is_finite() && pixels > 0.0 && pixels <= f64::from(i32::MAX))
        .then_some(pixels.round() as i32)
}

pub(crate) fn ttml_coordinate(value: &str, source_extent: i32, scale: f64) -> Option<i32> {
    let value = value.trim();
    let source_coordinate = if let Some(percent) = value.strip_suffix('%') {
        percent.trim().parse::<f64>().ok()? * f64::from(source_extent) / 100.0
    } else {
        value.trim_end_matches("px").trim().parse::<f64>().ok()?
    };
    let coordinate = source_coordinate * scale;
    (coordinate.is_finite()
        && coordinate >= f64::from(i32::MIN)
        && coordinate <= f64::from(i32::MAX))
    .then_some(coordinate.round() as i32)
}

pub(crate) fn ttml_region_geometry(
    region_tag: Option<&str>,
    plane: TtmlDisplayPlane,
) -> (i32, i32, Option<i32>, Option<i32>) {
    let Some(tag) = region_tag else {
        return (960, 920, None, None);
    };
    let origin_value = attribute(tag, "tts:origin").unwrap_or_default();
    let mut origin = origin_value.split_whitespace();
    let x = origin
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_width, plane.horizontal_scale()))
        .unwrap_or(960);
    let y = origin
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_height, plane.vertical_scale()))
        .unwrap_or(920);
    let extent_value = attribute(tag, "tts:extent").unwrap_or_default();
    let mut extent = extent_value.split_whitespace();
    let width = extent
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_width, plane.horizontal_scale()));
    let height = extent
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_height, plane.vertical_scale()));
    (x, y, width, height)
}

fn ttml_source_region_geometry(
    region_tag: Option<&str>,
    plane: TtmlDisplayPlane,
) -> (i32, i32, Option<i32>, Option<i32>) {
    let Some(tag) = region_tag else {
        return (
            plane.source_width / 2,
            (f64::from(plane.source_height) * 920.0 / 1080.0).round() as i32,
            None,
            None,
        );
    };
    let origin_value = attribute(tag, "tts:origin").unwrap_or_default();
    let mut origin = origin_value.split_whitespace();
    let x = origin
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_width, 1.0))
        .unwrap_or(plane.source_width / 2);
    let y = origin
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_height, 1.0))
        .unwrap_or_else(|| (f64::from(plane.source_height) * 920.0 / 1080.0).round() as i32);
    let extent_value = attribute(tag, "tts:extent").unwrap_or_default();
    let mut extent = extent_value.split_whitespace();
    let width = extent
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_width, 1.0));
    let height = extent
        .next()
        .and_then(|value| ttml_coordinate(value, plane.source_height, 1.0));
    (x, y, width, height)
}

fn format_ttml_length(value: f64) -> String {
    if (value - value.round()).abs() < 0.001 {
        format!("{}px", value.round() as i32)
    } else {
        format!("{value:.3}px")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

fn scale_ttml_px_tokens(value: &str, scale: f64) -> String {
    value
        .split_whitespace()
        .map(|token| {
            token
                .strip_suffix("px")
                .and_then(|number| number.parse::<f64>().ok())
                .filter(|number| number.is_finite())
                .map(|number| format_ttml_length(number * scale))
                .unwrap_or_else(|| token.to_owned())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalise_ttml_style_lengths(style: &mut TtmlCaptionStyle, plane: TtmlDisplayPlane) {
    let scale = plane.text_scale();
    if (scale - 1.0).abs() < f64::EPSILON {
        return;
    }
    for value in [
        &mut style.font_size,
        &mut style.line_height,
        &mut style.letter_spacing,
        &mut style.text_outline,
    ]
    .into_iter()
    .flatten()
    {
        *value = scale_ttml_px_tokens(value, scale);
    }
}

fn normalise_ttml_inline_length_attributes(body: String, plane: TtmlDisplayPlane) -> String {
    let scale = plane.text_scale();
    if (scale - 1.0).abs() < f64::EPSILON {
        return body;
    }
    let mut output = body;
    for name in [
        "tts:fontSize",
        "tts:lineHeight",
        "tts:letterSpacing",
        "arib-tt:letter-spacing",
        "tts:textOutline",
    ] {
        for quote in ['\"', '\''] {
            let marker = format!("{name}={quote}");
            let mut cursor = 0;
            while let Some(found) = output[cursor..].find(&marker) {
                let value_start = cursor + found + marker.len();
                let Some(value_length) = output[value_start..].find(quote) else {
                    break;
                };
                let value_end = value_start + value_length;
                let scaled = scale_ttml_px_tokens(&output[value_start..value_end], scale);
                output.replace_range(value_start..value_end, &scaled);
                cursor = value_start + scaled.len() + 1;
            }
        }
    }
    output
}

pub(crate) fn ttml_inline_style(tag: &str) -> TtmlCaptionStyle {
    let writing_mode = attribute(tag, "tts:writingMode");
    let direction =
        attribute(tag, "tts:direction").or_else(|| match writing_mode.as_deref().map(str::trim) {
            Some("lrtb") => Some("ltr".to_owned()),
            Some("rltb") => Some("rtl".to_owned()),
            _ => None,
        });
    TtmlCaptionStyle {
        color: attribute(tag, "tts:color"),
        background_color: attribute(tag, "tts:backgroundColor"),
        background_scope: None,
        font_size: attribute(tag, "tts:fontSize"),
        font_family: attribute(tag, "tts:fontFamily"),
        font_style: attribute(tag, "tts:fontStyle"),
        font_weight: attribute(tag, "tts:fontWeight"),
        writing_mode: writing_mode.map(|value| canonical_writing_mode(&value)),
        direction,
        text_align: attribute(tag, "tts:textAlign"),
        text_outline: attribute(tag, "tts:textOutline"),
        line_height: attribute(tag, "tts:lineHeight"),
        letter_spacing: attribute(tag, "tts:letterSpacing")
            .or_else(|| attribute(tag, "arib-tt:letter-spacing")),
        opacity: attribute(tag, "tts:opacity"),
        display_align: attribute(tag, "tts:displayAlign"),
        background_image: attribute(tag, "smpte:backgroundImage")
            .or_else(|| attribute(tag, "backgroundImage")),
        font_resource: attribute(tag, "arib-tt:font-face"),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn canonical_writing_mode(value: &str) -> String {
    match value.trim() {
        "lrtb" | "horizontal-tb" => "horizontal-tb".into(),
        "rltb" => "horizontal-tb".into(),
        "tblr" | "vertical-lr" => "vertical-lr".into(),
        "tbrl" | "vertical-rl" => "vertical-rl".into(),
        other => other.to_owned(),
    }
}

pub(crate) fn merge_ttml_style(into: &mut TtmlCaptionStyle, next: &TtmlCaptionStyle) {
    if next.color.is_some() {
        into.color.clone_from(&next.color);
    }
    if next.background_color.is_some() {
        into.background_color.clone_from(&next.background_color);
    }
    if next.background_scope.is_some() {
        into.background_scope = next.background_scope;
    }
    if next.font_size.is_some() {
        into.font_size.clone_from(&next.font_size);
    }
    if next.font_family.is_some() {
        into.font_family.clone_from(&next.font_family);
    }
    if next.font_style.is_some() {
        into.font_style.clone_from(&next.font_style);
    }
    if next.font_weight.is_some() {
        into.font_weight.clone_from(&next.font_weight);
    }
    if next.writing_mode.is_some() {
        into.writing_mode.clone_from(&next.writing_mode);
    }
    if next.direction.is_some() {
        into.direction.clone_from(&next.direction);
    }
    if next.text_align.is_some() {
        into.text_align.clone_from(&next.text_align);
    }
    if next.text_outline.is_some() {
        into.text_outline.clone_from(&next.text_outline);
    }
    if next.line_height.is_some() {
        into.line_height.clone_from(&next.line_height);
    }
    if next.letter_spacing.is_some() {
        into.letter_spacing.clone_from(&next.letter_spacing);
    }
    if next.opacity.is_some() {
        into.opacity.clone_from(&next.opacity);
    }
    if next.display_align.is_some() {
        into.display_align.clone_from(&next.display_align);
    }
    if next.background_image.is_some() {
        into.background_image.clone_from(&next.background_image);
    }
    if next.font_resource.is_some() {
        into.font_resource.clone_from(&next.font_resource);
    }
}

pub(crate) fn ttml_style_definitions(xml: &str) -> BTreeMap<String, TtmlCaptionStyle> {
    let mut definitions = BTreeMap::new();
    let mut remaining = xml;
    while let Some(offset) = remaining.find("<style") {
        remaining = &remaining[offset..];
        let Some(end) = remaining.find('>') else {
            break;
        };
        let tag = &remaining[..end + 1];
        if let Some(id) = attribute(tag, "xml:id") {
            definitions.insert(id, ttml_inline_style(tag));
        }
        remaining = &remaining[end + 1..];
    }
    definitions
}

pub(crate) fn ttml_apply_style(
    resolved: &mut TtmlCaptionStyle,
    source: &str,
    definitions: &BTreeMap<String, TtmlCaptionStyle>,
) {
    if let Some(references) = attribute(source, "style") {
        for reference in references.split_whitespace() {
            if let Some(style) = definitions.get(reference) {
                merge_ttml_style(resolved, style);
            }
        }
    }
    merge_ttml_style(resolved, &ttml_inline_style(source));
}

fn ttml_apply_scoped_style(
    resolved: &mut TtmlCaptionStyle,
    source: &str,
    definitions: &BTreeMap<String, TtmlCaptionStyle>,
    background_scope: TtmlBackgroundScope,
) {
    let mut element = TtmlCaptionStyle::default();
    ttml_apply_style(&mut element, source, definitions);
    if element.background_color.is_some() {
        element.background_scope = Some(background_scope);
    }
    merge_ttml_style(resolved, &element);
}

pub(crate) fn ttml_resolved_style(
    tag: &str,
    parents: &[&str],
    region: Option<&str>,
    definitions: &BTreeMap<String, TtmlCaptionStyle>,
) -> TtmlCaptionStyle {
    let mut resolved = TtmlCaptionStyle::default();
    for parent in parents {
        ttml_apply_scoped_style(
            &mut resolved,
            parent,
            definitions,
            TtmlBackgroundScope::Block,
        );
    }
    if let Some(region) = region {
        ttml_apply_scoped_style(
            &mut resolved,
            region,
            definitions,
            TtmlBackgroundScope::Region,
        );
    }
    ttml_apply_scoped_style(&mut resolved, tag, definitions, TtmlBackgroundScope::Block);
    resolved
}

/// Return the `<div>` elements still open immediately before `offset`.
/// This deliberately small XML walk avoids treating a simple `rfind("<div")`
/// as ancestry: nested time containers and styles are common in ARIB-TTML,
/// and a preceding closed sibling must not influence the next `<p>`.
pub(crate) fn ttml_open_div_stack(xml: &str, offset: usize) -> Vec<&str> {
    let mut stack = Vec::new();
    let mut remaining = &xml[..offset.min(xml.len())];
    while let Some(start) = remaining.find('<') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('>') else {
            break;
        };
        let tag = &remaining[start..start + end + 2];
        let contents = tag[1..tag.len() - 1].trim();
        if contents == "/div" || contents.starts_with("/div ") {
            stack.pop();
        } else if (contents == "div" || contents.starts_with("div ")) && !contents.ends_with('/') {
            stack.push(tag);
        }
        remaining = &after_start[end + 1..];
    }
    stack
}

pub(crate) fn ttml_parent_timing(parents: &[&str]) -> (i64, Option<i64>) {
    let mut begin = 0_i64;
    let mut enclosing_end = None;
    for parent in parents {
        let parent_base = begin;
        begin = begin.saturating_add(
            attribute(parent, "begin")
                .and_then(|value| ttml_time_ms(&value))
                .unwrap_or(0),
        );
        let own_end = attribute(parent, "end")
            .and_then(|value| ttml_time_ms(&value))
            .map(|end| parent_base.saturating_add(end));
        let own_duration = attribute(parent, "dur")
            .and_then(|value| ttml_time_ms(&value))
            .map(|duration| begin.saturating_add(duration));
        if let Some(end) = own_end.or(own_duration) {
            enclosing_end = Some(enclosing_end.map_or(end, |previous: i64| previous.min(end)));
        }
    }
    (begin, enclosing_end)
}

pub(crate) fn parse_ttml_captions(xml: &str, base_pts_ms: i64) -> Vec<TtmlCaption> {
    parse_ttml_captions_until(xml, base_pts_ms, None)
}

fn ttml_node_opening_tag<'input>(xml: &'input str, node: Node<'_, 'input>) -> Option<&'input str> {
    let range = node.range();
    let start = range.start;
    let end = xml.get(start..range.end)?.find('>')? + start;
    xml.get(start..=end)
}

fn ttml_node_inner_xml<'input>(xml: &'input str, node: Node<'_, 'input>) -> Option<&'input str> {
    let range = node.range();
    let value = xml.get(range.clone())?;
    let start = value.find('>')? + 1;
    let end = value.rfind('<')?;
    (end >= start).then(|| &value[start..end])
}

fn ttml_node_attribute<'input>(node: Node<'input, 'input>, name: &str) -> Option<&'input str> {
    let local_name = name.rsplit(':').next().unwrap_or(name);
    node.attributes()
        .find(|attribute| attribute.name() == local_name)
        .map(|attribute| attribute.value())
}

fn append_ttml_node_text(node: Node<'_, '_>, output: &mut String) {
    match node.node_type() {
        NodeType::Text => output.push_str(node.text().unwrap_or_default()),
        NodeType::Element if node.tag_name().name() == "br" => output.push('\n'),
        _ => {
            for child in node.children() {
                append_ttml_node_text(child, output);
            }
        }
    }
}

fn ttml_node_plain_text(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    append_ttml_node_text(node, &mut output);
    output.trim().to_owned()
}

fn ttml_style_definitions_from_document<'input>(
    xml: &'input str,
    document: &Document<'input>,
) -> BTreeMap<String, TtmlCaptionStyle> {
    let mut definitions = BTreeMap::new();
    for node in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "style")
    {
        let (Some(id), Some(tag)) = (
            ttml_node_attribute(node, "xml:id"),
            ttml_node_opening_tag(xml, node),
        ) else {
            continue;
        };
        definitions.insert(id.to_owned(), ttml_inline_style(tag));
    }
    definitions
}

pub(crate) fn ttml_document_has_paragraph(xml: &str) -> bool {
    match Document::parse(xml) {
        Ok(document) => document
            .descendants()
            .any(|node| node.is_element() && node.tag_name().name() == "p"),
        Err(_) => xml.contains("<p ") || xml.contains("<p>"),
    }
}

/// Parse one TTML document. ARIB-TTML carried as a sequential document stream
/// may omit element timing; in that case `document_end_ms` is the timestamp of
/// the next complete document on the same component/PID.
pub(crate) fn parse_ttml_captions_until(
    xml: &str,
    base_pts_ms: i64,
    document_end_ms: Option<i64>,
) -> Vec<TtmlCaption> {
    let mut captions = Vec::new();
    let Ok(document) = Document::parse(xml) else {
        // Older recorder exports and a number of preserved fixtures use TTML
        // prefixes without declaring their namespace. They are not XML
        // namespace-conformant, so keep the bounded legacy reader isolated as
        // a compatibility route. Conformant documents always use the DOM path.
        return parse_ttml_captions_legacy(xml, base_pts_ms, document_end_ms);
    };
    let root_tag = ttml_node_opening_tag(xml, document.root_element());
    let style_definitions = ttml_style_definitions_from_document(xml, &document);
    let display_plane = ttml_display_plane_with_root(xml, root_tag);
    for paragraph in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "p")
    {
        let Some(tag) = ttml_node_opening_tag(xml, paragraph) else {
            continue;
        };
        let body = ttml_node_inner_xml(xml, paragraph).unwrap_or_default();
        let mut parent_divs = paragraph
            .ancestors()
            .filter(|node| node.is_element() && node.tag_name().name() == "div")
            .filter_map(|node| ttml_node_opening_tag(xml, node))
            .collect::<Vec<_>>();
        parent_divs.reverse();
        let (parent_begin, parent_end) = ttml_parent_timing(&parent_divs);
        let own_begin = attribute(tag, "begin").and_then(|value| ttml_time_ms(&value));
        let start = Some(parent_begin.saturating_add(own_begin.unwrap_or(0)));
        let own_end = attribute(tag, "end").and_then(|value| ttml_time_ms(&value));
        let duration = attribute(tag, "dur").and_then(|value| ttml_time_ms(&value));
        let end = own_end
            .map(|end| parent_begin.saturating_add(end))
            .or_else(|| {
                start
                    .zip(duration)
                    .map(|(start, duration)| start.saturating_add(duration))
            })
            .or(parent_end)
            .or_else(|| document_end_ms.map(|end| end.saturating_sub(base_pts_ms)));
        let text = ttml_node_plain_text(paragraph);
        if let (Some(start), Some(end)) = (start, end)
            && !text.is_empty()
            && base_pts_ms.saturating_add(end) > base_pts_ms.saturating_add(start)
        {
            let region = attribute(tag, "region")
                .or_else(|| {
                    parent_divs
                        .iter()
                        .rev()
                        .find_map(|div| attribute(div, "region"))
                })
                .unwrap_or_default();
            let region_tag = document
                .descendants()
                .find(|node| {
                    node.is_element()
                        && node.tag_name().name() == "region"
                        && ttml_node_attribute(*node, "xml:id") == Some(region.as_str())
                })
                .and_then(|node| ttml_node_opening_tag(xml, node));
            let (x, y, width, height) = ttml_region_geometry(region_tag, display_plane);
            let mut style = ttml_resolved_style(tag, &parent_divs, region_tag, &style_definitions);
            // A single wrapper span can safely provide the caption-level
            // presentation style. With multiple spans the styles belong to
            // individual runs and promoting the first one would bleed its
            // colour/size into every following run and ruby annotation.
            if body.matches("<span").count() == 1
                && let Some(span_tag) = ttml_first_span_tag(body)
            {
                ttml_apply_scoped_style(
                    &mut style,
                    span_tag,
                    &style_definitions,
                    TtmlBackgroundScope::Inline,
                );
            }
            let source_style = style.clone();
            let source_rich_body = safe_ttml_inline_body(body)
                .map(|safe| expand_ttml_inline_style_references(&safe, &style_definitions));
            let (source_x, source_y, source_width, source_height) =
                ttml_source_region_geometry(region_tag, display_plane);
            normalise_ttml_style_lengths(&mut style, display_plane);
            let rich_body = source_rich_body
                .clone()
                .map(|body| normalise_ttml_inline_length_attributes(body, display_plane));
            let ruby_writing_mode = match style.writing_mode.as_deref() {
                Some("vertical-lr" | "tblr") => RubyWritingMode::VerticalLr,
                Some("vertical-rl" | "tbrl") => RubyWritingMode::VerticalRl,
                _ => RubyWritingMode::HorizontalTb,
            };
            let ruby_bindings = rich_body
                .as_deref()
                .map(|body| {
                    ttml_ruby_bindings(&parse_ttml_inline_runs(body, &style), ruby_writing_mode)
                })
                .unwrap_or_default();
            captions.push(TtmlCaption {
                start_ms: base_pts_ms + start,
                end_ms: base_pts_ms + end,
                text,
                x,
                y,
                width,
                height,
                style,
                rich_body,
                ruby_bindings,
                source_layout: Some(TtmlSourceLayout {
                    plane_width: display_plane.source_width,
                    plane_height: display_plane.source_height,
                    plane_basis: display_plane.basis,
                    x: source_x,
                    y: source_y,
                    width: source_width,
                    height: source_height,
                    style: source_style,
                    rich_body: source_rich_body,
                }),
                source: None,
            });
        }
    }
    captions
}

fn parse_ttml_captions_legacy(
    xml: &str,
    base_pts_ms: i64,
    document_end_ms: Option<i64>,
) -> Vec<TtmlCaption> {
    let mut captions = Vec::new();
    let style_definitions = ttml_style_definitions(xml);
    let display_plane = ttml_display_plane(xml);
    let mut remaining = xml;
    while let Some(offset) = remaining.find("<p") {
        let absolute_offset = xml.len().saturating_sub(remaining.len()) + offset;
        remaining = &remaining[offset..];
        let Some(tag_end) = remaining.find('>') else {
            break;
        };
        let tag = &remaining[..tag_end + 1];
        let Some(close) = remaining[tag_end + 1..].find("</p>") else {
            break;
        };
        let body_end = tag_end + 1 + close;
        let body = &remaining[tag_end + 1..body_end];
        let parent_divs = ttml_open_div_stack(xml, absolute_offset);
        let (parent_begin, parent_end) = ttml_parent_timing(&parent_divs);
        let own_begin = attribute(tag, "begin").and_then(|value| ttml_time_ms(&value));
        let start = parent_begin.saturating_add(own_begin.unwrap_or(0));
        let own_end = attribute(tag, "end").and_then(|value| ttml_time_ms(&value));
        let duration = attribute(tag, "dur").and_then(|value| ttml_time_ms(&value));
        let end = own_end
            .map(|end| parent_begin.saturating_add(end))
            .or_else(|| duration.map(|duration| start.saturating_add(duration)))
            .or(parent_end)
            .or_else(|| document_end_ms.map(|end| end.saturating_sub(base_pts_ms)));
        let text = ttml_plain_text(body);
        if let Some(end) = end
            && !text.is_empty()
            && end > start
        {
            let region = attribute(tag, "region")
                .or_else(|| {
                    parent_divs
                        .iter()
                        .rev()
                        .find_map(|div| attribute(div, "region"))
                })
                .unwrap_or_default();
            let region_tag = ttml_tag_with_xml_id(xml, "region", &region);
            let (x, y, width, height) = ttml_region_geometry(region_tag, display_plane);
            let mut style = ttml_resolved_style(tag, &parent_divs, region_tag, &style_definitions);
            if body.matches("<span").count() == 1
                && let Some(span_tag) = ttml_first_span_tag(body)
            {
                ttml_apply_scoped_style(
                    &mut style,
                    span_tag,
                    &style_definitions,
                    TtmlBackgroundScope::Inline,
                );
            }
            let source_style = style.clone();
            let source_rich_body = safe_ttml_inline_body(body)
                .map(|safe| expand_ttml_inline_style_references(&safe, &style_definitions));
            let (source_x, source_y, source_width, source_height) =
                ttml_source_region_geometry(region_tag, display_plane);
            normalise_ttml_style_lengths(&mut style, display_plane);
            let rich_body = source_rich_body
                .clone()
                .map(|body| normalise_ttml_inline_length_attributes(body, display_plane));
            let ruby_writing_mode = match style.writing_mode.as_deref() {
                Some("vertical-lr" | "tblr") => RubyWritingMode::VerticalLr,
                Some("vertical-rl" | "tbrl") => RubyWritingMode::VerticalRl,
                _ => RubyWritingMode::HorizontalTb,
            };
            let ruby_bindings = rich_body
                .as_deref()
                .map(|body| {
                    ttml_ruby_bindings(&parse_ttml_inline_runs(body, &style), ruby_writing_mode)
                })
                .unwrap_or_default();
            captions.push(TtmlCaption {
                start_ms: base_pts_ms.saturating_add(start),
                end_ms: base_pts_ms.saturating_add(end),
                text,
                x,
                y,
                width,
                height,
                style,
                rich_body,
                ruby_bindings,
                source_layout: Some(TtmlSourceLayout {
                    plane_width: display_plane.source_width,
                    plane_height: display_plane.source_height,
                    plane_basis: display_plane.basis,
                    x: source_x,
                    y: source_y,
                    width: source_width,
                    height: source_height,
                    style: source_style,
                    rich_body: source_rich_body,
                }),
                source: None,
            });
        }
        remaining = &remaining[body_end + 4..];
    }
    captions
}

mod document;
pub(crate) use document::*;
mod scan;
pub(crate) use scan::*;
pub(crate) fn flush_b24_pes(
    pes: &mut Vec<u8>,
    decoder: &mut native_b24::NativeB24Decoder,
    last_pts: &mut i64,
    timeline_origin_ms: &mut Option<i64>,
    summary: &mut B24DecodeSummary,
) -> Option<native_b24::CaptionScene> {
    let Some((payload, pts)) = b24_payload_from_pes(pes) else {
        pes.clear();
        return None;
    };
    if let Some(pts) = pts {
        let pts = pts.to_millis();
        let origin = *timeline_origin_ms.get_or_insert(pts);
        *last_pts = normalise_pts(pts, origin);
    }
    summary.pes_packets += 1;
    let result = decoder.feed(payload, *last_pts);
    match result.status {
        2 => {
            summary.captions += 1;
            if let Some(scene) = result.scene {
                summary.regions += scene.regions.len() as u64;
                summary.characters += scene.characters.len() as u64;
                summary.drcs_glyphs += scene.drcs_glyphs.len() as u64;
                pes.clear();
                return Some(scene);
            } else {
                summary.decoder_errors += 1;
            }
        }
        0 => summary.decoder_errors += 1,
        _ => {}
    }
    pes.clear();
    None
}

pub struct ConversionReport {
    pub output: PathBuf,
    pub ass: Option<PathBuf>,
    pub font_directory: Option<PathBuf>,
    pub drcs_directory: Option<PathBuf>,
    pub drcs_report: Option<PathBuf>,
    pub ttml: Option<PathBuf>,
    pub archive: Option<PathBuf>,
    pub raw: Option<PathBuf>,
    pub srt: Option<PathBuf>,
    pub webvtt: Option<PathBuf>,
    pub summary: B24DecodeSummary,
}

#[derive(Debug, Clone)]
#[cfg_attr(any(not(test), feature = "libaribtlv"), allow(dead_code))]
pub(crate) struct CaptionPreview {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub text_color: u32,
    pub background_color: u32,
}
