use crate::{
    models::{DrcsGlyph, DrcsMappingInput, DrcsReport},
    storage::write_atomic,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::{fs, path::PathBuf};
use tauri::Manager;

const MAX_DRCS_ASSET_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DRCS_REPORT_BYTES: u64 = 1024 * 1024;
const MAX_DRCS_GLYPHS: usize = 64;

pub fn drcs_svg(width: u32, height: u32, depth_bits: u8, bytes: &[u8]) -> Result<String, String> {
    if width == 0 || height == 0 || width > 512 || height > 512 || depth_bits != 2 {
        return Err("The DRCS glyph has unsupported dimensions or pixel depth.".into());
    }
    let pixel_count = (width as usize) * (height as usize);
    let expected = pixel_count.div_ceil(4);
    if bytes.len() < expected {
        return Err("The DRCS glyph resource is truncated.".into());
    }
    let mut body = String::new();
    for index in 0..pixel_count {
        let value = (bytes[index / 4] >> (6 - 2 * (index % 4))) & 0b11;
        if value != 0 {
            let x = index % width as usize;
            let y = index / width as usize;
            body.push_str(&format!("<rect x=\"{x}\" y=\"{y}\" width=\"1\" height=\"1\" fill=\"white\" fill-opacity=\"{:.3}\"/>", value as f32 / 3.0));
        }
    }
    Ok(format!("data:image/svg+xml;base64,{}", BASE64.encode(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" shape-rendering=\"crispEdges\"><rect width=\"100%\" height=\"100%\" fill=\"#090d12\"/>{body}</svg>"))))
}

pub fn b62_font_glyph_svg(
    source_codepoint: u32,
    resource_format: Option<&str>,
    bytes: &[u8],
) -> Result<String, String> {
    let character = char::from_u32(source_codepoint)
        .ok_or_else(|| "The B62 DRCS source codepoint is invalid.".to_owned())?;
    let font = match resource_format {
        Some("woff") => Some(("font/woff", "woff")),
        Some("woff2") => Some(("font/woff2", "woff2")),
        Some("truetype") => Some(("font/ttf", "truetype")),
        Some("opentype") => Some(("font/otf", "opentype")),
        _ => None,
    };
    let body = if let Some((mime, format)) = font {
        let encoded = BASE64.encode(bytes);
        format!(
            "<style>@font-face{{font-family:B62Preview;src:url(data:{mime};base64,{encoded}) format('{format}')}}</style><text x='48' y='67' text-anchor='middle' font-family='B62Preview' font-size='64'>{character}</text>"
        )
    } else {
        format!(
            "<text x='48' y='43' text-anchor='middle' fill='white' font-size='14'>B62 DRCS</text><text x='48' y='65' text-anchor='middle' fill='white' font-size='12'>U+{source_codepoint:04X}</text>"
        )
    };
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        BASE64.encode(format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 96 96'><rect width='100%' height='100%' fill='#090d12'/>{body}</svg>"))
    ))
}

#[tauri::command]
pub fn load_drcs_report(path: String) -> Result<Vec<DrcsGlyph>, String> {
    let path = PathBuf::from(path);
    let report_path = if path.to_string_lossy().ends_with(".drcs.json") {
        path
    } else {
        path.with_extension("drcs.json")
    };
    load_drcs_report_path(&report_path)
}

fn load_drcs_report_path(report_path: &std::path::Path) -> Result<Vec<DrcsGlyph>, String> {
    let report_metadata = fs::metadata(report_path)
        .map_err(|error| format!("No DRCS report is available for this task: {error}"))?;
    if !report_metadata.is_file() || report_metadata.len() > MAX_DRCS_REPORT_BYTES {
        return Err("The DRCS report is not a bounded regular file.".into());
    }
    let report: DrcsReport = serde_json::from_slice(
        &fs::read(report_path)
            .map_err(|error| format!("No DRCS report is available for this task: {error}"))?,
    )
    .map_err(|error| format!("Could not read DRCS report: {error}"))?;
    if report.glyphs.len() > MAX_DRCS_GLYPHS {
        return Err("The DRCS report contains too many glyphs.".into());
    }
    let mut rendered_asset_bytes = 0_u64;
    report
        .glyphs
        .into_iter()
        .enumerate()
        .map(|(index, glyph)| {
            let expected_directory = report_path.with_extension("");
            let requested = PathBuf::from(&glyph.asset);
            let asset = if glyph.mapping_id.is_some() {
                requested
            } else {
                requested.with_extension("bin")
            };
            let expected_directory = expected_directory
                .canonicalize()
                .map_err(|error| format!("Could not resolve the DRCS asset directory: {error}"))?;
            let report_directory = report_path
                .parent()
                .ok_or("The DRCS report has no parent directory.")?
                .canonicalize()
                .map_err(|error| format!("Could not resolve the DRCS report directory: {error}"))?;
            if !expected_directory.starts_with(&report_directory) {
                return Err("The DRCS asset directory escapes the report directory.".into());
            }
            let asset = asset
                .canonicalize()
                .map_err(|error| format!("Could not resolve DRCS glyph resource: {error}"))?;
            if !asset.starts_with(&expected_directory) {
                return Err("The DRCS glyph resource is outside its report directory.".into());
            }
            let metadata = fs::metadata(&asset)
                .map_err(|error| format!("Could not inspect DRCS glyph resource: {error}"))?;
            rendered_asset_bytes = rendered_asset_bytes.saturating_add(metadata.len());
            if !metadata.is_file() || rendered_asset_bytes > MAX_DRCS_ASSET_BYTES {
                return Err("The DRCS glyph resource is not a bounded regular file.".into());
            }
            let raw = fs::read(&asset)
                .map_err(|error| format!("Could not read DRCS glyph resource: {error}"))?;
            if let (Some(id), Some(source_codepoint)) =
                (glyph.mapping_id.as_ref(), glyph.source_codepoint)
            {
                return Ok(DrcsGlyph {
                    id: id.clone(),
                    width: 96,
                    height: 96,
                    alternative_text: glyph.alternative_text,
                    image: b62_font_glyph_svg(
                        source_codepoint,
                        glyph.resource_format.as_deref(),
                        &raw,
                    )?,
                });
            }
            let width = glyph.width.ok_or("The DRCS report is missing width.")?;
            let height = glyph.height.ok_or("The DRCS report is missing height.")?;
            let depth_bits = glyph
                .depth_bits
                .ok_or("The DRCS report is missing pixel depth.")?;
            let drcs_code = glyph
                .drcs_code
                .ok_or("The DRCS report is missing its source code.")?;
            Ok(DrcsGlyph {
                id: format!("0x{drcs_code:X}-{}", index + 1),
                width,
                height,
                alternative_text: glyph.alternative_text,
                image: drcs_svg(width, height, depth_bits, &raw)?,
            })
        })
        .collect()
}

fn mappings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("drcs-mappings.json"))
}

#[tauri::command]
pub fn load_drcs_mappings(app: tauri::AppHandle) -> Result<Vec<DrcsMappingInput>, String> {
    match fs::read(mappings_path(&app)?) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("Could not decode DRCS mappings: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not read DRCS mappings: {error}")),
    }
}

#[tauri::command]
pub fn save_drcs_mappings(
    app: tauri::AppHandle,
    mappings: Vec<DrcsMappingInput>,
) -> Result<(), String> {
    let path = mappings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create DRCS mapping directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(&mappings)
        .map_err(|error| format!("Could not encode DRCS mappings: {error}"))?;
    write_atomic(&path, &bytes).map_err(|error| format!("Could not publish DRCS mappings: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{b62_font_glyph_svg, drcs_svg, load_drcs_report_path};
    #[test]
    fn converts_packed_two_bit_drcs_to_an_embeddable_svg() {
        assert!(
            drcs_svg(2, 2, 2, &[0b11_10_01_00])
                .unwrap()
                .starts_with("data:image/svg+xml;base64,")
        );
    }
    #[test]
    fn rejects_a_truncated_drcs_payload() {
        assert!(drcs_svg(36, 36, 2, &[]).is_err());
    }
    #[test]
    fn creates_a_scoped_b62_font_preview_without_interpreting_the_mapping_id() {
        let preview = b62_font_glyph_svg(0xe000, Some("woff2"), b"font-bytes").unwrap();
        assert!(preview.starts_with("data:image/svg+xml;base64,"));
        assert!(b62_font_glyph_svg(0x11_0000, Some("woff2"), b"font").is_err());
    }
    #[test]
    fn loads_a_scoped_b62_report_id_for_dictionary_persistence() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("studio-b62-report-{stamp}"));
        let asset_directory = directory.join("captions.drcs");
        std::fs::create_dir_all(&asset_directory).unwrap();
        let asset = asset_directory.join("font.woff2");
        std::fs::write(&asset, b"font-bytes").unwrap();
        let id = format!("b62:sha256:{}:u+E000", "a".repeat(64));
        let report = directory.join("captions.drcs.json");
        std::fs::write(
            &report,
            serde_json::to_vec(&serde_json::json!({
                "glyphs": [{
                    "kind": "b62_font",
                    "mapping_id": id,
                    "source_codepoint": 0xe000,
                    "resource_format": "woff2",
                    "asset": asset,
                    "alternative_text": ""
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let glyphs = load_drcs_report_path(&report).unwrap();
        assert_eq!(glyphs.len(), 1);
        assert_eq!(glyphs[0].id, id);
        assert!(glyphs[0].image.starts_with("data:image/svg+xml;base64,"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_a_report_asset_outside_its_owned_directory() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("studio-b62-report-scope-{stamp}"));
        let asset_directory = directory.join("captions.drcs");
        std::fs::create_dir_all(&asset_directory).unwrap();
        let outside = directory.join("outside.woff2");
        std::fs::write(&outside, b"private").unwrap();
        let report = directory.join("captions.drcs.json");
        std::fs::write(
            &report,
            serde_json::to_vec(&serde_json::json!({
                "glyphs": [{
                    "mapping_id": format!("b62:sha256:{}:u+E000", "a".repeat(64)),
                    "source_codepoint": 0xe000,
                    "resource_format": "woff2",
                    "asset": outside
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let error = match load_drcs_report_path(&report) {
            Ok(_) => panic!("out-of-scope DRCS asset was accepted"),
            Err(error) => error,
        };
        assert!(error.contains("outside its report directory"));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
