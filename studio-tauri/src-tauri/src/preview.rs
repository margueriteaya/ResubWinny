use crate::{
    caption_renderer,
    models::{
        BroadcastMetadata, CaptionRenderProfile, CaptionRenderSnapshot, PlaybackTimeMapping,
        PreviewCapabilities, PreviewOverlaySyncResult, PreviewPlaybackState, PreviewRect,
        PreviewRenderDiagnostics, PreviewRuntime,
    },
    preview_surface,
    state::AppState,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use std::{
    collections::{HashMap, HashSet},
    fs,
    hash::{Hash, Hasher},
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex, OnceLock},
    time::SystemTime,
};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn set_caption_font(state: State<'_, Arc<AppState>>, font: String) -> Result<(), String> {
    if font != "arib" && font != "system" {
        return Err("Unsupported caption preview font policy.".into());
    }
    *state
        .caption_font
        .lock()
        .map_err(|_| "Player configuration is unavailable")? = font;
    Ok(())
}

#[tauri::command]
pub fn get_preview_capabilities(app: AppHandle) -> PreviewCapabilities {
    let resource_dir = app.path().resource_dir().ok();
    let runtime = crate::libmpv::discover_library(resource_dir.as_deref())
        .ok()
        .and_then(|path| {
            crate::libmpv::render_api_available(&path)
                .ok()
                .map(|render_api| (path, render_api))
        });
    let runtime_ready = runtime.is_some();
    let render_surface_ready = runtime.as_ref().is_some_and(|(_, render_api)| *render_api);
    let native_embedding_supported = cfg!(windows);
    let client_available = native_embedding_supported && runtime_ready;
    PreviewCapabilities {
        video_backend: "libmpv-client".into(),
        caption_overlay_modes: preview_surface::capabilities(
            client_available,
            native_embedding_supported && render_surface_ready,
            native_embedding_supported,
        ),
        selected_caption_overlay: if native_embedding_supported && render_surface_ready {
            "libmpv-render".into()
        } else if client_available {
            "libmpv-client-overlay".into()
        } else {
            "none".into()
        },
        caption_plane_modes: vec![
            "b24-native-rgba".into(),
            "ttml-horizontal-native".into(),
            "ttml-horizontal-ruby-basic-native".into(),
            "ttml-vertical-basic-native".into(),
            "ttml-vertical-ruby-basic-native".into(),
            "ttml-structural-only".into(),
        ],
        available_caption_plane_modes: vec![
            "b24-native-rgba".into(),
            "ttml-horizontal-native".into(),
            "ttml-horizontal-ruby-basic-native".into(),
            "ttml-vertical-basic-native".into(),
            "ttml-vertical-ruby-basic-native".into(),
            "ttml-structural-only".into(),
        ],
    }
}

#[tauri::command]
pub fn get_preview_runtime(app: AppHandle) -> PreviewRuntime {
    let resource_dir = app.path().resource_dir().ok();
    match crate::libmpv::discover_library(resource_dir.as_deref()) {
        Ok(path) => PreviewRuntime {
            backend: "libmpv-client".into(),
            platform: std::env::consts::OS.into(),
            library_path: Some(path.display().to_string()),
            available: true,
            render_api_available: crate::libmpv::render_api_available(&path).unwrap_or(false),
            detail: "libmpv runtime discovered. Windows uses the native OpenGL render surface when the runtime exports the complete render API; a failed per-source render setup falls back to the client overlay route.".into(),
        },
        Err(detail) => PreviewRuntime {
            backend: "libmpv-client".into(),
            platform: std::env::consts::OS.into(),
            library_path: None,
            available: false,
            render_api_available: false,
            detail,
        },
    }
}

#[tauri::command]
pub fn get_preview_render_diagnostics(
    state: State<'_, Arc<AppState>>,
) -> Result<PreviewRenderDiagnostics, String> {
    let player = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let Some(player) = player.as_ref() else {
        return Ok(PreviewRenderDiagnostics {
            route: "none".into(),
            active: false,
            frames_presented: 0,
            presents_per_second: 0.0,
            caption_texture_uploads: 0,
            caption_texture_clears: 0,
            video_aspect: None,
            surface_width: None,
            surface_height: None,
            decoder_mode: None,
            fallback_reason: None,
            last_error: None,
        });
    };
    let Some(stats) = player.player.render_diagnostics() else {
        return Ok(PreviewRenderDiagnostics {
            route: "libmpv-client-overlay".into(),
            active: true,
            frames_presented: 0,
            presents_per_second: 0.0,
            caption_texture_uploads: 0,
            caption_texture_clears: 0,
            video_aspect: None,
            surface_width: None,
            surface_height: None,
            decoder_mode: Some("auto-safe".into()),
            fallback_reason: player.render_fallback_reason.clone(),
            last_error: None,
        });
    };
    Ok(PreviewRenderDiagnostics {
        route: "libmpv-render".into(),
        active: true,
        frames_presented: stats.frames_presented,
        presents_per_second: stats.presents_per_second,
        caption_texture_uploads: stats.caption_texture_uploads,
        caption_texture_clears: stats.caption_texture_clears,
        video_aspect: stats.video_aspect,
        surface_width: Some(stats.surface_width),
        surface_height: Some(stats.surface_height),
        decoder_mode: Some(stats.decoder_mode),
        fallback_reason: None,
        last_error: stats.last_error,
    })
}

fn set_caption_overlay_impl(
    state: State<'_, Arc<AppState>>,
    png_base64: &str,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let bytes = BASE64
        .decode(png_base64)
        .map_err(|_| "Caption overlay is not valid base64.".to_string())?;
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("Could not decode caption overlay: {error}"))?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .map_err(|error| format!("Could not read caption overlay: {error}"))?;
    if info.width == 0 || info.height == 0 {
        return Err("Caption overlay has no pixels.".into());
    }
    let rgba = &pixels[..info.buffer_size()];
    let mut bgra = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    platform_caption_overlay(state, bgra, info.width as i32, info.height as i32, x, y)
}

#[tauri::command]
pub fn clear_caption_overlay(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    platform_clear_caption_overlay(state.clone())?;
    reset_overlay_sync(state.inner());
    Ok(())
}

#[tauri::command]
pub fn get_preview_time(state: State<'_, Arc<AppState>>) -> Result<Option<f64>, String> {
    platform_preview_time(state)
}

#[tauri::command]
pub fn get_preview_duration(state: State<'_, Arc<AppState>>) -> Result<Option<f64>, String> {
    platform_preview_duration(state)
}

#[tauri::command]
pub fn get_preview_playback_state(
    state: State<'_, Arc<AppState>>,
) -> Result<PreviewPlaybackState, String> {
    platform_preview_playback_state(state)
}

#[tauri::command]
pub fn get_preview_broadcast_metadata(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    service_id: Option<u16>,
) -> Result<BroadcastMetadata, String> {
    platform_preview_broadcast_metadata(app, state, service_id)
}

#[tauri::command]
pub fn get_playback_time_mapping(
    state: State<'_, Arc<AppState>>,
) -> Result<PlaybackTimeMapping, String> {
    state
        .playback_time_mapping
        .lock()
        .map(|mapping| mapping.clone())
        .map_err(|_| "Playback time mapping is unavailable.".into())
}

#[tauri::command]
pub fn update_playback_time_mapping(
    state: State<'_, Arc<AppState>>,
    mapping: PlaybackTimeMapping,
) -> Result<(), String> {
    mapping.project_time_ms(mapping.media_anchor_ms)?;
    *state
        .playback_time_mapping
        .lock()
        .map_err(|_| "Playback time mapping is unavailable.")? = mapping;
    reset_overlay_sync(state.inner());
    Ok(())
}

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

#[tauri::command]
pub fn render_preview_at(
    state: State<'_, Arc<AppState>>,
    archive: String,
    time_ms: i64,
    x: i32,
    y: i32,
) -> Result<CaptionRenderSnapshot, String> {
    let snapshot = render_at(archive, time_ms)?;
    let has_player = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?
        .is_some();
    if let Some(encoded) = snapshot.composed_png_base64.as_deref().or_else(|| {
        snapshot.intervals.iter().find_map(|interval| {
            interval
                .get("rendered_image")
                .and_then(|image| image.get("png_base64").or_else(|| image.get("pngBase64")))
                .and_then(serde_json::Value::as_str)
        })
    }) {
        set_caption_overlay_impl(state, encoded, x, y)?;
    } else if has_player {
        platform_clear_caption_overlay(state)?;
    }
    Ok(snapshot)
}

/// Synchronizes the native caption plane against mpv's authoritative playback
/// time. The UI may poll this low-frequency operation, but never estimates
/// media time, lays out subtitles, or sends video frames through the WebView.
#[tauri::command]
pub fn sync_preview_overlay(
    state: State<'_, Arc<AppState>>,
    archive: String,
) -> Result<PreviewOverlaySyncResult, String> {
    let Some(player_time) = platform_preview_time(state.clone())? else {
        return Ok(PreviewOverlaySyncResult {
            action: "awaiting-player-time".into(),
            media_time_ms: None,
            project_time_ms: None,
            snapshot: None,
        });
    };
    let media_time_ms = seconds_to_milliseconds(player_time);
    let project_time_ms = state
        .playback_time_mapping
        .lock()
        .map_err(|_| "Playback time mapping is unavailable.")?
        .project_time_ms(media_time_ms)?;
    // Preview indexing starts in parallel with libmpv. The bounded archive
    // may not have created its first `.part` file yet; that is an expected
    // readiness state, not a user-facing backend failure.
    if !Path::new(&archive).is_file() {
        return Ok(PreviewOverlaySyncResult {
            action: "awaiting-caption-index".into(),
            media_time_ms: Some(media_time_ms),
            project_time_ms: Some(project_time_ms),
            snapshot: None,
        });
    }
    let snapshot = render_at(archive.clone(), project_time_ms)?;
    let fingerprint = snapshot_overlay_fingerprint(&snapshot);
    let action = {
        let sync = state
            .preview_overlay_sync
            .lock()
            .map_err(|_| "Preview overlay state is unavailable")?;
        decide_overlay_action(&sync, &archive, fingerprint)
    };

    match action {
        OverlayAction::Apply => {
            let encoded = snapshot_overlay_png(&snapshot)
                .ok_or("Caption renderer produced an invalid overlay state.")?;
            set_caption_overlay_impl(state.clone(), encoded, 0, 0)?;
        }
        OverlayAction::Clear => platform_clear_caption_overlay(state.clone())?,
        OverlayAction::Unchanged => {}
    }

    let mut sync = state
        .preview_overlay_sync
        .lock()
        .map_err(|_| "Preview overlay state is unavailable")?;
    sync.archive = archive;
    sync.fingerprint = fingerprint;
    sync.overlay_visible = fingerprint.is_some();
    Ok(PreviewOverlaySyncResult {
        action: action.as_str().into(),
        media_time_ms: Some(media_time_ms),
        project_time_ms: Some(project_time_ms),
        snapshot: Some(snapshot),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverlayAction {
    Apply,
    Clear,
    Unchanged,
}

impl OverlayAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "applied",
            Self::Clear => "cleared",
            Self::Unchanged => "unchanged",
        }
    }
}

fn decide_overlay_action(
    sync: &crate::state::PreviewOverlaySyncState,
    archive: &str,
    fingerprint: Option<u64>,
) -> OverlayAction {
    match fingerprint {
        Some(value) if sync.archive != archive || sync.fingerprint != Some(value) => {
            OverlayAction::Apply
        }
        Some(_) => OverlayAction::Unchanged,
        None if sync.overlay_visible => OverlayAction::Clear,
        None => OverlayAction::Unchanged,
    }
}

fn snapshot_overlay_png(snapshot: &CaptionRenderSnapshot) -> Option<&str> {
    snapshot.composed_png_base64.as_deref().or_else(|| {
        snapshot.intervals.iter().find_map(|interval| {
            interval
                .get("rendered_image")
                .and_then(|image| image.get("png_base64").or_else(|| image.get("pngBase64")))
                .and_then(serde_json::Value::as_str)
        })
    })
}

fn snapshot_overlay_fingerprint(snapshot: &CaptionRenderSnapshot) -> Option<u64> {
    let image = snapshot_overlay_png(snapshot)?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    image.hash(&mut hasher);
    Some(hasher.finish())
}

fn playback_file_offset(
    file_size: u64,
    time_seconds: Option<f64>,
    duration_seconds: Option<f64>,
    fallback_offset: Option<f64>,
) -> u64 {
    if let (Some(time), Some(duration)) = (time_seconds, duration_seconds)
        && time.is_finite()
        && duration.is_finite()
        && time >= 0.0
        && duration > 0.0
        && file_size > 0
    {
        return ((time / duration).clamp(0.0, 1.0) * file_size as f64)
            .round()
            .clamp(0.0, file_size.saturating_sub(1) as f64) as u64;
    }
    fallback_offset
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0) as u64
}

fn seconds_to_milliseconds(seconds: f64) -> i64 {
    (seconds * 1_000.0).clamp(0.0, i64::MAX as f64).round() as i64
}

fn reset_overlay_sync(state: &AppState) {
    if let Ok(mut sync) = state.preview_overlay_sync.lock() {
        *sync = crate::state::PreviewOverlaySyncState::default();
    }
}

fn encode_scene_image(mut value: serde_json::Value) -> serde_json::Value {
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

fn interval_bounds(value: &serde_json::Value) -> Option<(i64, i64)> {
    let begin = ["begin_ms", "start_ms", "beginMs", "startMs"]
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_i64))?;
    let end = ["end_ms", "endMs"]
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_i64))
        .unwrap_or_else(|| begin.saturating_add(5_000));
    (end > begin).then_some((begin, end))
}

fn scene_bounds(value: &serde_json::Value, kind: Option<&str>) -> Option<(i64, i64)> {
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

#[cfg(test)]
fn parse_mpv_time_response(response: &str) -> Option<f64> {
    let value = serde_json::from_str::<serde_json::Value>(response).ok()?;
    if value
        .get("error")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|error| error != "success")
    {
        return None;
    }
    value
        .get("data")
        .and_then(serde_json::Value::as_f64)
        .filter(|time| time.is_finite() && *time >= 0.0)
}

#[cfg(test)]
fn mpv_overlay_command(path: &Path, x: i32, y: i32, width: i32, height: i32) -> serde_json::Value {
    serde_json::json!({
        "command": [
            "overlay-add", 1, x, y, path.to_string_lossy(), 0, "bgra", width,
            height, width.saturating_mul(4)
        ]
    })
}

#[path = "preview/native.rs"]
mod native;
#[cfg(test)]
#[path = "preview/tests.rs"]
mod tests;
#[cfg(windows)]
pub use native::{
    caption_overlay as platform_caption_overlay,
    clear_caption_overlay as platform_clear_caption_overlay,
    preview_broadcast_metadata as platform_preview_broadcast_metadata,
    preview_command as platform_preview_command, preview_duration as platform_preview_duration,
    preview_playback_state as platform_preview_playback_state,
    preview_time as platform_preview_time, recover_preview as platform_recover_preview,
    resize_preview as platform_resize_preview, start_preview as platform_start_preview,
    stop_preview as platform_stop_preview,
};
#[cfg(not(windows))]
#[path = "preview/unsupported.rs"]
mod unsupported;
#[cfg(not(windows))]
pub use unsupported::{
    caption_overlay as platform_caption_overlay,
    clear_caption_overlay as platform_clear_caption_overlay,
    preview_broadcast_metadata as platform_preview_broadcast_metadata,
    preview_command as platform_preview_command, preview_duration as platform_preview_duration,
    preview_playback_state as platform_preview_playback_state,
    preview_time as platform_preview_time, recover_preview as platform_recover_preview,
    resize_preview as platform_resize_preview, start_preview as platform_start_preview,
    stop_preview as platform_stop_preview,
};

// Tauri's command registration macro requires the generated command symbols to
// live in this module, so platform implementations are deliberately wrapped.
#[tauri::command]
pub fn start_preview(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewRect,
) -> Result<(), String> {
    reset_overlay_sync(state.inner());
    platform_start_preview(app, state, source, rect)
}
#[tauri::command]
#[allow(
    clippy::too_many_arguments,
    reason = "the command restores a complete native playback session"
)]
pub fn recover_preview(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewRect,
    time_seconds: Option<f64>,
    paused: bool,
    volume: f64,
) -> Result<(), String> {
    reset_overlay_sync(state.inner());
    platform_recover_preview(app, state, source, rect, time_seconds, paused, volume)
}
#[tauri::command]
pub fn resize_preview(state: State<'_, Arc<AppState>>, rect: PreviewRect) -> Result<(), String> {
    platform_resize_preview(state, rect)
}
#[tauri::command]
pub fn stop_preview(state: State<'_, Arc<AppState>>) {
    platform_stop_preview(state.clone());
    reset_overlay_sync(state.inner());
}
#[tauri::command]
pub fn preview_command(state: State<'_, Arc<AppState>>, command: String) -> Result<(), String> {
    platform_preview_command(state, command)
}
