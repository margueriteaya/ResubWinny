use super::*;

#[test]
fn parses_namespace_conformant_arib_ttml_without_element_timing_until_next_document() {
    let xml = r##"<?xml version="1.0" encoding="utf-8"?>
<tt xmlns="http://www.w3.org/ns/ttml"
    xmlns:tts="http://www.w3.org/ns/ttml#styling"
    xmlns:arib-tt="http://www.arib.or.jp/ns/arib-ttml/v1_0">
  <head><styling><style xml:id="font" tts:fontSize="144px 144px"/></styling>
    <layout><region xml:id="display" tts:extent="2480px 1920px" tts:origin="680px 1560px"/></layout>
  </head>
  <body><div><p region="display"><span style="font">字幕です。</span></p></div></body>
</tt>"##;

    assert!(parse_ttml_captions(xml, 2_000).is_empty());
    let captions = parse_ttml_captions_until(xml, 2_000, Some(4_750));
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].text, "字幕です。");
    assert_eq!((captions[0].start_ms, captions[0].end_ms), (2_000, 4_750));
}

#[test]
fn parses_namespace_prefixed_ttml_elements_by_local_name() {
    let xml = r#"<tt:tt xmlns:tt="http://www.w3.org/ns/ttml"><tt:body><tt:div>
      <tt:p begin="1s" end="2s">prefixed</tt:p>
    </tt:div></tt:body></tt:tt>"#;
    let captions = parse_ttml_captions(xml, 500);
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].text, "prefixed");
    assert_eq!((captions[0].start_ms, captions[0].end_ms), (1_500, 2_500));
}

#[test]
fn rejects_private_pes_with_zero_filled_fake_pts() {
    let pes = [
        0x00, 0x00, 0x01, 0xbd, 0x00, 0x20, 0x80, 0x80, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(pes_pts_from_header(&pes), None);
}

fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

fn render_ass_fixture(
    directory: &Path,
    ass: &str,
    image_name: &str,
    timestamp: &str,
) -> image::RgbImage {
    fs::create_dir_all(directory).expect("visual test directory");
    fs::write(directory.join("fixture.ass"), ass).expect("write ASS fixture");
    let font = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/rounded-mplus-1m-arib/rounded-mplus-1m-arib.ttf");
    fs::copy(font, directory.join("rounded-mplus-1m-arib.ttf")).expect("copy test font");
    let filter = format!("ass=filename='fixture.ass':fontsdir='.',select='gte(t,{timestamp})'");
    let status = std::process::Command::new("ffmpeg")
        .current_dir(directory)
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x202124:s=1920x1080:r=10:d=2",
            "-vf",
            &filter,
            "-frames:v",
            "1",
            "-update",
            "1",
            image_name,
        ])
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "ffmpeg/libass visual render failed");
    image::open(directory.join(image_name))
        .expect("rendered PNG")
        .to_rgb8()
}

fn yellow_pixel_center(
    image: &image::RgbImage,
    x_range: std::ops::Range<u32>,
    y_range: std::ops::Range<u32>,
) -> f32 {
    let mut minimum = u32::MAX;
    let mut maximum = 0_u32;
    for y in y_range {
        for x in x_range.clone() {
            let pixel = image.get_pixel(x, y).0;
            if pixel[0] > 160 && pixel[1] > 160 && pixel[2] < 100 {
                minimum = minimum.min(x);
                maximum = maximum.max(x);
            }
        }
    }
    assert!(minimum <= maximum, "expected yellow caption pixels");
    (minimum + maximum) as f32 * 0.5
}

fn magenta_pixel_center(
    image: &image::RgbImage,
    x_range: std::ops::Range<u32>,
    y_range: std::ops::Range<u32>,
) -> f32 {
    let mut minimum = u32::MAX;
    let mut maximum = 0_u32;
    for y in y_range {
        for x in x_range.clone() {
            let pixel = image.get_pixel(x, y).0;
            if pixel[0] > 160 && pixel[1] < 100 && pixel[2] > 160 {
                minimum = minimum.min(x);
                maximum = maximum.max(x);
            }
        }
    }
    assert!(minimum <= maximum, "expected magenta caption pixels");
    (minimum + maximum) as f32 * 0.5
}

fn finish_visual_fixture(directory: &Path) {
    if std::env::var_os("RESUBWINNY_KEEP_VISUAL_TESTS").is_some() {
        eprintln!("kept visual fixture at {}", directory.display());
    } else {
        fs::remove_dir_all(directory).expect("cleanup visual test");
    }
}

#[test]
fn parses_ttml_div_timing_and_region() {
    let xml = r#"<tt><head><layout><region xml:id="r1" tts:origin="552px 1676px"/></layout></head><body><div begin="00:00:23.000" end="00:00:25.667"><p region="r1"><span>字幕 &amp; text</span></p></div></body></tt>"#;
    let captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].start_ms, 23_000);
    assert_eq!(captions[0].end_ms, 25_667);
    assert_eq!(captions[0].text, "字幕 & text");
    assert_eq!((captions[0].x, captions[0].y), (552, 1676));
}

#[test]
fn parses_ttml_offset_durations_single_quotes_and_line_breaks() {
    let xml = "<tt><body><div begin='1.5s' tts:color='#12AB34' tts:backgroundColor='#00000080' tts:fontSize='42px' tts:fontFamily='Noto Sans JP' tts:fontStyle='italic' tts:fontWeight='bold' tts:writingMode='tbrl' tts:textAlign='center' tts:textOutline='1px #000000' tts:lineHeight='48px' tts:letterSpacing='2px' tts:opacity='0.8' tts:displayAlign='after'><p dur='750ms'>first<br/>second</p></div></body></tt>";
    let captions = parse_ttml_captions(xml, 500);
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].start_ms, 2_000);
    assert_eq!(captions[0].end_ms, 2_750);
    assert_eq!(captions[0].text, "first\nsecond");
    assert_eq!(captions[0].style.color.as_deref(), Some("#12AB34"));
    assert_eq!(
        captions[0].style.background_color.as_deref(),
        Some("#00000080")
    );
    assert_eq!(captions[0].style.font_size.as_deref(), Some("42px"));
    assert_eq!(
        captions[0].style.font_family.as_deref(),
        Some("Noto Sans JP")
    );
    assert_eq!(captions[0].style.font_style.as_deref(), Some("italic"));
    assert_eq!(captions[0].style.font_weight.as_deref(), Some("bold"));
    assert_eq!(
        captions[0].style.writing_mode.as_deref(),
        Some("vertical-rl")
    );
    assert_eq!(captions[0].style.text_align.as_deref(), Some("center"));
    assert_eq!(
        captions[0].style.text_outline.as_deref(),
        Some("1px #000000")
    );
    assert_eq!(captions[0].style.line_height.as_deref(), Some("48px"));
    assert_eq!(captions[0].style.letter_spacing.as_deref(), Some("2px"));
    assert_eq!(captions[0].style.opacity.as_deref(), Some("0.8"));
    assert_eq!(captions[0].style.display_align.as_deref(), Some("after"));
    assert_eq!(ttml_time_ms("2m"), Some(120_000));
    assert_eq!(ttml_time_ms("0.5h"), Some(1_800_000));
    assert_eq!(
        ass_color_from_ttml("#12AB34").as_deref(),
        Some("&H0034AB12&")
    );
    assert_eq!(
        ass_color_from_ttml("#FFFF00FF").as_deref(),
        Some("&H0000FFFF&")
    );
    assert_eq!(ass_font_size_from_ttml("42px"), Some(42));
    assert_eq!(ass_font_size_from_ttml("144px 144px"), Some(144));
    assert_eq!(ass_letter_spacing_from_ttml("-2.2px"), Some(-2));
    assert_eq!(ass_letter_spacing_from_ttml("normal"), None);
}

#[test]
fn resolves_arib_ttml_span_style_references_into_interchange_markup() {
    let xml = r##"<tt><head><styling><style xml:id="family" tts:fontFamily="丸ゴシック" tts:fontWeight="normal"/><style xml:id="size" tts:fontSize="144px 144px" tts:lineHeight="240px" arib-tt:letter-spacing="16px"/><style xml:id="fore" tts:color="#FFFF00FF"/><style xml:id="back" tts:backgroundColor="#00000080"/></styling></head><body><div begin="0s" end="1s"><p><span style="family size fore back">字幕</span></p></div></body></tt>"##;
    let captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].style.font_family.as_deref(), Some("丸ゴシック"));
    assert_eq!(captions[0].style.font_size.as_deref(), Some("144px 144px"));
    assert_eq!(captions[0].style.color.as_deref(), Some("#FFFF00FF"));
    assert_eq!(
        captions[0].style.background_color.as_deref(),
        Some("#00000080")
    );
    assert_eq!(captions[0].style.letter_spacing.as_deref(), Some("16px"));
    let rich = captions[0]
        .rich_body
        .as_deref()
        .expect("expanded rich body");
    assert!(rich.contains("tts:fontFamily=\"丸ゴシック\""));
    assert!(rich.contains("tts:fontSize=\"144px 144px\""));
    assert!(rich.contains("tts:color=\"#FFFF00FF\""));
    assert!(rich.contains("tts:backgroundColor=\"#00000080\""));
    assert!(rich.contains("tts:letterSpacing=\"16px\""));
    assert!(!rich.contains("style=\"family size fore back\""));
}

#[test]
fn converts_ttml_rgba_colours_for_the_structural_preview() {
    assert_eq!(preview_color_from_ttml(Some("#123456"), 0), 0xff12_3456);
    assert_eq!(preview_color_from_ttml(Some("#12345680"), 0), 0x8012_3456);
    assert_eq!(
        preview_color_from_ttml(Some("invalid"), 0xaabb_ccdd),
        0xaabb_ccdd
    );
}

#[test]
fn writes_extended_ttml_styles_without_downgrading_them() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-ttml-style-{stamp}.ttml"));
    let xml = "<tt><body><p begin='0s' end='1s' tts:fontFamily='Noto Sans JP' tts:fontWeight='bold' tts:writingMode='rltb' tts:textOutline='1px #000000' tts:letterSpacing='2px' tts:opacity='0.8'>styled</p></body></tt>";
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ttml_header(&mut writer).expect("header");
    write_ttml_caption(&mut writer, &caption, &ConversionOptions::default()).expect("caption");
    write_ttml_footer(&mut writer).expect("footer");
    writer.flush().expect("flush");
    let text = fs::read_to_string(&output).expect("read");
    assert!(text.contains("tts:fontFamily=\"Noto Sans JP\""));
    assert!(text.contains("tts:fontWeight=\"bold\""));
    assert!(text.contains("tts:writingMode=\"horizontal-tb\""));
    assert!(text.contains("tts:direction=\"rtl\""));
    assert!(text.contains("tts:textOutline=\"1px #000000\""));
    assert!(text.contains("tts:letterSpacing=\"2px\""));
    assert!(text.contains("tts:opacity=\"0.8\""));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn preserves_safe_ttml_ruby_and_span_markup_for_ttml_interchange() {
    let xml = "<tt><body><p begin='0s' end='1s'>字幕 <ruby><span tts:ruby='base'>漢</span><rt><span tts:ruby='text'>かん</span></rt></ruby><br/><span tts:color='#ffffff' tts:textCombine='all'>24</span></p></body></tt>";
    let captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 1);
    assert!(
        captions[0]
            .rich_body
            .as_deref()
            .expect("safe rich body")
            .contains("tts:ruby='text'")
    );
    assert!(
        captions[0]
            .rich_body
            .as_deref()
            .expect("safe rich body")
            .contains("tts:textCombine='all'")
    );
    assert_eq!(captions[0].ruby_bindings.len(), 1);
    let binding = &captions[0].ruby_bindings[0];
    assert_eq!(binding.ruby_text, "かん");
    assert_eq!(binding.base_text, "漢");
    assert_eq!(binding.base_run_end - binding.base_run_start, 1);
    assert_eq!(binding.placement, RubyPlacement::Above);
    assert_eq!(safe_ttml_inline_body("<script>x</script>"), None);
}

#[test]
fn dropping_ruby_removes_annotations_but_keeps_styled_base_text() {
    let options = ConversionOptions {
        preserve_ruby: false,
        ..Default::default()
    };
    for body in [
        "<ruby><span tts:ruby='base' tts:color='#ff0000'>漢</span><rt><span>かん</span></rt></ruby><span>終</span>",
        "<span tts:ruby='container'><span tts:ruby='base' tts:color='#ff0000'>漢</span><span tts:ruby='text'>かん</span></span><span>終</span>",
    ] {
        let filtered = filter_ttml_preserved_body(body, &TtmlCaptionStyle::default(), &options)
            .expect("rich body");
        assert!(!filtered.contains("かん"));
        assert!(!filtered.contains("ruby"));
        assert!(filtered.contains("tts:color='#ff0000'"));
        assert_eq!(ttml_plain_text(&filtered), "漢終");
    }
}

#[test]
fn drops_only_resource_backed_b62_drcs_text() {
    let body = "<span arib-tt:font-face='subt://9'>&#xE000;</span><span arib-tt:font-face='subt://9'>字</span>終";
    let mut options = ConversionOptions {
        drcs_mode: DrcsMode::UseUserMapping,
        ..Default::default()
    };
    options.drcs_replacements.insert(0xe000, "映".into());
    let unresolved = filter_ttml_preserved_body(body, &TtmlCaptionStyle::default(), &options)
        .expect("unresolved rich body");
    assert_eq!(ttml_plain_text(&unresolved), "&#xE000;字終");

    options.preserve_drcs = false;
    let dropped = filter_ttml_preserved_body(body, &TtmlCaptionStyle::default(), &options)
        .expect("dropped rich body");
    assert_eq!(ttml_plain_text(&dropped), "字終");
}

fn attach_test_b62_resource(caption: &mut TtmlCaption, bytes: &[u8]) -> String {
    let digest = resource_sha256(bytes);
    caption.source = Some(TtmlCaptionSource {
        route: "test",
        source_offset: 0,
        mmpt_packet_id: 1,
        mpu_sequence_number: Some(1),
        mmtp_sequence_number: None,
        presentation_ntp: None,
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
        xml_encoding: "UTF-8".into(),
        resources: vec![TtmlResourceMetadata {
            index: 9,
            data_type: 1,
            byte_length: bytes.len(),
            content_sha256: digest.clone(),
            format_hint: Some("woff2"),
            format_validation: "header-validated",
            width: None,
            height: None,
            preview_available: false,
        }],
        resources_complete: true,
    });
    b62_drcs_mapping_key(&digest, 0xe000)
}

#[test]
fn scoped_b62_mapping_changes_ass_ttml_srt_and_webvtt_output() {
    let mut caption = parse_ttml_captions(
        r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body><p begin='0s' end='1s'><span arib-tt:font-face='subt://9'>&#xE000;</span>終</p></body></tt>"#,
        0,
    )
    .remove(0);
    let mapping_key = attach_test_b62_resource(&mut caption, b"font-resource-a");
    let mut options = ConversionOptions {
        drcs_mode: DrcsMode::UseUserMapping,
        overwrite: true,
        ..Default::default()
    };
    options
        .ttml_drcs_replacements
        .insert(mapping_key, "映".into());

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("arib-b62-scoped-map-{stamp}"));
    fs::create_dir_all(&directory).expect("temporary directory");
    let ass = directory.join("mapped.ass");
    let ttml = directory.join("mapped.ttml");
    let mut ass_writer = BufWriter::new(File::create(&ass).expect("ASS output"));
    write_ass_header(&mut ass_writer).expect("ASS header");
    write_ass_ttml_group(&mut ass_writer, std::slice::from_ref(&caption), &options)
        .expect("ASS caption");
    ass_writer.flush().expect("ASS flush");
    let mut ttml_writer = BufWriter::new(File::create(&ttml).expect("TTML output"));
    write_ttml_header(&mut ttml_writer).expect("TTML header");
    write_ttml_caption(&mut ttml_writer, &caption, &options).expect("TTML caption");
    write_ttml_footer(&mut ttml_writer).expect("TTML footer");
    ttml_writer.flush().expect("TTML flush");
    let srt = write_srt_from_ass(&ass, true)
        .expect("SRT")
        .expect("SRT path");
    let vtt = write_webvtt_from_ass(&ass, true)
        .expect("WebVTT")
        .expect("WebVTT path");

    for output in [&ass, &ttml, &srt, &vtt] {
        let text = fs::read_to_string(output).expect("mapped output");
        assert!(text.contains('映'), "{}: {text}", output.display());
        assert!(text.contains('終'), "{}: {text}", output.display());
        assert!(!text.contains('\u{e000}'), "{}: {text}", output.display());
    }
    assert!(!fs::read_to_string(&ttml).unwrap().contains("font-face"));
    fs::remove_dir_all(directory).expect("cleanup mapped outputs");
}

#[test]
fn scoped_b62_mapping_reaches_standalone_ruby_ass_text() {
    let mut captions = parse_ttml_captions(
        r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body><div>
          <p begin='0s' end='1s'>漢</p>
          <p begin='0s' end='1s'><span arib-tt:font-face='subt://9'>&#xE000;</span></p>
        </div></body></tt>"#,
        0,
    );
    let mapping_key = attach_test_b62_resource(&mut captions[1], b"ruby-font-resource");
    captions[1].ruby_bindings.push(TtmlRubyBinding {
        ruby_text: "\u{e000}".into(),
        base_caption_index: 0,
        base_run_start: 0,
        base_run_end: 1,
        base_start: 0,
        base_end: 1,
        base_text: "漢".into(),
        base_cell_boxes: vec![RubyLayoutBox {
            x: 960,
            y: 920,
            width: 42,
            height: 42,
        }],
        base_box: Some(RubyLayoutBox {
            x: 960,
            y: 920,
            width: 42,
            height: 42,
        }),
        source_ruby_box: Some(RubyLayoutBox {
            x: 960,
            y: 890,
            width: 42,
            height: 18,
        }),
        placement: RubyPlacement::Above,
        writing_mode: RubyWritingMode::HorizontalTb,
        resolver: RubyBindingResolver::SourceGeometry,
        ruby_style: TtmlCaptionStyle::default(),
    });
    let mut options = ConversionOptions {
        drcs_mode: DrcsMode::UseUserMapping,
        ..Default::default()
    };
    options
        .ttml_drcs_replacements
        .insert(mapping_key, "映".into());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-b62-ruby-map-{stamp}.ass"));
    let mut writer = BufWriter::new(File::create(&output).expect("ASS output"));
    write_ass_header(&mut writer).expect("ASS header");
    write_ass_ttml_group(&mut writer, &captions, &options).expect("ASS captions");
    writer.flush().expect("ASS flush");

    let ass = fs::read_to_string(&output).expect("mapped ASS");
    assert!(ass.contains('映'), "{ass}");
    assert!(!ass.contains('\u{e000}'), "{ass}");
    fs::remove_file(output).expect("cleanup mapped Ruby ASS");
}

#[test]
fn b62_drcs_drop_reaches_ass_and_ttml_outputs() {
    let caption = parse_ttml_captions(
        r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body><p begin='0s' end='1s'><span arib-tt:font-face='subt://9'>&#xE000;</span></p></body></tt>"#,
        0,
    )
    .remove(0);
    let options = ConversionOptions {
        drcs_mode: DrcsMode::UseUserMapping,
        preserve_drcs: false,
        overwrite: true,
        ..Default::default()
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("arib-b62-drcs-output-{stamp}"));
    fs::create_dir_all(&directory).expect("temporary directory");
    let ass = directory.join("dropped.ass");
    let ttml = directory.join("dropped.ttml");

    let mut ass_writer = BufWriter::new(File::create(&ass).expect("ASS output"));
    write_ass_header(&mut ass_writer).expect("ASS header");
    write_ass_ttml_group(&mut ass_writer, std::slice::from_ref(&caption), &options)
        .expect("ASS caption");
    ass_writer.flush().expect("ASS flush");
    let mut ttml_writer = BufWriter::new(File::create(&ttml).expect("TTML output"));
    write_ttml_header(&mut ttml_writer).expect("TTML header");
    write_ttml_caption(&mut ttml_writer, &caption, &options).expect("TTML caption");
    write_ttml_footer(&mut ttml_writer).expect("TTML footer");
    ttml_writer.flush().expect("TTML flush");

    assert!(
        !fs::read_to_string(ass)
            .expect("dropped ASS text")
            .contains("Dialogue:")
    );
    assert!(
        !fs::read_to_string(ttml)
            .expect("dropped TTML text")
            .contains("<p ")
    );
    fs::remove_dir_all(directory).expect("cleanup dropped outputs");
}

#[test]
fn resolves_b62_drcs_font_style_at_the_character_run() {
    let xml = r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><head><styling>
      <style xml:id='drcs' arib-tt:font-face='subt://9'/>
    </styling></head><body><p begin='0s' end='1s'>
      <span>字</span><span xml:id='wrapper'><span style='drcs'>&#xE000;</span></span>
    </p></body></tt>"#;
    let caption = parse_ttml_captions(xml, 0).remove(0);

    assert_eq!(caption.drcs_uses.len(), 1);
    assert_eq!(caption.drcs_uses[0].source_codepoint, 0xe000);
    assert_eq!(caption.drcs_uses[0].resource_index, 9);
}

#[test]
fn escaped_numeric_text_is_not_double_decoded_as_b62_drcs() {
    let xml = r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body>
      <p begin='0s' end='1s' arib-tt:font-face='subt://9'>&amp;#xE000;</p>
    </body></tt>"#;
    let caption = parse_ttml_captions(xml, 0).remove(0);

    assert_eq!(caption.text, "&#xE000;");
    assert!(caption.drcs_uses.is_empty());
}

#[test]
fn ttml_output_removes_unpublished_b62_font_resource_reference() {
    let caption = parse_ttml_captions(
        r#"<tt xmlns:arib-tt='http://www.arib.or.jp/ns/arib-ttml/v1_0'><body>
          <p begin='0s' end='1s' arib-tt:font-face='subt://9'>字</p>
        </body></tt>"#,
        0,
    )
    .remove(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-b62-font-strip-{stamp}.ttml"));
    let mut writer = BufWriter::new(File::create(&output).expect("TTML output"));
    write_ttml_header(&mut writer).expect("TTML header");
    write_ttml_caption(&mut writer, &caption, &ConversionOptions::default()).expect("TTML caption");
    write_ttml_footer(&mut writer).expect("TTML footer");
    writer.flush().expect("TTML flush");

    let text = fs::read_to_string(&output).expect("TTML text");
    assert!(!text.contains("font-face"));
    assert!(roxmltree::Document::parse(&text).is_ok());
    fs::remove_file(output).expect("cleanup TTML output");
}

#[test]
fn dropping_accessibility_or_gaiji_keeps_ttml_ruby_and_inline_colour() {
    let xml = "<tt><body><p begin='0s' end='1s'>♪<ruby><span tts:ruby='base' tts:color='#ff0000'>漢</span><rt><span tts:ruby='text'>かん</span></rt></ruby><span>終&amp;</span></p></body></tt>";
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    for (preserve_gaiji, preserve_accessibility) in [(false, true), (true, false), (false, false)] {
        let options = ConversionOptions {
            preserve_gaiji,
            preserve_accessibility,
            ..Default::default()
        };
        let output = std::env::temp_dir().join(format!(
            "arib-independent-ruby-{}-{preserve_gaiji}-{preserve_accessibility}.ttml",
            std::process::id()
        ));
        let mut writer = BufWriter::new(File::create(&output).expect("output"));
        write_ttml_header(&mut writer).unwrap();
        write_ttml_caption(&mut writer, &caption, &options).unwrap();
        write_ttml_footer(&mut writer).unwrap();
        writer.flush().unwrap();
        let text = fs::read_to_string(&output).unwrap();
        assert!(
            text.contains("tts:ruby='text'"),
            "body={:?} output={text}",
            caption.rich_body
        );
        assert!(text.contains("tts:color='#ff0000'"));
        assert!(text.contains("かん"));
        assert!(text.contains("終&amp;"));
        assert_eq!(text.contains('♪'), preserve_accessibility);
        fs::remove_file(&output).unwrap();
    }
}

#[test]
fn drops_nested_colour_when_ttml_colour_preservation_is_disabled() {
    let xml = "<tt><body><p begin='0s' end='1s'><ruby><span tts:ruby='base' tts:color='#ffffff'>漢</span><rt><span tts:ruby='text' tts:color='#ff00ff'>かん</span></rt></ruby></p></body></tt>";
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    let output =
        std::env::temp_dir().join(format!("arib-ttml-no-colour-{}.ttml", std::process::id()));
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ttml_header(&mut writer).expect("header");
    write_ttml_caption(
        &mut writer,
        &caption,
        &ConversionOptions {
            preserve_color: false,
            ..ConversionOptions::default()
        },
    )
    .expect("caption");
    write_ttml_footer(&mut writer).expect("footer");
    writer.flush().expect("flush");
    let text = fs::read_to_string(&output).expect("read");
    assert!(!text.contains("tts:color"));
    assert!(text.contains("tts:ruby='text'"));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn ass_export_preserves_inline_colour_and_places_ruby_on_a_separate_layer() {
    let xml = "<tt><body><p begin='0s' end='1s' tts:fontSize='72px' tts:fontFamily='丸ゴシック' tts:color='#FFFFFF' tts:textOutline='2px #000000'><span tts:color='#00FFFF'>字幕</span><ruby><span tts:ruby='base' tts:color='#FFFF00'>漢</span><rt><span tts:ruby='text' tts:color='#FF00FF' tts:fontSize='30px'>かん</span></rt></ruby><span tts:color='#00FF00'>終</span></p></body></tt>";
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-rich-ass-{stamp}.ass"));
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_header(&mut writer).expect("write ASS header");
    write_ass_ttml_group(
        &mut writer,
        std::slice::from_ref(&caption),
        &ConversionOptions::default(),
    )
    .expect("write ASS");
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    assert!(ass.contains("Title: ResubWinny"));
    assert!(ass.contains("Style: Default,Rounded M+ 1m for ARIB,42,"));
    assert!(ass.contains(
        "\\c&H00FFFF00&\\fs72\\fnRounded M+ 1m for ARIB\\b0\\i0\\fsp0\\bord2.00\\3c&H00000000&}字幕"
    ));
    assert!(ass.contains(
        "\\c&H0000FFFF&\\fs72\\fnRounded M+ 1m for ARIB\\b0\\i0\\fsp0\\bord2.00\\3c&H00000000&}漢"
    ));
    assert!(ass.contains("Dialogue: 1,"));
    assert!(ass.contains("\\c&H00FF00FF&"));
    assert!(ass.contains("\\fs30"));
    assert!(ass.contains("\\bord2.00\\3c&H00000000&"));
    let ruby_lines = ass
        .lines()
        .filter(|line| {
            line.starts_with("Dialogue: 1,") && (line.ends_with('か') || line.ends_with('ん'))
        })
        .collect::<Vec<_>>();
    assert_eq!(ruby_lines.len(), 2);
    assert!(ruby_lines.iter().all(|line| line.contains("\\fs30")));
    assert!(ruby_lines.iter().all(|line| line.contains(",920)")));
    assert!(ass.contains("\\pos(960,962)"));
    assert!(ass.contains(
        "\\c&H0000FF00&\\fs72\\fnRounded M+ 1m for ARIB\\b0\\i0\\fsp0\\bord2.00\\3c&H00000000&}終"
    ));
    if ffmpeg_available() {
        let directory = std::env::temp_dir().join(format!("resubwinny-inline-ruby-{stamp}"));
        let image = render_ass_fixture(&directory, &ass, "inline.png", "0.5");
        let ruby_center = magenta_pixel_center(&image, 1000..1220, 880..950);
        let base_center = yellow_pixel_center(&image, 1000..1220, 940..1030);
        assert!(
            (ruby_center - base_center).abs() <= 3.0,
            "inline ruby centre {ruby_center} differs from base glyph centre {base_center}"
        );
        finish_visual_fixture(&directory);
    }
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn ass_export_centres_a_bounded_standalone_kana_annotation() {
    let xml = "<tt><head><layout><region xml:id='ruby' tts:origin='1396px 838px' tts:extent='80px 60px'/><region xml:id='base' tts:origin='1236px 898px' tts:extent='440px 120px'/></layout></head><body><div begin='1s' end='2s'><p region='ruby'><span tts:fontSize='36px 36px' tts:letterSpacing='4px'>ささ</span></p><p region='base'><span tts:fontSize='72px 72px' tts:letterSpacing='8px'>祈り捧げる</span><span tts:fontSize='36px 72px' tts:letterSpacing='4px'>」</span></p></div></body></tt>";
    let mut captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 2);
    associate_standalone_ttml_ruby(&mut captions);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-standalone-ruby-{stamp}.ass"));
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_ttml_group(&mut writer, &captions, &ConversionOptions::default())
        .expect("write grouped ASS");
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    let dialogues = ass
        .lines()
        .filter(|line| line.starts_with("Dialogue:"))
        .collect::<Vec<_>>();
    assert_eq!(dialogues.len(), 3, "only Ruby glyphs may be split");
    assert!(ass.contains("Dialogue: 1,"));
    let ruby_lines = dialogues
        .iter()
        .filter(|line| line.starts_with("Dialogue: 1,"))
        .collect::<Vec<_>>();
    assert_eq!(ruby_lines.len(), 2);
    assert!(ruby_lines.iter().any(|line| line.ends_with('さ')));
    assert!(ass.contains("Dialogue: 0,"));
    assert!(ass.contains("{\\an7\\pos(1236,898)}"));
    let base = dialogues
        .iter()
        .find(|line| line.starts_with("Dialogue: 0,"))
        .expect("base dialogue");
    assert!(base.contains("祈り捧げる"));
    assert!(ass.contains("\\fs72"));
    assert!(!ass.contains("\\fscx"));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn ass_export_centres_multi_character_ruby_below_the_lower_line() {
    let xml = "<tt><head><layout><region xml:id='base' tts:origin='640px 660px' tts:extent='800px 240px'/><region xml:id='ruby' tts:origin='800px 900px' tts:extent='160px 60px'/></layout></head><body><div begin='1s' end='2s'><p region='base' tts:lineHeight='120px'><span tts:fontSize='72px 72px' tts:letterSpacing='8px'>ニュースです</span><br/><span tts:fontSize='72px 72px' tts:letterSpacing='8px'>字幕放送です</span></p><p region='ruby'><span tts:fontSize='36px 36px' tts:letterSpacing='4px'>ほうそう</span></p></div></body></tt>";
    let mut captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 2);
    associate_standalone_ttml_ruby(&mut captions);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-ruby-below-{stamp}.ass"));
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_ttml_group(&mut writer, &captions, &ConversionOptions::default())
        .expect("write grouped ASS");
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    assert!(ass.contains("Dialogue: 1,"));
    let ruby_lines = ass
        .lines()
        .filter(|line| line.starts_with("Dialogue: 1,"))
        .collect::<Vec<_>>();
    assert_eq!(ruby_lines.len(), 4);
    assert_eq!(
        ruby_lines
            .iter()
            .filter(|line| ['ほ', 'う', 'そ']
                .iter()
                .any(|character| line.ends_with(*character)))
            .count(),
        4
    );
    assert_eq!(
        ass.lines()
            .filter(|line| line.starts_with("Dialogue: 0,"))
            .count(),
        1,
        "both base rows must remain one shaped event"
    );
    let base = ass
        .lines()
        .find(|line| line.starts_with("Dialogue: 0,"))
        .expect("multiline base dialogue");
    assert!(base.contains("ニュースです"));
    assert!(base.contains("\\N"));
    assert!(base.contains("字幕放送です"));
    assert!(!ass.contains("\\fscx"));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn libass_render_centres_multi_character_ruby_below_the_unchanged_lower_line() {
    if !ffmpeg_available() {
        eprintln!("skipping libass visual test because ffmpeg is unavailable");
        return;
    }
    let xml = "<tt><head><layout><region xml:id='base' tts:origin='640px 660px' tts:extent='800px 240px'/><region xml:id='ruby' tts:origin='800px 900px' tts:extent='160px 60px'/></layout></head><body><div begin='1s' end='2s'><p region='base' tts:lineHeight='120px'><span tts:color='#FFFFFFFF' tts:fontFamily='丸ゴシック' tts:fontSize='72px 72px' tts:letterSpacing='8px'>ニュースです</span><br/><span tts:color='#FFFFFFFF' tts:fontFamily='丸ゴシック' tts:fontSize='72px 72px' tts:letterSpacing='8px'>字幕</span><span tts:color='#FFFF00FF' tts:fontFamily='丸ゴシック' tts:fontSize='72px 72px' tts:letterSpacing='8px'>放送</span><span tts:color='#FFFFFFFF' tts:fontFamily='丸ゴシック' tts:fontSize='72px 72px' tts:letterSpacing='8px'>です</span></p><p region='ruby'><span tts:color='#FFFF00FF' tts:fontFamily='丸ゴシック' tts:fontSize='36px 36px' tts:letterSpacing='4px'>ほうそう</span></p></div></body></tt>";
    let mut captions = parse_ttml_captions(xml, 0);
    associate_standalone_ttml_ruby(&mut captions);
    assert_eq!(captions[1].ruby_bindings.len(), 1);
    assert_eq!(captions[1].ruby_bindings[0].base_text, "放送");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("resubwinny-libass-lower-ruby-{stamp}"));
    let ass_path = directory.join("source.ass");
    fs::create_dir_all(&directory).expect("fixture directory");
    let mut writer = BufWriter::new(File::create(&ass_path).expect("ASS output"));
    write_ass_header(&mut writer).expect("ASS header");
    write_ass_ttml_group(&mut writer, &captions, &ConversionOptions::default())
        .expect("ASS captions");
    writer.flush().expect("flush ASS");
    let ass = fs::read_to_string(ass_path).expect("read ASS");
    assert_eq!(
        ass.lines()
            .filter(|line| line.starts_with("Dialogue: 0,"))
            .count(),
        1,
        "both base rows must remain in one Dialogue event"
    );
    let image = render_ass_fixture(&directory.join("full"), &ass, "full.png", "1.0");
    let ruby_center = yellow_pixel_center(&image, 700..1000, 840..920);
    let base_center = yellow_pixel_center(&image, 700..1000, 700..840);
    assert!(
        (ruby_center - base_center).abs() <= 3.0,
        "lower-line ruby centre {ruby_center} differs from multi-kanji centre {base_center}"
    );

    let base_only = ass
        .lines()
        .filter(|line| !line.starts_with("Dialogue: 1,"))
        .collect::<Vec<_>>()
        .join("\n");
    let reference = render_ass_fixture(&directory.join("base"), &base_only, "base.png", "1.0");
    for y in 650..840 {
        for x in 600..1250 {
            assert_eq!(
                image.get_pixel(x, y),
                reference.get_pixel(x, y),
                "adding Ruby must not alter the base-line raster at ({x}, {y})"
            );
        }
    }
    finish_visual_fixture(&directory);
}

#[test]
fn libass_render_centres_standalone_ruby_over_the_base_glyph() {
    if std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_err()
    {
        eprintln!("skipping libass visual test because ffmpeg is unavailable");
        return;
    }
    let xml = "<tt><head><layout><region xml:id='ruby' tts:origin='1396px 838px' tts:extent='80px 60px'/><region xml:id='base' tts:origin='1236px 898px' tts:extent='440px 120px'/></layout></head><body><div begin='1s' end='2s'><p region='ruby'><span tts:color='#FFFF00FF' tts:fontSize='36px 36px' tts:fontFamily='丸ゴシック' tts:letterSpacing='4px'>ささ</span></p><p region='base'><span tts:color='#FFFFFFFF' tts:fontSize='72px 72px' tts:fontFamily='丸ゴシック' tts:letterSpacing='8px'>祈り</span><span tts:color='#FFFF00FF' tts:fontSize='72px 72px' tts:fontFamily='丸ゴシック' tts:letterSpacing='8px'>捧</span><span tts:color='#FFFFFFFF' tts:fontSize='72px 72px' tts:fontFamily='丸ゴシック' tts:letterSpacing='8px'>げる</span></p></div></body></tt>";
    let mut captions = parse_ttml_captions(xml, 0);
    associate_standalone_ttml_ruby(&mut captions);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("resubwinny-libass-ruby-{stamp}"));
    fs::create_dir_all(&directory).expect("visual test directory");
    let ass_path = directory.join("ruby.ass");
    let mut writer = BufWriter::new(File::create(&ass_path).expect("ASS output"));
    write_ass_header(&mut writer).expect("ASS header");
    write_ass_ttml_group(&mut writer, &captions, &ConversionOptions::default())
        .expect("ASS captions");
    writer.flush().expect("flush ASS");
    let ass = fs::read_to_string(&ass_path).expect("read rendered ASS");
    assert_eq!(
        ass.lines()
            .filter(|line| line.starts_with("Dialogue: 0,"))
            .count(),
        1,
        "visual fixture must not split the base line"
    );
    let base = ass
        .lines()
        .find(|line| line.starts_with("Dialogue: 0,"))
        .expect("base dialogue");
    assert!(base.contains("祈り"));
    assert!(base.contains("捧"));
    assert!(base.contains("げる"));
    let font = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/rounded-mplus-1m-arib/rounded-mplus-1m-arib.ttf");
    fs::copy(font, directory.join("rounded-mplus-1m-arib.ttf")).expect("copy test font");
    let status = std::process::Command::new("ffmpeg")
        .current_dir(&directory)
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x202124:s=1920x1080:r=1:d=2",
            "-vf",
            "ass=filename='ruby.ass':fontsdir='.',select='gte(t,1)'",
            "-frames:v",
            "1",
            "-update",
            "1",
            "ruby.png",
        ])
        .status()
        .expect("run ffmpeg");
    assert!(status.success(), "ffmpeg/libass visual render failed");
    let image = image::open(directory.join("ruby.png"))
        .expect("rendered PNG")
        .to_rgb8();
    let yellow_center = |y_start: u32, y_end: u32| {
        let mut minimum = u32::MAX;
        let mut maximum = 0_u32;
        for y in y_start..y_end {
            for x in 1400..1490 {
                let pixel = image.get_pixel(x, y).0;
                if pixel[0] > 160 && pixel[1] > 160 && pixel[2] < 100 {
                    minimum = minimum.min(x);
                    maximum = maximum.max(x);
                }
            }
        }
        assert!(minimum <= maximum, "expected yellow caption pixels");
        (minimum + maximum) as f32 * 0.5
    };
    let ruby_center = yellow_center(820, 890);
    let base_center = yellow_center(895, 980);
    assert!(
        (ruby_center - base_center).abs() <= 3.0,
        "ruby centre {ruby_center} differs from base glyph centre {base_center}"
    );
    finish_visual_fixture(&directory);
}

#[test]
fn resolves_ttml_style_references_from_div_region_and_caption() {
    let xml = r##"<tt><head><styling><style xml:id="base" tts:color="#112233" tts:backgroundColor="#00000080" tts:fontSize="32px"/><style xml:id="vertical" tts:writingMode="tbrl"/></styling><layout><region xml:id="bottom" style="base" tts:origin="100px 200px"/></layout></head><body><div style="vertical"><p begin="0s" end="1s" region="bottom" style="base" tts:fontSize="48px">styled</p></div></body></tt>"##;
    let captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].style.color.as_deref(), Some("#112233"));
    assert_eq!(
        captions[0].style.background_color.as_deref(),
        Some("#00000080")
    );
    assert_eq!(captions[0].style.font_size.as_deref(), Some("48px"));
    assert_eq!(
        captions[0].style.writing_mode.as_deref(),
        Some("vertical-rl")
    );
    assert_eq!((captions[0].x, captions[0].y), (100, 200));
}

#[test]
fn normalises_declared_2k_4k_and_8k_ttml_planes_to_identical_viewer_geometry() {
    fn document(width: i32, height: i32, factor: i32) -> String {
        format!(
            "<tt tts:extent='{width}px {height}px'><head><layout><region xml:id='r' tts:origin='{}px {}px' tts:extent='{}px {}px'/></layout></head><body><p begin='0s' end='1s' region='r' tts:fontSize='{}px {}px' tts:lineHeight='{}px' tts:letterSpacing='{}px' tts:textOutline='{}px #000000'><span tts:fontSize='{}px'>字幕</span></p></body></tt>",
            240 * factor,
            810 * factor,
            1440 * factor,
            180 * factor,
            72 * factor,
            72 * factor,
            96 * factor,
            4 * factor,
            2 * factor,
            72 * factor,
        )
    }

    let captions = [(1920, 1080, 1), (3840, 2160, 2), (7680, 4320, 4)]
        .into_iter()
        .map(|(width, height, factor)| {
            parse_ttml_captions(&document(width, height, factor), 0)
                .pop()
                .expect("caption")
        })
        .collect::<Vec<_>>();
    for caption in &captions {
        assert_eq!(
            (caption.x, caption.y, caption.width, caption.height),
            (240, 810, Some(1440), Some(180))
        );
        assert_eq!(caption.style.font_size.as_deref(), Some("72px"));
        assert_eq!(caption.style.line_height.as_deref(), Some("96px"));
        assert_eq!(caption.style.letter_spacing.as_deref(), Some("4px"));
        assert_eq!(caption.style.text_outline.as_deref(), Some("2px #000000"));
        assert!(
            caption
                .rich_body
                .as_deref()
                .is_some_and(|body| body.contains("tts:fontSize='72px'"))
        );
    }
}

#[test]
fn ttml_without_a_root_extent_keeps_the_logical_2k_coordinate_space() {
    let xml = "<tt><head><layout><region xml:id='r' tts:origin='240px 810px' tts:extent='1440px 180px'/></layout></head><body><p begin='0s' end='1s' region='r' tts:fontSize='72px'>字幕</p></body></tt>";
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    assert_eq!(
        (caption.x, caption.y, caption.width, caption.height),
        (240, 810, Some(1440), Some(180))
    );
    assert_eq!(caption.style.font_size.as_deref(), Some("72px"));
}

#[test]
fn infers_a_canonical_4k_plane_from_complete_region_geometry_without_root_extent() {
    let xml = "<tt><head><layout><region xml:id='r' tts:origin='552px 1676px' tts:extent='2640px 240px'/></layout></head><body><p begin='0s' end='1s' region='r' tts:fontSize='144px'>字幕</p></body></tt>";
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    assert_eq!(
        (caption.x, caption.y, caption.width, caption.height),
        (276, 838, Some(1320), Some(120))
    );
    assert_eq!(caption.style.font_size.as_deref(), Some("72px"));
}

#[test]
fn infers_a_canonical_plane_when_one_extentless_geometry_axis_exceeds_logical_2k() {
    let xml = "<tt><head><layout><region xml:id='r' tts:origin='552px 1676px' tts:extent='320px 240px'/></layout></head><body><p begin='0s' end='1s' region='r' tts:fontSize='144px'>♬〜</p></body></tt>";
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    assert_eq!(
        (caption.x, caption.y, caption.width, caption.height),
        (276, 838, Some(160), Some(120))
    );
    assert_eq!(caption.style.font_size.as_deref(), Some("72px"));
}

#[test]
fn b62_region_capacity_does_not_promote_a_4k_document_to_an_8k_plane() {
    let xml = r##"<tt><head><styling><style xml:id="caption" tts:fontSize="144px 144px" tts:lineHeight="240px" arib-tt:letter-spacing="16px"/></styling><layout><region xml:id="display1" tts:extent="2480px 1920px" tts:origin="680px 1080px"/><region xml:id="display2" tts:extent="2480px 1920px" tts:origin="680px 1320px"/></layout></head><body><div><p begin="0s" end="1s" region="display1"><span style="caption">字幕</span></p></div></body></tt>"##;
    let caption = parse_ttml_captions(xml, 0).pop().expect("caption");
    assert_eq!(
        (caption.x, caption.y, caption.width, caption.height),
        (340, 540, Some(1240), Some(960))
    );
    assert_eq!(caption.style.font_size.as_deref(), Some("72px 72px"));
    assert_eq!(caption.style.line_height.as_deref(), Some("120px"));
    assert_eq!(caption.style.letter_spacing.as_deref(), Some("8px"));
    let source = caption.source_layout.as_ref().expect("source layout");
    assert_eq!((source.plane_width, source.plane_height), (3840, 2160));
    assert_eq!(source.plane_basis, TtmlSourcePlaneBasis::Inferred);
    assert_eq!(
        (source.x, source.y, source.width, source.height),
        (680, 1080, Some(2480), Some(1920))
    );
    assert_eq!(source.style.font_size.as_deref(), Some("144px 144px"));
    assert_eq!(source.style.line_height.as_deref(), Some("240px"));
    assert_eq!(source.style.letter_spacing.as_deref(), Some("16px"));
}

#[test]
fn leaves_2k_or_out_of_canonical_extentless_ttml_geometry_unmodified() {
    for (origin, extent) in [
        ("1800px 810px", "100px 180px"),
        ("4000px 100px", "5000px 100px"),
    ] {
        let xml = format!(
            "<tt><head><layout><region xml:id='r' tts:origin='{origin}' tts:extent='{extent}'/></layout></head><body><p begin='0s' end='1s' region='r'>字幕</p></body></tt>"
        );
        let caption = parse_ttml_captions(&xml, 0).pop().expect("caption");
        let values = origin
            .split_whitespace()
            .map(|value| value.trim_end_matches("px").parse::<i32>().expect("pixels"))
            .collect::<Vec<_>>();
        assert_eq!((caption.x, caption.y), (values[0], values[1]));
    }
}

#[test]
fn resolves_nested_ttml_begin_end_and_duration_against_the_parent_time_container() {
    let xml = "<tt><body><div begin='2s' dur='3s'><p begin='1s' dur='1s'>child</p><p>parent duration</p></div></body></tt>";
    let captions = parse_ttml_captions(xml, 500);
    assert_eq!(captions.len(), 2);
    assert_eq!((captions[0].start_ms, captions[0].end_ms), (3_500, 4_500));
    assert_eq!((captions[1].start_ms, captions[1].end_ms), (2_500, 5_500));
}

#[test]
fn resolves_all_open_ttml_div_ancestors_without_leaking_closed_siblings() {
    let xml = r##"<tt><head><styling><style xml:id="blue" tts:color="#123456"/><style xml:id="vertical" tts:writingMode="tbrl"/></styling><layout><region xml:id="inherited" tts:origin="20% 50%"/></layout></head><body><div begin="10s" end="20s" style="blue"><div begin="2s" dur="3s" style="vertical" region="inherited"><p begin="1s" dur="1s">nested</p></div><div begin="4s" dur="1s"><p>second</p></div></div></body></tt>"##;
    let captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 2);
    assert_eq!((captions[0].start_ms, captions[0].end_ms), (13_000, 14_000));
    assert_eq!(captions[0].style.color.as_deref(), Some("#123456"));
    assert_eq!(
        captions[0].style.writing_mode.as_deref(),
        Some("vertical-rl")
    );
    assert_eq!((captions[0].x, captions[0].y), (384, 540));
    assert_eq!((captions[1].start_ms, captions[1].end_ms), (14_000, 15_000));
    assert_eq!(captions[1].style.color.as_deref(), Some("#123456"));
    assert_eq!(captions[1].style.writing_mode, None);
    assert_eq!((captions[1].x, captions[1].y), (960, 920));
}

#[test]
fn preserves_ttml_percentage_region_origin_and_extent() {
    let xml = "<tt><head><layout><region xml:id='r' tts:origin='10% 75%' tts:extent='80% 10%'/></layout></head><body><p begin='0s' end='1s' region='r'>positioned</p></body></tt>";
    let captions = parse_ttml_captions(xml, 0);
    assert_eq!(captions.len(), 1);
    assert_eq!((captions[0].x, captions[0].y), (192, 810));
    assert_eq!(
        (captions[0].width, captions[0].height),
        (Some(1536), Some(108))
    );
}

#[test]
fn accepts_complete_ttml_documents_with_or_without_an_xml_declaration() {
    let documents = ttml_documents(
        b"prefix <tt xml:lang='ja'><body/></tt> gap <?xml version='1.0'?><tt><body/></tt>",
    );
    assert_eq!(documents.len(), 2);
    assert!(documents[0].xml.starts_with("<tt"));
    assert!(documents[1].xml.starts_with("<?xml"));
    assert_eq!(
        ttml_documents("<tt xml:lang='ja'><body><p>字幕</p></body></tt>".as_bytes()).len(),
        1
    );
    assert!(ttml_documents(b"<ttml>not a TTML root</ttml>").is_empty());
}

#[test]
fn finds_utf8_ttml_inside_a_private_pes_with_non_utf8_framing() {
    let pes = [
            &[0x00, 0x00, 0x01, 0xbd, 0x80, 0x80, 0xff, 0x00][..],
            b"\x01\x00\x01\x7f<?xml version='1.0' encoding='UTF-8'?><tt><body><p begin='0s' end='1s'>BS4K</p></body></tt>",
        ]
        .concat();
    let documents = ttml_documents(&pes);
    assert_eq!(documents.len(), 1);
    let captions = parse_ttml_captions(&documents[0].xml, 0);
    assert_eq!(captions.len(), 1);
    assert_eq!(captions[0].text, "BS4K");
}

#[test]
fn decodes_utf16le_ttml_inside_a_private_pes() {
    let xml = "<?xml version='1.0' encoding='UTF-16'?><tt><body><p begin='0s' end='1s'>字幕</p></body></tt>";
    let mut pes = vec![0xff, 0x01, 0xbd, 0x7f, 0xfe, 0xff];
    for unit in xml.encode_utf16() {
        pes.extend_from_slice(&unit.to_le_bytes());
    }
    let documents = ttml_documents(&pes);
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].encoding, XmlTextEncoding::Utf16Le);
    assert_eq!(parse_ttml_captions(&documents[0].xml, 0)[0].text, "字幕");
}

#[test]
fn decodes_declared_shift_jis_ttml_without_lossy_replacement() {
    let xml = "<?xml version='1.0' encoding='Shift_JIS'?><tt><body><p begin='0s' end='1s'>ニュース字幕</p></body></tt>";
    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(xml);
    assert!(!had_errors);
    let documents = ttml_documents(&encoded);
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].encoding, XmlTextEncoding::ShiftJis);
    assert_eq!(
        parse_ttml_captions(&documents[0].xml, 0)[0].text,
        "ニュース字幕"
    );
}

#[test]
fn rejects_unsupported_or_invalid_xml_encodings_without_replacement() {
    assert!(
        ttml_documents(b"<?xml version='1.0' encoding='windows-1252'?><tt><body/></tt>").is_empty()
    );
    assert!(
        ttml_documents(b"<?xml version='1.0' encoding='UTF-8'?><tt><body>\xff</body></tt>")
            .is_empty()
    );
}
