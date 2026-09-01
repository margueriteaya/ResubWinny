use super::*;

#[derive(Clone, Copy)]
pub(super) struct HorizontalLine {
    pub(super) width: f32,
    pub(super) glyph_height: f32,
    pub(super) ruby_height: f32,
}

pub(super) fn horizontal_lines(font: &Font, runs: &[StyledRun]) -> Vec<HorizontalLine> {
    let mut lines = Vec::new();
    let mut width = 0.0;
    let mut glyph_height = 0.0_f32;
    let mut ruby_height = 0.0_f32;
    for run in runs {
        for character in run.text.chars() {
            if character == '\n' {
                lines.push(HorizontalLine {
                    width,
                    glyph_height: glyph_height.max(run.font_size),
                    ruby_height,
                });
                width = 0.0;
                glyph_height = 0.0;
                ruby_height = 0.0;
            } else {
                width += measure_text(font, &character.to_string(), run.font_size, 0.0)
                    + run.letter_spacing;
                glyph_height = glyph_height.max(run.font_size);
                if run
                    .ruby_text
                    .as_deref()
                    .is_some_and(|ruby| !ruby.is_empty())
                {
                    ruby_height = ruby_height.max(ruby_style(run).font_size);
                }
            }
        }
    }
    lines.push(HorizontalLine {
        width,
        glyph_height: glyph_height.max(runs.first().map(|run| run.font_size).unwrap_or(42.0)),
        ruby_height,
    });
    lines
}

fn aligned_line_x(
    x: i32,
    width: i32,
    line_width: f32,
    text_align: Option<&str>,
    right_to_left: bool,
) -> f32 {
    let available = width.max(line_width.ceil() as i32) as f32;
    let offset = match text_align.unwrap_or("start").trim() {
        "center" => (available - line_width) * 0.5,
        "end" => {
            if right_to_left {
                0.0
            } else {
                available - line_width
            }
        }
        "right" => available - line_width,
        "left" => 0.0,
        _ if right_to_left => available - line_width,
        _ => 0.0,
    };
    x as f32 + offset.max(0.0)
}

fn aligned_block_y(y: i32, height: i32, block_height: f32, display_align: Option<&str>) -> i32 {
    let available = height.max(block_height.ceil() as i32) as f32;
    let offset = match display_align.unwrap_or("before").trim() {
        "center" => (available - block_height) * 0.5,
        "after" => available - block_height,
        _ => 0.0,
    };
    y.saturating_add(offset.max(0.0).round() as i32)
}

#[allow(
    clippy::too_many_arguments,
    reason = "the allocation-free renderer keeps geometry and diagnostics explicit"
)]
pub(super) fn draw_horizontal_text(
    canvas: &mut [u8],
    font: &Font,
    runs: &[StyledRun],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    right_to_left: bool,
    text_align: Option<&str>,
    display_align: Option<&str>,
    line_height: f32,
    missing_glyph_count: &mut usize,
) -> (bool, usize) {
    let lines = horizontal_lines(font, runs);
    let line_heights: Vec<f32> = lines
        .iter()
        .map(|line| line_height.max(line.glyph_height + line.ruby_height))
        .collect();
    let block_y = aligned_block_y(y, height, line_heights.iter().sum(), display_align);
    let mut line_index = 0;
    let mut cursor_x = aligned_line_x(x, width, lines[0].width, text_align, right_to_left);
    if right_to_left {
        cursor_x += lines[0].width;
    }
    let mut drawn = false;
    let mut ruby_count = 0;
    let mut line_y = block_y;
    let mut ruby_base_start = None;
    for run in runs {
        if !run.ruby_base {
            ruby_base_start = None;
        }
        let segments = run.text.split('\n').collect::<Vec<_>>();
        for (segment_index, segment) in segments.iter().enumerate() {
            let line = lines[line_index];
            let baseline = line_y
                .saturating_add(line.ruby_height.ceil() as i32)
                .saturating_add(line.glyph_height.ceil() as i32);
            let run_start = cursor_x;
            let segment_width = measure_text(font, segment, run.font_size, run.letter_spacing);
            if !segment.is_empty() && run.background[3] > 0 {
                let run_end = if right_to_left {
                    run_start - segment_width
                } else {
                    run_start + segment_width
                };
                fill_rect(
                    canvas,
                    run_start.min(run_end).floor() as i32,
                    line_y.saturating_add(line.ruby_height.ceil() as i32),
                    segment_width.ceil() as i32,
                    line_height.max(line.glyph_height).ceil() as i32,
                    run.background,
                );
            }
            if run.ruby_base && ruby_base_start.is_none() {
                ruby_base_start = Some(run_start);
            }
            if right_to_left {
                for character in segment.chars().rev() {
                    let advance = measure_text(font, &character.to_string(), run.font_size, 0.0);
                    cursor_x -= advance + run.letter_spacing;
                    let drawn_advance = draw_character(
                        canvas,
                        font,
                        character,
                        cursor_x.round() as i32,
                        baseline,
                        run.font_size,
                        run.color,
                        run.outline,
                        missing_glyph_count,
                    );
                    drawn |= drawn_advance > 0.0;
                }
            } else {
                for character in segment.chars() {
                    let advance = draw_character(
                        canvas,
                        font,
                        character,
                        cursor_x.round() as i32,
                        baseline,
                        run.font_size,
                        run.color,
                        run.outline,
                        missing_glyph_count,
                    );
                    cursor_x += advance + run.letter_spacing;
                    drawn |= advance > 0.0;
                }
            }
            if segments.len() == 1
                && let Some(ruby) = run.ruby_text.as_deref().filter(|ruby| !ruby.is_empty())
            {
                let ruby_style = ruby_style(run);
                let ruby_size = ruby_style.font_size;
                let ruby_width = measure_text(font, ruby, ruby_size, ruby_style.letter_spacing);
                let group_start = if run.ruby_group_base_count > 1 {
                    ruby_base_start.unwrap_or(run_start)
                } else {
                    run_start
                };
                let base_width = (cursor_x - group_start).abs();
                let run_left = group_start.min(cursor_x);
                let mut ruby_x = run_left + (base_width - ruby_width) * 0.5;
                let ruby_baseline = line_y.saturating_add(ruby_size.ceil() as i32);
                if right_to_left {
                    ruby_x += ruby_width;
                    for character in ruby.chars().rev() {
                        let advance = measure_text(font, &character.to_string(), ruby_size, 0.0);
                        ruby_x -= advance + ruby_style.letter_spacing;
                        let drawn_advance = draw_character(
                            canvas,
                            font,
                            character,
                            ruby_x.round() as i32,
                            ruby_baseline,
                            ruby_size,
                            ruby_style.color,
                            ruby_style.outline,
                            missing_glyph_count,
                        );
                        drawn |= drawn_advance > 0.0;
                    }
                } else {
                    for character in ruby.chars() {
                        let advance = draw_character(
                            canvas,
                            font,
                            character,
                            ruby_x.round() as i32,
                            ruby_baseline,
                            ruby_size,
                            ruby_style.color,
                            ruby_style.outline,
                            missing_glyph_count,
                        );
                        ruby_x += advance + ruby_style.letter_spacing;
                        drawn |= advance > 0.0;
                    }
                }
                ruby_count += 1;
                ruby_base_start = None;
            }
            if segment_index + 1 < segments.len() {
                line_y = line_y.saturating_add(line_heights[line_index].round() as i32);
                line_index += 1;
                cursor_x =
                    aligned_line_x(x, width, lines[line_index].width, text_align, right_to_left);
                if right_to_left {
                    cursor_x += lines[line_index].width;
                }
            }
        }
    }
    (drawn, ruby_count)
}

#[allow(
    clippy::too_many_arguments,
    reason = "the allocation-free renderer keeps geometry and diagnostics explicit"
)]
pub(super) fn draw_vertical_text(
    canvas: &mut [u8],
    font: &Font,
    runs: &[StyledRun],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    writing_mode: &str,
    missing_glyph_count: &mut usize,
) -> (bool, usize) {
    let initial_size = runs.first().map(|run| run.font_size).unwrap_or(42.0);
    let limit_y = y.saturating_add(height.max(initial_size.ceil() as i32));
    let right_to_left = writing_mode.ends_with("rl");
    let mut cursor_x = if right_to_left {
        x.saturating_add(width.max(initial_size.ceil() as i32))
            .saturating_sub(initial_size.ceil() as i32)
    } else {
        x
    };
    let mut cursor_y = y;
    let mut drawn = false;
    let mut ruby_count = 0;
    let mut ruby_group_cells = Vec::new();
    let mut ruby_group_wrapped = false;
    for run in runs {
        if !run.ruby_base {
            ruby_group_cells.clear();
            ruby_group_wrapped = false;
        }
        let run_start_x = cursor_x;
        let run_start_y = cursor_y;
        let mut run_wrapped = false;
        let mut base_cells = Vec::new();
        let cell = (run.font_size + run.letter_spacing).max(1.0);
        let characters: Vec<char> = run.text.chars().collect();
        let mut character_index = 0;
        while character_index < characters.len() {
            let character = characters[character_index];
            if character == '\n' || cursor_y.saturating_add(run.font_size.ceil() as i32) > limit_y {
                run_wrapped = true;
                cursor_y = y;
                cursor_x = if right_to_left {
                    cursor_x.saturating_sub(cell.ceil() as i32)
                } else {
                    cursor_x.saturating_add(cell.ceil() as i32)
                };
                if character == '\n' {
                    character_index += 1;
                    continue;
                }
            }
            if run.background[3] > 0 {
                fill_rect(
                    canvas,
                    cursor_x,
                    cursor_y,
                    run.font_size.ceil() as i32,
                    cell.ceil() as i32,
                    run.background,
                );
            }
            base_cells.push((cursor_x, cursor_y));
            let combined_digit_count = run
                .text_combine
                .then(|| text_combine_digit_count(&characters, character_index))
                .flatten();
            let advance = if let Some(digit_count) = combined_digit_count {
                let digits: String = characters[character_index..character_index + digit_count]
                    .iter()
                    .collect();
                draw_tate_chu_yoko(
                    canvas,
                    font,
                    &digits,
                    cursor_x,
                    cursor_y,
                    run,
                    missing_glyph_count,
                )
            } else {
                draw_vertical_character(
                    canvas,
                    font,
                    character,
                    cursor_x,
                    cursor_y.saturating_add(run.font_size.ceil() as i32),
                    run.font_size,
                    run.color,
                    run.outline,
                    missing_glyph_count,
                )
            };
            drawn |= advance > 0.0;
            cursor_y = cursor_y.saturating_add(cell.ceil() as i32);
            character_index += combined_digit_count.unwrap_or(1);
        }
        if run.ruby_base {
            ruby_group_cells.extend_from_slice(&base_cells);
            ruby_group_wrapped |= run_wrapped;
        }
        if let Some(ruby) = run.ruby_text.as_deref().filter(|ruby| !ruby.is_empty()) {
            let (annotation_cells, annotation_wrapped) = if run.ruby_group_base_count > 1 {
                (ruby_group_cells.as_slice(), ruby_group_wrapped)
            } else {
                (base_cells.as_slice(), run_wrapped)
            };
            if annotation_wrapped {
                drawn |= draw_wrapped_vertical_ruby(
                    canvas,
                    font,
                    ruby,
                    annotation_cells,
                    run,
                    right_to_left,
                    missing_glyph_count,
                );
                ruby_count += 1;
            } else {
                let ruby_style = ruby_style(run);
                let ruby_size = ruby_style.font_size;
                let ruby_height = ruby.chars().count() as f32 * ruby_size;
                let (group_x, group_y, group_height) = annotation_cells
                    .first()
                    .map(|(first_x, first_y)| {
                        let last_y = annotation_cells
                            .last()
                            .map(|(_, last_y)| *last_y)
                            .unwrap_or(*first_y);
                        (
                            *first_x,
                            *first_y,
                            (last_y - *first_y).max(0) as f32 + run.font_size,
                        )
                    })
                    .unwrap_or((run_start_x, run_start_y, run.font_size));
                let mut ruby_y = group_y as f32 + (group_height - ruby_height) * 0.5;
                let ruby_x = group_x.saturating_add((run.font_size * 0.55).ceil() as i32);
                for character in ruby.chars() {
                    let advance = draw_vertical_character(
                        canvas,
                        font,
                        character,
                        ruby_x,
                        ruby_y.round() as i32 + ruby_size.ceil() as i32,
                        ruby_size,
                        ruby_style.color,
                        ruby_style.outline,
                        missing_glyph_count,
                    );
                    ruby_y += ruby_size;
                    drawn |= advance > 0.0;
                }
                ruby_count += 1;
            }
            ruby_group_cells.clear();
            ruby_group_wrapped = false;
        }
    }
    (drawn, ruby_count)
}

/// Returns a bounded, explicitly requested tate-chu-yoko sequence. We only
/// combine one or two ASCII digits: longer runs and other scripts keep the
/// ordinary vertical path instead of inventing B62 orientation behaviour.
pub(super) fn text_combine_digit_count(characters: &[char], start: usize) -> Option<usize> {
    if !characters.get(start)?.is_ascii_digit() {
        return None;
    }
    let count = characters[start..]
        .iter()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if !(1..=2).contains(&count) {
        return None;
    }
    // ASCII-only input makes byte and character offsets equivalent.
    Some(count)
}

fn draw_tate_chu_yoko(
    canvas: &mut [u8],
    font: &Font,
    digits: &str,
    x: i32,
    y: i32,
    run: &StyledRun,
    missing_glyph_count: &mut usize,
) -> f32 {
    let size = (run.font_size * 0.52).clamp(6.0, 80.0);
    let width: f32 = digits
        .chars()
        .map(|character| font.metrics(character, size).advance_width)
        .sum();
    let mut cursor_x = x as f32 + (run.font_size - width).max(0.0) * 0.5;
    let baseline = y.saturating_add(((run.font_size + size) * 0.5).round() as i32);
    let mut drawn = false;
    for character in digits.chars() {
        let advance = draw_character(
            canvas,
            font,
            character,
            cursor_x.round() as i32,
            baseline,
            size,
            run.color,
            run.outline,
            missing_glyph_count,
        );
        cursor_x += advance;
        drawn |= advance > 0.0;
    }
    if drawn { width.max(1.0) } else { 0.0 }
}

/// Continues an explicitly associated ruby annotation over the same vertical
/// reading path when its base run crosses columns. The annotation is spaced by
/// progress along base cells instead of being silently discarded. This does not
/// claim support for arbitrary ruby placement/merge rules; it is only used for
/// the existing explicit base/text association retained in the archive model.
fn draw_wrapped_vertical_ruby(
    canvas: &mut [u8],
    font: &Font,
    ruby: &str,
    base_cells: &[(i32, i32)],
    run: &StyledRun,
    right_to_left: bool,
    missing_glyph_count: &mut usize,
) -> bool {
    let ruby_characters: Vec<_> = ruby.chars().collect();
    if ruby_characters.is_empty() || base_cells.is_empty() {
        return false;
    }
    let ruby_style = ruby_style(run);
    let ruby_size = ruby_style.font_size;
    let ruby_x_offset = (run.font_size * 0.55).ceil() as i32;
    let mut drawn = false;
    for (index, character) in ruby_characters.iter().enumerate() {
        let progress =
            (index as f32 + 0.5) * base_cells.len() as f32 / ruby_characters.len() as f32;
        let cell_index = progress.floor().min((base_cells.len() - 1) as f32) as usize;
        let within_cell = progress.fract();
        let (base_x, base_y) = base_cells[cell_index];
        let ruby_x = if right_to_left {
            base_x.saturating_add(ruby_x_offset)
        } else {
            base_x.saturating_sub(ruby_size.ceil() as i32)
        };
        let ruby_baseline = base_y
            .saturating_add((within_cell * run.font_size).round() as i32)
            .saturating_add(ruby_size.ceil() as i32);
        let advance = draw_vertical_character(
            canvas,
            font,
            *character,
            ruby_x,
            ruby_baseline,
            ruby_size,
            ruby_style.color,
            ruby_style.outline,
            missing_glyph_count,
        );
        drawn |= advance > 0.0;
    }
    drawn
}

/// Maps only punctuation with an explicit Unicode vertical presentation form.
/// Explicit one/two-digit `tts:textCombine` is handled separately; Latin
/// rotation and ARIB-specific orientation rules still require verified B62
/// samples and deliberately remain outside this bounded fallback.
pub(super) fn vertical_presentation_form(character: char) -> Option<char> {
    Some(match character {
        '、' | '，' => '\u{FE11}',
        '。' | '．' => '\u{FE12}',
        '：' => '\u{FE13}',
        '；' => '\u{FE14}',
        '！' => '\u{FE15}',
        '？' => '\u{FE16}',
        '…' => '\u{FE19}',
        '‥' => '\u{FE30}',
        '—' => '\u{FE31}',
        '–' => '\u{FE32}',
        '_' => '\u{FE33}',
        '(' | '（' => '\u{FE35}',
        ')' | '）' => '\u{FE36}',
        '{' | '｛' => '\u{FE37}',
        '}' | '｝' => '\u{FE38}',
        '〔' => '\u{FE39}',
        '〕' => '\u{FE3A}',
        '【' => '\u{FE3B}',
        '】' => '\u{FE3C}',
        '《' => '\u{FE3D}',
        '》' => '\u{FE3E}',
        '〈' => '\u{FE3F}',
        '〉' => '\u{FE40}',
        '「' => '\u{FE41}',
        '」' => '\u{FE42}',
        '『' => '\u{FE43}',
        '』' => '\u{FE44}',
        '[' | '［' => '\u{FE47}',
        ']' | '］' => '\u{FE48}',
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum VerticalGlyphOrientation {
    Upright,
    RotateClockwise,
}

/// Conservative native subset of Unicode Vertical_Orientation. CJK and
/// full-width characters stay upright; ordinary Latin/ASCII characters rotate
/// unless a vertical presentation form was selected first. This deliberately
/// keeps unclassified scripts upright instead of guessing a B62-specific
/// orientation rule.
pub(super) fn vertical_glyph_orientation(character: char) -> VerticalGlyphOrientation {
    if character.is_ascii_graphic()
        || matches!(character, '\u{00A0}'..='\u{02AF}' | '\u{1E00}'..='\u{1EFF}')
    {
        VerticalGlyphOrientation::RotateClockwise
    } else {
        VerticalGlyphOrientation::Upright
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the glyph hot path keeps drawing state explicit and allocation-free"
)]
fn draw_vertical_character(
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
    let vertical = vertical_presentation_form(character)
        .filter(|presentation| font.lookup_glyph_index(*presentation) != 0)
        .unwrap_or(character);
    match vertical_glyph_orientation(vertical) {
        VerticalGlyphOrientation::Upright => draw_character(
            canvas,
            font,
            vertical,
            x,
            baseline,
            font_size,
            foreground,
            outline,
            missing_glyph_count,
        ),
        VerticalGlyphOrientation::RotateClockwise => draw_rotated_vertical_character(
            canvas,
            font,
            vertical,
            x,
            baseline,
            font_size,
            foreground,
            outline,
            missing_glyph_count,
        ),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the glyph hot path keeps drawing state explicit and allocation-free"
)]
fn draw_rotated_vertical_character(
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
    if bitmap.len() > MAX_TEXT_PIXELS || metrics.width == 0 || metrics.height == 0 {
        return font_size;
    }
    let rotated = rotate_bitmap_clockwise(&bitmap, metrics.width, metrics.height);
    let rotated_width = metrics.height;
    let rotated_height = metrics.width;
    let cell_size = font_size.ceil() as i32;
    let glyph_x = x.saturating_add((cell_size - rotated_width as i32) / 2);
    let glyph_y = baseline
        .saturating_sub(cell_size)
        .saturating_add((cell_size - rotated_height as i32) / 2);
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
                    rotated_width,
                    rotated_height,
                    &rotated,
                    outline.color,
                );
            }
        }
    }
    blend_glyph(
        canvas,
        glyph_x,
        glyph_y,
        rotated_width,
        rotated_height,
        &rotated,
        foreground,
    );
    font_size
}

pub(super) fn rotate_bitmap_clockwise(bitmap: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rotated = vec![0; bitmap.len()];
    for source_y in 0..height {
        for source_x in 0..width {
            let destination_x = height - source_y - 1;
            let destination_y = source_x;
            rotated[destination_y * height + destination_x] = bitmap[source_y * width + source_x];
        }
    }
    rotated
}
