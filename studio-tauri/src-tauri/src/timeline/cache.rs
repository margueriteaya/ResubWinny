use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{BufRead, BufReader, Seek, SeekFrom},
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

use super::{TimelineEvent, TimelineWindow, parse_timeline_event};

const MAX_CACHED_TIMELINE_EVENTS: usize = 2_048;

#[derive(Default)]
struct TimelineWindowCache {
    source: String,
    size: u64,
    modified: Option<SystemTime>,
    offset: u64,
    next_index: usize,
    begin_ms: i64,
    end_ms: i64,
    has_intervals: bool,
    overflowed: bool,
    items: VecDeque<TimelineEvent>,
    recent_overflowed: bool,
    recent_items: VecDeque<TimelineEvent>,
}

static TIMELINE_WINDOW_CACHE: OnceLock<Mutex<TimelineWindowCache>> = OnceLock::new();

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
    let range_missed =
        source_changed || file_replaced || start < cache.begin_ms || finish > cache.end_ms;
    if range_missed {
        *cache = TimelineWindowCache {
            source: archive.to_owned(),
            begin_ms: cache_begin,
            end_ms: cache_end,
            ..Default::default()
        };
    }
    append_timeline_cache(&mut cache, size, modified)?;
    let mut items = Vec::with_capacity(requested.saturating_add(1));
    for event in &cache.items {
        if (cache.has_intervals && event.kind == "scene")
            || event.end_ms < start
            || event.begin_ms > finish
        {
            continue;
        }
        items.push(event.clone());
        if items.len() > requested {
            break;
        }
    }
    let has_more = cache.overflowed || items.len() > requested;
    items.truncate(requested);
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
    let cache_lock = TIMELINE_WINDOW_CACHE.get_or_init(|| Mutex::new(Default::default()));
    let mut cache = cache_lock
        .lock()
        .map_err(|_| "Caption timeline cache is unavailable.".to_owned())?;
    let source_changed = cache.source != archive;
    let file_replaced = !source_changed
        && (size < cache.offset || (size == cache.size && modified != cache.modified));
    if source_changed || file_replaced {
        *cache = TimelineWindowCache {
            source: archive.to_owned(),
            ..Default::default()
        };
    }
    append_timeline_cache(&mut cache, size, modified)?;
    let mut items = cache
        .recent_items
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
    let has_more = cache.recent_overflowed || items.len() > requested;
    items.truncate(requested);
    items.reverse();
    Ok(TimelineWindow { items, has_more })
}

fn append_timeline_cache(
    cache: &mut TimelineWindowCache,
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
        if let Some(event) = parse_timeline_event(&line, cache.next_index) {
            cache.next_index = cache.next_index.saturating_add(1);
            if event.kind != "scene" && !cache.has_intervals {
                cache.has_intervals = true;
                cache.items.retain(|item| item.kind != "scene");
                cache.recent_items.retain(|item| item.kind != "scene");
            }
            if !cache.has_intervals || event.kind != "scene" {
                if cache.recent_items.len() == MAX_CACHED_TIMELINE_EVENTS {
                    cache.recent_items.pop_front();
                    cache.recent_overflowed = true;
                }
                cache.recent_items.push_back(event.clone());
                if event.end_ms >= cache.begin_ms && event.begin_ms <= cache.end_ms {
                    if cache.items.len() == MAX_CACHED_TIMELINE_EVENTS {
                        cache.items.pop_front();
                        cache.overflowed = true;
                    }
                    cache.items.push_back(event);
                }
            }
        }
        cache.offset = reader.stream_position().unwrap_or(cache.offset);
    }
    cache.size = size;
    cache.modified = modified;
    Ok(())
}
