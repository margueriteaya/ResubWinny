use super::{
    archive::{
        encode_scene_image, interval_bounds, preview_cache_metrics, render_at, render_overlay_at,
        scene_bounds,
    },
    overlay::{
        OverlayAction, decide_overlay_action, playback_file_offset, seconds_to_milliseconds,
    },
    parse_mpv_time_response,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde_json::json;

// The preview archive cache intentionally mirrors the single active archive
// used by the desktop player.  Keep tests that exercise it serialized so a
// parallel test cannot swap the global cache source between metric snapshots.
static PREVIEW_ARCHIVE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn preview_archive_test_guard() -> std::sync::MutexGuard<'static, ()> {
    PREVIEW_ARCHIVE_TEST_LOCK
        .lock()
        .expect("preview archive test lock")
}

#[test]
fn overlay_sync_only_updates_when_the_caption_plane_changes() {
    let empty = crate::state::PreviewOverlaySyncState::default();
    assert_eq!(
        decide_overlay_action(&empty, "archive.jsonl", Some(7)),
        OverlayAction::Apply
    );
    let current = crate::state::PreviewOverlaySyncState {
        archive: "archive.jsonl".into(),
        fingerprint: Some(7),
        overlay_visible: true,
        revision: 0,
    };
    assert_eq!(
        decide_overlay_action(&current, "archive.jsonl", Some(7)),
        OverlayAction::Unchanged
    );
    assert_eq!(
        decide_overlay_action(&current, "archive.jsonl", Some(8)),
        OverlayAction::Apply
    );
    assert_eq!(
        decide_overlay_action(&current, "archive.jsonl", None),
        OverlayAction::Clear
    );
    assert_eq!(
        decide_overlay_action(&empty, "archive.jsonl", None),
        OverlayAction::Unchanged
    );
}

#[test]
fn overlay_sync_converts_mpv_seconds_without_negative_times() {
    assert_eq!(seconds_to_milliseconds(1.234), 1_234);
    assert_eq!(seconds_to_milliseconds(-0.1), 0);
}

#[test]
fn broadcast_metadata_offset_follows_the_displayed_playback_time() {
    assert_eq!(
        playback_file_offset(1_000_000, Some(25.0), Some(100.0), Some(900_000.0)),
        250_000
    );
    assert_eq!(
        playback_file_offset(1_000_000, None, Some(100.0), Some(123_456.0)),
        123_456
    );
}

fn png_fingerprint(encoded: &str) -> u64 {
    BASE64
        .decode(encoded)
        .expect("base64 PNG")
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

#[test]
fn accepts_snake_and_camel_case_interval_times() {
    assert_eq!(
        interval_bounds(&json!({"begin_ms": 100, "end_ms": 250})),
        Some((100, 250))
    );
    assert_eq!(
        interval_bounds(&json!({"startMs": 300, "endMs": 450})),
        Some((300, 450))
    );
}

#[test]
fn gives_open_caption_records_a_bounded_preview_duration() {
    assert_eq!(
        interval_bounds(&json!({"start_ms": 1_000})),
        Some((1_000, 6_000))
    );
    assert_eq!(
        interval_bounds(&json!({"start_ms": 1_000, "end_ms": 1_000})),
        None
    );
}

#[test]
fn uses_scene_pts_and_wait_for_render_snapshots() {
    let value = json!({"kind": "scene", "pts_ms": 2_000, "wait_duration_ms": 1_500});
    assert_eq!(scene_bounds(&value, Some("scene")), Some((2_000, 3_500)));
}

#[test]
fn keeps_an_open_b24_scene_until_a_later_scene_replaces_it() {
    let value = json!({"pts_ms": 2_000, "wait_duration_ms": i64::MAX});
    assert_eq!(scene_bounds(&value, Some("scene")), Some((2_000, i64::MAX)));
}

#[test]
fn converts_strided_rgba_scene_pixels_to_png() {
    let raw = base64::engine::general_purpose::STANDARD.encode([255_u8, 0, 0, 128]);
    let value = json!({
        "kind": "scene",
        "rendered_image": {"width": 1, "height": 1, "stride": 4, "rgba_base64": raw}
    });
    let encoded = encode_scene_image(value);
    let image = encoded.get("rendered_image").expect("rendered image");
    assert!(
        image
            .get("png_base64")
            .and_then(|value| value.as_str())
            .is_some()
    );
    assert!(image.get("rgba_base64").is_none());
}

#[test]
fn parses_mpv_time_only_from_successful_finite_responses() {
    assert_eq!(
        parse_mpv_time_response(r#"{"error":"success","data":12.5}"#),
        Some(12.5)
    );
    assert_eq!(
        parse_mpv_time_response(r#"{"error":"property unavailable","data":null}"#),
        None
    );
    assert_eq!(
        parse_mpv_time_response(r#"{"error":"success","data":-1}"#),
        None
    );
    assert_eq!(parse_mpv_time_response("not json"), None);
}

#[test]
fn builds_documented_mpv_overlay_add_arguments() {
    let command = super::mpv_overlay_command(
        std::path::Path::new("C:\\Temp\\caption.bgra"),
        12,
        24,
        960,
        540,
    );
    assert_eq!(command["command"][0], "overlay-add");
    assert_eq!(command["command"][4], "C:\\Temp\\caption.bgra");
    assert_eq!(command["command"][6], "bgra");
    assert_eq!(command["command"][9], 3_840);
}

#[test]
fn reads_worker_jsonl_envelopes_for_active_scenes() {
    let _cache_guard = preview_archive_test_guard();
    let path =
        std::env::temp_dir().join(format!("resubwinny-render-at-{}.jsonl", std::process::id()));
    std::fs::write(
        &path,
        r#"{"type":"scene","value":{"pts_ms":1000,"wait_duration_ms":500,"text":"字幕"}}"#,
    )
    .expect("archive fixture");
    let snapshot = render_at(path.to_string_lossy().into_owned(), 1_200).expect("render snapshot");
    assert_eq!(snapshot.intervals.len(), 1);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn renders_only_the_latest_b24_scene_snapshot() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-scene-replacement-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
            &path,
            concat!(
                r#"{"type":"scene","value":{"pts_ms":1000,"wait_duration_ms":9223372036854775807,"text":"old"}}"#,
                "\n",
                r#"{"type":"region_interval","value":{"begin_ms":1000,"end_ms":5000,"text":"old region"}}"#,
                "\n",
                r#"{"type":"scene","value":{"pts_ms":2000,"wait_duration_ms":9223372036854775807,"text":"new"}}"#,
            ),
        )
        .expect("archive fixture");
    let snapshot =
        render_at(path.to_string_lossy().into_owned(), 2_500).expect("scene replacement snapshot");
    assert_eq!(snapshot.intervals.len(), 1);
    assert_eq!(snapshot.intervals[0]["text"], "new");
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_native_ttml_ruby_visual_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-ttml-ruby-golden-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
            &path,
            r##"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"漢","x":800,"y":800,"width":160,"height":160,"style":{"font_size":"96px","color":"#FFFFFFFF","background_color":"#00000080","writing_mode":"horizontal-tb","text_outline":"2px #000000"},"rich_body":"<ruby><span tts:ruby='base'>漢</span><rt><span tts:ruby='text'>かん</span></rt></ruby>"}}"##,
        )
        .expect("archive fixture");

    let snapshot =
        render_at(path.to_string_lossy().into_owned(), 1_200).expect("native ruby snapshot");
    let png = snapshot.composed_png_base64.expect("native PNG");
    assert_eq!(
        snapshot.caption_plane_mode,
        "ttml-horizontal-ruby-basic-native"
    );
    assert_eq!(snapshot.rendered_ruby_count, 1);
    assert_eq!(png_fingerprint(&png), 0x25B1_5CE8_08DB_AF4F);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_arib_receiver_baseline_stroke_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-ttml-receiver-stroke-golden-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
        &path,
        r##"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"字幕","x":760,"y":760,"width":400,"height":160,"style":{"font_size":"96px","font_family":"丸ゴシック","color":"#FFFFFFFF","background_color":"#00000080","writing_mode":"horizontal-tb"}}}"##,
    )
    .expect("archive fixture");

    let snapshot = render_at(path.to_string_lossy().into_owned(), 1_200)
        .expect("ARIB receiver baseline snapshot");
    let png = snapshot.composed_png_base64.expect("native PNG");
    assert_eq!(snapshot.caption_plane_mode, "ttml-horizontal-native");
    assert_eq!(png_fingerprint(&png), 0x57A4_3C37_BBB3_6040);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_native_vertical_ruby_visual_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-ttml-vertical-ruby-golden-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
            &path,
            r##"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"漢字","x":960,"y":120,"width":180,"height":600,"style":{"font_size":"96px","color":"#FFFFFFFF","background_color":"#00000080","writing_mode":"vertical-rl","text_outline":"2px #000000"},"rich_body":"<ruby><span tts:ruby='base'>漢字</span><rt><span tts:ruby='text'>かんじ</span></rt></ruby>"}}"##,
        )
        .expect("archive fixture");

    let snapshot = render_at(path.to_string_lossy().into_owned(), 1_200)
        .expect("native vertical ruby snapshot");
    let png = snapshot.composed_png_base64.expect("native PNG");
    assert_eq!(
        snapshot.caption_plane_mode,
        "ttml-vertical-ruby-basic-native"
    );
    assert_eq!(snapshot.rendered_ruby_count, 1);
    assert_eq!(png_fingerprint(&png), 0x4C48_B78F_5D57_3626);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_wrapped_vertical_ruby_visual_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-ttml-wrapped-vertical-ruby-golden-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
            &path,
            r##"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"漢字仮名","x":960,"y":120,"width":180,"height":96,"style":{"font_size":"96px","color":"#FFFFFFFF","background_color":"#00000080","writing_mode":"vertical-rl","text_outline":"2px #000000"},"rich_body":"<ruby><span tts:ruby='base'>漢字仮名</span><rt><span tts:ruby='text'>かんじかな</span></rt></ruby>"}}"##,
        )
        .expect("archive fixture");

    let snapshot = render_at(path.to_string_lossy().into_owned(), 1_200)
        .expect("wrapped native vertical ruby snapshot");
    let png = snapshot.composed_png_base64.expect("native PNG");
    assert_eq!(
        snapshot.caption_plane_mode,
        "ttml-vertical-ruby-basic-native"
    );
    assert_eq!(snapshot.rendered_ruby_count, 1);
    assert_eq!(png_fingerprint(&png), 0xEFCF_409A_9E61_3AA7);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_vertical_text_combine_visual_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-ttml-vertical-text-combine-golden-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
            &path,
            r##"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"24年","x":960,"y":120,"width":120,"height":300,"style":{"font_size":"96px","color":"#FFFFFFFF","background_color":"#00000080","writing_mode":"vertical-rl","text_outline":"2px #000000"},"rich_body":"<span tts:textCombine='all'>24</span>年"}}"##,
        )
        .expect("archive fixture");

    let snapshot = render_at(path.to_string_lossy().into_owned(), 1_200)
        .expect("native vertical text-combine snapshot");
    let png = snapshot.composed_png_base64.expect("native PNG");
    assert_eq!(snapshot.caption_plane_mode, "ttml-vertical-basic-native");
    assert_eq!(snapshot.missing_glyph_count, 0);
    assert_eq!(png_fingerprint(&png), 0x552C_864E_D8F9_8943);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_vertical_punctuation_visual_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-ttml-vertical-punctuation-golden-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
            &path,
            r##"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"「字幕」。","x":960,"y":120,"width":120,"height":600,"style":{"font_size":"96px","color":"#FFFFFFFF","background_color":"#00000080","writing_mode":"vertical-rl","text_outline":"2px #000000"}}}"##,
        )
        .expect("archive fixture");

    let snapshot = render_at(path.to_string_lossy().into_owned(), 1_200)
        .expect("native vertical punctuation snapshot");
    let png = snapshot.composed_png_base64.expect("native PNG");
    assert_eq!(snapshot.caption_plane_mode, "ttml-vertical-basic-native");
    assert_eq!(snapshot.missing_glyph_count, 0);
    assert_eq!(png_fingerprint(&png), 0x5399_899F_880B_2454);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_multiline_alignment_visual_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-ttml-multiline-alignment-golden-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
            &path,
            r##"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"字幕一行目\n字幕二行目","x":400,"y":600,"width":800,"height":300,"style":{"font_size":"72px","line_height":"120px","color":"#FFFF00FF","background_color":"#00000080","writing_mode":"horizontal-tb","text_align":"center","display_align":"after","text_outline":"2px #000000"}}}"##,
        )
        .expect("archive fixture");

    let snapshot =
        render_at(path.to_string_lossy().into_owned(), 1_200).expect("native multiline snapshot");
    let png = snapshot.composed_png_base64.expect("native PNG");
    assert_eq!(snapshot.caption_plane_mode, "ttml-horizontal-native");
    assert_eq!(png_fingerprint(&png), 0xAFCF_A4D5_24CC_7CD6);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn render_at_has_a_stable_b24_composition_visual_golden() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-b24-composition-golden-{}.jsonl",
        std::process::id()
    ));
    let rgba = BASE64.encode([255_u8, 0, 0, 255, 0, 0, 255, 255]);
    std::fs::write(
            &path,
            format!(
                r#"{{"type":"scene","value":{{"pts_ms":1000,"wait_duration_ms":500,"rendered_image":{{"width":2,"height":1,"stride":8,"rgba_base64":"{rgba}","dst_x":0,"dst_y":0}}}}}}"#
            ),
        )
        .expect("archive fixture");

    let snapshot =
        render_at(path.to_string_lossy().into_owned(), 1_200).expect("B24 composition snapshot");
    let png = snapshot.composed_png_base64.expect("composed PNG");
    assert_eq!(snapshot.caption_plane_mode, "b24-native-rgba");
    assert_eq!(
        (snapshot.plane_width, snapshot.plane_height),
        (Some(2), Some(1))
    );
    assert_eq!(png_fingerprint(&png), 0x5C36_DCE2_120D_B61F);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn playback_cache_keeps_raw_rgba_without_eager_png_encoding() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-b24-direct-rgba-{}.jsonl",
        std::process::id()
    ));
    let rgba = BASE64.encode([255_u8, 0, 0, 255, 0, 0, 255, 255]);
    std::fs::write(
        &path,
        format!(
            r#"{{"type":"scene","value":{{"pts_ms":1000,"wait_duration_ms":500,"rendered_image":{{"width":2,"height":1,"stride":8,"rgba_base64":"{rgba}","dst_x":0,"dst_y":0}}}}}}"#
        ),
    )
    .expect("archive fixture");

    let render =
        render_overlay_at(path.to_string_lossy().into_owned(), 1_200).expect("cached RGBA render");
    assert!(render.snapshot.composed_png_base64.is_none());
    let overlay = render.overlay.as_ref().expect("native RGBA overlay");
    assert_eq!(&*overlay.pixels, &[255, 0, 0, 255, 0, 0, 255, 255]);
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
#[ignore = "requires RESUBWINNY_PREVIEW_PERF_ARCHIVE"]
fn real_archive_overlay_meets_the_interactive_performance_gate() {
    let _cache_guard = preview_archive_test_guard();
    let archive = std::env::var("RESUBWINNY_PREVIEW_PERF_ARCHIVE")
        .expect("set RESUBWINNY_PREVIEW_PERF_ARCHIVE");
    let target_ms = std::env::var("RESUBWINNY_PREVIEW_PERF_TIME_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000_000_i64);
    let started = std::time::Instant::now();
    let render = render_overlay_at(archive, target_ms).expect("large preview overlay");
    let elapsed = started.elapsed();
    let (_, parsed_lines, compositions, resets, checkpoints) = preview_cache_metrics();
    eprintln!(
        "large preview overlay: {} layers, {parsed_lines} parsed lines, {compositions} compositions, {resets} resets, {checkpoints} checkpoints in {:.2} ms",
        render.snapshot.active_layer_count,
        elapsed.as_secs_f64() * 1_000.0
    );
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "large preview overlay took {elapsed:?}"
    );
}

#[test]
fn advances_preview_cursor_without_reloading_earlier_records() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-render-cache-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(
        &path,
        concat!(
            r#"{"type":"scene","value":{"pts_ms":1000,"wait_duration_ms":500,"text":"一"}}"#,
            "\n",
            r#"{"type":"scene","value":{"pts_ms":2000,"wait_duration_ms":500,"text":"二"}}"#,
            "\n",
        ),
    )
    .expect("archive fixture");
    assert_eq!(
        render_at(path.to_string_lossy().into_owned(), 1_100)
            .expect("first snapshot")
            .intervals
            .len(),
        1
    );
    assert_eq!(
        render_at(path.to_string_lossy().into_owned(), 2_100)
            .expect("second snapshot")
            .intervals
            .len(),
        1
    );
    assert_eq!(
        render_at(path.to_string_lossy().into_owned(), 1_100)
            .expect("rewound snapshot")
            .intervals
            .len(),
        1
    );
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn attaches_same_mpu_resource_previews_to_active_ttml_captions() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-resource-preview-{}.jsonl",
        std::process::id()
    ));
    std::fs::write(&path, concat!(
            r#"{"type":"resource_evidence","value":{"record_key":"stpp-resource:packet:1113:mpu:7:subsample:4","format_hint":"png","format_validation":"header-validated","preview_data_uri":"data:image/png;base64,AA=="}}"#, "\n",
            r#"{"type":"caption","value":{"start_ms":1000,"end_ms":1500,"text":"字幕","style":{"background_image":"subt://4"},"source":{"mmpt_packet_id":1113,"mpu_sequence_number":7}}}"#, "\n",
        )).expect("archive fixture");
    let snapshot = render_at(path.to_string_lossy().into_owned(), 1_200).expect("render snapshot");
    assert_eq!(snapshot.resource_previews.len(), 1);
    assert_eq!(snapshot.resource_previews[0]["usage"], "background-image");
    assert_eq!(
        snapshot.intervals[0]["native_resources"][0]["record_key"],
        "stpp-resource:packet:1113:mpu:7:subsample:4"
    );
    std::fs::remove_file(path).expect("remove fixture");
}

#[test]
fn growing_preview_archive_keeps_cursor_and_reuses_caption_plane() {
    let _cache_guard = preview_archive_test_guard();
    use std::io::Write as _;

    let path = std::env::temp_dir().join(format!(
        "resubwinny-growing-preview-cache-{}.jsonl",
        std::process::id()
    ));
    let mut fixture = String::new();
    for index in 0..=120 {
        fixture.push_str(&format!(
            "{{\"type\":\"scene\",\"value\":{{\"pts_ms\":{},\"wait_duration_ms\":1000,\"text\":\"event-{index}\"}}}}\n",
            index * 1_000
        ));
    }
    std::fs::write(&path, fixture).expect("growing archive fixture");

    render_at(path.to_string_lossy().into_owned(), 90_200).expect("initial render");
    let before = preview_cache_metrics();
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("append growing archive");
    writeln!(
        file,
        "{{\"type\":\"scene\",\"value\":{{\"pts_ms\":121000,\"wait_duration_ms\":1000,\"text\":\"appended\"}}}}"
    )
    .expect("append preview event");
    drop(file);

    render_at(path.to_string_lossy().into_owned(), 90_250).expect("render after append");
    let after = preview_cache_metrics();
    assert_eq!(after.3, before.3, "archive growth must not reset the cache");
    assert_eq!(
        after.2, before.2,
        "an unchanged caption plane must not be recomposed"
    );
    assert!(
        after.1.saturating_sub(before.1) <= 2,
        "archive growth reparsed too many lines: before={}, after={}",
        before.1,
        after.1
    );
    std::fs::remove_file(path).expect("remove growing archive fixture");
}

#[test]
fn rewind_uses_sparse_preview_checkpoint_instead_of_file_start() {
    let _cache_guard = preview_archive_test_guard();
    let path = std::env::temp_dir().join(format!(
        "resubwinny-preview-checkpoints-{}.jsonl",
        std::process::id()
    ));
    let mut fixture = String::new();
    for index in 0..=180 {
        fixture.push_str(&format!(
            "{{\"type\":\"scene\",\"value\":{{\"pts_ms\":{},\"wait_duration_ms\":1000,\"text\":\"event-{index}\"}}}}\n",
            index * 1_000
        ));
    }
    std::fs::write(&path, fixture).expect("checkpoint archive fixture");

    for time_ms in [100, 30_100, 60_100, 90_100] {
        render_at(path.to_string_lossy().into_owned(), time_ms).expect("forward checkpoint render");
    }
    let before = preview_cache_metrics();
    assert!(before.4 >= 4, "expected sparse preview checkpoints");
    render_at(path.to_string_lossy().into_owned(), 60_100).expect("checkpoint rewind render");
    let after = preview_cache_metrics();
    assert!(
        after.0 < before.0,
        "rewind should restore an earlier byte cursor"
    );
    assert!(
        after.1.saturating_sub(before.1) <= 2,
        "checkpoint rewind reparsed too many lines: before={}, after={}",
        before.1,
        after.1
    );
    std::fs::remove_file(path).expect("remove checkpoint archive fixture");
}
