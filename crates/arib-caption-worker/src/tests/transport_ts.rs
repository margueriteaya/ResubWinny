use super::*;

fn ts_packet(pid: u16, payload_start: bool, payload: &[u8]) -> [u8; 188] {
    assert!(payload.len() <= 184);
    let mut packet = [0xff; 188];
    packet[0] = 0x47;
    packet[1] = ((pid >> 8) as u8 & 0x1f) | u8::from(payload_start) << 6;
    packet[2] = pid as u8;
    packet[3] = 0x10;
    packet[4..4 + payload.len()].copy_from_slice(payload);
    packet
}

fn ts_packet_fragment(pid: u16, payload_start: bool, payload: &[u8]) -> [u8; 188] {
    assert!(!payload.is_empty() && payload.len() <= 183);
    let mut packet = [0xff; 188];
    packet[0] = 0x47;
    packet[1] = ((pid >> 8) as u8 & 0x1f) | u8::from(payload_start) << 6;
    packet[2] = pid as u8;
    packet[3] = 0x30;
    packet[4] = (183 - payload.len()) as u8;
    let offset = 5 + usize::from(packet[4]);
    packet[offset..offset + payload.len()].copy_from_slice(payload);
    packet
}

fn fragmented_private_pes_ttml_ts_fixture() -> Vec<u8> {
    let pat = [
        0x00, 0xb0, 0x0d, 0, 1, 0xc1, 0, 0, 0, 1, 0xe1, 0x00, 0, 0, 0, 0,
    ];
    let pmt = [
        0x02, 0xb0, 0x12, 0, 1, 0xc1, 0, 0, 0xe1, 0x20, 0xf0, 0, 0x06, 0xe1, 0x20, 0xf0, 0, 0, 0,
        0, 0,
    ];
    let ttml = b"<?xml version=\"1.0\"?><tt><body><p begin=\"0s\" end=\"1s\">Fragmented PSI</p></body></tt>";
    [
        ts_packet(0, true, &[0, 0x00, 0xbf, 0xff]).to_vec(),
        ts_packet_fragment(0, true, &[0, pat[0], pat[1], pat[2], pat[3], pat[4]]).to_vec(),
        ts_packet_fragment(0, false, &pat[5..]).to_vec(),
        ts_packet_fragment(0x0100, true, &[0, pmt[0], pmt[1], pmt[2], pmt[3], pmt[4]]).to_vec(),
        ts_packet_fragment(0x0100, false, &pmt[5..]).to_vec(),
        ts_packet(0x0120, true, ttml).to_vec(),
    ]
    .concat()
}

pub(super) fn private_pes_ttml_ts_fixture() -> Vec<u8> {
    let pat = [
        0, 0x00, 0xb0, 0x0d, 0, 1, 0xc1, 0, 0, 0, 1, 0xe1, 0x00, 0, 0, 0, 0,
    ];
    let pmt = [
        0, 0x02, 0xb0, 0x12, 0, 1, 0xc1, 0, 0, 0xe1, 0x20, 0xf0, 0, 0x06, 0xe1, 0x20, 0xf0, 0, 0,
        0, 0, 0,
    ];
    let ttml = b"<?xml version=\"1.0\"?><tt><body><p begin=\"0s\" end=\"1s\" tts:color=\"#12AB34\">TS TTML</p></body></tt>";
    [
        ts_packet(0, true, &pat).to_vec(),
        ts_packet(0x0100, true, &pmt).to_vec(),
        ts_packet(0x0120, true, ttml).to_vec(),
        ts_packet(0x1fff, false, &[]).to_vec(),
    ]
    .concat()
}

pub(super) fn m2ts_from_ts_packets(ts: &[u8]) -> Vec<u8> {
    let mut m2ts = Vec::with_capacity(ts.len() / 188 * 192);
    for packet in ts.chunks_exact(188) {
        m2ts.extend([0; 4]);
        m2ts.extend(packet);
    }
    m2ts
}

#[test]
fn converts_private_pes_ttml_in_a_188_byte_mpeg_ts_container() {
    let stem = format!("arib-caption-ts-ttml-{}", std::process::id());
    let input_path = std::env::temp_dir().join(format!("{stem}.ts"));
    let output_path = std::env::temp_dir().join(format!("{stem}.ass"));
    fs::write(&input_path, private_pes_ttml_ts_fixture()).expect("fixture");

    let tracks = discover_mpeg_ts_data_tracks(&input_path)
        .expect("inspect private PES")
        .expect("private PES tracks");
    assert_eq!(tracks.pids, vec![0x0120]);
    assert!(
        discover_b24_tracks(&input_path)
            .expect("inspect B24")
            .is_empty()
    );
    let inspection = inspect_input(&input_path).expect("inspection");
    assert_eq!(inspection.route_code, "mpeg_ts_ttml_candidate");
    assert_eq!(inspection.tracks.len(), 1);

    let options = ConversionOptions {
        ttml: true,
        archive: true,
        raw: true,
        ..ConversionOptions::default()
    };
    let report =
        convert_with_options_and_cancel(&input_path, &output_path, options, |_| {}, || false)
            .expect("convert TS TTML");
    assert_eq!(report.summary.captions, 1);
    assert_eq!(report.summary.characters, 7);
    assert!(
        fs::read_to_string(&output_path)
            .expect("ASS output")
            .contains("TS TTML")
    );
    assert!(
        fs::read_to_string(report.archive.as_ref().expect("archive"))
            .expect("archive output")
            .contains("TS TTML")
    );
    assert!(
        fs::read_to_string(report.raw.as_ref().expect("raw"))
            .expect("raw output")
            .contains("arib_ttml_private_pes")
    );
    assert_eq!(
        preview_caption(&input_path)
            .expect("preview")
            .expect("caption")
            .text,
        "TS TTML"
    );

    fs::remove_file(&input_path).expect("cleanup input");
    fs::remove_file(&output_path).expect("cleanup ASS");
    fs::remove_file(report.ttml.expect("TTML output")).expect("cleanup TTML");
    fs::remove_file(report.archive.expect("archive output")).expect("cleanup archive");
    fs::remove_file(report.raw.expect("raw output")).expect("cleanup raw");
}

#[test]
fn discovers_private_pes_after_fragmented_or_malformed_psi() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let ts_path = std::env::temp_dir().join(format!("arib-fragmented-psi-{stamp}.ts"));
    let m2ts_path = std::env::temp_dir().join(format!("arib-fragmented-psi-{stamp}.m2ts"));
    let recording = fragmented_private_pes_ttml_ts_fixture();
    fs::write(&ts_path, &recording).expect("TS fixture");
    fs::write(&m2ts_path, m2ts_from_ts_packets(&recording)).expect("M2TS fixture");

    assert_eq!(
        discover_mpeg_ts_data_tracks(&ts_path)
            .expect("discover TS")
            .expect("TS data tracks")
            .pids,
        vec![0x0120]
    );
    assert_eq!(
        discover_m2ts_data_tracks(&m2ts_path)
            .expect("discover M2TS")
            .expect("M2TS data tracks")
            .pids,
        vec![0x0120]
    );
    fs::remove_file(ts_path).expect("cleanup TS");
    fs::remove_file(m2ts_path).expect("cleanup M2TS");
}

#[test]
fn m2ts_scan_ignores_malformed_packets_and_truncated_trailing_bytes() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("arib-m2ts-truncated-{stamp}.m2ts"));
    let mut ts = private_pes_ttml_ts_fixture();
    let mut malformed = [0u8; 188];
    malformed[0] = 0x47;
    malformed[3] = 0x30; // adaptation + payload with an invalid adaptation length
    malformed[4] = 0xff;
    ts.extend(malformed);
    let mut recording = m2ts_from_ts_packets(&ts);
    let complete_bytes = recording.len() as u64;
    recording.extend([0x00; 17]);
    fs::write(&path, recording).expect("fixture");

    let tracks = discover_m2ts_data_tracks(&path)
        .expect("inspect M2TS")
        .expect("private PES track");
    let mut captions = Vec::new();
    let summary = scan_m2ts_ttml(
        &path,
        &tracks,
        |caption| {
            captions.push(caption);
            Ok(())
        },
        |_| {},
        || false,
        |_, _, _| Ok(()),
    )
    .expect("scan M2TS");

    assert_eq!(summary.bytes_read, complete_bytes);
    assert_eq!(summary.decoder_errors, 0);
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].text, "TS TTML");
    fs::remove_file(path).expect("cleanup");
}

#[test]
fn parses_bounded_nit_eit_and_tot_broadcast_metadata() {
    let network_name = [0x0e, b'N', b'H', b'K'];
    let mut nit = vec![0x40, 0xb0, 0x00, 0, 1, 0xc1, 0, 0, 0xf0, 6, 0x40, 4];
    nit.extend(network_name);
    nit.extend([0, 0, 0, 0]);
    let section_length = nit.len() - 3;
    nit[1] = 0xb0 | ((section_length >> 8) as u8 & 0x0f);
    nit[2] = section_length as u8;
    assert_eq!(network_name_from_nit(&nit).as_deref(), Some("NHK"));

    let name = [0x0e, b'N', b'e', b'w', b's'];
    let description = [0x0e, b'E', b'v', b'e', b'n', b'i', b'n', b'g'];
    let mut short_event = vec![b'j', b'p', b'n', name.len() as u8];
    short_event.extend(name);
    short_event.push(description.len() as u8);
    short_event.extend(description);
    let mut descriptors = vec![0x4d, short_event.len() as u8];
    descriptors.extend(short_event);
    let mut eit = vec![0x4e, 0xb0, 0, 0, 1, 0xc1, 0, 0, 0, 1, 0, 1, 0, 0];
    eit.extend([
        0,
        1,
        0xff,
        0xff,
        0xff,
        0xff,
        0xff,
        0,
        0,
        0,
        0x80 | ((descriptors.len() >> 8) as u8 & 0x0f),
        descriptors.len() as u8,
    ]);
    eit.extend(descriptors);
    eit.extend([0, 0, 0, 0]);
    let section_length = eit.len() - 3;
    eit[1] = 0xb0 | ((section_length >> 8) as u8 & 0x0f);
    eit[2] = section_length as u8;
    assert_eq!(
        programme_from_eit(&eit, Some(1)),
        Some(("News".into(), Some("Evening".into())))
    );
    assert!(programme_from_eit(&eit, Some(2)).is_none());

    let tot = [0x73, 0x70, 0x0b, 0xeb, 0x96, 0x12, 0x34, 0x56];
    assert_eq!(
        utc_time_from_tdt_tot(&tot).as_deref(),
        Some("2024-01-01 12:34:56 UTC")
    );
}

#[test]
fn discovers_broadcast_metadata_from_the_requested_packet_position() {
    let network_name = [0x0e, b'N', b'H', b'K'];
    let mut nit = vec![0x40, 0xb0, 0x00, 0, 1, 0xc1, 0, 0, 0xf0, 6, 0x40, 4];
    nit.extend(network_name);
    nit.extend([0, 0, 0, 0]);
    let section_length = nit.len() - 3;
    nit[1] = 0xb0 | ((section_length >> 8) as u8 & 0x0f);
    nit[2] = section_length as u8;

    let name = [0x0e, b'N', b'e', b'w', b's'];
    let description = [0x0e, b'L', b'i', b'v', b'e'];
    let mut short_event = vec![b'j', b'p', b'n', name.len() as u8];
    short_event.extend(name);
    short_event.push(description.len() as u8);
    short_event.extend(description);
    let mut descriptors = vec![0x4d, short_event.len() as u8];
    descriptors.extend(short_event);
    let mut eit = vec![0x4e, 0xb0, 0, 0, 1, 0xc1, 0, 0, 0, 1, 0, 1, 0, 0];
    eit.extend([
        0,
        1,
        0xff,
        0xff,
        0xff,
        0xff,
        0xff,
        0,
        0,
        0,
        0x80 | ((descriptors.len() >> 8) as u8 & 0x0f),
        descriptors.len() as u8,
    ]);
    eit.extend(descriptors);
    eit.extend([0, 0, 0, 0]);
    let section_length = eit.len() - 3;
    eit[1] = 0xb0 | ((section_length >> 8) as u8 & 0x0f);
    eit[2] = section_length as u8;
    let tot = [0x73, 0x70, 0x0b, 0xeb, 0x96, 0x12, 0x34, 0x56];

    let prefix_packets = 7_u64;
    let mut recording = vec![0xff; prefix_packets as usize * 188];
    for packet in recording.chunks_exact_mut(188) {
        packet.copy_from_slice(&ts_packet(0x1fff, false, &[]));
    }
    recording.extend(ts_packet(0x10, true, &[vec![0], nit].concat()));
    recording.extend(ts_packet(0x12, true, &[vec![0], eit].concat()));
    recording.extend(ts_packet(0x14, true, &[vec![0], tot.to_vec()].concat()));

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("arib-broadcast-at-{stamp}.ts"));
    fs::write(&path, recording).expect("broadcast fixture");
    let metadata =
        discover_broadcast_metadata_at(&path, 188, 0, Some(1), prefix_packets * 188 + 40)
            .expect("metadata at packet position");
    assert_eq!(metadata.network_name.as_deref(), Some("NHK"));
    assert_eq!(metadata.programme_name.as_deref(), Some("News"));
    assert_eq!(metadata.programme_description.as_deref(), Some("Live"));
    assert_eq!(
        metadata.broadcast_time_utc.as_deref(),
        Some("2024-01-01 12:34:56 UTC")
    );
    fs::remove_file(path).expect("cleanup broadcast fixture");
}

#[test]
fn psi_assembler_keeps_every_section_in_one_payload() {
    let first = [0x70, 0x70, 0x04, 0, 0, 0, 0];
    let second = [0x73, 0x70, 0x04, 0, 0, 0, 0];
    let payload = [vec![0], first.to_vec(), second.to_vec(), vec![0xff; 8]].concat();
    let sections = PsiAssembler::default().push_all(&payload, true);
    assert_eq!(sections, vec![first.to_vec(), second.to_vec()]);
}

#[test]
fn psi_assembler_accepts_a_bounded_long_eit_section() {
    let mut section = vec![0xff; 1_503];
    section[0] = 0x4e;
    let section_length = section.len() - 3;
    section[1] = 0xb0 | ((section_length >> 8) as u8 & 0x0f);
    section[2] = section_length as u8;
    let mut assembler = PsiAssembler::default();
    let mut completed = Vec::new();
    for (index, chunk) in section.chunks(173).enumerate() {
        let payload = if index == 0 {
            [vec![0], chunk.to_vec()].concat()
        } else {
            chunk.to_vec()
        };
        completed.extend(assembler.push_all(&payload, index == 0));
    }
    assert_eq!(completed, vec![section]);
}
