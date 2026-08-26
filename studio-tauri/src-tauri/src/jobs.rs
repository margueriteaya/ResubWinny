use crate::export::{cancel_export_impl, send_control_impl, start_export_impl};
use crate::models::{
    ArtifactRecord, CheckpointRecord, DiagnosticRecord, DrcsMappingInput, ExportPreservation,
    ExportSelection, TaskHistoryRecord,
};
use crate::state::AppState;
use crate::storage::write_atomic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{self, File},
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};

static JOB_ID_SEQUENCE: AtomicU64 = AtomicU64::new(1);

mod repository;
mod supervisor;

pub use repository::*;
pub use supervisor::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum JobState {
    Created,
    Inspecting,
    Ready,
    Queued,
    Starting,
    Running,
    Pausing,
    Paused,
    Resuming,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub job_id: String,
    pub source: String,
    pub output: String,
    pub archive: bool,
    pub raw: bool,
    #[serde(default)]
    pub track_id: Option<u16>,
    #[serde(default)]
    pub drcs_report: bool,
    #[serde(default)]
    pub drcs_mappings: Vec<DrcsMappingInput>,
    #[serde(default)]
    pub export_selection: ExportSelection,
    pub state: JobState,
    pub created_at: u64,
    pub updated_at: u64,
}
