//! Minimal project-owned WGL surface primitives for the future libmpv render
//! backend. They deliberately do not expose a WebView texture or video frame.

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
#[cfg(test)]
const GL_RGBA_FORMAT: u32 = 0x1908;
#[cfg(test)]
const GL_FRONT: u32 = 0x0404;
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
    fn glDisable(capability: u32);
    fn glEnable(capability: u32);
    fn glEnd();
    fn glGenTextures(count: i32, textures: *mut u32);
    fn glLoadIdentity();
    fn glMatrixMode(mode: u32);
    fn glOrtho(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64);
    fn glPopAttrib();
    fn glPopMatrix();
    #[cfg(test)]
    fn glReadBuffer(mode: u32);
    #[cfg(test)]
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
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn ChoosePixelFormat(hdc: *mut c_void, descriptor: *const PixelFormatDescriptor) -> i32;
    fn SetPixelFormat(
        hdc: *mut c_void,
        format: i32,
        descriptor: *const PixelFormatDescriptor,
    ) -> i32;
    fn SwapBuffers(hdc: *mut c_void) -> i32;
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
    /// `hwnd` must be a live child window owned by the current process.
    pub(crate) unsafe fn create(hwnd: isize) -> Result<Self, String> {
        let hwnd = hwnd as *mut c_void;
        let hdc = unsafe { GetDC(hwnd) };
        if hdc.is_null() {
            return Err("Could not acquire a device context for libmpv rendering.".into());
        }
        let descriptor = PixelFormatDescriptor::rgba_double_buffered();
        let format = unsafe { ChoosePixelFormat(hdc, &descriptor) };
        if format == 0 || unsafe { SetPixelFormat(hdc, format, &descriptor) } == 0 {
            unsafe { ReleaseDC(hwnd, hdc) };
            return Err("Could not configure an OpenGL pixel format for libmpv rendering.".into());
        }
        let context = unsafe { wglCreateContext(hdc) };
        if context.is_null() {
            unsafe { ReleaseDC(hwnd, hdc) };
            return Err("Could not create an OpenGL context for libmpv rendering.".into());
        }
        let result = Self { hwnd, hdc, context };
        result.make_current()?;
        Ok(result)
    }

    pub(crate) fn make_current(&self) -> Result<(), String> {
        (unsafe { wglMakeCurrent(self.hdc, self.context) } != 0)
            .then_some(())
            .ok_or_else(|| "Could not make the libmpv OpenGL context current.".into())
    }

    pub(crate) fn swap_buffers(&self) -> Result<(), String> {
        (unsafe { SwapBuffers(self.hdc) } != 0)
            .then_some(())
            .ok_or_else(|| "Could not present the libmpv OpenGL frame.".into())
    }

    /// Captures the current back buffer on the owning WGL thread. This is for
    /// native render validation only; video frames never cross into WebView.
    #[cfg(test)]
    pub(crate) fn read_front_rgba(&self, width: i32, height: i32) -> Result<Vec<u8>, String> {
        let width = usize::try_from(width.max(1)).map_err(|_| "Invalid capture width.")?;
        let height = usize::try_from(height.max(1)).map_err(|_| "Invalid capture height.")?;
        let bytes = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= 128 * 1024 * 1024)
            .ok_or_else(|| "Native render capture exceeds its bounded size.".to_string())?;
        let mut pixels = vec![0; bytes];
        unsafe {
            glReadBuffer(GL_FRONT);
            glReadPixels(
                0,
                0,
                width as i32,
                height as i32,
                GL_RGBA_FORMAT,
                GL_UNSIGNED_BYTE,
                pixels.as_mut_ptr().cast(),
            );
        }
        Ok(pixels)
    }
}

pub(crate) unsafe extern "C" fn get_proc_address(
    _: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    if name.is_null() {
        return ptr::null_mut();
    }
    let wgl = unsafe { wglGetProcAddress(name) };
    if !wgl.is_null() && !matches!(wgl as isize, 1 | 2 | 3 | -1) {
        return wgl;
    }
    let module_name = CString::new("opengl32.dll").expect("literal has no NUL");
    let module = unsafe { GetModuleHandleA(module_name.as_ptr()) };
    if !module.is_null() {
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
        unsafe { glGenTextures(1, &mut id) };
        if id == 0 {
            return Err("Could not allocate a native caption texture.".into());
        }
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
        // display-relative when the native child surface is resized.
        let left = viewport.x as f32 + self.x as f32 / self.width as f32 * viewport.width as f32;
        let top = viewport.y as f32 + self.y as f32 / self.height as f32 * viewport.height as f32;
        let right = viewport.x as f32
            + (self.x + self.width) as f32 / self.width as f32 * viewport.width as f32;
        let bottom = viewport.y as f32
            + (self.y + self.height) as f32 / self.height as f32 * viewport.height as f32;
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
        unsafe { glDeleteTextures(1, &self.id) };
    }
}

impl Drop for WglContext {
    fn drop(&mut self) {
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
