use super::*;
use std::env;
use std::sync::{Arc, atomic::Ordering};

fn emit_feature_events(
    summary: &B24DecodeSummary,
    seen: &mut CaptionFeatureSummary,
    complete: bool,
) {
    for event in feature_events(summary, seen, complete) {
        emit_json(&event);
    }
}

fn feature_events(
    summary: &B24DecodeSummary,
    seen: &mut CaptionFeatureSummary,
    complete: bool,
) -> Vec<serde_json::Value> {
    let logical_track = std::env::var("RESUBWINNY_LOGICAL_TRACK")
        .unwrap_or_else(|_| "logical-track:default".into());
    let mut events = Vec::new();
    let features = [
        ("ruby", summary.features.ruby),
        ("drcs", summary.features.drcs),
        ("position", summary.features.position),
        ("color", summary.features.color),
        ("gaiji", summary.features.gaiji),
        ("accessibility", summary.features.accessibility),
    ];
    for (feature, present) in features {
        let was_present = match feature {
            "ruby" => seen.ruby,
            "drcs" => seen.drcs,
            "position" => seen.position,
            "color" => seen.color,
            "gaiji" => seen.gaiji,
            _ => seen.accessibility,
        };
        if present && !was_present {
            events.push(serde_json::json!({
                "type": "feature_observed",
                "feature": feature,
                "logicalTrack": logical_track,
                "observedCount": summary.features.observed_counts.get(feature).copied().unwrap_or(1),
                "complete": false
            }));
        }
    }
    *seen = summary.features.clone();
    if complete {
        for (feature, _) in features {
            events.push(serde_json::json!({
                "type": "feature_summary",
                "feature": feature,
                "logicalTrack": logical_track,
                "state": summary.features.state(feature),
                "observedCount": summary.features.observed_counts.get(feature).copied().unwrap_or(0),
                "complete": true
            }));
        }
    }
    events
}

#[cfg(test)]
mod feature_event_tests {
    use super::*;

    #[test]
    fn feature_events_are_first_observation_and_eof_only() {
        let summary = B24DecodeSummary {
            features: CaptionFeatureSummary {
                ruby: true,
                observed_counts: [("ruby".into(), 3)].into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut seen = CaptionFeatureSummary::default();
        let first = feature_events(&summary, &mut seen, false);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0]["type"], "feature_observed");
        assert_eq!(first[0]["feature"], "ruby");
        assert_eq!(first[0]["observedCount"], 3);

        assert!(feature_events(&summary, &mut seen, false).is_empty());

        let final_events = feature_events(&summary, &mut seen, true);
        assert_eq!(final_events.len(), 6);
        assert!(final_events.iter().all(|event| event["type"] == "feature_summary"));
        assert_eq!(
            final_events.iter().filter(|event| event["feature"] == "ruby").count(),
            1
        );
    }
}

/// Makes the archive the requested primary artifact after conversion has
/// completed. The conversion pipeline deliberately writes its ordinary
/// caption output first; archive-only is a CLI publishing policy layered on
/// top, rather than a separate subtitle-semantic path.
pub(crate) fn publish_archive_only(
    output: &Path,
    report: &mut ConversionReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let archive = report
        .archive
        .take()
        .ok_or("--archive-only did not produce an archive artifact")?;
    if report.output != output {
        return Err("--archive-only output path does not match the conversion target".into());
    }
    // The ordinary conversion has already published its primary artifact.
    // Do not delete it before the archive can replace it: on Windows an
    // existing destination cannot be renamed over directly.
    publish_file(&archive, output, true)?;
    report.output = output.to_path_buf();
    report.ass = None;
    report.archive = Some(output.to_path_buf());
    Ok(())
}

pub(crate) fn artifact_events(report: &ConversionReport) -> Vec<serde_json::Value> {
    let mut seen = BTreeSet::new();
    let mut events = Vec::new();
    let mut push = |kind: &str, path: Option<&PathBuf>| {
        let Some(path) = path else { return };
        if seen.insert(path.clone()) {
            events.push(serde_json::json!({
                "type": "artifact-created",
                "kind": kind,
                "path": path,
                "status": "completed",
            }));
        }
    };
    let primary_kind = if report.archive.as_ref() == Some(&report.output) {
        "archive"
    } else {
        "captions"
    };
    push(primary_kind, report.ass.as_ref());
    push("font-directory", report.font_directory.as_ref());
    push("drcs-directory", report.drcs_directory.as_ref());
    push("drcs-report", report.drcs_report.as_ref());
    push("ttml", report.ttml.as_ref());
    push("archive", report.archive.as_ref());
    push("raw-evidence", report.raw.as_ref());
    push("srt", report.srt.as_ref());
    push("webvtt", report.webvtt.as_ref());
    events
}

fn emit_empty_caption_diagnostic(summary: &B24DecodeSummary) {
    if summary.captions != 0 {
        return;
    }
    emit_json(&serde_json::json!({
        "type": "diagnostic",
        "level": "warning",
        "code": "caption.no_decoded_statements",
        "message": "The declared ARIB text service produced no decoded caption statements. It may carry only superimpose or service-management data.",
        "parameters": {
            "bytesRead": summary.bytes_read,
            "pesPackets": summary.pes_packets,
            "decoderErrors": summary.decoder_errors,
        },
    }));
}

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        eprintln!(
            "usage: arib-caption-worker <inspect|broadcast-at|decode-b24|convert-b24|convert|dump-tlv|render-at> ..."
        );
        std::process::exit(2);
    };
    if command == "capabilities" {
        emit_json(&serde_json::json!({
            "type": "capabilities",
            "capabilities": serde_json::from_str::<serde_json::Value>(include_str!("../../../shared/format_capabilities.json"))?
        }));
        return Ok(());
    }
    if command != "inspect"
        && command != "broadcast-at"
        && command != "decode-b24"
        && command != "convert-b24"
        && command != "convert"
        && command != "dump-tlv"
        && command != "render-at"
    {
        eprintln!(
            "unsupported command; use inspect, broadcast-at, decode-b24, convert-b24, convert, dump-tlv, or render-at"
        );
        std::process::exit(2);
    }
    let Some(path) = args.next() else {
        eprintln!(
            "usage: arib-caption-worker <inspect|broadcast-at|decode-b24|convert-b24|convert|dump-tlv|render-at> <recording> [output] [--ttml] [--srt] [--webvtt] [--archive|--archive-only] [--raw] [--no-ass] [--drop-position] [--drop-color] [--drop-ruby] [--drop-drcs] [--drop-gaiji] [--drop-accessibility] [--overwrite]"
        );
        std::process::exit(2);
    };
    let path = Path::new(&path);
    emit_hello(&command.to_string_lossy());
    if command == "broadcast-at" {
        emit_stage("reading-broadcast-metadata");
        let Some(offset) = args.next() else {
            return Err("broadcast-at requires a byte offset".into());
        };
        let byte_offset = offset
            .to_string_lossy()
            .parse::<u64>()
            .map_err(|_| "invalid broadcast-at byte offset")?;
        let mut service_id = None;
        while let Some(flag) = args.next() {
            if flag != "--service-id" {
                return Err("broadcast-at accepts only --service-id after the offset".into());
            }
            let Some(value) = args.next() else {
                return Err("--service-id requires a decimal service identifier".into());
            };
            service_id = Some(
                value
                    .to_string_lossy()
                    .parse::<u16>()
                    .map_err(|_| "invalid broadcast-at service identifier")?,
            );
        }
        let probe = probe_path(path)?;
        let metadata = match probe.kind {
            InputKind::MpegTs => {
                discover_broadcast_metadata_at(path, 188, 0, service_id, byte_offset)?
            }
            InputKind::M2ts => {
                discover_broadcast_metadata_at(path, 192, 4, service_id, byte_offset)?
            }
            _ => return Err("broadcast-at requires MPEG-TS or M2TS packetisation".into()),
        };
        emit_json(&serde_json::json!({
            "type": "broadcast-metadata",
            "source_offset": byte_offset,
            "broadcast": metadata,
        }));
        return Ok(());
    }
    if command == "render-at" {
        emit_stage("rendering-preview");
        let Some(time) = args.next() else {
            eprintln!("render-at requires a time in milliseconds");
            std::process::exit(2);
        };
        if args.next().is_some() {
            eprintln!("render-at accepts exactly an archive path and time");
            std::process::exit(2);
        }
        let time_ms = time
            .to_string_lossy()
            .parse::<i64>()
            .map_err(|_| "invalid render-at time")?;
        let preview = render_archive_at(path, time_ms)?;
        emit_json(&serde_json::json!({
            "type": "rendered",
            "source": preview.source,
            "time_ms": preview.time_ms,
            "intervals": preview.intervals,
        }));
        return Ok(());
    }
    if command == "dump-tlv" {
        emit_stage("probing");
        let output = args
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.with_extension("caption.mmtp.jsonl"));
        let mut overwrite = false;
        for flag in args.by_ref() {
            match flag.to_string_lossy().as_ref() {
                "--overwrite" => overwrite = true,
                _ => {
                    eprintln!("dump-tlv accepts only --overwrite");
                    std::process::exit(2);
                }
            }
        }
        let summary = dump_tlv_stpp_raw(path, &output, overwrite)?;
        emit_stage("publishing-artifacts");
        emit_json(
            &serde_json::json!({ "type": "completed", "output": output, "summary": summary }),
        );
        return Ok(());
    }
    if command == "convert-b24" || command == "convert" {
        emit_stage("probing");
        let output = args
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.with_extension("ass"));
        let mut options = ConversionOptions::default();
        let mut archive_only = false;
        while let Some(flag) = args.next() {
            match flag.to_string_lossy().as_ref() {
                "--ttml" => options.ttml = true,
                "--archive" => options.archive = true,
                "--archive-only" => {
                    options.archive = true;
                    archive_only = true;
                }
                "--raw" => options.raw = true,
                "--drcs-report" => options.drcs_report = true,
                "--track-id" => {
                    let Some(value) = args.next() else {
                        eprintln!("--track-id requires a PID or asset identifier");
                        std::process::exit(2);
                    };
                    let text = value.to_string_lossy();
                    let parsed = text
                        .strip_prefix("0x")
                        .or_else(|| text.strip_prefix("0X"))
                        .and_then(|number| u16::from_str_radix(number, 16).ok())
                        .or_else(|| text.parse::<u16>().ok());
                    let Some(track_id) = parsed else {
                        eprintln!("invalid --track-id value: {text}");
                        std::process::exit(2);
                    };
                    options.track_id = Some(track_id);
                }
                "--drcs-map" => {
                    let Some(mapping_path) = args.next() else {
                        eprintln!("--drcs-map requires a JSON mapping file");
                        std::process::exit(2);
                    };
                    match load_drcs_mapping(Path::new(&mapping_path)) {
                        Ok(mapping) => {
                            options.drcs_mode = DrcsMode::UseUserMapping;
                            options.drcs_replacements = mapping;
                        }
                        Err(error) => {
                            eprintln!("could not load DRCS mapping: {error}");
                            std::process::exit(2);
                        }
                    }
                }
                "--webvtt" => options.webvtt = true,
                "--srt" => options.srt = true,
                "--no-ass" => options.keep_ass = false,
                "--drop-position" => options.preserve_position = false,
                "--drop-color" => options.preserve_color = false,
                "--drop-ruby" => options.preserve_ruby = false,
                "--drop-drcs" => options.preserve_drcs = false,
                "--drop-gaiji" => options.preserve_gaiji = false,
                "--drop-accessibility" => options.preserve_accessibility = false,
                "--overwrite" => options.overwrite = true,
                _ => {
                    eprintln!(
                        "unknown convert option; use --ttml, --srt, --webvtt, --archive, --archive-only, --raw, --no-ass, preservation flags, --drcs-report, --drcs-map, --track-id, or --overwrite"
                    );
                    std::process::exit(2);
                }
            }
        }
        if archive_only
            && (options.raw || options.ttml || options.webvtt || options.srt || !options.keep_ass)
        {
            return Err(
                "--archive-only cannot be combined with another output format or --no-ass".into(),
            );
        }
        let control = Arc::new(WorkerControl::default());
        spawn_control_listener(Arc::clone(&control));
        let progress_control = Arc::clone(&control);
        let cancel_control = Arc::clone(&control);
        let mut seen_features = CaptionFeatureSummary::default();
        emit_stage("decoding");
        let report = if command == "convert-b24" {
            convert_b24_with_options_and_cancel(
                path,
                &output,
                options,
                |summary| {
                    if progress_control.wait_if_paused() {
                        return;
                    }
                    emit_json(
                        &serde_json::json!({"type": "progress", "bytes_read": summary.bytes_read, "captions": summary.captions, "warnings": summary.decoder_errors}),
                    );
                    emit_feature_events(summary, &mut seen_features, false);
                },
                move || cancel_control.wait_if_paused(),
            )
        } else {
            convert_with_options_and_cancel(
                path,
                &output,
                options,
                |summary| {
                    if progress_control.wait_if_paused() {
                        return;
                    }
                    emit_json(
                        &serde_json::json!({"type": "progress", "bytes_read": summary.bytes_read, "captions": summary.captions, "warnings": summary.decoder_errors}),
                    );
                    emit_feature_events(summary, &mut seen_features, false);
                },
                move || cancel_control.wait_if_paused(),
            )
        };
        let mut report = match report {
            Ok(report) => report,
            Err(error) if control.cancelled.load(Ordering::Relaxed) => {
                emit_json(&serde_json::json!({"type": "cancelled", "reason": error.to_string()}));
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        emit_feature_events(&report.summary, &mut seen_features, true);
        if control.cancelled.load(Ordering::Relaxed) {
            emit_json(&serde_json::json!({"type": "cancelled"}));
            return Ok(());
        }
        if archive_only {
            publish_archive_only(&output, &mut report)?;
        }
        emit_empty_caption_diagnostic(&report.summary);
        if report.summary.drcs_glyphs > 0 {
            emit_json(&serde_json::json!({
                "type": "drcs-discovered",
                "count": report.summary.drcs_glyphs,
                "directory": report.drcs_directory,
                "report": report.drcs_report,
            }));
        }
        emit_stage("publishing-artifacts");
        for artifact in artifact_events(&report) {
            emit_json(&artifact);
        }
        emit_stage("completed");
        emit_json(&serde_json::json!({
            "type": "completed", "output": report.output, "ass": report.ass, "font_directory": report.font_directory, "drcs_directory": report.drcs_directory, "drcs_report": report.drcs_report,
            "ttml": report.ttml, "archive": report.archive, "raw": report.raw, "srt": report.srt, "webvtt": report.webvtt,
            "summary": report.summary,
        }));
        return Ok(());
    }
    if args.next().is_some() {
        eprintln!("command accepts exactly one recording path");
        std::process::exit(2);
    }
    emit_stage("probing");
    let probe = probe_path(path)?;
    emit_stage("discovering-tracks");
    let b24_track = if probe.kind == InputKind::MpegTs {
        discover_b24(path)?
    } else {
        None
    };
    let b24_tracks = if probe.kind == InputKind::MpegTs {
        discover_b24_tracks(path)?
    } else {
        Vec::new()
    };
    let m2ts_data_tracks = if probe.kind == InputKind::M2ts {
        discover_m2ts_data_tracks(path)?
    } else {
        None
    };
    let mut mpeg_ts_data_tracks = if probe.kind == InputKind::MpegTs {
        discover_mpeg_ts_data_tracks(path)?
    } else {
        None
    };
    if let Some(tracks) = &mut mpeg_ts_data_tracks {
        tracks
            .pids
            .retain(|pid| !b24_tracks.iter().any(|track| track.caption_pid == *pid));
        if tracks.pids.is_empty() {
            mpeg_ts_data_tracks = None;
        }
    }
    if command == "inspect" {
        let inspection = inspect_input(path)?;
        for track in &b24_tracks {
            emit_json(&serde_json::json!({
                "type": "track-discovered",
                "route": "mpeg_ts_b24_verified",
                "track": track,
            }));
        }
        if let Some(tracks) = &mpeg_ts_data_tracks {
            for pid in &tracks.pids {
                emit_json(&serde_json::json!({
                    "type": "track-discovered",
                    "route": "mpeg_ts_ttml_candidate",
                    "trackId": pid,
                    "componentKind": tracks.component_kind(*pid),
                }));
            }
        }
        if let Some(tracks) = &m2ts_data_tracks {
            for pid in &tracks.pids {
                emit_json(&serde_json::json!({
                    "type": "track-discovered",
                    "route": "mpeg_ts_ttml_candidate",
                    "trackId": pid,
                    "componentKind": tracks.component_kind(*pid),
                }));
            }
        }
        emit_json(&serde_json::json!({
            "type": "input_probe",
            "path": path,
            "probe": probe,
            "inspection": inspection,
            "b24_track": b24_track,
            "b24_tracks": b24_tracks,
            "mpeg_ts_data_tracks": mpeg_ts_data_tracks,
            "m2ts_data_tracks": m2ts_data_tracks,
        }));
    } else {
        let track = b24_track.ok_or("no traditional B24 track found")?;
        emit_json(&serde_json::json!({
            "type": "track-discovered",
            "route": "mpeg_ts_b24_verified",
            "track": track,
        }));
        emit_json(&serde_json::json!({ "type": "started", "track": track }));
        emit_stage("decoding");
        let summary = decode_b24_with_progress(path, &track, |summary| {
            emit_json(&serde_json::json!({
                "type": "progress",
                "bytes_read": summary.bytes_read,
                "captions": summary.captions,
                "warnings": summary.decoder_errors,
            }));
        })?;
        emit_empty_caption_diagnostic(&summary);
        emit_stage("completed");
        emit_json(&serde_json::json!({ "type": "completed", "summary": summary }));
    }
    Ok(())
}
