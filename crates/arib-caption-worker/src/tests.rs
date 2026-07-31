use super::*;
use base64::Engine;
use std::env;

#[test]
fn archive_only_publishes_the_archive_at_the_requested_target() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("archive-only-{stamp}.jsonl"));
    let archive = std::env::temp_dir().join(format!("archive-only-{stamp}.caption.jsonl"));
    fs::write(&output, "[Script Info]\n").expect("temporary primary output");
    fs::write(&archive, "{\"type\":\"arib_caption_studio_archive\"}\n").expect("archive output");
    let mut report = ConversionReport {
        output: output.clone(),
        ass: Some(output.clone()),
        font_directory: None,
        drcs_directory: None,
        drcs_report: None,
        ttml: None,
        archive: Some(archive.clone()),
        raw: None,
        srt: None,
        webvtt: None,
        summary: B24DecodeSummary::default(),
    };

    crate::cli::publish_archive_only(&output, &mut report).expect("publish archive only");

    assert_eq!(report.output, output);
    assert_eq!(report.archive.as_deref(), Some(output.as_path()));
    assert!(!archive.exists());
    assert!(!output.with_extension("jsonl.backup").exists());
    assert!(
        fs::read_to_string(&output)
            .expect("published archive")
            .contains("arib_caption_studio_archive")
    );
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn artifact_events_report_unique_completed_outputs() {
    let report = ConversionReport {
        output: PathBuf::from("captions.ass"),
        ass: Some(PathBuf::from("captions.ass")),
        font_directory: Some(PathBuf::from("captions.fonts")),
        drcs_directory: Some(PathBuf::from("captions.drcs")),
        drcs_report: Some(PathBuf::from("captions.drcs.json")),
        ttml: Some(PathBuf::from("captions.ttml")),
        archive: Some(PathBuf::from("captions.caption.jsonl")),
        raw: Some(PathBuf::from("captions.caption.pes.jsonl")),
        srt: None,
        webvtt: None,
        summary: B24DecodeSummary::default(),
    };
    let events = crate::cli::artifact_events(&report);
    assert_eq!(events.len(), 7);
    assert_eq!(events[0]["kind"], "captions");
    assert_eq!(events[1]["kind"], "font-directory");
    assert!(events.iter().all(|event| event["status"] == "completed"));
}

#[test]
fn ass_font_sidecar_contains_the_bundled_arib_font_and_license() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-font-sidecar-{stamp}.ass"));
    let directory = write_ass_font_directory(&output, false).expect("write font sidecar");
    let font = directory.join("rounded-mplus-1m-arib.ttf");
    let license = directory.join("LICENSE.rounded-mplus-1m-arib.txt");
    assert_eq!(fs::read(&font).expect("font"), bundled_ass_font());
    assert!(
        fs::read_to_string(&license)
            .expect("license")
            .contains("Rounded M+ 1m for ARIB")
    );
    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn canonicalises_b62_writing_mode_aliases() {
    assert_eq!(canonical_writing_mode("lrtb"), "horizontal-tb");
    assert_eq!(canonical_writing_mode("rltb"), "horizontal-tb");
    assert_eq!(canonical_writing_mode("tblr"), "vertical-lr");
    assert_eq!(canonical_writing_mode("tbrl"), "vertical-rl");
    assert_eq!(canonical_writing_mode("vertical-rl"), "vertical-rl");
}

#[test]
fn preserves_b62_horizontal_legacy_direction() {
    let captions = parse_ttml_captions(
        "<tt><body><p begin='0s' end='1s' tts:writingMode='rltb'>右から</p></body></tt>",
        0,
    );
    assert_eq!(
        captions[0].style.writing_mode.as_deref(),
        Some("horizontal-tb")
    );
    assert_eq!(captions[0].style.direction.as_deref(), Some("rtl"));

    let explicit = parse_ttml_captions(
        "<tt><body><p begin='0s' end='1s' tts:writingMode='rltb' tts:direction='ltr'>明示</p></body></tt>",
        0,
    );
    assert_eq!(explicit[0].style.direction.as_deref(), Some("ltr"));
}

#[test]
fn b62_layout_golden_fixture_keeps_timing_regions_and_ruby_evidence() {
    let captions = parse_ttml_captions(include_str!("../testdata/golden/b62-layout.xml"), 0);
    let actual = captions
        .iter()
        .map(|caption| {
            serde_json::json!({
                "start_ms": caption.start_ms,
                "end_ms": caption.end_ms,
                "text": caption.text,
                "x": caption.x,
                "y": caption.y,
                "width": caption.width,
                "height": caption.height,
                "writing_mode": caption.style.writing_mode,
                "font_size": caption.style.font_size,
                "color": caption.style.color,
            })
        })
        .collect::<Vec<_>>();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../testdata/golden/b62-layout.expected.json"))
            .expect("golden JSON");
    assert_eq!(serde_json::Value::Array(actual), expected);
    assert!(
        captions[0]
            .rich_body
            .as_deref()
            .is_some_and(|body| body.contains("tts:ruby=\"text\""))
    );
}

#[test]
fn preserves_b62_resource_references_in_the_caption_model() {
    let xml = r#"<tt><body><div><p begin="0s" end="1s" smpte:backgroundImage="subt://4" arib-tt:font-face="subt://9">字幕</p></div></body></tt>"#;
    let captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 1);
    assert_eq!(
        captions[0].style.background_image.as_deref(),
        Some("subt://4")
    );
    assert_eq!(captions[0].style.font_resource.as_deref(), Some("subt://9"));
}

#[test]
fn records_only_well_formed_subt_resource_references() {
    let xml = r#"<tt><body><div><p begin="0s" end="1s" smpte:backgroundImage="subt://4" arib-tt:font-face="subt://bad">字幕</p></div></body></tt>"#;
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    let references = ttml_resource_references(&caption);
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].resource_index, 4);
    assert_eq!(references[0].usage, "background-image");
    assert_eq!(subt_resource_index("subt://0009"), Some(9));
    assert_eq!(subt_resource_index("subt://9/path"), None);
    assert_eq!(subt_resource_index("https://example.invalid/9"), None);
}

#[test]
fn scopes_subt_references_to_the_caption_mpu() {
    let xml = r#"<tt><body><div><p begin="0s" end="1s" smpte:backgroundImage="subt://4">字幕</p></div></body></tt>"#;
    let mut first = parse_ttml_captions(xml, 0).pop().expect("caption");
    first.source = Some(TtmlCaptionSource {
        route: "isdb_s3_tlv_mmtp_stpp",
        source_offset: 10,
        mmpt_packet_id: 0x0459,
        mpu_sequence_number: 7,
        mmtp_sequence_number: 11,
        presentation_ntp: 12,
        xml_encoding: "UTF-8".into(),
        resources: Vec::new(),
        resources_complete: false,
    });
    let mut second = first.clone();
    second.source.as_mut().unwrap().mpu_sequence_number = 8;

    let first_reference = &ttml_resource_references(&first)[0];
    let second_reference = &ttml_resource_references(&second)[0];
    assert_eq!(
        first_reference.association.scope,
        Some(TtmlResourceScope {
            packet_id: 0x0459,
            mpu_sequence_number: 7,
        })
    );
    assert_eq!(
        first_reference.association.scope_key.as_deref(),
        Some("packet:1113:mpu:7")
    );
    assert_ne!(
        first_reference.association.scope_key,
        second_reference.association.scope_key
    );
    assert_eq!(first_reference.association.status, "unresolved");
    assert!(first_reference.association.reason.contains("MPU"));
}

#[test]
fn matches_subt_reference_to_same_mpu_resource_evidence() {
    let xml = r#"<tt><body><div><p begin="0s" end="1s" smpte:backgroundImage="subt://4">字幕</p></div></body></tt>"#;
    let mut caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    caption.source = Some(TtmlCaptionSource {
        route: "isdb_s3_tlv_mmtp_stpp",
        source_offset: 10,
        mmpt_packet_id: 0x0459,
        mpu_sequence_number: 7,
        mmtp_sequence_number: 11,
        presentation_ntp: 12,
        xml_encoding: "UTF-8".into(),
        resources: vec![TtmlResourceMetadata {
            index: 4,
            data_type: 1,
            byte_length: 12,
            format_hint: Some("png"),
            format_validation: "header-validated",
            width: Some(1920),
            height: Some(1080),
            preview_available: true,
        }],
        resources_complete: true,
    });

    let reference = &ttml_resource_references(&caption)[0];
    assert_eq!(reference.association.status, "same-mpu-evidence");
    assert_eq!(reference.association.resource_data_type, Some(1));
    assert_eq!(reference.association.resource_format_hint, Some("png"));
    assert_eq!(
        reference.association.resource_format_validation,
        Some("header-validated")
    );
    assert_eq!(reference.association.resource_width, Some(1920));
    assert_eq!(reference.association.resource_height, Some(1080));
    assert_eq!(reference.association.resource_preview_available, Some(true));
    assert_eq!(
        reference.association.resource_record_key.as_deref(),
        Some("stpp-resource:packet:1113:mpu:7:subsample:4")
    );
}

#[test]
fn parses_b62_resource_subsamples_without_treating_them_as_ttml() {
    let mut unit = vec![0, 0, 2, 2, 0x10];
    unit.extend((8_u16).to_be_bytes());
    unit.extend(b"resource");
    let parsed = parse_subtitle_mfu_payload(&unit).expect("resource MFU");
    assert_eq!(parsed.subsample_number, 2);
    assert_eq!(parsed.last_subsample_number, 2);
    assert_eq!(parsed.data_type, 1);
    assert_eq!(parsed.payload, b"resource");

    let mut state = TlvSubtitleResourceState::default();
    state.add(TlvSubtitleResource {
        index: parsed.subsample_number,
        data_type: parsed.data_type,
        bytes: parsed.payload.to_vec(),
    });
    assert!(!state.is_complete(parsed.last_subsample_number));
    assert_eq!(state.resources.get(&2).unwrap().bytes, b"resource");
}

#[test]
fn resource_scope_key_matches_raw_asset_scope_key() {
    let scope = TtmlResourceScope {
        packet_id: 0x045a,
        mpu_sequence_number: 9,
    };
    assert_eq!(scope.key(), "packet:1114:mpu:9");
}

#[test]
fn reports_only_bounded_binary_format_signatures_for_raw_assets() {
    assert_eq!(
        bounded_payload_format_hint(b"\x89PNG\r\n\x1a\nrest"),
        Some("png")
    );
    assert_eq!(
        bounded_payload_format_hint(b"\xff\xd8\xffrest"),
        Some("jpeg")
    );
    assert_eq!(
        bounded_payload_format_hint(b"RIFF\x10\0\0\0WEBPrest"),
        Some("webp")
    );
    assert_eq!(bounded_payload_format_hint(b"wOF2rest"), Some("woff2"));
    assert_eq!(bounded_payload_format_hint(b"<svg>"), None);
    assert_eq!(bounded_payload_format_hint(b"unknown"), None);
}

#[test]
fn validates_bounded_png_and_font_headers_without_decoding_payloads() {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend(1920_u32.to_be_bytes());
    png.extend(1080_u32.to_be_bytes());
    let png_info = bounded_resource_format(&png);
    assert_eq!(png_info.format_hint, Some("png"));
    assert_eq!(png_info.format_validation, "header-validated");
    assert_eq!((png_info.width, png_info.height), (Some(1920), Some(1080)));

    let mut woff = b"wOFF\0\x01\0\0".to_vec();
    woff.extend(44_u32.to_be_bytes());
    woff.extend(2_u16.to_be_bytes());
    woff.resize(44, 0);
    let woff_info = bounded_resource_format(&woff);
    assert_eq!(woff_info.format_hint, Some("woff"));
    assert_eq!(woff_info.format_validation, "header-validated");

    let png = base64::engine::general_purpose::STANDARD
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=")
            .expect("fixture PNG");
    let preview = bounded_png_preview_data_uri(&png).expect("bounded PNG preview");
    assert!(preview.starts_with("data:image/png;base64,"));
    assert!(!png_has_bounded_image_chunks(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn reads_hexadecimal_and_decimal_drcs_mapping_codes() {
    let path =
        std::env::temp_dir().join(format!("arib-caption-drcs-map-{}.json", std::process::id()));
    fs::write(&path, r#"{"0x2A7F":"秘","10880":"替"}"#).unwrap();
    let mapping = load_drcs_mapping(&path).unwrap();
    let _ = fs::remove_file(path);
    assert_eq!(mapping.get(&0x2A7F), Some(&"秘".to_string()));
    assert_eq!(mapping.get(&10880), Some(&"替".to_string()));
}

#[test]
fn json_line_writer_reports_a_broken_pipe_without_panicking() {
    struct BrokenPipe;
    impl Write for BrokenPipe {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "reader closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut writer = BrokenPipe;
    assert!(write_json_line(&mut writer, &serde_json::json!({"type": "progress"})).is_err());
}

#[test]
fn versioned_events_preserve_legacy_fields_and_add_envelope_metadata() {
    let event = versioned_event(&serde_json::json!({"type": "progress", "bytes_read": 42}))
        .expect("event should serialize");
    assert_eq!(
        event.get("type").and_then(|value| value.as_str()),
        Some("progress")
    );
    assert_eq!(
        event
            .get("protocolVersion")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        event
            .get("payload")
            .and_then(|value| value.get("bytes_read"))
            .and_then(|value| value.as_u64()),
        Some(42)
    );
    assert!(event.get("jobId").is_some());
    assert!(event.get("sequence").is_some());
}

#[test]
fn detects_mpeg_ts() {
    let mut bytes = vec![0; 188 * 5];
    for index in (0..bytes.len()).step_by(188) {
        bytes[index] = 0x47;
        bytes[index + 3] = 0x10;
    }
    assert_eq!(probe_bytes(&bytes).kind, InputKind::MpegTs);
}

#[test]
fn detects_m2ts() {
    let mut bytes = vec![0; 192 * 5];
    for index in (4..bytes.len()).step_by(192) {
        bytes[index] = 0x47;
        bytes[index + 3] = 0x10;
    }
    assert_eq!(probe_bytes(&bytes).kind, InputKind::M2ts);
}

#[test]
fn inspection_keeps_m2ts_private_pes_as_a_ttml_candidate() {
    let path = std::env::temp_dir().join(format!(
        "arib-caption-m2ts-candidate-{}.m2ts",
        std::process::id()
    ));
    fs::write(&path, m2ts_from_ts_packets(&private_pes_ttml_ts_fixture())).expect("fixture");

    let inspection = inspect_input(&path).expect("inspection");
    assert_eq!(inspection.route_code, "mpeg_ts_ttml_candidate");
    assert!(inspection.route.contains("strict XML extraction"));
    fs::remove_file(path).expect("cleanup");
}

#[test]
fn inspection_does_not_verify_m2ts_without_a_private_pes_candidate() {
    let path = std::env::temp_dir().join(format!(
        "arib-caption-m2ts-without-caption-{}.m2ts",
        std::process::id()
    ));
    let mut bytes = vec![0; 192 * 5];
    for index in (0..bytes.len()).step_by(192) {
        bytes[index + 4] = 0x47;
        bytes[index + 7] = 0x10;
    }
    fs::write(&path, bytes).expect("fixture");

    let inspection = inspect_input(&path).expect("inspection");
    assert_eq!(inspection.route_code, "unknown_unsupported");
    assert!(inspection.tracks.is_empty());
    fs::remove_file(path).expect("cleanup");
}

#[path = "tests/tlv.rs"]
mod tlv;
#[test]
fn inspection_reports_unknown_input_without_scanning_the_whole_file() {
    let path =
        std::env::temp_dir().join(format!("arib-caption-unknown-{}.bin", std::process::id()));
    fs::write(&path, b"not a transport stream").expect("fixture");
    let inspection = inspect_input(&path).expect("inspection");
    assert_eq!(inspection.container, "Unknown container");
    assert_eq!(inspection.bytes, 22);
    assert!(inspection.tracks.is_empty());
    fs::remove_file(path).expect("cleanup");
}

#[test]
fn rejects_unrecognised_input() {
    assert_eq!(probe_bytes(&[0; 4096]).kind, InputKind::Unknown);
}

#[path = "tests/b24_timeline.rs"]
mod b24_timeline;
#[path = "tests/ttml.rs"]
mod ttml;
#[test]
fn overwrite_publishes_complete_replacement_without_leaving_backup() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-ass-{stamp}.ass"));
    let temporary = output.with_extension("ass.part");
    fs::write(&output, "old").expect("old output");
    fs::write(&temporary, "new").expect("new output");
    publish_file(&temporary, &output, true).expect("replace output");
    assert_eq!(
        fs::read_to_string(&output).expect("published output"),
        "new"
    );
    assert!(!output.with_extension("ass.backup").exists());
    fs::remove_file(output).expect("cleanup output");
}

#[test]
fn archive_is_json_lines_with_a_header_and_summary() {
    let path =
        std::env::temp_dir().join(format!("arib-caption-archive-{}.jsonl", std::process::id()));
    let mut writer = BufWriter::new(File::create(&path).expect("archive"));
    write_archive_header(&mut writer, Path::new("sample.ts"), "arib_std_b24").expect("header");
    write_archive_record(&mut writer, "summary", &B24DecodeSummary::default()).expect("summary");
    writer.flush().expect("flush");
    let lines = fs::read_to_string(&path)
        .expect("archive text")
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("json line"))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["format"], "jsonl");
    assert_eq!(lines[0]["source"], "sample.ts");
    assert_eq!(lines[1]["type"], "summary");
    fs::remove_file(path).expect("cleanup");
}

#[test]
fn raw_pes_export_is_jsonl_with_source_metadata_and_lossless_hex() {
    let path = std::env::temp_dir().join(format!("arib-caption-raw-{}.jsonl", std::process::id()));
    let mut writer = BufWriter::new(File::create(&path).expect("raw"));
    write_raw_header(&mut writer, Path::new("sample.ts"), "arib_std_b24").expect("header");
    write_raw_pes_record(&mut writer, 0x1200, 188, &[0, 0, 1, 0xbd, 0x7f]).expect("record");
    writer.flush().expect("flush");
    let lines = fs::read_to_string(&path)
        .expect("raw text")
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("json line"))
        .collect::<Vec<_>>();
    assert_eq!(lines[0]["type"], "arib_caption_raw_pes");
    assert_eq!(lines[1]["pid"], 0x1200);
    assert_eq!(lines[1]["packet_offset"], 188);
    assert_eq!(lines[1]["pes_hex"], "000001bd7f");
    fs::remove_file(path).expect("cleanup");
}

#[test]
fn webvtt_compatibility_export_writes_cues_from_ass_dialogue() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let ass = std::env::temp_dir().join(format!("arib-webvtt-{stamp}.ass"));
    fs::write(
        &ass,
        "[Events]\nDialogue: 0,0:00:01.20,0:00:02.34,Default,,0,0,0,,{\\pos(10,20)}字幕\\N次行\n",
    )
    .expect("ASS fixture");
    let vtt = write_webvtt_from_ass(&ass, false)
        .expect("export WebVTT")
        .expect("WebVTT path");
    let text = fs::read_to_string(&vtt).expect("read WebVTT");
    assert!(text.starts_with("WEBVTT"));
    assert!(text.contains("00:00:01.200 --> 00:00:02.340"));
    assert!(text.contains("字幕\n次行"));
    fs::remove_file(ass).expect("cleanup ASS");
    fs::remove_file(vtt).expect("cleanup WebVTT");
}

#[test]
fn webvtt_text_removes_ass_tags_and_marks_drcs_drawings() {
    assert_eq!(
        ass_to_webvtt_text("{\\an7\\pos(10,20)}字幕\\N次行"),
        "字幕\n次行"
    );
    assert_eq!(ass_to_webvtt_text("{\\p1}m 0 0 l 2 0"), "[DRCS glyph]");
    assert_eq!(ass_to_webvtt_text("{\\p1}m 0 0{\\p0}文字"), "文字");
}

#[test]
fn srt_compatibility_export_writes_plain_numbered_cues() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let ass = std::env::temp_dir().join(format!("arib-srt-{stamp}.ass"));
    fs::write(
        &ass,
        "[Events]\nDialogue: 0,0:00:01.20,0:00:02.34,Default,,0,0,0,,{\\pos(10,20)}字幕\\N次行\n",
    )
    .expect("ASS fixture");
    let srt = write_srt_from_ass(&ass, false)
        .expect("export SRT")
        .expect("SRT path");
    let text = fs::read_to_string(&srt).expect("read SRT");
    assert!(text.starts_with("1\n00:00:01,200 --> 00:00:02,340"));
    assert!(text.contains("字幕\n次行"));
    fs::remove_file(ass).expect("cleanup ASS");
    fs::remove_file(srt).expect("cleanup SRT");
}

#[test]
fn no_ass_selection_keeps_only_requested_compatibility_output() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let ass = std::env::temp_dir().join(format!("arib-no-ass-{stamp}.ass"));
    fs::write(
        &ass,
        "[Events]\nDialogue: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,字幕\n",
    )
    .expect("ASS fixture");
    let options = ConversionOptions {
        srt: true,
        keep_ass: false,
        ..ConversionOptions::default()
    };
    let (kept_ass, font_directory, srt, webvtt) =
        finalize_ass_outputs(&ass, &options).expect("finalize selected outputs");
    assert!(kept_ass.is_none());
    assert!(font_directory.is_none());
    assert!(webvtt.is_none());
    assert!(!ass.exists());
    let srt = srt.expect("SRT retained");
    assert!(srt.exists());
    fs::remove_file(srt).expect("cleanup SRT");
}

#[path = "tests/corpus.rs"]
mod corpus;
#[path = "tests/transport_ts.rs"]
mod transport_ts;
use transport_ts::{m2ts_from_ts_packets, private_pes_ttml_ts_fixture};
