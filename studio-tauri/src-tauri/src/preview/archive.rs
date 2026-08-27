use super::*;

const PREVIEW_CHECKPOINT_INTERVAL_MS: i64 = 30_000;
// Four hours of 30-second B24 checkpoints can otherwise retain hundreds of
// large scene payloads. Older positions now use the bounded binary seek path.
const MAX_PREVIEW_CHECKPOINTS: usize = 128;
const PREVIEW_RANDOM_SEEK_THRESHOLD_MS: i64 = 2 * 60 * 1_000;
// A ten-minute rewind forced a random seek to parse hundreds of megabytes of
// B24 scene images before it could display one frame. Two minutes still
// covers long caption intervals and nearby resource evidence, while sparse
// checkpoints retain exact state for positions already visited.
const PREVIEW_RANDOM_SEEK_LOOKBACK_MS: i64 = 2 * 60 * 1_000;

#[derive(Default)]
struct PreviewArchiveCache {
    source: String,
    size: u64,
    modified: Option<SystemTime>,
    offset: u64,
    last_time_ms: Option<i64>,
    active: Vec<CachedPreviewRecord>,
    resources: HashMap<String, Arc<serde_json::Value>>,
    checkpoints: Vec<PreviewCheckpoint>,
    content_revision: u64,
    composition: Option<CachedComposition>,
    parsed_lines: u64,
    composition_count: u64,
    composition_serial: u64,
    reset_count: u64,
}

#[derive(Clone)]
struct CachedPreviewRecord {
    end_ms: i64,
    is_scene: bool,
    value: Arc<serde_json::Value>,
}

#[derive(Clone)]
struct PreviewCheckpoint {
    time_ms: i64,
    offset: u64,
    active: Vec<CachedPreviewRecord>,
    resources: HashMap<String, Arc<serde_json::Value>>,
}

struct CachedComposition {
    revision: u64,
    render: Arc<CachedCaptionRender>,
}

pub(super) struct CachedCaptionRender {
    pub(super) snapshot: Arc<CaptionRenderSnapshot>,
    pub(super) fingerprint: Option<u64>,
    pub(super) overlay: Option<Arc<caption_renderer::CaptionPlaneFrame>>,
}

static PREVIEW_CACHE: OnceLock<Mutex<PreviewArchiveCache>> = OnceLock::new();

#[tauri::command]
pub fn render_at(archive: String, time_ms: i64) -> Result<CaptionRenderSnapshot, String> {
    let render = render_overlay_at(archive.clone(), time_ms)?;
    let mut snapshot = (*render.snapshot).clone();
    snapshot.source = archive;
    snapshot.time_ms = time_ms;
    snapshot.composed_png_base64 = render.overlay.as_ref().and_then(|frame| frame.png_base64());
    snapshot.intervals = snapshot
        .intervals
        .into_iter()
        .map(encode_scene_image)
        .collect();
    Ok(snapshot)
}

pub(super) fn render_overlay_at(
    archive: String,
    time_ms: i64,
) -> Result<Arc<CachedCaptionRender>, String> {
    let metadata = fs::metadata(&archive)
        .map_err(|error| format!("Could not inspect caption archive: {error}"))?;
    let size = metadata.len();
    let modified = metadata.modified().ok();
    let cache_lock = PREVIEW_CACHE.get_or_init(|| Mutex::new(PreviewArchiveCache::default()));
    let mut cache = cache_lock
        .lock()
        .map_err(|_| "Caption preview cache is unavailable.")?;
    let source_changed = cache.source != archive;
    let file_replaced = !source_changed
        && (size < cache.size
            || size < cache.offset
            || (size == cache.size && modified != cache.modified));
    if source_changed || file_replaced {
        let reset_count = cache.reset_count.saturating_add(1);
        let composition_serial = cache.composition_serial;
        *cache = PreviewArchiveCache {
            source: archive.clone(),
            size,
            modified,
            composition_serial,
            reset_count,
            ..Default::default()
        };
        if time_ms > PREVIEW_RANDOM_SEEK_THRESHOLD_MS {
            reposition_preview_cache(&mut cache, &archive, size, time_ms)?;
        }
    } else if cache.last_time_ms.is_some_and(|last| time_ms < last) {
        if !restore_preview_checkpoint(&mut cache, time_ms) {
            reposition_preview_cache(&mut cache, &archive, size, time_ms)?;
        }
    } else if cache
        .last_time_ms
        .is_some_and(|last| time_ms > last.saturating_add(PREVIEW_RANDOM_SEEK_THRESHOLD_MS))
    {
        reposition_preview_cache(&mut cache, &archive, size, time_ms)?;
    }
    let active_before = cache.active.len();
    cache.active.retain(|record| time_ms < record.end_ms);
    if cache.active.len() != active_before {
        mark_preview_content_changed(&mut cache);
    }
    let file = fs::File::open(&archive)
        .map_err(|error| format!("Could not open caption archive: {error}"))?;
    let mut reader = BufReader::new(file);
    reader
        .seek(SeekFrom::Start(cache.offset))
        .map_err(|error| format!("Could not seek caption archive: {error}"))?;
    let mut line = String::new();
    // B24 scenes can carry multi-megabyte RGBA/base64 fields before pts_ms.
    // While scanning toward a random target, retain only the latest raw scene
    // and deserialize that one after the cursor reaches the target. Parsing
    // every replaced snapshot dominated seek latency without changing the
    // resulting caption plane.
    let mut pending_scene: Option<(u64, i64)> = None;
    loop {
        let line_start = reader
            .stream_position()
            .map_err(|error| format!("Could not inspect caption archive position: {error}"))?;
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|error| format!("Could not read caption archive: {error}"))?
            == 0
        {
            break;
        }
        let terminated = line.ends_with('\n');
        cache.parsed_lines = cache.parsed_lines.saturating_add(1);
        if terminated
            && crate::timeline::timeline_record_kind(&line) == Some("scene")
            && let Some(begin) = json_i64_field(&line, "pts_ms")
        {
            if begin > time_ms {
                cache.offset = line_start;
                break;
            }
            let wait = json_i64_field(&line, "wait_duration_ms").unwrap_or(5_000);
            let end = if wait > 0 && wait < i64::MAX / 2 {
                begin.saturating_add(wait)
            } else {
                i64::MAX
            };
            pending_scene = Some((line_start, end));
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        }
        let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&line) else {
            if !terminated {
                // A live worker may be midway through its final JSONL record.
                // Leave that byte range unread until the next append.
                cache.offset = line_start;
                break;
            }
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        };
        let kind = envelope
            .get("kind")
            .and_then(|value| value.as_str())
            .or_else(|| envelope.get("type").and_then(|value| value.as_str()))
            .map(str::to_owned);
        let value = envelope.get("value").cloned().unwrap_or(envelope);
        if kind.as_deref() == Some("resource_evidence") {
            if let Some(key) = value
                .get("record_key")
                .or_else(|| value.get("recordKey"))
                .and_then(serde_json::Value::as_str)
                && (cache.resources.len() < 64 || cache.resources.contains_key(key))
            {
                cache.resources.insert(key.to_owned(), Arc::new(value));
            }
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        }
        if !matches!(
            kind.as_deref(),
            Some("region_interval") | Some("caption") | Some("scene")
        ) {
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        }
        let Some((begin, end)) =
            interval_bounds(&value).or_else(|| scene_bounds(&value, kind.as_deref()))
        else {
            cache.offset = reader.stream_position().unwrap_or(cache.offset);
            continue;
        };
        if begin > time_ms {
            cache.offset = line_start;
            break;
        }
        let is_scene = kind.as_deref() == Some("scene");
        if is_scene {
            // A complete final JSONL record is valid even without a trailing
            // newline and therefore takes precedence over any deferred scene
            // encountered earlier in the scan.
            pending_scene = None;
            // A B24 scene is a complete caption-plane snapshot, not an
            // independently composable layer. A newer scene replaces every
            // older scene even if its wait duration was open-ended.
            let active_before = cache.active.len();
            cache.active.retain(|record| !record.is_scene);
            if cache.active.len() != active_before {
                mark_preview_content_changed(&mut cache);
            }
        }
        if time_ms < end && (is_scene || cache.active.len() < 128) {
            let value = attach_resource_evidence(value, &cache.resources);
            cache.active.push(CachedPreviewRecord {
                end_ms: end,
                is_scene,
                value: Arc::new(value),
            });
            mark_preview_content_changed(&mut cache);
        }
        cache.offset = reader.stream_position().unwrap_or(cache.offset);
    }
    if let Some((scene_offset, end_ms)) = pending_scene {
        // The latest scene replaces every older scene even when it has already
        // expired by the requested time. Clear that state first, but avoid
        // reopening and decoding a multi-megabyte image payload unless the
        // scene can actually contribute to the requested frame.
        let active_before = cache.active.len();
        cache.active.retain(|record| !record.is_scene);
        if cache.active.len() != active_before {
            mark_preview_content_changed(&mut cache);
        }
        if time_ms < end_ms {
            let mut scene_reader = BufReader::new(
                fs::File::open(&archive)
                    .map_err(|error| format!("Could not reopen caption archive: {error}"))?,
            );
            scene_reader
                .seek(SeekFrom::Start(scene_offset))
                .map_err(|error| format!("Could not seek to the active B24 scene: {error}"))?;
            let mut scene = String::new();
            scene_reader
                .read_line(&mut scene)
                .map_err(|error| format!("Could not read the active B24 scene: {error}"))?;
            let envelope = serde_json::from_str::<serde_json::Value>(&scene)
                .map_err(|error| format!("Could not decode the active B24 scene: {error}"))?;
            let value = envelope.get("value").cloned().unwrap_or(envelope);
            let value = attach_resource_evidence(value, &cache.resources);
            cache.active.push(CachedPreviewRecord {
                end_ms,
                is_scene: true,
                value: Arc::new(value),
            });
            mark_preview_content_changed(&mut cache);
        }
    }
    cache.size = size;
    cache.modified = modified;
    cache.last_time_ms = Some(time_ms);
    save_preview_checkpoint(&mut cache, time_ms);
    // The rendered B24 scene is authoritative whenever one exists. Region
    // intervals are retained for timeline/export semantics, but rendering
    // them together with a scene duplicates old and new caption states.
    let has_scene = cache.active.iter().any(|record| record.is_scene);
    let intervals: Vec<serde_json::Value> = cache
        .active
        .iter()
        .filter(|record| record.is_scene == has_scene)
        .map(|record| (*record.value).clone())
        .collect();
    if cache
        .composition
        .as_ref()
        .is_none_or(|composition| composition.revision != cache.content_revision)
    {
        let resource_previews = active_resource_previews(&intervals);
        let composed = caption_renderer::compose(&intervals);
        let overlay = composed.map(Arc::new);
        let snapshot = Arc::new(CaptionRenderSnapshot {
            source: archive.clone(),
            time_ms,
            intervals,
            resource_previews,
            plane_width: overlay.as_ref().map(|frame| frame.width),
            plane_height: overlay.as_ref().map(|frame| frame.height),
            // Playback consumes the cached RGBA plane directly. PNG is only
            // generated by the explicit render/debug commands.
            composed_png_base64: None,
            active_layer_count: overlay.as_ref().map(|frame| frame.layer_count).unwrap_or(0),
            caption_plane_mode: overlay
                .as_ref()
                .map(|frame| frame.mode.into())
                .unwrap_or_else(|| "ttml-structural-only".into()),
            missing_glyph_count: overlay
                .as_ref()
                .map(|frame| frame.missing_glyph_count)
                .unwrap_or(0),
            rendered_ruby_count: overlay
                .as_ref()
                .map(|frame| frame.rendered_ruby_count)
                .unwrap_or(0),
            render_profile: CaptionRenderProfile {
                renderer: "native-caption-plane-compositor+libaribcaption".into(),
                font_family: "Rounded M+ 1m for ARIB".into(),
                preserve_character_cells: true,
                ruby_scale: 0.5,
                background_alpha_from_source: true,
                stroke_from_source: true,
                drcs_policy: "preserve-glyph-assets".into(),
            },
        });
        cache.composition_serial = cache.composition_serial.wrapping_add(1);
        let fingerprint = overlay.as_ref().map(|_| cache.composition_serial);
        cache.composition = Some(CachedComposition {
            revision: cache.content_revision,
            render: Arc::new(CachedCaptionRender {
                snapshot,
                fingerprint,
                overlay,
            }),
        });
        cache.composition_count = cache.composition_count.saturating_add(1);
    }
    Ok(cache
        .composition
        .as_ref()
        .expect("preview composition must be populated")
        .render
        .clone())
}

fn json_i64_field(line: &str, field: &str) -> Option<i64> {
    let needle = format!("\"{field}\"");
    let bytes = line.as_bytes();
    // `str::find` uses an optimized substring search. A byte-by-byte windows
    // scan is disproportionately expensive in debug builds and on scene
    // records whose RGBA payload can be several megabytes.
    let position = if field == "wait_duration_ms" {
        // Worker scene records place the wait at the end, after an optional
        // multi-megabyte RGBA payload. Search backwards so an open-ended clear
        // scene does not scan the entire image a second time.
        line.rfind(&needle)?
    } else {
        line.find(&needle)?
    };
    let mut cursor = position + needle.len();
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b':') {
        return None;
    }
    cursor += 1;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    let start = cursor;
    if bytes.get(cursor) == Some(&b'-') {
        cursor += 1;
    }
    while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
    }
    (cursor > start)
        .then(|| {
            std::str::from_utf8(&bytes[start..cursor])
                .ok()?
                .parse()
                .ok()
        })
        .flatten()
}

fn mark_preview_content_changed(cache: &mut PreviewArchiveCache) {
    cache.content_revision = cache.content_revision.wrapping_add(1);
}

fn restore_preview_checkpoint(cache: &mut PreviewArchiveCache, time_ms: i64) -> bool {
    if let Some(checkpoint) = cache
        .checkpoints
        .iter()
        .rev()
        .find(|checkpoint| checkpoint.time_ms <= time_ms)
        .cloned()
    {
        cache.offset = checkpoint.offset;
        cache.last_time_ms = Some(checkpoint.time_ms);
        cache.active = checkpoint.active;
        cache.resources = checkpoint.resources;
        mark_preview_content_changed(cache);
        cache.composition = None;
        true
    } else {
        false
    }
}

fn reposition_preview_cache(
    cache: &mut PreviewArchiveCache,
    archive: &str,
    size: u64,
    time_ms: i64,
) -> Result<(), String> {
    let seek_target = time_ms
        .saturating_sub(PREVIEW_RANDOM_SEEK_LOOKBACK_MS)
        .max(0);
    cache.offset = crate::timeline::approximate_archive_time_offset(archive, size, seek_target)?;
    cache.last_time_ms = None;
    cache.active.clear();
    cache.resources.clear();
    mark_preview_content_changed(cache);
    cache.composition = None;
    Ok(())
}

fn save_preview_checkpoint(cache: &mut PreviewArchiveCache, time_ms: i64) {
    if cache
        .checkpoints
        .iter()
        .any(|checkpoint| (checkpoint.time_ms - time_ms).abs() < PREVIEW_CHECKPOINT_INTERVAL_MS)
    {
        return;
    }
    let checkpoint = PreviewCheckpoint {
        time_ms,
        offset: cache.offset,
        active: cache.active.clone(),
        resources: cache.resources.clone(),
    };
    let position = cache
        .checkpoints
        .partition_point(|existing| existing.time_ms < time_ms);
    cache.checkpoints.insert(position, checkpoint);
    if cache.checkpoints.len() > MAX_PREVIEW_CHECKPOINTS {
        cache.checkpoints.remove(0);
    }
}

#[cfg(test)]
pub(super) fn preview_cache_metrics() -> (u64, u64, u64, u64, usize) {
    let cache = PREVIEW_CACHE
        .get_or_init(|| Mutex::new(PreviewArchiveCache::default()))
        .lock()
        .expect("preview cache metrics");
    (
        cache.offset,
        cache.parsed_lines,
        cache.composition_count,
        cache.reset_count,
        cache.checkpoints.len(),
    )
}

fn attach_resource_evidence(
    mut value: serde_json::Value,
    resources: &HashMap<String, Arc<serde_json::Value>>,
) -> serde_json::Value {
    let Some(source) = value.get("source") else {
        return value;
    };
    let packet_id = source
        .get("mmpt_packet_id")
        .or_else(|| source.get("mmptPacketId"))
        .and_then(serde_json::Value::as_u64);
    let mpu_sequence = source
        .get("mpu_sequence_number")
        .or_else(|| source.get("mpuSequenceNumber"))
        .and_then(serde_json::Value::as_u64);
    let Some((packet_id, mpu_sequence)) = packet_id.zip(mpu_sequence) else {
        return value;
    };
    let Some(style) = value.get("style") else {
        return value;
    };
    let mut attached = Vec::new();
    for (usage, uri) in [
        (
            "background-image",
            style
                .get("background_image")
                .or_else(|| style.get("backgroundImage")),
        ),
        (
            "font-face",
            style
                .get("font_resource")
                .or_else(|| style.get("fontResource")),
        ),
    ] {
        let Some(uri) = uri.and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(index) = uri
            .strip_prefix("subt://")
            .filter(|index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()))
        else {
            continue;
        };
        let Ok(index) = index.parse::<u8>() else {
            continue;
        };
        let record_key =
            format!("stpp-resource:packet:{packet_id}:mpu:{mpu_sequence}:subsample:{index}");
        let Some(resource) = resources.get(&record_key) else {
            continue;
        };
        attached.push(serde_json::json!({
            "usage": usage,
            "uri": uri,
            "record_key": record_key,
            "format_hint": resource.get("format_hint").or_else(|| resource.get("formatHint")),
            "format_validation": resource.get("format_validation").or_else(|| resource.get("formatValidation")),
            "width": resource.get("width"),
            "height": resource.get("height"),
            "preview_data_uri": resource.get("preview_data_uri").or_else(|| resource.get("previewDataUri")),
        }));
    }
    if !attached.is_empty()
        && let Some(object) = value.as_object_mut()
    {
        object.insert(
            "native_resources".into(),
            serde_json::Value::Array(attached),
        );
    }
    value
}

fn active_resource_previews(intervals: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut keys = HashSet::new();
    let mut previews = Vec::new();
    for interval in intervals {
        let Some(resources) = interval
            .get("native_resources")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for resource in resources {
            let Some(key) = resource
                .get("record_key")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            if keys.insert(key.to_owned()) {
                previews.push(resource.clone());
            }
        }
    }
    previews
}

pub(super) fn encode_scene_image(mut value: serde_json::Value) -> serde_json::Value {
    let Some(image) = value
        .get_mut("rendered_image")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return value;
    };
    let Some(raw) = image.get("rgba_base64").and_then(serde_json::Value::as_str) else {
        return value;
    };
    let Some(width) = image.get("width").and_then(serde_json::Value::as_u64) else {
        return value;
    };
    let Some(height) = image.get("height").and_then(serde_json::Value::as_u64) else {
        return value;
    };
    let Some(stride) = image.get("stride").and_then(serde_json::Value::as_u64) else {
        return value;
    };
    let Ok(rgba) = BASE64.decode(raw) else {
        return value;
    };
    let expected = stride.saturating_mul(height) as usize;
    if width == 0 || height == 0 || stride < width.saturating_mul(4) || rgba.len() < expected {
        return value;
    }
    let packed: Vec<u8> = (0..height as usize)
        .flat_map(|row| {
            let start = row * stride as usize;
            rgba[start..start + width as usize * 4].iter().copied()
        })
        .collect();
    let mut png_bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut png_bytes, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let Ok(mut writer) = encoder.write_header() else {
        return value;
    };
    if writer.write_image_data(&packed).is_err() {
        return value;
    }
    drop(writer);
    image.remove("rgba_base64");
    image.insert(
        "png_base64".into(),
        serde_json::Value::String(BASE64.encode(png_bytes)),
    );
    value
}

pub(super) fn interval_bounds(value: &serde_json::Value) -> Option<(i64, i64)> {
    let begin = ["begin_ms", "start_ms", "beginMs", "startMs"]
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_i64))?;
    let end = ["end_ms", "endMs"]
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_i64))
        .unwrap_or_else(|| begin.saturating_add(5_000));
    (end > begin).then_some((begin, end))
}

pub(super) fn scene_bounds(value: &serde_json::Value, kind: Option<&str>) -> Option<(i64, i64)> {
    if kind != Some("scene") {
        return None;
    }
    let begin = value.get("pts_ms").and_then(serde_json::Value::as_i64)?;
    let wait = value
        .get("wait_duration_ms")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(5_000);
    let end = if wait > 0 && wait < i64::MAX / 2 {
        begin.saturating_add(wait)
    } else {
        // An open-ended B24 scene stays visible until the next scene replaces
        // it. The cache performs that replacement explicitly above.
        i64::MAX
    };
    (end > begin).then_some((begin, end))
}
