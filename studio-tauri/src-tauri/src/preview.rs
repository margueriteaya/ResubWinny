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
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex, OnceLock},
    time::SystemTime,
};
use tauri::{AppHandle, Manager, State};
pub(crate) mod archive;
pub(crate) mod overlay;

pub(crate) use overlay::{playback_file_offset, reset_overlay_sync};
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
#[cfg(windows)]
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

#[cfg(not(windows))]
#[tauri::command]
pub fn get_preview_render_diagnostics(
    _: State<'_, Arc<AppState>>,
) -> Result<PreviewRenderDiagnostics, String> {
    Ok(PreviewRenderDiagnostics {
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
        fallback_reason: Some("Native preview is only implemented on Windows.".into()),
        last_error: None,
    })
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
#[cfg(windows)]
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

fn invalidate_overlay_after_seek(state: State<'_, Arc<AppState>>) {
    if let Ok(mut sync) = state.preview_overlay_sync.lock() {
        let overlay_was_visible = sync.overlay_visible;
        let revision = sync.revision.wrapping_add(1);
        *sync = crate::state::PreviewOverlaySyncState {
            revision,
            ..Default::default()
        };
        if overlay_was_visible {
            let _ = platform_clear_caption_overlay(state.clone());
        }
    } else {
        let _ = platform_clear_caption_overlay(state);
    }
}

#[tauri::command]
pub fn preview_command(state: State<'_, Arc<AppState>>, command: String) -> Result<(), String> {
    let moves_timeline = matches!(
        command.as_str(),
        "seek-back" | "seek-forward" | "frame-back" | "frame-forward"
    ) || command.starts_with("seek-absolute:");
    platform_preview_command(state.clone(), command)?;
    if moves_timeline {
        invalidate_overlay_after_seek(state);
    }
    Ok(())
}

#[tauri::command]
pub fn seek_preview_project(
    state: State<'_, Arc<AppState>>,
    project_time_ms: i64,
    exact: bool,
) -> Result<i64, String> {
    let media_time_ms = state
        .playback_time_mapping
        .lock()
        .map_err(|_| "Playback time mapping is unavailable.")?
        .media_time_ms(project_time_ms)?
        .max(0);
    platform_preview_command(
        state.clone(),
        format!(
            "{}:{}",
            if exact {
                "seek-absolute"
            } else {
                "seek-preview"
            },
            media_time_ms as f64 / 1_000.0
        ),
    )?;
    // Approximate seeks are emitted continuously while the user drags. Do
    // not tear down the native caption plane for every intermediate target:
    // that turns a scrub into a sequence of expensive clear/rebuild cycles
    // and leaves the overlay visibly behind the pointer. An exact seek (the
    // release/click path) invalidates the cached plane so the next sync can
    // apply the frame at the authoritative player time.
    if exact {
        invalidate_overlay_after_seek(state);
    } else if let Ok(mut sync) = state.preview_overlay_sync.lock() {
        sync.revision = sync.revision.wrapping_add(1);
    }
    Ok(media_time_ms)
}
