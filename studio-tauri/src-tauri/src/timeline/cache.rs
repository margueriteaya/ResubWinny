use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, BufRead, BufReader, Seek, SeekFrom},
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

use super::{
    TimelineEvent, TimelineWindow, parse_timeline_event, redact_timeline_payloads,
    timeline_event_id, timeline_record_kind,
};

// Retain enough metadata for normal forward/backward editing without allowing
// an all-day scene archive to grow the UI cache without bound. Older ranges
// are recovered through the bounded binary-seek path.
const MAX_CACHED_TIMELINE_EVENTS: usize = 8_192;
const MAX_SEQUENTIAL_TIMELINE_GAP_MS: i64 = 5 * 60 * 1_000;
const MIN_BINARY_SEEK_BYTES: u64 = 512 * 1_024;
const PROBE_EVENT_LIMIT: usize = 256;
const RECENT_TIMELINE_TAIL_BYTES: u64 = 32 * 1_024 * 1_024;

#[derive(Default)]
struct TimelineWindowCache {
    source: String,
    size: u64,
    modified: Option<SystemTime>,
    offset: u64,
    parsed_events: usize,
    begin_ms: i64,
    end_ms: i64,
    covered_end_ms: i64,
    indexed_begin_ms: i64,
    has_intervals: bool,
    overflowed: bool,
    items: VecDeque<TimelineEvent>,
}

#[derive(Default)]
struct RecentTimelineCache {
    source: String,
    size: u64,
    modified: Option<SystemTime>,
    offset: u64,
    has_intervals: bool,
    overflowed: bool,
    items: VecDeque<TimelineEvent>,
}

static TIMELINE_WINDOW_CACHE: OnceLock<Mutex<TimelineWindowCache>> = OnceLock::new();
static RECENT_TIMELINE_CACHE: OnceLock<Mutex<RecentTimelineCache>> = OnceLock::new();

pub(super) fn collect_cached_time_window(
    archive: &str,
    start: i64,
    finish: i64,
    requested: usize,
) -> Result<TimelineWindow, String> {
    let metadata = fs::metadata(archive)
        .map_err(|error| format!("Could not inspect caption archive: {error}"))?;
    let size = metadata.len();
    let modified = metadata.modified().ok();
    let span = finish.saturating_sub(start).max(5_000);
    let prefetch = span / 2;
    let cache_begin = start.saturating_sub(prefetch).max(0);
    let cache_end = finish.saturating_add(prefetch);
    let cache_lock = TIMELINE_WINDOW_CACHE.get_or_init(|| Mutex::new(Default::default()));
    let mut cache = cache_lock
        .lock()
        .map_err(|_| "Caption timeline cache is unavailable.".to_owned())?;
    let source_changed = cache.source != archive;
    let file_replaced = !source_changed
        && (size < cache.offset || (size == cache.size && modified != cache.modified));
    let needs_earlier_index = !source_changed && cache_begin < cache.indexed_begin_ms;
    let skips_large_forward_gap = !source_changed
        && cache.covered_end_ms >= 0
        && cache.covered_end_ms != i64::MAX
        && cache_begin
            > cache
                .covered_end_ms
                .saturating_add(MAX_SEQUENTIAL_TIMELINE_GAP_MS);
    if source_changed || file_replaced || needs_earlier_index || skips_large_forward_gap {
        reset_time_cache(&mut cache, archive, cache_begin, cache_end);
        let seek_target = cache_begin.saturating_sub(span.max(120_000));
        cache.offset = find_timeline_time_offset(archive, size, seek_target)?;
        cache.indexed_begin_ms = seek_target;
    } else {
        // Keep the slim, already parsed timeline index across both forward and
        // backward navigation. The JSONL records can contain large caption
        // images; reparsing them from byte zero on every reverse pan is far
        // more expensive than retaining the extracted timeline metadata.
        cache.begin_ms = cache_begin;
        cache.end_ms = cache_end;
    }

    extend_time_cache(&mut cache, size, modified)?;
    let mut items = Vec::with_capacity(requested.saturating_add(1));
    for event in &cache.items {
        if (cache.has_intervals && event.kind == "scene")
            || event.end_ms <= start
            || event.begin_ms >= finish
        {
            continue;
        }
        items.push(event.clone());
        if items.len() > requested {
            break;
        }
    }
    let has_more = items.len() > requested || cache.overflowed;
    items.truncate(requested);
    super::materialize_open_scene_bounds(&mut items, Some(finish));
    Ok(TimelineWindow { items, has_more })
}

pub(super) fn collect_recent_timeline_window(
    archive: &str,
    requested: usize,
    features: &[String],
) -> Result<TimelineWindow, String> {
    let metadata = fs::metadata(archive)
        .map_err(|error| format!("Could not inspect caption archive: {error}"))?;
    let size = metadata.len();
    let modified = metadata.modified().ok();
    let cache_lock = RECENT_TIMELINE_CACHE.get_or_init(|| Mutex::new(Default::default()));
    let mut cache = cache_lock
        .lock()
        .map_err(|_| "Recent caption timeline cache is unavailable.".to_owned())?;
    let source_changed = cache.source != archive;
    let file_replaced = !source_changed
        && (size < cache.offset || (size == cache.size && modified != cache.modified));
    if source_changed || file_replaced {
        let offset = recent_timeline_start_offset(archive, size)?;
        *cache = RecentTimelineCache {
            source: archive.to_owned(),
            offset,
            // Starting from the tail intentionally omits older records.
            overflowed: offset > 0,
            ..Default::default()
        };
    }
    extend_recent_cache(&mut cache, size, modified)?;
    let mut items = cache
        .items
        .iter()
        .rev()
        .filter(|event| {
            features.is_empty()
                || features
                    .iter()
                    .any(|feature| event.features.contains(feature))
        })
        .take(requested.saturating_add(1))
        .cloned()
        .collect::<Vec<_>>();
    let has_more = cache.overflowed || items.len() > requested;
    items.truncate(requested);
    items.reverse();
    super::materialize_open_scene_bounds(&mut items, None);
    Ok(TimelineWindow { items, has_more })
}

#[derive(Clone, Copy)]
struct TimelineProbe {
    line_start: u64,
    next_offset: u64,
    begin_ms: i64,
}

pub(super) fn find_timeline_time_offset(
    archive: &str,
    size: u64,
    target_ms: i64,
) -> Result<u64, String> {
    if target_ms <= 0 || size < MIN_BINARY_SEEK_BYTES {
        return Ok(0);
    }
    let mut file =
        File::open(archive).map_err(|error| format!("Could not open caption archive: {error}"))?;
    let mut low = 0_u64;
    let mut high = size;
    for _ in 0..32 {
        if high.saturating_sub(low) <= MIN_BINARY_SEEK_BYTES {
            break;
        }
        let middle = low + (high - low) / 2;
        let probe = probe_timeline_event(&mut file, middle)
            .map_err(|error| format!("Could not seek caption timeline: {error}"))?;
        match probe {
            Some(probe) if probe.begin_ms < target_ms => {
                if probe.next_offset <= low {
                    break;
                }
                low = probe.next_offset.min(size);
            }
            Some(probe) => {
                if probe.line_start >= high {
                    break;
                }
                high = probe.line_start;
            }
            None => high = middle,
        }
    }
    Ok(low.min(size))
}

fn recent_timeline_start_offset(archive: &str, size: u64) -> Result<u64, String> {
    let desired = size.saturating_sub(RECENT_TIMELINE_TAIL_BYTES);
    if desired == 0 {
        return Ok(0);
    }
    let mut file =
        File::open(archive).map_err(|error| format!("Could not open caption archive: {error}"))?;
    file.seek(SeekFrom::Start(desired))
        .map_err(|error| format!("Could not seek caption archive: {error}"))?;
    let mut reader = BufReader::new(&mut file);
    discard_partial_line(&mut reader)
        .map_err(|error| format!("Could not align caption timeline: {error}"))?;
    reader
        .stream_position()
        .map_err(|error| format!("Could not inspect caption timeline position: {error}"))
}

fn probe_timeline_event(file: &mut File, offset: u64) -> io::Result<Option<TimelineProbe>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::new(file);
    if offset > 0 {
        discard_partial_line(&mut reader)?;
    }
    for _ in 0..PROBE_EVENT_LIMIT {
        let line_start = reader.stream_position()?;
        // Scene records put `pts_ms` after the character array and rendered
        // image.  On real B24 archives that field can be hundreds of
        // kilobytes into a line, so a fixed prefix probe incorrectly reports
        // "no timestamp" and sends the binary search back to byte zero.  Scan
        // the line in bounded BufReader chunks instead: this keeps the probe
        // allocation small while still finding timing keys at any position.
        let Some(begin_ms) = read_record_begin_ms(&mut reader)? else {
            // Archives may interleave metadata/resource records without a
            // timeline timestamp. Probe forward to the next timed record
            // instead of incorrectly shrinking the binary-search range.
            continue;
        };
        let next_offset = reader.stream_position()?;
        return Ok(Some(TimelineProbe {
            line_start,
            next_offset,
            begin_ms,
        }));
    }
    Ok(None)
}

/// Find a timeline timestamp while consuming exactly one JSONL record.  The
/// timestamp is not guaranteed to occur near the envelope header: a scene may
/// start with a very large `characters` array and only then emit `pts_ms`.
/// Keeping a short overlap between chunks handles a key/value split at a
/// BufReader boundary without retaining the image payload.
fn read_record_begin_ms(reader: &mut impl BufRead) -> io::Result<Option<i64>> {
    const OVERLAP_BYTES: usize = 96;
    let mut overlap = Vec::with_capacity(OVERLAP_BYTES);
    let mut result = None;
    loop {
        let (consumed, complete) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                return Ok(result);
            }
            let newline = available.iter().position(|byte| *byte == b'\n');
            let consumed = newline.map_or(available.len(), |index| index + 1);
            let content_len = newline.unwrap_or(available.len());
            if result.is_none() {
                let mut combined = Vec::with_capacity(overlap.len() + content_len);
                combined.extend_from_slice(&overlap);
                combined.extend_from_slice(&available[..content_len]);
                result = extract_timeline_begin_bytes(&combined);
                let keep_from = combined.len().saturating_sub(OVERLAP_BYTES);
                overlap.clear();
                overlap.extend_from_slice(&combined[keep_from..]);
            }
            (consumed, newline.is_some())
        };
        reader.consume(consumed);
        if complete {
            return Ok(result);
        }
    }
}

fn discard_partial_line(reader: &mut impl BufRead) -> io::Result<()> {
    loop {
        let (consumed, complete) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                return Ok(());
            }
            match available.iter().position(|byte| *byte == b'\n') {
                Some(index) => (index + 1, true),
                None => (available.len(), false),
            }
        };
        reader.consume(consumed);
        if complete {
            return Ok(());
        }
    }
}

fn extract_timeline_begin_bytes(bytes: &[u8]) -> Option<i64> {
    for key in [
        b"\"begin_ms\"".as_slice(),
        b"\"beginMs\"".as_slice(),
        b"\"start_ms\"".as_slice(),
        b"\"startMs\"".as_slice(),
        b"\"pts_ms\"".as_slice(),
    ] {
        let Some(position) = bytes.windows(key.len()).position(|window| window == key) else {
            continue;
        };
        let mut cursor = position + key.len();
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b':') {
            continue;
        }
        cursor += 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        let number_start = cursor;
        if bytes.get(cursor) == Some(&b'-') {
            cursor += 1;
        }
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor > number_start && bytes.get(number_start..cursor).is_some() {
            let value = std::str::from_utf8(&bytes[number_start..cursor])
                .ok()?
                .parse()
                .ok()?;
            return Some(value);
        }
    }
    None
}

fn reset_time_cache(cache: &mut TimelineWindowCache, archive: &str, begin_ms: i64, end_ms: i64) {
    *cache = TimelineWindowCache {
        source: archive.to_owned(),
        begin_ms,
        end_ms,
        covered_end_ms: -1,
        ..Default::default()
    };
}

fn extend_time_cache(
    cache: &mut TimelineWindowCache,
    size: u64,
    modified: Option<SystemTime>,
) -> Result<(), String> {
    let file_changed = size != cache.size || modified != cache.modified;
    if !file_changed && cache.covered_end_ms >= cache.end_ms {
        return Ok(());
    }
    let file = File::open(&cache.source)
        .map_err(|error| format!("Could not open caption archive: {error}"))?;
    let mut reader = BufReader::new(file);
    reader
        .seek(SeekFrom::Start(cache.offset))
        .map_err(|error| format!("Could not seek caption archive: {error}"))?;
    let mut line = String::new();
    loop {
        let line_start = reader
            .stream_position()
            .map_err(|error| format!("Could not inspect caption archive position: {error}"))?;
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Could not read caption archive: {error}"))?;
        if bytes == 0 {
            cache.offset = line_start;
            cache.covered_end_ms = i64::MAX;
            break;
        }
        if !line.ends_with('\n') {
            cache.offset = line_start;
            break;
        }
        let kind = timeline_record_kind(&line);
        if cache.has_intervals && kind == Some("scene") {
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        }
        if kind.is_some_and(|kind| !matches!(kind, "scene" | "caption" | "region_interval")) {
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        }
        redact_timeline_payloads(&mut line);
        if let Some(event) = parse_timeline_event(&line, timeline_event_id(line_start)) {
            // Worker archives are emitted in presentation-time order. Keep the
            // first later record unread so extending the window resumes here.
            if event.begin_ms > cache.end_ms {
                cache.offset = line_start;
                cache.covered_end_ms = cache.end_ms;
                break;
            }
            cache.parsed_events = cache.parsed_events.saturating_add(1);
            if event.kind != "scene" && !cache.has_intervals {
                cache.has_intervals = true;
                cache.items.retain(|item| item.kind != "scene");
            }
            if !cache.has_intervals || event.kind != "scene" {
                if event.kind == "scene"
                    && let Some(previous) = cache.items.back_mut()
                    && previous.kind == "scene"
                    && event.begin_ms > previous.begin_ms
                {
                    previous.end_ms = previous.end_ms.min(event.begin_ms);
                }
                if cache.items.len() == MAX_CACHED_TIMELINE_EVENTS {
                    cache.items.pop_front();
                    cache.overflowed = true;
                    cache.indexed_begin_ms = cache
                        .items
                        .front()
                        .map(|item| item.begin_ms)
                        .unwrap_or(event.begin_ms);
                }
                cache.items.push_back(event);
            }
        }
        cache.offset = reader.stream_position().unwrap_or(cache.offset);
    }
    cache.size = size;
    cache.modified = modified;
    Ok(())
}

fn extend_recent_cache(
    cache: &mut RecentTimelineCache,
    size: u64,
    modified: Option<SystemTime>,
) -> Result<(), String> {
    if size == cache.size && modified == cache.modified {
        return Ok(());
    }
    let file = File::open(&cache.source)
        .map_err(|error| format!("Could not open caption archive: {error}"))?;
    let mut reader = BufReader::new(file);
    reader
        .seek(SeekFrom::Start(cache.offset))
        .map_err(|error| format!("Could not seek caption archive: {error}"))?;
    let mut line = String::new();
    loop {
        let line_start = reader
            .stream_position()
            .map_err(|error| format!("Could not inspect caption archive position: {error}"))?;
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Could not read caption archive: {error}"))?;
        if bytes == 0 {
            break;
        }
        if !line.ends_with('\n') {
            cache.offset = line_start;
            break;
        }
        let kind = timeline_record_kind(&line);
        if cache.has_intervals && kind == Some("scene") {
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        }
        if kind.is_some_and(|kind| !matches!(kind, "scene" | "caption" | "region_interval")) {
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        }
        redact_timeline_payloads(&mut line);
        if let Some(event) = parse_timeline_event(&line, timeline_event_id(line_start)) {
            if event.kind != "scene" && !cache.has_intervals {
                cache.has_intervals = true;
                cache.items.retain(|item| item.kind != "scene");
            }
            if !cache.has_intervals || event.kind != "scene" {
                if event.kind == "scene"
                    && let Some(previous) = cache.items.back_mut()
                    && previous.kind == "scene"
                    && event.begin_ms > previous.begin_ms
                {
                    previous.end_ms = previous.end_ms.min(event.begin_ms);
                }
                if cache.items.len() == MAX_CACHED_TIMELINE_EVENTS {
                    cache.items.pop_front();
                    cache.overflowed = true;
                }
                cache.items.push_back(event);
            }
        }
        cache.offset = reader.stream_position().unwrap_or(cache.offset);
    }
    cache.size = size;
    cache.modified = modified;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn bounded_window_stops_at_prefetch_end_and_resumes_from_its_cursor() {
        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-cursor-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = File::create(&path).unwrap();
        for index in 0..10_000 {
            writeln!(
                file,
                "{{\"type\":\"caption\",\"value\":{{\"start_ms\":{},\"end_ms\":{},\"text\":\"event-{index}\"}}}}",
                index * 1_000,
                index * 1_000 + 500
            )
            .unwrap();
        }
        drop(file);

        let metadata = fs::metadata(&path).unwrap();
        let mut cache = TimelineWindowCache::default();
        reset_time_cache(&mut cache, &path.to_string_lossy(), 0, 7_500);
        extend_time_cache(&mut cache, metadata.len(), metadata.modified().ok()).unwrap();
        let first_offset = cache.offset;
        let first_count = cache.parsed_events;
        assert!(
            first_count < 20,
            "the first bounded window parsed {first_count} events"
        );
        assert!(first_offset < metadata.len() / 10);

        cache.begin_ms = 5_000;
        cache.end_ms = 15_000;
        extend_time_cache(&mut cache, metadata.len(), metadata.modified().ok()).unwrap();
        assert!(cache.offset > first_offset);
        assert!(cache.parsed_events > first_count);
        assert!(cache.parsed_events < 30);
        assert!(cache.offset < metadata.len() / 10);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn backward_window_reuses_parsed_timeline_index() {
        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-rewind-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = File::create(&path).unwrap();
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

        let metadata = fs::metadata(&path).unwrap();
        let mut cache = TimelineWindowCache::default();
        reset_time_cache(&mut cache, &path.to_string_lossy(), 0, 90_000);
        extend_time_cache(&mut cache, metadata.len(), metadata.modified().ok()).unwrap();
        let forward_offset = cache.offset;
        let forward_count = cache.parsed_events;
        assert!(cache.items.iter().any(|event| event.begin_ms == 60_000));

        cache.begin_ms = 10_000;
        cache.end_ms = 20_000;
        extend_time_cache(&mut cache, metadata.len(), metadata.modified().ok()).unwrap();
        assert_eq!(cache.offset, forward_offset);
        assert_eq!(cache.parsed_events, forward_count);
        assert!(cache.items.iter().any(|event| event.begin_ms == 15_000));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn binary_seek_starts_large_timeline_near_the_requested_time() {
        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-binary-seek-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = File::create(&path).unwrap();
        let payload = "A".repeat(8_192);
        for index in 0..600 {
            writeln!(
                file,
                "{{\"type\":\"scene\",\"value\":{{\"pts_ms\":{},\"text\":\"event-{index}\",\"rendered_image\":{{\"rgba_base64\":\"{payload}\"}}}}}}",
                index * 1_000,
            )
            .unwrap();
        }
        drop(file);
        let size = fs::metadata(&path).unwrap().len();
        let offset = find_timeline_time_offset(&path.to_string_lossy(), size, 500_000).unwrap();
        assert!(offset > size / 2, "binary seek stayed near byte zero");
        let window =
            collect_cached_time_window(&path.to_string_lossy(), 500_000, 510_000, 20).unwrap();
        assert!(window.items.iter().any(|event| event.begin_ms >= 500_000));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn binary_seek_finds_scene_time_after_a_large_image_payload() {
        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-late-scene-time-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = File::create(&path).unwrap();
        let payload = "A".repeat(32 * 1_024);
        for index in 0..256 {
            // B24 worker scene ordering places pts_ms after rendered_image.
            // Keep that ordering here so the test guards the real archive
            // shape rather than the easier timestamp-first fixture above.
            writeln!(
                file,
                "{{\"type\":\"scene\",\"value\":{{\"rendered_image\":{{\"rgba_base64\":\"{payload}\"}},\"pts_ms\":{},\"wait_duration_ms\":500}}}}",
                index * 1_000,
            )
            .unwrap();
        }
        drop(file);

        let size = fs::metadata(&path).unwrap().len();
        let offset = find_timeline_time_offset(&path.to_string_lossy(), size, 200_000).unwrap();
        assert!(
            offset > size / 2,
            "late scene timestamp forced seek to byte zero"
        );
        let window =
            collect_cached_time_window(&path.to_string_lossy(), 200_000, 205_000, 20).unwrap();
        assert!(
            window.items.iter().any(|event| event.begin_ms >= 200_000),
            "time window missed scenes whose timestamp follows the payload"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn timeline_probe_skips_untimed_records() {
        let path = std::env::temp_dir().join(format!(
            "resubwinny-timeline-untimed-probe-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            concat!(
                r#"{"type":"archive_metadata","value":{"route":"arib_std_b24"}}"#,
                "\n",
                r#"{"type":"caption","value":{"start_ms":42000,"end_ms":43000,"text":"字幕"}}"#,
                "\n",
            ),
        )
        .unwrap();
        let mut file = File::open(&path).unwrap();
        let probe = probe_timeline_event(&mut file, 0)
            .unwrap()
            .expect("timed record after metadata");
        assert_eq!(probe.begin_ms, 42_000);
        fs::remove_file(path).unwrap();
    }

    #[test]
    #[ignore = "requires RESUBWINNY_TIMELINE_PERF_ARCHIVE"]
    fn real_archive_time_window_meets_the_interactive_performance_gate() {
        let archive = std::env::var("RESUBWINNY_TIMELINE_PERF_ARCHIVE")
            .expect("set RESUBWINNY_TIMELINE_PERF_ARCHIVE");
        let target_ms = std::env::var("RESUBWINNY_TIMELINE_PERF_TIME_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(10_000_000_i64);
        let started = std::time::Instant::now();
        let window =
            collect_cached_time_window(&archive, target_ms, target_ms.saturating_add(30_000), 500)
                .expect("large timeline window");
        let elapsed = started.elapsed();
        eprintln!(
            "large timeline: {} events in {:.2} ms",
            window.items.len(),
            elapsed.as_secs_f64() * 1_000.0
        );
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "large timeline window took {elapsed:?}"
        );
        let recent_started = std::time::Instant::now();
        let recent = collect_recent_timeline_window(&archive, 200, &[])
            .expect("recent large timeline window");
        let recent_elapsed = recent_started.elapsed();
        eprintln!(
            "recent timeline: {} events in {:.2} ms",
            recent.items.len(),
            recent_elapsed.as_secs_f64() * 1_000.0
        );
        assert!(!recent.items.is_empty(), "recent timeline window is empty");
        assert!(
            recent_elapsed < std::time::Duration::from_secs(5),
            "recent timeline window took {recent_elapsed:?}"
        );
    }
}
