use crate::jobs::JobRecord;
use crate::models::{BroadcastMetadata, DiagnosticRecord, PlaybackTimeMapping};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    process::Child,
    sync::{Arc, Mutex, atomic::AtomicBool},
};

pub struct AppState {
    pub child: Mutex<Option<Child>>,
    pub player: Mutex<Option<PlayerHost>>,
    pub caption_font: Mutex<String>,
    pub preview_overlay_sync: Mutex<PreviewOverlaySyncState>,
    pub preview_broadcast_cache: Mutex<Option<PreviewBroadcastCache>>,
    pub playback_time_mapping: Mutex<PlaybackTimeMapping>,
    pub jobs: Mutex<Vec<JobRecord>>,
    pub diagnostics: Mutex<HashMap<String, Vec<DiagnosticRecord>>>,
    pub queue_paused: Mutex<bool>,
    pub queue: Mutex<VecDeque<String>>,
    pub active_queue_job: Mutex<Option<String>>,
    pub supervisor_running: AtomicBool,
    pub forced_cancel: AtomicBool,
}

#[derive(Default)]
pub struct PreviewOverlaySyncState {
    pub archive: String,
    pub fingerprint: Option<u64>,
    pub overlay_visible: bool,
    pub revision: u64,
}

#[derive(Clone)]
pub struct PreviewBroadcastCache {
    pub source: PathBuf,
    pub offset_bucket: u64,
    pub service_id: Option<u16>,
    pub metadata: BroadcastMetadata,
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            player: Mutex::new(None),
            caption_font: Mutex::new("arib".into()),
            preview_overlay_sync: Mutex::new(PreviewOverlaySyncState::default()),
            preview_broadcast_cache: Mutex::new(None),
            playback_time_mapping: Mutex::new(PlaybackTimeMapping::default()),
            jobs: Mutex::new(Vec::new()),
            diagnostics: Mutex::new(HashMap::new()),
            queue_paused: Mutex::new(false),
            queue: Mutex::new(VecDeque::new()),
            active_queue_job: Mutex::new(None),
            supervisor_running: AtomicBool::new(false),
            forced_cancel: AtomicBool::new(false),
        }
    }
}

#[cfg(windows)]
pub struct PlayerHost {
    pub host: isize,
    pub app: tauri::AppHandle,
    pub frame_pool: Arc<crate::preview::frame_bridge::PreviewFramePool>,
    pub source: PathBuf,
    pub player: NativePlayer,
}

#[cfg(windows)]
pub struct NativePlayer(pub(crate) crate::libmpv::LibMpvRenderWorker);

#[cfg(windows)]
impl NativePlayer {
    pub fn command(&self, arguments: &[&str]) -> Result<(), String> {
        self.0.command(arguments)
    }

    pub fn time_seconds(&self) -> Result<Option<f64>, String> {
        self.0.time_seconds()
    }

    pub fn duration_seconds(&self) -> Result<Option<f64>, String> {
        self.0.duration_seconds()
    }

    pub fn paused(&self) -> Result<Option<bool>, String> {
        self.0.paused()
    }

    pub fn stream_position(&self) -> Result<Option<f64>, String> {
        self.0.stream_position()
    }

    pub fn resize(&self, width: i32, height: i32) {
        self.0.resize(width, height);
    }

    pub fn set_caption_overlay(
        &self,
        pixels: Arc<[u8]>,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
    ) -> Result<(), String> {
        self.0.set_caption_overlay(pixels, width, height, x, y)
    }

    pub fn clear_caption_overlay(&self) -> Result<(), String> {
        self.0.clear_caption_overlay()
    }

    pub fn stop(self) {
        self.0.stop();
    }

    pub fn render_diagnostics(&self) -> crate::libmpv::RenderWorkerStats {
        self.0.diagnostics()
    }
}
#[cfg(not(windows))]
pub struct PlayerHost;
