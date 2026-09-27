use std::collections::HashMap;

use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::{TtmlCaption, TtmlCaptionStyle, attribute, native_b24};

#[derive(Debug, Clone, Default)]
pub(crate) struct TtmlInlineRun {
    pub(crate) text: String,
    pub(crate) style: TtmlCaptionStyle,
    pub(crate) ruby_text: Option<String>,
    pub(crate) ruby_style: Option<TtmlCaptionStyle>,
    pub(crate) ruby_group_base_count: usize,
    pub(crate) ruby_placement: Option<RubyPlacement>,
    pub(crate) ruby_base: bool,
    pub(crate) id: Option<String>,
    pub(crate) ruby_target_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TtmlRubyBinding {
    pub(crate) ruby_text: String,
    pub(crate) base_caption_index: usize,
    pub(crate) base_run_start: usize,
    pub(crate) base_run_end: usize,
    pub(crate) base_start: usize,
    pub(crate) base_end: usize,
    pub(crate) base_text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) base_cell_boxes: Vec<RubyLayoutBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) base_box: Option<RubyLayoutBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_ruby_box: Option<RubyLayoutBox>,
    pub(crate) placement: RubyPlacement,
    pub(crate) writing_mode: RubyWritingMode,
    pub(crate) resolver: RubyBindingResolver,
    pub(crate) ruby_style: TtmlCaptionStyle,
}

pub(crate) fn parse_ttml_inline_runs(
    body: &str,
    base_style: &TtmlCaptionStyle,
) -> Vec<TtmlInlineRun> {
    let mut runs = Vec::new();
    let mut remaining = body;
    while !remaining.is_empty() {
        let Some(tag_start) = remaining.find('<') else {
            push_ttml_text_run(&mut runs, remaining, base_style);
            break;
        };
        push_ttml_text_run(&mut runs, &remaining[..tag_start], base_style);
        let Some(relative_tag_end) = remaining[tag_start..].find('>') else {
            push_ttml_text_run(&mut runs, &remaining[tag_start..], base_style);
            break;
        };
        let tag_end = tag_start + relative_tag_end;
        let tag = &remaining[tag_start..=tag_end];
        if tag.starts_with("<br") {
            push_ttml_text_run(&mut runs, "\n", base_style);
            remaining = &remaining[tag_end + 1..];
            continue;
        }
        if !tag.starts_with("<span") {
            remaining = &remaining[tag_end + 1..];
            continue;
        }
        let content = &remaining[tag_end + 1..];
        let Some((close_start, close_end)) = matching_ttml_span_end(content) else {
            break;
        };
        let text = plain_ttml_inline_text(&content[..close_start]);
        let role = attribute(tag, "tts:ruby").or_else(|| attribute(tag, "ruby"));
        if role.as_deref() == Some("text") {
            attach_ttml_ruby_to_trailing_bases(&mut runs, text, tag, base_style);
        } else if !text.is_empty() {
            let mut style = base_style.clone();
            merge_ttml_inline_style(&mut style, tag);
            let target = attribute(tag, "arib-tt:ruby");
            if role.is_none()
                && attribute(tag, "xml:id").is_none()
                && target.is_none()
                && content[..close_start].contains("<span")
            {
                runs.extend(parse_ttml_inline_runs(&content[..close_start], &style));
            } else {
                runs.push(TtmlInlineRun {
                    text,
                    style,
                    ruby_text: None,
                    ruby_style: target
                        .as_ref()
                        .map(|_| ttml_ruby_inline_style(tag, base_style)),
                    ruby_group_base_count: 0,
                    ruby_placement: target.as_ref().and_then(|_| ruby_placement_from_tag(tag)),
                    ruby_base: role.as_deref() == Some("base"),
                    id: attribute(tag, "xml:id").or_else(|| attribute(tag, "id")),
                    ruby_target_id: target,
                });
            }
        }
        remaining = &content[close_end..];
    }
    resolve_arib_ttml_ruby_targets(&mut runs);
    runs
}

pub(crate) fn ttml_ruby_bindings(
    runs: &[TtmlInlineRun],
    writing_mode: RubyWritingMode,
) -> Vec<TtmlRubyBinding> {
    runs.iter()
        .enumerate()
        .filter_map(|(index, run)| {
            let ruby_text = run.ruby_text.as_ref()?.clone();
            let base_count = run.ruby_group_base_count.max(1).min(index + 1);
            let base_run_start = index + 1 - base_count;
            let base_start = runs[..base_run_start]
                .iter()
                .map(|run| {
                    run.text
                        .graphemes(true)
                        .filter(|text| *text != "\n")
                        .count()
                })
                .sum::<usize>();
            let base_length = runs[base_run_start..=index]
                .iter()
                .map(|run| {
                    run.text
                        .graphemes(true)
                        .filter(|text| *text != "\n")
                        .count()
                })
                .sum::<usize>();
            Some(TtmlRubyBinding {
                ruby_text,
                base_caption_index: 0,
                base_run_start,
                base_run_end: index + 1,
                base_start,
                base_end: base_start.saturating_add(base_length),
                base_text: runs[base_run_start..=index]
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect(),
                base_cell_boxes: Vec::new(),
                base_box: None,
                source_ruby_box: None,
                placement: run.ruby_placement.unwrap_or(RubyPlacement::Above),
                writing_mode,
                resolver: RubyBindingResolver::ExplicitTtml,
                ruby_style: run.ruby_style.clone().unwrap_or_else(|| run.style.clone()),
            })
        })
        .collect()
}

#[derive(Clone)]
struct TtmlSourceCell {
    run_index: usize,
    grapheme_index: usize,
    text: String,
    bounds: RubyLayoutBox,
}

pub(crate) fn associate_standalone_ttml_ruby(captions: &mut [TtmlCaption]) {
    let cells = captions.iter().map(ttml_source_cells).collect::<Vec<_>>();
    let max_sizes = cells
        .iter()
        .map(|cells| {
            cells
                .iter()
                .map(|cell| cell.bounds.height)
                .max()
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let mut discovered = Vec::new();

    for (ruby_index, ruby_caption) in captions.iter().enumerate() {
        if !ruby_caption.ruby_bindings.is_empty()
            || !is_horizontal_ttml_writing_mode(ruby_caption)
            || !is_kana_annotation_text(&ruby_caption.text)
        {
            continue;
        }
        let Some(ruby_width) = ruby_caption.width.filter(|value| *value > 0) else {
            continue;
        };
        let Some(ruby_height) = ruby_caption.height.filter(|value| *value > 0) else {
            continue;
        };
        let ruby_box = RubyLayoutBox {
            x: ruby_caption.x,
            y: ruby_caption.y,
            width: ruby_width,
            height: ruby_height,
        };
        let ruby_size = max_sizes[ruby_index].max(1);

        let candidate = captions
            .iter()
            .enumerate()
            .filter_map(|(base_index, base_caption)| {
                if base_index == ruby_index || !is_horizontal_ttml_writing_mode(base_caption) {
                    return None;
                }
                let base_width = base_caption.width.filter(|value| *value > 0)?;
                let base_height = base_caption.height.filter(|value| *value > 0)?;
                let base_size = max_sizes[base_index];
                if base_size < ruby_size.saturating_mul(2)
                    || base_height < ruby_height.saturating_mul(2)
                    || ruby_box.x < base_caption.x
                    || ruby_box.right() > base_caption.x.saturating_add(base_width)
                {
                    return None;
                }
                let mut rows = cells[base_index]
                    .iter()
                    .filter(|cell| {
                        cell.bounds.x < ruby_box.right() && cell.bounds.right() > ruby_box.x
                    })
                    .map(|cell| cell.bounds.y)
                    .collect::<Vec<_>>();
                rows.sort_unstable();
                rows.dedup();
                rows.into_iter()
                    .filter_map(|row| {
                        let row_bottom = cells[base_index]
                            .iter()
                            .filter(|cell| cell.bounds.y == row)
                            .map(|cell| cell.bounds.bottom())
                            .max()?;
                        let (placement, gap) = if ruby_box.bottom() <= row {
                            (RubyPlacement::Above, row - ruby_box.bottom())
                        } else if row_bottom <= ruby_box.y {
                            (RubyPlacement::Below, ruby_box.y - row_bottom)
                        } else {
                            return None;
                        };
                        (gap <= (ruby_height / 2).max(8)).then_some((row, placement, gap))
                    })
                    .min_by_key(|(_, _, gap)| *gap)
                    .map(|(row, placement, gap)| (base_index, row, placement, gap))
            })
            .min_by_key(|(_, _, _, gap)| *gap);

        let Some((base_index, row, placement, _)) = candidate else {
            continue;
        };
        let row_cells = cells[base_index]
            .iter()
            .filter(|cell| cell.bounds.y == row)
            .cloned()
            .collect::<Vec<_>>();
        let selected = ttml_target_cell_indices(&row_cells, ruby_box, &ruby_caption.text);
        let Some(first) = selected.first().copied() else {
            continue;
        };
        let Some(last) = selected.last().copied() else {
            continue;
        };
        let selected_cells = &row_cells[first..=last];
        let base_cell_boxes = selected_cells
            .iter()
            .map(|cell| cell.bounds)
            .collect::<Vec<_>>();
        let Some(base_box) = union_boxes(&base_cell_boxes) else {
            continue;
        };
        let base_run_start = selected_cells
            .iter()
            .map(|cell| cell.run_index)
            .min()
            .unwrap_or(0);
        let base_run_end = selected_cells
            .iter()
            .map(|cell| cell.run_index)
            .max()
            .unwrap_or(base_run_start)
            .saturating_add(1);
        let base_start = selected_cells
            .first()
            .map(|cell| cell.grapheme_index)
            .unwrap_or(0);
        let base_end = selected_cells
            .last()
            .map(|cell| cell.grapheme_index.saturating_add(1))
            .unwrap_or(base_start);
        discovered.push((
            ruby_index,
            TtmlRubyBinding {
                ruby_text: ruby_caption.text.clone(),
                base_caption_index: base_index,
                base_run_start,
                base_run_end,
                base_start,
                base_end,
                base_text: selected_cells
                    .iter()
                    .map(|cell| cell.text.as_str())
                    .collect(),
                base_cell_boxes,
                base_box: Some(base_box),
                source_ruby_box: Some(ruby_box),
                placement,
                writing_mode: RubyWritingMode::HorizontalTb,
                resolver: RubyBindingResolver::SourceGeometry,
                ruby_style: ruby_caption.style.clone(),
            },
        ));
    }

    for (ruby_index, binding) in discovered {
        captions[ruby_index].ruby_bindings.push(binding);
    }
}

fn ttml_source_cells(caption: &TtmlCaption) -> Vec<TtmlSourceCell> {
    let mut runs = caption
        .rich_body
        .as_deref()
        .map(|body| parse_ttml_inline_runs(body, &caption.style))
        .unwrap_or_default();
    if runs.is_empty() {
        runs.push(TtmlInlineRun {
            text: caption.text.clone(),
            style: caption.style.clone(),
            ..TtmlInlineRun::default()
        });
    }
    let mut cells = Vec::new();
    let mut x = caption.x;
    let mut y = caption.y;
    let mut grapheme_index = 0;
    for (run_index, run) in runs.iter().enumerate() {
        let (font_width, font_height) = ttml_font_dimensions(&run.style).unwrap_or((42.0, 42.0));
        let spacing = run
            .style
            .letter_spacing
            .as_deref()
            .and_then(ttml_first_pixel_length)
            .unwrap_or(0.0);
        let line_height = run
            .style
            .line_height
            .as_deref()
            .and_then(ttml_first_pixel_length)
            .or_else(|| {
                caption
                    .style
                    .line_height
                    .as_deref()
                    .and_then(ttml_first_pixel_length)
            })
            .unwrap_or_else(|| caption.height.unwrap_or(font_height.round() as i32) as f32)
            .max(font_height);
        let cell_width = (font_width + spacing).max(1.0).round() as i32;
        for grapheme in run.text.graphemes(true) {
            if grapheme == "\n" {
                x = caption.x;
                y = y.saturating_add(line_height.round() as i32);
                continue;
            }
            cells.push(TtmlSourceCell {
                run_index,
                grapheme_index,
                text: grapheme.to_owned(),
                bounds: RubyLayoutBox {
                    x,
                    y,
                    width: cell_width,
                    height: line_height.round().max(1.0) as i32,
                },
            });
            grapheme_index = grapheme_index.saturating_add(1);
            x = x.saturating_add(cell_width);
        }
    }
    cells
}

fn ttml_font_dimensions(style: &TtmlCaptionStyle) -> Option<(f32, f32)> {
    let value = style.font_size.as_deref()?;
    let values = value
        .split_whitespace()
        .filter_map(ttml_first_pixel_length)
        .collect::<Vec<_>>();
    let width = *values.first()?;
    let height = *values.get(1).unwrap_or(&width);
    Some((width, height))
}

fn ttml_first_pixel_length(value: &str) -> Option<f32> {
    value
        .trim()
        .strip_suffix("px")?
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn is_horizontal_ttml_writing_mode(caption: &TtmlCaption) -> bool {
    !matches!(
        caption.style.writing_mode.as_deref(),
        Some("vertical-rl" | "vertical-lr" | "tbrl" | "tblr")
    )
}

fn is_kana_annotation_text(text: &str) -> bool {
    let mut count = 0;
    text.chars().all(|character| {
        count += 1;
        count <= 12
            && (('\u{3040}'..='\u{30ff}').contains(&character)
                || ('\u{31f0}'..='\u{31ff}').contains(&character))
    }) && count > 0
}

fn ttml_target_cell_indices(
    cells: &[TtmlSourceCell],
    ruby_box: RubyLayoutBox,
    ruby_text: &str,
) -> Vec<usize> {
    let mut candidates = Vec::new();
    for start in 0..cells.len() {
        for end in start..cells.len() {
            let left = cells[start].bounds.x;
            let right = cells[end].bounds.right();
            if right.min(ruby_box.right()) <= left.max(ruby_box.x) {
                continue;
            }
            let all_han = cells[start..=end]
                .iter()
                .all(|cell| text_is_han(&cell.text));
            let all_ascii_or_digit = cells[start..=end]
                .iter()
                .all(|cell| text_is_ascii_or_digit(&cell.text));
            let non_kana = cells[start..=end]
                .iter()
                .all(|cell| !text_is_kana(&cell.text));
            let class_rank = if ruby_text_is_hiragana(ruby_text) {
                if all_han {
                    0
                } else if non_kana {
                    1
                } else {
                    2
                }
            } else if ruby_text_is_katakana(ruby_text) {
                if all_han || all_ascii_or_digit {
                    0
                } else if non_kana {
                    1
                } else {
                    2
                }
            } else if non_kana {
                0
            } else {
                1
            };
            let width_error = right
                .saturating_sub(left)
                .saturating_sub(ruby_box.width)
                .abs();
            let center_error = left
                .saturating_add(right)
                .saturating_sub(ruby_box.x.saturating_add(ruby_box.right()))
                .abs();
            candidates.push((start, end, class_rank, width_error * 2 + center_error));
        }
    }
    candidates.sort_by_key(|candidate| (candidate.2, candidate.3));
    candidates
        .first()
        .map(|(start, end, _, _)| (*start..=*end).collect())
        .unwrap_or_default()
}

fn text_is_han(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|ch| {
            matches!(
                ch as u32,
                0x3400..=0x4dbf
                    | 0x4e00..=0x9fff
                    | 0xf900..=0xfaff
                    | 0x20000..=0x323af
                    | 0x3005
                    | 0x3007
            )
        })
}

fn text_is_kana(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| matches!(ch as u32, 0x3040..=0x30ff | 0x31f0..=0x31ff | 0xff66..=0xff9f))
}

fn text_is_ascii_or_digit(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(ch as u32, 0xff10..=0xff19 | 0xff21..=0xff3a | 0xff41..=0xff5a)
        })
}

fn matching_ttml_span_end(content: &str) -> Option<(usize, usize)> {
    let mut depth = 1_usize;
    let mut cursor = 0;
    while let Some(relative_start) = content[cursor..].find('<') {
        let start = cursor + relative_start;
        let end = start + content[start..].find('>')? + 1;
        let tag = content[start + 1..end - 1].trim();
        let name = tag
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or_default();
        if name == "span" {
            if tag.starts_with('/') {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((start, end));
                }
            } else if !tag.ends_with('/') {
                depth += 1;
            }
        }
        cursor = end;
    }
    None
}

fn push_ttml_text_run(runs: &mut Vec<TtmlInlineRun>, raw: &str, style: &TtmlCaptionStyle) {
    let text = plain_ttml_inline_text(raw);
    if !text.is_empty() {
        runs.push(TtmlInlineRun {
            text,
            style: style.clone(),
            ..TtmlInlineRun::default()
        });
    }
}

fn plain_ttml_inline_text(value: &str) -> String {
    let mut output = String::new();
    let mut inside_tag = false;
    let mut tag = String::new();
    for character in value.chars() {
        match character {
            '<' => {
                inside_tag = true;
                tag.clear();
            }
            '>' => {
                if tag.trim_start().to_ascii_lowercase().starts_with("br") {
                    output.push('\n');
                }
                inside_tag = false;
            }
            _ if inside_tag => tag.push(character),
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    let output = decode_numeric_xml_entities(&output);
    output
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
}

fn decode_numeric_xml_entities(value: &str) -> String {
    let mut decoded = String::new();
    let mut remaining = value;
    while let Some(start) = remaining.find("&#") {
        decoded.push_str(&remaining[..start]);
        let entity = &remaining[start + 2..];
        let Some(end) = entity.find(';') else {
            decoded.push_str(&remaining[start..]);
            return decoded;
        };
        let digits = &entity[..end];
        let codepoint = digits
            .strip_prefix('x')
            .or_else(|| digits.strip_prefix('X'))
            .and_then(|digits| u32::from_str_radix(digits, 16).ok())
            .or_else(|| digits.parse::<u32>().ok());
        if let Some(character) = codepoint.and_then(char::from_u32) {
            decoded.push(character);
        } else {
            decoded.push_str(&remaining[start..start + 3 + end]);
        }
        remaining = &entity[end + 1..];
    }
    decoded.push_str(remaining);
    decoded
}

fn ttml_inline_style(tag: &str, inherited: &TtmlCaptionStyle) -> TtmlCaptionStyle {
    let mut style = inherited.clone();
    merge_ttml_inline_style(&mut style, tag);
    style
}

fn ttml_ruby_inline_style(tag: &str, inherited: &TtmlCaptionStyle) -> TtmlCaptionStyle {
    let mut style = ttml_inline_style(tag, inherited);
    if attribute(tag, "tts:fontSize").is_none() {
        style.font_size = None;
    }
    style
}

fn merge_ttml_inline_style(style: &mut TtmlCaptionStyle, tag: &str) {
    for (target, name) in [
        (&mut style.color, "tts:color"),
        (&mut style.background_color, "tts:backgroundColor"),
        (&mut style.font_size, "tts:fontSize"),
        (&mut style.font_family, "tts:fontFamily"),
        (&mut style.font_style, "tts:fontStyle"),
        (&mut style.font_weight, "tts:fontWeight"),
        (&mut style.text_outline, "tts:textOutline"),
        (&mut style.letter_spacing, "tts:letterSpacing"),
        (&mut style.opacity, "tts:opacity"),
    ] {
        if let Some(value) = attribute(tag, name) {
            *target = Some(value);
        }
    }
    if let Some(value) = attribute(tag, "arib-tt:font-face") {
        style.font_resource = Some(value);
    }
}

fn attach_ttml_ruby_to_trailing_bases(
    runs: &mut [TtmlInlineRun],
    text: String,
    tag: &str,
    base_style: &TtmlCaptionStyle,
) {
    if text.is_empty() {
        return;
    }
    let count = runs.iter().rev().take_while(|run| run.ruby_base).count();
    let Some(last) = runs.last_mut() else {
        return;
    };
    last.ruby_text = Some(text);
    last.ruby_style = Some(ttml_ruby_inline_style(tag, base_style));
    last.ruby_group_base_count = count.max(1);
    last.ruby_placement = ruby_placement_from_tag(tag);
}

fn resolve_arib_ttml_ruby_targets(runs: &mut Vec<TtmlInlineRun>) {
    let annotations = runs
        .iter()
        .filter_map(|run| {
            run.ruby_target_id.as_ref().map(|target| {
                (
                    target.clone(),
                    run.text.clone(),
                    run.ruby_style.clone().unwrap_or_else(|| run.style.clone()),
                    run.ruby_placement,
                )
            })
        })
        .collect::<Vec<_>>();
    for (target, text, style, placement) in annotations {
        if let Some(base) = runs
            .iter_mut()
            .find(|run| run.ruby_target_id.is_none() && run.id.as_deref() == Some(target.as_str()))
        {
            base.ruby_text = Some(text);
            base.ruby_style = Some(style);
            base.ruby_group_base_count = 1;
            base.ruby_placement = placement;
        }
    }
    runs.retain(|run| run.ruby_target_id.is_none());
}

fn ruby_placement_from_tag(tag: &str) -> Option<RubyPlacement> {
    attribute(tag, "tts:rubyPosition")
        .or_else(|| attribute(tag, "rubyPosition"))
        .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
            "before" | "above" | "outside" => Some(RubyPlacement::Above),
            "after" | "below" => Some(RubyPlacement::Below),
            _ => None,
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RubyPlacement {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RubyWritingMode {
    HorizontalTb,
    VerticalRl,
    VerticalLr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum RubyBindingResolver {
    SourceGeometry,
    ExplicitTtml,
    User,
    Llm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct RubyLayoutBox {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl RubyLayoutBox {
    pub(crate) fn right(self) -> i32 {
        self.x.saturating_add(self.width)
    }

    pub(crate) fn bottom(self) -> i32 {
        self.y.saturating_add(self.height)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct RubyBinding {
    pub(crate) ruby_text: String,
    pub(crate) base_region_index: usize,
    pub(crate) base_start: usize,
    pub(crate) base_end: usize,
    pub(crate) base_text: String,
    pub(crate) base_cell_boxes: Vec<RubyLayoutBox>,
    pub(crate) base_box: RubyLayoutBox,
    pub(crate) source_ruby_box: RubyLayoutBox,
    pub(crate) placement: RubyPlacement,
    pub(crate) writing_mode: RubyWritingMode,
    pub(crate) source_gap: i32,
    pub(crate) resolver: RubyBindingResolver,
    #[serde(skip)]
    pub(crate) base_characters: Vec<native_b24::CaptionCharacter>,
}

#[derive(Clone, Copy)]
struct RegionGeometry {
    index: usize,
    is_ruby: bool,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

pub(crate) fn scene_ruby_bindings(scene: &native_b24::CaptionScene) -> HashMap<usize, RubyBinding> {
    let geometries = scene
        .regions
        .iter()
        .enumerate()
        .filter_map(|(index, region)| {
            let characters = region_characters(scene, region)?;
            Some(RegionGeometry {
                index,
                is_ruby: region.is_ruby,
                left: characters.iter().map(|character| character.x).min()?,
                right: characters
                    .iter()
                    .map(|character| character.x.saturating_add(character.width.max(1)))
                    .max()?,
                top: characters.iter().map(|character| character.y).min()?,
                bottom: characters
                    .iter()
                    .map(|character| {
                        character.y.saturating_add(
                            (character.height.max(1) as f32 * character.vertical_scale.max(0.1))
                                .round() as i32,
                        )
                    })
                    .max()?,
            })
        })
        .collect::<Vec<_>>();
    let common_gap = geometries
        .iter()
        .filter(|geometry| geometry.is_ruby)
        .flat_map(|ruby| {
            geometries.iter().filter_map(move |base| {
                if base.index == ruby.index
                    || base.is_ruby
                    || base.left >= ruby.right
                    || base.right <= ruby.left
                {
                    return None;
                }
                if ruby.bottom <= base.top {
                    Some(base.top.saturating_sub(ruby.bottom))
                } else if base.bottom <= ruby.top {
                    Some(ruby.top.saturating_sub(base.bottom))
                } else {
                    None
                }
            })
        })
        .min()
        .unwrap_or(0)
        .max(0);
    let mut bindings = HashMap::new();
    for ruby in geometries.iter().filter(|geometry| geometry.is_ruby) {
        let Some((base, placement)) = nearest_base_geometry(*ruby, &geometries) else {
            continue;
        };
        let Some(ruby_region) = scene.regions.get(ruby.index) else {
            continue;
        };
        let Some(ruby_characters) = region_characters(scene, ruby_region) else {
            continue;
        };
        let Some(base_region) = scene.regions.get(base.index) else {
            continue;
        };
        let Some(base_characters) = region_characters(scene, base_region) else {
            continue;
        };
        let ruby_text = ruby_characters
            .iter()
            .map(|character| character.utf8.as_str())
            .collect::<String>();
        let selected = ruby_target_indices(base_characters, ruby.left, ruby.right, &ruby_text);
        let Some(first) = selected.first().copied() else {
            continue;
        };
        let Some(last) = selected.last().copied() else {
            continue;
        };
        let base_cell_boxes = selected
            .iter()
            .map(|index| base_character_box(base_characters, *index))
            .collect::<Vec<_>>();
        let Some(base_box) = union_boxes(&base_cell_boxes) else {
            continue;
        };
        let ruby_height = ruby.bottom.saturating_sub(ruby.top).max(1);
        let source_ruby_box = RubyLayoutBox {
            x: ruby.left,
            y: match placement {
                RubyPlacement::Above => base
                    .top
                    .saturating_sub(ruby_height)
                    .saturating_sub(common_gap),
                RubyPlacement::Below => base.bottom.saturating_add(common_gap),
            },
            width: ruby.right.saturating_sub(ruby.left).max(1),
            height: ruby_height,
        };
        bindings.insert(
            ruby.index,
            RubyBinding {
                ruby_text,
                base_region_index: base.index,
                base_start: first,
                base_end: last.saturating_add(1),
                base_text: base_characters[first..=last]
                    .iter()
                    .map(|character| character.utf8.as_str())
                    .collect(),
                base_cell_boxes,
                base_box,
                source_ruby_box,
                placement,
                writing_mode: RubyWritingMode::HorizontalTb,
                source_gap: common_gap,
                resolver: RubyBindingResolver::SourceGeometry,
                base_characters: base_characters.to_vec(),
            },
        );
    }
    bindings
}

fn region_characters<'a>(
    scene: &'a native_b24::CaptionScene,
    region: &native_b24::CaptionRegion,
) -> Option<&'a [native_b24::CaptionCharacter]> {
    let start = region.first_character as usize;
    let end = start
        .saturating_add(region.character_count as usize)
        .min(scene.characters.len());
    scene.characters.get(start..end)
}

fn nearest_base_geometry(
    ruby: RegionGeometry,
    geometries: &[RegionGeometry],
) -> Option<(RegionGeometry, RubyPlacement)> {
    geometries
        .iter()
        .filter_map(|base| {
            if base.index == ruby.index
                || base.is_ruby
                || base.left >= ruby.right
                || base.right <= ruby.left
            {
                return None;
            }
            if ruby.bottom <= base.top {
                Some((*base, RubyPlacement::Above, base.top - ruby.bottom))
            } else if base.bottom <= ruby.top {
                Some((*base, RubyPlacement::Below, ruby.top - base.bottom))
            } else {
                None
            }
        })
        .min_by_key(|(_, _, gap)| *gap)
        .map(|(base, placement, _)| (base, placement))
}

fn union_boxes(boxes: &[RubyLayoutBox]) -> Option<RubyLayoutBox> {
    let left = boxes.iter().map(|bounds| bounds.x).min()?;
    let top = boxes.iter().map(|bounds| bounds.y).min()?;
    let right = boxes.iter().map(|bounds| bounds.right()).max()?;
    let bottom = boxes.iter().map(|bounds| bounds.bottom()).max()?;
    Some(RubyLayoutBox {
        x: left,
        y: top,
        width: right.saturating_sub(left).max(1),
        height: bottom.saturating_sub(top).max(1),
    })
}

pub(crate) fn b24_section_width(character: &native_b24::CaptionCharacter) -> i32 {
    ((character.width.max(1) as f32 * character.horizontal_scale.max(0.1)
        + character.horizontal_spacing as f32 * character.horizontal_scale.max(0.1))
    .floor() as i32)
        .max(1)
}

pub(crate) fn b24_base_cell_right(base: &[native_b24::CaptionCharacter], index: usize) -> i32 {
    let character = &base[index];
    base.get(index + 1)
        .filter(|next| next.y == character.y && next.x > character.x)
        .map(|next| next.x)
        .unwrap_or_else(|| character.x.saturating_add(b24_section_width(character)))
}

fn base_character_box(base: &[native_b24::CaptionCharacter], index: usize) -> RubyLayoutBox {
    let character = &base[index];
    RubyLayoutBox {
        x: character.x,
        y: character.y,
        width: b24_base_cell_right(base, index)
            .saturating_sub(character.x)
            .max(1),
        height: ((character.height.max(1) as f32 * character.vertical_scale.max(0.1)).round()
            as i32)
            .max(1),
    }
}

fn is_han_character(character: &native_b24::CaptionCharacter) -> bool {
    character.utf8.chars().all(|ch| {
        matches!(
            ch as u32,
            0x3400..=0x4dbf
                | 0x4e00..=0x9fff
                | 0xf900..=0xfaff
                | 0x20000..=0x323af
                | 0x3005
                | 0x3007
        )
    }) && !character.utf8.is_empty()
}

fn is_kana_character(character: &native_b24::CaptionCharacter) -> bool {
    character
        .utf8
        .chars()
        .all(|ch| matches!(ch as u32, 0x3040..=0x30ff | 0x31f0..=0x31ff | 0xff66..=0xff9f))
        && !character.utf8.is_empty()
}

fn is_ascii_or_digit_character(character: &native_b24::CaptionCharacter) -> bool {
    character.utf8.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(ch as u32, 0xff10..=0xff19 | 0xff21..=0xff3a | 0xff41..=0xff5a)
    }) && !character.utf8.is_empty()
}

fn ruby_text_is_hiragana(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| matches!(ch as u32, 0x3040..=0x309f | 0x30fc))
}

fn ruby_text_is_katakana(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| matches!(ch as u32, 0x30a0..=0x30ff))
}

pub(crate) fn ruby_target_indices(
    base: &[native_b24::CaptionCharacter],
    ruby_left: i32,
    ruby_right: i32,
    ruby_text: &str,
) -> Vec<usize> {
    if base.is_empty() {
        return Vec::new();
    }
    let ruby_width = ruby_right.saturating_sub(ruby_left).max(1);
    let ruby_center_twice = ruby_left.saturating_add(ruby_right);
    let hiragana_reading = ruby_text_is_hiragana(ruby_text);
    let katakana_reading = ruby_text_is_katakana(ruby_text);
    let mut candidates = Vec::new();
    for start in 0..base.len() {
        let mut end = start;
        while end < base.len() {
            if end > start
                && (base[end].y != base[end - 1].y
                    || base[end].x <= base[end - 1].x
                    || base[end].x > b24_base_cell_right(base, end - 1))
            {
                break;
            }
            let left = base[start].x;
            let right = b24_base_cell_right(base, end);
            let overlap = right.min(ruby_right).saturating_sub(left.max(ruby_left));
            if overlap > 0 {
                let all_han = base[start..=end].iter().all(is_han_character);
                let all_ascii_or_digit = base[start..=end].iter().all(is_ascii_or_digit_character);
                let non_kana = base[start..=end]
                    .iter()
                    .all(|character| !is_kana_character(character));
                let class_rank = if hiragana_reading {
                    if all_han {
                        0
                    } else if non_kana {
                        1
                    } else {
                        2
                    }
                } else if katakana_reading {
                    if all_han || all_ascii_or_digit {
                        0
                    } else if non_kana {
                        1
                    } else {
                        2
                    }
                } else if non_kana {
                    0
                } else {
                    1
                };
                let candidate_width = right.saturating_sub(left).max(1);
                let width_error = candidate_width.saturating_sub(ruby_width).abs();
                let center_error_twice = left
                    .saturating_add(right)
                    .saturating_sub(ruby_center_twice)
                    .abs();
                let geometry_score = width_error
                    .saturating_mul(2)
                    .saturating_add(center_error_twice);
                candidates.push((
                    start,
                    end,
                    class_rank,
                    geometry_score,
                    width_error,
                    center_error_twice,
                ));
            }
            end += 1;
        }
    }
    if candidates.is_empty() {
        return vec![
            base.iter()
                .enumerate()
                .min_by_key(|(_, character)| (character.x - ruby_left).abs())
                .map(|(index, _)| index)
                .unwrap_or(0),
        ];
    }
    candidates.sort_by(|a, b| {
        a.2.cmp(&b.2)
            .then_with(|| a.3.cmp(&b.3))
            .then_with(|| a.4.cmp(&b.4))
            .then_with(|| a.5.cmp(&b.5))
            .then_with(|| b.1.saturating_sub(b.0).cmp(&a.1.saturating_sub(a.0)))
    });
    let (start, end, _, _, _, _) = candidates[0];
    (start..=end).collect()
}
