use super::*;

fn local_fixture_path(name: &str) -> Option<PathBuf> {
    env::var_os("ARIB_FIXTURE_DIR").map(|directory| PathBuf::from(directory).join(name))
}

#[test]
fn decodes_terrestrial_fixture_when_enabled() {
    if std::env::var("ARIB_LONG_FIXTURE").as_deref() != Ok("1") {
        return;
    }
    let path = local_fixture_path("chijo_digital_test.ts")
        .expect("ARIB_FIXTURE_DIR must point to the local fixture directory");
    let track = discover_b24(&path)
        .expect("probe fixture")
        .expect("B24 track");
    let inspection = inspect_input(&path).expect("inspect terrestrial recording");
    assert!(inspection.broadcast.network_name.is_some());
    assert!(inspection.broadcast.programme_name.is_some());
    assert!(inspection.broadcast.broadcast_time_utc.is_some());
    let summary = decode_b24(&path, &track).expect("decode fixture");
    assert_eq!(summary.bytes_read, 18_579_078_944);
    assert_eq!(summary.pes_packets, 13_653);
    assert_eq!(summary.captions, 2_230);
    assert_eq!(summary.regions, 2_736);
    assert_eq!(summary.characters, 29_892);
    assert_eq!(summary.drcs_glyphs, 61);
    assert_eq!(summary.decoder_errors, 0);
}
#[test]
fn decodes_bs4k_fixture_when_enabled() {
    if std::env::var("ARIB_LONG_FIXTURE").as_deref() != Ok("1") {
        return;
    }
    let path = local_fixture_path("bs4k_test.m2ts")
        .expect("ARIB_FIXTURE_DIR must point to the local fixture directory");
    let tracks = discover_m2ts_data_tracks(&path)
        .expect("probe fixture")
        .expect("M2TS data tracks");
    let summary = scan_m2ts_ttml(
        &path,
        &tracks,
        |_| Ok(()),
        |_| {},
        || false,
        |_, _, _| Ok(()),
    )
    .expect("decode fixture");
    assert_eq!(summary.bytes_read, 11_517_020_160);
    assert_eq!(summary.pes_packets, 330);
    assert_eq!(summary.captions, 422);
    assert_eq!(summary.characters, 5_051);
    assert_eq!(summary.decoder_errors, 0);
}

#[test]
fn decodes_bs4k_b24_recording_tracks_when_enabled() {
    if std::env::var("ARIB_LONG_FIXTURE").as_deref() != Ok("1") {
        return;
    }
    let path = local_fixture_path("bs4k_test_2.ts")
        .expect("ARIB_FIXTURE_DIR must point to the local fixture directory");
    let tracks = discover_b24_tracks(&path).expect("probe fixture");
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].service_id, 101);
    assert_eq!(tracks[0].service_name.as_deref(), Some("NHK　BSP4K"));
    assert_eq!(tracks[0].caption_pid, 304);
    assert_eq!(tracks[1].caption_pid, 312);

    let inspection = inspect_input(&path).expect("inspect B24 recording");
    assert_eq!(inspection.route_code, "mpeg_ts_b24_verified");
    assert_eq!(inspection.service, "NHK　BSP4K · service 101 · PMT 0x1000");
    assert_eq!(inspection.tracks.len(), 2);
    assert!(inspection.tracks[1].detail.contains("PID 0x0138"));
    assert!(inspection.broadcast.network_name.is_some());
    assert!(inspection.broadcast.programme_name.is_some());
    assert!(inspection.broadcast.broadcast_time_utc.is_some());

    let primary = decode_b24(&path, &tracks[0]).expect("decode primary B24 track");
    assert_eq!(primary.bytes_read, 3_089_047_552);
    assert_eq!(primary.pes_packets, 2_038);
    assert_eq!(primary.captions, 118);
    assert_eq!(primary.regions, 157);
    assert_eq!(primary.characters, 1_661);
    assert_eq!(primary.drcs_glyphs, 0);
    assert_eq!(primary.decoder_errors, 0);

    let inactive = decode_b24(&path, &tracks[1]).expect("decode inactive B24 track");
    assert_eq!(inactive.captions, 0);
    assert_eq!(inactive.characters, 0);
    assert_eq!(inactive.decoder_errors, 0);
}
