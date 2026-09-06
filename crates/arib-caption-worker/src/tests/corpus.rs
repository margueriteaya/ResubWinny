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

#[cfg(feature = "libaribtlv")]
#[test]
fn decodes_native_b62_fixture_when_enabled() {
    if std::env::var("ARIB_LONG_FIXTURE").as_deref() != Ok("1") {
        return;
    }
    let Some(path) = local_fixture_path("8k1.mmts") else {
        return;
    };
    if !path.is_file() {
        return;
    }

    let mut captions = Vec::new();
    let mut payloads = Vec::new();
    let summary = scan_tlv_ttml(
        &path,
        Some(0xf130),
        |caption| {
            captions.push(caption);
            Ok(())
        },
        |_| {},
        || false,
        |_, payload| {
            payloads.push((
                payload.packet_id,
                payload.mpu_sequence_number,
                payload.resources.len(),
            ));
            Ok(())
        },
        |_| Ok(()),
    )
    .expect("decode native B62 fixture");

    assert_eq!(summary.bytes_read, 364_994_560);
    assert_eq!(summary.pes_packets, 5);
    assert_eq!(summary.captions, 8);
    assert_eq!(summary.characters, 97);
    assert_eq!(summary.decoder_errors, 10);
    assert_eq!(payloads.len(), 5);
    assert!(
        payloads
            .iter()
            .all(|(packet_id, _, _)| *packet_id == 0xf130)
    );
    assert_eq!(
        payloads
            .iter()
            .filter_map(|(_, sequence, _)| *sequence)
            .collect::<Vec<_>>(),
        [322_960, 322_961, 322_962, 322_963, 322_964]
    );
    assert!(payloads.iter().all(|(_, _, resources)| *resources == 0));
    assert!(captions.iter().all(|caption| {
        caption.source.as_ref().is_some_and(|source| {
            source.route == "isdb_s3_tlv_libaribtlv_b62"
                && source.mmpt_packet_id == 0xf130
                && source.resources_complete
        })
    }));

    let alternate_callbacks = std::cell::Cell::new(0_u64);
    let alternate = scan_tlv_ttml(
        &path,
        Some(0xf138),
        |_| {
            alternate_callbacks.set(alternate_callbacks.get() + 1);
            Ok(())
        },
        |_| {},
        || false,
        |_, _| {
            alternate_callbacks.set(alternate_callbacks.get() + 1);
            Ok(())
        },
        |_| Ok(()),
    )
    .expect_err("alternate asset must not reuse captions from packet 0xF130");
    assert_eq!(alternate_callbacks.get(), 0);
    assert!(
        alternate
            .to_string()
            .contains("no complete normalized XML TTML captions")
    );
}
