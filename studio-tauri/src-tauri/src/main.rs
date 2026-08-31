mod about;
#[path = "../../../shared/arib_symbols.rs"]
mod arib_symbols;
#[path = "../../../shared/caption_features.rs"]
mod caption_features;
mod caption_renderer;
mod drcs;
mod export;
mod inspection;
mod jobs;
// Playback and caption composition stay behind the native backend API. The
// WebView only forwards typed controls and displays bounded state.
#[allow(dead_code)]
mod libmpv;
mod models;
mod preview;
mod preview_surface;
mod settings;
mod state;
mod storage;
mod timeline;
#[cfg(windows)]
#[allow(dead_code)]
mod windows_gl;
mod worker;

use std::sync::Arc;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(state::AppState::default()))
        .setup(|app| {
            let state = app.state::<Arc<state::AppState>>();
            jobs::load_persisted_jobs(app.handle(), state.inner());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            about::get_about_info,
            about::open_project_link,
            inspection::inspect_source,
            jobs::create_job,
            jobs::default_output_path,
            jobs::list_jobs,
            jobs::load_task_history,
            jobs::save_task_history,
            jobs::get_job,
            jobs::get_job_diagnostics,
            jobs::get_job_diagnostics_window,
            jobs::list_jobs_window,
            jobs::get_job_artifacts,
            jobs::get_job_checkpoint,
            jobs::remove_job,
            jobs::start_job,
            jobs::enqueue_jobs,
            jobs::pause_job,
            jobs::resume_job,
            jobs::cancel_job,
            jobs::pause_queue,
            jobs::resume_queue,
            jobs::queue_is_paused,
            export::start_export,
            export::start_preview_index,
            export::cancel_export,
            export::cancel_export_and_wait,
            export::pause_export,
            export::resume_export,
            preview::set_caption_font,
            preview::get_preview_capabilities,
            preview::get_preview_runtime,
            preview::get_preview_render_diagnostics,
            preview::overlay::clear_caption_overlay,
            preview::overlay::get_preview_time,
            preview::overlay::get_preview_duration,
            preview::overlay::get_preview_playback_state,
            preview::overlay::get_preview_broadcast_metadata,
            preview::overlay::get_playback_time_mapping,
            preview::overlay::update_playback_time_mapping,
            preview::archive::render_at,
            preview::overlay::render_preview_at,
            preview::overlay::sync_preview_overlay,
            settings::get_settings,
            settings::update_settings,
            settings::list_language_packs,
            settings::open_language_pack_directory,
            drcs::load_drcs_report,
            drcs::load_drcs_mappings,
            drcs::save_drcs_mappings,
            preview::start_preview,
            preview::recover_preview,
            preview::resize_preview,
            preview::stop_preview,
            preview::preview_command,
            preview::seek_preview_project,
            timeline::get_timeline_window,
            timeline::get_timeline_window_filtered,
            timeline::get_timeline_recent_window_filtered,
            timeline::get_timeline_time_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running ResubWinny");
}
