use super::archive::render_at;
use super::*;

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
pub(super) enum OverlayAction {
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

pub(super) fn decide_overlay_action(
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

pub(crate) fn playback_file_offset(
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

pub(super) fn seconds_to_milliseconds(seconds: f64) -> i64 {
    (seconds * 1_000.0).clamp(0.0, i64::MAX as f64).round() as i64
}

pub(crate) fn reset_overlay_sync(state: &AppState) {
    if let Ok(mut sync) = state.preview_overlay_sync.lock() {
        *sync = crate::state::PreviewOverlaySyncState::default();
    }
}
