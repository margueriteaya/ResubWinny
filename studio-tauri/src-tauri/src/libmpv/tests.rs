use super::{LibMpv, LibMpvRenderWorker};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::HWND,
        System::{
            ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            Threading::GetCurrentProcess,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, SWP_NOACTIVATE, SetWindowPos, WS_POPUP,
        },
    },
    core::w,
};

#[test]
fn bundled_windows_runtime_exposes_the_required_client_abi() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/libmpv/windows-x86_64/libmpv-2.dll");
    let library = LibMpv::load(&path).expect("bundled libmpv client ABI");
    assert!(library.supports_render_api());
}

#[test]
#[ignore = "requires RESUBWINNY_RENDER_SMOKE_SOURCE and a Windows desktop OpenGL session"]
fn render_worker_starts_and_stops_on_a_real_recording() {
    let source = PathBuf::from(
        std::env::var_os("RESUBWINNY_RENDER_SMOKE_SOURCE")
            .expect("set RESUBWINNY_RENDER_SMOKE_SOURCE to a legal local recording"),
    );
    assert!(source.is_file(), "render smoke source must be a file");
    let host = unsafe {
        CreateWindowExW(
            Default::default(),
            w!("STATIC"),
            w!("ResubWinnyRenderSmoke"),
            WS_POPUP,
            -10_000,
            -10_000,
            640,
            360,
            None,
            None,
            None,
            None,
        )
    }
    .expect("create hidden native render host");
    let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/libmpv/windows-x86_64/libmpv-2.dll");
    let worker = LibMpvRenderWorker::start(runtime, source, host.0 as isize, 640, 360)
        .expect("start native libmpv render worker");
    let mut media_ready = false;
    for _ in 0..40 {
        if worker.duration_seconds().ok().flatten().is_some() {
            media_ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(media_ready, "libmpv did not finish opening the recording");
    let startup_frame = wait_for_non_uniform_frame(&worker, Duration::from_secs(5));
    if startup_frame.is_none() {
        let stats = worker.diagnostics();
        eprintln!(
            "startup diagnostics: frames={}, fps={}, mode={}, error={:?}, aspect={:?}",
            stats.frames_presented,
            stats.presents_per_second,
            stats.decoder_mode,
            stats.last_error,
            stats.video_aspect
        );
    }
    let startup_frame =
        startup_frame.expect("paused startup did not produce a decoded first frame");
    assert_eq!((startup_frame.width, startup_frame.height), (640, 360));
    worker
        .command(&["seek", "30", "absolute+exact"])
        .expect("seek to programme video");
    wait_for_non_uniform_frame(&worker, Duration::from_secs(5))
        .expect("explicit seek did not produce a decoded video frame");
    let mut overlay = vec![0; 16 * 16 * 4];
    for (row, pixels) in overlay.chunks_exact_mut(16 * 4).enumerate() {
        for pixel in pixels.chunks_exact_mut(4) {
            // Use an asymmetric top-down BGRA marker so this gate catches a
            // vertically inverted native caption texture. Top is red; bottom
            // is blue.
            pixel.copy_from_slice(if row < 8 {
                &[255, 0, 0, 255]
            } else {
                &[0, 0, 255, 255]
            });
        }
    }
    worker
        .set_caption_overlay(overlay.into(), 16, 16, 0, 0)
        .expect("upload native caption texture");
    unsafe {
        SetWindowPos(
            host,
            Some(HWND(std::ptr::null_mut())),
            -10_000,
            -10_000,
            1920,
            1080,
            SWP_NOACTIVATE,
        )
    }
    .expect("resize native render host");
    worker.resize(1920, 1080);
    thread::sleep(Duration::from_millis(100));
    let frame = worker.capture_frame().expect("capture native render frame");
    assert_eq!((frame.width, frame.height), (1920, 1080));
    // glReadPixels returns the framebuffer bottom row first. Compare marker
    // centroids instead of sampling the outermost rows because the video's
    // display aspect may put the caption plane inside a letter/pillarbox.
    let red_row = marker_row_centroid(&frame.rgba, frame.width, |pixel| {
        pixel[0] > 240 && pixel[1] < 8 && pixel[2] < 8 && pixel[3] > 240
    })
    .expect("native caption texture lost its red top marker");
    let blue_row = marker_row_centroid(&frame.rgba, frame.width, |pixel| {
        pixel[0] < 8 && pixel[1] < 8 && pixel[2] > 240 && pixel[3] > 240
    })
    .expect("native caption texture lost its blue bottom marker");
    assert!(
        red_row > blue_row,
        "native caption texture is vertically inverted: red top row {red_row:.1}, blue bottom row {blue_row:.1}"
    );
    worker
        .clear_caption_overlay()
        .expect("clear native caption texture");
    unsafe {
        SetWindowPos(
            host,
            Some(HWND(std::ptr::null_mut())),
            -10_000,
            -10_000,
            3840,
            2160,
            SWP_NOACTIVATE,
        )
    }
    .expect("resize native render host to 4K");
    worker.resize(3840, 2160);
    thread::sleep(Duration::from_millis(500));
    let diagnostics = worker.diagnostics();
    worker.stop();
    unsafe { DestroyWindow(HWND(host.0)) }.expect("destroy render host");
    assert!(
        diagnostics.frames_presented > 0,
        "native render worker did not present a video frame"
    );
    assert_eq!(
        diagnostics.caption_texture_uploads, 1,
        "native caption texture was not uploaded exactly once"
    );
    assert_eq!(
        diagnostics.caption_texture_clears, 1,
        "native caption texture was not cleared exactly once"
    );
    assert_eq!(
        (diagnostics.surface_width, diagnostics.surface_height),
        (3840, 2160),
        "native render worker did not retain the 4K surface dimensions"
    );
    assert!(
        diagnostics.presents_per_second > 1.0,
        "native render worker did not maintain measurable presentation cadence: {} fps",
        diagnostics.presents_per_second
    );
    assert!(
        diagnostics.last_error.is_none(),
        "native render worker reported: {:?}",
        diagnostics.last_error
    );
}

fn marker_row_centroid(rgba: &[u8], width: i32, matches: impl Fn(&[u8]) -> bool) -> Option<f64> {
    let row_bytes = usize::try_from(width).ok()?.checked_mul(4)?;
    let mut row_sum = 0_u64;
    let mut count = 0_u64;
    for (row, pixels) in rgba.chunks_exact(row_bytes).enumerate() {
        for pixel in pixels.chunks_exact(4) {
            if matches(pixel) {
                row_sum = row_sum.saturating_add(row as u64);
                count = count.saturating_add(1);
            }
        }
    }
    (count > 0).then(|| row_sum as f64 / count as f64)
}

#[test]
#[ignore = "requires a legal 4K recording and a Windows desktop OpenGL session"]
fn render_worker_meets_the_long_4k_performance_gate() {
    let source = smoke_source();
    let gate_seconds = env_u64("RESUBWINNY_RENDER_GATE_SECONDS", 120).max(10);
    let minimum_fps = env_f64("RESUBWINNY_RENDER_GATE_MIN_FPS", 20.0);
    let maximum_rss_mib = env_f64("RESUBWINNY_RENDER_GATE_MAX_RSS_MIB", 2048.0);
    let maximum_growth_mib = env_f64("RESUBWINNY_RENDER_GATE_MAX_GROWTH_MIB", 512.0);
    let maximum_startup_ms = env_f64("RESUBWINNY_RENDER_GATE_MAX_STARTUP_MS", 10_000.0);
    let maximum_control_ms = env_f64("RESUBWINNY_RENDER_GATE_MAX_CONTROL_MS", 1_000.0);
    let maximum_overlay_ms = env_f64("RESUBWINNY_RENDER_GATE_MAX_OVERLAY_MS", 1_000.0);
    let maximum_shutdown_ms = env_f64("RESUBWINNY_RENDER_GATE_MAX_SHUTDOWN_MS", 3_000.0);

    let host = NativeHost::create(1920, 1080, w!("ResubWinnyRenderPerformance"));
    let runtime = bundled_runtime();
    let startup_started = Instant::now();
    let worker = LibMpvRenderWorker::start(runtime, source.clone(), host.0.0 as isize, 1920, 1080)
        .expect("start native libmpv render worker");
    let startup_ms = startup_started.elapsed().as_secs_f64() * 1_000.0;

    wait_for_media(&worker, Duration::from_secs(5));
    worker
        .command(&["set", "volume", "0"])
        .expect("mute performance gate");
    worker
        .command(&["seek", "30", "absolute+exact"])
        .expect("seek performance gate to programme video");
    resize_host(&host, 3840, 2160);
    worker.resize(3840, 2160);
    thread::sleep(Duration::from_secs(2));
    let baseline_rss_bytes = current_working_set_bytes().unwrap_or(0);

    let play_started =
        timed_command(&worker, &["set", "pause", "no"]).expect("start performance playback");
    let mut maximum_observed_control_ms = play_started;
    let starting_frames = worker.diagnostics().frames_presented;
    let measurement_started = Instant::now();
    let mut paused_seconds = 0.0;
    let mut peak_rss_bytes = baseline_rss_bytes;
    let mut maximum_observed_overlay_ms = 0.0_f64;
    let overlay_moments = [gate_seconds / 4, gate_seconds / 2, gate_seconds * 3 / 4];
    let mut next_overlay = 0usize;
    let mut pause_exercised = false;

    while measurement_started.elapsed() < Duration::from_secs(gate_seconds) {
        let elapsed = measurement_started.elapsed().as_secs();
        peak_rss_bytes = peak_rss_bytes.max(current_working_set_bytes().unwrap_or(0));

        if next_overlay < overlay_moments.len() && elapsed >= overlay_moments[next_overlay] {
            let overlay_started = Instant::now();
            worker
                .set_caption_overlay(
                    performance_overlay(next_overlay as u8).into(),
                    1920,
                    1080,
                    0,
                    0,
                )
                .expect("update full native caption plane");
            maximum_observed_overlay_ms =
                maximum_observed_overlay_ms.max(overlay_started.elapsed().as_secs_f64() * 1_000.0);
            next_overlay += 1;
        }

        if !pause_exercised && elapsed >= gate_seconds / 2 {
            maximum_observed_control_ms = maximum_observed_control_ms.max(
                timed_command(&worker, &["set", "pause", "yes"]).expect("pause performance gate"),
            );
            let pause_started = Instant::now();
            thread::sleep(Duration::from_millis(750));
            maximum_observed_control_ms = maximum_observed_control_ms.max(
                timed_command(&worker, &["set", "pause", "no"]).expect("resume performance gate"),
            );
            maximum_observed_control_ms = maximum_observed_control_ms.max(
                timed_command(&worker, &["seek", "5", "relative+exact"])
                    .expect("seek during performance gate"),
            );
            paused_seconds += pause_started.elapsed().as_secs_f64();
            pause_exercised = true;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let diagnostics = worker.diagnostics();
    let measured_seconds = measurement_started.elapsed().as_secs_f64();
    let active_seconds = (measured_seconds - paused_seconds).max(0.001);
    let measured_frames = diagnostics.frames_presented.saturating_sub(starting_frames);
    let measured_fps = measured_frames as f64 / active_seconds;
    let peak_rss_mib = peak_rss_bytes as f64 / (1024.0 * 1024.0);
    let baseline_rss_mib = baseline_rss_bytes as f64 / (1024.0 * 1024.0);
    let working_set_growth_mib = peak_rss_mib - baseline_rss_mib;
    worker
        .clear_caption_overlay()
        .expect("clear performance caption plane");
    let shutdown_started = Instant::now();
    worker.stop();
    let shutdown_ms = shutdown_started.elapsed().as_secs_f64() * 1_000.0;

    let failures = [
        (startup_ms > maximum_startup_ms).then(|| {
            format!("startup {startup_ms:.1} ms exceeded {maximum_startup_ms:.1} ms")
        }),
        (measured_fps < minimum_fps)
            .then(|| format!("cadence {measured_fps:.2} fps was below {minimum_fps:.2} fps")),
        (peak_rss_mib > maximum_rss_mib).then(|| {
            format!("peak RSS {peak_rss_mib:.1} MiB exceeded {maximum_rss_mib:.1} MiB")
        }),
        (working_set_growth_mib > maximum_growth_mib).then(|| {
            format!(
                "working-set growth {working_set_growth_mib:.1} MiB exceeded {maximum_growth_mib:.1} MiB"
            )
        }),
        (maximum_observed_control_ms > maximum_control_ms).then(|| {
            format!(
                "control latency {maximum_observed_control_ms:.1} ms exceeded {maximum_control_ms:.1} ms"
            )
        }),
        (maximum_observed_overlay_ms > maximum_overlay_ms).then(|| {
            format!(
                "caption upload {maximum_observed_overlay_ms:.1} ms exceeded {maximum_overlay_ms:.1} ms"
            )
        }),
        (shutdown_ms > maximum_shutdown_ms).then(|| {
            format!("shutdown {shutdown_ms:.1} ms exceeded {maximum_shutdown_ms:.1} ms")
        }),
        ((diagnostics.surface_width, diagnostics.surface_height) != (3840, 2160))
            .then(|| "native surface did not remain 3840x2160".to_owned()),
        diagnostics
            .last_error
            .as_ref()
            .map(|error| format!("native render error: {error}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let report = json!({
        "schemaVersion": 1,
        "route": "libmpv-render",
        "source": source,
        "surface": { "width": diagnostics.surface_width, "height": diagnostics.surface_height },
        "decoderMode": diagnostics.decoder_mode,
        "durationSeconds": measured_seconds,
        "activeSeconds": active_seconds,
        "framesPresented": measured_frames,
        "presentsPerSecond": measured_fps,
        "startupMs": startup_ms,
        "maximumControlLatencyMs": maximum_observed_control_ms,
        "maximumCaptionUploadMs": maximum_observed_overlay_ms,
        "baselineWorkingSetMiB": baseline_rss_mib,
        "peakWorkingSetMiB": peak_rss_mib,
        "workingSetGrowthMiB": working_set_growth_mib,
        "shutdownMs": shutdown_ms,
        "thresholds": {
            "minimumPresentsPerSecond": minimum_fps,
            "maximumStartupMs": maximum_startup_ms,
            "maximumControlLatencyMs": maximum_control_ms,
            "maximumCaptionUploadMs": maximum_overlay_ms,
            "maximumPeakWorkingSetMiB": maximum_rss_mib,
            "maximumWorkingSetGrowthMiB": maximum_growth_mib,
            "maximumShutdownMs": maximum_shutdown_ms
        },
        "passed": failures.is_empty(),
        "failures": failures
    });
    let report_text = serde_json::to_string_pretty(&report).expect("serialise preview report");
    println!("{report_text}");
    if let Some(path) = std::env::var_os("RESUBWINNY_RENDER_GATE_REPORT") {
        fs::write(path, &report_text).expect("write preview performance report");
    }
    assert!(
        failures.is_empty(),
        "native 4K preview performance gate failed: {}",
        failures.join("; ")
    );
}

struct NativeHost(HWND);

impl NativeHost {
    fn create(width: i32, height: i32, title: windows::core::PCWSTR) -> Self {
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                title,
                WS_POPUP,
                -10_000,
                -10_000,
                width,
                height,
                None,
                None,
                None,
                None,
            )
        }
        .expect("create hidden native render host");
        Self(hwnd)
    }
}

impl Drop for NativeHost {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

fn smoke_source() -> PathBuf {
    let source = PathBuf::from(
        std::env::var_os("RESUBWINNY_RENDER_SMOKE_SOURCE")
            .expect("set RESUBWINNY_RENDER_SMOKE_SOURCE to a legal local recording"),
    );
    assert!(source.is_file(), "render smoke source must be a file");
    source
}

fn bundled_runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/libmpv/windows-x86_64/libmpv-2.dll")
}

fn wait_for_media(worker: &LibMpvRenderWorker, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if worker.duration_seconds().ok().flatten().is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("libmpv did not finish opening the recording");
}

fn resize_host(host: &NativeHost, width: i32, height: i32) {
    unsafe {
        SetWindowPos(
            host.0,
            Some(HWND(std::ptr::null_mut())),
            -10_000,
            -10_000,
            width,
            height,
            SWP_NOACTIVATE,
        )
    }
    .expect("resize native render host");
}

fn timed_command(worker: &LibMpvRenderWorker, arguments: &[&str]) -> Result<f64, String> {
    let started = Instant::now();
    worker.command(arguments)?;
    Ok(started.elapsed().as_secs_f64() * 1_000.0)
}

fn performance_overlay(seed: u8) -> Vec<u8> {
    let mut pixels = vec![0_u8; 1920 * 1080 * 4];
    let colour = [32_u8.saturating_add(seed * 24), 220, 240, 176];
    for y in 760..920 {
        for x in 320..1600 {
            let offset = (y * 1920 + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&colour);
        }
    }
    pixels
}

fn current_working_set_bytes() -> Option<usize> {
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    }
    .ok()?;
    Some(counters.WorkingSetSize)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}

fn wait_for_non_uniform_frame(
    worker: &LibMpvRenderWorker,
    timeout: Duration,
) -> Option<super::render_worker::NativeRenderFrame> {
    let started = std::time::Instant::now();
    let mut last_counts = (0, 0, 0, 0);
    while started.elapsed() < timeout {
        if let Ok(frame) = worker.capture_frame() {
            let mut dark_pixels = 0usize;
            let mut bright_pixels = 0usize;
            let mut min_luminance = u16::MAX;
            let mut max_luminance = 0_u16;
            for pixel in frame.rgba.chunks_exact(4).step_by(32) {
                let luminance = (u16::from(pixel[0]) * 54
                    + u16::from(pixel[1]) * 183
                    + u16::from(pixel[2]) * 19)
                    / 256;
                min_luminance = min_luminance.min(luminance);
                max_luminance = max_luminance.max(luminance);
                dark_pixels += usize::from(luminance < 16);
                bright_pixels += usize::from(luminance > 32);
            }
            last_counts = (dark_pixels, bright_pixels, min_luminance, max_luminance);
            if bright_pixels > 100
                && max_luminance.saturating_sub(min_luminance) > 16
                && max_luminance > 32
            {
                return Some(frame);
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    eprintln!(
        "last captured framebuffer sample: dark={}, bright={}, min={}, max={}",
        last_counts.0, last_counts.1, last_counts.2, last_counts.3
    );
    None
}
