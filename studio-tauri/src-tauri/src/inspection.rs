use crate::{
    models::{Inspection, Track, WorkerProbe},
    worker::worker_path,
};
use std::collections::HashSet;
use std::{fs, path::PathBuf, process::Command};
use tauri::AppHandle;

fn append_tlv_caption_tracks(inspection: &crate::models::WorkerInspection, tracks: &mut Vec<Track>) {
    for (index, track) in inspection
        .tracks
        .iter()
        .filter(|track| track.kind.as_deref() == Some("stpp"))
        .enumerate()
    {
        let Some(track_id) = track.track_id else {
            continue;
        };
        tracks.push(Track {
            label: format!("tlv_mmtp_stpp:{}", index + 1),
            detail: "track.tlv_mmtp_stpp".into(),
            pid: Some(format!("MMTP 0x{track_id:04X}")),
            kind: "tlv_mmtp_stpp".into(),
            ordinal: index + 1,
            service_id: None,
            language: None,
            service_name: None,
            logical_track: format!("tlv-mmtp:asset=stpp:packet={track_id:04x}"),
        });
    }
}

#[tauri::command]
pub fn inspect_source(app: AppHandle, path: String) -> Result<Inspection, String> {
    let source = PathBuf::from(&path);
    let metadata =
        fs::metadata(&source).map_err(|error| format!("Cannot read source file: {error}"))?;
    let output = Command::new(worker_path(Some(&app))?)
        .arg("inspect")
        .arg(&source)
        .output()
        .map_err(|error| format!("Could not start ARIB worker: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let probe: WorkerProbe = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|event| event.get("type").and_then(|value| value.as_str()) == Some("input_probe"))
        .ok_or_else(|| "Worker did not return an input_probe event".to_owned())
        .and_then(|event| {
            serde_json::from_value(event)
                .map_err(|error| format!("Worker returned invalid inspection data: {error}"))
        })?;
    let mut tracks = Vec::new();
    let b24_tracks = if probe.b24_tracks.is_empty() {
        probe.b24_track.into_iter().collect()
    } else {
        probe.b24_tracks
    };
    let verified_b24_pids = b24_tracks
        .iter()
        .map(|track| track.caption_pid)
        .collect::<HashSet<_>>();
    for (index, track) in b24_tracks.into_iter().enumerate() {
        let logical_track = format!(
            "b24:service={}:component={:02x}:language={}",
            track.service_id.unwrap_or_default(),
            track.component_tag,
            track.language.as_deref().unwrap_or("und")
        );
        tracks.push(Track {
            // User-facing wording is resolved from `kind` and `ordinal` by
            // the frontend locale pack. These values are stable fallbacks.
            label: format!("b24_verified:{}", index + 1),
            detail: "track.b24_verified".into(),
            pid: Some(format!("PID 0x{:04X}", track.caption_pid)),
            kind: "b24_verified".into(),
            ordinal: index + 1,
            service_id: track.service_id,
            language: track.language,
            service_name: track.service_name,
            logical_track,
        });
    }
    if let Some(data_tracks) = probe.mpeg_ts_data_tracks {
        let pmt_pid = data_tracks.pmt_pid;
        for (index, pid) in data_tracks
            .pids
            .into_iter()
            .filter(|pid| !verified_b24_pids.contains(pid))
            .enumerate()
        {
            let kind = if data_tracks.caption_pids.contains(&pid) {
                "mpeg_ts_ttml_caption"
            } else if data_tracks.superimpose_pids.contains(&pid) {
                "mpeg_ts_ttml_superimpose"
            } else {
                "mpeg_ts_ttml_candidate"
            };
            tracks.push(Track {
                label: format!("{kind}:{}", index + 1),
                detail: format!("track.{kind}"),
                pid: Some(format!("PID 0x{pid:04X}")),
                kind: kind.into(),
                ordinal: index + 1,
                service_id: None,
                language: None,
                service_name: None,
                logical_track: format!(
                    "mpeg-ts-ttml:pmt={pmt_pid}:kind={kind}:ordinal={}",
                    index + 1
                ),
            });
        }
    }
    if let Some(data_tracks) = probe.m2ts_data_tracks {
        let pmt_pid = data_tracks.pmt_pid;
        for (index, pid) in data_tracks.pids.into_iter().enumerate() {
            let kind = if data_tracks.caption_pids.contains(&pid) {
                "m2ts_ttml_caption"
            } else if data_tracks.superimpose_pids.contains(&pid) {
                "m2ts_ttml_superimpose"
            } else {
                "m2ts_ttml_candidate"
            };
            tracks.push(Track {
                label: format!("{kind}:{}", index + 1),
                detail: format!("track.{kind}"),
                pid: Some(format!("PID 0x{pid:04X}")),
                kind: kind.into(),
                ordinal: index + 1,
                service_id: None,
                language: None,
                service_name: None,
                logical_track: format!("m2ts-ttml:pmt={pmt_pid}:kind={kind}:ordinal={}", index + 1),
            });
        }
    }
    append_tlv_caption_tracks(&probe.inspection, &mut tracks);
    Ok(Inspection {
        name: source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Recording")
            .to_owned(),
        path,
        size: metadata.len(),
        container: probe.probe.kind.to_uppercase().replace('_', "-"),
        packet_size: probe.probe.packet_size,
        route_code: probe.inspection.route_code,
        route: probe.inspection.route,
        service: probe.inspection.service,
        tracks,
        broadcast: probe.inspection.broadcast,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WorkerInspection;

    #[test]
    fn exposes_each_stpp_asset_as_a_separate_selectable_track() {
        let inspection: WorkerInspection = serde_json::from_value(serde_json::json!({
            "route_code": "tlv_mmtp_experimental",
            "route": "TLV",
            "service": "MMT",
            "tracks": [
                { "kind": "hev1", "track_id": 0xf100 },
                { "kind": "stpp", "track_id": 0xf130 },
                { "kind": "stpp", "track_id": 0xf138 }
            ]
        }))
        .expect("worker inspection contract");
        let mut tracks = Vec::new();

        append_tlv_caption_tracks(&inspection, &mut tracks);

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].pid.as_deref(), Some("MMTP 0xF130"));
        assert_eq!(tracks[1].pid.as_deref(), Some("MMTP 0xF138"));
        assert_ne!(tracks[0].logical_track, tracks[1].logical_track);
        assert!(tracks.iter().all(|track| track.kind == "tlv_mmtp_stpp"));
    }
}
