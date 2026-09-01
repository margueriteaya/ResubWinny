use super::*;

#[test]
fn prefers_structured_tlv_when_transport_bytes_look_like_a_filename_named_ts() {
    let mut bytes = Vec::new();
    for packet_type in 0..MIN_SYNC_HITS {
        bytes.extend([0x7f, packet_type as u8, 0, 188]);
        let payload_start = bytes.len();
        bytes.resize(payload_start + 188, 0);
        bytes[payload_start] = 0x47;
    }
    let probe = probe_bytes(&bytes);
    assert_eq!(probe.kind, InputKind::Tlv);
    assert_eq!(probe.sync_offset, Some(0));
}

#[test]
fn detects_content_framed_isdb_s3_tlv() {
    let mut bytes = Vec::new();
    for packet_type in 0..MIN_SYNC_HITS {
        bytes.extend([0x7f, packet_type as u8, 0, 3, 0x60, 0, packet_type as u8]);
    }
    let probe = probe_bytes(&bytes);
    assert_eq!(probe.kind, InputKind::Tlv);
    assert_eq!(probe.sync_offset, Some(0));
    assert_eq!(probe.confidence, MIN_SYNC_HITS);
}

#[test]
fn finds_direct_ipv6_udp_destination_without_assuming_mmtp() {
    let mut payload = vec![0_u8; 48];
    payload[0] = 0x60;
    payload[4..6].copy_from_slice(&8_u16.to_be_bytes());
    payload[6] = 17;
    payload[42..44].copy_from_slice(&5000_u16.to_be_bytes());
    assert_eq!(direct_ipv6_udp_destination_port(&payload), Some(5000));
    payload[6] = 6;
    assert_eq!(direct_ipv6_udp_destination_port(&payload), None);
}

#[test]
fn preserves_mpt_descriptor_tags_and_exact_timestamp_metadata() {
    let mut timestamp = vec![0x00, 0x01, 0x0c];
    timestamp.extend(42_u32.to_be_bytes());
    timestamp.extend(0x1234_5678_9abc_def0_u64.to_be_bytes());
    assert_eq!(
        parse_mpt_descriptors(&[0x80, 0x20, 0x02, 0xaa, 0xbb, 0xf0, 0x01, 0,]),
        Some((vec![0x8020, 0xf001], BTreeMap::new()))
    );
    assert_eq!(
        parse_mpt_descriptors(&timestamp),
        Some((vec![0x0001], BTreeMap::from([(42, 0x1234_5678_9abc_def0)])))
    );
    assert_eq!(parse_mpt_descriptors(&[0x80, 0x20, 0x02, 0xaa]), None);
}

#[test]
fn converts_ntp_deltas_to_the_shared_millisecond_timeline_without_relabelling_ntp_as_pts() {
    let origin = 0x1234_5678_0000_0000_u64;
    assert_eq!(ntp_delta_ms(origin, origin), 0);
    assert_eq!(ntp_delta_ms(origin + (2_u64 << 32), origin), 2_000);
    assert_eq!(ntp_delta_ms(origin - (1_u64 << 31), origin), -500);
}

#[test]
fn observes_unfragmented_mpt_stpp_asset_without_extracting_it() {
    // A compact synthetic M2 signalling message carrying one MPT asset.
    // It exercises bounded observation only; it is not a caption payload.
    let mut mpt_body = vec![0, 0, 0, 0, 1];
    mpt_body.extend([0, 0, 0, 0, 0, 0]);
    mpt_body.extend(*b"stpp");
    mpt_body.extend([0, 1, 0, 0x12, 0x34, 0, 0]);
    let mut table = vec![0x20, 0];
    table.extend((mpt_body.len() as u16).to_be_bytes());
    table.extend(mpt_body);

    let mut message = vec![0x80, 0, 0];
    message.extend((table.len() as u16).to_be_bytes());
    message.extend(table);

    let mut signalling_payload = vec![0, 0];
    signalling_payload.extend(message);
    let mut mmtp = vec![0, 0x02, 0x04, 0x56, 0, 0, 0, 0];
    mmtp.extend(7_u32.to_be_bytes());
    mmtp.extend(signalling_payload);
    let packet = parse_mmtp_packet(&mmtp).expect("MMTP packet");

    let mut diagnostics = TlvDiagnostics::default();
    let mut assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    inspect_mmtp_packet(
        &packet,
        &mut diagnostics,
        &mut assemblers,
        &mut mpu_assemblers,
        None,
        None,
    );

    assert_eq!(diagnostics.mmtp_packets, 1);
    assert_eq!(diagnostics.mmtp_sequences.get(&0x0456), Some(&7));
    assert_eq!(
        diagnostics.mpt_assets.get(&0x1234),
        Some(&"stpp".to_owned())
    );
}

#[test]
fn reassembles_contiguous_mpt_signalling_fragments_with_a_hard_cap() {
    let mut mpt_body = vec![0, 0, 0, 0, 1];
    mpt_body.extend([0, 0, 0, 0, 0, 0]);
    mpt_body.extend(*b"stpp");
    mpt_body.extend([0, 1, 0, 0x45, 0x67, 0, 0]);
    let mut table = vec![0x20, 0];
    table.extend((mpt_body.len() as u16).to_be_bytes());
    table.extend(mpt_body);
    let mut message = vec![0x80, 0, 0];
    message.extend((table.len() as u16).to_be_bytes());
    message.extend(table);

    let mmtp_packet = |sequence: u32, indicator: u8, part: &[u8]| {
        let mut bytes = vec![0, 0x02, 0x04, 0x57, 0, 0, 0, 0];
        bytes.extend(sequence.to_be_bytes());
        bytes.extend([indicator << 6, 0]);
        bytes.extend(part);
        bytes
    };
    let split = 8;
    let first = mmtp_packet(100, 0b01, &message[..split]);
    let last = mmtp_packet(101, 0b11, &message[split..]);
    let mut diagnostics = TlvDiagnostics::default();
    let mut assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    inspect_mmtp_packet(
        &parse_mmtp_packet(&first).expect("first fragment"),
        &mut diagnostics,
        &mut assemblers,
        &mut mpu_assemblers,
        None,
        None,
    );
    assert!(diagnostics.mpt_assets.is_empty());
    inspect_mmtp_packet(
        &parse_mmtp_packet(&last).expect("last fragment"),
        &mut diagnostics,
        &mut assemblers,
        &mut mpu_assemblers,
        None,
        None,
    );
    assert_eq!(diagnostics.signalling_fragments_reassembled, 1);
    assert_eq!(diagnostics.signalling_fragments_dropped, 0);
    assert_eq!(
        diagnostics.mpt_assets.get(&0x4567),
        Some(&"stpp".to_owned())
    );
}

#[test]
fn drops_signalling_fragments_when_sequence_is_not_contiguous() {
    let mut diagnostics = TlvDiagnostics::default();
    let mut assemblers = BTreeMap::new();
    assert_eq!(
        assemble_signalling_fragment(&mut assemblers, 0x2222, 3, 0b01, b"first", &mut diagnostics,),
        Some(Vec::new())
    );
    assert_eq!(
        assemble_signalling_fragment(&mut assemblers, 0x2222, 5, 0b11, b"last", &mut diagnostics,),
        None
    );
    assert_eq!(diagnostics.signalling_fragments_dropped, 1);
}

#[test]
fn tlv_inspection_reports_a_discovered_stpp_asset() {
    let mut mpt_body = vec![0, 0, 0, 0, 1];
    mpt_body.extend([0, 0, 0, 0, 0, 0]);
    mpt_body.extend(*b"stpp");
    mpt_body.extend([0, 1, 0, 0x33, 0x44, 0, 0]);
    let mut table = vec![0x20, 0];
    table.extend((mpt_body.len() as u16).to_be_bytes());
    table.extend(mpt_body);
    let mut message = vec![0x80, 0, 0];
    message.extend((table.len() as u16).to_be_bytes());
    message.extend(table);
    let mut mmtp = vec![0, 0x02, 0x04, 0x58, 0, 0, 0, 0];
    mmtp.extend(1_u32.to_be_bytes());
    mmtp.extend([0, 0]);
    mmtp.extend(message);
    let mut tlv = Vec::new();
    let mut hcfb = vec![0, 0, 0x61];
    hcfb.extend(mmtp);
    tlv.extend([0x7f, 0x03]);
    tlv.extend((hcfb.len() as u16).to_be_bytes());
    tlv.extend(hcfb);
    for packet_type in 0..3 {
        tlv.extend([0x7f, packet_type, 0, 1, 0]);
    }
    let path = std::env::temp_dir().join(format!(
        "arib-caption-tlv-inspection-{}.tlv",
        std::process::id()
    ));
    fs::write(&path, tlv).expect("fixture");
    let inspection = inspect_input(&path).expect("inspection");
    assert!(inspection.route.contains("stpp"));
    assert!(
        inspection
            .tracks
            .iter()
            .any(|track| track.label == "MMT asset · stpp" && track.detail.contains("0x3344"))
    );
    fs::remove_file(path).expect("cleanup");
}

#[test]
fn asset_evidence_preserves_only_confirmed_mpt_metadata() {
    let mut diagnostics = TlvDiagnostics::default();
    diagnostics.mpt_assets.insert(0x0459, "stpp".into());
    diagnostics
        .mpt_descriptor_tags
        .insert(0x0459, vec![0x0001, 0x0020]);
    diagnostics
        .mpt_presentation_ntp
        .insert((0x0459, 42), 0x0000_0001_8000_0000);
    let evidence = tlv_asset_evidence(&diagnostics);
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].packet_id, 0x0459);
    assert_eq!(evidence[0].source_offset, 0);
    assert_eq!(evidence[0].asset_type, "stpp");
    assert_eq!(evidence[0].descriptor_tags, vec![0x0001, 0x0020]);
    assert_eq!(
        evidence[0].presentation_ntp.get(&42),
        Some(&0x0000_0001_8000_0000)
    );
    assert!(
        evidence[0]
            .payload_route
            .contains("validated asset extractor")
    );
}

#[test]
fn observes_complete_stpp_closed_caption_mfu_without_decoding_it() {
    let ttml = b"<tt><body><p>caption</p></body></tt>";
    let mut closed_caption = vec![0, 0, 0, 0, 0];
    closed_caption.extend((ttml.len() as u16).to_be_bytes());
    closed_caption.extend(ttml);
    let mut mfu = vec![0, 0, 0, 0];
    mfu.extend(closed_caption);
    let mut mpu = vec![0x20, 0];
    mpu.extend(42_u32.to_be_bytes());
    mpu.extend(mfu);
    let mut payload = Vec::new();
    payload.extend((mpu.len() as u16).to_be_bytes());
    payload.extend(mpu);
    let mut mmtp = vec![0, 0x00, 0x04, 0x59, 0, 0, 0, 0];
    mmtp.extend(8_u32.to_be_bytes());
    mmtp.extend(payload);
    let packet = parse_mmtp_packet(&mmtp).expect("MPU packet");
    let mut diagnostics = TlvDiagnostics::default();
    diagnostics.mpt_assets.insert(0x0459, "stpp".into());
    let mut signalling_assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    inspect_mmtp_packet(
        &packet,
        &mut diagnostics,
        &mut signalling_assemblers,
        &mut mpu_assemblers,
        None,
        None,
    );
    assert_eq!(diagnostics.stpp_mfu_fragments, 1);
    assert_eq!(diagnostics.stpp_mfu_completed, 1);
    assert_eq!(diagnostics.stpp_payload_bytes, ttml.len() as u64);
}

#[test]
fn attaches_same_mpu_resources_to_a_stpp_caption_payload() {
    let resource = b"resource";
    let mut resource_unit = vec![0, 0, 1, 1, 0x10];
    resource_unit.extend((resource.len() as u16).to_be_bytes());
    resource_unit.extend(resource);
    let mut resource_mfu = vec![0, 0, 0, 0];
    resource_mfu.extend(resource_unit);
    let mut resource_mpu = vec![0x20, 0];
    resource_mpu.extend(42_u32.to_be_bytes());
    resource_mpu.extend(resource_mfu);
    let mut resource_payload = Vec::new();
    resource_payload.extend((resource_mpu.len() as u16).to_be_bytes());
    resource_payload.extend(resource_mpu);
    let mut resource_mmtp = vec![0, 0x00, 0x04, 0x59, 0, 0, 0, 0];
    resource_mmtp.extend(8_u32.to_be_bytes());
    resource_mmtp.extend(resource_payload);

    let ttml = b"<tt><body><p>caption</p></body></tt>";
    let mut caption_unit = vec![0, 0, 0, 1, 0];
    caption_unit.extend((ttml.len() as u16).to_be_bytes());
    caption_unit.extend(ttml);
    let mut caption_mfu = vec![0, 0, 0, 0];
    caption_mfu.extend(caption_unit);
    let mut caption_mpu = vec![0x20, 0];
    caption_mpu.extend(42_u32.to_be_bytes());
    caption_mpu.extend(caption_mfu);
    let mut caption_payload = Vec::new();
    caption_payload.extend((caption_mpu.len() as u16).to_be_bytes());
    caption_payload.extend(caption_mpu);
    let mut caption_mmtp = vec![0, 0x00, 0x04, 0x59, 0, 0, 0, 0];
    caption_mmtp.extend(9_u32.to_be_bytes());
    caption_mmtp.extend(caption_payload);

    let resource_packet = parse_mmtp_packet(&resource_mmtp).expect("resource packet");
    let caption_packet = parse_mmtp_packet(&caption_mmtp).expect("caption packet");
    let mut diagnostics = TlvDiagnostics::default();
    diagnostics.mpt_assets.insert(0x0459, "stpp".into());
    let mut signalling_assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    let mut captured = Vec::new();
    inspect_mmtp_packet(
        &resource_packet,
        &mut diagnostics,
        &mut signalling_assemblers,
        &mut mpu_assemblers,
        Some(&mut captured),
        None,
    );
    assert!(captured.is_empty());
    inspect_mmtp_packet(
        &caption_packet,
        &mut diagnostics,
        &mut signalling_assemblers,
        &mut mpu_assemblers,
        Some(&mut captured),
        None,
    );
    assert_eq!(captured.len(), 1);
    assert!(captured[0].resources_complete);
    assert_eq!(captured[0].resources[0].index, 1);
    assert_eq!(captured[0].resources[0].data_type, 1);
    assert_eq!(captured[0].resources[0].bytes, resource);
}

#[test]
fn captures_a_bounded_non_stpp_mpu_as_raw_asset_evidence() {
    let resource = b"resource-bytes";
    let mut mfu = vec![0, 0, 0, 0];
    mfu.extend(resource);
    let mut mpu = vec![0x20, 0];
    mpu.extend(7_u32.to_be_bytes());
    mpu.extend(mfu);
    let mut payload = Vec::new();
    payload.extend((mpu.len() as u16).to_be_bytes());
    payload.extend(mpu);
    let mut mmtp = vec![0, 0x00, 0x04, 0x5a, 0, 0, 0, 0];
    mmtp.extend(9_u32.to_be_bytes());
    mmtp.extend(payload);
    let packet = parse_mmtp_packet(&mmtp).expect("MPU packet");
    let mut diagnostics = TlvDiagnostics::default();
    diagnostics.mpt_assets.insert(0x045a, "font".into());
    let mut signalling_assemblers = BTreeMap::new();
    let mut mpu_assemblers = BTreeMap::new();
    let mut captions = Vec::new();
    let mut assets = Vec::new();
    inspect_mmtp_packet(
        &packet,
        &mut diagnostics,
        &mut signalling_assemblers,
        &mut mpu_assemblers,
        Some(&mut captions),
        Some(&mut assets),
    );
    assert!(captions.is_empty());
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].asset_type, "font");
    assert_eq!(assets[0].mpu_sequence_number, 7);
    assert_eq!(assets[0].bytes, resource);
    assert_eq!(diagnostics.non_stpp_mfu_completed, 1);
}

#[test]
fn dumps_tlv_stpp_payloads_as_streamed_raw_jsonl() {
    let ttml = b"<?xml version=\"1.0\"?><tt><body><p begin=\"00:00:00.000\" end=\"00:00:02.000\" tts:color=\"#12AB34\" tts:fontSize=\"42px\" tts:writingMode=\"tbrl\">raw <ruby><span tts:ruby=\"base\">evidence</span><rt><span tts:ruby=\"text\">proof</span></rt></ruby></p></body></tt>";
    let mut mpt_body = vec![0, 0, 0, 0, 1];
    mpt_body.extend([0, 0, 0, 0, 0, 0]);
    mpt_body.extend(*b"stpp");
    mpt_body.extend([0, 1, 0, 0x04, 0x5a]);
    let presentation_ntp = 0x1234_5678_9abc_def0_u64;
    mpt_body.extend(15_u16.to_be_bytes());
    mpt_body.extend([0, 1, 12]);
    mpt_body.extend(1_u32.to_be_bytes());
    mpt_body.extend(presentation_ntp.to_be_bytes());
    let mut table = vec![0x20, 0];
    table.extend((mpt_body.len() as u16).to_be_bytes());
    table.extend(mpt_body);
    let mut signal_message = vec![0x80, 0, 0];
    signal_message.extend((table.len() as u16).to_be_bytes());
    signal_message.extend(table);
    let mut signal_mmtp = vec![0, 0x02, 0x04, 0x5a, 0, 0, 0, 0];
    signal_mmtp.extend(1_u32.to_be_bytes());
    signal_mmtp.extend([0, 0]);
    signal_mmtp.extend(signal_message);

    let mut closed_caption = vec![0, 0, 0, 0, 0];
    closed_caption.extend((ttml.len() as u16).to_be_bytes());
    closed_caption.extend(ttml);
    let mut mfu = vec![0, 0, 0, 0];
    mfu.extend(closed_caption);
    let mut mpu = vec![0x20, 0];
    mpu.extend(1_u32.to_be_bytes());
    mpu.extend(mfu);
    let mut mpu_payload = Vec::new();
    mpu_payload.extend((mpu.len() as u16).to_be_bytes());
    mpu_payload.extend(mpu);
    let mut mpu_mmtp = vec![0, 0x00, 0x04, 0x5a, 0, 0, 0, 0];
    mpu_mmtp.extend(2_u32.to_be_bytes());
    mpu_mmtp.extend(mpu_payload);

    let tlv_packet = |packet_type: u8, payload: Vec<u8>| {
        let mut packet = vec![0x7f, packet_type];
        packet.extend((payload.len() as u16).to_be_bytes());
        packet.extend(payload);
        packet
    };
    let mut input = Vec::new();
    let mut signal_hcfb = vec![0, 0, 0x61];
    signal_hcfb.extend(signal_mmtp);
    input.extend(tlv_packet(0x03, signal_hcfb));
    let mut mpu_hcfb = vec![0, 0, 0x61];
    mpu_hcfb.extend(mpu_mmtp);
    input.extend(tlv_packet(0x03, mpu_hcfb));
    for packet_type in 0..2 {
        input.extend(tlv_packet(packet_type, vec![0]));
    }
    let stem = format!("arib-caption-tlv-dump-{}", std::process::id());
    let input_path = std::env::temp_dir().join(format!("{stem}.tlv"));
    let output_path = std::env::temp_dir().join(format!("{stem}.jsonl"));
    #[cfg(not(feature = "libaribtlv"))]
    let converted_path = std::env::temp_dir().join(format!("{stem}.ass"));
    fs::write(&input_path, input).expect("fixture");
    let summary = dump_tlv_stpp_raw(&input_path, &output_path, false).expect("raw dump");
    assert_eq!(summary.stpp_payloads, 1);
    assert_eq!(summary.stpp_payload_bytes, ttml.len() as u64);
    let raw = fs::read_to_string(&output_path).expect("raw output");
    assert!(raw.contains("stpp_closed_caption_payload"));
    assert!(raw.contains(&hex_encode(ttml)));
    assert!(raw.contains(&presentation_ntp.to_string()));
    assert!(raw.contains("\"pts_ms\":null"));
    // The compact protocol fixture above intentionally targets the project's
    // bounded Rust envelope parser. It is not a full libaribtlv-compliant MMT
    // stream; the native backend has its own bridge and callback fixtures.
    #[cfg(not(feature = "libaribtlv"))]
    {
        let options = ConversionOptions {
            ttml: true,
            archive: true,
            raw: true,
            ..ConversionOptions::default()
        };
        let report = convert_with_options_and_cancel(
            &input_path,
            &converted_path,
            options,
            |_| {},
            || false,
        )
        .expect("clocked TTML conversion");
        assert_eq!(report.summary.captions, 1);
        assert_eq!(report.summary.decoder_errors, 0);
        let ass = fs::read_to_string(&converted_path).expect("ASS output");
        assert!(ass.contains("}raw "));
        assert!(ass.contains("}evidence"));
        assert!(ass.contains("Dialogue: 1,"));
        let ruby_lines = ass
            .lines()
            .filter(|line| line.starts_with("Dialogue: 1,"))
            .collect::<Vec<_>>();
        assert_eq!(ruby_lines.len(), 5);
        for character in "proof".chars() {
            assert!(ruby_lines.iter().any(|line| line.ends_with(character)));
        }
        assert!(!ass.contains("raw evidenceproof"));
        assert!(ass.contains("\\c&H0034AB12&\\fs42"));
        assert!(
            fs::read_to_string(report.raw.as_ref().expect("raw output"))
                .expect("raw conversion output")
                .contains(&presentation_ntp.to_string())
        );
        let archive = fs::read_to_string(report.archive.as_ref().expect("archive output"))
            .expect("archive conversion output");
        assert!(archive.contains("isdb_s3_tlv_mmtp_stpp"));
        assert!(archive.contains("\"ruby_bindings\""));
        assert!(archive.contains("\"base_text\":\"evidence\""));
        assert!(
            fs::read_to_string(report.ttml.as_ref().expect("TTML output"))
                .expect("TTML conversion output")
                .contains("tts:writingMode=\"vertical-rl\"")
        );
        assert!(
            fs::read_to_string(report.ttml.as_ref().expect("TTML output"))
                .expect("TTML conversion output")
                .contains("tts:ruby=\"text\"")
        );
        let preview = preview_caption(&input_path)
            .expect("TLV preview")
            .expect("caption preview");
        assert_eq!(preview.text, "raw evidenceproof");
        assert!((0.0..=1.0).contains(&preview.x));
        assert!((0.0..=1.0).contains(&preview.y));
        assert_eq!(preview.text_color, 0xff12_ab34);
        assert_eq!(preview.background_color, 0xb000_0000);
        fs::remove_file(input_path).expect("cleanup input");
        fs::remove_file(output_path).expect("cleanup output");
        fs::remove_file(converted_path).expect("cleanup ASS");
        fs::remove_file(report.ttml.expect("TTML output")).expect("cleanup TTML");
        fs::remove_file(report.archive.expect("archive output")).expect("cleanup archive");
        fs::remove_file(report.raw.expect("raw output")).expect("cleanup conversion raw");
    }
}

#[test]
fn mpu_fragment_sequence_gap_is_dropped() {
    let mut diagnostics = TlvDiagnostics::default();
    let mut assemblers = BTreeMap::new();
    assert_eq!(
        assemble_mpu_fragment(
            &mut assemblers,
            0x0123,
            4,
            100,
            0b01,
            b"first",
            &mut diagnostics,
        ),
        Some(Vec::new())
    );
    assert_eq!(
        assemble_mpu_fragment(
            &mut assemblers,
            0x0123,
            4,
            102,
            0b11,
            b"last",
            &mut diagnostics,
        ),
        None
    );
    assert_eq!(diagnostics.stpp_mfu_dropped, 1);
}

#[test]
fn extracts_hcfb_context_61_mmtp_payload_without_guessing_headers() {
    let mmtp = [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
    let mut hcfb = vec![0, 0, 0x61];
    hcfb.extend(mmtp);
    assert_eq!(tlv_mmtp_payload(0x03, &hcfb), Some(mmtp.as_slice()));
    assert_eq!(tlv_mmtp_payload(0x03, &[0, 0, 0x5f]), None);
}
