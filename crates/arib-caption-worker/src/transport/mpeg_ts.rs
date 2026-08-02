use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use crate::{
    B24DecodeSummary, B24Track, BroadcastMetadata, DataTracks, InputProbe, PROBE_BYTES,
    PSI_SCAN_BYTES, TtmlCaption,
    caption::ttml::{TsFraming, scan_ts_ttml_impl},
    probe_bytes,
};

// MPEG-2 private sections used by ARIB SI (notably EIT) may carry a
// 12-bit section_length up to 4093 bytes. PAT/PMT are smaller, but applying
// their 1021-byte limit globally caused valid programme metadata to vanish.
const PSI_SECTION_LIMIT: usize = 4096;
const SI_SCAN_BYTES: u64 = 64 * 1024 * 1024;
const DYNAMIC_SI_SCAN_BYTES: u64 = 24 * 1024 * 1024;
const PSI_SAMPLE_WINDOW_BYTES: u64 = 1024 * 1024;
const PSI_SAMPLE_WINDOW_COUNT: u64 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum B24TextServiceKind {
    Caption,
    Superimpose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct B24CaptionStream {
    pub(crate) pid: u16,
    pub(crate) component_tag: u8,
    pub(crate) language: Option<String>,
}

/// Reassemble one bounded PSI section for a single PID during stream discovery.
/// PSI is not retained after discovery: this deliberately caps malformed input
/// before it can turn a short inspection scan into unbounded allocation.
#[derive(Default)]
pub(crate) struct PsiAssembler {
    buffer: Vec<u8>,
    expected_len: Option<usize>,
}

impl PsiAssembler {
    pub(crate) fn push(&mut self, payload: &[u8], payload_start: bool) -> Option<Vec<u8>> {
        self.push_all(payload, payload_start).into_iter().next()
    }

    pub(crate) fn push_all(&mut self, payload: &[u8], payload_start: bool) -> Vec<Vec<u8>> {
        let mut sections = Vec::new();
        let bytes = if payload_start {
            let Some(first) = payload.first() else {
                return sections;
            };
            let pointer = usize::from(*first);
            let Some(split) = 1usize.checked_add(pointer) else {
                self.reset();
                return sections;
            };
            let Some(before) = payload.get(1..split) else {
                self.reset();
                return sections;
            };
            if !self.buffer.is_empty() {
                self.feed(before, &mut sections);
            }
            // A payload-unit start marks a new section after the pointer. Any
            // incomplete previous section is malformed and cannot consume it.
            self.reset();
            let Some(bytes) = payload.get(split..) else {
                return sections;
            };
            bytes
        } else {
            if self.buffer.is_empty() {
                return sections;
            }
            payload
        };
        self.feed(bytes, &mut sections);
        sections
    }

    fn feed(&mut self, bytes: &[u8], sections: &mut Vec<Vec<u8>>) {
        for &byte in bytes {
            if self.buffer.is_empty() && byte == 0xff {
                break;
            }
            if self.buffer.len() >= PSI_SECTION_LIMIT {
                self.reset();
            }
            self.buffer.push(byte);
            if self.buffer.len() == 3 {
                let length =
                    (usize::from(self.buffer[1] & 0x0f) << 8) | usize::from(self.buffer[2]);
                let expected = length.saturating_add(3);
                if !(7..=PSI_SECTION_LIMIT).contains(&expected) {
                    self.reset();
                    continue;
                }
                self.expected_len = Some(expected);
            }
            if self.expected_len == Some(self.buffer.len()) {
                sections.push(std::mem::take(&mut self.buffer));
                self.expected_len = None;
            }
        }
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.expected_len = None;
    }
}

pub(crate) fn probe_path(path: &Path) -> io::Result<InputProbe> {
    let mut file = File::open(path)?;
    let mut bytes = vec![0; PROBE_BYTES];
    let length = file.read(&mut bytes)?;
    bytes.truncate(length);
    Ok(probe_bytes(&bytes))
}

pub(crate) fn ts_payload(packet: &[u8]) -> Option<(u16, bool, &[u8])> {
    if packet.len() != 188 || packet[0] != 0x47 {
        return None;
    }
    let pid = (u16::from(packet[1] & 0x1f) << 8) | u16::from(packet[2]);
    let payload_start = packet[1] & 0x40 != 0;
    let control = (packet[3] >> 4) & 0x03;
    if control == 0 || control == 2 {
        return None;
    }
    let mut offset = 4;
    if control == 3 {
        offset += 1 + usize::from(*packet.get(offset)?);
    }
    // Adaptation-field length is broadcast input and may be corrupt. Do not
    // construct a slice until the declared boundary has been checked.
    let payload = packet.get(offset..)?;
    (!payload.is_empty()).then_some((pid, payload_start, payload))
}

// Kept as a stateless parser for the isolated fuzz target. Production discovery
// uses `PsiAssembler` so PSI may span recorder packet boundaries.
#[allow(dead_code)]
pub(crate) fn psi_section(payload: &[u8]) -> Option<&[u8]> {
    let pointer = usize::from(*payload.first()?);
    let section = payload.get(1 + pointer..)?;
    let length = (usize::from(section.get(1)? & 0x0f) << 8) | usize::from(*section.get(2)?);
    if !(4..PSI_SECTION_LIMIT).contains(&length) {
        return None;
    }
    section.get(..length + 3)
}

pub(crate) fn first_pmt_pid(section: &[u8]) -> Option<u16> {
    if section.len() < 12 || section[0] != 0x00 {
        return None;
    }
    for entry in section[8..section.len() - 4].chunks_exact(4) {
        if entry[0] != 0 || entry[1] != 0 {
            return Some((u16::from(entry[2] & 0x1f) << 8) | u16::from(entry[3]));
        }
    }
    None
}

pub(crate) fn pmt_programs(section: &[u8]) -> Vec<(u16, u16)> {
    if section.len() < 12 || section[0] != 0x00 {
        return Vec::new();
    }
    section[8..section.len().saturating_sub(4)]
        .chunks_exact(4)
        .filter_map(|entry| {
            let program = u16::from_be_bytes([entry[0], entry[1]]);
            (program != 0).then_some((
                program,
                (u16::from(entry[2] & 0x1f) << 8) | u16::from(entry[3]),
            ))
        })
        .collect()
}

fn descriptor_component_tag(descriptors: &[u8]) -> Option<u8> {
    let mut index = 0;
    while index + 2 <= descriptors.len() {
        let tag = descriptors[index];
        let length = usize::from(descriptors[index + 1]);
        let body = descriptors.get(index + 2..index + 2 + length)?;
        if tag == 0x52 && body.len() == 1 {
            return body.first().copied();
        }
        index += 2 + length;
    }
    None
}

fn has_b24_data_component_descriptor(descriptors: &[u8]) -> bool {
    let mut index = 0;
    while index + 2 <= descriptors.len() {
        let tag = descriptors[index];
        let length = usize::from(descriptors[index + 1]);
        let Some(body) = descriptors.get(index + 2..index + 2 + length) else {
            return false;
        };
        if tag == 0xfd && body.starts_with(&[0x00, 0x08]) {
            return true;
        }
        index += 2 + length;
    }
    false
}

pub(crate) fn b24_text_service_kind(descriptors: &[u8]) -> Option<B24TextServiceKind> {
    match descriptor_component_tag(descriptors)? {
        0x30..=0x37 => Some(B24TextServiceKind::Caption),
        0x38..=0x3f => Some(B24TextServiceKind::Superimpose),
        _ => None,
    }
}

pub(crate) fn b24_descriptor(descriptors: &[u8]) -> bool {
    b24_text_service_kind(descriptors) == Some(B24TextServiceKind::Caption)
        && has_b24_data_component_descriptor(descriptors)
}

pub(crate) fn iso639_language(descriptors: &[u8]) -> Option<String> {
    let mut index = 0;
    while index + 2 <= descriptors.len() {
        let tag = descriptors[index];
        let length = usize::from(descriptors[index + 1]);
        let body = descriptors.get(index + 2..index + 2 + length)?;
        if tag == 0x0a && body.len() >= 3 {
            return String::from_utf8(body[..3].to_vec()).ok();
        }
        index += 2 + length;
    }
    None
}

pub(crate) fn service_name_from_sdt(section: &[u8], service_id: u16) -> Option<String> {
    if !matches!(section.first(), Some(0x42 | 0x46)) || section.len() < 15 {
        return None;
    }
    let mut index = 11;
    let end = section.len().checked_sub(4)?;
    while index + 5 <= end {
        let current = u16::from_be_bytes([section[index], section[index + 1]]);
        let loop_length =
            (usize::from(section[index + 3] & 0x0f) << 8) | usize::from(section[index + 4]);
        let descriptors = section.get(index + 5..index + 5 + loop_length)?;
        if current == service_id {
            let mut cursor = 0;
            while cursor + 2 <= descriptors.len() {
                let tag = descriptors[cursor];
                let length = usize::from(descriptors[cursor + 1]);
                let body = descriptors.get(cursor + 2..cursor + 2 + length)?;
                if tag == 0x48 && body.len() >= 3 {
                    let provider_len = usize::from(body[1]);
                    let name_len_index = 2 + provider_len;
                    if body.len() > name_len_index {
                        let name_len = usize::from(body[name_len_index]);
                        let name = body.get(name_len_index + 1..name_len_index + 1 + name_len)?;
                        return crate::arib_text::decode_service_name(name);
                    }
                }
                cursor += 2 + length;
            }
        }
        index += 5 + loop_length;
    }
    None
}

fn descriptor_body(descriptors: &[u8], wanted_tag: u8) -> Option<&[u8]> {
    let mut index = 0;
    while index + 2 <= descriptors.len() {
        let tag = descriptors[index];
        let length = usize::from(descriptors[index + 1]);
        let body = descriptors.get(index + 2..index + 2 + length)?;
        if tag == wanted_tag {
            return Some(body);
        }
        index += 2 + length;
    }
    None
}

pub(crate) fn network_name_from_nit(section: &[u8]) -> Option<String> {
    if section.first() != Some(&0x40) || section.len() < 14 {
        return None;
    }
    let descriptors_length = (usize::from(section[8] & 0x0f) << 8) | usize::from(section[9]);
    let descriptors = section.get(10..10 + descriptors_length)?;
    crate::arib_text::decode_service_name(descriptor_body(descriptors, 0x40)?)
}

pub(crate) fn programme_from_eit(
    section: &[u8],
    preferred_service_id: Option<u16>,
) -> Option<(String, Option<String>)> {
    if !matches!(section.first(), Some(0x4e | 0x4f)) || section.len() < 30 {
        return None;
    }
    let service_id = u16::from_be_bytes([section[3], section[4]]);
    if preferred_service_id.is_some_and(|preferred| preferred != service_id) {
        return None;
    }
    // Present/following uses section zero for present and section one for
    // following. Other section numbers cannot identify the current event.
    if section[6] > 1 {
        return None;
    }
    let mut index = 14;
    let end = section.len().checked_sub(4)?;
    let mut fallback = None;
    while index + 12 <= end {
        let event = section.get(index..end)?;
        let descriptors_length = (usize::from(event[10] & 0x0f) << 8) | usize::from(event[11]);
        let next = index.checked_add(12 + descriptors_length)?;
        let descriptors = section.get(index + 12..next)?;
        if let Some(short_event) = descriptor_body(descriptors, 0x4d)
            && short_event.len() >= 5
        {
            let name_length = usize::from(short_event[3]);
            let name = short_event.get(4..4 + name_length)?;
            let description_length = usize::from(*short_event.get(4 + name_length)?);
            let description =
                short_event.get(5 + name_length..5 + name_length + description_length)?;
            if let Some(name) = crate::arib_text::decode_service_name(name) {
                let decoded = (
                    name,
                    crate::arib_text::decode_service_name(description)
                        .filter(|value| !value.is_empty()),
                );
                let running_status = (event[10] >> 5) & 0x07;
                if section[6] == 0 && matches!(running_status, 2..=4) {
                    return Some(decoded);
                }
                fallback.get_or_insert(decoded);
            }
        }
        index = next;
    }
    fallback
}

fn bcd(value: u8) -> Option<u8> {
    let high = value >> 4;
    let low = value & 0x0f;
    (high <= 9 && low <= 9).then_some(high * 10 + low)
}

fn civil_date_from_unix_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

pub(crate) fn utc_time_from_tdt_tot(section: &[u8]) -> Option<String> {
    if !matches!(section.first(), Some(0x70 | 0x73)) || section.len() < 8 {
        return None;
    }
    let mjd = i64::from(u16::from_be_bytes([section[3], section[4]]));
    let hour = bcd(section[5])?;
    let minute = bcd(section[6])?;
    let second = bcd(section[7])?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let (year, month, day) = civil_date_from_unix_days(mjd - 40_587);
    Some(format!(
        "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC"
    ))
}

pub(crate) fn discover_broadcast_metadata(
    path: &Path,
    packet_size: usize,
    ts_offset: usize,
    preferred_service_id: Option<u16>,
) -> io::Result<BroadcastMetadata> {
    scan_broadcast_metadata(
        path,
        packet_size,
        ts_offset,
        preferred_service_id,
        0,
        SI_SCAN_BYTES,
    )
}

pub(crate) fn discover_broadcast_metadata_at(
    path: &Path,
    packet_size: usize,
    ts_offset: usize,
    preferred_service_id: Option<u16>,
    byte_offset: u64,
) -> io::Result<BroadcastMetadata> {
    let packet_size_u64 = packet_size as u64;
    let aligned_offset = byte_offset / packet_size_u64 * packet_size_u64;
    scan_broadcast_metadata(
        path,
        packet_size,
        ts_offset,
        preferred_service_id,
        aligned_offset,
        DYNAMIC_SI_SCAN_BYTES,
    )
}

fn scan_broadcast_metadata(
    path: &Path,
    packet_size: usize,
    ts_offset: usize,
    preferred_service_id: Option<u16>,
    start_offset: u64,
    scan_bytes: u64,
) -> io::Result<BroadcastMetadata> {
    let mut file = BufReader::with_capacity(64 * 1024, File::open(path)?);
    file.seek(SeekFrom::Start(start_offset))?;
    let mut packet = vec![0; packet_size];
    let mut bytes_read = 0_u64;
    let mut metadata = BroadcastMetadata::default();
    let mut psi = HashMap::<u16, PsiAssembler>::new();
    while bytes_read.saturating_add(packet_size as u64) <= scan_bytes {
        match file.read_exact(&mut packet) {
            Ok(()) => bytes_read += packet_size as u64,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error),
        }
        let Some(ts) = packet.get(ts_offset..ts_offset + 188) else {
            continue;
        };
        let Some((pid, start, payload)) = ts_payload(ts) else {
            continue;
        };
        if !matches!(pid, 0x10 | 0x12 | 0x14) {
            continue;
        }
        for section in psi.entry(pid).or_default().push_all(payload, start) {
            match pid {
                0x10 if metadata.network_name.is_none() => {
                    metadata.network_name = network_name_from_nit(&section);
                }
                0x12 if metadata.programme_name.is_none() => {
                    if let Some((name, description)) =
                        programme_from_eit(&section, preferred_service_id)
                    {
                        metadata.programme_name = Some(name);
                        metadata.programme_description = description;
                    }
                }
                0x14 if metadata.broadcast_time_utc.is_none() => {
                    metadata.broadcast_time_utc = utc_time_from_tdt_tot(&section);
                }
                _ => {}
            }
        }
        if metadata.network_name.is_some()
            && metadata.programme_name.is_some()
            && metadata.broadcast_time_utc.is_some()
        {
            break;
        }
    }
    Ok(metadata)
}

#[cfg_attr(not(feature = "fuzzing"), allow(dead_code))]
pub(crate) fn b24_caption_pids(section: &[u8]) -> Vec<u16> {
    b24_caption_streams(section)
        .into_iter()
        .map(|stream| stream.pid)
        .collect()
}

pub(crate) fn b24_caption_streams(section: &[u8]) -> Vec<B24CaptionStream> {
    if section.len() < 16 || section[0] != 0x02 {
        return Vec::new();
    }
    let program_info_length = (usize::from(section[10] & 0x0f) << 8) | usize::from(section[11]);
    let mut index = 12 + program_info_length;
    let end = match section.len().checked_sub(4) {
        Some(end) => end,
        None => return Vec::new(),
    };
    let mut streams = Vec::new();
    while index + 5 <= end {
        let stream_type = section[index];
        let pid = (u16::from(section[index + 1] & 0x1f) << 8) | u16::from(section[index + 2]);
        let descriptor_length =
            (usize::from(section[index + 3] & 0x0f) << 8) | usize::from(section[index + 4]);
        let Some(descriptors) = section.get(index + 5..index + 5 + descriptor_length) else {
            return Vec::new();
        };
        if stream_type == 0x06
            && b24_descriptor(descriptors)
            && let Some(component_tag) = descriptor_component_tag(descriptors)
        {
            streams.push(B24CaptionStream {
                pid,
                component_tag,
                language: iso639_language(descriptors),
            });
        }
        index += 5 + descriptor_length;
    }
    streams
}

#[cfg_attr(not(feature = "fuzzing"), allow(dead_code))]
pub(crate) fn data_pids(section: &[u8]) -> Vec<u16> {
    classified_data_pids(section).0
}

pub(crate) fn classified_data_pids(section: &[u8]) -> (Vec<u16>, Vec<u16>, Vec<u16>) {
    if section.len() < 16 || section[0] != 0x02 {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let program_info_length = (usize::from(section[10] & 0x0f) << 8) | usize::from(section[11]);
    let mut index = 12 + program_info_length;
    let end = match section.len().checked_sub(4) {
        Some(end) => end,
        None => return (Vec::new(), Vec::new(), Vec::new()),
    };
    let mut pids = Vec::new();
    let mut caption_pids = Vec::new();
    let mut superimpose_pids = Vec::new();
    while index + 5 <= end {
        let stream_type = section[index];
        let pid = (u16::from(section[index + 1] & 0x1f) << 8) | u16::from(section[index + 2]);
        let descriptor_length =
            (usize::from(section[index + 3] & 0x0f) << 8) | usize::from(section[index + 4]);
        let Some(descriptors) = section.get(index + 5..index + 5 + descriptor_length) else {
            return (Vec::new(), Vec::new(), Vec::new());
        };
        let declared_b24_text_service = b24_text_service_kind(descriptors).is_some()
            && has_b24_data_component_descriptor(descriptors);
        if stream_type == 0x06 && !declared_b24_text_service {
            pids.push(pid);
            match descriptor_component_tag(descriptors) {
                Some(0x30..=0x37) => caption_pids.push(pid),
                Some(0x38..=0x3f) => superimpose_pids.push(pid),
                _ => {}
            }
        }
        index += 5 + descriptor_length;
    }
    (pids, caption_pids, superimpose_pids)
}

/// Discover private PES streams in a normal 188-byte MPEG-TS recording.
/// Their payload is inspected as declared ARIB-TTML only after the bounded
/// PES/XML route has isolated a complete document.
pub(crate) fn discover_mpeg_ts_data_tracks(path: &Path) -> io::Result<Option<DataTracks>> {
    let mut file = File::open(path)?;
    let mut bytes = vec![0; PSI_SCAN_BYTES];
    let length = file.read(&mut bytes)?;
    let mut pmt_pid = None;
    let mut psi = HashMap::<u16, PsiAssembler>::new();
    for packet in bytes[..length].chunks_exact(188) {
        let Some((pid, start, payload)) = ts_payload(packet) else {
            continue;
        };
        if pid != 0 && Some(pid) != pmt_pid {
            continue;
        }
        let Some(section) = psi.entry(pid).or_default().push(payload, start) else {
            continue;
        };
        if pid == 0 && section[0] == 0x00 {
            pmt_pid = first_pmt_pid(&section);
        }
        if Some(pid) == pmt_pid && section[0] == 0x02 {
            let (pids, caption_pids, superimpose_pids) = classified_data_pids(&section);
            if !pids.is_empty() {
                return Ok(Some(DataTracks {
                    pmt_pid: pid,
                    pids,
                    caption_pids,
                    superimpose_pids,
                }));
            }
        }
    }
    Ok(None)
}

pub(crate) fn scan_mpeg_ts_ttml<F, P, C, R>(
    path: &Path,
    tracks: &DataTracks,
    on_caption: F,
    on_progress: P,
    cancelled: C,
    on_pes: R,
) -> io::Result<B24DecodeSummary>
where
    F: FnMut(TtmlCaption) -> io::Result<()>,
    P: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
    R: FnMut(u16, u64, &[u8]) -> io::Result<()>,
{
    scan_ts_ttml_impl(
        path,
        tracks,
        TsFraming {
            packet_size: 188,
            ts_offset: 0,
        },
        on_caption,
        on_progress,
        cancelled,
        on_pes,
    )
}

pub(crate) fn discover_b24_tracks(path: &Path) -> io::Result<Vec<B24Track>> {
    let mut file = File::open(path)?;
    let file_length = file.metadata()?.len();
    let mut pmt_programs_by_pid = HashMap::new();
    let mut service_names = HashMap::new();
    let mut tracks = Vec::new();
    let initial_length = (PSI_SCAN_BYTES as u64).min(file_length);
    scan_b24_psi_window(
        &mut file,
        0,
        initial_length,
        &mut pmt_programs_by_pid,
        &mut service_names,
        &mut tracks,
    )?;

    if file_length > initial_length {
        let maximum_start = file_length.saturating_sub(PSI_SAMPLE_WINDOW_BYTES);
        for sample in 1..=PSI_SAMPLE_WINDOW_COUNT {
            let offset = maximum_start.saturating_mul(sample) / PSI_SAMPLE_WINDOW_COUNT;
            let aligned_offset = offset / 188 * 188;
            scan_b24_psi_window(
                &mut file,
                aligned_offset,
                PSI_SAMPLE_WINDOW_BYTES.min(file_length.saturating_sub(aligned_offset)),
                &mut pmt_programs_by_pid,
                &mut service_names,
                &mut tracks,
            )?;
        }
    }
    for track in &mut tracks {
        if track.service_name.is_none() {
            track.service_name = service_names.get(&track.service_id).cloned();
        }
    }
    Ok(tracks)
}

fn scan_b24_psi_window(
    file: &mut File,
    start_offset: u64,
    length: u64,
    pmt_programs_by_pid: &mut HashMap<u16, u16>,
    service_names: &mut HashMap<u16, String>,
    tracks: &mut Vec<B24Track>,
) -> io::Result<()> {
    file.seek(SeekFrom::Start(start_offset))?;
    let mut packet = [0_u8; 188];
    let mut bytes_read = 0_u64;
    let mut psi = HashMap::<u16, PsiAssembler>::new();
    while bytes_read.saturating_add(188) <= length {
        match file.read_exact(&mut packet) {
            Ok(()) => bytes_read += 188,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error),
        }
        let Some((pid, start, payload)) = ts_payload(&packet) else {
            continue;
        };
        if pid != 0 && pid != 0x11 && !pmt_programs_by_pid.contains_key(&pid) {
            continue;
        }
        for section in psi.entry(pid).or_default().push_all(payload, start) {
            if pid == 0 && section[0] == 0x00 {
                for (program, pmt) in pmt_programs(&section) {
                    pmt_programs_by_pid.insert(pmt, program);
                }
            }
            if pid == 0x11 && matches!(section.first(), Some(0x42) | Some(0x46)) {
                for service_id in pmt_programs_by_pid.values().copied() {
                    if let Some(name) = service_name_from_sdt(&section, service_id) {
                        service_names.insert(service_id, name);
                    }
                }
            }
            if section[0] == 0x02
                && section.get(5).is_some_and(|flags| flags & 0x01 != 0)
                && let Some(&service_id) = pmt_programs_by_pid.get(&pid)
            {
                for stream in b24_caption_streams(&section) {
                    if let Some(track) = tracks.iter_mut().find(|track| {
                        track.service_id == service_id
                            && track.component_tag == stream.component_tag
                    }) {
                        if !track.caption_pids.contains(&stream.pid) {
                            track.caption_pids.push(stream.pid);
                        }
                        track.caption_pid = stream.pid;
                        track.pmt_pid = pid;
                        if stream.language.is_some() {
                            track.language = stream.language;
                        }
                    } else {
                        tracks.push(B24Track {
                            service_id,
                            pmt_pid: pid,
                            caption_pid: stream.pid,
                            component_tag: stream.component_tag,
                            caption_pids: vec![stream.pid],
                            language: stream.language,
                            service_name: service_names.get(&service_id).cloned(),
                        });
                    }
                }
                for track in tracks.iter().filter(|track| track.service_id == service_id) {
                    debug_assert!((0x30..=0x37).contains(&track.component_tag));
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn discover_b24(path: &Path) -> io::Result<Option<B24Track>> {
    Ok(discover_b24_tracks(path)?.into_iter().next())
}
