use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::Serialize;
use std::{
    ffi::{c_char, c_void},
    ptr::NonNull,
};

#[repr(C)]
#[derive(Default)]
struct CaptionSummary {
    status: i32,
    pts_ms: i64,
    wait_duration_ms: i64,
    plane_width: i32,
    plane_height: i32,
    region_count: u32,
    character_count: u32,
    unresolved_drcs_count: u32,
}

#[repr(C)]
#[derive(Default)]
struct NativeRegion {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    is_ruby: u8,
    first_character: u32,
    character_count: u32,
}

#[repr(C)]
#[derive(Default)]
struct NativeCharacter {
    kind: u32,
    codepoint: u32,
    pua_codepoint: u32,
    drcs_code: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    horizontal_spacing: i32,
    vertical_spacing: i32,
    horizontal_scale: f32,
    vertical_scale: f32,
    text_color: u32,
    back_color: u32,
    stroke_color: u32,
    style: u32,
    enclosure_style: u32,
    utf8: [c_char; 8],
}

#[repr(C)]
struct NativeDrcsGlyph {
    drcs_code: u32,
    width: i32,
    height: i32,
    depth: i32,
    depth_bits: i32,
    alternative_codepoint: u32,
    pixel_count: usize,
    md5: [c_char; 33],
    alternative_text: [c_char; 8],
}

#[repr(C)]
#[derive(Default)]
struct NativeRenderedImage {
    width: i32,
    height: i32,
    stride: i32,
    dst_x: i32,
    dst_y: i32,
    bitmap_size: usize,
}

impl Default for NativeDrcsGlyph {
    fn default() -> Self {
        Self {
            drcs_code: 0,
            width: 0,
            height: 0,
            depth: 0,
            depth_bits: 0,
            alternative_codepoint: 0,
            pixel_count: 0,
            md5: [0; 33],
            alternative_text: [0; 8],
        }
    }
}

unsafe extern "C" {
    fn acb_decoder_create() -> *mut c_void;
    fn acb_decoder_destroy(decoder: *mut c_void);
    fn acb_decoder_feed(
        decoder: *mut c_void,
        data: *const u8,
        size: usize,
        pts_ms: i64,
        summary: *mut CaptionSummary,
        event: *mut *mut c_void,
    ) -> i32;
    fn acb_decoder_get_rendered_image(
        decoder: *const c_void,
        image: *mut NativeRenderedImage,
    ) -> i32;
    fn acb_decoder_copy_rendered_rgba(
        decoder: *const c_void,
        destination: *mut u8,
        capacity: usize,
    ) -> usize;
    fn acb_caption_event_destroy(event: *mut c_void);
    fn acb_caption_event_region_count(event: *const c_void) -> u32;
    fn acb_caption_event_region_at(
        event: *const c_void,
        index: u32,
        region: *mut NativeRegion,
    ) -> i32;
    fn acb_caption_event_character_count(event: *const c_void) -> u32;
    fn acb_caption_event_character_at(
        event: *const c_void,
        index: u32,
        character: *mut NativeCharacter,
    ) -> i32;
    fn acb_caption_event_drcs_count(event: *const c_void) -> u32;
    fn acb_caption_event_drcs_at(
        event: *const c_void,
        index: u32,
        glyph: *mut NativeDrcsGlyph,
    ) -> i32;
    fn acb_caption_event_copy_drcs_pixels(
        event: *const c_void,
        index: u32,
        destination: *mut u8,
        capacity: usize,
    ) -> usize;
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptionScene {
    pub pts_ms: i64,
    pub wait_duration_ms: i64,
    pub plane_width: i32,
    pub plane_height: i32,
    pub regions: Vec<CaptionRegion>,
    pub characters: Vec<CaptionCharacter>,
    pub drcs_glyphs: Vec<DrcsGlyph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_image: Option<RenderedCaptionImage>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RenderedCaptionImage {
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub dst_x: i32,
    pub dst_y: i32,
    pub rgba_base64: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptionRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub is_ruby: bool,
    pub first_character: u32,
    pub character_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaptionCharacter {
    pub kind: u32,
    pub codepoint: u32,
    pub pua_codepoint: u32,
    pub drcs_code: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub horizontal_spacing: i32,
    pub vertical_spacing: i32,
    pub horizontal_scale: f32,
    pub vertical_scale: f32,
    pub text_color: u32,
    pub back_color: u32,
    pub stroke_color: u32,
    pub style: u32,
    pub enclosure_style: u32,
    pub utf8: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DrcsGlyph {
    pub drcs_code: u32,
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    pub depth_bits: i32,
    pub alternative_codepoint: u32,
    pub md5: String,
    pub alternative_text: String,
    pub pixels: Vec<u8>,
}

pub struct DecodeResult {
    pub status: i32,
    pub scene: Option<CaptionScene>,
}

fn nul_terminated_text(bytes: &[c_char]) -> String {
    let bytes: Vec<u8> = bytes
        .iter()
        .map(|byte| *byte as u8)
        .take_while(|byte| *byte != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

unsafe fn copy_scene(
    event: *const c_void,
    summary: &CaptionSummary,
    rendered_image: Option<RenderedCaptionImage>,
) -> Option<CaptionScene> {
    let mut regions = Vec::with_capacity(unsafe { acb_caption_event_region_count(event) } as usize);
    for index in 0..unsafe { acb_caption_event_region_count(event) } {
        let mut region = NativeRegion::default();
        if unsafe { acb_caption_event_region_at(event, index, &mut region) } == 0 {
            return None;
        }
        regions.push(CaptionRegion {
            x: region.x,
            y: region.y,
            width: region.width,
            height: region.height,
            is_ruby: region.is_ruby != 0,
            first_character: region.first_character,
            character_count: region.character_count,
        });
    }
    let mut characters =
        Vec::with_capacity(unsafe { acb_caption_event_character_count(event) } as usize);
    for index in 0..unsafe { acb_caption_event_character_count(event) } {
        let mut character = NativeCharacter::default();
        if unsafe { acb_caption_event_character_at(event, index, &mut character) } == 0 {
            return None;
        }
        characters.push(CaptionCharacter {
            kind: character.kind,
            codepoint: character.codepoint,
            pua_codepoint: character.pua_codepoint,
            drcs_code: character.drcs_code,
            x: character.x,
            y: character.y,
            width: character.width,
            height: character.height,
            horizontal_spacing: character.horizontal_spacing,
            vertical_spacing: character.vertical_spacing,
            horizontal_scale: character.horizontal_scale,
            vertical_scale: character.vertical_scale,
            text_color: character.text_color,
            back_color: character.back_color,
            stroke_color: character.stroke_color,
            style: character.style,
            enclosure_style: character.enclosure_style,
            utf8: nul_terminated_text(&character.utf8),
        });
    }
    let mut drcs_glyphs =
        Vec::with_capacity(unsafe { acb_caption_event_drcs_count(event) } as usize);
    for index in 0..unsafe { acb_caption_event_drcs_count(event) } {
        let mut glyph = NativeDrcsGlyph::default();
        if unsafe { acb_caption_event_drcs_at(event, index, &mut glyph) } == 0 {
            return None;
        }
        let pixel_count =
            unsafe { acb_caption_event_copy_drcs_pixels(event, index, std::ptr::null_mut(), 0) };
        if pixel_count != glyph.pixel_count {
            return None;
        }
        let mut pixels = vec![0; pixel_count];
        if pixel_count != 0
            && unsafe {
                acb_caption_event_copy_drcs_pixels(event, index, pixels.as_mut_ptr(), pixels.len())
            } != pixel_count
        {
            return None;
        }
        drcs_glyphs.push(DrcsGlyph {
            drcs_code: glyph.drcs_code,
            width: glyph.width,
            height: glyph.height,
            depth: glyph.depth,
            depth_bits: glyph.depth_bits,
            alternative_codepoint: glyph.alternative_codepoint,
            md5: nul_terminated_text(&glyph.md5),
            alternative_text: nul_terminated_text(&glyph.alternative_text),
            pixels,
        });
    }
    Some(CaptionScene {
        pts_ms: summary.pts_ms,
        wait_duration_ms: summary.wait_duration_ms,
        plane_width: summary.plane_width,
        plane_height: summary.plane_height,
        regions,
        characters,
        drcs_glyphs,
        rendered_image,
    })
}

pub struct NativeB24Decoder(NonNull<c_void>);

impl NativeB24Decoder {
    pub fn new() -> Option<Self> {
        // SAFETY: the bridge constructs and owns the native context internally.
        NonNull::new(unsafe { acb_decoder_create() }).map(Self)
    }

    pub fn feed(&mut self, data: &[u8], pts_ms: i64) -> DecodeResult {
        if data.is_empty() {
            return DecodeResult {
                status: 1,
                scene: None,
            };
        }
        let mut summary = CaptionSummary::default();
        let mut event = std::ptr::null_mut();
        // SAFETY: self owns a valid decoder; data, summary and event pointer live for the call.
        let status = unsafe {
            acb_decoder_feed(
                self.0.as_ptr(),
                data.as_ptr(),
                data.len(),
                pts_ms,
                &mut summary,
                &mut event,
            )
        };
        let scene = NonNull::new(event).and_then(|event| {
            // SAFETY: bridge guarantees event ownership until its matching destroy call.
            let rendered_image = unsafe { copy_rendered_image(self.0.as_ptr()) };
            let copied = unsafe { copy_scene(event.as_ptr(), &summary, rendered_image) };
            // SAFETY: event was returned by the bridge and is no longer used after copying.
            unsafe { acb_caption_event_destroy(event.as_ptr()) };
            copied
        });
        DecodeResult { status, scene }
    }
}

unsafe fn copy_rendered_image(decoder: *const c_void) -> Option<RenderedCaptionImage> {
    let mut image = NativeRenderedImage::default();
    if unsafe { acb_decoder_get_rendered_image(decoder, &mut image) } == 0 || image.bitmap_size == 0
    {
        return None;
    }
    let mut pixels = vec![0_u8; image.bitmap_size];
    if unsafe { acb_decoder_copy_rendered_rgba(decoder, pixels.as_mut_ptr(), pixels.len()) }
        != image.bitmap_size
    {
        return None;
    }
    Some(RenderedCaptionImage {
        width: image.width,
        height: image.height,
        stride: image.stride,
        dst_x: image.dst_x,
        dst_y: image.dst_y,
        rgba_base64: BASE64.encode(pixels),
    })
}

impl Drop for NativeB24Decoder {
    fn drop(&mut self) {
        // SAFETY: the pointer originated from acb_decoder_create and is destroyed once.
        unsafe { acb_decoder_destroy(self.0.as_ptr()) };
    }
}
