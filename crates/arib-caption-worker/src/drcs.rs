use crate::*;

pub(crate) fn load_drcs_mapping(path: &Path) -> io::Result<HashMap<u32, String>> {
    let values: HashMap<String, String> =
        serde_json::from_reader(File::open(path)?).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid DRCS mapping file: {error}"),
            )
        })?;
    values
        .into_iter()
        .map(|(key, value)| {
            let code = key
                .trim()
                .strip_prefix("0x")
                .or_else(|| key.trim().strip_prefix("0X"))
                .map(|number| u32::from_str_radix(number, 16))
                .unwrap_or_else(|| key.trim().parse::<u32>())
                .map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid DRCS code {key:?}: {error}"),
                    )
                })?;
            Ok((code, value))
        })
        .collect()
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            track_id: None,
            drcs_mode: DrcsMode::PreserveGlyph,
            drcs_replacements: HashMap::new(),
            overwrite: false,
            webvtt: false,
            srt: false,
            keep_ass: true,
            ttml: false,
            archive: false,
            raw: false,
            drcs_report: false,
            preserve_ruby: true,
            preserve_position: true,
            preserve_color: true,
            preserve_drcs: true,
            preserve_gaiji: true,
            preserve_accessibility: true,
        }
    }
}

pub(crate) fn drcs_drawing(glyph: &native_b24::DrcsGlyph) -> String {
    let bits = glyph.depth_bits;
    if glyph.width <= 0 || glyph.height <= 0 || !(1..=8).contains(&bits) || 8 % bits != 0 {
        return String::new();
    }
    let pixels_per_byte = 8 / bits as usize;
    let required = glyph.width as usize * glyph.height as usize;
    if glyph.pixels.len().saturating_mul(pixels_per_byte) < required {
        return String::new();
    }
    let mut drawing = String::new();
    let mask = (1u8 << bits) - 1;
    for y in 0..glyph.height as usize {
        let mut x = 0;
        while x < glyph.width as usize {
            let value = glyph_pixel(glyph, x, y, pixels_per_byte, mask, bits);
            if value == 0 {
                x += 1;
                continue;
            }
            let start = x;
            x += 1;
            while x < glyph.width as usize
                && glyph_pixel(glyph, x, y, pixels_per_byte, mask, bits) != 0
            {
                x += 1;
            }
            drawing.push_str(&format!(
                "m {start} {y} l {x} {y} l {x} {} l {start} {} ",
                y + 1,
                y + 1
            ));
        }
    }
    drawing
}

pub(crate) fn glyph_pixel(
    glyph: &native_b24::DrcsGlyph,
    x: usize,
    y: usize,
    pixels_per_byte: usize,
    mask: u8,
    bits: i32,
) -> u8 {
    let index = y * glyph.width as usize + x;
    let byte = glyph.pixels[index / pixels_per_byte];
    let shift = 8 - bits as usize * ((index % pixels_per_byte) + 1);
    (byte >> shift) & mask
}

pub(crate) fn write_drcs_assets(
    directory: &Path,
    scene: &native_b24::CaptionScene,
    known: &mut HashSet<String>,
) -> io::Result<bool> {
    let mut wrote = false;
    for glyph in &scene.drcs_glyphs {
        let key = drcs_asset_key(glyph);
        if !known.insert(key.clone()) {
            continue;
        }
        fs::create_dir_all(directory)?;
        let data_path = directory.join(format!("drcs-{key}.bin"));
        let info_path = directory.join(format!("drcs-{key}.json"));
        fs::write(data_path, &glyph.pixels)?;
        fs::write(
            info_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "drcs_code": glyph.drcs_code,
                "width": glyph.width,
                "height": glyph.height,
                "depth": glyph.depth,
                "depth_bits": glyph.depth_bits,
                "alternative_codepoint": glyph.alternative_codepoint,
                "alternative_text": glyph.alternative_text,
                "pixel_encoding": "libaribcaption raw pixels",
            }))?,
        )?;
        wrote = true;
    }
    Ok(wrote)
}

pub(crate) fn drcs_asset_key(glyph: &native_b24::DrcsGlyph) -> String {
    if glyph.md5.is_empty() {
        format!("{:08X}", glyph.drcs_code)
    } else {
        glyph.md5.clone()
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct DrcsReportGlyph {
    drcs_code: u32,
    width: i32,
    height: i32,
    depth: i32,
    depth_bits: i32,
    alternative_codepoint: u32,
    alternative_text: String,
    asset: String,
}

pub(crate) fn write_drcs_report(
    output: &Path,
    source: &Path,
    directory: &Path,
    glyphs: &BTreeMap<String, native_b24::DrcsGlyph>,
    overwrite: bool,
) -> io::Result<Option<PathBuf>> {
    if glyphs.is_empty() {
        return Ok(None);
    }
    let report = output.with_extension("drcs.json");
    if report.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "DRCS report already exists",
        ));
    }
    let temporary = report.with_extension("json.part");
    let glyphs = glyphs
        .values()
        .map(|glyph| {
            let key = drcs_asset_key(glyph);
            DrcsReportGlyph {
                drcs_code: glyph.drcs_code,
                width: glyph.width,
                height: glyph.height,
                depth: glyph.depth,
                depth_bits: glyph.depth_bits,
                alternative_codepoint: glyph.alternative_codepoint,
                alternative_text: glyph.alternative_text.clone(),
                asset: directory
                    .join(format!("drcs-{key}.json"))
                    .display()
                    .to_string(),
            }
        })
        .collect::<Vec<_>>();
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&serde_json::json!({
            "type": "arib_caption_drcs_report",
            "version": 1,
            "source": source,
            "asset_directory": directory,
            "glyph_count": glyphs.len(),
            "glyphs": glyphs,
        }))?,
    )?;
    publish_file(&temporary, &report, overwrite)?;
    Ok(Some(report))
}
