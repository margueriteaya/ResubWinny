use crate::*;

pub(crate) fn inspect_input(path: &Path) -> io::Result<InputInspection> {
    let bytes = fs::metadata(path)?.len();
    match probe_path(path)?.kind {
        InputKind::MpegTs => {
            let b24_tracks = discover_b24_tracks(path)?;
            let data_tracks = discover_mpeg_ts_data_tracks(path)?;
            let broadcast = discover_broadcast_metadata(
                path,
                188,
                0,
                b24_tracks.first().map(|track| track.service_id),
            )?;
            let (route_code, route, service, tracks) = match b24_tracks.first() {
                Some(track) => (
                    "mpeg_ts_b24_verified",
                    format!(
                        "Traditional MPEG-TS · ARIB STD-B24 caption route · PMT 0x{:04X}",
                        track.pmt_pid
                    ),
                    track
                        .service_name
                        .as_deref()
                        .map(|name| format!("{name} · service {} · PMT 0x{:04X}", track.service_id, track.pmt_pid))
                        .unwrap_or_else(|| format!("Service {} · PMT 0x{:04X}", track.service_id, track.pmt_pid)),
                    b24_tracks
                        .iter()
                        .map(|candidate| CaptionTrackInspection {
                            label: "ARIB STD-B24 caption".into(),
                            detail: format!(
                                "PID 0x{:04X} · service {}{}",
                                candidate.caption_pid,
                                candidate.service_id,
                                candidate
                                    .service_name
                                    .as_deref()
                                    .map(|name| format!(" · {name}"))
                                    .unwrap_or_default()
                            ),
                        })
                        .collect(),
                ),
                None => match data_tracks {
                    Some(track) => (
                        "mpeg_ts_ttml_candidate",
                        format!(
                            "MPEG-TS private PES route · PMT 0x{:04X} · ARIB-TTML is validated only after strict XML extraction",
                            track.pmt_pid
                        ),
                        format!("Program map 0x{:04X}", track.pmt_pid),
                        track
                            .pids
                            .iter()
                            .map(|pid| {
                                let kind = track.component_kind(*pid);
                                CaptionTrackInspection {
                                    label: format!("MPEG-TS private {kind}"),
                                    detail: format!(
                                        "PID 0x{pid:04X} · {kind} component · candidate ARIB-TTML PES route"
                                    ),
                                }
                            })
                            .collect(),
                    ),
                    None => (
                        "unknown_unsupported",
                        "MPEG-TS · no supported B24 or private data stream found in the initial PSI scan".into(),
                        "No caption service discovered".into(),
                        Vec::new(),
                    ),
                },
            };
            Ok(InputInspection {
                bytes,
                container: "MPEG-2 TS · 188-byte packets".into(),
                route_code,
                route,
                service,
                tracks,
                broadcast,
            })
        }
        InputKind::M2ts => {
            let track = discover_m2ts_data_tracks(path)?;
            let broadcast = discover_broadcast_metadata(path, 192, 4, None)?;
            let (route_code, route, service, tracks) = match track {
                Some(track) => {
                    let tracks = track
                        .pids
                        .iter()
                        .map(|pid| {
                            let kind = track.component_kind(*pid);
                            CaptionTrackInspection {
                                label: format!("BS4K/8K private {kind}"),
                                detail: format!(
                                    "PID 0x{pid:04X} · {kind} component · candidate ARIB-TTML PES route"
                                ),
                            }
                        })
                        .collect();
                    (
                        "mpeg_ts_ttml_candidate",
                        format!(
                            "192-byte MPEG-TS private PES route · PMT 0x{:04X} · ARIB-TTML is validated only after strict XML extraction",
                            track.pmt_pid
                        ),
                        format!("Program map 0x{:04X}", track.pmt_pid),
                        tracks,
                    )
                }
                None => (
                    "unknown_unsupported",
                    "M2TS · no private data PID found in the initial PSI scan".into(),
                    "No caption service discovered".into(),
                    Vec::new(),
                ),
            };
            Ok(InputInspection {
                bytes,
                container: "M2TS · 192-byte packets".into(),
                route_code,
                route,
                service,
                tracks,
                broadcast,
            })
        }
        InputKind::Tlv => {
            let probe = probe_path(path)?;
            let diagnostics = scan_tlv_diagnostics(path, probe.sync_offset.unwrap_or_default())?;
            let packet_types = diagnostics
                .types
                .iter()
                .map(|(kind, count)| format!("0x{kind:02X} × {count}"))
                .collect::<Vec<_>>()
                .join(", ");
            let udp_ports = diagnostics
                .udp_ports
                .iter()
                .map(|(port, count)| format!("{port} × {count}"))
                .collect::<Vec<_>>()
                .join(", ");
            let mmtp_packet_ids = diagnostics
                .mmtp_packet_ids
                .iter()
                .map(|(packet_id, count)| {
                    let sequence = diagnostics
                        .mmtp_sequences
                        .get(packet_id)
                        .copied()
                        .unwrap_or_default();
                    format!("0x{packet_id:04X} × {count} (seq {sequence})")
                })
                .collect::<Vec<_>>()
                .join(", ");
            let mmtp_payload_types = diagnostics
                .mmtp_payload_types
                .iter()
                .map(|(kind, count)| format!("0x{kind:02X} × {count}"))
                .collect::<Vec<_>>()
                .join(", ");
            let mut tracks = vec![CaptionTrackInspection {
                label: "TLV/MMTP transport diagnostics".into(),
                detail: format!(
                    "TLV types: {}. Direct IPv6/UDP packets: {}. UDP destinations: {}. MMTP packets: {}. Packet IDs: {}. MMTP payload types: {}. Signalling messages reassembled: {}; fragments dropped: {}. stpp MFUs: {} fragments, {} complete payloads / {} bytes, {} dropped.",
                    if packet_types.is_empty() {
                        "none"
                    } else {
                        &packet_types
                    },
                    diagnostics.ipv6_packets,
                    if udp_ports.is_empty() {
                        "none"
                    } else {
                        &udp_ports
                    },
                    diagnostics.mmtp_packets,
                    if mmtp_packet_ids.is_empty() {
                        "none"
                    } else {
                        &mmtp_packet_ids
                    },
                    if mmtp_payload_types.is_empty() {
                        "none"
                    } else {
                        &mmtp_payload_types
                    },
                    diagnostics.signalling_fragments_reassembled,
                    diagnostics.signalling_fragments_dropped,
                    diagnostics.stpp_mfu_fragments,
                    diagnostics.stpp_mfu_completed,
                    diagnostics.stpp_payload_bytes,
                    diagnostics.stpp_mfu_dropped,
                ),
            }];
            tracks.extend(diagnostics.mpt_assets.iter().map(|(packet_id, asset_type)| {
                let descriptor_tags = diagnostics
                    .mpt_descriptor_tags
                    .get(packet_id)
                    .map(|tags| {
                        if tags.is_empty() {
                            "none".to_owned()
                        } else {
                            tags.iter()
                                .map(|tag| format!("0x{tag:04X}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    })
                    .unwrap_or_else(|| "unavailable".to_owned());
                CaptionTrackInspection {
                    label: format!("MMT asset · {asset_type}"),
                    detail: format!(
                        "MMTP packet ID 0x{packet_id:04X}. MPT descriptor tags: {descriptor_tags}. {} MPT MPU presentation timestamp(s) retained as raw NTP metadata. Complete stpp payloads are available through dump-tlv; conversion is limited to self-contained, clocked TTML payloads with a supported XML encoding.",
                        diagnostics
                            .mpt_presentation_ntp
                            .keys()
                            .filter(|(id, _)| id == packet_id)
                            .count(),
                    ),
                }
            }));
            let route = if diagnostics.mmtp_packets == 0 {
                "TLV framing was recognised, but no supported MMTP packet was observed in the bounded probe window. No caption asset is guessed.".into()
            } else if diagnostics
                .mpt_assets
                .values()
                .any(|asset_type| asset_type == "stpp")
            {
                format!(
                    "Bounded TLV/MMTP inspection found an stpp timed-text asset in MPT signalling. It observed {} complete closed-caption payload(s) and {} exact MPU presentation NTP value(s). dump-tlv retains raw evidence; conversion accepts only complete TTML payloads with supported XML encoding and matching NTP metadata.",
                    diagnostics.stpp_mfu_completed,
                    diagnostics.mpt_presentation_ntp.len(),
                )
            } else {
                "Bounded TLV/MMTP inspection completed. It reports supported signalling reassembly and raw source metadata; conversion requires a discovered clocked stpp TTML payload with supported XML encoding.".into()
            };
            Ok(InputInspection {
                bytes,
                container: "ISDB-S3 TLV · variable-length packets".into(),
                route_code: "tlv_mmtp_experimental",
                route,
                service: format!(
                    "TLV sync offset {} · {} consecutive packets in probe · {} packets / {} bytes observed · {} MMTP packets · {} MPT assets",
                    probe.sync_offset.unwrap_or_default(),
                    probe.confidence,
                    diagnostics.packets,
                    diagnostics.payload_bytes,
                    diagnostics.mmtp_packets,
                    diagnostics.mpt_assets.len(),
                ),
                tracks,
                broadcast: BroadcastMetadata::default(),
            })
        }
        InputKind::Unknown => Ok(InputInspection {
            bytes,
            container: "Unknown container".into(),
            route_code: "unknown_unsupported",
            route: "Unsupported or unrecognised recording container.".into(),
            service: "No service information available".into(),
            tracks: Vec::new(),
            broadcast: BroadcastMetadata::default(),
        }),
    }
}

pub(crate) fn sync_hits(bytes: &[u8], packet_size: usize, offset: usize) -> usize {
    (offset..bytes.len())
        .step_by(packet_size)
        .take_while(|index| *index < bytes.len())
        .filter(|index| {
            let Some(ts) = bytes.get(*index..index.saturating_add(188)) else {
                return false;
            };
            ts.first() == Some(&0x47) && (ts[3] & 0x30) != 0
        })
        .count()
}

pub(crate) fn probe_bytes(bytes: &[u8]) -> InputProbe {
    let ts_hits = sync_hits(bytes, 188, 0);
    let m2ts_hits = sync_hits(bytes, 192, 4);
    if let Some((offset, packets)) = tlv_probe(bytes) {
        let strongest_ts_candidate = ts_hits.max(m2ts_hits);
        if packets >= MIN_SYNC_HITS && packets >= strongest_ts_candidate {
            return InputProbe {
                kind: InputKind::Tlv,
                packet_size: None,
                sync_offset: Some(offset),
                confidence: packets,
            };
        }
    }
    if ts_hits >= MIN_SYNC_HITS && ts_hits >= m2ts_hits {
        return InputProbe {
            kind: InputKind::MpegTs,
            packet_size: Some(188),
            sync_offset: Some(0),
            confidence: ts_hits,
        };
    }
    if m2ts_hits >= MIN_SYNC_HITS {
        return InputProbe {
            kind: InputKind::M2ts,
            packet_size: Some(192),
            sync_offset: Some(4),
            confidence: m2ts_hits,
        };
    }
    if let Some((offset, packets)) = tlv_probe(bytes) {
        return InputProbe {
            kind: InputKind::Tlv,
            packet_size: None,
            sync_offset: Some(offset),
            confidence: packets,
        };
    }
    InputProbe {
        kind: InputKind::Unknown,
        packet_size: None,
        sync_offset: None,
        confidence: 0,
    }
}

pub(crate) fn tlv_probe(bytes: &[u8]) -> Option<(usize, usize)> {
    bytes
        .iter()
        .enumerate()
        .filter_map(|(offset, value)| (*value == 0x7f).then_some(offset))
        .filter_map(|offset| {
            let mut position = offset;
            let mut packets = 0;
            while let Some(header) = bytes.get(position..position.saturating_add(4)) {
                if header[0] != 0x7f {
                    break;
                }
                let length = usize::from(u16::from_be_bytes([header[2], header[3]]));
                let Some(next) = position.checked_add(4 + length) else {
                    break;
                };
                if next > bytes.len() {
                    break;
                }
                packets += 1;
                position = next;
            }
            (packets >= MIN_SYNC_HITS).then_some((offset, packets))
        })
        .max_by_key(|(_, packets)| *packets)
}
