use super::*;

#[derive(Default)]
struct PreviewArchiveCache {
    source: String,
    size: u64,
    modified: Option<SystemTime>,
    offset: u64,
    last_time_ms: Option<i64>,
    active: Vec<CachedPreviewRecord>,
    resources: HashMap<String, serde_json::Value>,
}

struct CachedPreviewRecord {
    end_ms: i64,
    is_scene: bool,
    value: serde_json::Value,
}

static PREVIEW_CACHE: OnceLock<Mutex<PreviewArchiveCache>> = OnceLock::new();

#[tauri::command]
pub fn render_at(archive: String, time_ms: i64) -> Result<CaptionRenderSnapshot, String> {
    let metadata = fs::metadata(&archive)
        .map_err(|error| format!("Could not inspect caption archive: {error}"))?;
    let size = metadata.len();
    let modified = metadata.modified().ok();
    let cache_lock = PREVIEW_CACHE.get_or_init(|| Mutex::new(PreviewArchiveCache::default()));
    let mut cache = cache_lock
        .lock()
        .map_err(|_| "Caption preview cache is unavailable.")?;
    let reset = cache.source != archive
        || cache.size != size
        || cache.modified != modified
        || cache.last_time_ms.is_some_and(|last| time_ms < last);
    if reset {
        cache.source = archive.clone();
        cache.size = size;
        cache.modified = modified;
        cache.offset = 0;
        cache.last_time_ms = None;
        cache.active.clear();
        cache.resources.clear();
    }
    cache.active.retain(|record| time_ms < record.end_ms);
    let file = fs::File::open(&archive)
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
        if reader
            .read_line(&mut line)
            .map_err(|error| format!("Could not read caption archive: {error}"))?
            == 0
        {
            break;
        }
        let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&line) else {
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
                cache.resources.insert(key.to_owned(), value);
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
            // A B24 scene is a complete caption-plane snapshot, not an
            // independently composable layer. A newer scene replaces every
            // older scene even if its wait duration was open-ended.
            cache.active.retain(|record| !record.is_scene);
        }
        if time_ms < end && (is_scene || cache.active.len() < 128) {
            let value = encode_scene_image(attach_resource_evidence(value, &cache.resources));
            cache.active.push(CachedPreviewRecord {
                end_ms: end,
                is_scene,
                value,
            });
        }
        cache.offset = reader.stream_position().unwrap_or(cache.offset);
    }
    cache.last_time_ms = Some(time_ms);
    // The rendered B24 scene is authoritative whenever one exists. Region
    // intervals are retained for timeline/export semantics, but rendering
    // them together with a scene duplicates old and new caption states.
    let has_scene = cache.active.iter().any(|record| record.is_scene);
    let intervals: Vec<serde_json::Value> = cache
        .active
        .iter()
        .filter(|record| record.is_scene == has_scene)
        .map(|record| record.value.clone())
        .collect();
    let resource_previews = active_resource_previews(&intervals);
    let composed = caption_renderer::compose(&intervals);
    Ok(CaptionRenderSnapshot {
        source: archive,
        time_ms,
        intervals,
        resource_previews,
        plane_width: composed.as_ref().map(|frame| frame.width),
        plane_height: composed.as_ref().map(|frame| frame.height),
        composed_png_base64: composed.as_ref().map(|frame| frame.png_base64.clone()),
        active_layer_count: composed
            .as_ref()
            .map(|frame| frame.layer_count)
            .unwrap_or(0),
        caption_plane_mode: composed
            .as_ref()
            .map(|frame| frame.mode.into())
            .unwrap_or_else(|| "ttml-structural-only".into()),
        missing_glyph_count: composed
            .as_ref()
            .map(|frame| frame.missing_glyph_count)
            .unwrap_or(0),
        rendered_ruby_count: composed
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
    })
}

fn attach_resource_evidence(
    mut value: serde_json::Value,
    resources: &HashMap<String, serde_json::Value>,
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
