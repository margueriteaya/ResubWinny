use crate::{
    models::{Inspection, Track, WorkerProbe},
    worker::worker_path,
};
use std::collections::HashSet;
use std::{fs, path::PathBuf, process::Command};
use tauri::AppHandle;

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
        });
    }
    if let Some(data_tracks) = probe.mpeg_ts_data_tracks {
        for (index, pid) in data_tracks
            .pids
            .into_iter()
            .filter(|pid| !verified_b24_pids.contains(pid))
            .enumerate()
        {
            tracks.push(Track {
                label: format!("mpeg_ts_ttml_candidate:{}", index + 1),
                detail: "track.mpeg_ts_ttml_candidate".into(),
                pid: Some(format!("PID 0x{pid:04X}")),
                kind: "mpeg_ts_ttml_candidate".into(),
                ordinal: index + 1,
                service_id: None,
                language: None,
                service_name: None,
            });
        }
    }
    if let Some(data_tracks) = probe.m2ts_data_tracks {
        for (index, pid) in data_tracks.pids.into_iter().enumerate() {
            tracks.push(Track {
                label: format!("m2ts_ttml:{}", index + 1),
                detail: "track.m2ts_ttml".into(),
                pid: Some(format!("PID 0x{pid:04X}")),
                kind: "m2ts_ttml".into(),
                ordinal: index + 1,
                service_id: None,
                language: None,
                service_name: None,
            });
        }
    }
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
