use super::*;

#[derive(Default)]
pub(crate) struct TlvDiagnostics {
    pub(crate) packets: u64,
    pub(crate) payload_bytes: u64,
    pub(crate) types: BTreeMap<u8, u64>,
    pub(crate) ipv6_packets: u64,
    pub(crate) udp_ports: BTreeMap<u16, u64>,
    pub(crate) mmtp_packets: u64,
    pub(crate) mmtp_packet_ids: BTreeMap<u16, u64>,
    pub(crate) mmtp_sequences: BTreeMap<u16, u32>,
    pub(crate) mmtp_payload_types: BTreeMap<u8, u64>,
    pub(crate) mpt_assets: BTreeMap<u16, String>,
    pub(crate) mpt_asset_offsets: BTreeMap<u16, u64>,
    pub(crate) current_source_offset: u64,
    pub(crate) mpt_descriptor_tags: BTreeMap<u16, Vec<u16>>,
    // Exact NTP presentation instants advertised by an MPT MPU timestamp
    // descriptor. These are source metadata, not a normalized caption PTS.
    pub(crate) mpt_presentation_ntp: BTreeMap<(u16, u32), u64>,
    pub(crate) signalling_fragments_reassembled: u64,
    pub(crate) signalling_fragments_dropped: u64,
    pub(crate) stpp_mfu_fragments: u64,
    pub(crate) stpp_mfu_completed: u64,
    pub(crate) stpp_mfu_dropped: u64,
    pub(crate) stpp_payload_bytes: u64,
    pub(crate) non_stpp_mfu_completed: u64,
    pub(crate) non_stpp_payload_bytes: u64,
    pub(crate) subtitle_resources: BTreeMap<(u16, u32), TlvSubtitleResourceState>,
}

#[derive(Default)]
pub(crate) struct SignallingFragmentAssembler {
    in_fragment: bool,
    last_sequence_number: u32,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct MpuFragmentAssembler {
    pub(super) in_fragment: bool,
    pub(super) last_sequence_number: u32,
    pub(super) bytes: Vec<u8>,
}

pub(crate) fn parse_mpt_descriptors(descriptors: &[u8]) -> Option<(Vec<u16>, BTreeMap<u32, u64>)> {
    let mut cursor = SliceCursor::new(descriptors);
    let mut tags = Vec::new();
    let mut presentation_ntp = BTreeMap::new();
    while cursor.remaining() > 0 {
        let tag = cursor.be_u16()?;
        let length = usize::from(cursor.u8()?);
        let payload = cursor.take(length)?;
        // MPU timestamp descriptor: each record carries a 32-bit MPU sequence
        // number followed by its 64-bit NTP presentation instant.
        if tag == 0x0001 {
            let mut timestamps = SliceCursor::new(payload);
            while timestamps.remaining() > 0 {
                let sequence = timestamps.be_u32()?;
                let seconds = u64::from(timestamps.be_u32()?);
                let fraction = u64::from(timestamps.be_u32()?);
                presentation_ntp.insert(sequence, (seconds << 32) | fraction);
            }
        }
        tags.push(tag);
    }
    Some((tags, presentation_ntp))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MptAsset {
    pub(crate) packet_id: u16,
    pub(crate) asset_type: String,
    pub(crate) descriptor_tags: Vec<u16>,
    pub(crate) presentation_ntp: BTreeMap<u32, u64>,
}

pub(crate) fn parse_mpt_assets(table: &[u8]) -> Option<Vec<MptAsset>> {
    if table.first().copied()? != 0x20 || table.len() < 4 {
        return None;
    }
    let declared_length = usize::from(u16::from_be_bytes([table[2], table[3]]));
    let body = table.get(4..4 + declared_length)?;
    let mut cursor = SliceCursor::new(body);
    cursor.u8()?; // MPT mode and reserved bits
    let package_id_length = usize::from(cursor.u8()?);
    cursor.take(package_id_length)?;
    let descriptors_length = usize::from(cursor.be_u16()?);
    cursor.take(descriptors_length)?;
    let asset_count = usize::from(cursor.u8()?);
    let mut assets = Vec::new();
    for _ in 0..asset_count {
        cursor.u8()?; // identifier type
        cursor.take(4)?; // asset ID scheme
        let asset_id_length = usize::from(cursor.u8()?);
        cursor.take(asset_id_length)?;
        let asset_type = std::str::from_utf8(cursor.take(4)?)
            .ok()
            .unwrap_or("????")
            .to_owned();
        cursor.u8()?; // asset clock relation flag and reserved bits
        let location_count = usize::from(cursor.u8()?);
        let mut packet_id = None;
        for _ in 0..location_count {
            packet_id = skip_mmt_general_location(&mut cursor)?.or(packet_id);
        }
        let descriptor_length = usize::from(cursor.be_u16()?);
        let (descriptor_tags, presentation_ntp) =
            parse_mpt_descriptors(cursor.take(descriptor_length)?)?;
        if let Some(packet_id) = packet_id {
            assets.push(MptAsset {
                packet_id,
                asset_type,
                descriptor_tags,
                presentation_ntp,
            });
        }
    }
    Some(assets)
}

pub(crate) fn inspect_signalling_message(message: &[u8], diagnostics: &mut TlvDiagnostics) {
    let Some(id) = message.get(..2) else {
        return;
    };
    let table = match id {
        [0x80, 0x00] => {
            let Some(length) = message
                .get(3..5)
                .map(|value| usize::from(u16::from_be_bytes([value[0], value[1]])))
            else {
                return;
            };
            message.get(5..5 + length)
        }
        [0x00, 0x00] => {
            let Some(length) = message.get(3..7).and_then(|value| {
                usize::try_from(u32::from_be_bytes([value[0], value[1], value[2], value[3]])).ok()
            }) else {
                return;
            };
            let Some(body) = message.get(7..7 + length) else {
                return;
            };
            let table_count = usize::from(*body.first().unwrap_or(&0));
            body.get(1 + table_count.saturating_mul(4)..)
        }
        _ => None,
    };
    let Some(mut table) = table else {
        return;
    };
    while let Some(header) = table.get(..4) {
        let length = usize::from(u16::from_be_bytes([header[2], header[3]]));
        let Some(next) = table.get(..4 + length) else {
            return;
        };
        if let Some(assets) = parse_mpt_assets(next) {
            for asset in assets {
                diagnostics
                    .mpt_assets
                    .insert(asset.packet_id, asset.asset_type);
                diagnostics
                    .mpt_asset_offsets
                    .entry(asset.packet_id)
                    .or_insert(diagnostics.current_source_offset);
                diagnostics
                    .mpt_descriptor_tags
                    .insert(asset.packet_id, asset.descriptor_tags);
                diagnostics.mpt_presentation_ntp.extend(
                    asset
                        .presentation_ntp
                        .into_iter()
                        .map(|(sequence, ntp)| ((asset.packet_id, sequence), ntp)),
                );
            }
        }
        table = &table[4 + length..];
    }
}

pub(crate) fn assemble_signalling_fragment(
    assemblers: &mut BTreeMap<u16, SignallingFragmentAssembler>,
    packet_id: u16,
    sequence_number: u32,
    fragmentation_indicator: u8,
    bytes: &[u8],
    diagnostics: &mut TlvDiagnostics,
) -> Option<Vec<u8>> {
    if bytes.len() > SIGNAL_FRAGMENT_BUFFER_LIMIT {
        diagnostics.signalling_fragments_dropped += 1;
        return None;
    }
    if !assemblers.contains_key(&packet_id) && assemblers.len() >= SIGNAL_FRAGMENT_ASSEMBLER_LIMIT {
        diagnostics.signalling_fragments_dropped += 1;
        return None;
    }
    let assembler = assemblers.entry(packet_id).or_default();
    let sequence_is_contiguous =
        assembler.in_fragment && sequence_number == assembler.last_sequence_number.wrapping_add(1);
    match fragmentation_indicator {
        // A complete message invalidates an unfinished older message, rather than
        // blending two signalling generations together.
        0b00 => {
            if assembler.in_fragment {
                diagnostics.signalling_fragments_dropped += 1;
                assembler.bytes.clear();
                assembler.in_fragment = false;
            }
            Some(bytes.to_vec())
        }
        0b01 => {
            assembler.bytes.clear();
            assembler.bytes.extend_from_slice(bytes);
            assembler.in_fragment = true;
            assembler.last_sequence_number = sequence_number;
            Some(Vec::new())
        }
        0b10 => {
            if !sequence_is_contiguous
                || assembler.bytes.len().saturating_add(bytes.len()) > SIGNAL_FRAGMENT_BUFFER_LIMIT
            {
                diagnostics.signalling_fragments_dropped += 1;
                assembler.bytes.clear();
                assembler.in_fragment = false;
                return None;
            }
            assembler.bytes.extend_from_slice(bytes);
            assembler.last_sequence_number = sequence_number;
            Some(Vec::new())
        }
        0b11 => {
            if !sequence_is_contiguous
                || assembler.bytes.len().saturating_add(bytes.len()) > SIGNAL_FRAGMENT_BUFFER_LIMIT
            {
                diagnostics.signalling_fragments_dropped += 1;
                assembler.bytes.clear();
                assembler.in_fragment = false;
                return None;
            }
            assembler.bytes.extend_from_slice(bytes);
            assembler.last_sequence_number = sequence_number;
            assembler.in_fragment = false;
            diagnostics.signalling_fragments_reassembled += 1;
            Some(std::mem::take(&mut assembler.bytes))
        }
        _ => None,
    }
}
