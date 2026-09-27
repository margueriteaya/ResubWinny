//! Project-owned WGL primitives for libmpv offscreen composition and bounded
//! frame readback into the WebView2 SharedBuffer bridge.

#![cfg(windows)]

use std::{
    ffi::{CString, c_char, c_void},
    ptr,
};

const PFD_DOUBLEBUFFER: u32 = 0x0000_0001;
const PFD_DRAW_TO_WINDOW: u32 = 0x0000_0004;
const PFD_SUPPORT_OPENGL: u32 = 0x0000_0020;
const PFD_TYPE_RGBA: u8 = 0;
const GL_ALL_ATTRIB_BITS: u32 = 0xffff_ffff;
const GL_BLEND: u32 = 0x0be2;
const GL_LINEAR: i32 = 0x2601;
const GL_MODELVIEW: u32 = 0x1700;
const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
const GL_PROJECTION: u32 = 0x1701;
const GL_QUADS: u32 = 0x0007;
const GL_RGBA: i32 = 0x1908;
const GL_RGBA_FORMAT: u32 = 0x1908;
const GL_COLOR_ATTACHMENT0: u32 = 0x8ce0;
const GL_FRAMEBUFFER: u32 = 0x8d40;
const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8cd5;
const GL_PIXEL_PACK_BUFFER: u32 = 0x88eb;
const GL_READ_ONLY: u32 = 0x88b8;
const GL_STREAM_READ: u32 = 0x88e1;
const GL_SRC_ALPHA: u32 = 0x0302;
const GL_TEXTURE_2D: u32 = 0x0de1;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_UNSIGNED_BYTE: u32 = 0x1401;

#[repr(C)]
struct PixelFormatDescriptor {
    size: u16,
    version: u16,
    flags: u32,
    pixel_type: u8,
    color_bits: u8,
    red_bits: u8,
    red_shift: u8,
    green_bits: u8,
    green_shift: u8,
    blue_bits: u8,
    blue_shift: u8,
    alpha_bits: u8,
    alpha_shift: u8,
    accum_bits: u8,
    accum_red_bits: u8,
    accum_green_bits: u8,
    accum_blue_bits: u8,
    accum_alpha_bits: u8,
    depth_bits: u8,
    stencil_bits: u8,
    aux_buffers: u8,
    layer_type: u8,
    reserved: u8,
    layer_mask: u32,
    visible_mask: u32,
    damage_mask: u32,
}

impl PixelFormatDescriptor {
    fn rgba_double_buffered() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u16,
            version: 1,
            flags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            pixel_type: PFD_TYPE_RGBA,
            color_bits: 32,
            red_bits: 0,
            red_shift: 0,
            green_bits: 0,
            green_shift: 0,
            blue_bits: 0,
            blue_shift: 0,
            alpha_bits: 8,
            alpha_shift: 0,
            accum_bits: 0,
            accum_red_bits: 0,
            accum_green_bits: 0,
            accum_blue_bits: 0,
            accum_alpha_bits: 0,
            depth_bits: 0,
            stencil_bits: 0,
            aux_buffers: 0,
            layer_type: 0,
            reserved: 0,
            layer_mask: 0,
            visible_mask: 0,
            damage_mask: 0,
        }
    }
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetDC(hwnd: *mut c_void) -> *mut c_void;
    fn ReleaseDC(hwnd: *mut c_void, hdc: *mut c_void) -> i32;
}

#[link(name = "opengl32")]
unsafe extern "system" {
    fn glBegin(mode: u32);
    fn glBindTexture(target: u32, texture: u32);
    fn glBlendFunc(source: u32, destination: u32);
    fn glColor4f(red: f32, green: f32, blue: f32, alpha: f32);
    fn glDeleteTextures(count: i32, textures: *const u32);
    fn glFinish();
    fn glDisable(capability: u32);
    fn glEnable(capability: u32);
    fn glEnd();
    fn glGenTextures(count: i32, textures: *mut u32);
    fn glLoadIdentity();
    fn glMatrixMode(mode: u32);
    fn glOrtho(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64);
    fn glPopAttrib();
    fn glPopMatrix();
    fn glReadBuffer(mode: u32);
    fn glReadPixels(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: u32,
        kind: u32,
        pixels: *mut c_void,
    );
    fn glPushAttrib(mask: u32);
    fn glPushMatrix();
    fn glTexCoord2f(s: f32, t: f32);
    fn glTexImage2D(
        target: u32,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        kind: u32,
        pixels: *const c_void,
    );
    fn glTexParameteri(target: u32, name: u32, value: i32);
    fn glVertex2f(x: f32, y: f32);
    fn glViewport(x: i32, y: i32, width: i32, height: i32);
}

type GlGenFramebuffers = unsafe extern "system" fn(i32, *mut u32);
type GlBindFramebuffer = unsafe extern "system" fn(u32, u32);
type GlFramebufferTexture2D = unsafe extern "system" fn(u32, u32, u32, u32, i32);
type GlCheckFramebufferStatus = unsafe extern "system" fn(u32) -> u32;
type GlDeleteFramebuffers = unsafe extern "system" fn(i32, *const u32);
type GlGenBuffers = unsafe extern "system" fn(i32, *mut u32);
type GlBindBuffer = unsafe extern "system" fn(u32, u32);
type GlBufferData = unsafe extern "system" fn(u32, isize, *const c_void, u32);
type GlMapBuffer = unsafe extern "system" fn(u32, u32) -> *mut c_void;
type GlUnmapBuffer = unsafe extern "system" fn(u32) -> u8;
type GlDeleteBuffers = unsafe extern "system" fn(i32, *const u32);

struct GlExtensions {
    gen_framebuffers: GlGenFramebuffers,
    bind_framebuffer: GlBindFramebuffer,
    framebuffer_texture_2d: GlFramebufferTexture2D,
    check_framebuffer_status: GlCheckFramebufferStatus,
    delete_framebuffers: GlDeleteFramebuffers,
    gen_buffers: GlGenBuffers,
    bind_buffer: GlBindBuffer,
    buffer_data: GlBufferData,
    map_buffer: GlMapBuffer,
    unmap_buffer: GlUnmapBuffer,
    delete_buffers: GlDeleteBuffers,
}

impl GlExtensions {
    fn load() -> Result<Self, String> {
        unsafe fn symbol(name: &'static [u8]) -> Result<*mut c_void, String> {
            // SAFETY: every caller passes a static NUL-terminated OpenGL name.
            let pointer = unsafe { get_proc_address(ptr::null_mut(), name.as_ptr().cast()) };
            if pointer.is_null() {
                Err(format!(
                    "The OpenGL driver does not expose {}.",
                    String::from_utf8_lossy(&name[..name.len() - 1])
                ))
            } else {
                Ok(pointer)
            }
        }
        // SAFETY: each resolved OpenGL symbol is cast to its documented ABI.
        unsafe {
            Ok(Self {
                gen_framebuffers: std::mem::transmute::<*mut c_void, GlGenFramebuffers>(symbol(
                    b"glGenFramebuffers\0",
                )?),
                bind_framebuffer: std::mem::transmute::<*mut c_void, GlBindFramebuffer>(symbol(
                    b"glBindFramebuffer\0",
                )?),
                framebuffer_texture_2d: std::mem::transmute::<*mut c_void, GlFramebufferTexture2D>(
                    symbol(b"glFramebufferTexture2D\0")?,
                ),
                check_framebuffer_status: std::mem::transmute::<
                    *mut c_void,
                    GlCheckFramebufferStatus,
                >(symbol(b"glCheckFramebufferStatus\0")?),
                delete_framebuffers: std::mem::transmute::<*mut c_void, GlDeleteFramebuffers>(
                    symbol(b"glDeleteFramebuffers\0")?,
                ),
                gen_buffers: std::mem::transmute::<*mut c_void, GlGenBuffers>(symbol(
                    b"glGenBuffers\0",
                )?),
                bind_buffer: std::mem::transmute::<*mut c_void, GlBindBuffer>(symbol(
                    b"glBindBuffer\0",
                )?),
                buffer_data: std::mem::transmute::<*mut c_void, GlBufferData>(symbol(
                    b"glBufferData\0",
                )?),
                map_buffer: std::mem::transmute::<*mut c_void, GlMapBuffer>(symbol(
                    b"glMapBuffer\0",
                )?),
                unmap_buffer: std::mem::transmute::<*mut c_void, GlUnmapBuffer>(symbol(
                    b"glUnmapBuffer\0",
                )?),
                delete_buffers: std::mem::transmute::<*mut c_void, GlDeleteBuffers>(symbol(
                    b"glDeleteBuffers\0",
                )?),
            })
        }
    }
}

/// Owns the texture-backed framebuffer rendered by libmpv and a three-buffer
/// pixel-pack queue. Each submitted frame is mapped immediately so video stays
/// synchronized with libmpv audio; the slots rotate to avoid reallocations.
pub(crate) struct OffscreenReadback {
    gl: GlExtensions,
    framebuffer: u32,
    color_texture: u32,
    pbos: [u32; 3],
    width: i32,
    height: i32,
    cursor: usize,
    submitted: usize,
}

impl OffscreenReadback {
    pub(crate) fn create(width: i32, height: i32) -> Result<Self, String> {
        let gl = GlExtensions::load()?;
        let mut framebuffer = 0;
        let mut color_texture = 0;
        let mut pbos = [0; 3];
        // SAFETY: a WGL context is current and all output arrays have the
        // requested number of entries.
        unsafe {
            (gl.gen_framebuffers)(1, &mut framebuffer);
            glGenTextures(1, &mut color_texture);
            (gl.gen_buffers)(pbos.len() as i32, pbos.as_mut_ptr());
        }
        let mut result = Self {
            gl,
            framebuffer,
            color_texture,
            pbos,
            width: 0,
            height: 0,
            cursor: 0,
            submitted: 0,
        };
        if framebuffer == 0 || color_texture == 0 || pbos.contains(&0) {
            return Err("Could not allocate the offscreen preview surface.".into());
        }
        result.resize(width, height)?;
        Ok(result)
    }

    pub(crate) fn framebuffer(&self) -> i32 {
        self.framebuffer as i32
    }

    pub(crate) fn bind(&mut self, width: i32, height: i32) -> Result<(), String> {
        if self.width != width || self.height != height {
            self.resize(width, height)?;
        }
        // SAFETY: this framebuffer belongs to the current context and has a
        // complete color attachment after resize succeeds.
        unsafe {
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, self.framebuffer);
            glViewport(0, 0, self.width, self.height);
        }
        Ok(())
    }

    pub(crate) fn readback_frame(&mut self) -> Result<Option<Vec<u8>>, String> {
        let bytes = frame_byte_len(self.width, self.height)?;
        let write_slot = self.cursor;
        // SAFETY: the FBO and PBO names belong to this current context. A null
        // pixels pointer directs glReadPixels into the bound pixel-pack buffer.
        unsafe {
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, self.framebuffer);
            glReadBuffer(GL_COLOR_ATTACHMENT0);
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, self.pbos[write_slot]);
            glReadPixels(
                0,
                0,
                self.width,
                self.height,
                GL_RGBA_FORMAT,
                GL_UNSIGNED_BYTE,
                ptr::null_mut(),
            );
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
        }
        self.cursor = (self.cursor + 1) % self.pbos.len();
        self.submitted = self.submitted.saturating_add(1);
        let read_slot = write_slot;
        let mut pixels = vec![0; bytes];
        // SAFETY: mapping the slot synchronizes this read with the preceding
        // glReadPixels. The map covers the allocation made in resize and is
        // unmapped exactly once.
        unsafe {
            // Some Windows OpenGL drivers do not make an immediately mapped
            // pixel-pack transfer visible without an explicit completion
            // point. This keeps the delivered frame aligned with mpv audio.
            glFinish();
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, self.pbos[read_slot]);
            let mapped = (self.gl.map_buffer)(GL_PIXEL_PACK_BUFFER, GL_READ_ONLY);
            if mapped.is_null() {
                (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
                self.unbind();
                return Err("Could not map the completed preview frame.".into());
            }
            ptr::copy_nonoverlapping(mapped.cast::<u8>(), pixels.as_mut_ptr(), bytes);
            if (self.gl.unmap_buffer)(GL_PIXEL_PACK_BUFFER) == 0 {
                (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
                self.unbind();
                return Err("The completed preview frame became invalid while reading it.".into());
            }
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
        }
        self.unbind();
        Ok(Some(pixels))
    }

    #[cfg(test)]
    pub(crate) fn readback_now(&self) -> Result<Vec<u8>, String> {
        let bytes = frame_byte_len(self.width, self.height)?;
        let mut pixels = vec![0; bytes];
        // SAFETY: this FBO is complete and pixels has exactly enough storage.
        unsafe {
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, self.framebuffer);
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
            glReadBuffer(GL_COLOR_ATTACHMENT0);
            glReadPixels(
                0,
                0,
                self.width,
                self.height,
                GL_RGBA_FORMAT,
                GL_UNSIGNED_BYTE,
                pixels.as_mut_ptr().cast(),
            );
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, 0);
        }
        Ok(pixels)
    }

    fn resize(&mut self, width: i32, height: i32) -> Result<(), String> {
        let bytes = frame_byte_len(width, height)?;
        // SAFETY: all names belong to the current context; null texture/PBO
        // data requests bounded storage without reading client memory.
        unsafe {
            glBindTexture(GL_TEXTURE_2D, self.color_texture);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA,
                width,
                height,
                0,
                GL_RGBA_FORMAT,
                GL_UNSIGNED_BYTE,
                ptr::null(),
            );
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, self.framebuffer);
            (self.gl.framebuffer_texture_2d)(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                self.color_texture,
                0,
            );
            if (self.gl.check_framebuffer_status)(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE {
                (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, 0);
                return Err("The OpenGL driver rejected the offscreen preview surface.".into());
            }
            for pbo in self.pbos {
                (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, pbo);
                (self.gl.buffer_data)(
                    GL_PIXEL_PACK_BUFFER,
                    bytes as isize,
                    ptr::null(),
                    GL_STREAM_READ,
                );
            }
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, 0);
        }
        self.width = width;
        self.height = height;
        self.cursor = 0;
        self.submitted = 0;
        Ok(())
    }

    fn unbind(&self) {
        // SAFETY: zero restores the context's default framebuffer.
        unsafe { (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, 0) };
    }
}

impl Drop for OffscreenReadback {
    fn drop(&mut self) {
        // SAFETY: the render worker drops this while its WGL context is current.
        unsafe {
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, 0);
            (self.gl.delete_buffers)(self.pbos.len() as i32, self.pbos.as_ptr());
            (self.gl.delete_framebuffers)(1, &self.framebuffer);
            glDeleteTextures(1, &self.color_texture);
        }
    }
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn ChoosePixelFormat(hdc: *mut c_void, descriptor: *const PixelFormatDescriptor) -> i32;
    fn SetPixelFormat(
        hdc: *mut c_void,
        format: i32,
        descriptor: *const PixelFormatDescriptor,
    ) -> i32;
}

#[link(name = "opengl32")]
unsafe extern "system" {
    fn wglCreateContext(hdc: *mut c_void) -> *mut c_void;
    fn wglDeleteContext(context: *mut c_void) -> i32;
    fn wglMakeCurrent(hdc: *mut c_void, context: *mut c_void) -> i32;
    fn wglGetProcAddress(name: *const c_char) -> *mut c_void;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const c_char) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
}

pub(crate) struct WglContext {
    hwnd: *mut c_void,
    hdc: *mut c_void,
    context: *mut c_void,
}

impl WglContext {
    /// # Safety
    /// `hwnd` must be a live hidden window owned by the current process.
    pub(crate) unsafe fn create(hwnd: isize) -> Result<Self, String> {
        let hwnd = hwnd as *mut c_void;
        // SAFETY: `hwnd` is a live window owned by this process, per the
        // contract above. The DC is released on every error path and in Drop.
        let hdc = unsafe { GetDC(hwnd) };
        if hdc.is_null() {
            return Err("Could not acquire a device context for libmpv rendering.".into());
        }
        let descriptor = PixelFormatDescriptor::rgba_double_buffered();
        // SAFETY: `hdc` is non-null and `descriptor` outlives both calls.
        let format = unsafe { ChoosePixelFormat(hdc, &descriptor) };
        if format == 0 || unsafe { SetPixelFormat(hdc, format, &descriptor) } == 0 {
            // SAFETY: releases the DC acquired above exactly once on this path.
            unsafe { ReleaseDC(hwnd, hdc) };
            return Err("Could not configure an OpenGL pixel format for libmpv rendering.".into());
        }
        // SAFETY: `hdc` has an OpenGL-capable pixel format set above, which is
        // what wglCreateContext requires.
        let context = unsafe { wglCreateContext(hdc) };
        if context.is_null() {
            // SAFETY: releases the DC acquired above exactly once on this path.
            unsafe { ReleaseDC(hwnd, hdc) };
            return Err("Could not create an OpenGL context for libmpv rendering.".into());
        }
        let result = Self { hwnd, hdc, context };
        result.make_current()?;
        Ok(result)
    }

    pub(crate) fn make_current(&self) -> Result<(), String> {
        // SAFETY: both handles are owned by `self` and stay valid until Drop.
        (unsafe { wglMakeCurrent(self.hdc, self.context) } != 0)
            .then_some(())
            .ok_or_else(|| "Could not make the libmpv OpenGL context current.".into())
    }
}

fn frame_byte_len(width: i32, height: i32) -> Result<usize, String> {
    let width = usize::try_from(width.max(1)).map_err(|_| "Invalid capture width.")?;
    let height = usize::try_from(height.max(1)).map_err(|_| "Invalid capture height.")?;
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|bytes| *bytes <= 128 * 1024 * 1024)
        .ok_or_else(|| "Native render capture exceeds its bounded size.".to_string())
}

/// # Safety
///
/// `name` must be a NUL-terminated symbol name. libmpv calls this with a
/// current GL context on the render thread.
pub(crate) unsafe extern "C" fn get_proc_address(
    _: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    if name.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: `name` was checked non-null and is NUL-terminated by
    // contract. wglGetProcAddress may return the documented 1/2/3/-1
    // sentinels, which are rejected below.
    let wgl = unsafe { wglGetProcAddress(name) };
    if !wgl.is_null() && !matches!(wgl as isize, 1 | 2 | 3 | -1) {
        return wgl;
    }
    let module_name = CString::new("opengl32.dll").expect("literal has no NUL");
    // SAFETY: the module name is a NUL-terminated literal. opengl32.dll is
    // already loaded because this process created a WGL context.
    let module = unsafe { GetModuleHandleA(module_name.as_ptr()) };
    if !module.is_null() {
        // SAFETY: `module` was checked non-null and `name` is NUL-terminated
        // by this function's contract.
        unsafe { GetProcAddress(module, name) }
    } else {
        ptr::null_mut()
    }
}

/// A project-owned native caption texture. All calls happen on the libmpv
/// render thread after its WGL context has been made current.
pub(crate) struct CaptionTexture {
    id: u32,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VideoViewport {
    pub surface_width: i32,
    pub surface_height: i32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Fits a display-aspect-correct video plane into the native render surface.
/// This is deliberately independent of caption semantics: caption pixels still
/// come from the Rust B24/B62 renderer, while libmpv supplies only its display
/// aspect ratio.
pub(crate) fn fit_video_viewport(
    output_width: i32,
    output_height: i32,
    video_aspect: Option<f64>,
) -> VideoViewport {
    let full = VideoViewport {
        surface_width: output_width.max(1),
        surface_height: output_height.max(1),
        x: 0,
        y: 0,
        width: output_width.max(1),
        height: output_height.max(1),
    };
    let Some(video_aspect) = video_aspect.filter(|aspect| aspect.is_finite() && *aspect > 0.0)
    else {
        return full;
    };
    let output_aspect = full.width as f64 / full.height as f64;
    if output_aspect > video_aspect {
        let width = (full.height as f64 * video_aspect)
            .round()
            .clamp(1.0, full.width as f64) as i32;
        VideoViewport {
            x: (full.width - width) / 2,
            y: 0,
            width,
            height: full.height,
            ..full
        }
    } else {
        let height = (full.width as f64 / video_aspect)
            .round()
            .clamp(1.0, full.height as f64) as i32;
        VideoViewport {
            x: 0,
            y: (full.height - height) / 2,
            width: full.width,
            height,
            ..full
        }
    }
}

impl CaptionTexture {
    pub(crate) fn upload(
        previous: Option<&mut Self>,
        pixels: &[u8],
        width: i32,
        height: i32,
        x: i32,
        y: i32,
    ) -> Result<Self, String> {
        if width <= 0 || height <= 0 || pixels.len() != width as usize * height as usize * 4 {
            return Err("Caption texture pixels are invalid.".into());
        }
        let mut id = 0;
        // SAFETY: the caller holds the current GL context, and the out pointer
        // is a live u32 for exactly the one name requested.
        unsafe { glGenTextures(1, &mut id) };
        if id == 0 {
            return Err("Could not allocate a native caption texture.".into());
        }
        // SAFETY: `id` is the texture just allocated, and `pixels` was checked
        // above to hold exactly width * height * 4 bytes for this upload.
        unsafe {
            glBindTexture(GL_TEXTURE_2D, id);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA,
                width,
                height,
                0,
                GL_RGBA as u32,
                GL_UNSIGNED_BYTE,
                pixels.as_ptr().cast(),
            );
        }
        if let Some(previous) = previous {
            // SAFETY: the previous texture belongs to this same GL context and is
            // replaced here, so its name is deleted exactly once.
            unsafe { glDeleteTextures(1, &previous.id) };
        }
        Ok(Self {
            id,
            width,
            height,
            x,
            y,
        })
    }

    pub(crate) fn draw(&self, viewport: VideoViewport) -> Result<(), String> {
        // Caption frames currently represent one full broadcast plane. Normalise
        // source coordinates to that plane so 2K/4K/8K logical geometry remains
        // display-relative when the Canvas render surface is resized.
        let left = viewport.x as f32 + self.x as f32 / self.width as f32 * viewport.width as f32;
        let top = viewport.y as f32 + self.y as f32 / self.height as f32 * viewport.height as f32;
        let right = viewport.x as f32
            + (self.x + self.width) as f32 / self.width as f32 * viewport.width as f32;
        let bottom = viewport.y as f32
            + (self.y + self.height) as f32 / self.height as f32 * viewport.height as f32;
        // SAFETY: the caller holds the current GL context. Every push below is
        // matched by a pop before this block returns.
        unsafe {
            glPushAttrib(GL_ALL_ATTRIB_BITS);
            glMatrixMode(GL_PROJECTION);
            glPushMatrix();
            glLoadIdentity();
            glOrtho(
                0.0,
                viewport.surface_width as f64,
                viewport.surface_height as f64,
                0.0,
                -1.0,
                1.0,
            );
            glMatrixMode(GL_MODELVIEW);
            glPushMatrix();
            glLoadIdentity();
            glEnable(GL_TEXTURE_2D);
            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
            glColor4f(1.0, 1.0, 1.0, 1.0);
            glBindTexture(GL_TEXTURE_2D, self.id);
            glBegin(GL_QUADS);
            // The renderer supplies top-down BGRA rows. OpenGL treats the first
            // uploaded row as t=0, while this projection also places the first
            // quad edge at the visual top. Keep those edges paired; reversing
            // t here flips both glyphs and multi-line caption order.
            glTexCoord2f(0.0, 0.0);
            glVertex2f(left, top);
            glTexCoord2f(1.0, 0.0);
            glVertex2f(right, top);
            glTexCoord2f(1.0, 1.0);
            glVertex2f(right, bottom);
            glTexCoord2f(0.0, 1.0);
            glVertex2f(left, bottom);
            glEnd();
            glDisable(GL_BLEND);
            glMatrixMode(GL_MODELVIEW);
            glPopMatrix();
            glMatrixMode(GL_PROJECTION);
            glPopMatrix();
            glPopAttrib();
        }
        Ok(())
    }
}

impl Drop for CaptionTexture {
    fn drop(&mut self) {
        // The render worker drops this texture before it releases WGL.
        // SAFETY: the render worker drops this texture while its GL context is
        // still current, so the name is deleted exactly once and in context.
        unsafe { glDeleteTextures(1, &self.id) };
    }
}

impl Drop for WglContext {
    fn drop(&mut self) {
        // SAFETY: the context is detached before deletion, and both handles
        // are owned by `self`, so each is released exactly once.
        unsafe {
            let _ = wglMakeCurrent(ptr::null_mut(), ptr::null_mut());
            let _ = wglDeleteContext(self.context);
            let _ = ReleaseDC(self.hwnd, self.hdc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{VideoViewport, fit_video_viewport, get_proc_address};
    use std::{ffi::CString, ptr};

    #[test]
    fn resolves_a_system_opengl_entry_point_without_a_webview() {
        let name = CString::new("glClear").expect("literal has no NUL");
        // SAFETY: the name is a NUL-terminated literal. This resolves a core
        // opengl32 entry point, which needs no current context.
        let resolved = unsafe { get_proc_address(ptr::null_mut(), name.as_ptr()) };
        assert!(!resolved.is_null());
    }

    #[test]
    fn fits_sixteen_by_nine_video_without_putting_captions_in_letterboxes() {
        assert_eq!(
            fit_video_viewport(1200, 900, Some(16.0 / 9.0)),
            VideoViewport {
                surface_width: 1200,
                surface_height: 900,
                x: 0,
                y: 112,
                width: 1200,
                height: 675,
            }
        );
    }

    #[test]
    fn fits_four_by_three_video_without_putting_captions_in_pillarboxes() {
        assert_eq!(
            fit_video_viewport(1600, 900, Some(4.0 / 3.0)),
            VideoViewport {
                surface_width: 1600,
                surface_height: 900,
                x: 200,
                y: 0,
                width: 1200,
                height: 900,
            }
        );
    }
}
