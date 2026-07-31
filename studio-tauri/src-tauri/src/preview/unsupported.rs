use super::*;
pub fn caption_overlay(
    _: State<'_, Arc<AppState>>,
    _: Vec<u8>,
    _: i32,
    _: i32,
    _: i32,
    _: i32,
) -> Result<(), String> {
    Err("mpv caption overlay is not implemented for this platform yet.".into())
}
pub fn clear_caption_overlay(_: State<'_, Arc<AppState>>) -> Result<(), String> {
    Err("mpv caption overlay is not implemented for this platform yet.".into())
}
pub fn preview_time(_: State<'_, Arc<AppState>>) -> Result<Option<f64>, String> {
    Err("mpv preview time is not implemented for this platform yet.".into())
}
pub fn preview_duration(_: State<'_, Arc<AppState>>) -> Result<Option<f64>, String> {
    Err("mpv preview duration is not implemented for this platform yet.".into())
}
pub fn preview_playback_state(
    _: State<'_, Arc<AppState>>,
) -> Result<crate::models::PreviewPlaybackState, String> {
    Err("mpv preview state is not implemented for this platform yet.".into())
}
pub fn preview_broadcast_metadata(
    _: AppHandle,
    _: State<'_, Arc<AppState>>,
    _: Option<u16>,
) -> Result<crate::models::BroadcastMetadata, String> {
    Err("mpv broadcast metadata is not implemented for this platform yet.".into())
}
#[tauri::command]
pub fn start_preview(
    _: AppHandle,
    _: State<'_, Arc<AppState>>,
    _: String,
    _: PreviewRect,
) -> Result<(), String> {
    Err("Native mpv embedding is not implemented for this platform yet.".into())
}
pub fn recover_preview(
    _: AppHandle,
    _: State<'_, Arc<AppState>>,
    _: String,
    _: PreviewRect,
    _: Option<f64>,
    _: bool,
    _: f64,
) -> Result<(), String> {
    Err("Native mpv recovery is not implemented for this platform yet.".into())
}
#[tauri::command]
pub fn resize_preview(_: State<'_, Arc<AppState>>, _: PreviewRect) -> Result<(), String> {
    Ok(())
}
#[tauri::command]
pub fn stop_preview(_: State<'_, Arc<AppState>>) {}
#[tauri::command]
pub fn preview_command(_: State<'_, Arc<AppState>>, _: String) -> Result<(), String> {
    Err("Native mpv preview controls are not implemented for this platform yet.".into())
}
