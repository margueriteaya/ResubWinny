use crate::{
    jobs::{
        JobState, mark_artifacts, mark_job_state, record_completed_artifact, record_diagnostic,
        record_diagnostic_with_parameters, source_checkpoint_identity, write_artifacts,
        write_checkpoint,
    },
    models::{
        ArtifactRecord, CheckpointRecord, DrcsMappingInput, ExportPreservation, ExportSelection,
    },
    state::AppState,
    worker::{TaskEventEmitter, worker_path},
};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};

fn worker_failure_details(
    event: &serde_json::Value,
) -> (String, String, BTreeMap<String, serde_json::Value>) {
    let message = event
        .get("message")
        .and_then(|value| value.as_str())
        .unwrap_or("Worker operation failed.")
        .to_owned();
    let code = event
        .get("code")
        .and_then(|value| value.as_str())
        .unwrap_or("worker.operation_failed")
        .to_owned();
    let parameters = event
        .get("parameters")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    (message, code, parameters)
}

#[allow(
    clippy::too_many_arguments,
    reason = "compatibility entry point mirrors the versioned export IPC contract"
)]
pub fn start_export_impl(
    app: AppHandle,
    state: &Arc<AppState>,
    source: String,
    output: String,
    archive: bool,
    raw: bool,
    drcs_report: bool,
    drcs_mappings: Option<Vec<DrcsMappingInput>>,
    track_id: Option<u16>,
    logical_track: Option<String>,
    job_id: Option<String>,
    export_selection: Option<ExportSelection>,
) -> Result<(), String> {
    if crate::jobs::paths_refer_to_same_location(
        std::path::Path::new(&source),
        std::path::Path::new(&output),
    )? {
        return Err(
            "The output path resolves to the source recording. Choose a different output location."
                .into(),
        );
    }
    let events = TaskEventEmitter::new(app.clone(), job_id.clone());
    let mut slot = state
        .child
        .lock()
        .map_err(|_| "Task state is unavailable")?;
    if slot.is_some() {
        return Err("An export task is already running.".into());
    }
    state.forced_cancel.store(false, Ordering::Release);
    let mut command = Command::new(worker_path(Some(&app))?);
    command
        .arg("convert")
        .arg(&source)
        .arg(&output)
        .arg("--overwrite")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(track_id) = track_id {
        command.arg("--track-id").arg(format!("0x{track_id:04X}"));
    }
    if let Some(logical_track) = logical_track.as_deref() {
        command.env("RESUBWINNY_LOGICAL_TRACK", logical_track);
    }
    if let Some(job_id) = job_id.as_deref().filter(|value| !value.is_empty()) {
        command.env("RESUBWINNY_JOB_ID", job_id);
    }
    let selection = export_selection.unwrap_or_else(|| ExportSelection {
        formats: if archive {
            vec!["JSON".into()]
        } else if raw {
            vec!["Raw Data".into()]
        } else if output.to_ascii_lowercase().ends_with(".ttml") {
            vec!["TTML".into()]
        } else {
            vec!["ASS".into()]
        },
        preservation: ExportPreservation::default(),
    });
    let checkpoint_identity = source_checkpoint_identity(std::path::Path::new(&source))?;
    let checkpoint_track_id = track_id;
    let has_format = |name: &str| selection.formats.iter().any(|format| format == name);
    if has_format("JSON") {
        command.arg("--archive");
    } else if archive {
        command.arg("--archive-only");
    }
    if has_format("TTML") {
        command.arg("--ttml");
    }
    if has_format("SRT") {
        command.arg("--srt");
    }
    if has_format("WebVTT") {
        command.arg("--webvtt");
    }
    if has_format("Raw Data") || raw {
        command.arg("--raw");
    }
    if !has_format("ASS") {
        command.arg("--no-ass");
    }
    for (preserve, flag) in [
        (selection.preservation.position, "--drop-position"),
        (selection.preservation.color, "--drop-color"),
        (selection.preservation.ruby, "--drop-ruby"),
        (selection.preservation.drcs, "--drop-drcs"),
        (selection.preservation.gaiji, "--drop-gaiji"),
        (selection.preservation.accessibility, "--drop-accessibility"),
    ] {
        if !preserve {
            command.arg(flag);
        }
    }
    if drcs_report {
        command.arg("--drcs-report");
    }
    if let Some(mappings) = drcs_mappings {
        let replacements: serde_json::Map<String, serde_json::Value> = mappings
            .into_iter()
            .filter(|m| m.action == "character" && !m.text.trim().is_empty())
            .filter_map(|m| {
                m.id.split_once('-')
                    .map(|(code, _)| (code.to_owned(), serde_json::Value::String(m.text)))
            })
            .collect();
        if !replacements.is_empty() {
            let map_path = PathBuf::from(&output).with_extension("drcs-map.json");
            fs::write(
                &map_path,
                serde_json::to_vec_pretty(&replacements)
                    .map_err(|e| format!("Could not encode DRCS mappings: {e}"))?,
            )
            .map_err(|e| format!("Could not save DRCS mapping file: {e}"))?;
            command.arg("--drcs-map").arg(map_path);
        }
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start export: {error}"))?;
    if let Some(job_id) = job_id.as_deref().filter(|value| !value.is_empty()) {
        let output_path = PathBuf::from(&output);
        let mut artifacts = Vec::new();
        for (format, kind, path) in [
            ("ASS", "ass", output_path.clone()),
            ("TTML", "ttml", output_path.with_extension("ttml")),
            ("SRT", "srt", output_path.with_extension("srt")),
            ("WebVTT", "webvtt", output_path.with_extension("vtt")),
            (
                "JSON",
                "archive",
                output_path.with_extension("caption.jsonl"),
            ),
            (
                "Raw Data",
                "raw-evidence",
                output_path.with_extension("caption.pes.jsonl"),
            ),
        ] {
            if has_format(format) {
                artifacts.push(ArtifactRecord {
                    kind: kind.into(),
                    temporary_path: path
                        .with_extension(format!(
                            "{}.part",
                            path.extension()
                                .and_then(|value| value.to_str())
                                .unwrap_or("output")
                        ))
                        .to_string_lossy()
                        .into_owned(),
                    existed_before_start: path.exists(),
                    path: path.to_string_lossy().into_owned(),
                    status: "pending".into(),
                });
            }
        }
        if drcs_report {
            let drcs = output_path.with_extension("drcs.json");
            artifacts.push(ArtifactRecord {
                kind: "drcs-report".into(),
                path: drcs.to_string_lossy().into_owned(),
                temporary_path: drcs
                    .with_extension("json.part")
                    .to_string_lossy()
                    .into_owned(),
                status: "pending".into(),
                existed_before_start: drcs.exists(),
            });
        }
        if raw {
            let raw_path = output_path.with_extension("caption.pes.jsonl");
            artifacts.push(ArtifactRecord {
                kind: "raw-evidence".into(),
                path: raw_path.to_string_lossy().into_owned(),
                temporary_path: raw_path
                    .with_extension("jsonl.part")
                    .to_string_lossy()
                    .into_owned(),
                status: "pending".into(),
                existed_before_start: raw_path.exists(),
            });
        }
        write_artifacts(&app, job_id, &artifacts);
    }
    let stdout = child.stdout.take().ok_or("Worker stdout is unavailable")?;
    let stderr = child.stderr.take().ok_or("Worker stderr is unavailable")?;
    let shared_state = Arc::clone(state);
    *slot = Some(child);
    drop(slot);
    events.emit(
        "started",
        "Caption extraction started.",
        Some(0),
        Some(0),
        Some(0),
        None,
    );
    let worker_cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_flag = Arc::clone(&worker_cancelled);
    let worker_failed = Arc::new(AtomicBool::new(false));
    let failed_flag = Arc::clone(&worker_failed);
    let stderr_events = events.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            stderr_events.emit("log", format!("worker: {line}"), None, None, None, None);
        }
    });
    let checkpoint_source = source.clone();
    let checkpoint_output = output.clone();
    let (checkpoint_source_size, checkpoint_source_modified, checkpoint_source_fingerprint) =
        checkpoint_identity;
    std::thread::spawn(move || {
        let mut last_sequence = None;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(event) => {
                    if event
                        .get("protocolVersion")
                        .and_then(|v| v.as_u64())
                        .is_some_and(|v| v != 1)
                    {
                        let mut parameters = BTreeMap::new();
                        parameters.insert(
                            "actual".into(),
                            event.get("protocolVersion").cloned().unwrap_or_default(),
                        );
                        parameters.insert("expected".into(), serde_json::Value::from(1));
                        record_diagnostic_with_parameters(
                            &app,
                            &shared_state,
                            job_id.as_deref(),
                            "error",
                            "worker.protocol_version",
                            parameters.clone(),
                            "Unsupported worker protocol version.",
                        );
                        events.emit_with_details(
                            "diagnostic",
                            "worker.protocol_version",
                            parameters,
                            "Unsupported worker protocol version.",
                            None,
                            None,
                            None,
                            None,
                        );
                        continue;
                    }
                    if let Some(sequence) = event.get("sequence").and_then(|v| v.as_u64()) {
                        if let Some(previous) =
                            last_sequence.filter(|previous| sequence <= *previous)
                        {
                            let mut parameters = BTreeMap::new();
                            parameters.insert("previous".into(), serde_json::Value::from(previous));
                            parameters.insert("current".into(), serde_json::Value::from(sequence));
                            record_diagnostic_with_parameters(
                                &app,
                                &shared_state,
                                job_id.as_deref(),
                                "error",
                                "worker.sequence",
                                parameters.clone(),
                                "Worker event sequence is out of order.",
                            );
                            events.emit_with_details(
                                "diagnostic",
                                "worker.sequence",
                                parameters,
                                "Worker event sequence is out of order.",
                                None,
                                None,
                                None,
                                None,
                            );
                        }
                        last_sequence = Some(sequence);
                    }
                    let kind = event.get("type").and_then(|v| v.as_str());
                    match kind {
                        Some("progress") => {
                            let bytes_read = event
                                .get("bytes_read")
                                .and_then(|v| v.as_u64())
                                .unwrap_or_default();
                            let captions = event
                                .get("captions")
                                .and_then(|v| v.as_u64())
                                .unwrap_or_default();
                            let warnings = event
                                .get("warnings")
                                .and_then(|v| v.as_u64())
                                .unwrap_or_default();
                            if let Some(job_id) = job_id.as_deref()
                                && write_checkpoint(
                                    &app,
                                    &CheckpointRecord {
                                        job_id: job_id.to_owned(),
                                        source: checkpoint_source.clone(),
                                        output: checkpoint_output.clone(),
                                        bytes_read,
                                        captions,
                                        warnings,
                                        strategy: "full-replay-from-trusted-origin".into(),
                                        updated_at: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|value| value.as_secs())
                                            .unwrap_or_default(),
                                        source_size: Some(checkpoint_source_size),
                                        source_modified: Some(checkpoint_source_modified),
                                        source_fingerprint: Some(
                                            checkpoint_source_fingerprint.clone(),
                                        ),
                                        track_id: checkpoint_track_id,
                                    },
                                )
                                .is_ok()
                            {
                                events.emit(
                                    "checkpoint-written",
                                    "Task checkpoint updated.",
                                    Some(bytes_read),
                                    Some(captions),
                                    Some(warnings),
                                    None,
                                );
                            }
                            events.emit(
                                "progress",
                                "Processing caption stream…",
                                Some(bytes_read),
                                Some(captions),
                                Some(warnings),
                                None,
                            )
                        }
                        Some("diagnostic") => {
                            let message = event
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Worker diagnostic.");
                            let code = event
                                .get("code")
                                .and_then(|v| v.as_str())
                                .unwrap_or("worker.diagnostic");
                            let parameters: BTreeMap<String, serde_json::Value> = event
                                .get("parameters")
                                .and_then(|value| serde_json::from_value(value.clone()).ok())
                                .unwrap_or_default();
                            record_diagnostic_with_parameters(
                                &app,
                                &shared_state,
                                job_id.as_deref(),
                                "warning",
                                code,
                                parameters.clone(),
                                message,
                            );
                            events.emit_with_details(
                                "diagnostic",
                                code,
                                parameters,
                                message,
                                None,
                                None,
                                None,
                                None,
                            );
                        }
                        Some("hello") => events.emit(
                            "hello",
                            "Worker protocol connected.",
                            None,
                            None,
                            None,
                            None,
                        ),
                        Some("stage-changed") => {
                            let stage = event
                                .get("stage")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown");
                            events.emit("stage-changed", stage, None, None, None, None)
                        }
                        Some("track-discovered") => events.emit(
                            "track-discovered",
                            "Caption track discovered.",
                            None,
                            None,
                            None,
                            None,
                        ),
                        Some("drcs-discovered") => events.emit(
                            "drcs-discovered",
                            "DRCS glyphs discovered.",
                            None,
                            None,
                            event.get("count").and_then(|value| value.as_u64()),
                            None,
                        ),
                        Some("feature_observed") | Some("feature_summary") => {
                            let kind = event.get("type").and_then(|value| value.as_str()).unwrap_or("feature_summary");
                            let parameters = ["logicalTrack", "feature", "state", "observedCount", "complete", "details"]
                                .into_iter()
                                .filter_map(|key| event.get(key).cloned().map(|value| (key.to_owned(), value)))
                                .collect();
                            events.emit_with_details(
                                kind,
                                format!("task.{kind}"),
                                parameters,
                                "Caption source feature updated.",
                                None,
                                None,
                                None,
                                None,
                            )
                        }
                        Some("checkpoint-written") => events.emit(
                            "checkpoint-written",
                            "Worker checkpoint updated.",
                            None,
                            None,
                            None,
                            None,
                        ),
                        Some("paused") => {
                            mark_job_state(
                                &app,
                                &shared_state,
                                job_id.as_deref(),
                                JobState::Paused,
                            );
                            events.emit("paused", "Export paused.", None, None, None, None)
                        }
                        Some("resumed") => {
                            mark_job_state(
                                &app,
                                &shared_state,
                                job_id.as_deref(),
                                JobState::Running,
                            );
                            events.emit("resumed", "Export resumed.", None, None, None, None)
                        }
                        Some("cancelled") => {
                            cancelled_flag.store(true, Ordering::Relaxed);
                            events.emit("cancelled", "Export cancelled.", None, None, None, None)
                        }
                        Some("artifact-created") => {
                            let kind = event.get("kind").and_then(|value| value.as_str());
                            let path = event.get("path").and_then(|value| value.as_str());
                            if let (Some(kind), Some(path)) = (kind, path) {
                                record_completed_artifact(&app, job_id.as_deref(), kind, path);
                                events.emit(
                                    "artifact-created",
                                    format!("Created {kind} artifact."),
                                    None,
                                    None,
                                    None,
                                    Some(path.to_owned()),
                                );
                            } else {
                                record_diagnostic(
                                    &app,
                                    &shared_state,
                                    job_id.as_deref(),
                                    "warning",
                                    "worker.artifact_invalid",
                                    "Worker reported an artifact without a usable kind or path.",
                                );
                            }
                        }
                        Some("failed") => {
                            failed_flag.store(true, Ordering::Relaxed);
                            let (message, code, parameters) = worker_failure_details(&event);
                            mark_job_state(
                                &app,
                                &shared_state,
                                job_id.as_deref(),
                                JobState::Failed,
                            );
                            record_diagnostic_with_parameters(
                                &app,
                                &shared_state,
                                job_id.as_deref(),
                                "error",
                                &code,
                                parameters.clone(),
                                &message,
                            );
                            events.emit_with_details(
                                "failed",
                                &code,
                                parameters,
                                &message,
                                None,
                                None,
                                None,
                                None,
                            );
                        }
                        _ => events.emit("log", event.to_string(), None, None, None, None),
                    }
                }
                Err(_) => {
                    record_diagnostic(
                        &app,
                        &shared_state,
                        job_id.as_deref(),
                        "warning",
                        "worker.invalid_json",
                        "Worker emitted a non-JSON line.",
                    );
                    events.emit("log", line, None, None, None, None)
                }
            }
        }
        let status = shared_state
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.take())
            .and_then(|mut child| child.wait().ok());
        if let Ok(mut active) = shared_state.active_queue_job.lock()
            && active.as_deref() == job_id.as_deref()
        {
            *active = None;
        }
        if cancelled_flag.load(Ordering::Relaxed)
            || shared_state.forced_cancel.swap(false, Ordering::AcqRel)
        {
            mark_job_state(&app, &shared_state, job_id.as_deref(), JobState::Cancelled);
            mark_artifacts(&app, job_id.as_deref(), "cancelled");
            events.emit("cancelled", "Export cancelled.", None, None, None, None);
        } else if worker_failed.load(Ordering::Relaxed) {
            mark_job_state(&app, &shared_state, job_id.as_deref(), JobState::Failed);
            mark_artifacts(&app, job_id.as_deref(), "failed");
        } else if status.is_some_and(|result| result.success()) {
            mark_job_state(&app, &shared_state, job_id.as_deref(), JobState::Completed);
            mark_artifacts(&app, job_id.as_deref(), "completed");
            events.emit(
                "completed",
                "Caption extraction completed.",
                None,
                None,
                None,
                Some(output),
            );
        } else {
            mark_job_state(&app, &shared_state, job_id.as_deref(), JobState::Failed);
            mark_artifacts(&app, job_id.as_deref(), "failed");
            events.emit(
                "failed",
                "Caption extraction stopped or failed. See the task log for details.",
                None,
                None,
                None,
                None,
            );
        }
    });
    Ok(())
}

#[cfg(test)]
mod contract_tests {
    use super::worker_failure_details;

    #[test]
    fn export_conflict_keeps_its_structured_parameters() {
        let event = serde_json::json!({
            "type": "failed",
            "code": "export_conflict",
            "message": "fallback text",
            "parameters": {
                "formats": ["ASS", "SRT"],
                "feature": "ruby",
                "logicalTrack": "service=1:component=48:lang=jpn",
                "availableActions": ["disable_preservation:ruby", "remove_format"]
            }
        });

        let (message, code, parameters) = worker_failure_details(&event);

        assert_eq!(message, "fallback text");
        assert_eq!(code, "export_conflict");
        assert_eq!(parameters["formats"], serde_json::json!(["ASS", "SRT"]));
        assert_eq!(parameters["feature"], "ruby");
        assert_eq!(
            parameters["logicalTrack"],
            "service=1:component=48:lang=jpn"
        );
        assert_eq!(parameters["availableActions"].as_array().unwrap().len(), 2);
    }
}

#[tauri::command]
#[allow(
    clippy::too_many_arguments,
    reason = "parameter names are part of the existing typed Tauri command contract"
)]
pub fn start_export(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    output: String,
    archive: bool,
    raw: bool,
    drcs_report: bool,
    drcs_mappings: Option<Vec<DrcsMappingInput>>,
    track_id: Option<u16>,
    job_id: Option<String>,
    formats: Option<Vec<String>>,
    preservation: Option<ExportPreservation>,
) -> Result<(), String> {
    start_export_impl(
        app,
        state.inner(),
        source,
        output,
        archive,
        raw,
        drcs_report,
        drcs_mappings,
        track_id,
        None,
        job_id,
        formats.map(|formats| ExportSelection {
            formats,
            preservation: preservation.unwrap_or_default(),
        }),
    )
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewIndexStart {
    pub archive_path: String,
}

#[tauri::command]
pub fn start_preview_index(
    app: AppHandle,
    source: String,
    track_id: Option<u16>,
    state: State<'_, Arc<AppState>>,
) -> Result<PreviewIndexStart, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve preview data directory: {error}"))?
        .join("preview-index");
    if root.exists() {
        fs::remove_dir_all(&root)
            .map_err(|error| format!("Could not replace stale preview index: {error}"))?;
    }
    let directory = root.join(format!("{stamp}"));
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create preview data directory: {error}"))?;
    let output = directory.join("captions.ass");
    let archive = output.with_extension("caption.jsonl");
    start_export_impl(
        app,
        state.inner(),
        source,
        output.to_string_lossy().into_owned(),
        false,
        false,
        false,
        None,
        track_id,
        None,
        None,
        Some(ExportSelection {
            formats: vec!["JSON".into()],
            preservation: ExportPreservation::default(),
        }),
    )?;
    Ok(PreviewIndexStart {
        archive_path: archive.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub fn cancel_export(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    cancel_export_impl(state.inner())
}

#[tauri::command]
pub fn cancel_export_and_wait(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    cancel_export_impl(state.inner())?;
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut forced = false;
    loop {
        let active = state
            .child
            .lock()
            .map_err(|_| "Task state is unavailable")?
            .is_some();
        if !active {
            // The worker reader emits its terminal event immediately after it
            // releases the child slot. Let that event reach the WebView before
            // a replacement source starts another worker session.
            std::thread::sleep(Duration::from_millis(50));
            return Ok(());
        }
        if !forced && Instant::now() >= deadline {
            let mut child = state
                .child
                .lock()
                .map_err(|_| "Task state is unavailable")?;
            if let Some(process) = child.as_mut() {
                state.forced_cancel.store(true, Ordering::Release);
                process.kill().map_err(|error| {
                    format!("Could not force-stop the unresponsive caption task: {error}")
                })?;
            }
            forced = true;
        } else if forced && Instant::now() >= deadline + Duration::from_secs(2) {
            return Err("The caption task remained active after it was force-stopped.".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn cancel_export_impl(state: &Arc<AppState>) -> Result<(), String> {
    let mut child = state
        .child
        .lock()
        .map_err(|_| "Task state is unavailable")?;
    if let Some(process) = child.as_mut() {
        if let Some(stdin) = process.stdin.as_mut() {
            stdin
                .write_all(br#"{"type":"cancel"}"#)
                .and_then(|_| stdin.write_all(b"\n"))
                .map_err(|error| format!("Could not send cancel message: {error}"))?;
        } else {
            process
                .kill()
                .map_err(|error| format!("Could not stop export: {error}"))?;
        }
    }
    Ok(())
}

pub fn send_control_impl(state: &Arc<AppState>, message: &str) -> Result<(), String> {
    let mut child = state
        .child
        .lock()
        .map_err(|_| "Task state is unavailable")?;
    let process = child.as_mut().ok_or("No export task is running.")?;
    let stdin = process
        .stdin
        .as_mut()
        .ok_or("Worker control channel is unavailable.")?;
    stdin
        .write_all(message.as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .map_err(|error| format!("Could not send worker control message: {error}"))
}

#[tauri::command]
pub fn pause_export(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    send_control_impl(state.inner(), r#"{"type":"pause"}"#)
}

#[tauri::command]
pub fn resume_export(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    send_control_impl(state.inner(), r#"{"type":"resume"}"#)
}
