#[cfg(not(feature = "libaribtlv"))]
use super::*;

#[cfg(feature = "libaribtlv")]
pub(crate) use crate::native_tlv::scan_tlv_ttml_native as scan_tlv_ttml;

#[cfg(any(not(feature = "libaribtlv"), test))]
pub(crate) fn ntp_delta_ms(value: u64, origin: u64) -> i64 {
    let delta = i128::from(value) - i128::from(origin);
    let milliseconds = (delta * 1_000) >> 32;
    milliseconds.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// Streams only a deliberately narrow, demonstrable TLV conversion route:
/// complete `stpp` payloads that are both clocked by an MPT MPU timestamp and
/// contain a self-contained XML TTML document. XML is decoded only with its
/// BOM or declared character encoding; other `stpp` payloads are retained by
/// `dump-tlv`, but never guessed into captions here.
#[cfg(not(feature = "libaribtlv"))]
pub(crate) fn scan_tlv_ttml<F, P, C, R, A>(
    path: &Path,
    mut on_caption: F,
    mut on_progress: P,
    mut cancelled: C,
    mut on_payload: R,
    mut on_asset: A,
) -> io::Result<B24DecodeSummary>
where
    F: FnMut(TtmlCaption) -> io::Result<()>,
    P: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
    R: FnMut(u64, &TlvCaptionPayload) -> io::Result<()>,
    A: FnMut(TlvAssetEvidence) -> io::Result<()>,
{
    let probe = probe_path(path)?;
    if probe.kind != InputKind::Tlv {
        return Err(io::Error::other(
            "TLV TTML conversion requires an ISDB-S3 TLV input",
        ));
    }
    let mut reader = BufReader::with_capacity(1024 * 1024, File::open(path)?);
    let mut offset = probe.sync_offset.unwrap_or_default() as u64;
    reader.seek(SeekFrom::Start(offset))?;
    let mut diagnostics = TlvDiagnostics::default();
    let mut signalling_assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    let mut timeline_origin = None;
    let mut summary = B24DecodeSummary::default();
    let mut next_progress = PROGRESS_INTERVAL;
    while let Some((packet_type, payload, packet_offset)) =
        read_tlv_packet(&mut reader, &mut offset)?
    {
        if cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "conversion cancelled",
            ));
        }
        summary.bytes_read = offset;
        if summary.bytes_read >= next_progress {
            on_progress(&summary);
            next_progress += PROGRESS_INTERVAL;
        }
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
        diagnostics.current_source_offset = packet_offset;
        inspect_mmtp_packet(
            &packet,
            &mut diagnostics,
            &mut signalling_assemblers,
            &mut mpu_assemblers,
            Some(&mut captured_payloads),
            None,
        );
        for payload in captured_payloads {
            on_payload(packet_offset, &payload)?;
            let Some(presentation_ntp) = payload.presentation_ntp else {
                summary.decoder_errors += 1;
                continue;
            };
            let documents = ttml_documents(&payload.bytes);
            if documents.is_empty() {
                summary.decoder_errors += 1;
                continue;
            }
            let origin = *timeline_origin.get_or_insert(presentation_ntp);
            let base_ms = ntp_delta_ms(presentation_ntp, origin);
            summary.pes_packets += 1;
            for document in documents {
                for mut caption in parse_ttml_captions(&document.xml, base_ms) {
                    summary.captions += 1;
                    summary.characters += caption.text.chars().count() as u64;
                    caption.source = Some(TtmlCaptionSource {
                        route: "isdb_s3_tlv_mmtp_stpp",
                        source_offset: packet_offset,
                        mmpt_packet_id: payload.packet_id,
                        mpu_sequence_number: payload.mpu_sequence_number,
                        mmtp_sequence_number: payload.mmtp_sequence_number,
                        presentation_ntp: Some(presentation_ntp),
                        normalized_pts: None,
                        reference_start_pts: None,
                        reference_start_ntp: None,
                        reference_start_time_leap_indicator: None,
                        timeline_basis: TlvTimelineBasis::MptPresentationNtp,
                        track_id: None,
                        component_tag: None,
                        timing_mode: None,
                        operation_mode: None,
                        display_mode: None,
                        compression_type: None,
                        random_access: false,
                        discontinuity: false,
                        discontinuity_reasons: 0,
                        xml_encoding: document.encoding.label().to_owned(),
                        resources: payload
                            .resources
                            .iter()
                            .map(|resource| {
                                let format = bounded_resource_format(&resource.bytes);
                                TtmlResourceMetadata {
                                    index: resource.index,
                                    data_type: resource.data_type,
                                    byte_length: resource.bytes.len(),
                                    format_hint: format.format_hint,
                                    format_validation: format.format_validation,
                                    width: format.width,
                                    height: format.height,
                                    preview_available: bounded_png_preview_data_uri(
                                        &resource.bytes,
                                    )
                                    .is_some(),
                                }
                            })
                            .collect(),
                        resources_complete: payload.resources_complete,
                    });
                    on_caption(caption)?;
                }
            }
        }
    }
    for asset in tlv_asset_evidence(&diagnostics) {
        on_asset(asset)?;
    }
    if summary.captions == 0 {
        return Err(io::Error::other(
            "no complete clocked TTML captions with a supported declared XML encoding were found in discovered TLV stpp assets; use dump-tlv for raw evidence",
        ));
    }
    Ok(summary)
}
