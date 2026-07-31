use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader},
};

use serde::Serialize;

use crate::{
    arib_symbols::{is_arib_additional_symbol, is_arib_additional_symbol_codepoint},
    caption_features::{accessibility_ranges, gaiji_ranges},
};

const MAX_WINDOW_SIZE: usize = 200;

mod cache;
use cache::{collect_cached_time_window, collect_recent_timeline_window};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub index: usize,
    pub kind: String,
    pub begin_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub region_x: Option<i64>,
    pub region_y: Option<i64>,
    pub track_id: Option<String>,
    pub features: Vec<String>,
    pub highlights: Vec<TimelineHighlight>,
    pub colors: Vec<TimelineColor>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineHighlight {
    pub start: usize,
    pub end: usize,
    pub feature: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct TimelineColor {
    pub role: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineWindow {
    pub items: Vec<TimelineEvent>,
    pub has_more: bool,
}

#[tauri::command]
pub fn get_timeline_window(
    archive: String,
    offset: usize,
    limit: usize,
) -> Result<TimelineWindow, String> {
    let requested = limit.clamp(1, MAX_WINDOW_SIZE);
    collect_timeline_window(&archive, offset, requested, &[])
}

#[tauri::command]
pub fn get_timeline_window_filtered(
    archive: String,
    offset: usize,
    limit: usize,
    features: Vec<String>,
) -> Result<TimelineWindow, String> {
    let requested = limit.clamp(1, MAX_WINDOW_SIZE);
    collect_timeline_window(&archive, offset, requested, &features)
}

#[tauri::command]
pub fn get_timeline_time_window(
    archive: String,
    begin_ms: i64,
    end_ms: i64,
    limit: usize,
) -> Result<TimelineWindow, String> {
    let requested = limit.clamp(1, MAX_WINDOW_SIZE);
    let start = begin_ms.max(0);
    let finish = end_ms.max(start);
    collect_cached_time_window(&archive, start, finish, requested)
}

#[tauri::command]
pub fn get_timeline_recent_window_filtered(
    archive: String,
    limit: usize,
    features: Vec<String>,
) -> Result<TimelineWindow, String> {
    let requested = limit.clamp(1, MAX_WINDOW_SIZE);
    collect_recent_timeline_window(&archive, requested, &features)
}

fn collect_timeline_window(
    archive: &str,
    offset: usize,
    requested: usize,
    features: &[String],
) -> Result<TimelineWindow, String> {
    let mut has_intervals = false;
    let mut skipped = 0_usize;
    let mut has_more = false;
    let mut items = Vec::with_capacity(requested.saturating_add(1));
    scan_timeline_events(archive, |event| {
        if event.kind != "scene" && !has_intervals {
            has_intervals = true;
            skipped = 0;
            has_more = false;
            items.clear();
        }
        if (has_intervals && event.kind == "scene")
            || (!features.is_empty()
                && !features
                    .iter()
                    .any(|feature| event.features.contains(feature)))
        {
            return true;
        }
        if skipped < offset {
            skipped += 1;
            return true;
        }
        if items.len() <= requested {
            items.push(event);
        }
        if items.len() > requested {
            has_more = true;
            // Before the first interval appears, keep scanning: a live B24
            // archive can contain provisional scene records followed by the
            // authoritative interval stream.
            return !has_intervals;
        }
        true
    })?;
    items.truncate(requested);
    Ok(TimelineWindow { items, has_more })
}

fn scan_timeline_events(
    archive: &str,
    mut on_event: impl FnMut(TimelineEvent) -> bool,
) -> Result<(), String> {
    let mut index = 0_usize;
    scan_archive_lines(archive, |line| {
        let Some(event) = parse_timeline_event(line, index) else {
            return true;
        };
        index = index.saturating_add(1);
        on_event(event)
    })
}

fn scan_archive_lines(archive: &str, mut on_line: impl FnMut(&str) -> bool) -> Result<(), String> {
    let file =
        File::open(archive).map_err(|error| format!("Could not open caption archive: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Could not read caption archive: {error}"))?;
        if bytes == 0 || !line.ends_with('\n') {
            break;
        }
        if !on_line(&line) {
            break;
        }
    }
    Ok(())
}

fn parse_timeline_event(line: &str, index: usize) -> Option<TimelineEvent> {
    let envelope = serde_json::from_str::<serde_json::Value>(line).ok()?;
    let kind = envelope
        .get("kind")
        .or_else(|| envelope.get("type"))
        .and_then(serde_json::Value::as_str)?;
    if !matches!(kind, "region_interval" | "caption" | "scene") {
        return None;
    }
    let value = envelope.get("value").unwrap_or(&envelope);
    let (begin_ms, end_ms) = bounds(value, kind)?;
    let (text, features, highlights, colors) = event_presentation(value);
    Some(TimelineEvent {
        index,
        kind: kind.to_owned(),
        begin_ms,
        end_ms,
        text,
        region_x: value
            .get("region")
            .and_then(|region| region.get("x"))
            .or_else(|| value.get("x"))
            .and_then(serde_json::Value::as_i64),
        region_y: value
            .get("region")
            .and_then(|region| region.get("y"))
            .or_else(|| value.get("y"))
            .and_then(serde_json::Value::as_i64),
        track_id: track_id(value),
        features,
        highlights,
        colors,
    })
}

fn track_id(value: &serde_json::Value) -> Option<String> {
    value
        .get("source_pid")
        .or_else(|| value.get("sourcePid"))
        .or_else(|| value.get("pid"))
        .or_else(|| value.get("source").and_then(|source| source.get("pid")))
        .and_then(serde_json::Value::as_u64)
        .map(|pid| format!("PID 0x{pid:04X}"))
}

fn bounds(value: &serde_json::Value, kind: &str) -> Option<(i64, i64)> {
    let begin = value
        .get("begin_ms")
        .or_else(|| value.get("beginMs"))
        .or_else(|| value.get("start_ms"))
        .or_else(|| value.get("startMs"))
        .or_else(|| (kind == "scene").then(|| value.get("pts_ms")).flatten())
        .and_then(serde_json::Value::as_i64)?;
    let end = value
        .get("end_ms")
        .or_else(|| value.get("endMs"))
        .or_else(|| value.get("finish_ms"))
        .or_else(|| value.get("finishMs"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(begin);
    Some((begin, end.max(begin)))
}

fn event_presentation(
    value: &serde_json::Value,
) -> (
    String,
    Vec<String>,
    Vec<TimelineHighlight>,
    Vec<TimelineColor>,
) {
    let mut features = BTreeSet::new();
    let mut highlights = Vec::new();
    let mut colors = BTreeSet::new();
    if value.get("x").is_some() || value.get("y").is_some() || value.get("region").is_some() {
        features.insert("position".to_owned());
    }
    let rich = value
        .get("rich_body")
        .or_else(|| value.get("richBody"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let ruby = value
        .get("region")
        .and_then(|region| region.get("is_ruby").or_else(|| region.get("isRuby")))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || value
            .get("ruby_bindings")
            .or_else(|| value.get("rubyBindings"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| !items.is_empty())
        || rich.contains("ruby");
    if ruby {
        features.insert("ruby".to_owned());
    }
    if let Some(style) = value.get("style") {
        if let Some(color) = style
            .get("color")
            .and_then(serde_json::Value::as_str)
            .and_then(normalize_ttml_color)
        {
            features.insert("color".to_owned());
            colors.insert(TimelineColor {
                role: "text".into(),
                value: color,
            });
        }
        if let Some(color) = style
            .get("background_color")
            .or_else(|| style.get("backgroundColor"))
            .and_then(serde_json::Value::as_str)
            .and_then(normalize_ttml_color)
        {
            features.insert("color".to_owned());
            colors.insert(TimelineColor {
                role: "background".into(),
                value: color,
            });
        }
    }
    if let Some(text) = value.get("text").and_then(serde_json::Value::as_str) {
        let text = truncate(text);
        add_text_features(&text, &mut features, &mut highlights);
        if ruby && !text.is_empty() {
            highlights.push(TimelineHighlight {
                start: 0,
                end: text.chars().count(),
                feature: "ruby".into(),
            });
        }
        return (
            text,
            features.into_iter().collect(),
            highlights,
            colors.into_iter().collect(),
        );
    }
    let Some(characters) = value
        .get("characters")
        .and_then(serde_json::Value::as_array)
    else {
        return (
            String::new(),
            features.into_iter().collect(),
            highlights,
            colors.into_iter().collect(),
        );
    };
    let mut text = String::new();
    for character in characters {
        let value = character
            .get("text")
            .or_else(|| character.get("character"))
            .or_else(|| character.get("unicode"))
            .or_else(|| character.get("utf8"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let start = text.chars().count();
        text.push_str(value);
        let end = text.chars().count();
        let drcs = character.get("kind").and_then(serde_json::Value::as_u64) == Some(1)
            || character
                .get("drcs_code")
                .or_else(|| character.get("drcsCode"))
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|code| code != 0);
        let displayed_gaiji = value.chars().any(is_arib_additional_symbol);
        let source_gaiji = character
            .get("pua_codepoint")
            .or_else(|| character.get("puaCodepoint"))
            .and_then(serde_json::Value::as_u64)
            .and_then(|codepoint| u32::try_from(codepoint).ok())
            .is_some_and(is_arib_additional_symbol_codepoint);
        // Unicode symbols are classified once after the complete text is
        // assembled. Source PUA evidence is needed only when that display
        // character no longer identifies the additional-symbol row.
        let gaiji = source_gaiji && !displayed_gaiji;
        let text_color = character
            .get("text_color")
            .or_else(|| character.get("textColor"))
            .and_then(serde_json::Value::as_u64);
        let color = text_color.is_some_and(|color| color & 0x00FF_FFFF != 0x00FF_FFFF);
        if color && let Some(color) = text_color {
            features.insert("color".into());
            colors.insert(TimelineColor {
                role: "text".into(),
                value: format!("#{:06X}", color & 0x00FF_FFFF),
            });
        }
        for (enabled, feature) in [(drcs, "drcs"), (gaiji, "gaiji"), (ruby, "ruby")] {
            if enabled {
                features.insert(feature.into());
                if end > start {
                    highlights.push(TimelineHighlight {
                        start,
                        end,
                        feature: feature.into(),
                    });
                }
            }
        }
    }
    let text = truncate(&text);
    let text_len = text.chars().count();
    highlights.retain(|item| item.start < text_len);
    for item in &mut highlights {
        item.end = item.end.min(text_len);
    }
    add_text_features(&text, &mut features, &mut highlights);
    (
        text,
        features.into_iter().collect(),
        highlights,
        colors.into_iter().collect(),
    )
}

fn normalize_ttml_color(value: &str) -> Option<String> {
    let hex = value.trim().strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex[..6].to_ascii_uppercase()))
}

fn add_text_features(
    text: &str,
    features: &mut BTreeSet<String>,
    highlights: &mut Vec<TimelineHighlight>,
) {
    for range in gaiji_ranges(text) {
        features.insert("gaiji".into());
        highlights.push(TimelineHighlight {
            start: range.start,
            end: range.end,
            feature: "gaiji".into(),
        });
    }
    for range in accessibility_ranges(text) {
        features.insert("accessibility".into());
        highlights.push(TimelineHighlight {
            start: range.start,
            end: range.end,
            feature: "accessibility".into(),
        });
    }
}

fn truncate(value: &str) -> String {
    const MAX_CHARS: usize = 240;
    let mut result = value.chars().take(MAX_CHARS).collect::<String>();
    if value.chars().nth(MAX_CHARS).is_some() {
        result.push_str("...");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn returns_only_requested_archive_window() {
        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            concat!(
                "{\"type\":\"arib_caption_studio_archive\"}\n",
                "{\"type\":\"region_interval\",\"value\":{\"begin_ms\":100,\"end_ms\":200,\"characters\":[{\"utf8\":\"first\"}],\"region\":{\"x\":12,\"y\":34}}}\n",
                "{\"type\":\"caption\",\"value\":{\"start_ms\":300,\"end_ms\":400,\"text\":\"second\"}}\n"
            ),
        ).unwrap();
        let page = get_timeline_window(path.to_string_lossy().into_owned(), 1, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].index, 1);
        assert_eq!(page.items[0].text, "second");
        assert!(!page.has_more);
        let first = get_timeline_window(path.to_string_lossy().into_owned(), 0, 1).unwrap();
        assert_eq!(first.items[0].text, "first");
        assert_eq!(first.items[0].region_x, Some(12));
        let timed =
            get_timeline_time_window(path.to_string_lossy().into_owned(), 250, 450, 10).unwrap();
        assert_eq!(timed.items.len(), 1);
        assert_eq!(timed.items[0].text, "second");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn pagination_does_not_commit_provisional_scenes_before_later_intervals() {
        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-preference-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            concat!(
                "{\"type\":\"scene\",\"value\":{\"pts_ms\":0,\"text\":\"scene-0\"}}\n",
                "{\"type\":\"scene\",\"value\":{\"pts_ms\":100,\"text\":\"scene-1\"}}\n",
                "{\"type\":\"scene\",\"value\":{\"pts_ms\":200,\"text\":\"scene-2\"}}\n",
                "{\"type\":\"caption\",\"value\":{\"start_ms\":0,\"end_ms\":500,\"text\":\"interval-0\"}}\n",
                "{\"type\":\"caption\",\"value\":{\"start_ms\":600,\"end_ms\":900,\"text\":\"interval-1\"}}\n"
            ),
        )
        .unwrap();
        let page = get_timeline_window(path.to_string_lossy().into_owned(), 0, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].text, "interval-0");
        assert!(page.has_more);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn streams_large_archive_pages_without_retaining_previous_windows() {
        use std::io::Write;

        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-page-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "{{\"type\":\"arib_caption_studio_archive\"}}").unwrap();
        for index in 0..1_000 {
            writeln!(
                file,
                "{{\"type\":\"caption\",\"value\":{{\"start_ms\":{},\"end_ms\":{},\"text\":\"event-{index}\"}}}}",
                index * 1_000,
                index * 1_000 + 500
            )
            .unwrap();
        }
        drop(file);

        let page = get_timeline_window(path.to_string_lossy().into_owned(), 790, 20).unwrap();
        assert_eq!(page.items.len(), 20);
        assert_eq!(page.items.first().unwrap().text, "event-790");
        assert_eq!(page.items.last().unwrap().text, "event-809");
        assert!(page.has_more);
        let final_page = get_timeline_window(path.to_string_lossy().into_owned(), 990, 20).unwrap();
        assert_eq!(final_page.items.len(), 10);
        assert!(!final_page.has_more);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn live_event_window_returns_only_the_recent_bounded_page() {
        use std::io::Write;

        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-recent-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = fs::File::create(&path).unwrap();
        for index in 0..300 {
            writeln!(
                file,
                "{{\"type\":\"caption\",\"value\":{{\"start_ms\":{},\"end_ms\":{},\"text\":\"event-{index}\"}}}}",
                index * 1_000,
                index * 1_000 + 500
            )
            .unwrap();
        }
        drop(file);

        let recent = get_timeline_recent_window_filtered(
            path.to_string_lossy().into_owned(),
            100,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(recent.items.len(), 100);
        assert_eq!(recent.items.first().unwrap().text, "event-200");
        assert_eq!(recent.items.last().unwrap().text, "event-299");
        assert!(recent.has_more);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn incrementally_indexes_complete_lines_and_prefers_intervals() {
        use std::io::Write;

        let path = std::env::temp_dir().join(format!(
            "resubwinny-live-timeline-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            concat!(
                "{\"type\":\"scene\",\"value\":{\"pts_ms\":100,\"text\":\"preview\"}}\n",
                "{\"type\":\"caption\",\"value\":{\"start_ms\":100,\"end_ms\":500,\"text\":\"stable\"}}\n",
                "{\"type\":\"caption\",\"value\":{\"start_ms\":600"
            ),
        )
        .unwrap();
        let first =
            get_timeline_time_window(path.to_string_lossy().into_owned(), 0, 500, 10).unwrap();
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].text, "stable");

        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, ",\"end_ms\":900,\"text\":\"appended\"}}}}").unwrap();
        let appended =
            get_timeline_time_window(path.to_string_lossy().into_owned(), 550, 950, 10).unwrap();
        assert_eq!(appended.items.len(), 1);
        assert_eq!(appended.items[0].text, "appended");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn classifies_event_features_and_character_highlights_in_the_backend() {
        let value = serde_json::json!({
            "region": { "x": 20, "y": 30, "is_ruby": true },
            "characters": [
                { "utf8": "♪", "kind": 0, "text_color": 0xFF00FFFF_u64 },
                { "utf8": "➡", "kind": 1, "drcs_code": 42, "pua_codepoint": 0xE28F }
            ]
        });
        let (text, features, highlights, colors) = event_presentation(&value);
        assert_eq!(text, "♪➡");
        for expected in [
            "position",
            "color",
            "ruby",
            "drcs",
            "gaiji",
            "accessibility",
        ] {
            assert!(
                features.iter().any(|feature| feature == expected),
                "missing {expected}"
            );
        }
        assert!(
            highlights
                .iter()
                .any(|item| item.feature == "accessibility" && item.start == 0)
        );
        assert!(
            highlights
                .iter()
                .any(|item| item.feature == "drcs" && item.start == 1)
        );
        assert!(!highlights.iter().any(|item| item.feature == "color"));
        assert_eq!(
            colors,
            vec![TimelineColor {
                role: "text".into(),
                value: "#00FFFF".into(),
            }]
        );
    }

    #[test]
    fn angle_brackets_mark_only_delimiters_and_retain_narration() {
        let value = serde_json::json!({ "text": "<ついに！>語りは残す" });
        let (text, features, highlights, _) = event_presentation(&value);
        assert_eq!(text, "<ついに！>語りは残す");
        assert!(features.iter().any(|feature| feature == "accessibility"));
        let ranges = highlights
            .iter()
            .filter(|item| item.feature == "accessibility")
            .map(|item| (item.start, item.end))
            .collect::<Vec<_>>();
        assert_eq!(ranges, vec![(0, 1), (5, 6)]);
    }

    #[test]
    fn parentheses_mark_the_complete_accessibility_annotation() {
        let value = serde_json::json!({ "text": "（シンジ）本文" });
        let (_, _, highlights, _) = event_presentation(&value);
        let ranges = highlights
            .iter()
            .filter(|item| item.feature == "accessibility")
            .map(|item| (item.start, item.end))
            .collect::<Vec<_>>();
        assert_eq!(ranges, vec![(0, 5)]);
    }

    #[test]
    fn music_marker_includes_immediately_following_wave_marks() {
        let value = serde_json::json!({ "text": "♪～〜本文" });
        let (_, _, highlights, _) = event_presentation(&value);
        let ranges = highlights
            .iter()
            .filter(|item| item.feature == "accessibility")
            .map(|item| (item.start, item.end))
            .collect::<Vec<_>>();
        assert_eq!(ranges, vec![(0, 3)]);
    }

    #[test]
    fn standalone_wave_is_not_accessibility_and_split_narration_delimiters_are() {
        let (_, features, highlights, _) =
            event_presentation(&serde_json::json!({ "text": "語調〜" }));
        assert!(!features.iter().any(|feature| feature == "accessibility"));
        assert!(
            !highlights
                .iter()
                .any(|item| item.feature == "accessibility")
        );

        for (text, expected) in [("<語り", (0, 1)), ("続き>", (2, 3))] {
            let (_, _, highlights, _) = event_presentation(&serde_json::json!({ "text": text }));
            assert!(highlights.iter().any(|item| {
                item.feature == "accessibility" && (item.start, item.end) == expected
            }));
        }
    }

    #[test]
    fn event_highlights_exactly_match_the_shared_export_deletion_mask() {
        let text = "♪〜語調〜<語り>⚟➡本文";
        let (_, _, highlights, _) = event_presentation(&serde_json::json!({ "text": text }));
        let highlighted = (0..text.chars().count())
            .map(|index| {
                highlights.iter().any(|item| {
                    matches!(item.feature.as_str(), "gaiji" | "accessibility")
                        && index >= item.start
                        && index < item.end
                })
            })
            .collect::<Vec<_>>();
        let deleted = crate::caption_features::retained_characters(text, false, false)
            .into_iter()
            .map(|retained| !retained)
            .collect::<Vec<_>>();
        assert_eq!(highlighted, deleted);
    }

    #[test]
    fn classifies_arib_additional_symbols_without_additional_kanji() {
        let value = serde_json::json!({
            "characters": [
                { "utf8": "➡", "codepoint": 0x27A1_u64, "pua_codepoint": 0 },
                { "utf8": "㊙", "codepoint": 0x3299_u64, "pua_codepoint": 0 },
                { "utf8": "㐂", "codepoint": 0x3402_u64, "pua_codepoint": 0 },
                { "utf8": "→", "codepoint": 0x2192_u64, "pua_codepoint": 0 },
                { "utf8": "年月日円", "pua_codepoint": 0 }
            ]
        });
        let (text, features, highlights, _) = event_presentation(&value);
        assert_eq!(text, "➡㊙㐂→年月日円");
        assert!(features.iter().any(|feature| feature == "gaiji"));
        let ranges = highlights
            .iter()
            .filter(|item| item.feature == "gaiji")
            .map(|item| (item.start, item.end))
            .collect::<Vec<_>>();
        assert_eq!(ranges, vec![(0, 1), (1, 2)]);
    }
}
