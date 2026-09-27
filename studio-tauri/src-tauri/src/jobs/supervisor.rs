use super::*;

#[tauri::command]
pub fn start_job(
    app: AppHandle,
    job_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    start_job_impl(app, state.inner(), &job_id)
}

fn start_job_impl(app: AppHandle, state: &Arc<AppState>, job_id: &str) -> Result<(), String> {
    let record = state
        .jobs
        .lock()
        .map_err(|_| "Job state is unavailable")?
        .iter()
        .find(|job| job.job_id == job_id)
        .cloned()
        .ok_or("Job was not found.")?;
    start_export_impl(
        app.clone(),
        state,
        record.source,
        record.output,
        record.archive,
        record.raw,
        record.drcs_report,
        Some(record.drcs_mappings),
        record.track_id,
        record.logical_track,
        Some(job_id.to_owned()),
        Some(record.export_selection),
    )?;
    let mut jobs = state.jobs.lock().map_err(|_| "Job state is unavailable")?;
    if let Some(job) = jobs.iter_mut().find(|job| job.job_id == job_id) {
        job.state = JobState::Running;
        job.updated_at = now();
    }
    persist(&app, &jobs)
}

fn run_queue_supervisor(app: AppHandle, state: Arc<AppState>) {
    if state.supervisor_running.swap(true, Ordering::AcqRel) {
        return;
    }
    thread::spawn(move || {
        loop {
            let paused = state
                .queue_paused
                .lock()
                .map(|value| *value)
                .unwrap_or(true);
            let active = state
                .child
                .lock()
                .map(|child| child.is_some())
                .unwrap_or(true);
            if paused || active {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            let Some(job_id) = state
                .queue
                .lock()
                .ok()
                .and_then(|mut queue| queue.pop_front())
            else {
                // Keep one lightweight supervisor alive. Exiting here creates
                // a lost-wakeup window when an enqueue races with flag reset.
                thread::sleep(Duration::from_millis(100));
                continue;
            };
            if let Ok(mut active) = state.active_queue_job.lock() {
                *active = Some(job_id.clone());
            }
            if let Err(error) = start_job_impl(app.clone(), &state, &job_id) {
                if let Ok(mut active) = state.active_queue_job.lock() {
                    *active = None;
                }
                record_diagnostic(
                    &app,
                    &state,
                    Some(&job_id),
                    "error",
                    "job.start",
                    error.clone(),
                );
                mark_job_state(&app, &state, Some(&job_id), JobState::Failed);
            } else if state
                .queue_paused
                .lock()
                .map(|value| *value)
                .unwrap_or(true)
            {
                mark_job_state(&app, &state, Some(&job_id), JobState::Pausing);
                let _ = send_control_impl(&state, r#"{"type":"pause"}"#);
            }
        }
    });
}

#[tauri::command]
pub fn enqueue_jobs(
    app: AppHandle,
    job_ids: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    {
        let jobs = state.jobs.lock().map_err(|_| "Job state is unavailable")?;
        let mut queue = state
            .queue
            .lock()
            .map_err(|_| "Queue state is unavailable")?;
        for job_id in job_ids {
            if jobs.iter().any(|job| job.job_id == job_id)
                && !queue.iter().any(|queued| queued == &job_id)
            {
                queue.push_back(job_id);
            }
        }
    }
    *state
        .queue_paused
        .lock()
        .map_err(|_| "Queue state is unavailable")? = false;
    run_queue_supervisor(app, state.inner().clone());
    Ok(())
}

#[tauri::command]
pub fn pause_job(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let job_id = state
        .active_queue_job
        .lock()
        .ok()
        .and_then(|value| value.clone());
    if job_id.is_none() {
        let active = state
            .jobs
            .lock()
            .map_err(|_| "Job state is unavailable")?
            .iter()
            .find(|job| matches!(job.state, JobState::Running | JobState::Resuming))
            .map(|job| job.job_id.clone());
        if let Some(active) = active {
            mark_job_state(&app, state.inner(), Some(&active), JobState::Pausing);
        }
    } else {
        mark_job_state(&app, state.inner(), job_id.as_deref(), JobState::Pausing);
    }
    send_control_impl(state.inner(), r#"{"type":"pause"}"#)
}

#[tauri::command]
pub fn resume_job(
    app: AppHandle,
    job_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if state
        .child
        .lock()
        .map_err(|_| "Task state is unavailable")?
        .is_some()
    {
        if let Some(active) = state
            .jobs
            .lock()
            .map_err(|_| "Job state is unavailable")?
            .iter()
            .find(|job| matches!(job.state, JobState::Paused | JobState::Pausing))
            .map(|job| job.job_id.clone())
        {
            mark_job_state(&app, state.inner(), Some(&active), JobState::Resuming);
        }
        return send_control_impl(state.inner(), r#"{"type":"resume"}"#);
    }
    let job_id = job_id.ok_or("No resumable job was supplied.")?;
    let record = state
        .jobs
        .lock()
        .map_err(|_| "Job state is unavailable")?
        .iter()
        .find(|job| job.job_id == job_id)
        .cloned()
        .ok_or("Job was not found.")?;
    if !matches!(
        record.state,
        JobState::Interrupted | JobState::Failed | JobState::Cancelled
    ) {
        return Err("Only interrupted, failed, or cancelled jobs can replay a checkpoint.".into());
    }
    if !PathBuf::from(&record.source).is_file() {
        return Err("The checkpoint source recording is no longer available.".into());
    }
    let checkpoint_path = checkpoint_path(&app, &job_id)?;
    let checkpoint_bytes = fs::read(&checkpoint_path)
        .map_err(|_| "No checkpoint is available for this job.".to_owned())?;
    let checkpoint: CheckpointRecord = serde_json::from_slice(&checkpoint_bytes)
        .map_err(|error| format!("Could not decode checkpoint: {error}"))?;
    if checkpoint.job_id != record.job_id
        || checkpoint.source != record.source
        || checkpoint.output != record.output
    {
        return Err("Checkpoint identity does not match this persisted job.".into());
    }
    let (source_size, source_modified, source_fingerprint) =
        source_checkpoint_identity(std::path::Path::new(&record.source))?;
    let expected_size = checkpoint
        .source_size
        .ok_or("This checkpoint predates source fingerprinting and cannot be resumed safely.")?;
    let expected_fingerprint = checkpoint
        .source_fingerprint
        .as_deref()
        .ok_or("This checkpoint predates source fingerprinting and cannot be resumed safely.")?;
    if source_size != expected_size || source_fingerprint != expected_fingerprint {
        return Err("The checkpoint source recording was replaced or modified.".into());
    }
    if checkpoint.bytes_read > source_size {
        return Err("Checkpoint progress exceeds the current source size.".into());
    }
    if checkpoint.track_id != record.track_id {
        return Err("Checkpoint track selection does not match this persisted job.".into());
    }
    if checkpoint
        .source_modified
        .is_some_and(|expected| expected != source_modified)
    {
        record_diagnostic(
            &app,
            state.inner(),
            Some(&job_id),
            "warning",
            "job.resume.source_mtime_changed",
            "The source timestamp changed, but its size and sampled fingerprint still match.",
        );
    }
    record_diagnostic(
        &app,
        state.inner(),
        Some(&job_id),
        "info",
        "job.resume.full_replay",
        "Decoder state is not serializable; restarting from the trusted origin after validating the recording identity.",
    );
    mark_job_state(&app, state.inner(), Some(&job_id), JobState::Resuming);
    if let Err(error) = start_job_impl(app.clone(), state.inner(), &job_id) {
        mark_job_state(&app, state.inner(), Some(&job_id), JobState::Failed);
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_job(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    cancel_export_impl(state.inner())
}

#[tauri::command]
pub fn pause_queue(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state
        .queue_paused
        .lock()
        .map_err(|_| "Queue state is unavailable")? = true;
    let active = state
        .active_queue_job
        .lock()
        .map_err(|_| "Queue state is unavailable")?
        .clone();
    if let Some(job_id) = active {
        mark_job_state(&app, state.inner(), Some(&job_id), JobState::Pausing);
        send_control_impl(state.inner(), r#"{"type":"pause"}"#)?;
    }
    Ok(())
}

#[tauri::command]
pub fn resume_queue(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state
        .queue_paused
        .lock()
        .map_err(|_| "Queue state is unavailable")? = false;
    let active = state
        .active_queue_job
        .lock()
        .map_err(|_| "Queue state is unavailable")?
        .clone();
    if let Some(job_id) = active {
        let should_resume = state
            .jobs
            .lock()
            .map_err(|_| "Job state is unavailable")?
            .iter()
            .any(|job| {
                job.job_id == job_id && matches!(job.state, JobState::Paused | JobState::Pausing)
            });
        if should_resume {
            mark_job_state(&app, state.inner(), Some(&job_id), JobState::Resuming);
            send_control_impl(state.inner(), r#"{"type":"resume"}"#)?;
        }
    }
    run_queue_supervisor(app, state.inner().clone());
    Ok(())
}

#[tauri::command]
pub fn queue_is_paused(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    state
        .queue_paused
        .lock()
        .map(|value| *value)
        .map_err(|_| "Queue state is unavailable".into())
}
