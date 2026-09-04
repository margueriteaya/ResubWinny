use serde::Serialize;
use std::io::{self, BufRead, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

pub(crate) fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

static EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub(crate) struct WorkerControl {
    pub(crate) cancelled: AtomicBool,
    pub(crate) paused: AtomicBool,
    pause_reported: AtomicBool,
}

impl WorkerControl {
    pub(crate) fn wait_if_paused(&self) -> bool {
        if self.cancelled.load(Ordering::Relaxed) {
            return true;
        }
        if self.paused.load(Ordering::Relaxed) && !self.pause_reported.swap(true, Ordering::Relaxed)
        {
            emit_json(&serde_json::json!({"type": "paused"}));
        }
        while self.paused.load(Ordering::Relaxed) && !self.cancelled.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if self.pause_reported.swap(false, Ordering::Relaxed)
            && !self.cancelled.load(Ordering::Relaxed)
        {
            emit_json(&serde_json::json!({"type": "resumed"}));
        }
        self.cancelled.load(Ordering::Relaxed)
    }
}

pub(crate) fn spawn_control_listener(control: Arc<WorkerControl>) {
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            match message.get("type").and_then(|value| value.as_str()) {
                Some("pause") => control.paused.store(true, Ordering::Relaxed),
                Some("resume") => control.paused.store(false, Ordering::Relaxed),
                Some("cancel") => {
                    control.cancelled.store(true, Ordering::Relaxed);
                    control.paused.store(false, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    });
}

pub(crate) fn versioned_event(
    value: &impl Serialize,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let mut value = match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(object)) => object,
        Ok(other) => {
            let mut object = serde_json::Map::new();
            object.insert("value".into(), other);
            object
        }
        Err(_) => return None,
    };
    let payload = serde_json::Value::Object(value.clone());
    value.insert("protocolVersion".into(), serde_json::json!(1));
    let job_id = std::env::var("RESUBWINNY_JOB_ID")
        .unwrap_or_else(|_| format!("worker-{}", std::process::id()));
    value.insert("jobId".into(), serde_json::json!(job_id));
    value.insert(
        "sequence".into(),
        serde_json::json!(EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed)),
    );
    value.insert("payload".into(), payload);
    Some(value)
}

pub(crate) fn emit_json(value: &impl Serialize) {
    let Some(value) = versioned_event(value) else {
        return;
    };
    let mut stdout = io::stdout().lock();
    let _ = write_json_line(&mut stdout, &value);
}

pub(crate) fn emit_hello(command: &str) {
    emit_json(&serde_json::json!({
        "type": "hello",
        "command": command,
        "controls": ["pause", "resume", "cancel"],
        "capabilities": [
            "mpeg_ts_b24_verified",
            "mpeg_ts_192_ttml_verified",
            "mpeg_ts_ttml_candidate",
            "tlv_mmtp_experimental"
        ]
    }));
}

pub(crate) fn emit_stage(stage: &str) {
    emit_json(&serde_json::json!({"type": "stage-changed", "stage": stage}));
}

pub(crate) fn emit_failed(code: &str, message: &str) {
    emit_json(&serde_json::json!({
        "type": "failed",
        "code": code,
        "message": message,
        "parameters": {}
    }));
}

pub(crate) fn emit_export_conflict(conflict: &crate::ExportConflict) {
    emit_json(&serde_json::json!({
        "type": "failed",
        "code": "export_conflict",
        "message": conflict.to_string(),
        "parameters": conflict
    }));
}
