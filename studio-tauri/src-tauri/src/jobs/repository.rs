use super::*;

const DIAGNOSTIC_MEMORY_LIMIT: usize = 500;
const DIAGNOSTIC_COMPATIBILITY_LIMIT: usize = 2_000;
pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn default_output_path_impl(
    source: &Path,
    output_directory: Option<&Path>,
) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or("The source path does not contain a file name.")?;
    let stem = source
        .file_stem()
        .filter(|value| !value.is_empty())
        .unwrap_or(file_name);
    let directory = output_directory
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| source.parent().map(Path::to_path_buf))
        .ok_or("Could not resolve an output directory for the source.")?;
    let mut output = directory.join(stem);
    output.set_extension("ass");
    if paths_refer_to_same_location(source, &output)? {
        output = directory.join(format!("{}.captions.ass", stem.to_string_lossy()));
    }
    Ok(output)
}

fn comparable_path(path: &Path) -> Result<String, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("Could not resolve the current directory: {error}"))?
            .join(path)
    };
    let resolved = if absolute.exists() {
        absolute.canonicalize().unwrap_or(absolute)
    } else if let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) {
        parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf())
            .join(name)
    } else {
        absolute
    };
    let value = resolved.to_string_lossy().replace('/', "\\");
    #[cfg(windows)]
    return Ok(value.to_lowercase());
    #[cfg(not(windows))]
    Ok(value)
}

pub(crate) fn paths_refer_to_same_location(source: &Path, output: &Path) -> Result<bool, String> {
    Ok(comparable_path(source)? == comparable_path(output)?)
}

#[tauri::command]
pub fn default_output_path(
    source: String,
    output_directory: Option<String>,
) -> Result<String, String> {
    default_output_path_impl(
        Path::new(&source),
        output_directory.as_deref().map(Path::new),
    )
    .map(|path| path.to_string_lossy().into_owned())
}

fn recovered_state(state: JobState) -> Option<JobState> {
    match state {
        JobState::Starting
        | JobState::Running
        | JobState::Pausing
        | JobState::Paused
        | JobState::Resuming
        | JobState::Cancelling => Some(JobState::Interrupted),
        // The in-memory queue is deliberately not restored automatically:
        // after an application restart it needs an explicit user action.
        JobState::Queued => Some(JobState::Ready),
        _ => None,
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::{JobState, recovered_state};

    #[test]
    fn restart_marks_active_work_interrupted_and_queue_ready() {
        assert!(matches!(
            recovered_state(JobState::Running),
            Some(JobState::Interrupted)
        ));
        assert!(matches!(
            recovered_state(JobState::Paused),
            Some(JobState::Interrupted)
        ));
        assert!(matches!(
            recovered_state(JobState::Queued),
            Some(JobState::Ready)
        ));
        assert!(recovered_state(JobState::Completed).is_none());
    }
}
fn jobs_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("jobs.json"))
}
fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("task-history.json"))
}
fn diagnostics_path(app: &AppHandle, job_id: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("jobs")
        .join(job_id)
        .join("diagnostics.jsonl"))
}

pub(crate) fn source_checkpoint_identity(
    path: &std::path::Path,
) -> Result<(u64, u64, String), String> {
    const SAMPLE_BYTES: u64 = 64 * 1024;
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not inspect checkpoint source: {error}"))?;
    let size = metadata.len();
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
        .unwrap_or_default();
    let mut file =
        File::open(path).map_err(|error| format!("Could not open checkpoint source: {error}"))?;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut sample = vec![0_u8; SAMPLE_BYTES.min(size) as usize];
    if !sample.is_empty() {
        file.read_exact(&mut sample)
            .map_err(|error| format!("Could not read checkpoint source head: {error}"))?;
        fnv1a_update(&mut hash, &sample);
        if size > SAMPLE_BYTES {
            file.seek(SeekFrom::Start(size.saturating_sub(SAMPLE_BYTES)))
                .map_err(|error| format!("Could not seek checkpoint source tail: {error}"))?;
            sample.resize(SAMPLE_BYTES.min(size) as usize, 0);
            file.read_exact(&mut sample)
                .map_err(|error| format!("Could not read checkpoint source tail: {error}"))?;
            fnv1a_update(&mut hash, &sample);
        }
    }
    fnv1a_update(&mut hash, &size.to_le_bytes());
    Ok((size, modified, format!("fnv1a64:{hash:016x}")))
}

fn fnv1a_update(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}
fn artifacts_path(app: &AppHandle, job_id: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("jobs")
        .join(job_id)
        .join("artifacts.json"))
}
pub(super) fn checkpoint_path(app: &AppHandle, job_id: &str) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("jobs")
        .join(job_id)
        .join("checkpoint.json"))
}
pub(super) fn persist(app: &AppHandle, jobs: &[JobRecord]) -> Result<(), String> {
    let path = jobs_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create job data directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(jobs)
        .map_err(|error| format!("Could not encode jobs: {error}"))?;
    write_atomic(&path, &bytes).map_err(|error| format!("Could not publish jobs: {error}"))
}

pub fn load_persisted_jobs(app: &AppHandle, state: &Arc<AppState>) {
    let Ok(path) = jobs_path(app) else { return };
    let Ok(bytes) = fs::read(path) else { return };
    let Ok(mut records) = serde_json::from_slice::<Vec<JobRecord>>(&bytes) else {
        return;
    };
    let mut recovered = false;
    for record in &mut records {
        let next_state = recovered_state(record.state.clone());
        if let Some(next_state) = next_state {
            record.state = next_state;
            record.updated_at = now();
            recovered = true;
        }
    }
    if let Ok(mut jobs) = state.jobs.lock() {
        *jobs = records.clone();
    }
    if recovered {
        let _ = persist(app, &records);
    }
    let Ok(job_ids) = state.jobs.lock().map(|jobs| {
        jobs.iter()
            .map(|job| job.job_id.clone())
            .collect::<Vec<_>>()
    }) else {
        return;
    };
    if let Ok(mut diagnostics) = state.diagnostics.lock() {
        for job_id in job_ids {
            let Ok(path) = diagnostics_path(app, &job_id) else {
                continue;
            };
            let Ok(items) = read_recent_diagnostics(&path, DIAGNOSTIC_MEMORY_LIMIT) else {
                continue;
            };
            if !items.is_empty() {
                diagnostics.insert(job_id, items);
            }
        }
    }
}

pub fn mark_job_state(
    app: &AppHandle,
    state: &Arc<AppState>,
    job_id: Option<&str>,
    next_state: JobState,
) {
    let Some(job_id) = job_id else { return };
    let Ok(mut jobs) = state.jobs.lock() else {
        return;
    };
    if let Some(job) = jobs.iter_mut().find(|job| job.job_id == job_id) {
        job.state = next_state;
        job.updated_at = now();
        let _ = persist(app, &jobs);
    }
}

pub fn record_diagnostic(
    app: &AppHandle,
    state: &Arc<AppState>,
    job_id: Option<&str>,
    severity: &str,
    code: &str,
    message: impl Into<String>,
) {
    record_diagnostic_with_parameters(app, state, job_id, severity, code, BTreeMap::new(), message);
}

pub fn record_diagnostic_with_parameters(
    app: &AppHandle,
    state: &Arc<AppState>,
    job_id: Option<&str>,
    severity: &str,
    code: &str,
    parameters: BTreeMap<String, Value>,
    message: impl Into<String>,
) {
    let Some(job_id) = job_id else { return };
    let record = DiagnosticRecord {
        timestamp: now(),
        severity: severity.to_owned(),
        code: code.to_owned(),
        parameters,
        message: message.into(),
    };
    if let Ok(mut diagnostics) = state.diagnostics.lock() {
        let entries = diagnostics.entry(job_id.to_owned()).or_default();
        entries.push(record.clone());
        if entries.len() > DIAGNOSTIC_MEMORY_LIMIT {
            let excess = entries.len() - DIAGNOSTIC_MEMORY_LIMIT;
            entries.drain(0..excess);
        }
        if let Ok(path) = diagnostics_path(app, job_id) {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(line) = serde_json::to_string(&record) {
                let _ = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .and_then(|mut file| {
                        use std::io::Write;
                        writeln!(file, "{line}")
                    });
            }
        }
    }
}

pub fn write_artifacts(app: &AppHandle, job_id: &str, artifacts: &[ArtifactRecord]) {
    let Ok(path) = artifacts_path(app, job_id) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(artifacts) {
        let _ = write_atomic(&path, &bytes);
    }
}

pub fn record_completed_artifact(app: &AppHandle, job_id: Option<&str>, kind: &str, path: &str) {
    let Some(job_id) = job_id else { return };
    let Ok(manifest_path) = artifacts_path(app, job_id) else {
        return;
    };
    let mut artifacts = fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<ArtifactRecord>>(&bytes).ok())
        .unwrap_or_default();
    if let Some(artifact) = artifacts.iter_mut().find(|artifact| artifact.path == path) {
        artifact.kind = kind.to_owned();
        artifact.status = "completed".into();
        artifact.temporary_path.clear();
    } else {
        artifacts.push(ArtifactRecord {
            kind: kind.to_owned(),
            path: path.to_owned(),
            temporary_path: String::new(),
            status: "completed".into(),
            existed_before_start: false,
        });
    }
    write_artifacts(app, job_id, &artifacts);
}

pub fn write_checkpoint(app: &AppHandle, checkpoint: &CheckpointRecord) -> Result<(), String> {
    let path = checkpoint_path(app, &checkpoint.job_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create checkpoint directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(checkpoint)
        .map_err(|error| format!("Could not encode checkpoint: {error}"))?;
    write_atomic(&path, &bytes).map_err(|error| format!("Could not publish checkpoint: {error}"))
}

pub fn mark_artifacts(app: &AppHandle, job_id: Option<&str>, status: &str) {
    let Some(job_id) = job_id else { return };
    let Ok(path) = artifacts_path(app, job_id) else {
        return;
    };
    let Ok(bytes) = fs::read(&path) else { return };
    let Ok(mut artifacts) = serde_json::from_slice::<Vec<ArtifactRecord>>(&bytes) else {
        return;
    };
    for artifact in &mut artifacts {
        artifact.status = reconciled_artifact_status(artifact, status);
    }
    write_artifacts(app, job_id, &artifacts);
}

fn reconciled_artifact_status(artifact: &ArtifactRecord, terminal_status: &str) -> String {
    // A Worker `artifact-created` event is stronger evidence than a subsequent
    // job-level failure: a later optional artifact may have failed, while this
    // one was already atomically published.
    if artifact.status == "completed" {
        return "completed".into();
    }
    if artifact.existed_before_start && PathBuf::from(&artifact.path).exists() {
        return "preserved".into();
    }
    if !artifact.temporary_path.is_empty() && PathBuf::from(&artifact.temporary_path).exists() {
        return "incomplete".into();
    }
    if terminal_status == "completed" && PathBuf::from(&artifact.path).exists() {
        return "completed".into();
    }
    terminal_status.into()
}

#[cfg(test)]
mod tests {
    use super::{
        default_output_path_impl, paths_refer_to_same_location, read_diagnostics_window,
        read_recent_diagnostics, reconciled_artifact_status, source_checkpoint_identity,
    };
    use crate::models::{ArtifactRecord, DiagnosticRecord};
    use std::{collections::BTreeMap, io::Write};

    fn artifact(path: std::path::PathBuf, temporary_path: std::path::PathBuf) -> ArtifactRecord {
        ArtifactRecord {
            kind: "captions".into(),
            path: path.to_string_lossy().into_owned(),
            temporary_path: temporary_path.to_string_lossy().into_owned(),
            status: "pending".into(),
            existed_before_start: false,
        }
    }

    #[test]
    fn default_output_path_preserves_dotted_directories_and_extensionless_names() {
        let source = std::path::Path::new("recordings.with.dots").join("programme");
        let output = default_output_path_impl(&source, None).expect("output path");
        assert_eq!(
            output,
            std::path::Path::new("recordings.with.dots").join("programme.ass")
        );
        assert!(!paths_refer_to_same_location(&source, &output).expect("compare paths"));
    }

    #[test]
    fn default_output_path_never_reuses_an_ass_named_source() {
        let source = std::path::Path::new("recording.ass");
        let output = default_output_path_impl(source, None).expect("output path");
        assert_eq!(output, std::path::Path::new("recording.captions.ass"));
        assert!(!paths_refer_to_same_location(source, &output).expect("compare paths"));
    }

    #[test]
    fn keeps_a_worker_confirmed_artifact_completed_after_later_failure() {
        let mut record = artifact("new.ass".into(), "new.ass.part".into());
        record.kind = "drcs-report".into();
        record.path = "new.drcs.json".into();
        record.temporary_path.clear();
        record.status = "completed".into();
        assert_eq!(reconciled_artifact_status(&record, "failed"), "completed");
    }

    #[test]
    fn distinguishes_preserved_previous_output_from_incomplete_output() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resubwinny-artifact-{stamp}.ass"));
        let part = path.with_extension("ass.part");
        std::fs::write(&path, b"previous output").expect("previous output");
        let mut preserved = artifact(path.clone(), part.clone());
        preserved.existed_before_start = true;
        assert_eq!(
            reconciled_artifact_status(&preserved, "failed"),
            "preserved"
        );
        std::fs::remove_file(&path).expect("remove old output");
        std::fs::write(&part, b"partial").expect("partial output");
        assert_eq!(
            reconciled_artifact_status(&preserved, "cancelled"),
            "incomplete"
        );
        std::fs::remove_file(part).expect("cleanup");
    }

    #[test]
    fn export_conflict_never_reconciles_a_part_file_as_completed() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resubwinny-conflict-{stamp}.ass"));
        let part = path.with_extension("ass.part");
        std::fs::write(&part, b"unpublishable partial output").expect("partial output");
        let record = artifact(path, part.clone());

        assert_eq!(reconciled_artifact_status(&record, "failed"), "incomplete");

        std::fs::remove_file(part).expect("cleanup");
    }

    #[test]
    fn diagnostics_are_paged_from_jsonl_and_only_recent_records_are_cached() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resubwinny-diagnostics-{stamp}.jsonl"));
        let mut file = std::fs::File::create(&path).expect("diagnostics");
        for timestamp in 0..10 {
            let record = DiagnosticRecord {
                timestamp,
                severity: "info".into(),
                code: format!("diagnostic.{timestamp}"),
                parameters: BTreeMap::new(),
                message: String::new(),
            };
            writeln!(file, "{}", serde_json::to_string(&record).unwrap()).unwrap();
        }
        writeln!(file, "incomplete-json").unwrap();
        drop(file);

        let page = read_diagnostics_window(&path, 3, 4).expect("page");
        assert_eq!(
            page.iter().map(|item| item.timestamp).collect::<Vec<_>>(),
            [3, 4, 5, 6]
        );
        let recent = read_recent_diagnostics(&path, 3).expect("recent");
        assert_eq!(
            recent.iter().map(|item| item.timestamp).collect::<Vec<_>>(),
            [7, 8, 9]
        );
        std::fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn checkpoint_identity_detects_source_replacement_without_reading_the_whole_recording() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resubwinny-source-{stamp}.ts"));
        std::fs::write(&path, vec![0x47; 192 * 1024]).expect("source");
        let original = source_checkpoint_identity(&path).expect("identity");
        let mut replacement = vec![0x47; 192 * 1024];
        *replacement.last_mut().unwrap() = 0;
        std::fs::write(&path, replacement).expect("replacement");
        let changed = source_checkpoint_identity(&path).expect("changed identity");
        assert_eq!(original.0, changed.0);
        assert_ne!(original.2, changed.2);
        std::fs::remove_file(path).expect("cleanup");
    }
}

#[tauri::command]
pub fn get_job_diagnostics(
    app: AppHandle,
    job_id: String,
) -> Result<Vec<DiagnosticRecord>, String> {
    let path = diagnostics_path(&app, &job_id)?;
    read_recent_diagnostics(&path, DIAGNOSTIC_COMPATIBILITY_LIMIT)
}

#[tauri::command]
pub fn get_job_diagnostics_window(
    app: AppHandle,
    job_id: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<DiagnosticRecord>, String> {
    let path = diagnostics_path(&app, &job_id)?;
    read_diagnostics_window(&path, offset, limit.min(500))
}

fn read_diagnostics_window(
    path: &std::path::Path,
    offset: usize,
    limit: usize,
) -> Result<Vec<DiagnosticRecord>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not open job diagnostics: {error}")),
    };
    let mut records = Vec::with_capacity(limit);
    for line in BufReader::new(file).lines().skip(offset).take(limit) {
        let line = line.map_err(|error| format!("Could not read job diagnostics: {error}"))?;
        let record = serde_json::from_str::<DiagnosticRecord>(&line)
            .map_err(|error| format!("Could not decode job diagnostic: {error}"))?;
        records.push(record);
    }
    Ok(records)
}

fn read_recent_diagnostics(
    path: &std::path::Path,
    limit: usize,
) -> Result<Vec<DiagnosticRecord>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not open job diagnostics: {error}")),
    };
    let mut recent = VecDeque::with_capacity(limit.min(256));
    let mut trailing_decode_error = None;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|error| format!("Could not read job diagnostics: {error}"))?;
        if let Some(error) = trailing_decode_error.take() {
            return Err(error);
        }
        let record = match serde_json::from_str::<DiagnosticRecord>(&line) {
            Ok(record) => record,
            Err(error) => {
                // A killed process may leave only its final JSONL record
                // partially written. Defer the error until another line proves
                // the corruption occurred in the middle of the stream.
                trailing_decode_error = Some(format!("Could not decode job diagnostic: {error}"));
                continue;
            }
        };
        if recent.len() == limit {
            recent.pop_front();
        }
        recent.push_back(record);
    }
    Ok(recent.into_iter().collect())
}

#[tauri::command]
pub fn list_jobs_window(
    offset: usize,
    limit: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<JobRecord>, String> {
    let limit = limit.min(200);
    state
        .jobs
        .lock()
        .map(|jobs| {
            jobs.iter()
                .rev()
                .skip(offset)
                .take(limit)
                .cloned()
                .collect()
        })
        .map_err(|_| "Job state is unavailable".into())
}

#[tauri::command]
pub fn get_job_artifacts(app: AppHandle, job_id: String) -> Result<Vec<ArtifactRecord>, String> {
    let path = artifacts_path(&app, &job_id)?;
    let bytes = fs::read(path).map_err(|_| "Artifact manifest is not available".to_owned())?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("Could not decode artifact manifest: {error}"))
}

#[tauri::command]
pub fn get_job_checkpoint(
    app: AppHandle,
    job_id: String,
) -> Result<Option<CheckpointRecord>, String> {
    let path = checkpoint_path(&app, &job_id)?;
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| format!("Could not decode checkpoint: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not read checkpoint: {error}")),
    }
}

#[tauri::command]
#[allow(
    clippy::too_many_arguments,
    reason = "parameter names are part of the existing typed Tauri command contract"
)]
pub fn create_job(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    source: String,
    output: String,
    archive: bool,
    raw: bool,
    track_id: Option<u16>,
    logical_track: Option<String>,
    drcs_report: bool,
    drcs_mappings: Vec<DrcsMappingInput>,
    formats: Option<Vec<String>>,
    preservation: Option<ExportPreservation>,
) -> Result<JobRecord, String> {
    let timestamp = now();
    let unique = JOB_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let record = JobRecord {
        job_id: format!("job-{nanos}-{}-{unique}", std::process::id()),
        source,
        output,
        archive,
        raw,
        track_id,
        logical_track,
        drcs_report,
        drcs_mappings,
        export_selection: ExportSelection {
            formats: formats
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| vec!["ASS".into()]),
            preservation: preservation.unwrap_or_default(),
        },
        state: JobState::Created,
        created_at: timestamp,
        updated_at: timestamp,
    };
    let mut jobs = state.jobs.lock().map_err(|_| "Job state is unavailable")?;
    jobs.push(record.clone());
    persist(&app, &jobs)?;
    Ok(record)
}

#[tauri::command]
pub fn list_jobs(state: State<'_, Arc<AppState>>) -> Result<Vec<JobRecord>, String> {
    state
        .jobs
        .lock()
        .map(|jobs| jobs.clone())
        .map_err(|_| "Job state is unavailable".into())
}

#[tauri::command]
pub fn load_task_history(app: AppHandle) -> Result<Vec<TaskHistoryRecord>, String> {
    match fs::read(history_path(&app)?) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("Could not decode task history: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not read task history: {error}")),
    }
}

#[tauri::command]
pub fn save_task_history(app: AppHandle, records: Vec<TaskHistoryRecord>) -> Result<(), String> {
    let path = history_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create history directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(&records)
        .map_err(|error| format!("Could not encode task history: {error}"))?;
    write_atomic(&path, &bytes).map_err(|error| format!("Could not publish task history: {error}"))
}

#[tauri::command]
pub fn get_job(
    job_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<JobRecord>, String> {
    state
        .jobs
        .lock()
        .map(|jobs| jobs.iter().find(|job| job.job_id == job_id).cloned())
        .map_err(|_| "Job state is unavailable".into())
}

#[tauri::command]
pub fn remove_job(
    app: AppHandle,
    job_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut jobs = state.jobs.lock().map_err(|_| "Job state is unavailable")?;
    jobs.retain(|job| job.job_id != job_id);
    persist(&app, &jobs)
}
