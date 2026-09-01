use super::{
    StyledRun, VerticalGlyphOrientation, apply_opacity, compose, horizontal_lines,
    is_arib_rounded_caption, parse_font_height, parse_opacity, parse_rgba, parse_text_outline,
    rotate_bitmap_clockwise, styled_runs, text_combine_digit_count, vertical_glyph_orientation,
    vertical_presentation_form,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use fontdue::{Font, FontSettings};
use serde_json::json;
use std::path::{Path, PathBuf};

fn visible_alpha_bounds(frame: &super::CaptionPlaneFrame) -> Option<(u32, u32, u32, u32)> {
    let bytes = BASE64
        .decode(frame.png_base64().expect("encoded PNG"))
        .expect("PNG base64");
    let (pixels, width, height) = super::decode_png(&bytes).expect("decoded PNG");
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for pixel_y in 0..height {
        for pixel_x in 0..width {
            let alpha = pixels[(pixel_y as usize * width as usize + pixel_x as usize) * 4 + 3];
            if alpha == 0 {
                continue;
            }
            bounds = Some(match bounds {
                Some((left, top, right, bottom)) => (
                    left.min(pixel_x),
                    top.min(pixel_y),
                    right.max(pixel_x),
                    bottom.max(pixel_y),
                ),
                None => (pixel_x, pixel_y, pixel_x, pixel_y),
            });
        }
    }
    bounds
}

const ONBOARDING_ASSET_NAMES: [&str; 5] = [
    "00-positioning.png",
    "01-white-copy.png",
    "02-yellow-caption.png",
    "03-ruby.png",
    "04-drcs.png",
];

const ONBOARDING_MOTION_ASSET_NAMES: [&str; 3] =
    ["00-main-cell.png", "00-ruby-cell.png", "05-drcs.png"];

const ONBOARDING_TITLE_ELEMENT_ASSETS: [(&str, usize, usize, usize, usize); 7] = [
    ("06-title-subtitle.png", 495, 690, 741, 842),
    ("07-title-wo.png", 741, 690, 851, 842),
    ("08-title-comma.png", 851, 760, 919, 842),
    ("09-title-shape-ruby.png", 919, 635, 1128, 842),
    ("10-title-ni.png", 1128, 705, 1204, 842),
    ("11-title-suru.png", 1204, 690, 1441, 842),
    ("12-title-drcs.png", 1441, 690, 1550, 842),
];

const ONBOARDING_TITLE_LIGHT_ASSET_NAMES: [&str; 7] = [
    "06-title-subtitle-light.png",
    "07-title-wo-light.png",
    "08-title-comma-light.png",
    "09-title-shape-ruby-light.png",
    "10-title-ni-light.png",
    "11-title-suru-light.png",
    "12-title-drcs-light.png",
];

const ONBOARDING_UNIFIED_TITLE_LIGHT_ASSET_NAME: &str = "13-title-unified-light.png";

fn onboarding_asset_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../public/onboarding")
}

fn onboarding_pixel(frame: &[u8], x: usize, y: usize) -> &[u8] {
    let index = (y * 1920 + x) * 4;
    &frame[index..index + 4]
}

fn onboarding_interval(rich_body: &str) -> serde_json::Value {
    json!({
        "text": "字幕を、形にする",
        "x": 400,
        "y": 620,
        "width": 1150,
        "height": 230,
        "rich_body": rich_body,
        "style": {
            "font_family": "丸ゴシック",
            "font_size": "112px",
            "line_height": "132px",
            "letter_spacing": "5px",
            "writing_mode": "horizontal-tb",
            "text_align": "center",
            "display_align": "center",
            "color": "#FFFFFFFF",
            "background_color": "#00000000",
            "text_outline": "4px #000000"
        }
    })
}

fn onboarding_frame(rich_body: &str) -> super::CaptionPlaneFrame {
    compose(&[onboarding_interval(rich_body)]).expect("onboarding caption frame")
}

fn onboarding_main_background() -> Vec<u8> {
    let mut pixels = vec![0_u8; 1920 * 1080 * 4];
    super::fill_rect(&mut pixels, 495, 700, 1055, 130, [0, 0, 0, 153]);
    pixels
}

fn onboarding_ruby_background() -> Vec<u8> {
    let mut pixels = vec![0_u8; 1920 * 1080 * 4];
    super::fill_rect(&mut pixels, 905, 655, 245, 45, [0, 0, 0, 153]);
    pixels
}

fn onboarding_background() -> Vec<u8> {
    let mut pixels = onboarding_main_background();
    let ruby = onboarding_ruby_background();
    super::blend_layer(&mut pixels, 1920, 1080, 0, 0, 1920, 1080, &ruby);
    pixels
}

fn add_synthetic_file_drcs(mut pixels: Vec<u8>) -> Vec<u8> {
    const DRCS: [&str; 12] = [
        "111111110000",
        "100000011000",
        "100000010100",
        "100000011111",
        "100000000001",
        "100111110001",
        "100100010001",
        "100101010001",
        "100100010001",
        "100111110001",
        "100000000001",
        "111111111111",
    ];
    let scale = 6_i32;
    let origin_x = 1454_i32;
    let origin_y = 728_i32;
    for (row, source) in DRCS.iter().enumerate() {
        for (column, value) in source.bytes().enumerate() {
            if value != b'1' {
                continue;
            }
            super::fill_rect(
                &mut pixels,
                origin_x + column as i32 * scale,
                origin_y + row as i32 * scale,
                scale - 1,
                scale - 1,
                [255, 255, 255, 255],
            );
        }
    }
    pixels
}

fn onboarding_title_element(
    source: &[u8],
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) -> Vec<u8> {
    let width = right - left;
    let height = bottom - top;
    let mut element = vec![0_u8; width * height * 4];
    for (target_y, source_y) in (top..bottom).enumerate() {
        let source_start = (source_y * 1920 + left) * 4;
        let source_end = (source_y * 1920 + right) * 4;
        let target_start = target_y * width * 4;
        element[target_start..target_start + width * 4]
            .copy_from_slice(&source[source_start..source_end]);
    }
    element
}

fn onboarding_title_light_mask(element: &[u8]) -> Vec<u8> {
    element
        .chunks_exact(4)
        .flat_map(|pixel| {
            let luminance = pixel[0].max(pixel[1]).max(pixel[2]);
            let light_alpha = if luminance <= 64 {
                0
            } else {
                ((u16::from(pixel[3]) * u16::from(luminance - 64)) / 191) as u8
            };
            [255, 255, 255, light_alpha]
        })
        .collect()
}

#[test]
#[ignore = "explicitly regenerates checked-in onboarding overlay assets"]
fn generate_onboarding_overlay_assets() {
    assert_eq!(
        std::env::var("RESUBWINNY_UPDATE_ONBOARDING_ASSETS").as_deref(),
        Ok("1")
    );
    let directory = onboarding_asset_directory();
    std::fs::create_dir_all(&directory).expect("onboarding asset directory");
    let main_background = onboarding_main_background();
    let ruby_background = onboarding_ruby_background();
    let blank = onboarding_background();
    let white = onboarding_frame("字幕を、形にする");
    let yellow = onboarding_frame("<span tts:color='#FFFF00FF'>字幕</span>を、形にする");
    let ruby = onboarding_frame(
        "<span tts:color='#FFFF00FF'>字幕</span>を、<ruby><span tts:ruby='base'>形</span><rt><span tts:ruby='text' tts:fontSize='44px'>ファイル</span></rt></ruby>にする",
    );
    let white_pixels = white.pixels.to_vec();
    let yellow_pixels = yellow.pixels.to_vec();
    let ruby_pixels = ruby.pixels.to_vec();
    let final_pixels = add_synthetic_file_drcs(ruby_pixels.clone());
    let frames: [&[u8]; 5] = [
        &blank,
        &white_pixels,
        &yellow_pixels,
        &ruby_pixels,
        &final_pixels,
    ];
    for (name, pixels) in ONBOARDING_ASSET_NAMES.iter().zip(frames) {
        let bytes = super::encode_png(1920, 1080, pixels).expect("encoded onboarding PNG");
        std::fs::write(directory.join(name), bytes).expect("written onboarding PNG");
    }
    let drcs_pixels = add_synthetic_file_drcs(vec![0_u8; 1920 * 1080 * 4]);
    for (name, pixels) in ONBOARDING_MOTION_ASSET_NAMES.iter().zip([
        main_background.as_slice(),
        ruby_background.as_slice(),
        drcs_pixels.as_slice(),
    ]) {
        let bytes = super::encode_png(1920, 1080, pixels).expect("encoded onboarding motion PNG");
        std::fs::write(directory.join(name), bytes).expect("written onboarding motion PNG");
    }
    for ((name, left, top, right, bottom), light_name) in ONBOARDING_TITLE_ELEMENT_ASSETS
        .into_iter()
        .zip(ONBOARDING_TITLE_LIGHT_ASSET_NAMES)
    {
        let pixels = onboarding_title_element(&final_pixels, left, top, right, bottom);
        let bytes = super::encode_png((right - left) as u32, (bottom - top) as u32, &pixels)
            .expect("encoded onboarding title element PNG");
        std::fs::write(directory.join(name), bytes).expect("written onboarding title element PNG");
        let light_mask = onboarding_title_light_mask(&pixels);
        let bytes = super::encode_png((right - left) as u32, (bottom - top) as u32, &light_mask)
            .expect("encoded onboarding title light mask PNG");
        std::fs::write(directory.join(light_name), bytes)
            .expect("written onboarding title light mask PNG");
    }
    let unified_light_mask = onboarding_title_light_mask(&final_pixels);
    let bytes = super::encode_png(1920, 1080, &unified_light_mask)
        .expect("encoded unified onboarding title light mask PNG");
    std::fs::write(
        directory.join(ONBOARDING_UNIFIED_TITLE_LIGHT_ASSET_NAME),
        bytes,
    )
    .expect("written unified onboarding title light mask PNG");
}

#[test]
fn checked_in_onboarding_overlay_assets_are_transparent_native_planes() {
    let directory = onboarding_asset_directory();
    let mut decoded = Vec::new();
    for name in ONBOARDING_ASSET_NAMES {
        let bytes = std::fs::read(directory.join(name)).expect("checked-in onboarding asset");
        let (pixels, width, height) = super::decode_png(&bytes).expect("valid onboarding PNG");
        assert_eq!((width, height), (1920, 1080));
        assert_eq!(
            &pixels[0..4],
            &[0, 0, 0, 0],
            "outside the caption region remains transparent"
        );
        decoded.push(pixels);
    }
    for name in ONBOARDING_MOTION_ASSET_NAMES {
        let bytes =
            std::fs::read(directory.join(name)).expect("checked-in onboarding motion asset");
        let (_, width, height) = super::decode_png(&bytes).expect("valid onboarding motion PNG");
        assert_eq!((width, height), (1920, 1080));
    }
    for ((name, left, top, right, bottom), light_name) in ONBOARDING_TITLE_ELEMENT_ASSETS
        .into_iter()
        .zip(ONBOARDING_TITLE_LIGHT_ASSET_NAMES)
    {
        let bytes =
            std::fs::read(directory.join(name)).expect("checked-in onboarding title element");
        let (pixels, width, height) =
            super::decode_png(&bytes).expect("valid onboarding title element PNG");
        assert_eq!(
            (width, height),
            ((right - left) as u32, (bottom - top) as u32)
        );
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "{name} contains visible ink"
        );
        assert!(
            left < right && top < bottom,
            "{name} has a non-empty local element boundary"
        );
        let bytes = std::fs::read(directory.join(light_name))
            .expect("checked-in onboarding title light mask");
        let (light_pixels, light_width, light_height) =
            super::decode_png(&bytes).expect("valid onboarding title light mask PNG");
        assert_eq!((light_width, light_height), (width, height));
        assert!(
            light_pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "{light_name} contains a luminous stroke mask"
        );
        assert!(
            light_pixels
                .chunks_exact(4)
                .all(|pixel| pixel[3] == 0 || pixel[0..3] == [255, 255, 255]),
            "{light_name} is a neutral alpha mask"
        );
        if light_name == "09-title-shape-ruby-light.png" {
            let ruby_band_end = width as usize * 80 * 4;
            assert!(
                light_pixels[..ruby_band_end]
                    .chunks_exact(4)
                    .any(|pixel| pixel[3] > 0),
                "ファイル participates in the luminous stroke mask"
            );
            assert!(
                light_pixels[ruby_band_end..]
                    .chunks_exact(4)
                    .any(|pixel| pixel[3] > 0),
                "形 remains in the same luminous element"
            );
        }
    }
    let bytes = std::fs::read(directory.join(ONBOARDING_UNIFIED_TITLE_LIGHT_ASSET_NAME))
        .expect("checked-in unified onboarding title light mask");
    let (unified_light, width, height) =
        super::decode_png(&bytes).expect("valid unified onboarding title light mask PNG");
    assert_eq!((width, height), (1920, 1080));
    assert!(
        (650..705).any(|y| (940..1128).any(|x| onboarding_pixel(&unified_light, x, y)[3] > 0)),
        "unified light includes ファイル"
    );
    assert!(
        (710..830).any(|y| (495..1440).any(|x| onboarding_pixel(&unified_light, x, y)[3] > 0)),
        "unified light includes the main title line"
    );
    assert!(
        (728..799).any(|y| (1454..1525).any(|x| onboarding_pixel(&unified_light, x, y)[3] > 0)),
        "unified light includes DRCS"
    );
    for (label, left, right) in [
        ("字", 495, 618),
        ("幕", 618, 741),
        ("を", 741, 851),
        ("、", 851, 919),
        ("形", 919, 1128),
        ("に", 1128, 1204),
        ("す", 1204, 1322),
        ("る", 1322, 1441),
    ] {
        assert!(
            (690..842)
                .any(|y| (left..right).any(|x| onboarding_pixel(&unified_light, x, y)[3] > 0)),
            "{label} has luminous source strokes"
        );
    }
    assert!(
        decoded[0]
            .chunks_exact(4)
            .any(|pixel| pixel == [0, 0, 0, 153])
    );
    assert!(
        decoded[0]
            .chunks_exact(4)
            .all(|pixel| pixel[3] == 0 || pixel[3] == 153),
        "background is one uniform-alpha union without stacked shadows"
    );
    assert_eq!(
        onboarding_pixel(&decoded[0], 500, 705),
        &[0, 0, 0, 153],
        "main-line cell background"
    );
    assert_eq!(
        onboarding_pixel(&decoded[0], 910, 660),
        &[0, 0, 0, 153],
        "raised ruby cell background"
    );
    assert_eq!(
        onboarding_pixel(&decoded[0], 800, 660),
        &[0, 0, 0, 0],
        "ruby background does not span the line"
    );
    assert_eq!(
        onboarding_pixel(&decoded[0], 500, 835),
        &[0, 0, 0, 0],
        "main-line background stays optically tight"
    );
    let mut main_bounds = (usize::MAX, usize::MAX, 0_usize, 0_usize);
    for y in 700..830 {
        for x in 495..1450 {
            if onboarding_pixel(&decoded[3], x, y)[3] <= 180 {
                continue;
            }
            main_bounds = (
                main_bounds.0.min(x),
                main_bounds.1.min(y),
                main_bounds.2.max(x),
                main_bounds.3.max(y),
            );
        }
    }
    let mut drcs_bounds = (usize::MAX, usize::MAX, 0_usize, 0_usize);
    for y in 700..830 {
        for x in 1400..1550 {
            if onboarding_pixel(&decoded[3], x, y) == onboarding_pixel(&decoded[4], x, y) {
                continue;
            }
            drcs_bounds = (
                drcs_bounds.0.min(x),
                drcs_bounds.1.min(y),
                drcs_bounds.2.max(x),
                drcs_bounds.3.max(y),
            );
        }
    }
    let main_center_y = (main_bounds.1 + main_bounds.3) as isize;
    let drcs_center_y = (drcs_bounds.1 + drcs_bounds.3) as isize;
    assert!(
        (main_center_y - drcs_center_y).abs() <= 1,
        "DRCS and main-line ink share an optical vertical centre"
    );
    let preceding_gap = drcs_bounds.0 - main_bounds.2 - 1;
    let trailing_gap = 1549 - drcs_bounds.2;
    assert!(
        (preceding_gap as isize - trailing_gap as isize).abs() <= 1,
        "DRCS is optically balanced in the final cell"
    );
    assert_ne!(
        decoded[1], decoded[2],
        "yellow caption state differs from white copy"
    );
    assert_ne!(decoded[2], decoded[3], "ruby adds a distinct native layer");
    assert_ne!(
        decoded[3], decoded[4],
        "synthetic DRCS adds a distinct bitmap cell"
    );
    assert!(
        decoded[2]
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 220 && pixel[1] > 210 && pixel[2] < 80 && pixel[3] > 0)
    );
}

#[test]
fn empty_caption_plane_returns_without_a_frame() {
    assert!(compose(&[]).is_none());
}

fn pixel_png(pixel: [u8; 4]) -> String {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(&pixel).expect("png pixel");
    drop(writer);
    BASE64.encode(bytes)
}

#[test]
fn two_axis_b62_font_size_uses_the_vertical_display_height() {
    assert_eq!(parse_font_height(Some("36px 72px")), Some(72.0));
    assert_eq!(parse_font_height(Some("72px")), Some(72.0));
}

#[test]
fn recognises_the_arib_rounded_caption_family_for_receiver_baseline() {
    assert!(is_arib_rounded_caption(
        &json!({"rich_body":"<span tts:fontFamily='丸ゴシック'>字幕</span>"}),
        &json!({"font_family":"丸ゴシック"}),
    ));
    assert!(!is_arib_rounded_caption(
        &json!({}),
        &json!({"font_family":"Unrelated Sans"}),
    ));
}

#[test]
fn composes_active_layers_with_positions() {
    let red = pixel_png([255, 0, 0, 255]);
    let blue = pixel_png([0, 0, 255, 255]);
    let frame = compose(&[
        json!({"rendered_image":{"png_base64":red,"dst_x":0,"dst_y":0}}),
        json!({"rendered_image":{"png_base64":blue,"dst_x":1,"dst_y":0}}),
    ])
    .expect("composed frame");
    assert_eq!((frame.width, frame.height, frame.layer_count), (2, 1, 2));
    assert!(!frame.png_base64().expect("encoded PNG").is_empty());
}

#[test]
fn renders_text_only_ttml_archives_natively_when_the_writing_mode_is_horizontal() {
    assert_eq!(
        compose(&[json!({"text":"TTML only"})])
            .expect("TTML frame")
            .mode,
        "ttml-horizontal-native"
    );
}

#[test]
fn renders_rtl_horizontal_runs_from_the_region_end() {
    let ltr = compose(&[json!({
        "text": "右左",
        "x": 400,
        "y": 300,
        "width": 300,
        "height": 120,
        "style": {"font_size": "96px", "writing_mode": "horizontal-tb", "direction": "ltr"}
    })])
    .expect("LTR frame");
    let rtl = compose(&[json!({
        "text": "右左",
        "x": 400,
        "y": 300,
        "width": 300,
        "height": 120,
        "style": {"font_size": "96px", "writing_mode": "horizontal-tb", "direction": "rtl"}
    })])
    .expect("RTL frame");
    assert_ne!(ltr.pixels, rtl.pixels);
}

#[test]
fn renders_horizontal_ttml_text_with_the_bundled_arib_font() {
    let frame = compose(&[json!({
            "text": "日本語字幕",
            "x": 120,
            "y": 860,
            "width": 600,
            "height": 120,
            "style": {"font_size": "72px", "color": "#FFFFFFFF", "background_color": "#00000080", "writing_mode": "horizontal-tb"}
        })]).expect("native TTML frame");
    assert_eq!(frame.mode, "ttml-horizontal-native");
    assert_eq!((frame.width, frame.height), (1920, 1080));
    assert_eq!(frame.layer_count, 1);
}

#[test]
fn applies_horizontal_text_and_display_alignment_in_native_layout() {
    let start = compose(&[json!({
            "text": "字幕",
            "x": 200,
            "y": 300,
            "width": 600,
            "height": 300,
            "style": {"font_size": "96px", "writing_mode": "horizontal-tb", "text_align": "start", "display_align": "before"}
        })])
        .expect("start-aligned frame");
    let centred_after = compose(&[json!({
            "text": "字幕",
            "x": 200,
            "y": 300,
            "width": 600,
            "height": 300,
            "style": {"font_size": "96px", "writing_mode": "horizontal-tb", "text_align": "center", "display_align": "after"}
        })])
        .expect("centred frame");
    let (start_x, start_y, _, _) = visible_alpha_bounds(&start).expect("start bounds");
    let (center_x, center_y, _, _) = visible_alpha_bounds(&centred_after).expect("centre bounds");
    assert!(center_x > start_x);
    assert!(center_y > start_y);
}

#[test]
fn lays_out_explicit_horizontal_line_breaks_without_discarding_them() {
    let frame = compose(&[json!({
        "text": "一行目\n二行目",
        "x": 200,
        "y": 300,
        "width": 600,
        "height": 300,
        "style": {"font_size": "72px", "line_height": "120px", "writing_mode": "horizontal-tb"}
    })])
    .expect("multiline frame");
    let (_, top, _, bottom) = visible_alpha_bounds(&frame).expect("multiline bounds");
    assert!(bottom - top > 100);
}

#[test]
fn renders_vertical_ttml_in_a_separate_basic_native_mode() {
    let frame = compose(&[json!({
        "text": "縦書き",
        "x": 960,
        "y": 120,
        "width": 120,
        "height": 600,
        "style": {"writing_mode": "vertical-rl"}
    })])
    .expect("vertical TTML frame");
    assert_eq!(frame.mode, "ttml-vertical-basic-native");
}

#[test]
fn maps_only_explicit_unicode_vertical_punctuation_forms() {
    assert_eq!(vertical_presentation_form('「'), Some('\u{FE41}'));
    assert_eq!(vertical_presentation_form('。'), Some('\u{FE12}'));
    assert_eq!(vertical_presentation_form('A'), None);
    assert_eq!(vertical_presentation_form('漢'), None);
}

#[test]
fn keeps_cjk_upright_and_rotates_only_conservative_latin_vertical_glyphs() {
    assert_eq!(
        vertical_glyph_orientation('漢'),
        VerticalGlyphOrientation::Upright
    );
    assert_eq!(
        vertical_glyph_orientation('Ａ'),
        VerticalGlyphOrientation::Upright
    );
    assert_eq!(
        vertical_glyph_orientation('A'),
        VerticalGlyphOrientation::RotateClockwise
    );
    assert_eq!(
        vertical_glyph_orientation('é'),
        VerticalGlyphOrientation::RotateClockwise
    );
    assert_eq!(
        rotate_bitmap_clockwise(&[1, 2, 3, 4, 5, 6], 2, 3),
        [5, 3, 1, 6, 4, 2]
    );
}

#[test]
fn renders_vertical_punctuation_with_the_native_b62_path() {
    let frame = compose(&[json!({
        "text": "「字幕」。",
        "x": 960,
        "y": 120,
        "width": 120,
        "height": 600,
        "style": {"font_size": "96px", "writing_mode": "vertical-rl"}
    })])
    .expect("vertical punctuation frame");
    assert_eq!(frame.mode, "ttml-vertical-basic-native");
    assert_eq!(frame.missing_glyph_count, 0);
}

#[test]
fn renders_explicit_two_digit_text_combine_in_one_vertical_cell() {
    let interval = json!({
        "rich_body": "<span tts:textCombine='all'>24</span>年"
    });
    let runs = styled_runs(
        &interval,
        "24年",
        parse_rgba("#FFFFFFFF"),
        96.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(runs.len(), 2);
    assert!(runs[0].text_combine);

    let frame = compose(&[json!({
        "text": "24年",
        "x": 960,
        "y": 120,
        "width": 120,
        "height": 300,
        "style": {"font_size": "96px", "writing_mode": "vertical-rl"},
        "rich_body": "<span tts:textCombine='all'>24</span>年"
    })])
    .expect("vertical text combine frame");
    assert_eq!(frame.mode, "ttml-vertical-basic-native");
    assert_eq!(frame.missing_glyph_count, 0);
    let (_, top, _, bottom) = visible_alpha_bounds(&frame).expect("text combine bounds");
    assert!(bottom - top < 220, "24 occupies one 96px vertical cell");
}

#[test]
fn accepts_only_one_or_two_ascii_digits_for_explicit_text_combine() {
    assert_eq!(text_combine_digit_count(&['2', '4', '年'], 0), Some(2));
    assert_eq!(text_combine_digit_count(&['7', '年'], 0), Some(1));
    assert_eq!(text_combine_digit_count(&['2', '0', '2', '6'], 0), None);
    assert_eq!(text_combine_digit_count(&['年'], 0), None);
}

#[test]
fn renders_unwrapped_vertical_ruby_beside_its_base_column() {
    let frame = compose(&[json!({
            "text": "漢字",
            "x": 960,
            "y": 120,
            "width": 180,
            "height": 600,
            "style": {"font_size": "96px", "writing_mode": "vertical-rl"},
            "rich_body": "<ruby><span tts:ruby='base'>漢字</span><rt><span tts:ruby='text'>かんじ</span></rt></ruby>"
        })])
        .expect("vertical ruby frame");
    assert_eq!(frame.mode, "ttml-vertical-ruby-basic-native");
    assert_eq!(frame.rendered_ruby_count, 1);
}

#[test]
fn continues_explicit_vertical_ruby_across_wrapped_columns() {
    let frame = compose(&[json!({
            "text": "漢字仮名",
            "x": 960,
            "y": 120,
            "width": 180,
            "height": 96,
            "style": {"font_size": "96px", "writing_mode": "vertical-rl"},
            "rich_body": "<ruby><span tts:ruby='base'>漢字仮名</span><rt><span tts:ruby='text'>かんじかな</span></rt></ruby>"
        })])
        .expect("wrapped vertical frame");
    assert_eq!(frame.mode, "ttml-vertical-ruby-basic-native");
    assert_eq!(frame.rendered_ruby_count, 1);
    let (left, top, right, bottom) = visible_alpha_bounds(&frame).expect("wrapped bounds");
    assert!(right - left >= 96, "base and ruby occupy adjacent columns");
    assert!(bottom - top >= 95, "base cells continue through the column");
}

#[test]
fn preserves_per_span_ttml_colour_and_size_without_rendering_ruby_text_inline() {
    let interval = json!({
        "rich_body": "<span tts:color='#FFFFFFFF' tts:fontSize='72px'>本</span><ruby><span tts:ruby='base'>漢</span><rt><span tts:ruby='text'>かん</span></rt></ruby><span tts:color='#FFFF00FF' tts:letterSpacing='8px'>文</span>"
    });
    let runs = styled_runs(
        &interval,
        "fallback",
        parse_rgba("#112233FF"),
        42.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        ["本", "漢", "文"]
    );
    assert_eq!(runs[0].font_size, 72.0);
    assert_eq!(runs[1].ruby_text.as_deref(), Some("かん"));
    assert_eq!(runs[2].color, parse_rgba("#FFFF00FF"));
    assert_eq!(runs[2].letter_spacing, 8.0);
}

#[test]
fn keeps_nested_span_closing_tags_out_of_native_text_runs() {
    let interval = json!({
        "rich_body": "<span tts:fontSize='72px'>外<span tts:color='#FFFF00FF'>内</span>側</span>"
    });
    let runs = styled_runs(
        &interval,
        "fallback",
        parse_rgba("#FFFFFFFF"),
        42.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        ["外", "内", "側"]
    );
    assert!(runs.iter().all(|run| run.font_size == 72.0));
    assert!(runs.iter().all(|run| !run.text.contains("</span>")));
}

#[test]
fn applies_supported_inner_span_style_without_losing_outer_style() {
    let interval = json!({
        "rich_body": "<span tts:fontSize='72px' tts:color='#FFFFFFFF'>外<span tts:color='#FFFF00FF'>内</span>側</span>"
    });
    let runs = styled_runs(
        &interval,
        "fallback",
        parse_rgba("#112233FF"),
        42.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        ["外", "内", "側"]
    );
    assert!(runs.iter().all(|run| run.font_size == 72.0));
    assert_eq!(runs[0].color, parse_rgba("#FFFFFFFF"));
    assert_eq!(runs[1].color, parse_rgba("#FFFF00FF"));
    assert_eq!(runs[2].color, parse_rgba("#FFFFFFFF"));
}

#[test]
fn renders_horizontal_ruby_above_its_base_span() {
    let frame = compose(&[json!({
            "text": "漢",
            "x": 800,
            "y": 800,
            "width": 160,
            "height": 160,
            "style": {"font_size": "96px", "writing_mode": "horizontal-tb"},
            "rich_body": "<ruby><span tts:ruby='base'>漢</span><rt><span tts:ruby='text'>かん</span></rt></ruby>"
        })]).expect("ruby frame");
    assert_eq!(frame.mode, "ttml-horizontal-ruby-basic-native");
    assert_eq!(frame.rendered_ruby_count, 1);
}

#[test]
fn preserves_explicit_ruby_annotation_style_without_changing_base_metrics() {
    let interval = json!({
        "rich_body": "<ruby><span tts:ruby='base' tts:color='#FFFFFFFF' tts:fontSize='96px'>漢</span><rt><span tts:ruby='text' tts:color='#FFFF00FF' tts:fontSize='32px' tts:letterSpacing='4px' tts:textOutline='2px #000000'>かん</span></rt></ruby>"
    });
    let runs = styled_runs(
        &interval,
        "漢",
        parse_rgba("#FFFFFFFF"),
        96.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font_size, 96.0);
    let annotation = runs[0].ruby_style.expect("ruby annotation style");
    assert_eq!(annotation.color, parse_rgba("#FFFF00FF"));
    assert_eq!(annotation.font_size, 32.0);
    assert_eq!(annotation.letter_spacing, 4.0);
    assert_eq!(annotation.outline.expect("ruby outline").radius, 2);

    let font = Font::from_bytes(
        include_bytes!("../../../../third_party/rounded-mplus-1m-arib/rounded-mplus-1m-arib.ttf")
            as &[u8],
        FontSettings::default(),
    )
    .expect("bundled ARIB font");
    assert_eq!(horizontal_lines(&font, &runs)[0].ruby_height, 32.0);

    let styled = compose(&[json!({
        "text": "漢",
        "x": 800,
        "y": 800,
        "width": 180,
        "height": 160,
        "style": {"font_size": "96px", "writing_mode": "horizontal-tb"},
        "rich_body": interval["rich_body"]
    })])
    .expect("styled ruby frame");
    let plain = compose(&[json!({
            "text": "漢",
            "x": 800,
            "y": 800,
            "width": 180,
            "height": 160,
            "style": {"font_size": "96px", "writing_mode": "horizontal-tb"},
            "rich_body": "<ruby><span tts:ruby='base'>漢</span><rt><span tts:ruby='text'>かん</span></rt></ruby>"
        })])
        .expect("plain ruby frame");
    assert_ne!(styled.pixels, plain.pixels);
}

#[test]
fn horizontal_ruby_reserves_a_separate_band_above_its_base_line() {
    let font = Font::from_bytes(
        include_bytes!("../../../../third_party/rounded-mplus-1m-arib/rounded-mplus-1m-arib.ttf")
            as &[u8],
        FontSettings::default(),
    )
    .expect("bundled ARIB font");
    let runs = vec![StyledRun {
        text: "漢".into(),
        id: None,
        ruby_target_id: None,
        color: [255, 255, 255, 255],
        font_size: 80.0,
        letter_spacing: 0.0,
        outline: None,
        ruby_text: Some("かん".into()),
        ruby_style: None,
        ruby_base: false,
        ruby_group_base_count: 1,
        text_combine: false,
    }];
    let line = horizontal_lines(&font, &runs)[0];
    assert_eq!(line.ruby_height, 40.0);
    assert_eq!(line.glyph_height, 80.0);
    assert_eq!(80.0_f32.max(line.glyph_height + line.ruby_height), 120.0);
}

#[test]
fn resolves_arib_ruby_target_ids_without_rendering_the_annotation_inline() {
    let interval = json!({
        "rich_body": "<span xml:id='kanji'>漢字</span><span arib-tt:ruby='kanji'>かんじ</span>"
    });
    let runs = styled_runs(
        &interval,
        "fallback",
        parse_rgba("#FFFFFFFF"),
        42.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "漢字");
    assert_eq!(runs[0].ruby_text.as_deref(), Some("かんじ"));

    let frame = compose(&[json!({
        "text": "漢字",
        "x": 800,
        "y": 800,
        "width": 240,
        "height": 160,
        "style": {"font_size": "96px", "writing_mode": "horizontal-tb"},
        "rich_body": "<span xml:id='kanji'>漢字</span><span arib-tt:ruby='kanji'>かんじ</span>"
    })])
    .expect("ARIB ruby frame");
    assert_eq!(frame.mode, "ttml-horizontal-ruby-basic-native");
    assert_eq!(frame.rendered_ruby_count, 1);
}

#[test]
fn retains_explicit_arib_ruby_annotation_style_on_its_target() {
    let interval = json!({
        "rich_body": "<span xml:id='kanji' tts:fontSize='96px'>漢字</span><span arib-tt:ruby='kanji' tts:color='#00FFFFFF' tts:fontSize='28px' tts:textOutline='2px #000000'>かんじ</span>"
    });
    let runs = styled_runs(
        &interval,
        "fallback",
        parse_rgba("#FFFFFFFF"),
        42.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(runs.len(), 1);
    let annotation = runs[0].ruby_style.expect("ARIB ruby annotation style");
    assert_eq!(annotation.color, parse_rgba("#00FFFFFF"));
    assert_eq!(annotation.font_size, 28.0);
    assert_eq!(annotation.outline.expect("ARIB ruby outline").radius, 2);
}

#[test]
fn preserves_text_outside_spans_and_attaches_ruby_to_the_preceding_base() {
    let interval = json!({
        "rich_body": "字幕 <ruby><span tts:ruby='base'>漢</span><rt><span tts:ruby='text'>かん</span></rt></ruby>"
    });
    let runs = styled_runs(
        &interval,
        "fallback",
        parse_rgba("#FFFFFFFF"),
        42.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        ["字幕 ", "漢"]
    );
    assert_eq!(runs[1].ruby_text.as_deref(), Some("かん"));
}

#[test]
fn groups_contiguous_ruby_base_spans_before_rendering_one_annotation() {
    let interval = json!({
        "rich_body": "<ruby><span tts:ruby='base' tts:color='#FFFFFFFF'>漢</span><span tts:ruby='base' tts:color='#00FFFFFF'>字</span><rt><span tts:ruby='text'>かんじ</span></rt></ruby>"
    });
    let runs = styled_runs(
        &interval,
        "fallback",
        parse_rgba("#FFFFFFFF"),
        96.0,
        0.0,
        None,
        1.0,
    );
    assert_eq!(runs.len(), 2);
    assert!(runs[0].ruby_text.is_none());
    assert_eq!(runs[1].ruby_text.as_deref(), Some("かんじ"));
    assert_eq!(runs[1].ruby_group_base_count, 2);

    let frame = compose(&[json!({
        "text": "漢字",
        "x": 800,
        "y": 800,
        "width": 260,
        "height": 180,
        "style": {"font_size": "96px", "writing_mode": "horizontal-tb"},
        "rich_body": interval["rich_body"]
    })])
    .expect("grouped ruby frame");
    assert_eq!(frame.mode, "ttml-horizontal-ruby-basic-native");
    assert_eq!(frame.rendered_ruby_count, 1);
}

#[test]
fn parses_direct_ttml_opacity_and_outline_conservatively() {
    assert_eq!(
        apply_opacity(parse_rgba("#FFFFFFFF"), parse_opacity(Some("50%")))[3],
        128
    );
    let outline = parse_text_outline(Some("2px #000000"), 0.5).expect("direct outline");
    assert_eq!(outline.radius, 2);
    assert_eq!(outline.color, [0, 0, 0, 128]);
    let named_outline = parse_text_outline(Some("yellow 3px"), 1.0).expect("named outline");
    assert_eq!(named_outline.radius, 3);
    assert_eq!(named_outline.color, [255, 255, 0, 255]);
    assert!(parse_text_outline(Some("none"), 1.0).is_none());
    assert!(parse_text_outline(Some("thin black"), 1.0).is_none());
    assert!(parse_text_outline(Some("2px #nothex"), 1.0).is_none());
}

#[test]
fn applies_a_supported_direct_ttml_outline_to_the_native_rgba_plane() {
    let plain = compose(&[json!({
        "text": "字幕",
        "x": 800,
        "y": 600,
        "width": 260,
        "height": 140,
        "style": {"font_size": "96px", "writing_mode": "horizontal-tb", "color": "#FFFFFFFF"}
    })])
    .expect("plain frame");
    let outlined = compose(&[json!({
            "text": "字幕",
            "x": 800,
            "y": 600,
            "width": 260,
            "height": 140,
            "style": {"font_size": "96px", "writing_mode": "horizontal-tb", "color": "#FFFFFFFF", "text_outline": "3px #000000"}
        })])
        .expect("outlined frame");
    assert_ne!(plain.pixels, outlined.pixels);
    let (plain_left, plain_top, plain_right, plain_bottom) =
        visible_alpha_bounds(&plain).expect("plain bounds");
    let (outline_left, outline_top, outline_right, outline_bottom) =
        visible_alpha_bounds(&outlined).expect("outline bounds");
    assert!(outline_left <= plain_left && outline_top <= plain_top);
    assert!(outline_right >= plain_right && outline_bottom >= plain_bottom);
}

#[test]
fn parses_standard_ttml_named_colours_without_a_browser_fallback() {
    assert_eq!(parse_rgba("yellow"), [255, 255, 0, 255]);
    assert_eq!(parse_rgba("WHITE"), [255, 255, 255, 255]);
    assert_eq!(parse_rgba("transparent"), [0, 0, 0, 0]);
    assert_eq!(parse_rgba("not-a-colour"), [255, 255, 255, 255]);
}
