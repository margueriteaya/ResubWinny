use super::*;

pub(crate) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(crate) fn ttml_clock(milliseconds: i64) -> String {
    let milliseconds = milliseconds.max(0);
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        milliseconds / 3_600_000,
        (milliseconds / 60_000) % 60,
        (milliseconds / 1_000) % 60,
        milliseconds % 1_000
    )
}

pub(crate) fn ttml_color(color: u32) -> String {
    format!("#{:06X}", color & 0x00ff_ffff)
}

pub(crate) fn ass_color_from_ttml(value: &str) -> Option<String> {
    let hex = value.trim().strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let alpha = if hex.len() == 8 {
        255_u8.saturating_sub(u8::from_str_radix(&hex[6..8], 16).ok()?)
    } else {
        0
    };
    Some(format!("&H{alpha:02X}{blue:02X}{green:02X}{red:02X}&"))
}

pub(crate) fn ass_font_size_from_ttml(value: &str) -> Option<i32> {
    let (_, height) = ass_font_dimensions_from_ttml(value)?;
    height
        .is_finite()
        .then_some(height.round() as i32)
        .filter(|size| (1..=512).contains(size))
}

pub(crate) fn ass_font_dimensions_from_ttml(value: &str) -> Option<(f32, f32)> {
    let mut dimensions = value.split_whitespace().map(|part| {
        part.strip_suffix("px")?
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite() && (1.0..=512.0).contains(value))
    });
    let width = dimensions.next()??;
    let height = dimensions.next().flatten().unwrap_or(width);
    Some((width, height))
}

pub(crate) fn ass_letter_spacing_from_ttml(value: &str) -> Option<i32> {
    let pixels = value
        .trim()
        .strip_suffix("px")?
        .trim()
        .parse::<f32>()
        .ok()?;
    pixels
        .is_finite()
        .then_some(pixels.round() as i32)
        .filter(|spacing| (-128..=128).contains(spacing))
}

pub(crate) fn write_ttml_header(writer: &mut BufWriter<File>) -> io::Result<()> {
    writer.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<tt xmlns=\"http://www.w3.org/ns/ttml\" xmlns:tts=\"http://www.w3.org/ns/ttml#styling\" xmlns:arib=\"https://resubwinny.dev/ns/arib\" xml:lang=\"ja\">\n  <body>\n    <div>\n")
}

pub(crate) fn write_ttml_footer(writer: &mut BufWriter<File>) -> io::Result<()> {
    writer.write_all(b"    </div>\n  </body>\n</tt>\n")
}

pub(crate) fn interval_ttml_text(interval: &RegionInterval, options: &ConversionOptions) -> String {
    struct Cell {
        classifier_text: String,
        markup: Option<String>,
        source_gaiji: bool,
    }
    let cells = interval
        .characters
        .iter()
        .filter_map(|character| {
            if !character.utf8.is_empty() {
                return Some(Cell {
                    classifier_text: character.utf8.clone(),
                    markup: None,
                    source_gaiji: b24_character_is_gaiji_source(character),
                });
            }
            if character.kind != 1 || !options.preserve_drcs {
                return None;
            }
            if options.drcs_mode == DrcsMode::UseUserMapping
                && let Some(replacement) = options.drcs_replacements.get(&character.drcs_code)
            {
                return Some(Cell {
                    classifier_text: replacement.clone(),
                    markup: None,
                    source_gaiji: false,
                });
            }
            let glyph = interval
                .drcs_glyphs
                .iter()
                .find(|glyph| glyph.drcs_code == character.drcs_code);
            let alternative = glyph
                .map(|glyph| glyph.alternative_text.as_str())
                .filter(|value| !value.is_empty());
            Some(match alternative {
                Some(text) => Cell {
                    classifier_text: text.to_owned(),
                    markup: None,
                    source_gaiji: false,
                },
                None => Cell {
                    classifier_text: "\u{FFFC}".to_owned(),
                    markup: Some(format!(
                        "<span arib:drcs-code=\"0x{:X}\"{}>\u{FFFC}</span>",
                        character.drcs_code,
                        glyph
                            .map(|glyph| format!(" arib:drcs-md5=\"{}\"", xml_escape(&glyph.md5)))
                            .unwrap_or_default()
                    )),
                    source_gaiji: false,
                },
            })
        })
        .collect::<Vec<_>>();
    let combined = cells
        .iter()
        .map(|cell| cell.classifier_text.as_str())
        .collect::<String>();
    let mut retained = crate::caption_features::retained_characters(
        &combined,
        options.preserve_gaiji,
        options.preserve_accessibility,
    );
    if !options.preserve_gaiji {
        let mut source_cursor = 0_usize;
        for cell in &cells {
            let end = source_cursor.saturating_add(cell.classifier_text.chars().count());
            if cell.source_gaiji {
                retained[source_cursor..end].fill(false);
            }
            source_cursor = end;
        }
    }
    let mut cursor = 0_usize;
    cells
        .into_iter()
        .filter_map(|cell| {
            let filtered = cell
                .classifier_text
                .chars()
                .filter(|_| {
                    let keep = retained.get(cursor).copied().unwrap_or(false);
                    cursor = cursor.saturating_add(1);
                    keep
                })
                .collect::<String>();
            if filtered.is_empty() {
                None
            } else if let Some(markup) = cell.markup {
                Some(markup)
            } else {
                Some(xml_escape(&filtered))
            }
        })
        .collect()
}

pub(crate) fn write_ttml_interval(
    writer: &mut BufWriter<File>,
    interval: &RegionInterval,
    options: &ConversionOptions,
) -> io::Result<()> {
    let text = interval_ttml_text(interval, options);
    if text.is_empty() {
        return Ok(());
    }
    let first = interval.characters.first().expect("non-empty interval");
    let position = if options.preserve_position {
        {
            format!(
                " tts:origin=\"{}px {}px\" tts:extent=\"{}px {}px\"",
                interval.region.x,
                interval.region.y,
                interval.region.width.max(1),
                interval.region.height.max(1),
            )
        }
    } else {
        Default::default()
    };
    let color = if options.preserve_color {
        format!(" tts:color=\"{}\"", ttml_color(first.text_color))
    } else {
        Default::default()
    };
    writeln!(
        writer,
        "      <p begin=\"{}\" end=\"{}\"{} tts:fontSize=\"{}px\"{}>{}</p>",
        ttml_clock(interval.begin_ms),
        ttml_clock(interval.end_ms),
        position,
        first.height.max(1),
        color,
        text,
    )
}

pub(crate) fn write_ttml_caption(
    writer: &mut BufWriter<File>,
    caption: &TtmlCaption,
    options: &ConversionOptions,
) -> io::Result<()> {
    let filtered_text = export_ttml_text(&caption.text, &caption.style, options);
    if filtered_text.is_empty() {
        return Ok(());
    }
    let mut style = String::new();
    if let (Some(width), Some(height)) = (caption.width, caption.height) {
        style.push_str(&format!(" tts:extent=\"{width}px {height}px\""));
    }
    if options.preserve_color
        && let Some(color) = &caption.style.color
    {
        style.push_str(&format!(" tts:color=\"{}\"", xml_escape(color)));
    }
    if options.preserve_color
        && let Some(background_color) = &caption.style.background_color
    {
        style.push_str(&format!(
            " tts:backgroundColor=\"{}\"",
            xml_escape(background_color)
        ));
    }
    if let Some(font_size) = &caption.style.font_size {
        style.push_str(&format!(" tts:fontSize=\"{}\"", xml_escape(font_size)));
    }
    for (name, value) in [
        ("fontFamily", caption.style.font_family.as_deref()),
        ("fontStyle", caption.style.font_style.as_deref()),
        ("fontWeight", caption.style.font_weight.as_deref()),
        ("direction", caption.style.direction.as_deref()),
        ("textAlign", caption.style.text_align.as_deref()),
        ("textOutline", caption.style.text_outline.as_deref()),
        ("lineHeight", caption.style.line_height.as_deref()),
        ("letterSpacing", caption.style.letter_spacing.as_deref()),
        ("opacity", caption.style.opacity.as_deref()),
        ("displayAlign", caption.style.display_align.as_deref()),
    ] {
        if let Some(value) = value {
            style.push_str(&format!(" tts:{name}=\"{}\"", xml_escape(value)));
        }
    }
    if let Some(writing_mode) = &caption.style.writing_mode {
        style.push_str(&format!(
            " tts:writingMode=\"{}\"",
            xml_escape(writing_mode)
        ));
    }
    let body = caption
        .rich_body
        .as_deref()
        .and_then(|body| filter_ttml_preserved_body(body, &caption.style, options))
        .unwrap_or_else(|| xml_escape(&filtered_text));
    let body = strip_ttml_font_resource_attributes(&body);
    if ttml_plain_text(&body).is_empty() {
        return Ok(());
    }
    writeln!(
        writer,
        "      <p begin=\"{}\" end=\"{}\"{}{}>{}</p>",
        ttml_clock(caption.start_ms),
        ttml_clock(caption.end_ms),
        if options.preserve_position {
            format!(" tts:origin=\"{}px {}px\"", caption.x, caption.y)
        } else {
            Default::default()
        },
        style,
        body,
    )
}
