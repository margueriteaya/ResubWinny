use super::ass_rich::*;
use super::ruby_layout::{BundledAssGlyphMetrics, RubyLayoutPlan, RubyLayoutRequest, layout_ruby};
use super::*;
use unicode_segmentation::UnicodeSegmentation;

const ASS_ARIB_FONT_FAMILY: &str = "Rounded M+ 1m for ARIB";
const ASS_ARIB_FONT_LICENSE: &[u8] =
    include_bytes!("../../../../third_party/rounded-mplus-1m-arib/LICENSE.txt");
const ASS_PLAY_RES_X: i32 = 1920;
const ASS_PLAY_RES_Y: i32 = 1080;
// Keep the visual air between a Ruby band and its base line symmetric whether
// the broadcast places the annotation above or below the line.
const ASS_RUBY_GAP: i32 = 12;
pub(crate) fn ass_time(milliseconds: i64) -> String {
    let milliseconds = milliseconds.max(0);
    format!(
        "{}:{:02}:{:02}.{:02}",
        milliseconds / 3_600_000,
        (milliseconds / 60_000) % 60,
        (milliseconds / 1_000) % 60,
        (milliseconds / 10) % 100
    )
}

pub(crate) fn ass_color(color: u32) -> String {
    let alpha = 255_u8.saturating_sub((color >> 24) as u8);
    format!(
        "&H{alpha:02X}{:02X}{:02X}{:02X}&",
        (color >> 16) & 0xff,
        (color >> 8) & 0xff,
        color & 0xff
    )
}

pub(crate) fn ass_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('\n', "\\N")
}

pub(crate) fn write_ass_header(writer: &mut BufWriter<File>) -> io::Result<()> {
    writer.write_all(format!(
        "[Script Info]\nTitle: ResubWinny\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\nStyle: Default,{ASS_ARIB_FONT_FAMILY},42,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,7,20,20,20,1\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n"
    ).as_bytes())
}
struct AssTtmlCell {
    grapheme_index: usize,
    ink_left: f32,
    ink_right: f32,
}

pub(crate) fn write_ass_ttml_group(
    writer: &mut BufWriter<File>,
    captions: &[TtmlCaption],
    options: &ConversionOptions,
) -> io::Result<()> {
    for (caption_index, caption) in captions.iter().enumerate() {
        if let Some(binding) = caption.ruby_bindings.iter().find(|binding| {
            binding.resolver == RubyBindingResolver::SourceGeometry
                && binding.base_caption_index != caption_index
        }) {
            if options.preserve_ruby {
                write_ass_standalone_ruby(writer, caption, binding, captions, options)?;
            }
            continue;
        }
        write_ass_ttml_caption_at(writer, caption, 0, 7, None, options)?;
    }
    Ok(())
}

fn write_ass_standalone_ruby(
    writer: &mut BufWriter<File>,
    caption: &TtmlCaption,
    binding: &TtmlRubyBinding,
    captions: &[TtmlCaption],
    options: &ConversionOptions,
) -> io::Result<()> {
    let preferred_font_size = caption
        .style
        .font_size
        .as_deref()
        .and_then(ass_font_size_from_ttml)
        .unwrap_or(18);
    let Some(base_caption) = captions.get(binding.base_caption_index) else {
        return write_ass_ttml_caption_at(writer, caption, 1, 8, None, options);
    };
    let Some(cells) = ass_ttml_cells(base_caption) else {
        return write_ass_ttml_caption_at(writer, caption, 1, 8, None, options);
    };
    let selected = cells
        .iter()
        .filter(|cell| (binding.base_start..binding.base_end).contains(&cell.grapheme_index))
        .collect::<Vec<_>>();
    let Some(target_left) = selected.iter().map(|cell| cell.ink_left).reduce(f32::min) else {
        return write_ass_ttml_caption_at(writer, caption, 1, 8, None, options);
    };
    let Some(target_right) = selected.iter().map(|cell| cell.ink_right).reduce(f32::max) else {
        return write_ass_ttml_caption_at(writer, caption, 1, 8, None, options);
    };
    let Some(base_box) = binding.base_box else {
        return write_ass_ttml_caption_at(writer, caption, 1, 8, None, options);
    };
    let ruby_y = match binding.placement {
        RubyPlacement::Above => base_box
            .y
            .saturating_sub(preferred_font_size)
            .saturating_sub(ASS_RUBY_GAP),
        RubyPlacement::Below => base_box.bottom().saturating_add(ASS_RUBY_GAP),
    };
    let Some(plan) = layout_ruby(
        &RubyLayoutRequest {
            text: &caption.text,
            container: RubyLayoutBox {
                x: target_left.floor() as i32,
                y: ruby_y,
                width: (target_right.ceil() as i32)
                    .saturating_sub(target_left.floor() as i32)
                    .max(1),
                height: preferred_font_size,
            },
            preferred_font_size,
            minimum_font_size: 6,
            placement: binding.placement,
            writing_mode: RubyWritingMode::HorizontalTb,
        },
        &BundledAssGlyphMetrics,
    ) else {
        return Ok(());
    };
    let style = format!(
        "{}\\fsp0\\fn{}",
        ass_ttml_style_tags(&caption.style, plan.font_size, options.preserve_color),
        plan.font_family
    );
    for glyph in plan.glyphs {
        writeln!(
            writer,
            "Dialogue: 1,{},{},Default,,0,0,0,,{{\\an8\\pos({},{}){}}}{}",
            ass_time(caption.start_ms),
            ass_time(caption.end_ms),
            glyph.anchor_x,
            glyph.anchor_y,
            style,
            ass_escape(&glyph.text),
        )?;
    }
    Ok(())
}

fn ass_ttml_cells(caption: &TtmlCaption) -> Option<Vec<AssTtmlCell>> {
    if !is_horizontal_ttml(caption) {
        return None;
    }
    let mut runs = caption
        .rich_body
        .as_deref()
        .map(|body| parse_ass_inline_runs(body, &caption.style))
        .unwrap_or_default();
    if runs.is_empty() {
        runs.push(AssInlineRun {
            text: caption.text.clone(),
            style: caption.style.clone(),
            ..AssInlineRun::default()
        });
    }
    if runs.iter().any(|run| run.ruby_text.is_some()) {
        return None;
    }
    let mut cells = Vec::new();
    let mut x = caption.x;
    let mut y = caption.y;
    let mut layout_x = caption.x as f32;
    let mut grapheme_index = 0;
    for run in runs {
        let (font_width, font_height) = run
            .style
            .font_size
            .as_deref()
            .and_then(ass_font_dimensions_from_ttml)
            .unwrap_or((42.0, 42.0));
        let spacing = run
            .style
            .letter_spacing
            .as_deref()
            .and_then(ttml_pixel_length)
            .unwrap_or(0.0);
        let line_height = run
            .style
            .line_height
            .as_deref()
            .and_then(ttml_pixel_length)
            .or_else(|| {
                caption
                    .style
                    .line_height
                    .as_deref()
                    .and_then(ttml_pixel_length)
            })
            .unwrap_or_else(|| caption.height.unwrap_or(font_height.round() as i32) as f32)
            .max(font_height);
        let cell_width = (font_width + spacing).max(1.0).round() as i32;
        for grapheme in run.text.graphemes(true) {
            if grapheme == "\n" {
                x = caption.x;
                y = y.saturating_add(line_height.round() as i32);
                layout_x = caption.x as f32;
                continue;
            }
            let (layout_width, ink_left, ink_right) = ass_text_ink_bounds(grapheme, font_height);
            cells.push(AssTtmlCell {
                grapheme_index,
                ink_left: layout_x + ink_left,
                ink_right: layout_x + ink_right,
            });
            grapheme_index = grapheme_index.saturating_add(1);
            x = x.saturating_add(cell_width);
            layout_x += layout_width + spacing;
        }
    }
    (!cells.is_empty()).then_some(cells)
}

fn ttml_pixel_length(value: &str) -> Option<f32> {
    value
        .split_whitespace()
        .next()?
        .strip_suffix("px")?
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn write_ass_ttml_caption_at(
    writer: &mut BufWriter<File>,
    caption: &TtmlCaption,
    layer: usize,
    alignment: usize,
    anchor: Option<(i32, i32)>,
    options: &ConversionOptions,
) -> io::Result<()> {
    let filtered_text = export_text(&caption.text, options);
    if filtered_text.is_empty() {
        return Ok(());
    }
    let fallback_size = caption
        .style
        .font_size
        .as_deref()
        .and_then(ass_font_size_from_ttml)
        .unwrap_or(42);
    let mut runs =
        if options.preserve_ruby && options.preserve_gaiji && options.preserve_accessibility {
            caption
                .rich_body
                .as_deref()
                .map(|body| filter_ttml_inline_body(body, options.preserve_color))
                .map(|body| parse_ass_inline_runs(&body, &caption.style))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
    if runs.is_empty() {
        runs.push(AssInlineRun {
            text: filtered_text,
            style: caption.style.clone(),
            ..AssInlineRun::default()
        });
    }
    let ruby_band = options
        .preserve_ruby
        .then_some(&caption.ruby_bindings)
        .into_iter()
        .flatten()
        .filter_map(|binding| {
            if matches!(binding.placement, RubyPlacement::Below) {
                return None;
            }
            Some(
                binding
                    .ruby_style
                    .font_size
                    .as_deref()
                    .and_then(ass_font_size_from_ttml)
                    .unwrap_or((fallback_size / 2).max(6)),
            )
        })
        .max()
        .unwrap_or(0);
    let ruby_writing_mode = match caption.style.writing_mode.as_deref() {
        Some("vertical-lr" | "tblr") => RubyWritingMode::VerticalLr,
        Some("vertical-rl" | "tbrl") => RubyWritingMode::VerticalRl,
        _ => RubyWritingMode::HorizontalTb,
    };
    let vertical = !matches!(ruby_writing_mode, RubyWritingMode::HorizontalTb);
    let base_x = anchor.map(|(x, _)| x).unwrap_or(caption.x);
    let calculated_y = if vertical {
        caption.y
    } else {
        caption
            .y
            .saturating_add(ruby_band)
            .saturating_add(if ruby_band > 0 { ASS_RUBY_GAP } else { 0 })
    };
    let base_y = anchor.map(|(_, y)| y).unwrap_or(calculated_y);
    let body = runs
        .iter()
        .map(|run| {
            format!(
                "{{{}}}{}",
                ass_ttml_style_tags(&run.style, fallback_size, options.preserve_color),
                ass_escape(&run.text)
            )
        })
        .collect::<String>();
    writeln!(
        writer,
        "Dialogue: {layer},{},{},Default,,0,0,0,,{}{}",
        ass_time(caption.start_ms),
        ass_time(caption.end_ms),
        if options.preserve_position {
            format!("{{\\an{alignment}\\pos({base_x},{base_y})}}")
        } else {
            "{\\an2}".to_owned()
        },
        body
    )?;

    let advances = runs
        .iter()
        .map(|run| run_advance(run, fallback_size))
        .collect::<Vec<_>>();
    for binding in options
        .preserve_ruby
        .then_some(&caption.ruby_bindings)
        .into_iter()
        .flatten()
    {
        let ruby = binding.ruby_text.as_str();
        if ruby.is_empty() || binding.base_run_end == 0 || binding.base_run_end > runs.len() {
            continue;
        }
        let index = binding.base_run_end - 1;
        let ruby_style = &binding.ruby_style;
        let placement = binding.placement;
        let ruby_font_size = ruby_style
            .font_size
            .as_deref()
            .and_then(ass_font_size_from_ttml)
            .unwrap_or((fallback_size / 2).max(6));
        let group_start = binding.base_run_start.min(index);
        let prefix = advances[..group_start].iter().sum::<f32>();
        let (first_left, _) = run_ink_bounds(&runs[group_start], fallback_size);
        let (_, last_right) = run_ink_bounds(&runs[index], fallback_size);
        let before_last = advances[group_start..index].iter().sum::<f32>();
        let target_start = prefix + first_left;
        let target_end = prefix + before_last + last_right;
        let ruby_axis = match placement {
            RubyPlacement::Above => base_y
                .saturating_sub(ruby_font_size)
                .saturating_sub(ASS_RUBY_GAP),
            RubyPlacement::Below => base_y
                .saturating_add(fallback_size)
                .saturating_add(ASS_RUBY_GAP),
        };
        let target_start = target_start.floor() as i32;
        let target_end = target_end.ceil() as i32;
        let container = if vertical {
            RubyLayoutBox {
                x: if matches!(ruby_writing_mode, RubyWritingMode::VerticalRl) {
                    caption
                        .x
                        .saturating_add(fallback_size)
                        .saturating_add(ASS_RUBY_GAP)
                } else {
                    caption
                        .x
                        .saturating_sub(ruby_font_size)
                        .saturating_sub(ASS_RUBY_GAP)
                },
                y: caption.y.saturating_add(target_start),
                width: ruby_font_size,
                height: target_end.saturating_sub(target_start).max(1),
            }
        } else {
            RubyLayoutBox {
                x: caption.x.saturating_add(target_start),
                y: ruby_axis,
                width: target_end.saturating_sub(target_start).max(1),
                height: ruby_font_size,
            }
        };
        let Some(plan) = layout_ruby(
            &RubyLayoutRequest {
                text: ruby,
                container,
                preferred_font_size: ruby_font_size,
                minimum_font_size: 6,
                placement,
                writing_mode: binding.writing_mode,
            },
            &BundledAssGlyphMetrics,
        ) else {
            continue;
        };
        let style = format!(
            "{}\\fsp0\\fn{}",
            ass_ttml_style_tags(ruby_style, plan.font_size, options.preserve_color),
            plan.font_family
        );
        for glyph in plan.glyphs {
            writeln!(
                writer,
                "Dialogue: 1,{},{},Default,,0,0,0,,{{\\an8\\pos({},{}){}}}{}",
                ass_time(caption.start_ms),
                ass_time(caption.end_ms),
                glyph.anchor_x,
                glyph.anchor_y,
                style,
                ass_escape(&glyph.text),
            )?;
        }
    }
    Ok(())
}

/// Remove presentation attributes from nested TTML spans when the user asks
/// for a structural export without colour.  The rich body remains intact for
/// Ruby and other inline semantics; only the explicitly disabled attributes
/// are removed before either TTML or ASS consumes it.
pub(crate) fn filter_ttml_inline_body(body: &str, preserve_color: bool) -> String {
    if preserve_color {
        return body.to_owned();
    }
    let mut output = String::with_capacity(body.len());
    let mut remaining = body;
    while let Some(start) = remaining.find('<') {
        output.push_str(&remaining[..start]);
        let Some(end) = remaining[start..].find('>') else {
            output.push_str(&remaining[start..]);
            break;
        };
        let end = start + end + 1;
        let tag = &remaining[start..end];
        if tag.starts_with("<span") {
            let mut filtered = tag.to_owned();
            for name in ["tts:color", "tts:backgroundColor", "tts:textOutline"] {
                loop {
                    let next = remove_xml_attribute(&filtered, name);
                    if next == filtered {
                        break;
                    }
                    filtered = next;
                }
            }
            output.push_str(&filtered);
        } else {
            output.push_str(tag);
        }
        remaining = &remaining[end..];
    }
    output.push_str(remaining);
    output
}

fn is_horizontal_ttml(caption: &TtmlCaption) -> bool {
    !matches!(
        caption.style.writing_mode.as_deref(),
        Some("vertical-rl" | "vertical-lr" | "tbrl" | "tblr")
    )
}

fn ass_ttml_style_tags(
    style: &TtmlCaptionStyle,
    fallback_size: i32,
    preserve_color: bool,
) -> String {
    let mut tags = String::new();
    if preserve_color && let Some(color) = style.color.as_deref().and_then(ass_color_from_ttml) {
        tags.push_str(&format!("\\c{color}"));
    }
    let font_size = style
        .font_size
        .as_deref()
        .and_then(ass_font_size_from_ttml)
        .unwrap_or(fallback_size);
    tags.push_str(&format!("\\fs{font_size}"));
    if let Some(font_family) = style.font_family.as_deref() {
        tags.push_str(&format!("\\fn{}", ass_escape(ass_font_family(font_family))));
    }
    tags.push_str(
        if matches!(style.font_weight.as_deref(), Some("bold" | "bolder")) {
            "\\b1"
        } else {
            "\\b0"
        },
    );
    tags.push_str(
        if matches!(style.font_style.as_deref(), Some("italic" | "oblique")) {
            "\\i1"
        } else {
            "\\i0"
        },
    );
    if let Some(letter_spacing) = style
        .letter_spacing
        .as_deref()
        .and_then(ass_letter_spacing_from_ttml)
    {
        tags.push_str(&format!("\\fsp{letter_spacing}"));
    } else {
        tags.push_str("\\fsp0");
    }
    if let Some(opacity) = style
        .opacity
        .as_deref()
        .and_then(|value| value.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
    {
        let alpha = (255.0 * (1.0 - opacity.clamp(0.0, 1.0))).round() as u8;
        tags.push_str(&format!("\\alpha&H{alpha:02X}&"));
    }
    if let Some(outline) = style.text_outline.as_deref() {
        let width = outline
            .split_whitespace()
            .find_map(|token| token.strip_suffix("px")?.parse::<f32>().ok())
            .unwrap_or(0.0)
            .clamp(0.0, 16.0);
        tags.push_str(&format!("\\bord{width:.2}"));
        if let Some(color) = outline.split_whitespace().find_map(ass_color_from_ttml) {
            tags.push_str(&format!("\\3c{color}"));
        }
    }
    tags
}

fn ass_font_family(source: &str) -> &str {
    match source.trim() {
        "丸ゴシック" | "丸ゴシック体" => ASS_ARIB_FONT_FAMILY,
        _ => source,
    }
}

fn b24_export_character_text<'a>(
    character: &'a native_b24::CaptionCharacter,
    interval: &'a RegionInterval,
    options: &'a ConversionOptions,
) -> Option<&'a str> {
    if !character.utf8.is_empty() {
        return Some(&character.utf8);
    }
    if character.kind != 1 || !options.preserve_drcs {
        return None;
    }
    let mapped = (options.drcs_mode == DrcsMode::UseUserMapping)
        .then(|| options.drcs_replacements.get(&character.drcs_code))
        .flatten()
        .filter(|text| !text.is_empty());
    mapped.map(String::as_str).or_else(|| {
        interval
            .drcs_glyphs
            .iter()
            .find(|glyph| glyph.drcs_code == character.drcs_code)
            .map(|glyph| glyph.alternative_text.as_str())
            .filter(|text| !text.is_empty())
    })
}

pub(crate) fn write_ass_interval(
    writer: &mut BufWriter<File>,
    interval: &RegionInterval,
    options: &ConversionOptions,
) -> io::Result<()> {
    if !options.preserve_position {
        return write_ass_interval_group(writer, std::slice::from_ref(interval), options);
    }
    if interval.region.is_ruby && !options.preserve_ruby {
        return Ok(());
    }
    if interval.characters.is_empty() {
        return Ok(());
    }
    let scale_x = ASS_PLAY_RES_X as f32 / interval.plane_width.max(1) as f32;
    let scale_y = ASS_PLAY_RES_Y as f32 / interval.plane_height.max(1) as f32;
    let scale_uniform = scale_x.min(scale_y);
    let mut line = Vec::new();
    for character in &interval.characters {
        if !options.preserve_gaiji && b24_character_is_gaiji_source(character) {
            write_filtered_ass_character_line(
                writer,
                interval,
                &line,
                scale_x,
                scale_y,
                scale_uniform,
                options,
            )?;
            line.clear();
            continue;
        }
        let text = b24_export_character_text(character, interval, options);
        let Some(text) = text.filter(|text| keep_text(text, options)) else {
            write_filtered_ass_character_line(
                writer,
                interval,
                &line,
                scale_x,
                scale_y,
                scale_uniform,
                options,
            )?;
            line.clear();
            continue;
        };
        if line
            .last()
            .is_some_and(|(previous, _)| !b24_characters_are_contiguous(previous, character))
        {
            write_filtered_ass_character_line(
                writer,
                interval,
                &line,
                scale_x,
                scale_y,
                scale_uniform,
                options,
            )?;
            line.clear();
        }
        line.push((character, text));
    }
    write_filtered_ass_character_line(
        writer,
        interval,
        &line,
        scale_x,
        scale_y,
        scale_uniform,
        options,
    )?;
    if !options.preserve_drcs || !options.preserve_position {
        return Ok(());
    }
    for character in &interval.characters {
        if character.kind != 1 || b24_export_character_text(character, interval, options).is_some()
        {
            continue;
        }
        let Some(glyph) = interval
            .drcs_glyphs
            .iter()
            .find(|glyph| glyph.drcs_code == character.drcs_code)
        else {
            continue;
        };
        let drawing = drcs_drawing(glyph);
        if drawing.is_empty() {
            continue;
        }
        let glyph_scale_x = character.width as f32 * scale_x * 100.0 / glyph.width.max(1) as f32;
        let glyph_scale_y = character.height as f32 * scale_y * 100.0 / glyph.height.max(1) as f32;
        writeln!(
            writer,
            "Dialogue: 1,{},{},Default,,0,0,0,,{{\\an7\\pos({},{})\\fscx{:.2}\\fscy{:.2}\\c{}\\p1}}{}",
            ass_time(interval.begin_ms),
            ass_time(interval.end_ms),
            scale_ass_coordinate(character.x, scale_x),
            scale_ass_coordinate(character.y, scale_y),
            glyph_scale_x,
            glyph_scale_y,
            ass_color(character.text_color),
            drawing,
        )?;
    }
    Ok(())
}

/// When position is deliberately discarded, adjacent B24 regions that share
/// one timing interval must become one editable subtitle cue. The source
/// coordinates are used only to order rows and fragments; no position tag is
/// emitted and the native preview path never calls this function.
pub(crate) fn write_ass_interval_group(
    writer: &mut BufWriter<File>,
    intervals: &[RegionInterval],
    options: &ConversionOptions,
) -> io::Result<()> {
    if options.preserve_position {
        for interval in intervals {
            write_ass_interval(writer, interval, options)?;
        }
        return Ok(());
    }
    let mut drawings = std::collections::HashMap::new();
    let mut rows: Vec<(i32, Vec<(&native_b24::CaptionCharacter, String)>)> = Vec::new();
    let mut ordered_intervals = intervals
        .iter()
        .filter(|interval| !interval.region.is_ruby)
        .collect::<Vec<_>>();
    ordered_intervals.sort_by_key(|interval| (interval.region.y, interval.region.x));
    for interval in ordered_intervals {
        for character in &interval.characters {
            let mut text = b24_export_character_text(character, interval, options)
                .map(|text| export_text(text, options))
                .unwrap_or_default();
            if b24_export_character_text(character, interval, options).is_none()
                && character.kind == 1
                && options.preserve_drcs
                && let Some(glyph) = interval
                    .drcs_glyphs
                    .iter()
                    .find(|glyph| glyph.drcs_code == character.drcs_code)
            {
                let drawing = drcs_drawing(glyph);
                if !drawing.is_empty() {
                    let sx = character.width as f32 * ASS_PLAY_RES_X as f32
                        / interval.plane_width.max(1) as f32
                        * 100.0
                        / glyph.width.max(1) as f32;
                    let sy = character.height as f32 * ASS_PLAY_RES_Y as f32
                        / interval.plane_height.max(1) as f32
                        * 100.0
                        / glyph.height.max(1) as f32;
                    drawings.insert(
                        character as *const _,
                        format!(
                            "{{\\fscx{sx:.2}\\fscy{sy:.2}\\p1}}{drawing}{{\\p0\\fscx100\\fscy100}}"
                        ),
                    );
                    text.push('\u{fffc}');
                }
            }
            if text.is_empty() {
                continue;
            }
            let row = rows
                .iter_mut()
                .find(|(y, _)| (character.y - *y).abs() <= character.height.max(1) / 2);
            if let Some((_, cells)) = row {
                cells.push((character, text));
            } else {
                rows.push((character.y, vec![(character, text)]));
            }
        }
    }
    rows.sort_by_key(|(y, _)| *y);
    let Some(first) = intervals.first() else {
        return Ok(());
    };
    let scale_x = ASS_PLAY_RES_X as f32 / first.plane_width.max(1) as f32;
    let scale_y = ASS_PLAY_RES_Y as f32 / first.plane_height.max(1) as f32;
    let mut body = String::from("{\\an2}");
    let mut active_style = None;
    let mut active_spacing = 0.0_f32;
    let mut wrote_row = false;
    for (_, cells) in &rows {
        let combined = cells
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<String>();
        let retained = crate::caption_features::retained_characters(
            &combined,
            options.preserve_gaiji,
            options.preserve_accessibility,
        );
        let mut cursor = 0_usize;
        let filtered_cells = cells
            .iter()
            .filter_map(|(character, text)| {
                let filtered = text
                    .chars()
                    .filter(|_| {
                        let keep = retained.get(cursor).copied().unwrap_or(false);
                        cursor = cursor.saturating_add(1);
                        keep && (options.preserve_gaiji
                            || !b24_character_is_gaiji_source(character))
                    })
                    .collect::<String>();
                (!filtered.is_empty()).then_some((*character, filtered))
            })
            .collect::<Vec<_>>();
        if filtered_cells.is_empty() {
            continue;
        }
        if wrote_row {
            body.push_str("\\N");
        }
        let refs = filtered_cells
            .iter()
            .map(|(character, text)| (*character, text.as_str()))
            .collect::<Vec<_>>();
        append_unpositioned_b24_body(
            &mut body,
            &refs,
            options,
            (scale_x, scale_y),
            &drawings,
            &mut active_style,
            &mut active_spacing,
        );
        wrote_row = true;
    }
    if body == "{\\an2}" {
        return Ok(());
    }
    writeln!(
        writer,
        "Dialogue: 0,{},{},Default,,0,0,0,,{body}",
        ass_time(first.begin_ms),
        ass_time(first.end_ms),
    )
}

fn append_unpositioned_b24_body(
    body: &mut String,
    line: &[(&native_b24::CaptionCharacter, &str)],
    options: &ConversionOptions,
    scale: (f32, f32),
    drawings: &std::collections::HashMap<*const native_b24::CaptionCharacter, String>,
    active_style: &mut Option<String>,
    active_spacing: &mut f32,
) {
    let (scale_x, scale_y) = scale;
    for (index, (character, text)) in line.iter().enumerate() {
        let font_size = scale_ass_coordinate(character.height.max(1), scale_y).max(1);
        let next_spacing = (line.get(index + 1).is_some() || text.chars().count() > 1)
            .then_some(character.horizontal_spacing as f32 * scale_x);
        let stroke = character.style & (1 << 3) != 0;
        let colors = if options.preserve_color {
            {
                format!(
                    "\\c{}\\3c{}",
                    ass_color(character.text_color),
                    ass_color(character.stroke_color)
                )
            }
        } else {
            Default::default()
        };
        let style = format!(
            "\\fs{font_size}{colors}\\bord{:.2}\\b{}\\i{}\\u{}",
            if stroke {
                2.0 * scale_x.min(scale_y)
            } else {
                0.0
            },
            usize::from(character.style & 1 != 0),
            usize::from(character.style & (1 << 1) != 0),
            usize::from(character.style & (1 << 2) != 0),
        );
        let spacing_changed = next_spacing.is_some_and(|spacing| {
            (spacing * 100.0).round() as i32 != (*active_spacing * 100.0).round() as i32
        });
        if active_style.as_ref() != Some(&style) || spacing_changed {
            if let Some(spacing) = next_spacing {
                *active_spacing = spacing;
            }
            body.push_str(&format!("{{{style}\\fsp{active_spacing:.2}}}"));
            *active_style = Some(style);
        }
        if let Some(drawing) = drawings.get(&(*character as *const _)) {
            body.push_str(drawing);
        } else {
            body.push_str(&ass_escape(text));
        }
    }
}

fn b24_characters_are_contiguous(
    previous: &native_b24::CaptionCharacter,
    next: &native_b24::CaptionCharacter,
) -> bool {
    if previous.y != next.y || next.x <= previous.x {
        return false;
    }
    // ARIB half-width punctuation can occupy a narrow ink box while its next
    // character still starts on the full caption-cell grid. Judge continuity
    // by the neighbouring cell sizes rather than the previous ink width alone.
    let cell_width = previous.width.abs().max(next.width.abs()).max(1);
    let spacing = previous
        .horizontal_spacing
        .abs()
        .max(next.horizontal_spacing.abs());
    let maximum_advance = cell_width
        .saturating_add(spacing)
        .saturating_add(cell_width / 2)
        .max(4);
    next.x.saturating_sub(previous.x) <= maximum_advance
}

fn write_filtered_ass_character_line(
    writer: &mut BufWriter<File>,
    interval: &RegionInterval,
    line: &[(&native_b24::CaptionCharacter, &str)],
    scale_x: f32,
    scale_y: f32,
    scale_uniform: f32,
    options: &ConversionOptions,
) -> io::Result<()> {
    if options.preserve_gaiji && options.preserve_accessibility {
        return write_ass_character_line(
            writer,
            interval,
            line,
            scale_x,
            scale_y,
            scale_uniform,
            options,
        );
    }
    let combined = line.iter().map(|(_, text)| *text).collect::<String>();
    let retained = crate::caption_features::retained_characters(
        &combined,
        options.preserve_gaiji,
        options.preserve_accessibility,
    );
    let mut cursor = 0_usize;
    let owned = line
        .iter()
        .filter_map(|(character, text)| {
            let filtered = text
                .chars()
                .filter(|_| {
                    let keep = retained.get(cursor).copied().unwrap_or(false);
                    cursor = cursor.saturating_add(1);
                    keep && (options.preserve_gaiji || !b24_character_is_gaiji_source(character))
                })
                .collect::<String>();
            (!filtered.is_empty()).then_some((*character, filtered))
        })
        .collect::<Vec<_>>();
    let borrowed = owned
        .iter()
        .map(|(character, text)| (*character, text.as_str()))
        .collect::<Vec<_>>();
    write_ass_character_line(
        writer,
        interval,
        &borrowed,
        scale_x,
        scale_y,
        scale_uniform,
        options,
    )
}

fn write_ass_character_line(
    writer: &mut BufWriter<File>,
    interval: &RegionInterval,
    line: &[(&native_b24::CaptionCharacter, &str)],
    scale_x: f32,
    scale_y: f32,
    scale_uniform: f32,
    options: &ConversionOptions,
) -> io::Result<()> {
    let Some((first, _)) = line.first() else {
        return Ok(());
    };
    let is_ruby = interval.region.is_ruby;
    if is_ruby && let Some(plan) = ass_ruby_layout_plan(interval, line, scale_x, scale_y, options) {
        return write_ass_ruby_layout(writer, interval, line, &plan, scale_uniform, options);
    }
    let anchor_x = scale_ass_coordinate(first.x, scale_x);
    let anchor_y = scale_ass_coordinate(
        interval
            .ruby_binding
            .as_ref()
            .map(|binding| binding.source_ruby_box.y)
            .unwrap_or(first.y),
        scale_y,
    );
    let alignment = if is_ruby { 8 } else { 7 };
    let mut body = if options.preserve_position {
        format!("{{\\an{alignment}\\pos({anchor_x},{anchor_y})}}")
    } else {
        "{\\an2}".to_owned()
    };
    let mut active_style = None;
    let mut active_spacing = 0.0_f32;
    for (index, (character, text)) in line.iter().enumerate() {
        let ruby_scale = if interval.region.is_ruby { 0.5 } else { 1.0 };
        let font_size = (scale_ass_coordinate(character.height.max(1), scale_y) as f32 * ruby_scale)
            .round()
            .max(1.0) as i32;
        let next_spacing = line.get(index + 1).map(|_| {
            (character.horizontal_spacing as f32 * scale_x)
                .clamp(-(font_size as f32) * 0.5, font_size as f32)
        });
        let stroke = character.style & (1 << 3) != 0;
        let colors = if options.preserve_color {
            {
                format!(
                    "\\c{}\\3c{}",
                    ass_color(character.text_color),
                    ass_color(character.stroke_color)
                )
            }
        } else {
            Default::default()
        };
        let style = format!(
            "\\fs{font_size}{colors}\\bord{:.2}\\b{}\\i{}\\u{}",
            if stroke { 2.0 * scale_uniform } else { 0.0 },
            usize::from(character.style & 1 != 0),
            usize::from(character.style & (1 << 1) != 0),
            usize::from(character.style & (1 << 2) != 0),
        );
        let spacing_changed = next_spacing.is_some_and(|spacing| {
            (spacing * 100.0).round() as i32 != (active_spacing * 100.0).round() as i32
        });
        if active_style.as_ref() != Some(&style) || spacing_changed {
            if let Some(spacing) = next_spacing {
                active_spacing = spacing;
            }
            body.push_str(&format!("{{{style}\\fsp{active_spacing:.2}}}"));
            active_style = Some(style);
        }
        body.push_str(&ass_escape(text));
    }
    let layer = usize::from(interval.region.is_ruby);
    writeln!(
        writer,
        "Dialogue: {layer},{},{},Default,,0,0,0,,{body}",
        ass_time(interval.begin_ms),
        ass_time(interval.end_ms),
    )
}

fn write_ass_ruby_layout(
    writer: &mut BufWriter<File>,
    interval: &RegionInterval,
    line: &[(&native_b24::CaptionCharacter, &str)],
    plan: &RubyLayoutPlan,
    scale_uniform: f32,
    options: &ConversionOptions,
) -> io::Result<()> {
    let source_characters = line
        .iter()
        .flat_map(|(character, text)| text.graphemes(true).map(move |_| *character))
        .collect::<Vec<_>>();
    let fallback_character = line.first().map(|(character, _)| *character);
    for (index, glyph) in plan.glyphs.iter().enumerate() {
        let Some(character) = source_characters.get(index).copied().or(fallback_character) else {
            continue;
        };
        let stroke = character.style & (1 << 3) != 0;
        let colors = if options.preserve_color {
            {
                format!(
                    "\\c{}\\3c{}",
                    ass_color(character.text_color),
                    ass_color(character.stroke_color)
                )
            }
        } else {
            Default::default()
        };
        let style = format!(
            "\\fn{}\\fs{}{colors}\\bord{:.2}\\b{}\\i{}\\u{}",
            plan.font_family,
            plan.font_size,
            if stroke { 2.0 * scale_uniform } else { 0.0 },
            usize::from(character.style & 1 != 0),
            usize::from(character.style & (1 << 1) != 0),
            usize::from(character.style & (1 << 2) != 0),
        );
        writeln!(
            writer,
            "Dialogue: 1,{},{},Default,,0,0,0,,{{\\an8\\pos({},{}){}}}{}",
            ass_time(interval.begin_ms),
            ass_time(interval.end_ms),
            glyph.anchor_x,
            glyph.anchor_y,
            style,
            ass_escape(&glyph.text),
        )?;
    }
    Ok(())
}

fn ass_ruby_layout_plan(
    interval: &RegionInterval,
    ruby_line: &[(&native_b24::CaptionCharacter, &str)],
    scale_x: f32,
    scale_y: f32,
    options: &ConversionOptions,
) -> Option<RubyLayoutPlan> {
    let binding = interval.ruby_binding.as_ref()?;
    let base = &binding.base_characters;
    let first_index = binding.base_start;
    let last_index = binding.base_end.checked_sub(1)?;
    if first_index > last_index || last_index >= base.len() {
        return None;
    }
    let mut segment_start = first_index;
    while segment_start > 0
        && b24_characters_are_contiguous(&base[segment_start - 1], &base[segment_start])
        && b24_character_has_ass_text(&base[segment_start - 1], options)
    {
        segment_start -= 1;
    }
    let mut segment_end = last_index;
    while segment_end + 1 < base.len()
        && b24_characters_are_contiguous(&base[segment_end], &base[segment_end + 1])
        && b24_character_has_ass_text(&base[segment_end + 1], options)
    {
        segment_end += 1;
    }
    let mut cursor = (base[segment_start].x as f32) * scale_x;
    let mut target_ink_left = f32::INFINITY;
    let mut target_ink_right = f32::NEG_INFINITY;
    for (index, character) in base[segment_start..=segment_end].iter().enumerate() {
        let index = segment_start + index;
        let text = if !character.utf8.is_empty() {
            character.utf8.as_str()
        } else if options.preserve_drcs
            && character.kind == 1
            && options.drcs_mode == DrcsMode::UseUserMapping
        {
            options
                .drcs_replacements
                .get(&character.drcs_code)
                .map(String::as_str)
                .unwrap_or("")
        } else {
            ""
        };
        if text.is_empty() {
            continue;
        }
        let size = (character.height.max(1) as f32 * scale_y).max(1.0);
        let (advance, ink_left, ink_right) = ass_text_ink_bounds(text, size);
        if (first_index..=last_index).contains(&index) {
            target_ink_left = target_ink_left.min(cursor + ink_left);
            target_ink_right = target_ink_right.max(cursor + ink_right);
        }
        cursor += advance;
        if index < segment_end {
            cursor += (character.horizontal_spacing as f32 * scale_x).clamp(-size * 0.5, size);
        }
    }
    if !(target_ink_left.is_finite() && target_ink_right.is_finite()) {
        return None;
    }
    let ruby_text = ruby_line.iter().map(|(_, text)| *text).collect::<String>();
    let preferred_font_size = ruby_line
        .iter()
        .map(|(character, _)| {
            (scale_ass_coordinate(character.height.max(1), scale_y) as f32 * 0.5)
                .round()
                .max(1.0) as i32
        })
        .max()?;
    let container_left = target_ink_left.floor() as i32;
    let container_right = target_ink_right.ceil() as i32;
    layout_ruby(
        &RubyLayoutRequest {
            text: &ruby_text,
            container: RubyLayoutBox {
                x: container_left,
                y: scale_ass_coordinate(binding.source_ruby_box.y, scale_y),
                width: container_right.saturating_sub(container_left).max(1),
                height: scale_ass_coordinate(binding.source_ruby_box.height, scale_y).max(1),
            },
            preferred_font_size,
            minimum_font_size: 6,
            placement: binding.placement,
            writing_mode: binding.writing_mode,
        },
        &BundledAssGlyphMetrics,
    )
}

fn b24_character_has_ass_text(
    character: &native_b24::CaptionCharacter,
    options: &ConversionOptions,
) -> bool {
    if !options.preserve_gaiji && b24_character_is_gaiji_source(character) {
        return false;
    }
    if !character.utf8.is_empty() {
        return keep_text(&character.utf8, options);
    }
    options.preserve_drcs
        && character.kind == 1
        && options.drcs_mode == DrcsMode::UseUserMapping
        && options
            .drcs_replacements
            .get(&character.drcs_code)
            .is_some_and(|text| keep_text(text, options))
}

fn scale_ass_coordinate(value: i32, scale: f32) -> i32 {
    (value as f32 * scale).round() as i32
}

pub(crate) fn write_ass_font_directory(output: &Path, overwrite: bool) -> io::Result<PathBuf> {
    let directory = output.with_extension("fonts");
    fs::create_dir_all(&directory)?;
    let font = directory.join("rounded-mplus-1m-arib.ttf");
    let license = directory.join("LICENSE.rounded-mplus-1m-arib.txt");
    if !overwrite && (font.exists() || license.exists()) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "ASS font sidecar already exists",
        ));
    }
    let font_part = font.with_extension("ttf.part");
    fs::write(&font_part, bundled_ass_font())?;
    publish_file(&font_part, &font, true)?;
    let license_part = license.with_extension("txt.part");
    fs::write(&license_part, ASS_ARIB_FONT_LICENSE)?;
    publish_file(&license_part, &license, true)?;
    Ok(directory)
}
