use crate::*;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn b24_preview(scene: &native_b24::CaptionScene) -> CaptionPreview {
    let text = scene
        .characters
        .iter()
        .map(|character| {
            if !character.utf8.is_empty() {
                character.utf8.clone()
            } else {
                scene
                    .drcs_glyphs
                    .iter()
                    .find(|glyph| glyph.drcs_code == character.drcs_code)
                    .map(|glyph| glyph.alternative_text.clone())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "▧".into())
            }
        })
        .collect();
    let (x, y) = scene
        .regions
        .first()
        .map(|region| (region.x, region.y))
        .unwrap_or((scene.plane_width / 2, scene.plane_height / 2));
    let first = scene.characters.first();
    CaptionPreview {
        text,
        x: (x as f32 / scene.plane_width.max(1) as f32).clamp(0.0, 1.0),
        y: (y as f32 / scene.plane_height.max(1) as f32).clamp(0.0, 1.0),
        text_color: 0xff00_0000
            | first
                .map(|character| character.text_color)
                .unwrap_or(0x00ff_ffff),
        background_color: 0xb000_0000
            | first
                .map(|character| character.back_color & 0x00ff_ffff)
                .unwrap_or(0),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn preview_color_from_ttml(value: Option<&str>, fallback: u32) -> u32 {
    let Some(value) = value else {
        return fallback;
    };
    let Some(hex) = value.trim().strip_prefix('#') else {
        return fallback;
    };
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return fallback;
    }
    let Ok(rgb) = u32::from_str_radix(&hex[..6], 16) else {
        return fallback;
    };
    let alpha = if hex.len() == 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
    } else {
        255
    };
    (u32::from(alpha) << 24) | rgb
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn preview_caption(path: &Path) -> io::Result<Option<CaptionPreview>> {
    let interrupted = |error: &io::Error| error.kind() == io::ErrorKind::Interrupted;
    match probe_path(path)?.kind {
        InputKind::MpegTs => {
            if let Some(track) = discover_b24(path)? {
                let mut preview = None;
                let result = scan_b24(
                    path,
                    &track,
                    |scene| {
                        preview = Some(b24_preview(&scene));
                        Err(io::Error::new(io::ErrorKind::Interrupted, "preview ready"))
                    },
                    |_| {},
                    || false,
                    |_, _, _| Ok(()),
                );
                if let Err(error) = result
                    && !interrupted(&error)
                {
                    return Err(error);
                }
                Ok(preview)
            } else {
                let tracks = discover_mpeg_ts_data_tracks(path)?.ok_or_else(|| {
                    io::Error::other("no B24 caption or private ARIB-TTML data PID found")
                })?;
                let mut preview = None;
                let result = scan_mpeg_ts_ttml(
                    path,
                    &tracks,
                    |caption| {
                        preview = Some(ttml_preview(&caption));
                        Err(io::Error::new(io::ErrorKind::Interrupted, "preview ready"))
                    },
                    |_| {},
                    || false,
                    |_, _, _| Ok(()),
                );
                if let Err(error) = result
                    && !interrupted(&error)
                {
                    return Err(error);
                }
                Ok(preview)
            }
        }
        InputKind::M2ts => {
            let tracks = discover_m2ts_data_tracks(path)?
                .ok_or_else(|| io::Error::other("no BS4K/8K private data PID found"))?;
            let mut preview = None;
            let result = scan_m2ts_ttml(
                path,
                &tracks,
                |caption| {
                    preview = Some(ttml_preview(&caption));
                    Err(io::Error::new(io::ErrorKind::Interrupted, "preview ready"))
                },
                |_| {},
                || false,
                |_, _, _| Ok(()),
            );
            if let Err(error) = result
                && !interrupted(&error)
            {
                return Err(error);
            }
            Ok(preview)
        }
        InputKind::Tlv => {
            let mut preview = None;
            let result = scan_tlv_ttml(
                path,
                |caption| {
                    preview = Some(ttml_preview(&caption));
                    Err(io::Error::new(io::ErrorKind::Interrupted, "preview ready"))
                },
                |_| {},
                || false,
                |_, _| Ok(()),
                |_| Ok(()),
            );
            if let Err(error) = result
                && !interrupted(&error)
            {
                return Err(error);
            }
            Ok(preview)
        }
        InputKind::Unknown => Err(io::Error::other(
            "unsupported or unrecognised recording container",
        )),
    }
}

fn ttml_preview(caption: &TtmlCaption) -> CaptionPreview {
    CaptionPreview {
        text: caption.text.clone(),
        x: (caption.x as f32 / 1920.0).clamp(0.0, 1.0),
        y: (caption.y as f32 / 1080.0).clamp(0.0, 1.0),
        text_color: preview_color_from_ttml(caption.style.color.as_deref(), 0xffff_ffff),
        background_color: preview_color_from_ttml(
            caption.style.background_color.as_deref(),
            0xb000_0000,
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrcsMode {
    PreserveGlyph,
    UseUserMapping,
}

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub track_id: Option<u16>,
    pub drcs_mode: DrcsMode,
    pub drcs_replacements: HashMap<u32, String>,
    pub overwrite: bool,
    pub webvtt: bool,
    pub srt: bool,
    pub keep_ass: bool,
    pub ttml: bool,
    pub archive: bool,
    pub raw: bool,
    pub drcs_report: bool,
    pub preserve_ruby: bool,
    pub preserve_position: bool,
    pub preserve_color: bool,
    pub preserve_drcs: bool,
    pub preserve_gaiji: bool,
    pub preserve_accessibility: bool,
}
