use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader, Seek},
};

use serde::Serialize;

use crate::{
    arib_symbols::{is_arib_additional_symbol, is_arib_additional_symbol_codepoint},
    caption_features::{accessibility_ranges, gaiji_ranges},
};

const MAX_WINDOW_SIZE: usize = 512;
const OPEN_SCENE_SENTINEL: i64 = i64::MAX;
const DEFAULT_OPEN_SCENE_SPAN_MS: i64 = 5_000;

mod cache;
use cache::{collect_cached_time_window, collect_recent_timeline_window};

pub(crate) fn approximate_archive_time_offset(
    archive: &str,
    size: u64,
    target_ms: i64,
) -> Result<u64, String> {
    cache::find_timeline_time_offset(archive, size, target_ms)
}

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
    let mut items: Vec<TimelineEvent> = Vec::with_capacity(requested.saturating_add(1));
    scan_timeline_events(archive, |event| {
        if event.kind != "scene" && !has_intervals {
            has_intervals = true;
            skipped = 0;
            has_more = false;
            items.clear();
        }
        if event.kind == "scene"
            && !has_intervals
            && let Some(previous) = items
                .iter_mut()
                .rev()
                .find(|previous| previous.kind == "scene")
            && event.begin_ms > previous.begin_ms
        {
            // B24 scenes are complete caption-plane snapshots. An open scene
            // ends when the next snapshot replaces it, even when the worker
            // encoded its wait duration as open-ended.
            previous.end_ms = previous.end_ms.min(event.begin_ms);
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
    materialize_open_scene_bounds(&mut items, None);
    Ok(TimelineWindow { items, has_more })
}

/// The native preview keeps an open B24 scene alive until a later snapshot
/// replaces it. A timeline still needs a finite bar so its ruler, zoom and
/// scrollbar remain usable when an archive contains only scene snapshots.
/// Materialize that sentinel only in the response copy; the cache keeps the
/// original open-ended value for correct preview composition and later range
/// queries.
pub(super) fn materialize_open_scene_bounds(
    items: &mut [TimelineEvent],
    requested_end_ms: Option<i64>,
) {
    for index in 0..items.len() {
        if items[index].kind != "scene" || items[index].end_ms < OPEN_SCENE_SENTINEL / 2 {
            continue;
        }
        let next_scene = items[index + 1..]
            .iter()
            .find(|item| item.kind == "scene" && item.begin_ms > items[index].begin_ms)
            .map(|item| item.begin_ms);
        let fallback = requested_end_ms
            .filter(|end| *end > items[index].begin_ms)
            .unwrap_or_else(|| {
                items[index]
                    .begin_ms
                    .saturating_add(DEFAULT_OPEN_SCENE_SPAN_MS)
            });
        items[index].end_ms = next_scene
            .unwrap_or(fallback)
            .max(items[index].begin_ms + 1);
    }
}

fn scan_timeline_events(
    archive: &str,
    mut on_event: impl FnMut(TimelineEvent) -> bool,
) -> Result<(), String> {
    let mut has_intervals = false;
    scan_archive_lines(archive, |line, line_start| {
        // Once an authoritative interval stream has appeared, later B24
        // scene snapshots are redundant for the timeline. Avoid touching
        // their often multi-hundred-kilobyte image payloads at all.
        let kind = timeline_record_kind(line);
        if has_intervals && kind == Some("scene") {
            return true;
        }
        if kind.is_some_and(|kind| !matches!(kind, "scene" | "caption" | "region_interval")) {
            return true;
        }
        redact_timeline_payloads(line);
        let Some(event) = parse_timeline_event(line, timeline_event_id(line_start)) else {
            return true;
        };
        if event.kind != "scene" {
            has_intervals = true;
        }
        on_event(event)
    })
}

fn scan_archive_lines(
    archive: &str,
    mut on_line: impl FnMut(&mut String, u64) -> bool,
) -> Result<(), String> {
    let file =
        File::open(archive).map_err(|error| format!("Could not open caption archive: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        let line_start = reader
            .stream_position()
            .map_err(|error| format!("Could not inspect caption archive: {error}"))?;
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Could not read caption archive: {error}"))?;
        if bytes == 0 || !line.ends_with('\n') {
            break;
        }
        if !on_line(&mut line, line_start) {
            break;
        }
    }
    Ok(())
}

/// The JSONL byte position is stable across pagination, time-window and tail
/// queries. The UI uses this value as a keyed-rendering identity; an ordinal
/// fabricated from a binary-seek byte offset changes whenever the cache is
/// rebuilt and causes hundreds of subtitle nodes to be unnecessarily remade.
pub(super) fn timeline_event_id(line_start: u64) -> usize {
    usize::try_from(line_start).unwrap_or(usize::MAX)
}

/// Read the envelope discriminator without deserializing the complete JSONL
/// record. Worker records put `type`/`kind` near the beginning of the line;
/// limiting the scan to a small prefix keeps this probe cheap even when a
/// scene carries a several-megabyte raster payload.
pub(super) fn timeline_record_kind(line: &str) -> Option<&'static str> {
    let bytes = line.as_bytes();
    let prefix_len = bytes.len().min(1_024);
    let prefix = &bytes[..prefix_len];
    for key in [b"\"type\"".as_slice(), b"\"kind\"".as_slice()] {
        let Some(position) = prefix.windows(key.len()).position(|window| window == key) else {
            continue;
        };
        let mut cursor = position + key.len();
        while prefix.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if prefix.get(cursor) != Some(&b':') {
            continue;
        }
        cursor += 1;
        while prefix.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if prefix.get(cursor) != Some(&b'"') {
            continue;
        }
        let start = cursor + 1;
        let end = prefix[start..]
            .iter()
            .position(|byte| *byte == b'"')
            .map(|relative| start + relative)?;
        return match &prefix[start..end] {
            b"scene" => Some("scene"),
            b"caption" => Some("caption"),
            b"region_interval" => Some("region_interval"),
            b"resource_evidence" => Some("resource_evidence"),
            _ => Some("other"),
        };
    }
    None
}

/// Timeline rows need timing, text and style metadata, never multi-megabyte
/// image payloads. Remove those JSON string values before deserializing the
/// envelope so the event index does not allocate/copy caption bitmaps.
pub(super) fn redact_timeline_payloads(line: &mut String) {
    // The vast majority of interval records contain no image payload. Avoid
    // six full-string scans (and temporary key allocations) for those lines;
    // only scene snapshots and resource records need redaction.
    if !line.contains("base64") && !line.contains("DataUri") && !line.contains("data_uri") {
        return;
    }
    for field in [
        "rgba_base64",
        "rgbaBase64",
        "png_base64",
        "pngBase64",
        "preview_data_uri",
        "previewDataUri",
    ] {
        redact_json_string_field(line, field);
    }
}

fn redact_json_string_field(line: &mut String, field: &str) {
    let needle = format!("\"{field}\"");
    let mut search_from = 0;
    while let Some(relative) = line[search_from..].find(&needle) {
        let key_end = search_from + relative + needle.len();
        let bytes = line.as_bytes();
        let mut value_start = key_end;
        while bytes.get(value_start).is_some_and(u8::is_ascii_whitespace) {
            value_start += 1;
        }
        if bytes.get(value_start) != Some(&b':') {
            search_from = key_end;
            continue;
        }
        value_start += 1;
        while bytes.get(value_start).is_some_and(u8::is_ascii_whitespace) {
            value_start += 1;
        }
        if bytes.get(value_start) != Some(&b'"') {
            search_from = value_start;
            continue;
        }
        let mut cursor = value_start + 1;
        let mut escaped = false;
        let mut value_end = None;
        while let Some(byte) = bytes.get(cursor).copied() {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                value_end = Some(cursor);
                break;
            }
            cursor += 1;
        }
        let Some(value_end) = value_end else {
            break;
        };
        line.replace_range(value_start..=value_end, "null");
        search_from = value_start + 4;
    }
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
    let explicit_end = value
        .get("end_ms")
        .or_else(|| value.get("endMs"))
        .or_else(|| value.get("finish_ms"))
        .or_else(|| value.get("finishMs"))
        .and_then(serde_json::Value::as_i64);
    let end = explicit_end.unwrap_or_else(|| {
        if kind == "scene" {
            let wait = value
                .get("wait_duration_ms")
                .or_else(|| value.get("waitDurationMs"))
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(5_000);
            if wait > 0 && wait < i64::MAX / 2 {
                begin.saturating_add(wait)
            } else {
                i64::MAX
            }
        } else {
            // Match the native preview fallback for an otherwise valid open
            // caption record instead of turning it into a zero-width event.
            begin.saturating_add(5_000)
        }
    });
    (end > begin).then_some((begin, end))
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
        // Timeline labels are intentionally capped. B24 scene snapshots may
        // contain thousands of character objects (plus a large rendered
        // image); stop walking once enough display text has been collected so
        // indexing never pays for the full raster-sized scene payload.
        if text.chars().count() >= 240 {
            break;
        }
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
        assert_eq!(page.items[0].text, "second");
        assert!(!page.has_more);
        let first = get_timeline_window(path.to_string_lossy().into_owned(), 0, 1).unwrap();
        assert_eq!(first.items[0].text, "first");
        assert_eq!(first.items[0].region_x, Some(12));
        assert!(page.items[0].index > first.items[0].index);
        let timed =
            get_timeline_time_window(path.to_string_lossy().into_owned(), 250, 450, 10).unwrap();
        assert_eq!(timed.items.len(), 1);
        assert_eq!(timed.items[0].text, "second");
        assert_eq!(timed.items[0].index, page.items[0].index);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn scene_timing_matches_preview_and_the_next_scene_closes_an_open_snapshot() {
        let open = serde_json::json!({
            "pts_ms": 2_000,
            "wait_duration_ms": i64::MAX,
        });
        let bounded = serde_json::json!({
            "pts_ms": 4_000,
            "wait_duration_ms": 1_500,
        });
        assert_eq!(bounds(&open, "scene"), Some((2_000, i64::MAX)));
        assert_eq!(bounds(&bounded, "scene"), Some((4_000, 5_500)));
        assert_eq!(
            bounds(&serde_json::json!({"start_ms": 7_000}), "caption"),
            Some((7_000, 12_000))
        );

        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-scene-duration-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            concat!(
                "{\"type\":\"scene\",\"value\":{\"pts_ms\":2000,\"wait_duration_ms\":9223372036854775807,\"text\":\"old\"}}\n",
                "{\"type\":\"scene\",\"value\":{\"pts_ms\":4000,\"wait_duration_ms\":1500,\"text\":\"new\"}}\n"
            ),
        )
        .unwrap();
        let window =
            get_timeline_time_window(path.to_string_lossy().into_owned(), 0, 6_000, 10).unwrap();
        assert_eq!(window.items.len(), 2);
        assert_eq!(
            (window.items[0].begin_ms, window.items[0].end_ms),
            (2_000, 4_000)
        );
        assert_eq!(
            (window.items[1].begin_ms, window.items[1].end_ms),
            (4_000, 5_500)
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn open_scene_response_always_has_a_finite_timeline_bar() {
        let mut items = vec![TimelineEvent {
            index: 1,
            kind: "scene".into(),
            begin_ms: 2_000,
            end_ms: i64::MAX,
            text: "open".into(),
            region_x: None,
            region_y: None,
            track_id: None,
            features: Vec::new(),
            highlights: Vec::new(),
            colors: Vec::new(),
        }];
        materialize_open_scene_bounds(&mut items, Some(9_000));
        assert_eq!(items[0].end_ms, 9_000);
    }

    #[test]
    fn timeline_parser_discards_large_image_strings_before_deserializing() {
        let payload = "A".repeat(1_000_000);
        let mut line = format!(
            r#"{{"type":"scene","value":{{"pts_ms":1000,"text":"subtitle","rendered_image":{{"rgba_base64":"{payload}"}}}}}}"#
        );
        redact_timeline_payloads(&mut line);
        assert!(line.len() < 200);
        let event = parse_timeline_event(&line, 7).expect("timeline event");
        assert_eq!(event.index, 7);
        assert_eq!(event.text, "subtitle");
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
