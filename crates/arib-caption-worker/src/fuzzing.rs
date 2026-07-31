pub fn probe_input(data: &[u8]) {
    let _ = crate::probe_bytes(data);
    let _ = crate::tlv_probe(data);
}

pub fn decode_ttml_envelopes(data: &[u8]) {
    for document in crate::ttml_documents(data).into_iter().take(16) {
        let _ = crate::parse_ttml_captions(&document.xml, 0);
    }
}

pub fn parse_ts_metadata(data: &[u8]) {
    for (packet_size, prefix) in [(188_usize, 0_usize), (192, 4)] {
        for packet in data.chunks_exact(packet_size).take(4_096) {
            let Some(ts_packet) = packet.get(prefix..prefix + 188) else {
                continue;
            };
            let Some((_, payload_start, payload)) =
                crate::transport::mpeg_ts::ts_payload(ts_packet)
            else {
                continue;
            };
            if !payload_start {
                continue;
            }
            let Some(section) = crate::transport::mpeg_ts::psi_section(payload) else {
                continue;
            };
            let _ = crate::transport::mpeg_ts::first_pmt_pid(section);
            let _ = crate::transport::mpeg_ts::pmt_programs(section);
            let _ = crate::transport::mpeg_ts::b24_caption_pids(section);
            let _ = crate::transport::mpeg_ts::data_pids(section);
            let _ = crate::transport::mpeg_ts::iso639_language(section);
            let _ = crate::transport::mpeg_ts::service_name_from_sdt(section, 0);
        }
    }
}

/// Exercise alignment loss, truncated packet tails, adaptation-field bounds,
/// and PSI pointer values independently of the normal content probe.
pub fn parse_ts_transport_packets(data: &[u8]) {
    const MAX_INPUT: usize = 64 * 1024;
    let data = &data[..data.len().min(MAX_INPUT)];
    for packet_size in [188_usize, 192] {
        let prefix = usize::from(packet_size == 192) * 4;
        for alignment in 0..packet_size.min(data.len()) {
            for packet in data[alignment..].chunks_exact(packet_size).take(512) {
                let Some(ts_packet) = packet.get(prefix..prefix + 188) else {
                    continue;
                };
                let Some((_, payload_start, payload)) =
                    crate::transport::mpeg_ts::ts_payload(ts_packet)
                else {
                    continue;
                };
                if payload_start {
                    let _ = crate::transport::mpeg_ts::psi_section(payload);
                }
            }
        }
    }
}

pub fn decode_arib_si_text(data: &[u8]) {
    let _ = crate::arib_text::decode_service_name(&data[..data.len().min(252)]);
}

pub fn parse_pes_b24_headers(data: &[u8]) {
    let limit = data.len().min(4 * 1024 * 1024);
    let data = &data[..limit];
    let _ = crate::caption::b24::b24_payload_from_pes(data);
    let _ = crate::caption::b24::pes_pts_from_header(data);

    for offset in 0..data.len().min(4_096) {
        let candidate = &data[offset..];
        let _ = crate::caption::b24::b24_payload_from_pes(candidate);
        let _ = crate::caption::b24::pes_pts_from_header(candidate);
    }
}

pub fn parse_mmtp_envelopes(data: &[u8]) {
    let _ = crate::transport::tlv_mmt::parse_mmtp_packet(data);
    for packet_type in [0x01, 0x02, 0x03, 0x60, 0x61, 0xff] {
        let _ = crate::transport::tlv_mmt::tlv_mmtp_payload(packet_type, data);
    }
}
