use crate::*;

#[derive(Debug, Default)]
pub(crate) struct DrcsMappings {
    pub(crate) b24: HashMap<u32, String>,
    pub(crate) b62: HashMap<String, String>,
}

pub(crate) fn load_drcs_mapping(path: &Path) -> io::Result<DrcsMappings> {
    let values: HashMap<String, String> =
        serde_json::from_reader(File::open(path)?).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid DRCS mapping file: {error}"),
            )
        })?;
    let mut mappings = DrcsMappings::default();
    for (key, value) in values {
        if let Some(identity) = parse_b62_mapping_key(&key) {
            mappings.b62.insert(identity, value);
            continue;
        }
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
        mappings.b24.insert(code, value);
    }
    Ok(mappings)
}

fn parse_b62_mapping_key(key: &str) -> Option<String> {
    let key = key.trim();
    let remainder = key.strip_prefix("b62:sha256:")?;
    let (digest, codepoint) = remainder.split_once(":u+")?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let source_codepoint = u32::from_str_radix(codepoint, 16).ok()?;
    char::from_u32(source_codepoint)?;
    Some(crate::resource::b62_drcs_mapping_key(
        &digest.to_ascii_lowercase(),
        source_codepoint,
    ))
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            track_id: None,
            drcs_mode: DrcsMode::PreserveGlyph,
            drcs_replacements: HashMap::new(),
            ttml_drcs_replacements: HashMap::new(),
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

pub(crate) fn write_drcs_asset(
    directory: &Path,
    glyph: &native_b24::DrcsGlyph,
    known: &mut HashSet<String>,
) -> io::Result<bool> {
    let key = drcs_asset_key(glyph);
    if !known.insert(key.clone()) {
        return Ok(false);
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
    Ok(true)
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

#[derive(Debug, Clone)]
pub(crate) struct B62DrcsReportGlyph {
    pub(crate) mapping_id: String,
    pub(crate) source_codepoint: u32,
    pub(crate) resource: std::sync::Arc<TlvSubtitleResource>,
}

pub(crate) fn write_b62_drcs_report(
    output: &Path,
    source: &Path,
    glyphs: &BTreeMap<String, B62DrcsReportGlyph>,
    overwrite: bool,
) -> io::Result<Option<PathBuf>> {
    let report = output.with_extension("drcs.json");
    if glyphs.is_empty() {
        return Ok(None);
    }
    if report.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "DRCS report already exists",
        ));
    }
    let directory = output.with_extension("drcs");
    fs::create_dir_all(&directory)?;
    let records = glyphs
        .values()
        .map(|glyph| {
            let digest = resource_sha256(&glyph.resource.bytes);
            let format = bounded_resource_format(&glyph.resource.bytes);
            let extension = format.format_hint.unwrap_or("bin");
            let asset = directory.join(format!("b62-{digest}.{extension}"));
            if !asset.exists() {
                let temporary = asset.with_extension(format!("{extension}.part"));
                fs::write(&temporary, &glyph.resource.bytes)?;
                publish_file(&temporary, &asset, false)?;
            }
            Ok(serde_json::json!({
                "kind": "b62_font",
                "mapping_id": glyph.mapping_id,
                "source_codepoint": glyph.source_codepoint,
                "resource_index": glyph.resource.index,
                "resource_format": format.format_hint,
                "asset": asset,
                "alternative_text": "",
            }))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let temporary = report.with_extension("json.part");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&serde_json::json!({
            "type": "arib_caption_drcs_report",
            "version": 2,
            "source": source,
            "asset_directory": directory,
            "glyph_count": records.len(),
            "glyphs": records,
        }))?,
    )?;
    publish_file(&temporary, &report, overwrite)?;
    Ok(Some(report))
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

#[cfg(test)]
mod b62_report_tests {
    use super::*;

    #[test]
    fn writes_scoped_b62_ids_and_deduplicated_resource_assets() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("arib-b62-report-{stamp}"));
        fs::create_dir_all(&directory).expect("temporary directory");
        let output = directory.join("captions.ass");
        let bytes = b"wOF2\0\x01\0\0\0\0\0\x30\0\x01font".to_vec();
        let digest = resource_sha256(&bytes);
        let mapping_id = b62_drcs_mapping_key(&digest, 0xe000);
        let mut glyphs = BTreeMap::new();
        glyphs.insert(
            mapping_id.clone(),
            B62DrcsReportGlyph {
                mapping_id: mapping_id.clone(),
                source_codepoint: 0xe000,
                resource: std::sync::Arc::new(TlvSubtitleResource {
                    index: 1,
                    data_type: 1,
                    bytes: bytes.clone(),
                }),
            },
        );

        let report = write_b62_drcs_report(&output, Path::new("source.tlv"), &glyphs, true)
            .expect("B62 report")
            .expect("report path");
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&report).expect("report bytes")).unwrap();
        assert_eq!(value["version"], 2);
        assert_eq!(value["glyphs"][0]["mapping_id"], mapping_id);
        let asset = PathBuf::from(value["glyphs"][0]["asset"].as_str().unwrap());
        assert_eq!(fs::read(asset).unwrap(), bytes);
        fs::remove_dir_all(directory).expect("cleanup B62 report");
    }

    #[test]
    fn empty_b62_report_does_not_remove_a_previous_completed_report() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("arib-b62-empty-report-{stamp}"));
        fs::create_dir_all(&directory).expect("temporary directory");
        let output = directory.join("captions.ass");
        let report = output.with_extension("drcs.json");
        fs::write(&report, b"previous report").expect("previous report");

        assert!(
            write_b62_drcs_report(&output, Path::new("source.tlv"), &BTreeMap::new(), true,)
                .expect("empty report")
                .is_none()
        );
        assert_eq!(fs::read(&report).unwrap(), b"previous report");
        fs::remove_dir_all(directory).expect("cleanup empty report");
    }
}
