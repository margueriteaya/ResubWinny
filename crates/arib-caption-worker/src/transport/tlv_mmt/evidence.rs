use super::*;

#[derive(Debug, Serialize)]
pub(crate) struct TlvRawExtractionSummary {
    pub(crate) tlv_packets: u64,
    pub(crate) mmtp_packets: u64,
    pub(crate) stpp_payloads: u64,
    pub(crate) stpp_payload_bytes: u64,
    pub(crate) non_stpp_payloads: u64,
    pub(crate) non_stpp_payload_bytes: u64,
    pub(crate) dropped_fragments: u64,
}

pub(crate) fn write_tlv_raw_header(writer: &mut BufWriter<File>, path: &Path) -> io::Result<()> {
    writeln!(
        writer,
        "{}",
        serde_json::json!({
            "type": "arib_caption_raw_mmtp_stpp",
            "version": 1,
            "source": path,
            "route": "isdb_s3_tlv_mmtp",
            "encoding": "hex",
            "timestamp": null,
            "note": "Records contain complete bounded payloads from MPT-confirmed assets. stpp records preserve the TTML subsample and same-MPU resource units; non-stpp records retain raw MPU/MFU bytes without guessed semantics. MMTP sequence numbers, source offsets, and exact MPT NTP presentation metadata when advertised are preserved; no PTS is inferred."
        })
    )
}

pub(crate) fn write_tlv_raw_payload(
    writer: &mut BufWriter<File>,
    source_offset: u64,
    payload: &TlvCaptionPayload,
) -> io::Result<()> {
    let scope_key = payload
        .mpu_sequence_number
        .map(|sequence| format!("packet:{}:mpu:{sequence}", payload.packet_id));
    serde_json::to_writer(
        &mut *writer,
        &serde_json::json!({
            "type": "stpp_closed_caption_payload",
            "tlv_packet_offset": source_offset,
            "mmpt_packet_id": payload.packet_id,
            "mpu_sequence_number": payload.mpu_sequence_number,
            "scope_key": scope_key,
            "mmtp_sequence_number": payload.mmtp_sequence_number,
            "presentation_ntp": payload.presentation_ntp,
            "timed_mfu": payload.timed,
            "resources_complete": payload.resources_complete,
            "resources": payload.resources.iter().map(|resource| {
                let format = bounded_resource_format(&resource.bytes);
                serde_json::json!({
                    "index": resource.index,
                    "data_type": resource.data_type,
                    "format_hint": format.format_hint,
                    "format_validation": format.format_validation,
                    "width": format.width,
                    "height": format.height,
                    "preview_data_uri": bounded_png_preview_data_uri(&resource.bytes),
                    "record_key": payload.mpu_sequence_number.map(|sequence| format!(
                        "stpp-resource:packet:{}:mpu:{sequence}:subsample:{}",
                        payload.packet_id, resource.index
                    )),
                    "payload_hex": hex_encode(&resource.bytes),
                })
            }).collect::<Vec<_>>(),
            "pts_ms": null,
            "payload_hex": hex_encode(&payload.bytes),
        }),
    )?;
    writer.write_all(b"\n")
}

pub(crate) fn write_tlv_asset_payload(
    writer: &mut BufWriter<File>,
    source_offset: u64,
    payload: &TlvResourcePayload,
) -> io::Result<()> {
    let format = bounded_resource_format(&payload.bytes);
    serde_json::to_writer(
        &mut *writer,
        &serde_json::json!({
            "type": "mmt_asset_payload",
            "asset_type": payload.asset_type,
            "tlv_packet_offset": source_offset,
            "mmtp_packet_id": payload.packet_id,
            "mpu_sequence_number": payload.mpu_sequence_number,
            "scope_key": format!("packet:{}:mpu:{}", payload.packet_id, payload.mpu_sequence_number),
            "mmtp_sequence_number": payload.mmtp_sequence_number,
            "presentation_ntp": payload.presentation_ntp,
            "timed_mfu": payload.timed,
            "format_hint": format.format_hint,
            "format_validation": format.format_validation,
            "width": format.width,
            "height": format.height,
            "preview_data_uri": bounded_png_preview_data_uri(&payload.bytes),
            "payload_hex": hex_encode(&payload.bytes),
            "note": "bounded complete MPU/MFU payload; asset semantics are not guessed",
        }),
    )?;
    writer.write_all(b"\n")
}

pub(crate) fn dump_tlv_stpp_raw(
    path: &Path,
    output: &Path,
    overwrite: bool,
) -> io::Result<TlvRawExtractionSummary> {
    let probe = probe_path(path)?;
    if probe.kind != InputKind::Tlv {
        return Err(io::Error::other(
            "raw MMTP extraction requires an ISDB-S3 TLV input",
        ));
    }
    if output.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "output exists; pass --overwrite to replace it",
        ));
    }
    let temporary = output.with_extension("jsonl.part");
    let mut writer = BufWriter::new(File::create(&temporary)?);
    write_tlv_raw_header(&mut writer, path)?;
    let mut reader = BufReader::with_capacity(1024 * 1024, File::open(path)?);
    let mut offset = probe.sync_offset.unwrap_or_default() as u64;
    reader.seek(SeekFrom::Start(offset))?;
    let mut diagnostics = TlvDiagnostics::default();
    let mut signalling_assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    while let Some((packet_type, payload, packet_offset)) =
        read_tlv_packet(&mut reader, &mut offset)?
    {
        diagnostics.packets += 1;
        diagnostics.payload_bytes += payload.len() as u64;
        *diagnostics.types.entry(packet_type).or_default() += 1;
        let Some(mmtp) = tlv_mmtp_payload(packet_type, &payload) else {
            continue;
        };
        let Some(packet) = parse_mmtp_packet(mmtp) else {
            continue;
        };
        let mut captured_payloads = Vec::new();
        let mut captured_assets = Vec::new();
        diagnostics.current_source_offset = packet_offset;
        inspect_mmtp_packet(
            &packet,
            &mut diagnostics,
            &mut signalling_assemblers,
            &mut mpu_assemblers,
            Some(&mut captured_payloads),
            Some(&mut captured_assets),
        );
        for payload in captured_payloads {
            write_tlv_raw_payload(&mut writer, packet_offset, &payload)?;
        }
        for payload in captured_assets {
            write_tlv_asset_payload(&mut writer, packet_offset, &payload)?;
        }
    }
    writer.flush()?;
    publish_file(&temporary, output, overwrite)?;
    Ok(TlvRawExtractionSummary {
        tlv_packets: diagnostics.packets,
        mmtp_packets: diagnostics.mmtp_packets,
        stpp_payloads: diagnostics.stpp_mfu_completed,
        stpp_payload_bytes: diagnostics.stpp_payload_bytes,
        non_stpp_payloads: diagnostics.non_stpp_mfu_completed,
        non_stpp_payload_bytes: diagnostics.non_stpp_payload_bytes,
        dropped_fragments: diagnostics.stpp_mfu_dropped,
    })
}
