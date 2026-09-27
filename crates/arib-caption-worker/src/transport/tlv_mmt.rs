use crate::*;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

pub(crate) struct SliceCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> SliceCursor<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    pub(crate) fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let end = self.position.checked_add(length)?;
        let value = self.bytes.get(self.position..end)?;
        self.position = end;
        Some(value)
    }

    pub(crate) fn u8(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }

    pub(crate) fn be_u16(&mut self) -> Option<u16> {
        let value = self.take(2)?;
        Some(u16::from_be_bytes([value[0], value[1]]))
    }

    pub(crate) fn be_u32(&mut self) -> Option<u32> {
        let value = self.take(4)?;
        Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
    }
}

pub(crate) struct MmtpPacket<'a> {
    pub(crate) packet_id: u16,
    pub(crate) sequence_number: u32,
    pub(crate) payload_type: u8,
    pub(crate) payload: &'a [u8],
}

pub(crate) fn parse_mmtp_packet(bytes: &[u8]) -> Option<MmtpPacket<'_>> {
    let mut cursor = SliceCursor::new(bytes);
    let flags = cursor.u8()?;
    let packet_counter_present = flags & 0x20 != 0;
    let extension_present = flags & 0x02 != 0;
    let payload_type = cursor.u8()? & 0x3f;
    let packet_id = cursor.be_u16()?;
    cursor.take(4)?;
    let sequence_number = cursor.be_u32()?;
    if packet_counter_present {
        cursor.take(4)?;
    }
    if extension_present {
        cursor.take(2)?;
        let extension_length = usize::from(cursor.be_u16()?);
        cursor.take(extension_length)?;
    }
    Some(MmtpPacket {
        packet_id,
        sequence_number,
        payload_type,
        payload: cursor.take(cursor.remaining())?,
    })
}

pub(crate) fn tlv_mmtp_payload(packet_type: u8, payload: &[u8]) -> Option<&[u8]> {
    match packet_type {
        0x02 => direct_ipv6_udp_payload(payload).map(|(_, bytes)| bytes),
        0x03 => {
            let context = *payload.get(2)?;
            match context {
                0x61 => payload.get(3..),
                0x60 => payload.get(3 + 38 + 4..),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(crate) fn direct_ipv6_udp_payload(payload: &[u8]) -> Option<(u16, &[u8])> {
    if payload.len() < 48 || payload[0] >> 4 != 6 || payload[6] != 17 {
        return None;
    }
    let declared_length = usize::from(u16::from_be_bytes([payload[4], payload[5]]));
    if declared_length < 8 || payload.len() < 40 + declared_length {
        return None;
    }
    let destination = u16::from_be_bytes([payload[42], payload[43]]);
    Some((destination, payload.get(48..40 + declared_length)?))
}

pub(crate) fn direct_ipv6_udp_destination_port(payload: &[u8]) -> Option<u16> {
    direct_ipv6_udp_payload(payload).map(|(port, _)| port)
}

pub(crate) fn skip_mmt_general_location(cursor: &mut SliceCursor<'_>) -> Option<Option<u16>> {
    match cursor.u8()? {
        0x00 => Some(Some(cursor.be_u16()?)),
        0x01 => {
            cursor.take(12)?;
            Some(None)
        }
        0x02 => {
            cursor.take(36)?;
            Some(None)
        }
        0x03 => {
            cursor.take(6)?;
            Some(None)
        }
        0x04 => {
            cursor.take(36)?;
            Some(None)
        }
        0x05 => {
            let length = usize::from(cursor.u8()?);
            cursor.take(length)?;
            Some(None)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimum_mmtp_header_without_guessing_payload() {
        let packet = [0x00, 0x00, 0x12, 0x34, 0, 0, 0, 0, 0, 0, 0, 7, 0xaa, 0xbb];
        let parsed = parse_mmtp_packet(&packet).expect("valid MMTP header");
        assert_eq!(parsed.packet_id, 0x1234);
        assert_eq!(parsed.sequence_number, 7);
        assert_eq!(parsed.payload, &[0xaa, 0xbb]);
    }

    #[test]
    fn rejects_non_udp_ipv6_payload() {
        let mut payload = vec![0u8; 48];
        payload[0] = 0x60;
        payload[6] = 6;
        assert!(direct_ipv6_udp_payload(&payload).is_none());
    }
}

mod signalling;
pub(crate) use signalling::*;

pub(crate) struct MpuMfu<'a> {
    mpu_sequence_number: u32,
    timed: bool,
    fragmentation_indicator: u8,
    bytes: &'a [u8],
}

#[derive(Debug, Clone)]
pub(crate) struct TlvCaptionPayload {
    pub(crate) packet_id: u16,
    pub(crate) mpu_sequence_number: Option<u32>,
    pub(crate) mmtp_sequence_number: Option<u32>,
    pub(crate) presentation_ntp: Option<u64>,
    pub(crate) timed: Option<bool>,
    pub(crate) bytes: Vec<u8>,
    pub(crate) resources: Vec<TlvSubtitleResource>,
    pub(crate) resources_complete: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TlvResourcePayload {
    pub(crate) packet_id: u16,
    pub(crate) asset_type: String,
    pub(crate) mpu_sequence_number: u32,
    pub(crate) mmtp_sequence_number: u32,
    pub(crate) presentation_ntp: Option<u64>,
    pub(crate) timed: bool,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TlvAssetEvidence {
    pub(crate) packet_id: u16,
    pub(crate) source_offset: u64,
    pub(crate) asset_type: String,
    pub(crate) descriptor_tags: Vec<u16>,
    pub(crate) presentation_ntp: BTreeMap<u32, u64>,
    pub(crate) payload_route: &'static str,
}

#[cfg_attr(feature = "libaribtlv", allow(dead_code))]
pub(crate) fn tlv_asset_evidence(diagnostics: &TlvDiagnostics) -> Vec<TlvAssetEvidence> {
    diagnostics
        .mpt_assets
        .iter()
        .map(|(packet_id, asset_type)| TlvAssetEvidence {
            packet_id: *packet_id,
            source_offset: diagnostics
                .mpt_asset_offsets
                .get(packet_id)
                .copied()
                .unwrap_or_default(),
            asset_type: asset_type.clone(),
            descriptor_tags: diagnostics
                .mpt_descriptor_tags
                .get(packet_id)
                .cloned()
                .unwrap_or_default(),
            presentation_ntp: diagnostics
                .mpt_presentation_ntp
                .iter()
                .filter_map(|((id, sequence), value)| {
                    (*id == *packet_id).then_some((*sequence, *value))
                })
                .collect(),
            payload_route: "mpt-signalling-observed; payload bytes require a validated asset extractor",
        })
        .collect()
}

pub(crate) fn mfu_payload(bytes: &[u8], timed: bool) -> Option<&[u8]> {
    let mut cursor = SliceCursor::new(bytes);
    cursor.take(if timed { 14 } else { 4 })?;
    cursor.take(cursor.remaining())
}

pub(crate) fn parse_mpu_mfus(payload: &[u8]) -> Option<Vec<MpuMfu<'_>>> {
    let mut cursor = SliceCursor::new(payload);
    let declared_length = usize::from(cursor.be_u16()?);
    if declared_length != cursor.remaining() {
        return None;
    }
    let header = cursor.u8()?;
    let fragment_type = header >> 4;
    let timed = header & 0x08 != 0;
    let fragmentation_indicator = (header >> 1) & 0x03;
    let aggregated = header & 0x01 != 0;
    cursor.take(1)?; // fragment counter
    let mpu_sequence_number = cursor.be_u32()?;
    if fragment_type != 0x02 || (aggregated && fragmentation_indicator != 0) {
        return None;
    }
    let mut mfus = Vec::new();
    if !aggregated {
        let bytes = mfu_payload(cursor.take(cursor.remaining())?, timed)?;
        mfus.push(MpuMfu {
            mpu_sequence_number,
            timed,
            fragmentation_indicator,
            bytes,
        });
        return Some(mfus);
    }
    while cursor.remaining() > 0 {
        let length = usize::from(cursor.be_u16()?);
        let bytes = mfu_payload(cursor.take(length)?, timed)?;
        mfus.push(MpuMfu {
            mpu_sequence_number,
            timed,
            fragmentation_indicator: 0,
            bytes,
        });
    }
    Some(mfus)
}

pub(crate) fn assemble_mpu_fragment(
    assemblers: &mut BTreeMap<(u16, u32), MpuFragmentAssembler>,
    packet_id: u16,
    mpu_sequence_number: u32,
    packet_sequence_number: u32,
    fragmentation_indicator: u8,
    bytes: &[u8],
    diagnostics: &mut TlvDiagnostics,
) -> Option<Vec<u8>> {
    if bytes.len() > MPU_FRAGMENT_BUFFER_LIMIT {
        diagnostics.stpp_mfu_dropped += 1;
        return None;
    }
    let key = (packet_id, mpu_sequence_number);
    if !assemblers.contains_key(&key) && assemblers.len() >= MPU_FRAGMENT_ASSEMBLER_LIMIT {
        diagnostics.stpp_mfu_dropped += 1;
        return None;
    }
    let assembler = assemblers.entry(key).or_default();
    let contiguous = assembler.in_fragment
        && packet_sequence_number == assembler.last_sequence_number.wrapping_add(1);
    match fragmentation_indicator {
        0b00 => {
            if assembler.in_fragment {
                diagnostics.stpp_mfu_dropped += 1;
                assembler.in_fragment = false;
                assembler.bytes.clear();
            }
            Some(bytes.to_vec())
        }
        0b01 => {
            assembler.bytes.clear();
            assembler.bytes.extend_from_slice(bytes);
            assembler.in_fragment = true;
            assembler.last_sequence_number = packet_sequence_number;
            Some(Vec::new())
        }
        0b10 => {
            if !contiguous
                || assembler.bytes.len().saturating_add(bytes.len()) > MPU_FRAGMENT_BUFFER_LIMIT
            {
                diagnostics.stpp_mfu_dropped += 1;
                assembler.in_fragment = false;
                assembler.bytes.clear();
                return None;
            }
            assembler.bytes.extend_from_slice(bytes);
            assembler.last_sequence_number = packet_sequence_number;
            Some(Vec::new())
        }
        0b11 => {
            if !contiguous
                || assembler.bytes.len().saturating_add(bytes.len()) > MPU_FRAGMENT_BUFFER_LIMIT
            {
                diagnostics.stpp_mfu_dropped += 1;
                assembler.in_fragment = false;
                assembler.bytes.clear();
                return None;
            }
            assembler.bytes.extend_from_slice(bytes);
            assembler.in_fragment = false;
            Some(std::mem::take(&mut assembler.bytes))
        }
        _ => None,
    }
}

pub(crate) struct ParsedSubtitleMfu<'a> {
    pub(crate) subsample_number: u8,
    pub(crate) last_subsample_number: u8,
    pub(crate) data_type: u8,
    pub(crate) payload: &'a [u8],
}

pub(crate) fn parse_subtitle_mfu_payload(mfu: &[u8]) -> Option<ParsedSubtitleMfu<'_>> {
    let mut cursor = SliceCursor::new(mfu);
    cursor.take(2)?; // subtitle tag and subtitle sequence number
    let subsample_number = cursor.u8()?;
    let last_subsample_number = cursor.u8()?;
    let flags = cursor.u8()?;
    let data_type = flags >> 4;
    let length_extended = flags & 0x08 != 0;
    let subsample_info_present = flags & 0x04 != 0;
    let data_size = if length_extended {
        usize::try_from(cursor.be_u32()?).ok()?
    } else {
        usize::from(cursor.be_u16()?)
    };
    if subsample_number == 0 && last_subsample_number > 0 && subsample_info_present {
        for _ in 0..last_subsample_number {
            cursor.take(1)?;
            cursor.take(if length_extended { 4 } else { 2 })?;
        }
    }
    Some(ParsedSubtitleMfu {
        subsample_number,
        last_subsample_number,
        data_type,
        payload: cursor.take(data_size)?,
    })
}

pub(crate) fn inspect_stpp_mpu(
    packet: &MmtpPacket<'_>,
    diagnostics: &mut TlvDiagnostics,
    assemblers: &mut BTreeMap<(u16, u32), MpuFragmentAssembler>,
) -> Vec<TlvCaptionPayload> {
    let mut payloads = Vec::new();
    if diagnostics
        .mpt_assets
        .get(&packet.packet_id)
        .map(String::as_str)
        != Some("stpp")
    {
        return payloads;
    }
    let Some(mfus) = parse_mpu_mfus(packet.payload) else {
        diagnostics.stpp_mfu_dropped += 1;
        return payloads;
    };
    for mfu in mfus {
        diagnostics.stpp_mfu_fragments += 1;
        if let Some(completed) = assemble_mpu_fragment(
            assemblers,
            packet.packet_id,
            mfu.mpu_sequence_number,
            packet.sequence_number,
            mfu.fragmentation_indicator,
            mfu.bytes,
            diagnostics,
        ) && !completed.is_empty()
        {
            let Some(parsed) = parse_subtitle_mfu_payload(&completed) else {
                diagnostics.stpp_mfu_dropped += 1;
                continue;
            };
            let state_key = (packet.packet_id, mfu.mpu_sequence_number);
            const RESOURCE_STATE_LIMIT: usize = 128;
            diagnostics
                .subtitle_resources
                .retain(|(packet_id, sequence), _| {
                    *packet_id != packet.packet_id
                        || sequence.saturating_add(4) >= mfu.mpu_sequence_number
                });
            if !diagnostics.subtitle_resources.contains_key(&state_key)
                && diagnostics.subtitle_resources.len() >= RESOURCE_STATE_LIMIT
                && let Some(oldest) = diagnostics.subtitle_resources.keys().next().copied()
            {
                diagnostics.subtitle_resources.remove(&oldest);
            }
            if parsed.subsample_number != 0 {
                diagnostics
                    .subtitle_resources
                    .entry(state_key)
                    .or_default()
                    .add(TlvSubtitleResource {
                        index: parsed.subsample_number,
                        data_type: parsed.data_type,
                        bytes: parsed.payload.to_vec(),
                    });
                continue;
            }
            if parsed.data_type != 0 {
                diagnostics.stpp_mfu_dropped += 1;
                continue;
            }
            let (resources, resources_complete) = {
                let resource_state = diagnostics.subtitle_resources.entry(state_key).or_default();
                resource_state.last_subsample_number = resource_state
                    .last_subsample_number
                    .max(parsed.last_subsample_number);
                (
                    resource_state.resources.values().cloned().collect(),
                    resource_state.is_complete(parsed.last_subsample_number),
                )
            };
            if resources_complete {
                diagnostics.subtitle_resources.remove(&state_key);
            }
            diagnostics.stpp_mfu_completed += 1;
            diagnostics.stpp_payload_bytes += parsed.payload.len() as u64;
            payloads.push(TlvCaptionPayload {
                packet_id: packet.packet_id,
                mpu_sequence_number: Some(mfu.mpu_sequence_number),
                mmtp_sequence_number: Some(packet.sequence_number),
                presentation_ntp: diagnostics
                    .mpt_presentation_ntp
                    .get(&(packet.packet_id, mfu.mpu_sequence_number))
                    .copied(),
                timed: Some(mfu.timed),
                bytes: parsed.payload.to_vec(),
                resources,
                resources_complete,
            });
        }
    }
    payloads
}
pub(crate) fn inspect_non_stpp_mpu(
    packet: &MmtpPacket<'_>,
    diagnostics: &mut TlvDiagnostics,
    assemblers: &mut BTreeMap<(u16, u32), MpuFragmentAssembler>,
) -> Vec<TlvResourcePayload> {
    let mut payloads = Vec::new();
    let Some(asset_type) = diagnostics.mpt_assets.get(&packet.packet_id).cloned() else {
        return payloads;
    };
    if asset_type == "stpp" {
        return payloads;
    }
    let Some(mfus) = parse_mpu_mfus(packet.payload) else {
        return payloads;
    };
    for mfu in mfus {
        if let Some(bytes) = assemble_mpu_fragment(
            assemblers,
            packet.packet_id,
            mfu.mpu_sequence_number,
            packet.sequence_number,
            mfu.fragmentation_indicator,
            mfu.bytes,
            diagnostics,
        ) && !bytes.is_empty()
        {
            diagnostics.non_stpp_mfu_completed += 1;
            diagnostics.non_stpp_payload_bytes += bytes.len() as u64;
            payloads.push(TlvResourcePayload {
                packet_id: packet.packet_id,
                asset_type: asset_type.clone(),
                mpu_sequence_number: mfu.mpu_sequence_number,
                mmtp_sequence_number: packet.sequence_number,
                presentation_ntp: diagnostics
                    .mpt_presentation_ntp
                    .get(&(packet.packet_id, mfu.mpu_sequence_number))
                    .copied(),
                timed: mfu.timed,
                bytes,
            });
        }
    }
    payloads
}

pub(crate) fn inspect_mmtp_packet(
    packet: &MmtpPacket<'_>,
    diagnostics: &mut TlvDiagnostics,
    assemblers: &mut BTreeMap<u16, SignallingFragmentAssembler>,
    mpu_assemblers: &mut BTreeMap<(u16, u32), MpuFragmentAssembler>,
    captured_payloads: Option<&mut Vec<TlvCaptionPayload>>,
    captured_assets: Option<&mut Vec<TlvResourcePayload>>,
) {
    diagnostics.mmtp_packets += 1;
    *diagnostics
        .mmtp_packet_ids
        .entry(packet.packet_id)
        .or_default() += 1;
    diagnostics
        .mmtp_sequences
        .insert(packet.packet_id, packet.sequence_number);
    *diagnostics
        .mmtp_payload_types
        .entry(packet.payload_type)
        .or_default() += 1;
    if packet.payload_type == 0x00 {
        let payloads = inspect_stpp_mpu(packet, diagnostics, mpu_assemblers);
        if let Some(captured_payloads) = captured_payloads {
            captured_payloads.extend(payloads);
        }
        if let Some(captured_assets) = captured_assets {
            captured_assets.extend(inspect_non_stpp_mpu(packet, diagnostics, mpu_assemblers));
        }
        return;
    }
    if packet.payload_type != 0x02 || packet.payload.len() < 2 {
        return;
    }
    let flags = packet.payload[0];
    let fragmentation_indicator = flags >> 6;
    let aggregated = flags & 1 != 0;
    if aggregated && fragmentation_indicator != 0 {
        diagnostics.signalling_fragments_dropped += 1;
        return;
    }
    let messages = &packet.payload[2..];
    if !aggregated {
        if let Some(message) = assemble_signalling_fragment(
            assemblers,
            packet.packet_id,
            packet.sequence_number,
            fragmentation_indicator,
            messages,
            diagnostics,
        ) && !message.is_empty()
        {
            inspect_signalling_message(&message, diagnostics);
        }
        return;
    }
    let length_is_32_bit = flags & 0x02 != 0;
    let mut cursor = SliceCursor::new(messages);
    while cursor.remaining() > 0 {
        let length = if length_is_32_bit {
            usize::try_from(cursor.be_u32().unwrap_or(0)).unwrap_or(0)
        } else {
            usize::from(cursor.be_u16().unwrap_or(0))
        };
        let Some(message) = cursor.take(length) else {
            return;
        };
        inspect_signalling_message(message, diagnostics);
    }
}

pub(crate) fn scan_tlv_diagnostics(path: &Path, start: usize) -> io::Result<TlvDiagnostics> {
    let mut reader = BufReader::with_capacity(1024 * 1024, crate::input::open_input(path)?);
    let mut bytes = vec![0; PSI_SCAN_BYTES];
    let length = reader.read(&mut bytes)?;
    bytes.truncate(length);
    let mut diagnostics = TlvDiagnostics::default();
    let mut signalling_assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    let mut offset = start;
    while let Some(header) = bytes.get(offset..offset.saturating_add(4)) {
        if header[0] != 0x7f {
            break;
        }
        let length = usize::from(u16::from_be_bytes([header[2], header[3]]));
        let Some(end) = offset.checked_add(4 + length) else {
            break;
        };
        let Some(payload) = bytes.get(offset + 4..end) else {
            break;
        };
        diagnostics.packets += 1;
        diagnostics.payload_bytes += payload.len() as u64;
        *diagnostics.types.entry(header[1]).or_default() += 1;
        if let Some(port) = direct_ipv6_udp_destination_port(payload) {
            diagnostics.ipv6_packets += 1;
            *diagnostics.udp_ports.entry(port).or_default() += 1;
        }
        if let Some(mmtp) = tlv_mmtp_payload(header[1], payload)
            && let Some(packet) = parse_mmtp_packet(mmtp)
        {
            diagnostics.current_source_offset = offset as u64;
            inspect_mmtp_packet(
                &packet,
                &mut diagnostics,
                &mut signalling_assemblers,
                &mut mpu_assemblers,
                None,
                None,
            );
        }
        offset = end;
    }
    Ok(diagnostics)
}

pub(crate) fn read_tlv_packet(
    reader: &mut BufReader<impl Read>,
    offset: &mut u64,
) -> io::Result<Option<(u8, Vec<u8>, u64)>> {
    let packet_offset = *offset;
    let mut header = [0_u8; 4];
    let first_read = reader.read(&mut header)?;
    if first_read == 0 {
        return Ok(None);
    }
    reader.read_exact(&mut header[first_read..])?;
    if header[0] != 0x7f {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("TLV sync byte missing at offset {packet_offset}"),
        ));
    }
    let length = usize::from(u16::from_be_bytes([header[2], header[3]]));
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    *offset = offset.saturating_add(4 + length as u64);
    Ok(Some((header[1], payload, packet_offset)))
}

mod evidence;
pub(crate) use evidence::*;
mod route;
pub(crate) use route::*;
