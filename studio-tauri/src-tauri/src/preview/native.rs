use super::*;
use crate::{
    models::{BroadcastMetadata, PreviewPlaybackState},
    state::{PlayerHost, PreviewBroadcastCache},
    worker::worker_path,
};
use std::process::Command;
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::ClientToScreen,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
            HWND_TOP, RegisterClassW, SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos,
            ShowWindow, WNDCLASSW, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
        },
    },
    core::w,
};

unsafe extern "system" fn preview_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn preview_window_instance() -> Result<HINSTANCE, String> {
    static INSTANCE: std::sync::OnceLock<Result<isize, String>> = std::sync::OnceLock::new();
    let instance = INSTANCE.get_or_init(|| {
        let module = unsafe { GetModuleHandleW(None) }
            .map_err(|error| format!("Could not locate the application module: {error}"))?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(preview_window_proc),
            hInstance: instance,
            lpszClassName: w!("ResubWinnyPreviewHost"),
            // An OpenGL host must not have a class background brush: a
            // WM_PAINT from its parent would otherwise erase the front buffer
            // after SwapBuffers and leave an apparently healthy black player.
            ..Default::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err("Could not register the native preview window class.".into());
        }
        Ok(instance.0 as isize)
    });
    instance
        .as_ref()
        .map(|value| HINSTANCE(*value as *mut _))
        .map_err(Clone::clone)
}

fn preview_screen_origin(owner: HWND, rect: &PreviewRect) -> Result<POINT, String> {
    let mut origin = POINT {
        x: rect.x,
        y: rect.y,
    };
    if unsafe { ClientToScreen(owner, &mut origin) }.as_bool() {
        Ok(origin)
    } else {
        Err(format!(
            "Could not position the native preview over the WebView: {}",
            windows::core::Error::from_win32()
        ))
    }
}

fn stop_host(state: &AppState) {
    if let Ok(mut slot) = state.player.lock()
        && let Some(player) = slot.take()
    {
        // The host is above WebView2. Hide it before libmpv teardown
        // so it cannot cover a newly selected Svelte page.
        unsafe {
            let _ = ShowWindow(HWND(player.host as *mut _), SW_HIDE);
        }
        let _ = fs::remove_file(&player.overlay_path);
        player.player.stop();
        unsafe {
            let _ = DestroyWindow(HWND(player.host as *mut _));
        }
    }
    if let Ok(mut cache) = state.preview_broadcast_cache.lock() {
        *cache = None;
    }
}
pub fn start_preview(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewRect,
) -> Result<(), String> {
    start_preview_impl(app, state, source, rect, false)
}

fn start_preview_impl(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewRect,
    force_client: bool,
) -> Result<(), String> {
    if rect.width < 32 || rect.height < 32 {
        return Err("The native preview surface has no usable size.".into());
    }
    stop_host(state.inner());
    let parent = app
        .get_webview_window("main")
        .ok_or("The main application window is unavailable.")?
        .hwnd()
        .map_err(|e| format!("Could not access the native window: {e}"))?;
    let instance = preview_window_instance()?;
    let origin = preview_screen_origin(parent, &rect)?;
    let host = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            w!("ResubWinnyPreviewHost"),
            w!("ResubWinnyMpvHost"),
            WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            origin.x,
            origin.y,
            rect.width,
            rect.height,
            Some(parent),
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|e| format!("Could not create the native preview surface: {e}"))?;
    unsafe {
        SetWindowPos(
            host,
            Some(HWND_TOP),
            origin.x,
            origin.y,
            rect.width,
            rect.height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    }
    .map_err(|e| format!("Could not position the native preview surface: {e}"))?;
    let process_id = std::process::id();
    let overlay_path =
        std::env::temp_dir().join(format!("resubwinny-mpv-overlay-{process_id}.bgra"));
    let _ = fs::remove_file(&overlay_path);
    let resource_dir = app.path().resource_dir().ok();
    let startup = (|| -> Result<_, String> {
        let library_path = crate::libmpv::discover_library(resource_dir.as_deref())?;
        let source_path = Path::new(&source);
        let start_client = |fallback_reason| {
            crate::libmpv::LibMpvPlayer::start(&library_path, host.0 as isize, source_path).map(
                |player| {
                    (
                        crate::state::NativePlayer::Client(player),
                        Some(fallback_reason),
                    )
                },
            )
        };
        if force_client {
            return start_client("libmpv-render was replaced after a runtime failure.".into());
        }
        match crate::libmpv::render_api_available(&library_path) {
            Ok(true) => match crate::libmpv::LibMpvRenderWorker::start(
                library_path.clone(),
                source_path.to_path_buf(),
                host.0 as isize,
                rect.width,
                rect.height,
            ) {
                Ok(worker) => Ok((crate::state::NativePlayer::Render(worker), None)),
                Err(reason) => start_client(format!("libmpv-render startup failed: {reason}")),
            },
            Ok(false) => start_client(
                "The bundled libmpv runtime does not expose the complete render API.".into(),
            ),
            Err(reason) => start_client(format!("Could not probe libmpv-render: {reason}")),
        }
    })();
    let (player, render_fallback_reason) = match startup {
        Ok(startup) => startup,
        Err(error) => {
            unsafe {
                let _ = DestroyWindow(host);
            }
            return Err(error);
        }
    };
    *state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")? = Some(PlayerHost {
        host: host.0 as isize,
        owner: parent.0 as isize,
        source: Path::new(&source).to_path_buf(),
        player,
        overlay_path,
        render_fallback_reason,
    });
    Ok(())
}

pub fn recover_preview(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewRect,
    time_seconds: Option<f64>,
    paused: bool,
    volume: f64,
) -> Result<(), String> {
    start_preview_impl(app, state.clone(), source, rect, true)?;
    let player_slot = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let player = player_slot
        .as_ref()
        .ok_or("Could not rebuild the native preview player.")?;
    if let Some(seconds) = time_seconds.filter(|value| value.is_finite() && *value >= 0.0) {
        player
            .player
            .command(&["seek", &seconds.to_string(), "absolute"])?;
    }
    player
        .player
        .command(&["set", "volume", &volume.clamp(0.0, 100.0).to_string()])?;
    player
        .player
        .command(&["set", "pause", if paused { "yes" } else { "no" }])
}
pub fn resize_preview(state: State<'_, Arc<AppState>>, rect: PreviewRect) -> Result<(), String> {
    if let Some(player) = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?
        .as_mut()
    {
        let origin = preview_screen_origin(HWND(player.owner as *mut _), &rect)?;
        unsafe {
            SetWindowPos(
                HWND(player.host as *mut _),
                Some(HWND_TOP),
                origin.x,
                origin.y,
                rect.width.max(1),
                rect.height.max(1),
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
        }
        .map_err(|e| format!("Could not resize native preview: {e}"))?;
        player.player.resize(rect.width, rect.height);
    }
    Ok(())
}
pub fn stop_preview(state: State<'_, Arc<AppState>>) {
    stop_host(state.inner())
}
pub fn preview_command(state: State<'_, Arc<AppState>>, command: String) -> Result<(), String> {
    let owned;
    let arguments: &[&str] = match command.as_str() {
        "toggle-pause" => &["cycle", "pause"],
        "seek-back" => &["seek", "-5", "relative"],
        "seek-forward" => &["seek", "5", "relative"],
        "frame-back" => &["frame-back-step"],
        "frame-forward" => &["frame-step"],
        value if value.starts_with("seek-absolute:") => {
            let seconds = value["seek-absolute:".len()..]
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .ok_or("Invalid absolute seek position.")?;
            owned = [
                "seek".to_owned(),
                seconds.to_string(),
                "absolute".to_owned(),
            ];
            &[owned[0].as_str(), owned[1].as_str(), owned[2].as_str()]
        }
        value if value.starts_with("set-volume:") => {
            let volume = value["set-volume:".len()..]
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
                .ok_or("Invalid preview volume.")?;
            owned = ["set".to_owned(), "volume".to_owned(), volume.to_string()];
            &[owned[0].as_str(), owned[1].as_str(), owned[2].as_str()]
        }
        _ => return Err("Unsupported native preview command.".into()),
    };
    state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?
        .as_ref()
        .ok_or("Start native preview before using player controls.")?
        .player
        .command(arguments)
}
pub fn caption_overlay(
    state: State<'_, Arc<AppState>>,
    pixels: Vec<u8>,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
) -> Result<(), String> {
    if width <= 0 || height <= 0 || pixels.len() != width as usize * height as usize * 4 {
        return Err("Caption overlay pixel dimensions are invalid.".into());
    }
    let player_slot = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let player = player_slot
        .as_ref()
        .ok_or("Start native preview before showing captions.")?;
    if player.player.is_render() {
        return player
            .player
            .set_caption_overlay(pixels, width, height, x, y);
    }
    let (pixels, width, height, x, y) = match player.player.osd_dimensions()? {
        Some((target_width, target_height))
            if target_width > 0
                && target_height > 0
                && (target_width != width || target_height != height) =>
        {
            // The archive renderer produces a complete caption plane.
            // Fit that plane inside mpv's OSD space and center it; filling
            // both axes independently distorts glyphs and ruby placement.
            let scale =
                (target_width as f64 / width as f64).min(target_height as f64 / height as f64);
            let scaled_width = ((width as f64 * scale).round() as i32).clamp(1, target_width);
            let scaled_height = ((height as f64 * scale).round() as i32).clamp(1, target_height);
            let offset_x = x.saturating_add((target_width - scaled_width) / 2);
            let offset_y = y.saturating_add((target_height - scaled_height) / 2);
            (
                scale_bgra_nearest(&pixels, width, height, scaled_width, scaled_height),
                scaled_width,
                scaled_height,
                offset_x,
                offset_y,
            )
        }
        _ => (pixels, width, height, x, y),
    };
    fs::write(&player.overlay_path, pixels)
        .map_err(|e| format!("Could not prepare caption overlay pixels: {e}"))?;
    let path = player.overlay_path.to_string_lossy().into_owned();
    player.player.command(&[
        "overlay-add",
        "1",
        &x.to_string(),
        &y.to_string(),
        &path,
        "0",
        "bgra",
        &width.to_string(),
        &height.to_string(),
        &width.saturating_mul(4).to_string(),
    ])
}

fn scale_bgra_nearest(
    source: &[u8],
    source_width: i32,
    source_height: i32,
    target_width: i32,
    target_height: i32,
) -> Vec<u8> {
    let mut target = vec![0; target_width as usize * target_height as usize * 4];
    for target_y in 0..target_height as usize {
        let source_y = target_y * source_height as usize / target_height as usize;
        for target_x in 0..target_width as usize {
            let source_x = target_x * source_width as usize / target_width as usize;
            let source_offset = (source_y * source_width as usize + source_x) * 4;
            let target_offset = (target_y * target_width as usize + target_x) * 4;
            target[target_offset..target_offset + 4]
                .copy_from_slice(&source[source_offset..source_offset + 4]);
        }
    }
    target
}
pub fn clear_caption_overlay(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let player_slot = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let player = player_slot
        .as_ref()
        .ok_or("Start native preview before clearing captions.")?;
    if player.player.is_render() {
        player.player.clear_caption_overlay()
    } else {
        player.player.command(&["overlay-remove", "1"])
    }
}
pub fn preview_time(state: State<'_, Arc<AppState>>) -> Result<Option<f64>, String> {
    let slot = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let player = slot
        .as_ref()
        .ok_or("Start native preview before querying time.")?;
    player.player.time_seconds()
}
pub fn preview_duration(state: State<'_, Arc<AppState>>) -> Result<Option<f64>, String> {
    let slot = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let player = slot
        .as_ref()
        .ok_or("Start native preview before querying duration.")?;
    player.player.duration_seconds()
}

pub fn preview_playback_state(
    state: State<'_, Arc<AppState>>,
) -> Result<PreviewPlaybackState, String> {
    let slot = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let player = slot
        .as_ref()
        .ok_or("Start native preview before querying playback state.")?;
    Ok(PreviewPlaybackState {
        time_seconds: player.player.time_seconds()?,
        duration_seconds: player.player.duration_seconds()?,
        paused: player.player.paused()?,
    })
}

pub fn preview_broadcast_metadata(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    service_id: Option<u16>,
) -> Result<BroadcastMetadata, String> {
    const OFFSET_BUCKET_BYTES: u64 = 8 * 1024 * 1024;
    const SI_LOOKBEHIND_BYTES: u64 = 2 * 1024 * 1024;
    let (source, playback_time, duration, fallback_offset) = {
        let slot = state
            .player
            .lock()
            .map_err(|_| "Preview state is unavailable")?;
        let player = slot
            .as_ref()
            .ok_or("Start native preview before querying broadcast metadata.")?;
        (
            player.source.clone(),
            player.player.time_seconds()?,
            player.player.duration_seconds()?,
            player.player.stream_position()?,
        )
    };
    let file_size = fs::metadata(&source)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let source_offset = playback_file_offset(file_size, playback_time, duration, fallback_offset)
        .saturating_sub(SI_LOOKBEHIND_BYTES);
    let offset_bucket = source_offset / OFFSET_BUCKET_BYTES;
    if let Some(cached) = state
        .preview_broadcast_cache
        .lock()
        .map_err(|_| "Preview broadcast metadata cache is unavailable.")?
        .as_ref()
        .filter(|cached| {
            cached.source == source
                && cached.offset_bucket == offset_bucket
                && cached.service_id == service_id
        })
        .cloned()
    {
        return Ok(cached.metadata);
    }

    let mut command = Command::new(worker_path(Some(&app))?);
    command
        .arg("broadcast-at")
        .arg(&source)
        .arg(source_offset.to_string());
    if let Some(service_id) = service_id {
        command.arg("--service-id").arg(service_id.to_string());
    }
    let output = command
        .output()
        .map_err(|error| format!("Could not start broadcast metadata query: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let metadata: BroadcastMetadata = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|event| {
            event.get("type").and_then(serde_json::Value::as_str) == Some("broadcast-metadata")
        })
        .and_then(|event| event.get("broadcast").cloned())
        .ok_or_else(|| "Worker did not return broadcast metadata.".to_owned())
        .and_then(|metadata| {
            serde_json::from_value(metadata)
                .map_err(|error| format!("Worker returned invalid broadcast metadata: {error}"))
        })?;
    *state
        .preview_broadcast_cache
        .lock()
        .map_err(|_| "Preview broadcast metadata cache is unavailable.")? =
        Some(PreviewBroadcastCache {
            source,
            offset_bucket,
            service_id,
            metadata: metadata.clone(),
        });
    Ok(metadata)
}
