use crate::{
    models::{DrcsGlyph, DrcsMappingInput, DrcsReport},
    storage::write_atomic,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::{fs, path::PathBuf};
use tauri::Manager;

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

#[tauri::command]
pub fn load_drcs_report(path: String) -> Result<Vec<DrcsGlyph>, String> {
    let report_path = PathBuf::from(&path).with_extension("drcs.json");
    let report: DrcsReport = serde_json::from_slice(
        &fs::read(&report_path)
            .map_err(|error| format!("No DRCS report is available for this task: {error}"))?,
    )
    .map_err(|error| format!("Could not read DRCS report: {error}"))?;
    report
        .glyphs
        .into_iter()
        .enumerate()
        .map(|(index, glyph)| {
            let raw = fs::read(PathBuf::from(&glyph.asset).with_extension("bin"))
                .map_err(|error| format!("Could not read DRCS glyph resource: {error}"))?;
            Ok(DrcsGlyph {
                id: format!("0x{:X}-{}", glyph.drcs_code, index + 1),
                width: glyph.width,
                height: glyph.height,
                alternative_text: glyph.alternative_text,
                image: drcs_svg(glyph.width, glyph.height, glyph.depth_bits, &raw)?,
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
    use super::drcs_svg;
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
}
