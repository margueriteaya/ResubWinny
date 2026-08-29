//! Small deterministic protocol fixtures for public tests and fuzz seeds.
//! These builders model packet boundaries and PSI/PES framing only; they are
//! not an open broadcast corpus or a claim of broadcaster-specific behaviour.

/// Build one 188-byte MPEG-TS packet with payload-only adaptation control.
pub fn make_ts_packet(pid: u16, payload_start: bool, payload: &[u8]) -> [u8; 188] {
    assert!(pid < 0x2000, "PID exceeds the MPEG-TS 13-bit range");
    assert!(payload.len() <= 184, "payload exceeds one TS packet");
    let mut packet = [0xff; 188];
    packet[0] = 0x47;
    packet[1] = ((pid >> 8) as u8 & 0x1f) | u8::from(payload_start) << 6;
    packet[2] = pid as u8;
    packet[3] = 0x10;
    packet[4..4 + payload.len()].copy_from_slice(payload);
    packet
}

/// Build a PAT section advertising one program and its PMT PID.
pub fn make_pat(program_number: u16, pmt_pid: u16) -> Vec<u8> {
    let mut section = vec![0x00, 0xb0, 0x0d];
    section.extend_from_slice(&[0x00, 0x01, 0xc1, 0x00, 0x00]);
    section.extend_from_slice(&program_number.to_be_bytes());
    section.extend_from_slice(&(0xe000 | (pmt_pid & 0x1fff)).to_be_bytes());
    append_crc(&mut section);
    section
}

/// Build a PMT section with one stream descriptor.
pub fn make_pmt(program_number: u16, pcr_pid: u16, stream_type: u8, stream_pid: u16) -> Vec<u8> {
    let mut section = vec![
        0x02,
        0xb0,
        0x12,
        (program_number >> 8) as u8,
        program_number as u8,
        0xc1,
        0x00,
        0x00,
    ];
    section.extend_from_slice(&(0xe000 | (pcr_pid & 0x1fff)).to_be_bytes());
    section.extend_from_slice(&[0xf0, 0x00, stream_type]);
    section.extend_from_slice(&(0xe000 | (stream_pid & 0x1fff)).to_be_bytes());
    section.extend_from_slice(&[0xf0, 0x00]);
    append_crc(&mut section);
    section
}

/// Build a minimal PES packet. `pts90k` is encoded when supplied.
pub fn make_pes(stream_id: u8, payload: &[u8], pts90k: Option<u64>) -> Vec<u8> {
    let has_pts = pts90k.is_some();
    let header_len = if has_pts { 5 } else { 0 };
    let pes_length = 3 + header_len + payload.len();
    assert!(
        pes_length <= u16::MAX as usize,
        "PES exceeds bounded synthetic fixture"
    );
    let mut pes = vec![
        0x00,
        0x00,
        0x01,
        stream_id,
        (pes_length >> 8) as u8,
        pes_length as u8,
        0x80,
        if has_pts { 0x80 } else { 0x00 },
        header_len as u8,
    ];
    if let Some(pts) = pts90k {
        let pts = crate::time::Pts90k::new(pts).expect("PTS exceeds the MPEG 33-bit range");
        pes.extend_from_slice(&encode_pts(pts));
    }
    pes.extend_from_slice(payload);
    pes
}

/// Build one bounded ARIB data-group envelope with CRC-16-CCITT evidence.
pub fn make_b24_data_group(group_id: u8, group_version: u8, data: &[u8]) -> Vec<u8> {
    assert!(
        data.len() <= u16::MAX as usize,
        "B24 data group is too large"
    );
    let mut group = vec![
        (group_id << 2) | (group_version & 0x03),
        0,
        0,
        (data.len() >> 8) as u8,
        data.len() as u8,
    ];
    group.extend_from_slice(data);
    let crc = crc16_ccitt(&group);
    group.extend_from_slice(&crc.to_be_bytes());
    group
}

/// Build the minimum MMTP header accepted by the experimental parser.
pub fn make_mmtp_packet(
    packet_id: u16,
    sequence_number: u32,
    payload_type: u8,
    payload: &[u8],
) -> Vec<u8> {
    let mut packet = vec![0, payload_type & 0x3f];
    packet.extend_from_slice(&packet_id.to_be_bytes());
    packet.extend_from_slice(&0_u32.to_be_bytes());
    packet.extend_from_slice(&sequence_number.to_be_bytes());
    packet.extend_from_slice(payload);
    packet
}

fn encode_pts(value: crate::time::Pts90k) -> [u8; 5] {
    let value = value.ticks();
    [
        0x21 | ((value >> 29) as u8 & 0x0e),
        (value >> 22) as u8,
        0x01 | ((value >> 14) as u8 & 0xfe),
        (value >> 7) as u8,
        0x01 | ((value as u8) << 1),
    ]
}

fn append_crc(section: &mut Vec<u8>) {
    let mut crc = 0xffff_ffffu32;
    for byte in section.iter().copied() {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04c1_1db7
            } else {
                crc << 1
            };
        }
    }
    section.extend_from_slice(&crc.to_be_bytes());
}

fn crc16_ccitt(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff_u16;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mpeg_ts::{psi_section, ts_payload};
    use crate::transport::tlv_mmt::parse_mmtp_packet;

    #[test]
    fn ts_builder_round_trips_payload_header() {
        let packet = make_ts_packet(0x123, true, b"fixture");
        let (pid, start, payload) = ts_payload(&packet).expect("synthetic TS");
        assert_eq!((pid, start), (0x123, true));
        assert_eq!(&payload[..7], b"fixture");
    }

    #[test]
    fn pat_and_pmt_have_valid_section_lengths_and_crc() {
        let mut pat = vec![0];
        pat.extend(make_pat(1, 0x100));
        let mut pmt = vec![0];
        pmt.extend(make_pmt(1, 0x101, 0x06, 0x102));
        assert!(psi_section(&pat).is_some());
        assert!(psi_section(&pmt).is_some());
    }

    #[test]
    fn pes_builder_encodes_optional_pts() {
        let pes = make_pes(0xbd, b"caption", Some(90_000));
        assert_eq!(&pes[..4], &[0, 0, 1, 0xbd]);
        assert_eq!(&pes[14..], b"caption");
    }

    #[test]
    fn b24_data_group_keeps_length_and_crc() {
        let group = make_b24_data_group(1, 2, b"caption");
        assert_eq!(&group[3..5], &[0, 7]);
        assert_eq!(crc16_ccitt(&group), 0);
    }

    #[test]
    fn mmtp_builder_round_trips_parser_header() {
        let bytes = make_mmtp_packet(0x456, 7, 2, b"signal");
        let packet = parse_mmtp_packet(&bytes).expect("synthetic MMTP");
        assert_eq!(packet.packet_id, 0x456);
        assert_eq!(packet.sequence_number, 7);
        assert_eq!(packet.payload_type, 2);
        assert_eq!(packet.payload, b"signal");
    }
}
