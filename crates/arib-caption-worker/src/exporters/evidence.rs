use super::*;

pub(crate) const CAPTION_ARCHIVE_SCHEMA_VERSION: u32 = 1;

pub(crate) fn write_archive_header(
    writer: &mut BufWriter<File>,
    path: &Path,
    kind: &str,
) -> io::Result<()> {
    writeln!(
        writer,
        "{}",
        serde_json::json!({
            "type": "arib_caption_studio_archive",
            // `version` was the original field. Keep it as a compatibility
            // alias while giving the long-lived archive schema an explicit,
            // unambiguous name.
            "schemaVersion": CAPTION_ARCHIVE_SCHEMA_VERSION,
            "version": CAPTION_ARCHIVE_SCHEMA_VERSION,
            "source": path,
            "route": kind,
            "format": "jsonl",
            "note": "Decoded caption scenes. Enable --raw to write selected source PES records alongside this archive."
        })
    )
}

pub(crate) fn write_archive_record<T: Serialize>(
    writer: &mut BufWriter<File>,
    kind: &str,
    value: &T,
) -> io::Result<()> {
    serde_json::to_writer(
        &mut *writer,
        &serde_json::json!({ "type": kind, "value": value }),
    )?;
    writer.write_all(b"\n")?;
    // The desktop timeline tails this bounded JSONL artifact while a job is
    // running. Caption records are sparse compared with transport packets, so
    // publishing each complete line is a worthwhile correctness trade-off.
    writer.flush()
}

pub(crate) fn write_caption_archive_record(
    writer: &mut BufWriter<File>,
    cue: CaptionCueRef<'_>,
) -> io::Result<()> {
    let timing = cue.timing();
    let region = cue.region();
    debug_assert!(timing.end_ms >= timing.begin_ms);
    debug_assert!(region.width.is_none_or(|width| width >= 0));
    debug_assert!(region.height.is_none_or(|height| height >= 0));
    let expected_route = cue.route();
    match cue {
        CaptionCueRef::B24(interval) => {
            debug_assert_eq!(expected_route, CaptionRoute::B24);
            write_archive_record(writer, "region_interval", interval)
        }
        CaptionCueRef::AribTtml(caption) => {
            debug_assert_eq!(expected_route, CaptionRoute::AribTtml);
            write_archive_record(writer, "caption", caption)
        }
    }
}

pub(crate) fn write_raw_header(
    writer: &mut BufWriter<File>,
    path: &Path,
    route: &str,
) -> io::Result<()> {
    writeln!(
        writer,
        "{}",
        serde_json::json!({
            "type": "arib_caption_raw_pes",
            "version": 1,
            "source": path,
            "route": route,
            "encoding": "hex",
            "note": "One source PES per record. packet_offset identifies the first transport packet carrying that PES."
        })
    )
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        encoded.push(DIGITS[usize::from(byte >> 4)] as char);
        encoded.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    encoded
}

pub(crate) fn write_raw_pes_record(
    writer: &mut BufWriter<File>,
    pid: u16,
    packet_offset: u64,
    pes: &[u8],
) -> io::Result<()> {
    serde_json::to_writer(
        &mut *writer,
        &serde_json::json!({
            "type": "pes",
            "pid": pid,
            "packet_offset": packet_offset,
            "pts_ms": pes_pts_from_header(pes).map(Pts90k::to_millis),
            "pes_hex": hex_encode(pes),
        }),
    )?;
    writer.write_all(b"\n")
}
