use super::*;

pub(super) fn styled_runs(
    interval: &Value,
    fallback_text: &str,
    color: [u8; 4],
    font_size: f32,
    letter_spacing: f32,
    outline: Option<TextOutline>,
    opacity: f32,
) -> Vec<StyledRun> {
    let base = StyledRun {
        text: String::new(),
        id: None,
        ruby_target_id: None,
        color,
        font_size,
        letter_spacing,
        outline,
        ruby_text: None,
        ruby_style: None,
        ruby_base: false,
        ruby_group_base_count: 0,
        text_combine: false,
    };
    let Some(body) = interval
        .get("rich_body")
        .or_else(|| interval.get("richBody"))
        .and_then(Value::as_str)
    else {
        return vec![StyledRun {
            text: fallback_text.into(),
            ..base
        }];
    };
    let mut runs = Vec::new();
    let mut remaining = body;
    while !remaining.is_empty() {
        let Some(tag_start) = remaining.find('<') else {
            push_text_run(&mut runs, remaining, &base);
            break;
        };
        push_text_run(&mut runs, &remaining[..tag_start], &base);
        let Some(tag_end) = remaining[tag_start..].find('>') else {
            push_text_run(&mut runs, &remaining[tag_start..], &base);
            break;
        };
        let tag_end = tag_start + tag_end;
        let tag = &remaining[tag_start..=tag_end];
        if tag.starts_with("<br") {
            push_text_run(&mut runs, "\n", &base);
            remaining = &remaining[tag_end + 1..];
            continue;
        }
        if !tag.starts_with("<span") {
            remaining = &remaining[tag_end + 1..];
            continue;
        }
        let content = &remaining[tag_end + 1..];
        let Some((close_start, close_end)) = matching_span_end(content) else {
            break;
        };
        let text = xml_unescape(&strip_tags(&content[..close_start]));
        let role = xml_attribute(tag, "tts:ruby").or_else(|| xml_attribute(tag, "ruby"));
        if role.is_some_and(|value| value == "text") {
            attach_ruby_to_trailing_bases(&mut runs, text, tag, opacity);
        } else if !text.is_empty() {
            let span_opacity = opacity * parse_opacity(xml_attribute(tag, "tts:opacity"));
            let ruby_target_id = xml_attribute(tag, "arib-tt:ruby").map(str::to_owned);
            let annotation_has_explicit_style = [
                "tts:color",
                "tts:fontSize",
                "tts:letterSpacing",
                "tts:textOutline",
                "tts:opacity",
            ]
            .into_iter()
            .any(|attribute| xml_attribute(tag, attribute).is_some());
            let span = StyledRun {
                text,
                id: xml_attribute(tag, "xml:id")
                    .or_else(|| xml_attribute(tag, "id"))
                    .map(str::to_owned),
                ruby_target_id: ruby_target_id.clone(),
                color: apply_opacity(
                    xml_attribute(tag, "tts:color")
                        .map(parse_rgba)
                        .unwrap_or(base.color),
                    span_opacity,
                ),
                font_size: parse_font_height(xml_attribute(tag, "tts:fontSize"))
                    .unwrap_or(base.font_size)
                    .clamp(8.0, 160.0),
                letter_spacing: parse_px(xml_attribute(tag, "tts:letterSpacing"))
                    .unwrap_or(base.letter_spacing)
                    .clamp(-20.0, 80.0),
                outline: parse_text_outline(xml_attribute(tag, "tts:textOutline"), span_opacity)
                    .or(base.outline),
                ruby_text: None,
                ruby_style: ruby_target_id
                    .as_ref()
                    .filter(|_| annotation_has_explicit_style)
                    .map(|_| ruby_style_from_tag(tag, &base, opacity)),
                ruby_base: role.is_some_and(|value| value == "base"),
                ruby_group_base_count: 0,
                text_combine: xml_attribute(tag, "tts:textCombine")
                    .or_else(|| xml_attribute(tag, "textCombine"))
                    .is_some_and(|value| matches!(value.trim(), "all" | "digits")),
            };
            // For ordinary nested styling, recurse through the already bounded
            // inline grammar so a child span does not lose its explicit colour
            // or size. Ruby/id association remains structural until its own
            // placement rules are proven, rather than being inherited blindly.
            if role.is_none()
                && span.id.is_none()
                && span.ruby_target_id.is_none()
                && content[..close_start].contains("<span")
            {
                let nested = serde_json::json!({ "rich_body": &content[..close_start] });
                runs.extend(styled_runs(
                    &nested,
                    "",
                    span.color,
                    span.font_size,
                    span.letter_spacing,
                    span.outline,
                    1.0,
                ));
            } else {
                runs.push(span);
            }
        }
        remaining = &content[close_end..];
    }
    resolve_arib_ruby_targets(&mut runs);
    if runs.is_empty() {
        vec![StyledRun {
            text: fallback_text.into(),
            ..base
        }]
    } else {
        runs
    }
}

/// Finds the closing tag for the current `<span>` while tolerating nested
/// inline spans. The renderer still applies the outer validated style to this
/// bounded run; it must not turn a nested closing tag into visible text.
fn matching_span_end(content: &str) -> Option<(usize, usize)> {
    let mut depth = 1_usize;
    let mut cursor = 0;
    while let Some(relative_start) = content[cursor..].find('<') {
        let start = cursor + relative_start;
        let relative_end = content[start..].find('>')?;
        let end = start + relative_end + 1;
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

fn push_text_run(runs: &mut Vec<StyledRun>, raw: &str, base: &StyledRun) {
    let text = xml_unescape(raw);
    if !text.is_empty() {
        runs.push(StyledRun {
            text,
            id: None,
            ruby_target_id: None,
            color: base.color,
            font_size: base.font_size,
            letter_spacing: base.letter_spacing,
            outline: base.outline,
            ruby_text: None,
            ruby_style: None,
            ruby_base: false,
            ruby_group_base_count: 0,
            text_combine: false,
        });
    }
}

/// Associates a ruby annotation with every immediately preceding explicit ruby
/// base span. This mirrors the recursive inline-span association used by the
/// reference implementation while keeping the renderer's bounded flat-run
/// representation. If legacy input omits a base role, retain the old
/// single-predecessor fallback instead of attaching unrelated prose.
fn attach_ruby_to_trailing_bases(
    runs: &mut [StyledRun],
    text: String,
    tag: &str,
    outer_opacity: f32,
) {
    if text.is_empty() {
        return;
    }
    let base_count = runs
        .iter()
        .rev()
        .take_while(|run| run.ruby_base)
        .count()
        .max(1);
    let Some(last_base) = runs.last_mut() else {
        return;
    };
    last_base.ruby_text = Some(text);
    last_base.ruby_style = Some(ruby_style_from_tag(tag, last_base, outer_opacity));
    last_base.ruby_group_base_count = base_count;
}

fn resolve_arib_ruby_targets(runs: &mut Vec<StyledRun>) {
    let annotations: Vec<(String, String, RubyStyle)> = runs
        .iter()
        .filter_map(|run| {
            run.ruby_target_id
                .as_ref()
                .filter(|target| !target.is_empty())
                .map(|target| {
                    (
                        target.clone(),
                        run.text.clone(),
                        run.ruby_style.unwrap_or_else(|| default_ruby_style(run)),
                    )
                })
        })
        .collect();
    for (target, ruby_text, annotation_style) in annotations {
        if ruby_text.is_empty() {
            continue;
        }
        if let Some(base) = runs
            .iter_mut()
            .find(|run| run.ruby_target_id.is_none() && run.id.as_deref() == Some(target.as_str()))
        {
            base.ruby_text = Some(ruby_text);
            base.ruby_style = Some(annotation_style);
            base.ruby_group_base_count = 1;
        }
    }
    runs.retain(|run| run.ruby_target_id.is_none());
}

fn ruby_style_from_tag(tag: &str, base: &StyledRun, outer_opacity: f32) -> RubyStyle {
    let local_opacity = parse_opacity(xml_attribute(tag, "tts:opacity"));
    RubyStyle {
        color: apply_opacity(
            xml_attribute(tag, "tts:color")
                .map(parse_rgba)
                .unwrap_or(base.color),
            if xml_attribute(tag, "tts:color").is_some() {
                outer_opacity * local_opacity
            } else {
                local_opacity
            },
        ),
        font_size: parse_font_height(xml_attribute(tag, "tts:fontSize"))
            .unwrap_or_else(|| default_ruby_style(base).font_size)
            .clamp(6.0, 80.0),
        letter_spacing: parse_px(xml_attribute(tag, "tts:letterSpacing"))
            .unwrap_or_else(|| default_ruby_style(base).letter_spacing)
            .clamp(-10.0, 40.0),
        outline: parse_text_outline(
            xml_attribute(tag, "tts:textOutline"),
            outer_opacity * local_opacity,
        )
        .or(base.outline),
    }
}

fn xml_attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let offset = tag.find(name)? + name.len();
    let value = tag.get(offset..)?.trim_start();
    let value = value.strip_prefix('=')?.trim_start();
    let quote = value.chars().next()?;
    (quote == '\'' || quote == '"').then_some(())?;
    let end = value[1..].find(quote)? + 1;
    value.get(1..end)
}

fn strip_tags(value: &str) -> String {
    let mut output = String::new();
    let mut inside = false;
    for character in value.chars() {
        match character {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => output.push(character),
            _ => {}
        }
    }
    output
}
fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
