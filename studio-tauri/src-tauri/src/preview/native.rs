use super::*;
use crate::{
    models::{BroadcastMetadata, PreviewPlaybackState},
    preview::frame_bridge::{MAX_FRAME_HEIGHT, MAX_FRAME_WIDTH, PreviewFramePool},
    state::{PlayerHost, PreviewBroadcastCache},
    worker::worker_path,
};
use std::process::Command;
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
            RegisterClassW, WNDCLASSW, WS_EX_NOACTIVATE, WS_POPUP,
        },
    },
    core::w,
};

// SAFETY: invoked by the window manager with the arguments it documents
// for a window procedure; every argument is forwarded unchanged.
unsafe extern "system" fn preview_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: forwards the window manager's own arguments unchanged to the
    // default handler.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn preview_window_instance() -> Result<HINSTANCE, String> {
    static INSTANCE: std::sync::OnceLock<Result<isize, String>> = std::sync::OnceLock::new();
    let instance = INSTANCE.get_or_init(|| {
        // SAFETY: passing None asks for this process's own module handle.
        let module = unsafe { GetModuleHandleW(None) }
            .map_err(|error| format!("Could not locate the application module: {error}"))?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(preview_window_proc),
            hInstance: instance,
            lpszClassName: w!("ResubWinnyOffscreenGlHost"),
            // The hidden host only supplies a stable device context for WGL.
            ..Default::default()
        };
        // SAFETY: `class` is fully initialized and its name string outlives
        // the call.
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

fn stop_host(state: &AppState) {
    if let Ok(mut slot) = state.player.lock()
        && let Some(player) = slot.take()
    {
        player.player.stop();
        player.frame_pool.stop(&player.app);
        // SAFETY: the host window is owned by this state and taken out of the
        // slot above, so it is destroyed exactly once.
        unsafe {
            let _ = DestroyWindow(HWND(player.host as *mut _));
        }
    }
    if let Ok(mut cache) = state.preview_broadcast_cache.lock() {
        *cache = None;
    }
}

pub(crate) fn shutdown_preview(state: &AppState) {
    stop_host(state);
}
pub fn start_preview(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewSurfaceSize,
) -> Result<(), String> {
    start_preview_impl(app, state, source, rect)
}

fn start_preview_impl(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewSurfaceSize,
) -> Result<(), String> {
    if rect.width < 32
        || rect.height < 32
        || rect.width > MAX_FRAME_WIDTH
        || rect.height > MAX_FRAME_HEIGHT
    {
        return Err("The preview surface must fit within 1920 by 1080 physical pixels.".into());
    }
    stop_host(state.inner());
    let instance = preview_window_instance()?;
    // This hidden window supplies only the device context required to create
    // WGL. Video pixels are rendered into an offscreen texture and displayed
    // by the Canvas inside the WebView.
    // SAFETY: the class was registered above and the window remains hidden.
    let host = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE,
            w!("ResubWinnyOffscreenGlHost"),
            w!("ResubWinnyOffscreenMpvHost"),
            WS_POPUP,
            -32_000,
            -32_000,
            rect.width,
            rect.height,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|e| format!("Could not create the native preview surface: {e}"))?;
    let frame_pool = match PreviewFramePool::create(&app) {
        Ok(pool) => pool,
        Err(error) => {
            // SAFETY: buffer setup failed after this thread created `host`.
            unsafe {
                let _ = DestroyWindow(host);
            }
            return Err(error);
        }
    };
    let resource_dir = app.path().resource_dir().ok();
    let startup = (|| -> Result<_, String> {
        let library_path = crate::libmpv::discover_library(resource_dir.as_deref())?;
        let source_path = Path::new(&source);
        match crate::libmpv::render_api_available(&library_path) {
            Ok(true) => match crate::libmpv::LibMpvRenderWorker::start(
                library_path.clone(),
                source_path.to_path_buf(),
                host.0 as isize,
                rect.width,
                rect.height,
                Some(frame_pool.clone()),
            ) {
                Ok(worker) => Ok(crate::state::NativePlayer(worker)),
                Err(reason) => Err(format!("libmpv-render startup failed: {reason}")),
            },
            Ok(false) => {
                Err("The bundled libmpv runtime does not expose the complete render API.".into())
            }
            Err(reason) => Err(format!("Could not probe libmpv-render: {reason}")),
        }
    })();
    let player = match startup {
        Ok(startup) => startup,
        Err(error) => {
            frame_pool.stop(&app);
            // SAFETY: startup failed after creating `host`, so this destroys it
            // exactly once before returning the error.
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
        app: app.clone(),
        frame_pool,
        source: Path::new(&source).to_path_buf(),
        player,
    });
    Ok(())
}

pub fn recover_preview(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    rect: PreviewSurfaceSize,
    time_seconds: Option<f64>,
    paused: bool,
    volume: f64,
) -> Result<(), String> {
    start_preview_impl(app, state.clone(), source, rect)?;
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
            .command(&["seek", &seconds.to_string(), "absolute+exact"])?;
    }
    player
        .player
        .command(&["set", "volume", &volume.clamp(0.0, 100.0).to_string()])?;
    player
        .player
        .command(&["set", "pause", if paused { "yes" } else { "no" }])
}
pub fn resize_preview(
    state: State<'_, Arc<AppState>>,
    rect: PreviewSurfaceSize,
) -> Result<(), String> {
    if rect.width < 1
        || rect.height < 1
        || rect.width > MAX_FRAME_WIDTH
        || rect.height > MAX_FRAME_HEIGHT
    {
        return Err("The preview surface must fit within 1920 by 1080 physical pixels.".into());
    }
    if let Some(player) = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?
        .as_mut()
    {
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
                "absolute+exact".to_owned(),
            ];
            &[owned[0].as_str(), owned[1].as_str(), owned[2].as_str()]
        }
        value if value.starts_with("seek-preview:") => {
            let seconds = value["seek-preview:".len()..]
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .ok_or("Invalid preview seek position.")?;
            owned = [
                "seek".to_owned(),
                seconds.to_string(),
                "absolute+keyframes".to_owned(),
            ];
            &[owned[0].as_str(), owned[1].as_str(), owned[2].as_str()]
        }
        value if value.starts_with("set-pause:") => {
            let paused = match &value["set-pause:".len()..] {
                "yes" => "yes",
                "no" => "no",
                _ => return Err("Invalid preview pause state.".into()),
            };
            owned = ["set".to_owned(), "pause".to_owned(), paused.to_owned()];
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
    pixels: Arc<[u8]>,
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
    player
        .player
        .set_caption_overlay(pixels, width, height, x, y)
}
pub fn clear_caption_overlay(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let player_slot = state
        .player
        .lock()
        .map_err(|_| "Preview state is unavailable")?;
    let player = player_slot
        .as_ref()
        .ok_or("Start native preview before clearing captions.")?;
    player.player.clear_caption_overlay()
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

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{GetParent, GetWindowRect, IsWindowVisible},
    };

    #[test]
    fn gl_host_stays_hidden_offscreen_and_outside_the_app_window_tree() {
        // SAFETY: the host is created and destroyed by this test on one thread.
        unsafe {
            let host = CreateWindowExW(
                WS_EX_NOACTIVATE,
                w!("ResubWinnyOffscreenGlHost"),
                w!("ResubWinnyOffscreenMpvHostTest"),
                WS_POPUP,
                -32_000,
                -32_000,
                640,
                360,
                None,
                None,
                Some(preview_window_instance().expect("preview host class")),
                None,
            )
            .expect("create hidden GL host");
            assert!(GetParent(host).is_err(), "the GL host must have no parent");
            assert!(!IsWindowVisible(host).as_bool());
            let mut bounds = RECT::default();
            GetWindowRect(host, &mut bounds).expect("get hidden host bounds");
            assert!(bounds.right <= -31_000 && bounds.bottom <= -31_000);
            DestroyWindow(host).expect("destroy hidden GL host");
        }
    }
}
