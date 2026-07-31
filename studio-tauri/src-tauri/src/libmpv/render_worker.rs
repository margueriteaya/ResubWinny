use super::*;

/// The OpenGL route has a stricter threading contract than the normal libmpv
/// client API. This worker owns its WGL context, render context and player on
/// one dedicated thread; callers only exchange small control messages.
#[cfg(windows)]
pub struct LibMpvRenderWorker {
    sender: Sender<RenderWorkerMessage>,
    join: Option<JoinHandle<()>>,
    stats: Arc<Mutex<RenderWorkerStats>>,
}

#[cfg(windows)]
#[derive(Clone, Default)]
pub struct RenderWorkerStats {
    pub frames_presented: u64,
    pub presents_per_second: f64,
    pub caption_texture_uploads: u64,
    pub caption_texture_clears: u64,
    pub video_aspect: Option<f64>,
    pub surface_width: i32,
    pub surface_height: i32,
    pub decoder_mode: String,
    pub last_error: Option<String>,
}

#[cfg(all(windows, test))]
pub struct NativeRenderFrame {
    pub width: i32,
    pub height: i32,
    pub rgba: Vec<u8>,
}

#[cfg(windows)]
enum RenderWorkerMessage {
    Command(Vec<String>, Sender<Result<(), String>>),
    Time(Sender<Result<Option<f64>, String>>),
    Duration(Sender<Result<Option<f64>, String>>),
    Paused(Sender<Result<Option<bool>, String>>),
    StreamPosition(Sender<Result<Option<f64>, String>>),
    Resize(i32, i32),
    Caption(Option<CaptionOverlay>, Sender<Result<(), String>>),
    #[cfg(test)]
    Capture(Sender<Result<NativeRenderFrame, String>>),
    Stop,
}

#[cfg(windows)]
struct CaptionOverlay {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}

#[cfg(windows)]
impl LibMpvRenderWorker {
    pub fn start(
        library_path: PathBuf,
        source: PathBuf,
        hwnd: isize,
        width: i32,
        height: i32,
    ) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let stats = Arc::new(Mutex::new(RenderWorkerStats::default()));
        let worker_stats = stats.clone();
        let join = thread::Builder::new()
            .name("resubwinny-libmpv-render".into())
            .spawn(move || {
                render_worker_main(
                    library_path,
                    source,
                    hwnd,
                    width.max(1),
                    height.max(1),
                    receiver,
                    ready_sender,
                    worker_stats,
                )
            })
            .map_err(|error| format!("Could not start the native libmpv render thread: {error}"))?;
        match ready_receiver.recv_timeout(Duration::from_secs(8)) {
            Ok(Ok(())) => Ok(Self {
                sender,
                join: Some(join),
                stats,
            }),
            Ok(Err(error)) => {
                let _ = join.join();
                Err(error)
            }
            Err(_) => {
                let _ = sender.send(RenderWorkerMessage::Stop);
                let _ = join.join();
                Err("Timed out while creating the native libmpv render surface.".into())
            }
        }
    }

    pub fn command(&self, arguments: &[&str]) -> Result<(), String> {
        let (reply_sender, reply_receiver) = mpsc::channel();
        let values = arguments.iter().map(|value| (*value).to_owned()).collect();
        self.sender
            .send(RenderWorkerMessage::Command(values, reply_sender))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Timed out waiting for the native libmpv render thread.".to_string())?
    }

    pub fn time_seconds(&self) -> Result<Option<f64>, String> {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.sender
            .send(RenderWorkerMessage::Time(reply_sender))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "Timed out waiting for native player time.".to_string())?
    }

    pub fn duration_seconds(&self) -> Result<Option<f64>, String> {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.sender
            .send(RenderWorkerMessage::Duration(reply_sender))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "Timed out waiting for native player duration.".to_string())?
    }

    pub fn paused(&self) -> Result<Option<bool>, String> {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.sender
            .send(RenderWorkerMessage::Paused(reply_sender))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "Timed out waiting for native player pause state.".to_string())?
    }

    pub fn stream_position(&self) -> Result<Option<f64>, String> {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.sender
            .send(RenderWorkerMessage::StreamPosition(reply_sender))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "Timed out waiting for native player stream position.".to_string())?
    }

    pub fn resize(&self, width: i32, height: i32) {
        let _ = self
            .sender
            .send(RenderWorkerMessage::Resize(width.max(1), height.max(1)));
    }

    pub fn set_caption_overlay(
        &self,
        pixels: Vec<u8>,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
    ) -> Result<(), String> {
        if width <= 0 || height <= 0 || pixels.len() != width as usize * height as usize * 4 {
            return Err("Caption overlay pixel dimensions are invalid.".into());
        }
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.sender
            .send(RenderWorkerMessage::Caption(
                Some(CaptionOverlay {
                    pixels,
                    width,
                    height,
                    x,
                    y,
                }),
                reply_sender,
            ))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Timed out updating the native caption texture.".to_string())?
    }

    pub fn clear_caption_overlay(&self) -> Result<(), String> {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.sender
            .send(RenderWorkerMessage::Caption(None, reply_sender))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Timed out clearing the native caption texture.".to_string())?
    }

    #[cfg(test)]
    pub fn capture_frame(&self) -> Result<NativeRenderFrame, String> {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.sender
            .send(RenderWorkerMessage::Capture(reply_sender))
            .map_err(|_| "The native libmpv render thread has stopped.".to_string())?;
        reply_receiver
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "Timed out capturing the native libmpv frame.".to_string())?
    }

    pub fn stop(mut self) {
        let _ = self.sender.send(RenderWorkerMessage::Stop);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    pub fn diagnostics(&self) -> RenderWorkerStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }
}

#[cfg(windows)]
#[allow(
    clippy::too_many_arguments,
    reason = "the render thread receives its complete immutable startup state explicitly"
)]
fn render_worker_main(
    library_path: PathBuf,
    source: PathBuf,
    hwnd: isize,
    mut width: i32,
    mut height: i32,
    receiver: Receiver<RenderWorkerMessage>,
    ready: mpsc::SyncSender<Result<(), String>>,
    stats: Arc<Mutex<RenderWorkerStats>>,
) {
    let surface = match unsafe { crate::windows_gl::WglContext::create(hwnd) } {
        Ok(surface) => surface,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    let mut player = match unsafe {
        LibMpvPlayer::start_render(&library_path, &source, crate::windows_gl::get_proc_address)
    } {
        Ok(player) => player,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    // mpv internally coalesces unchanged frames. A bounded 60 Hz pump keeps
    // video smooth without allowing control calls to move GL work off-thread.
    let interval = Duration::from_millis(16);
    let mut running = true;
    let mut caption = None;
    let mut caption_dirty = false;
    #[cfg(test)]
    let mut pending_capture = None;
    let mut video_aspect = None;
    let mut ready_sent = false;
    let mut decoder_mode = "auto-safe (requested; not yet reported)".to_owned();
    let mut next_aspect_probe = std::time::Instant::now();
    let mut startup_frame_primed = false;
    let mut startup_prime_attempts = 0_u8;
    let mut next_startup_prime = std::time::Instant::now();
    let render_started = std::time::Instant::now();
    while running {
        match receiver.recv_timeout(interval) {
            Ok(RenderWorkerMessage::Command(arguments, reply)) => {
                let borrowed: Vec<_> = arguments.iter().map(String::as_str).collect();
                let _ = reply.send(player.command(&borrowed));
            }
            Ok(RenderWorkerMessage::Time(reply)) => {
                let _ = reply.send(player.time_seconds());
            }
            Ok(RenderWorkerMessage::Duration(reply)) => {
                let _ = reply.send(player.duration_seconds());
            }
            Ok(RenderWorkerMessage::Paused(reply)) => {
                let _ = reply.send(player.paused());
            }
            Ok(RenderWorkerMessage::StreamPosition(reply)) => {
                let _ = reply.send(player.stream_position());
            }
            Ok(RenderWorkerMessage::Resize(next_width, next_height)) => {
                width = next_width;
                height = next_height;
                caption_dirty = true;
                if let Ok(mut stats) = stats.lock() {
                    stats.surface_width = width;
                    stats.surface_height = height;
                }
            }
            Ok(RenderWorkerMessage::Caption(next_caption, reply)) => {
                let result = match next_caption {
                    Some(overlay) => crate::windows_gl::CaptionTexture::upload(
                        caption.as_mut(),
                        &overlay.pixels,
                        overlay.width,
                        overlay.height,
                        overlay.x,
                        overlay.y,
                    )
                    .map(|texture| caption = Some(texture)),
                    None => {
                        caption = None;
                        Ok(())
                    }
                };
                if result.is_ok() {
                    caption_dirty = true;
                    if let Ok(mut stats) = stats.lock() {
                        if caption.is_some() {
                            stats.caption_texture_uploads =
                                stats.caption_texture_uploads.saturating_add(1);
                        } else {
                            stats.caption_texture_clears =
                                stats.caption_texture_clears.saturating_add(1);
                        }
                    }
                }
                let _ = reply.send(result);
            }
            #[cfg(test)]
            Ok(RenderWorkerMessage::Capture(reply)) => {
                pending_capture = Some(reply);
                caption_dirty = true;
            }
            Ok(RenderWorkerMessage::Stop) | Err(RecvTimeoutError::Disconnected) => running = false,
            Err(RecvTimeoutError::Timeout) => {}
        }
        if !running {
            break;
        }
        if surface.make_current().is_err() {
            record_render_error(&stats, "Could not make the native WGL context current.");
            break;
        }
        if std::time::Instant::now() >= next_aspect_probe {
            video_aspect = player.video_aspect().ok().flatten();
            if let Some(actual) = player.hwdec_current() {
                decoder_mode = actual;
            }
            if let Ok(mut stats) = stats.lock() {
                stats.video_aspect = video_aspect;
            }
            next_aspect_probe = std::time::Instant::now() + Duration::from_millis(500);
        }
        // A libmpv render client opened in the paused state may expose media
        // metadata without decoding anything into the render target. Prime one
        // frame after the source is ready so the preview opens on video while
        // remaining paused and silent.
        if !startup_frame_primed
            && startup_prime_attempts < 20
            && std::time::Instant::now() >= next_startup_prime
            && player.duration_seconds().ok().flatten().is_some()
        {
            // Some broadcast recordings have audio/PES timestamps before the
            // first video GOP. Seeking to the normalised zero therefore leaves
            // a paused render client without a decodable video frame. A small
            // bounded probe lands inside the first GOP while retaining paused
            // startup; the UI can still seek to the exact requested position.
            let prime_result = player
                .command(&["seek", "1", "absolute+exact"])
                .and_then(|_| player.command(&["frame-step"]));
            match prime_result {
                Ok(()) => {
                    startup_frame_primed = true;
                    caption_dirty = true;
                }
                Err(_) => {
                    // Media metadata can become visible just before frame-step
                    // is accepted. Priming is a paused-start convenience, not
                    // a render failure: retry briefly and let the actual frame
                    // and health checks decide whether this source is usable.
                    startup_prime_attempts = startup_prime_attempts.saturating_add(1);
                    next_startup_prime = std::time::Instant::now() + Duration::from_millis(100);
                }
            }
        }
        match unsafe { player.render_frame(width, height, caption_dirty) } {
            Ok(true) => {
                if let Some(caption) = caption.as_ref()
                    && caption
                        .draw(crate::windows_gl::fit_video_viewport(
                            width,
                            height,
                            video_aspect,
                        ))
                        .is_err()
                {
                    record_render_error(&stats, "Could not blend the native caption texture.");
                    break;
                }
                if surface.swap_buffers().is_err() {
                    record_render_error(&stats, "Could not present the native libmpv frame.");
                    break;
                }
                unsafe { player.report_swap() };
                #[cfg(test)]
                if let Some(reply) = pending_capture.take() {
                    let _ = reply.send(surface.read_front_rgba(width, height).map(|rgba| {
                        NativeRenderFrame {
                            width,
                            height,
                            rgba,
                        }
                    }));
                }
                if let Ok(mut stats) = stats.lock() {
                    stats.frames_presented = stats.frames_presented.saturating_add(1);
                }
                if !ready_sent {
                    let _ = ready.send(Ok(()));
                    ready_sent = true;
                }
                caption_dirty = false;
            }
            Ok(false) => {}
            Err(error) => {
                if !ready_sent {
                    let _ = ready.send(Err(error.clone()));
                    ready_sent = true;
                }
                record_render_error(&stats, &error);
                break;
            }
        }
        if let Ok(mut stats) = stats.lock() {
            stats.surface_width = width;
            stats.surface_height = height;
            stats.decoder_mode = decoder_mode.clone();
            let seconds = render_started.elapsed().as_secs_f64();
            if seconds > 0.0 {
                stats.presents_per_second = stats.frames_presented as f64 / seconds;
            }
        }
    }
    let _ = surface.make_current();
    if !ready_sent {
        let _ = ready.send(Err(
            "The native preview stopped before presenting its first video frame.".into(),
        ));
    }
    unsafe { player.destroy_render_context() };
}

#[cfg(windows)]
fn record_render_error(stats: &Arc<Mutex<RenderWorkerStats>>, error: &str) {
    if let Ok(mut stats) = stats.lock() {
        stats.last_error = Some(error.into());
    }
}
