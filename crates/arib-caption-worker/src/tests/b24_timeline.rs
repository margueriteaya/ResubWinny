use super::*;

#[test]
fn finds_b24_component_descriptor() {
    for component_tag in 0x30..=0x37 {
        assert!(b24_descriptor(&[
            0x52,
            0x01,
            component_tag,
            0xfd,
            0x02,
            0x00,
            0x08,
        ]));
        assert!(!b24_descriptor(&[0x52, 0x01, component_tag]));
    }
    for component_tag in 0x38..=0x3f {
        assert!(!b24_descriptor(&[0x52, 0x01, component_tag]));
        assert!(!b24_descriptor(&[
            0xfd,
            0x02,
            0x00,
            0x08,
            0x52,
            0x01,
            component_tag,
        ]));
    }
    assert!(b24_descriptor(&[0xfd, 0x02, 0x00, 0x08, 0x52, 0x01, 0x30]));
    assert!(!b24_descriptor(&[0xfd, 0x02, 0x00, 0x08]));
}

#[test]
fn selects_the_requested_b24_track_instead_of_only_the_first_track() {
    let tracks = vec![
        B24Track {
            service_id: 101,
            pmt_pid: 0x0100,
            caption_pid: 0x0120,
            component_tag: 0x30,
            caption_pids: vec![0x0120],
            language: Some("jpn".into()),
            service_name: None,
        },
        B24Track {
            service_id: 101,
            pmt_pid: 0x0100,
            caption_pid: 0x0121,
            component_tag: 0x31,
            caption_pids: vec![0x0121],
            language: Some("jpn".into()),
            service_name: None,
        },
    ];
    assert_eq!(
        select_b24_track(tracks.clone(), Some(0x0121))
            .expect("requested track")
            .caption_pid,
        0x0121
    );
    assert_eq!(
        select_b24_track(tracks, None)
            .expect("default track")
            .caption_pid,
        0x0120
    );
}

#[test]
fn native_b24_decoder_initializes() {
    let mut decoder = native_b24::NativeB24Decoder::new().expect("native decoder");
    assert_eq!(decoder.feed(&[0x80, 0xff, 0x00], 0).status, 0);
}

#[test]
fn parses_b24_payload_and_pts() {
    let pes = [
        0, 0, 1, 0xbd, 0, 0, 0x80, 0x80, 5, 0x21, 0, 5, 0xbf, 0x21, 0x80,
    ];
    let (payload, pts) = b24_payload_from_pes(&pes).expect("B24 PES");
    assert_eq!(pts.map(Pts90k::to_millis), Some(1000));
    assert_eq!(payload, [0x80]);
}

#[test]
fn indefinite_duration_ends_at_next_caption() {
    assert_eq!(caption_end(1_000, i64::MAX, 2_500), 2_500);
    assert_eq!(caption_end(1_000, 750, 2_500), 1_750);
}

#[test]
fn normalises_pts_to_recording_start_and_handles_wrap() {
    assert_eq!(normalise_pts(31_853_500, 31_852_803), 697);
    assert_eq!(normalise_pts(120, PTS_WRAP_MS - 380), 500);
}

fn scene_with_text_regions(pts_ms: i64, regions: &[(i32, i32, &str)]) -> native_b24::CaptionScene {
    let mut characters = Vec::new();
    let regions = regions
        .iter()
        .map(|(x, y, text)| {
            let first_character = characters.len() as u32;
            for (index, character) in text.chars().enumerate() {
                characters.push(native_b24::CaptionCharacter {
                    kind: 0,
                    codepoint: character as u32,
                    pua_codepoint: 0,
                    drcs_code: 0,
                    x: *x + index as i32 * 20,
                    y: *y,
                    width: 20,
                    height: 24,
                    horizontal_spacing: 0,
                    vertical_spacing: 0,
                    horizontal_scale: 1.0,
                    vertical_scale: 1.0,
                    text_color: 0xffffff,
                    back_color: 0,
                    stroke_color: 0,
                    style: 0,
                    enclosure_style: 0,
                    utf8: character.to_string(),
                });
            }
            native_b24::CaptionRegion {
                x: *x,
                y: *y,
                width: (text.chars().count() as i32 * 20).max(20),
                height: 24,
                is_ruby: false,
                first_character,
                character_count: text.chars().count() as u32,
            }
        })
        .collect();
    native_b24::CaptionScene {
        pts_ms,
        wait_duration_ms: i64::MAX,
        plane_width: 960,
        plane_height: 540,
        regions,
        characters,
        drcs_glyphs: Vec::new(),
        rendered_image: None,
    }
}

#[test]
fn region_intervals_keep_independent_lifetimes() {
    let first = scene_with_text_regions(1_000, &[(100, 100, "label")]);
    let second = scene_with_text_regions(1_200, &[(100, 100, "label"), (500, 900, "body")]);
    let third = scene_with_text_regions(1_500, &[(500, 900, "body")]);
    let mut active = HashMap::new();

    assert!(apply_scene_intervals(&mut active, &first).is_empty());
    assert!(apply_scene_intervals(&mut active, &second).is_empty());
    let closed = apply_scene_intervals(&mut active, &third);
    assert_eq!(closed.len(), 1);
    assert_eq!((closed[0].begin_ms, closed[0].end_ms), (1_000, 1_500));
    assert_eq!(closed[0].characters[0].utf8, "l");

    let remaining = finish_scene_intervals(&mut active, 2_000);
    assert_eq!(remaining.len(), 1);
    assert_eq!((remaining[0].begin_ms, remaining[0].end_ms), (1_200, 2_000));
    assert_eq!(remaining[0].characters[0].utf8, "b");
}

#[test]
fn region_interval_uses_its_wait_duration_before_a_later_scene() {
    let mut first = scene_with_text_regions(1_000, &[(100, 100, "short")]);
    first.wait_duration_ms = 300;
    let mut clear = first.clone();
    clear.pts_ms = 1_500;
    clear.regions.clear();
    clear.characters.clear();
    let mut active = HashMap::new();
    assert!(apply_scene_intervals(&mut active, &first).is_empty());
    let closed = apply_scene_intervals(&mut active, &clear);
    assert_eq!(closed.len(), 1);
    assert_eq!((closed[0].begin_ms, closed[0].end_ms), (1_000, 1_300));
}

#[test]
fn writes_ttml_region_interval_with_its_own_timing() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-region-{stamp}.ttml"));
    let scene = scene_with_text_regions(1_250, &[(100, 200, "a&b")]);
    let mut interval = scene_intervals(&scene).pop().expect("interval");
    interval.end_ms = 2_500;
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ttml_header(&mut writer).expect("header");
    write_ttml_interval(&mut writer, &interval, &ConversionOptions::default()).expect("interval");
    write_ttml_footer(&mut writer).expect("footer");
    writer.flush().expect("flush");
    let text = fs::read_to_string(&output).expect("read");
    assert!(text.contains("begin=\"00:00:01.250\" end=\"00:00:02.500\""));
    assert!(text.contains("tts:origin=\"100px 200px\""));
    assert!(text.contains("a&amp;b"));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn ttml_feature_filter_uses_the_complete_b24_text_range() {
    let scene = scene_with_text_regions(1_250, &[(100, 200, "(寛太)説明⚟➡本文")]);
    let interval = scene_intervals(&scene).pop().expect("interval");
    let options = ConversionOptions {
        preserve_accessibility: false,
        preserve_gaiji: false,
        ..ConversionOptions::default()
    };
    assert_eq!(interval_ttml_text(&interval, &options), "説明本文");
}

#[test]
fn turns_packed_drcs_pixels_into_ass_drawing() {
    let glyph = native_b24::DrcsGlyph {
        drcs_code: 1,
        width: 2,
        height: 1,
        depth: 4,
        depth_bits: 2,
        alternative_codepoint: 0,
        md5: String::new(),
        alternative_text: String::new(),
        pixels: vec![0b1111_0000],
    };
    assert!(drcs_drawing(&glyph).contains("m 0 0 l 2 0"));
}

#[test]
fn writes_a_region_that_contains_only_unresolved_drcs() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-drcs-only-{stamp}.ass"));
    let scene = native_b24::CaptionScene {
        pts_ms: 0,
        wait_duration_ms: 1_000,
        plane_width: 960,
        plane_height: 540,
        regions: vec![native_b24::CaptionRegion {
            x: 100,
            y: 100,
            width: 20,
            height: 20,
            is_ruby: false,
            first_character: 0,
            character_count: 1,
        }],
        characters: vec![native_b24::CaptionCharacter {
            kind: 1,
            codepoint: 0,
            pua_codepoint: 0,
            drcs_code: 1,
            x: 100,
            y: 100,
            width: 20,
            height: 20,
            horizontal_spacing: 0,
            vertical_spacing: 0,
            horizontal_scale: 1.0,
            vertical_scale: 1.0,
            text_color: 0xffffff,
            back_color: 0,
            stroke_color: 0,
            style: 0,
            enclosure_style: 0,
            utf8: String::new(),
        }],
        drcs_glyphs: vec![native_b24::DrcsGlyph {
            drcs_code: 1,
            width: 2,
            height: 1,
            depth: 4,
            depth_bits: 2,
            alternative_codepoint: 0,
            md5: "test".into(),
            alternative_text: String::new(),
            pixels: vec![0b1111_0000],
        }],
        rendered_image: None,
    };
    let mut interval = scene_intervals(&scene).pop().expect("region interval");
    interval.end_ms = 1_000;
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_interval(&mut writer, &interval, &ConversionOptions::default()).expect("write scene");
    writer.flush().expect("flush");
    assert!(fs::read_to_string(&output).expect("read").contains("\\p1"));
    for count in [1, 2] {
        for preserve_drcs in [true, false] {
            let options = ConversionOptions {
                preserve_position: false,
                preserve_drcs,
                ..Default::default()
            };
            let mut writer = BufWriter::new(File::create(&output).expect("output"));
            write_ass_interval_group(&mut writer, &vec![interval.clone(); count], &options)
                .expect("unpositioned glyphs");
            writer.flush().expect("flush");
            let ass = fs::read_to_string(&output).expect("read");
            assert_eq!(
                ass.matches("\\p1").count(),
                if preserve_drcs { count } else { 0 }
            );
            assert!(!ass.contains("\\pos("));
            assert!(!ass.contains('\u{fffc}'));
        }
    }
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn exports_drcs_alternative_text_for_positioned_and_grouped_text_targets() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-drcs-alternative-{stamp}.ass"));
    let scene = native_b24::CaptionScene {
        pts_ms: 0,
        wait_duration_ms: 1_000,
        plane_width: 960,
        plane_height: 540,
        regions: vec![native_b24::CaptionRegion {
            x: 100,
            y: 100,
            width: 20,
            height: 20,
            is_ruby: false,
            first_character: 0,
            character_count: 1,
        }],
        characters: vec![native_b24::CaptionCharacter {
            kind: 1,
            codepoint: 0,
            pua_codepoint: 0,
            drcs_code: 1,
            x: 100,
            y: 100,
            width: 20,
            height: 20,
            horizontal_spacing: 0,
            vertical_spacing: 0,
            horizontal_scale: 1.0,
            vertical_scale: 1.0,
            text_color: 0xffffff,
            back_color: 0,
            stroke_color: 0,
            style: 0,
            enclosure_style: 0,
            utf8: String::new(),
        }],
        drcs_glyphs: vec![native_b24::DrcsGlyph {
            drcs_code: 1,
            width: 2,
            height: 1,
            depth: 4,
            depth_bits: 2,
            alternative_codepoint: 0,
            md5: "test".into(),
            alternative_text: "字".into(),
            pixels: vec![0b1111_0000],
        }],
        rendered_image: None,
    };
    let mut interval = scene_intervals(&scene).pop().expect("region interval");
    interval.end_ms = 1_000;
    for preserve_position in [true, false] {
        let options = ConversionOptions {
            preserve_position,
            ..ConversionOptions::default()
        };
        let mut writer = BufWriter::new(File::create(&output).expect("output"));
        write_ass_interval_group(&mut writer, &[interval.clone(), interval.clone()], &options)
            .expect("write scene");
        writer.flush().expect("flush");
        let ass = fs::read_to_string(&output).expect("read");
        assert!(ass.contains("字"));
        assert!(!ass.contains("\\p1"));
        let srt = write_srt_from_ass(&output, true)
            .expect("SRT")
            .expect("path");
        assert!(fs::read_to_string(&srt).expect("text").contains("字"));
        fs::remove_file(srt).expect("cleanup SRT");
    }
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn ass_export_groups_editable_ruby_text_and_keeps_inline_styles() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-b24-ruby-{stamp}.ass"));
    let interval = RegionInterval {
        begin_ms: 100,
        end_ms: 1_000,
        wait_duration_ms: 900,
        plane_width: 960,
        plane_height: 540,
        source_pid: Some(0x120),
        region: native_b24::CaptionRegion {
            x: 300,
            y: 400,
            width: 80,
            height: 24,
            is_ruby: true,
            first_character: 0,
            character_count: 2,
        },
        characters: vec![
            native_b24::CaptionCharacter {
                kind: 0,
                codepoint: 'か' as u32,
                pua_codepoint: 0,
                drcs_code: 0,
                x: 312,
                y: 401,
                width: 20,
                height: 20,
                horizontal_spacing: 2,
                vertical_spacing: 0,
                horizontal_scale: 0.5,
                vertical_scale: 1.0,
                text_color: 0xff00_00ff,
                back_color: 0,
                stroke_color: 0xff00_0000,
                style: 1 | (1 << 3),
                enclosure_style: 0,
                utf8: "か".into(),
            },
            native_b24::CaptionCharacter {
                kind: 0,
                codepoint: 'ん' as u32,
                pua_codepoint: 0,
                drcs_code: 0,
                x: 334,
                y: 401,
                width: 20,
                height: 20,
                horizontal_spacing: 2,
                vertical_spacing: 0,
                horizontal_scale: 1.0,
                vertical_scale: 1.0,
                text_color: 0xff00_ff00,
                back_color: 0,
                stroke_color: 0xff00_0000,
                style: 1 << 2,
                enclosure_style: 0,
                utf8: "ん".into(),
            },
        ],
        drcs_glyphs: Vec::new(),
        ruby_binding: None,
    };
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_interval(&mut writer, &interval, &ConversionOptions::default()).expect("write ASS");
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    assert!(ass.contains("Dialogue: 1,"));
    assert!(ass.contains("\\pos(624,802)"));
    assert!(!ass.contains("\\pos(668,802)"));
    assert_eq!(ass.matches("Dialogue: ").count(), 1);
    assert!(ass.contains("か"));
    assert!(ass.contains("ん"));
    assert!(ass.contains("\\fs20"));
    assert!(ass.contains("\\c&H000000FF&"));
    assert!(ass.contains("\\c&H0000FF00&"));
    assert!(!ass.contains("\\fscx"));
    assert!(!ass.contains("\\fscy"));
    assert!(ass.contains("\\bord4.00\\b1"));
    assert!(ass.contains("\\u1"));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn ass_export_splits_discontinuous_b24_positions() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-b24-gap-{stamp}.ass"));
    let character = |text: &str, x: i32| native_b24::CaptionCharacter {
        kind: 0,
        codepoint: text.chars().next().unwrap() as u32,
        pua_codepoint: 0,
        drcs_code: 0,
        x,
        y: 200,
        width: 36,
        height: 36,
        horizontal_spacing: 4,
        vertical_spacing: 0,
        horizontal_scale: 1.0,
        vertical_scale: 1.0,
        text_color: 0xffff_ffff,
        back_color: 0,
        stroke_color: 0xff00_0000,
        style: 0,
        enclosure_style: 0,
        utf8: text.into(),
    };
    let interval = RegionInterval {
        begin_ms: 100,
        end_ms: 1_000,
        wait_duration_ms: 900,
        plane_width: 960,
        plane_height: 540,
        source_pid: Some(0x120),
        region: native_b24::CaptionRegion {
            x: 100,
            y: 200,
            width: 400,
            height: 36,
            is_ruby: false,
            first_character: 0,
            character_count: 3,
        },
        characters: vec![
            character("日", 100),
            character("本", 140),
            character("語", 300),
        ],
        drcs_glyphs: Vec::new(),
        ruby_binding: None,
    };
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_interval(&mut writer, &interval, &ConversionOptions::default()).expect("write ASS");
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    assert_eq!(ass.matches("Dialogue: ").count(), 2);
    assert!(ass.contains("日本"));
    assert!(ass.contains("語"));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn unpositioned_b24_group_orders_fragments_by_source_rows_and_writes_one_cue() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-b24-unpositioned-{stamp}.ass"));
    let interval = |text: &str, x: i32, y: i32| RegionInterval {
        begin_ms: 36_900,
        end_ms: 43_400,
        wait_duration_ms: 6_500,
        plane_width: 960,
        plane_height: 540,
        source_pid: Some(0x130),
        region: native_b24::CaptionRegion {
            x,
            y,
            width: 900,
            height: 72,
            is_ruby: false,
            first_character: 0,
            character_count: 1,
        },
        characters: text
            .chars()
            .enumerate()
            .map(|(index, character)| native_b24::CaptionCharacter {
                kind: 0,
                codepoint: character as u32,
                pua_codepoint: 0,
                drcs_code: 0,
                x: x + index as i32 * 40,
                y,
                width: 36,
                height: 36,
                horizontal_spacing: 4,
                vertical_spacing: 0,
                horizontal_scale: 1.0,
                vertical_scale: 1.0,
                text_color: 0xffff_ffff,
                back_color: 0,
                stroke_color: 0xff00_0000,
                style: 0,
                enclosure_style: 0,
                utf8: character.to_string(),
            })
            .collect(),
        drcs_glyphs: Vec::new(),
        ruby_binding: None,
    };
    // Deliberately use transport/closure order rather than visual order.
    let intervals = vec![
        interval("教会の友人で", 900, 800),
        interval("困った時は お互いさまですから｡", 520, 872),
        interval("(寛太)いえ 直美さんとは ", 420, 800),
    ];
    let options = ConversionOptions {
        preserve_position: false,
        preserve_accessibility: false,
        preserve_gaiji: false,
        ..ConversionOptions::default()
    };
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_interval_group(&mut writer, &intervals, &options).expect("write group");
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    assert_eq!(ass.matches("Dialogue: ").count(), 1);
    assert!(ass.contains("いえ 直美さんとは 教会の友人で\\N困った時は お互いさまですから｡"));
    assert!(ass.contains("\\fs72"));
    assert!(!ass.contains("(寛太)"));
    assert!(!ass.contains("\\pos("));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn export_feature_filter_removes_the_same_character_ranges_as_the_event_inspector() {
    let filtered =
        crate::caption_features::filtered_text("(寛太)説明⚟➡♬〜本文<語り>", false, false);
    assert_eq!(filtered, "説明本文語り");
}

#[test]
fn b24_gaiji_filter_uses_the_arib_symbol_row_instead_of_every_pua_source() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-b24-pua-{stamp}.ass"));
    let mut ordinary = scene_intervals(&scene_with_text_regions(0, &[(100, 100, "常")]))
        .pop()
        .expect("ordinary interval");
    ordinary.characters[0].pua_codepoint = 0xE000;
    let mut symbol = scene_intervals(&scene_with_text_regions(1_000, &[(100, 100, "X")]))
        .pop()
        .expect("symbol interval");
    symbol.characters[0].pua_codepoint = 0xE28F;
    let options = ConversionOptions {
        preserve_gaiji: false,
        ..ConversionOptions::default()
    };
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    write_ass_interval(&mut writer, &ordinary, &options).expect("ordinary output");
    write_ass_interval(&mut writer, &symbol, &options).expect("symbol output");
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    assert!(ass.contains('常'));
    assert!(!ass.contains('X'));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn b24_ruby_above_and_below_use_the_same_visual_gap() {
    let character = |text: &str, x: i32, y: i32, scale: f32| native_b24::CaptionCharacter {
        kind: 0,
        codepoint: text.chars().next().unwrap() as u32,
        pua_codepoint: 0,
        drcs_code: 0,
        x,
        y,
        width: 36,
        height: 36,
        horizontal_spacing: 4,
        vertical_spacing: 24,
        horizontal_scale: scale,
        vertical_scale: scale,
        text_color: 0xffff_ffff,
        back_color: 0,
        stroke_color: 0xff00_0000,
        style: 0,
        enclosure_style: 0,
        utf8: text.into(),
    };
    let characters = vec![
        character("の", 340, 329, 0.5),
        character("こ", 360, 329, 0.5),
        character("遺", 340, 359, 1.0),
        character("想", 420, 419, 1.0),
        character("お", 420, 479, 0.5),
        character("も", 440, 479, 0.5),
    ];
    let scene = native_b24::CaptionScene {
        pts_ms: 0,
        wait_duration_ms: 1_000,
        plane_width: 960,
        plane_height: 540,
        regions: vec![
            native_b24::CaptionRegion {
                x: 340,
                y: 329,
                width: 40,
                height: 30,
                is_ruby: true,
                first_character: 0,
                character_count: 2,
            },
            native_b24::CaptionRegion {
                x: 340,
                y: 359,
                width: 40,
                height: 60,
                is_ruby: false,
                first_character: 2,
                character_count: 1,
            },
            native_b24::CaptionRegion {
                x: 420,
                y: 419,
                width: 40,
                height: 60,
                is_ruby: false,
                first_character: 3,
                character_count: 1,
            },
            native_b24::CaptionRegion {
                x: 420,
                y: 479,
                width: 40,
                height: 30,
                is_ruby: true,
                first_character: 4,
                character_count: 2,
            },
        ],
        characters,
        drcs_glyphs: Vec::new(),
        rendered_image: None,
    };
    let intervals = scene_intervals(&scene);
    assert_eq!(
        intervals[0]
            .ruby_binding
            .as_ref()
            .map(|binding| binding.source_ruby_box.y),
        Some(329)
    );
    assert_eq!(
        intervals[3]
            .ruby_binding
            .as_ref()
            .map(|binding| binding.source_ruby_box.y),
        Some(467)
    );
    assert_eq!(
        intervals[0]
            .ruby_binding
            .as_ref()
            .map(|binding| binding.base_cell_boxes.len()),
        Some(1)
    );
    assert_eq!(
        intervals[3]
            .ruby_binding
            .as_ref()
            .map(|binding| binding.base_cell_boxes.len()),
        Some(1)
    );

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-b24-ruby-centre-{stamp}.ass"));
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    for interval in &intervals {
        write_ass_interval(&mut writer, interval, &ConversionOptions::default()).expect("write");
    }
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    let ruby_lines = ass
        .lines()
        .filter(|line| {
            line.starts_with("Dialogue: 1,")
                && ['の', 'こ', 'お', 'も']
                    .iter()
                    .any(|character| line.ends_with(*character))
        })
        .collect::<Vec<_>>();
    assert_eq!(ruby_lines.len(), 4);
    assert!(ruby_lines.iter().all(|line| line.contains("{\\an8\\pos(")));
    assert!(
        ruby_lines
            .iter()
            .all(|line| line.contains("\\fnRounded M+ 1m for ARIB"))
    );
    assert!(ruby_lines.iter().all(|line| !line.contains("\\pos(680,")));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn b24_multi_character_ruby_centres_on_the_base_layout_axis_without_moving_it() {
    let character = |text: &str, x: i32, y: i32, scale: f32| native_b24::CaptionCharacter {
        kind: 0,
        codepoint: text.chars().next().unwrap() as u32,
        pua_codepoint: 0,
        drcs_code: 0,
        x,
        y,
        width: 36,
        height: 36,
        horizontal_spacing: 4,
        vertical_spacing: 24,
        horizontal_scale: scale,
        vertical_scale: scale,
        text_color: 0xffff_ffff,
        back_color: 0,
        stroke_color: 0xff00_0000,
        style: 0,
        enclosure_style: 0,
        utf8: text.into(),
    };
    let scene = native_b24::CaptionScene {
        pts_ms: 0,
        wait_duration_ms: 1_000,
        plane_width: 960,
        plane_height: 540,
        regions: vec![
            native_b24::CaptionRegion {
                x: 100,
                y: 120,
                width: 80,
                height: 30,
                is_ruby: true,
                first_character: 0,
                character_count: 4,
            },
            native_b24::CaptionRegion {
                x: 100,
                y: 150,
                width: 80,
                height: 60,
                is_ruby: false,
                first_character: 4,
                character_count: 2,
            },
        ],
        characters: vec![
            character("じ", 100, 120, 0.5),
            character("ゅ", 120, 120, 0.5),
            character("し", 140, 120, 0.5),
            character("ん", 160, 120, 0.5),
            character("受", 100, 150, 1.0),
            character("信", 140, 150, 1.0),
        ],
        drcs_glyphs: Vec::new(),
        rendered_image: None,
    };
    let mut intervals = scene_intervals(&scene);
    for interval in &mut intervals {
        interval.end_ms = 1_000;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-b24-group-ruby-{stamp}.ass"));
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    writeln!(
        writer,
        "[Script Info]\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\n\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\nStyle: Default,Rounded M+ 1m for ARIB,42,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,7,20,20,20,1\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text"
    )
    .expect("header");
    for interval in &intervals {
        write_ass_interval(&mut writer, interval, &ConversionOptions::default()).expect("write");
    }
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    let base = ass
        .lines()
        .find(|line| line.contains("受信"))
        .expect("base");
    let ruby = ass
        .lines()
        .filter(|line| {
            line.starts_with("Dialogue: 1,")
                && ['じ', 'ゅ', 'し', 'ん']
                    .iter()
                    .any(|character| line.ends_with(*character))
        })
        .collect::<Vec<_>>();
    assert!(base.contains("{\\an7\\pos(200,300)}"));
    assert_eq!(ruby.len(), 4);
    assert!(ruby.iter().all(|line| line.contains("{\\an8\\pos(")));

    // Verify the rendered result, not only the ASS coordinates. Ruby and its
    // base line must have the same horizontal pixel centre in libass.
    let fixture = output.with_extension("png");
    let fixture_font = output.with_extension("ttf");
    fs::write(&fixture_font, bundled_ass_font()).expect("write bundled test font");
    let ass_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .expect("ASS fixture name");
    let filter = format!("ass=filename='{ass_name}':fontsdir='.'");
    let status = std::process::Command::new("ffmpeg")
        .current_dir(output.parent().expect("fixture directory"))
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=1920x1080:r=1:d=1",
            "-vf",
            &filter,
            "-frames:v",
            "1",
            "-y",
            fixture.to_string_lossy().as_ref(),
        ])
        .status();
    if let Ok(status) = status {
        assert!(status.success(), "ffmpeg/libass B24 visual render failed");
        let image = image::open(&fixture)
            .expect("read B24 visual fixture")
            .to_rgb8();
        let centre = |y_start: u32, y_end: u32| {
            let mut min = u32::MAX;
            let mut max = 0;
            for y in y_start..y_end {
                for x in 100..500 {
                    let [r, g, b] = image.get_pixel(x, y).0;
                    if r > 180 && g > 180 && b > 180 {
                        min = min.min(x);
                        max = max.max(x);
                    }
                }
            }
            assert!(min <= max, "expected rendered caption pixels");
            (min + max) as i32 / 2
        };
        let ruby_centre = centre(220, 300);
        let base_centre = centre(300, 390);
        assert!(
            (ruby_centre - base_centre).abs() <= 2,
            "ruby centre {ruby_centre}, base centre {base_centre}"
        );
        fs::remove_file(fixture).expect("cleanup visual fixture");
    }
    fs::remove_file(fixture_font).expect("cleanup test font");
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn b24_ruby_grid_does_not_claim_the_following_overlapping_cell() {
    let character = |text: &str, x: i32, y: i32, scale: f32| native_b24::CaptionCharacter {
        kind: 0,
        codepoint: text.chars().next().unwrap() as u32,
        pua_codepoint: 0,
        drcs_code: 0,
        x,
        y,
        width: 36,
        height: 36,
        horizontal_spacing: 4,
        vertical_spacing: 24,
        horizontal_scale: scale,
        vertical_scale: scale,
        text_color: 0xffff_ffff,
        back_color: 0,
        stroke_color: 0xff00_0000,
        style: 0,
        enclosure_style: 0,
        utf8: text.into(),
    };
    let mut unresolved_drcs = character("遺", 300, 150, 1.0);
    unresolved_drcs.kind = 1;
    unresolved_drcs.codepoint = 0;
    unresolved_drcs.drcs_code = 0x1234;
    unresolved_drcs.utf8.clear();
    let scene = native_b24::CaptionScene {
        pts_ms: 0,
        wait_duration_ms: 1_000,
        plane_width: 960,
        plane_height: 540,
        regions: vec![
            native_b24::CaptionRegion {
                x: 340,
                y: 120,
                width: 40,
                height: 30,
                is_ruby: true,
                first_character: 0,
                character_count: 2,
            },
            native_b24::CaptionRegion {
                x: 300,
                y: 150,
                width: 120,
                height: 60,
                is_ruby: false,
                first_character: 2,
                character_count: 3,
            },
        ],
        characters: vec![
            character("の", 340, 120, 0.5),
            character("こ", 360, 120, 0.5),
            unresolved_drcs,
            character("遺", 340, 150, 1.0),
            character("し", 380, 150, 1.0),
        ],
        drcs_glyphs: Vec::new(),
        rendered_image: None,
    };
    let mut intervals = scene_intervals(&scene);
    for interval in &mut intervals {
        interval.end_ms = 1_000;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("resubwinny-b24-grid-ruby-{stamp}.ass"));
    let binding = intervals[0].ruby_binding.as_ref().expect("ruby binding");
    assert_eq!(binding.base_start, 1);
    assert_eq!(binding.base_end, 2);
    assert_eq!(binding.base_text, "遺");
    let mut writer = BufWriter::new(File::create(&output).expect("output"));
    writeln!(writer, "[Script Info]\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\n\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\nStyle: Default,Rounded M+ 1m for ARIB,42,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,7,20,20,20,1\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text").expect("header");
    for interval in &intervals {
        write_ass_interval(&mut writer, interval, &ConversionOptions::default()).expect("write");
    }
    writer.flush().expect("flush");
    let ass = fs::read_to_string(&output).expect("read ASS");
    let ruby = ass
        .lines()
        .filter(|line| {
            line.starts_with("Dialogue: 1,") && (line.ends_with('の') || line.ends_with('こ'))
        })
        .collect::<Vec<_>>();
    assert_eq!(ruby.len(), 2);
    assert!(ruby.iter().all(|line| line.contains("{\\an8\\pos(")));
    fs::remove_file(output).expect("cleanup");
}

#[test]
fn b24_ruby_target_recovery_handles_mixed_full_and_half_width_cells() {
    let character = |text: &str, x: i32, scale: f32| native_b24::CaptionCharacter {
        kind: 0,
        codepoint: text.chars().next().unwrap() as u32,
        pua_codepoint: 0,
        drcs_code: 0,
        x,
        y: 449,
        width: 36,
        height: 36,
        horizontal_spacing: 4,
        vertical_spacing: 24,
        horizontal_scale: scale,
        vertical_scale: 1.0,
        text_color: 0xffff_ffff,
        back_color: 0,
        stroke_color: 0xff00_0000,
        style: 0,
        enclosure_style: 0,
        utf8: text.into(),
    };

    let self_build = vec![
        character("自", 140, 1.0),
        character("家", 180, 1.0),
        character("製", 220, 1.0),
        character("ハ", 260, 1.0),
        character("ウ", 300, 1.0),
        character("ス", 340, 1.0),
    ];
    assert_eq!(
        ruby_target_indices(&self_build, 140, 276, "セルフビルド"),
        vec![0, 1, 2]
    );

    let vomit = vec![
        character("(", 380, 0.5),
        character("嘔", 400, 1.0),
        character("吐", 440, 1.0),
        character("す", 480, 1.0),
        character("る", 520, 1.0),
        character(")", 560, 0.5),
    ];
    assert_eq!(ruby_target_indices(&vomit, 400, 476, "おうと"), vec![1, 2]);

    let and = vec![character("及", 360, 1.0), character("び", 400, 1.0)];
    assert_eq!(ruby_target_indices(&and, 360, 416, "およ"), vec![0]);

    let twenty = vec![
        character("ス", 660, 1.0),
        character("2", 700, 0.5),
        character("0", 720, 0.5),
    ];
    assert_eq!(
        ruby_target_indices(&twenty, 680, 776, "フタマル"),
        vec![1, 2]
    );
}

#[test]
fn writes_a_drcs_report_that_references_visual_assets_without_pixels() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let output = std::env::temp_dir().join(format!("arib-drcs-report-{stamp}.ass"));
    let directory = output.with_extension("drcs");
    let glyph = native_b24::DrcsGlyph {
        drcs_code: 0x2a7f,
        width: 16,
        height: 16,
        depth: 4,
        depth_bits: 2,
        alternative_codepoint: 0,
        md5: "glyph-hash".into(),
        alternative_text: "".into(),
        pixels: vec![0; 64],
    };
    let mut glyphs = BTreeMap::new();
    glyphs.insert(drcs_asset_key(&glyph), glyph);
    let report = write_drcs_report(&output, Path::new("sample.ts"), &directory, &glyphs, false)
        .expect("write report")
        .expect("report path");
    let report_text = fs::read_to_string(&report).expect("read report");
    assert!(report_text.contains("arib_caption_drcs_report"));
    assert!(report_text.contains("drcs-glyph-hash.json"));
    assert!(!report_text.contains("pixels"));
    fs::remove_file(report).expect("cleanup report");
}
