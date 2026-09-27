use std::{
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
    time::Duration,
};

use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_SHARED_BUFFER_ACCESS, COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_ONLY,
    COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_WRITE, ICoreWebView2_17, ICoreWebView2Environment12,
    ICoreWebView2SharedBuffer,
};
use windows::core::{Interface, PCWSTR};

pub const MAX_FRAME_WIDTH: i32 = 1_920;
pub const MAX_FRAME_HEIGHT: i32 = 1_080;
const PIXEL_BYTES: usize = MAX_FRAME_WIDTH as usize * MAX_FRAME_HEIGHT as usize * 4;
const SLOT_COUNT: usize = 3;
const CONTROL_BYTES: usize = 4_096;
const MAGIC: u32 = 0x5257_4652;
const VERSION: u32 = 1;

const HEADER_MAGIC: usize = 0;
const HEADER_VERSION: usize = 1;
const HEADER_GENERATION: usize = 2;
const HEADER_RUNNING: usize = 3;
const HEADER_LATEST_SEQUENCE: usize = 4;
const HEADER_DROPPED: usize = 5;
const SLOT_WORDS: usize = 5;
const SLOT_BASE: usize = 16;
const SLOT_STATE: usize = 0;
const SLOT_SEQUENCE: usize = 1;
const SLOT_WIDTH: usize = 2;
const SLOT_HEIGHT: usize = 3;
const SLOT_STRIDE: usize = 4;
const SLOT_FREE: u32 = 0;
const SLOT_WRITING: u32 = 1;
const SLOT_READY: u32 = 2;

struct SharedBuffer {
    buffer: ICoreWebView2SharedBuffer,
    pointer: NonNull<u8>,
    size: usize,
}

// SAFETY: WebView2 COM methods are only called inside with_webview callbacks
// on the UI STA. Other threads access only the stable mapped pointer while the
// COM object is retained. Slot atomics prevent concurrent pixel access.
unsafe impl Send for SharedBuffer {}
// SAFETY: see the ownership and synchronization invariant above.
unsafe impl Sync for SharedBuffer {}

struct UiCloseBuffers(Vec<ICoreWebView2SharedBuffer>);

// SAFETY: the wrapper is moved directly into a with_webview callback. The COM
// objects are never called before that callback reaches their owning UI STA.
unsafe impl Send for UiCloseBuffers {}

impl UiCloseBuffers {
    fn close(self) {
        for buffer in self.0 {
            // SAFETY: caller runs this method on the WebView UI STA.
            let _ = unsafe { buffer.Close() };
        }
    }
}

impl SharedBuffer {
    fn atomic(&self, word: usize) -> &AtomicU32 {
        assert!((word + 1) * size_of::<u32>() <= self.size);
        // SAFETY: WebView2 returns page-aligned memory and word is in bounds.
        unsafe { &*self.pointer.as_ptr().cast::<AtomicU32>().add(word) }
    }
}

pub struct PreviewFramePool {
    control: SharedBuffer,
    pixels: [SharedBuffer; SLOT_COUNT],
    next_sequence: AtomicU32,
}

impl PreviewFramePool {
    pub fn create(app: &AppHandle) -> Result<Arc<Self>, String> {
        static NEXT_GENERATION: AtomicU32 = AtomicU32::new(1);
        let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed).max(1);
        let webview = app
            .get_webview_window("main")
            .ok_or("The main WebView is unavailable for video frames.")?;
        let (sender, receiver) = mpsc::sync_channel(1);
        webview
            .with_webview(move |platform| {
                // SAFETY: Tauri runs this callback on the WebView UI STA.
                let result = unsafe { create_pool_on_ui_thread(platform, generation) };
                let _ = sender.send(result.map(Arc::new));
            })
            .map_err(|error| format!("Could not schedule the WebView frame bridge: {error}"))?;
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Timed out while creating WebView shared video buffers.".to_string())?
    }

    pub fn write_rgba(&self, pixels: &[u8], width: i32, height: i32) -> Result<bool, String> {
        let bytes = usize::try_from(width.max(0))
            .ok()
            .and_then(|width| {
                usize::try_from(height.max(0))
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|count| count.checked_mul(4))
            .ok_or("Preview frame dimensions overflowed.")?;
        if width <= 0
            || height <= 0
            || width > MAX_FRAME_WIDTH
            || height > MAX_FRAME_HEIGHT
            || pixels.len() != bytes
        {
            return Err("Preview frame dimensions exceed the shared surface.".into());
        }
        let Some(slot) = (0..SLOT_COUNT).find(|slot| {
            self.slot_atomic(*slot, SLOT_STATE)
                .compare_exchange(SLOT_FREE, SLOT_WRITING, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        }) else {
            self.control
                .atomic(HEADER_DROPPED)
                .fetch_add(1, Ordering::Relaxed);
            return Ok(false);
        };
        // SAFETY: the producer exclusively owns a WRITING slot and bytes is bounded.
        unsafe {
            std::ptr::copy_nonoverlapping(
                pixels.as_ptr(),
                self.pixels[slot].pointer.as_ptr(),
                bytes,
            )
        };
        let sequence = self
            .next_sequence
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        self.slot_atomic(slot, SLOT_SEQUENCE)
            .store(sequence, Ordering::Relaxed);
        self.slot_atomic(slot, SLOT_WIDTH)
            .store(width as u32, Ordering::Relaxed);
        self.slot_atomic(slot, SLOT_HEIGHT)
            .store(height as u32, Ordering::Relaxed);
        self.slot_atomic(slot, SLOT_STRIDE)
            .store((width * 4) as u32, Ordering::Relaxed);
        self.slot_atomic(slot, SLOT_STATE)
            .store(SLOT_READY, Ordering::Release);
        self.control
            .atomic(HEADER_LATEST_SEQUENCE)
            .store(sequence, Ordering::Release);
        Ok(true)
    }

    pub fn stop(self: Arc<Self>, app: &AppHandle) {
        self.control
            .atomic(HEADER_RUNNING)
            .store(0, Ordering::Release);
        let buffers = match Arc::try_unwrap(self) {
            Ok(pool) => {
                let mut buffers = Vec::with_capacity(SLOT_COUNT + 1);
                buffers.push(pool.control.buffer);
                buffers.extend(pool.pixels.into_iter().map(|slot| slot.buffer));
                UiCloseBuffers(buffers)
            }
            Err(pool) => UiCloseBuffers(
                std::iter::once(pool.control.buffer.clone())
                    .chain(pool.pixels.iter().map(|slot| slot.buffer.clone()))
                    .collect::<Vec<_>>(),
            ),
        };
        if let Some(webview) = app.get_webview_window("main") {
            let _ = webview.with_webview(move |_| {
                buffers.close();
            });
        }
    }

    pub fn dropped_frames(&self) -> u32 {
        self.control.atomic(HEADER_DROPPED).load(Ordering::Relaxed)
    }

    fn slot_atomic(&self, slot: usize, field: usize) -> &AtomicU32 {
        self.control.atomic(SLOT_BASE + slot * SLOT_WORDS + field)
    }
}

unsafe fn create_pool_on_ui_thread(
    platform: tauri::webview::PlatformWebview,
    generation: u32,
) -> Result<PreviewFramePool, String> {
    let environment: ICoreWebView2Environment12 = platform
        .environment()
        .cast()
        .map_err(|error| format!("WebView2 SharedBuffer is unavailable: {error}"))?;
    // SAFETY: this function runs on the UI STA and the controller is live.
    let core = unsafe { platform.controller().CoreWebView2() }
        .map_err(|error| format!("Could not access the WebView2 core: {error}"))?;
    let core: ICoreWebView2_17 = core
        .cast()
        .map_err(|error| format!("WebView2 frame sharing is unavailable: {error}"))?;

    // SAFETY: allocation and publication stay on the UI STA.
    let control = unsafe { create_buffer(&environment, CONTROL_BYTES) }?;
    // SAFETY: the mapped allocation is CONTROL_BYTES long and writable.
    unsafe { std::ptr::write_bytes(control.pointer.as_ptr(), 0, CONTROL_BYTES) };
    control.atomic(HEADER_MAGIC).store(MAGIC, Ordering::Relaxed);
    control
        .atomic(HEADER_VERSION)
        .store(VERSION, Ordering::Relaxed);
    control
        .atomic(HEADER_GENERATION)
        .store(generation, Ordering::Relaxed);
    control.atomic(HEADER_RUNNING).store(1, Ordering::Release);

    let pixels = [
        unsafe { create_buffer(&environment, PIXEL_BYTES) }?,
        unsafe { create_buffer(&environment, PIXEL_BYTES) }?,
        unsafe { create_buffer(&environment, PIXEL_BYTES) }?,
    ];
    unsafe {
        post_buffer(
            &core,
            &control.buffer,
            COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_WRITE,
            &format!("{{\"kind\":\"control\",\"generation\":{generation},\"slots\":{SLOT_COUNT}}}"),
        )?;
        for (slot, buffer) in pixels.iter().enumerate() {
            post_buffer(
                &core,
                &buffer.buffer,
                COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_ONLY,
                &format!("{{\"kind\":\"pixels\",\"generation\":{generation},\"slot\":{slot}}}"),
            )?;
        }
    }
    Ok(PreviewFramePool {
        control,
        pixels,
        next_sequence: AtomicU32::new(0),
    })
}

unsafe fn create_buffer(
    environment: &ICoreWebView2Environment12,
    size: usize,
) -> Result<SharedBuffer, String> {
    // SAFETY: called on the UI STA with a bounded allocation size.
    let buffer = unsafe { environment.CreateSharedBuffer(size as u64) }
        .map_err(|error| format!("Could not create a WebView shared buffer: {error}"))?;
    let mut pointer = std::ptr::null_mut();
    // SAFETY: pointer is a live out parameter and buffer is retained.
    unsafe { buffer.Buffer(&mut pointer) }
        .map_err(|error| format!("Could not map a WebView shared buffer: {error}"))?;
    let pointer =
        NonNull::new(pointer).ok_or("WebView2 returned an empty shared buffer mapping.")?;
    Ok(SharedBuffer {
        buffer,
        pointer,
        size,
    })
}

unsafe fn post_buffer(
    core: &ICoreWebView2_17,
    buffer: &ICoreWebView2SharedBuffer,
    access: COREWEBVIEW2_SHARED_BUFFER_ACCESS,
    metadata: &str,
) -> Result<(), String> {
    let wide = metadata.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    // SAFETY: called on the UI STA and wide is NUL-terminated.
    unsafe { core.PostSharedBufferToScript(buffer, access, PCWSTR(wide.as_ptr())) }
        .map_err(|error| format!("Could not publish a WebView shared buffer: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{MAX_FRAME_HEIGHT, MAX_FRAME_WIDTH, PIXEL_BYTES, SLOT_COUNT};

    #[test]
    fn shared_pool_is_bounded_to_three_full_hd_frames() {
        assert_eq!(SLOT_COUNT, 3);
        assert_eq!(PIXEL_BYTES, 1_920 * 1_080 * 4);
        assert_eq!(MAX_FRAME_WIDTH, 1_920);
        assert_eq!(MAX_FRAME_HEIGHT, 1_080);
    }
}
