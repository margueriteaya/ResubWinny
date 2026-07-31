use crate::models::TaskEvent;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone)]
pub struct TaskEventEmitter {
    app: AppHandle,
    job_id: Option<String>,
}

impl TaskEventEmitter {
    pub fn new(app: AppHandle, job_id: Option<String>) -> Self {
        Self { app, job_id }
    }

    pub fn emit(
        &self,
        kind: &str,
        message: impl Into<String>,
        bytes_read: Option<u64>,
        captions: Option<u64>,
        warnings: Option<u64>,
        output: Option<String>,
    ) {
        self.emit_with_details(
            kind,
            format!("task.{kind}"),
            BTreeMap::new(),
            message,
            bytes_read,
            captions,
            warnings,
            output,
        );
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the fields mirror the stable worker event contract"
    )]
    pub fn emit_with_details(
        &self,
        kind: &str,
        code: impl Into<String>,
        parameters: BTreeMap<String, Value>,
        message: impl Into<String>,
        bytes_read: Option<u64>,
        captions: Option<u64>,
        warnings: Option<u64>,
        output: Option<String>,
    ) {
        let _ = self.app.emit(
            "task-event",
            TaskEvent {
                job_id: self.job_id.clone(),
                kind: kind.into(),
                code: code.into(),
                parameters,
                message: message.into(),
                bytes_read,
                captions,
                warnings,
                output,
            },
        );
    }
}

pub fn worker_path(app: Option<&AppHandle>) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("RESUBWINNY_WORKER") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }
    if let Some(app) = app
        && let Ok(resource_dir) = app.path().resource_dir()
    {
        let candidate = resource_dir.join("arib-caption-worker.exe");
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for candidate in [
        root.join("build/cargo/release/arib-caption-worker.exe"),
        root.join("build/cargo/debug/arib-caption-worker.exe"),
    ] {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("ARIB worker was not found. Build crates/arib-caption-worker first, or set RESUBWINNY_WORKER.".into())
}
