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
    pub owner: isize,
    pub source: PathBuf,
    pub player: NativePlayer,
    pub overlay_path: PathBuf,
    pub render_fallback_reason: Option<String>,
}

#[cfg(windows)]
pub enum NativePlayer {
    Client(crate::libmpv::LibMpvPlayer),
    Render(crate::libmpv::LibMpvRenderWorker),
}

#[cfg(windows)]
impl NativePlayer {
    pub fn command(&self, arguments: &[&str]) -> Result<(), String> {
        match self {
            Self::Client(player) => player.command(arguments),
            Self::Render(worker) => worker.command(arguments),
        }
    }

    pub fn time_seconds(&self) -> Result<Option<f64>, String> {
        match self {
            Self::Client(player) => player.time_seconds(),
            Self::Render(worker) => worker.time_seconds(),
        }
    }

    pub fn duration_seconds(&self) -> Result<Option<f64>, String> {
        match self {
            Self::Client(player) => player.duration_seconds(),
            Self::Render(worker) => worker.duration_seconds(),
        }
    }

    pub fn paused(&self) -> Result<Option<bool>, String> {
        match self {
            Self::Client(player) => player.paused(),
            Self::Render(worker) => worker.paused(),
        }
    }

    pub fn stream_position(&self) -> Result<Option<f64>, String> {
        match self {
            Self::Client(player) => player.stream_position(),
            Self::Render(worker) => worker.stream_position(),
        }
    }

    pub fn resize(&self, width: i32, height: i32) {
        if let Self::Render(worker) = self {
            worker.resize(width, height);
        }
    }

    pub fn osd_dimensions(&self) -> Result<Option<(i32, i32)>, String> {
        match self {
            Self::Client(player) => player.osd_dimensions(),
            // The render route maps the complete logical caption plane to the
            // display-aspect-correct video viewport on its WGL thread.
            Self::Render(_) => Ok(None),
        }
    }

    pub fn is_render(&self) -> bool {
        matches!(self, Self::Render(_))
    }

    pub fn set_caption_overlay(
        &self,
        pixels: Arc<[u8]>,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
    ) -> Result<(), String> {
        match self {
            Self::Client(_) => {
                Err("The client preview does not accept native texture uploads.".into())
            }
            Self::Render(worker) => worker.set_caption_overlay(pixels, width, height, x, y),
        }
    }

    pub fn clear_caption_overlay(&self) -> Result<(), String> {
        match self {
            Self::Client(_) => {
                Err("The client preview does not own a native caption texture.".into())
            }
            Self::Render(worker) => worker.clear_caption_overlay(),
        }
    }

    pub fn stop(self) {
        if let Self::Render(worker) = self {
            worker.stop();
        }
    }

    pub fn render_diagnostics(&self) -> Option<crate::libmpv::RenderWorkerStats> {
        match self {
            Self::Client(_) => None,
            Self::Render(worker) => Some(worker.diagnostics()),
        }
    }
}
#[cfg(not(windows))]
pub struct PlayerHost;
