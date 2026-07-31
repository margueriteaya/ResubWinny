//! Minimal, project-owned dynamic libmpv client ABI.
//!
//! This deliberately owns only playback control. Caption composition remains in
//! the ResubWinny renderer rather than being delegated to mpv's subtitle stack.

use libloading::Library;
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[repr(C)]
pub struct MpvHandle {
    _private: [u8; 0],
}

type Create = unsafe extern "C" fn() -> *mut MpvHandle;
type Initialize = unsafe extern "C" fn(*mut MpvHandle) -> c_int;
type TerminateDestroy = unsafe extern "C" fn(*mut MpvHandle);
type SetOptionString = unsafe extern "C" fn(*mut MpvHandle, *const c_char, *const c_char) -> c_int;
type Command = unsafe extern "C" fn(*mut MpvHandle, *const *const c_char) -> c_int;
type GetProperty = unsafe extern "C" fn(*mut MpvHandle, *const c_char, c_int, *mut c_void) -> c_int;
type GetPropertyString = unsafe extern "C" fn(*mut MpvHandle, *const c_char) -> *mut c_char;
type Free = unsafe extern "C" fn(*mut c_void);
type RenderContextCreate =
    unsafe extern "C" fn(*mut *mut c_void, *mut MpvHandle, *const c_void) -> c_int;
type RenderContextFree = unsafe extern "C" fn(*mut c_void);
type RenderContextRender = unsafe extern "C" fn(*mut c_void, *const c_void) -> c_int;
type RenderContextUpdate = unsafe extern "C" fn(*mut c_void) -> u64;
type RenderContextReportSwap = unsafe extern "C" fn(*mut c_void);

const MPV_FORMAT_DOUBLE: c_int = 5;
const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;
const MPV_RENDER_UPDATE_FRAME: u64 = 1;

#[repr(C)]
struct MpvRenderParam {
    kind: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct MpvOpenGlInitParams {
    get_proc_address: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
struct MpvOpenGlFbo {
    fbo: c_int,
    width: c_int,
    height: c_int,
    internal_format: c_int,
}

pub struct LibMpv {
    _library: Library,
    create: Create,
    initialize: Initialize,
    terminate_destroy: TerminateDestroy,
    set_option_string: SetOptionString,
    command: Command,
    get_property: GetProperty,
    get_property_string: GetPropertyString,
    free: Free,
    render_context_create: Option<RenderContextCreate>,
    render_context_free: Option<RenderContextFree>,
    render_context_render: Option<RenderContextRender>,
    render_context_update: Option<RenderContextUpdate>,
    render_context_report_swap: Option<RenderContextReportSwap>,
}

pub struct LibMpvPlayer {
    api: LibMpv,
    handle: *mut MpvHandle,
    render_context: Option<*mut c_void>,
}

// Access is serialized by AppState's player mutex. libmpv permits client API
// calls from application threads once its instance has been initialized.
unsafe impl Send for LibMpvPlayer {}

impl LibMpv {
    pub fn load(path: &Path) -> Result<Self, String> {
        let library = unsafe { Library::new(path) }
            .map_err(|error| format!("Could not load libmpv at {}: {error}", path.display()))?;
        unsafe {
            Ok(Self {
                create: *library.get(b"mpv_create\0").map_err(symbol_error)?,
                initialize: *library.get(b"mpv_initialize\0").map_err(symbol_error)?,
                terminate_destroy: *library
                    .get(b"mpv_terminate_destroy\0")
                    .map_err(symbol_error)?,
                set_option_string: *library
                    .get(b"mpv_set_option_string\0")
                    .map_err(symbol_error)?,
                command: *library.get(b"mpv_command\0").map_err(symbol_error)?,
                get_property: *library.get(b"mpv_get_property\0").map_err(symbol_error)?,
                get_property_string: *library
                    .get(b"mpv_get_property_string\0")
                    .map_err(symbol_error)?,
                free: *library.get(b"mpv_free\0").map_err(symbol_error)?,
                // The client API remains usable on a runtime that does not
                // expose libmpv's render API. The native render-surface path
                // is feature-detected separately instead of making playback
                // fail just because that optional route is unavailable.
                render_context_create: library
                    .get::<RenderContextCreate>(b"mpv_render_context_create\0")
                    .ok()
                    .map(|symbol| *symbol),
                render_context_free: library
                    .get::<RenderContextFree>(b"mpv_render_context_free\0")
                    .ok()
                    .map(|symbol| *symbol),
                render_context_render: library
                    .get::<RenderContextRender>(b"mpv_render_context_render\0")
                    .ok()
                    .map(|symbol| *symbol),
                render_context_update: library
                    .get::<RenderContextUpdate>(b"mpv_render_context_update\0")
                    .ok()
                    .map(|symbol| *symbol),
                render_context_report_swap: library
                    .get::<RenderContextReportSwap>(b"mpv_render_context_report_swap\0")
                    .ok()
                    .map(|symbol| *symbol),
                _library: library,
            })
        }
    }

    pub fn supports_render_api(&self) -> bool {
        self.render_context_create.is_some()
            && self.render_context_free.is_some()
            && self.render_context_render.is_some()
            && self.render_context_update.is_some()
            && self.render_context_report_swap.is_some()
    }
}

pub fn render_api_available(path: &Path) -> Result<bool, String> {
    Ok(LibMpv::load(path)?.supports_render_api())
}

impl LibMpvPlayer {
    pub fn start(library_path: &Path, window_id: isize, source: &Path) -> Result<Self, String> {
        let api = LibMpv::load(library_path)?;
        let handle = unsafe { (api.create)() };
        if handle.is_null() {
            return Err("libmpv could not allocate a playback instance.".into());
        }
        let player = Self {
            api,
            handle,
            render_context: None,
        };
        player.set_option("wid", &window_id.to_string())?;
        // `wid` is the compatibility route when the render API or per-source
        // WGL initialization is unavailable.
        player.set_option("vo", "gpu")?;
        player.set_option("hwdec", "auto-safe")?;
        player.set_option("force-window", "yes")?;
        player.set_option("keep-open", "yes")?;
        player.set_option("pause", "yes")?;
        // Captions are composed by the Rust backend from the archive model.
        // Do not let mpv select and paint a second embedded subtitle stream.
        player.set_option("sid", "no")?;
        player.set_option("secondary-sid", "no")?;
        player.set_option("sub-auto", "no")?;
        player.set_option("sub-visibility", "no")?;
        player.set_option("terminal", "no")?;
        if unsafe { (player.api.initialize)(player.handle) } < 0 {
            return Err("libmpv could not initialize the native playback instance.".into());
        }
        player.command(&["loadfile", &source.to_string_lossy(), "replace"])?;
        Ok(player)
    }

    /// Starts the OpenGL render API before loading a media source. The caller
    /// must keep the matching OpenGL context current for this player's entire
    /// render-context lifetime, including `destroy_render_context`.
    pub unsafe fn start_render(
        library_path: &Path,
        source: &Path,
        get_proc_address: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    ) -> Result<Self, String> {
        let api = LibMpv::load(library_path)?;
        if !api.supports_render_api() {
            return Err("This libmpv runtime does not expose the complete render API.".into());
        }
        let handle = unsafe { (api.create)() };
        if handle.is_null() {
            return Err("libmpv could not allocate a render playback instance.".into());
        }
        let mut player = Self {
            api,
            handle,
            render_context: None,
        };
        player.set_option("vo", "libmpv")?;
        player.set_option("gpu-api", "opengl")?;
        // `auto-safe` lets libmpv select a compatible copy-back decoder while
        // retaining a WGL-owned render target. This is deliberately not a
        // promise of zero-copy D3D/ANGLE interop; the actual mode is exposed
        // through backend diagnostics.
        player.set_option("hwdec", "auto-safe")?;
        player.set_option("force-window", "no")?;
        player.set_option("keep-open", "yes")?;
        player.set_option("pause", "yes")?;
        player.set_option("sid", "no")?;
        player.set_option("secondary-sid", "no")?;
        player.set_option("sub-auto", "no")?;
        player.set_option("sub-visibility", "no")?;
        player.set_option("terminal", "no")?;
        if unsafe { (player.api.initialize)(player.handle) } < 0 {
            return Err("libmpv could not initialize the OpenGL playback instance.".into());
        }
        let api_type = CString::new("opengl").expect("literal has no NUL");
        let mut init = MpvOpenGlInitParams {
            get_proc_address,
            get_proc_address_ctx: std::ptr::null_mut(),
        };
        let mut params = [
            MpvRenderParam {
                kind: MPV_RENDER_PARAM_API_TYPE,
                data: api_type.as_ptr().cast_mut().cast(),
            },
            MpvRenderParam {
                kind: MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: (&mut init as *mut MpvOpenGlInitParams).cast(),
            },
            MpvRenderParam {
                kind: 0,
                data: std::ptr::null_mut(),
            },
        ];
        let mut context = std::ptr::null_mut();
        let create = player
            .api
            .render_context_create
            .expect("checked render api");
        let result = unsafe { create(&mut context, player.handle, params.as_mut_ptr().cast()) };
        if result < 0 || context.is_null() {
            return Err(format!(
                "libmpv could not create the OpenGL render context ({result})."
            ));
        }
        player.render_context = Some(context);
        if let Err(error) = player.command(&["loadfile", &source.to_string_lossy(), "replace"]) {
            unsafe { player.destroy_render_context() };
            return Err(error);
        }
        Ok(player)
    }

    /// Draws an mpv frame into the current default OpenGL framebuffer.
    /// `force` redraws the last video frame so a low-frequency caption-plane
    /// change is visible even while video playback is paused.
    pub unsafe fn render_frame(
        &self,
        width: i32,
        height: i32,
        force: bool,
    ) -> Result<bool, String> {
        if width <= 0 || height <= 0 {
            return Ok(false);
        }
        let Some(context) = self.render_context else {
            return Ok(false);
        };
        let update = self.api.render_context_update.expect("checked render api");
        if !force && unsafe { update(context) } & MPV_RENDER_UPDATE_FRAME == 0 {
            return Ok(false);
        }
        let mut fbo = MpvOpenGlFbo {
            fbo: 0,
            width,
            height,
            internal_format: 0,
        };
        let mut flip_y: c_int = 1;
        let mut params = [
            MpvRenderParam {
                kind: MPV_RENDER_PARAM_OPENGL_FBO,
                data: (&mut fbo as *mut MpvOpenGlFbo).cast(),
            },
            MpvRenderParam {
                kind: MPV_RENDER_PARAM_FLIP_Y,
                data: (&mut flip_y as *mut c_int).cast(),
            },
            MpvRenderParam {
                kind: 0,
                data: std::ptr::null_mut(),
            },
        ];
        let render = self.api.render_context_render.expect("checked render api");
        let result = unsafe { render(context, params.as_mut_ptr().cast()) };
        if result < 0 {
            return Err(format!("libmpv OpenGL frame rendering failed ({result})."));
        }
        Ok(true)
    }

    /// Reports a completed platform-buffer swap to libmpv. This must happen
    /// after `SwapBuffers`, not merely after drawing into the WGL back buffer.
    pub unsafe fn report_swap(&self) {
        if let Some(context) = self.render_context {
            unsafe {
                (self
                    .api
                    .render_context_report_swap
                    .expect("checked render api"))(context)
            };
        }
    }

    /// # Safety
    /// The render API's OpenGL context must be current in this thread.
    pub unsafe fn destroy_render_context(&mut self) {
        if let Some(context) = self.render_context.take() {
            unsafe { (self.api.render_context_free.expect("checked render api"))(context) };
        }
    }

    pub fn command(&self, arguments: &[&str]) -> Result<(), String> {
        let values: Result<Vec<_>, _> =
            arguments.iter().map(|value| CString::new(*value)).collect();
        let values = values.map_err(|_| "libmpv command contains an interior NUL.".to_string())?;
        let mut pointers: Vec<*const c_char> = values.iter().map(|value| value.as_ptr()).collect();
        pointers.push(std::ptr::null());
        let result = unsafe { (self.api.command)(self.handle, pointers.as_ptr()) };
        (result >= 0)
            .then_some(())
            .ok_or_else(|| format!("libmpv command failed ({result})."))
    }

    pub fn time_seconds(&self) -> Result<Option<f64>, String> {
        self.double_property("time-pos")
            .map(|value| value.filter(|value| *value >= 0.0))
    }

    pub fn duration_seconds(&self) -> Result<Option<f64>, String> {
        self.double_property("duration")
            .map(|value| value.filter(|value| *value >= 0.0))
    }

    pub fn paused(&self) -> Result<Option<bool>, String> {
        Ok(self
            .string_property("pause")
            .and_then(|value| match value.as_str() {
                "yes" => Some(true),
                "no" => Some(false),
                _ => None,
            }))
    }

    pub fn stream_position(&self) -> Result<Option<f64>, String> {
        self.double_property("stream-pos")
            .map(|value| value.filter(|value| *value >= 0.0))
    }

    pub fn video_aspect(&self) -> Result<Option<f64>, String> {
        // `video-out-params/aspect` follows the final display aspect after
        // sample-aspect ratio handling, which is the coordinate space captions
        // need when a native preview window has letterbox bars.
        self.double_property("video-out-params/aspect")
            .map(|value| value.filter(|value| *value > 0.0))
    }

    pub fn osd_dimensions(&self) -> Result<Option<(i32, i32)>, String> {
        let width = self.double_property("osd-width")?;
        let height = self.double_property("osd-height")?;
        let Some((width, height)) = width.zip(height) else {
            return Ok(None);
        };
        let width = width.round() as i32;
        let height = height.round() as i32;
        Ok((width > 0 && height > 0).then_some((width, height)))
    }

    /// Returns libmpv's actual decoder selection, not the policy requested by
    /// ResubWinny. `None` means this runtime/source cannot report it yet.
    pub fn hwdec_current(&self) -> Option<String> {
        self.string_property("hwdec-current")
    }

    fn string_property(&self, property: &str) -> Option<String> {
        let name = CString::new(property).ok()?;
        let value = unsafe { (self.api.get_property_string)(self.handle, name.as_ptr()) };
        if value.is_null() {
            return None;
        }
        let text = unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned();
        unsafe { (self.api.free)(value.cast()) };
        (!text.trim().is_empty()).then_some(text)
    }

    fn double_property(&self, property: &str) -> Result<Option<f64>, String> {
        let name = CString::new(property)
            .map_err(|_| "libmpv property contains an interior NUL.".to_string())?;
        let mut value: f64 = 0.0;
        let result = unsafe {
            (self.api.get_property)(
                self.handle,
                name.as_ptr(),
                MPV_FORMAT_DOUBLE,
                (&mut value as *mut f64).cast(),
            )
        };
        if result < 0 || !value.is_finite() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        let name = CString::new(name).map_err(|_| "Invalid libmpv option name.".to_string())?;
        let value = CString::new(value).map_err(|_| "Invalid libmpv option value.".to_string())?;
        let result =
            unsafe { (self.api.set_option_string)(self.handle, name.as_ptr(), value.as_ptr()) };
        (result >= 0)
            .then_some(())
            .ok_or_else(|| format!("libmpv option failed ({result})."))
    }
}

#[cfg(windows)]
mod render_worker;
#[cfg(windows)]
pub use render_worker::{LibMpvRenderWorker, RenderWorkerStats};

impl Drop for LibMpvPlayer {
    fn drop(&mut self) {
        // Platform hosts must destroy a render context while its GL context is
        // current. Keep this fallback for error paths that never completed setup.
        if self.render_context.is_some() {
            tracing_fallback_render_context_drop();
        }
        if !self.handle.is_null() {
            unsafe { (self.api.terminate_destroy)(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}

fn tracing_fallback_render_context_drop() {
    // No logging dependency is pulled into the desktop shell. This intentionally
    // does not free the render context without its required current GL context.
}

fn symbol_error(error: libloading::Error) -> String {
    format!("libmpv exports are incomplete: {error}")
}

pub fn bundled_library_path(resource_dir: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        resource_dir.join("libmpv-2.dll")
    }
    #[cfg(target_os = "macos")]
    {
        return resource_dir.join("libmpv.dylib");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return resource_dir.join("libmpv.so.2");
    }
}

pub fn discover_library(resource_dir: Option<&Path>) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("RESUBWINNY_LIBMPV") {
        candidates.push(PathBuf::from(path));
    }
    if let Some(directory) = resource_dir {
        candidates.push(bundled_library_path(directory));
    }
    #[cfg(windows)]
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../third_party/libmpv/windows-x86_64/libmpv-2.dll"),
    );
    #[cfg(target_os = "macos")]
    candidates.extend([
        PathBuf::from("/opt/homebrew/lib/libmpv.dylib"),
        PathBuf::from("/usr/local/lib/libmpv.dylib"),
    ]);
    #[cfg(all(unix, not(target_os = "macos")))]
    candidates.extend([
        PathBuf::from("/usr/lib/libmpv.so.2"),
        PathBuf::from("/usr/lib64/libmpv.so.2"),
        PathBuf::from("/usr/local/lib/libmpv.so.2"),
    ]);
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "libmpv runtime was not found. Bundle the platform library or set RESUBWINNY_LIBMPV."
                .into()
        })
}

#[cfg(all(test, windows))]
#[path = "libmpv/tests.rs"]
mod tests;
