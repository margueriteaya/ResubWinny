use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub ui_font: String,
    pub caption_font: String,
    pub default_format: String,
    pub default_timeline: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub workspace_layout: WorkspaceLayoutSettings,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayoutSettings {
    pub source_width: u16,
    pub output_width: u16,
    pub source_collapsed: bool,
    pub output_collapsed: bool,
}

impl Default for WorkspaceLayoutSettings {
    fn default() -> Self {
        Self {
            source_width: 240,
            output_width: 300,
            source_collapsed: false,
            output_collapsed: false,
        }
    }
}

fn default_locale() -> String {
    "system".into()
}
fn default_theme() -> String {
    "system".into()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ui_font: "system".into(),
            caption_font: "arib".into(),
            default_format: "ASS".into(),
            default_timeline: "Auto (Gap Merge + Overlap Resolve)".into(),
            locale: default_locale(),
            theme: default_theme(),
            workspace_layout: WorkspaceLayoutSettings::default(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePack {
    pub locale: String,
    pub name: String,
    pub messages: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCapabilities {
    pub video_backend: String,
    pub caption_overlay_modes: Vec<PreviewSurfaceCapability>,
    pub selected_caption_overlay: String,
    pub caption_plane_modes: Vec<String>,
    pub available_caption_plane_modes: Vec<String>,
}

/// A platform-owned preview route. The WebView receives this only to present
/// an honest capability state; it never owns the video or caption pixels.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSurfaceCapability {
    pub id: String,
    pub available: bool,
    pub experimental: bool,
    pub unavailable_reason_code: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRuntime {
    pub backend: String,
    pub platform: String,
    pub library_path: Option<String>,
    pub available: bool,
    pub render_api_available: bool,
    pub detail: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRenderDiagnostics {
    pub route: String,
    pub active: bool,
    pub frames_presented: u64,
    pub presents_per_second: f64,
    pub caption_texture_uploads: u64,
    pub caption_texture_clears: u64,
    pub video_aspect: Option<f64>,
    pub surface_width: Option<i32>,
    pub surface_height: Option<i32>,
    pub decoder_mode: Option<String>,
    pub fallback_reason: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPlaybackState {
    pub time_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub paused: Option<bool>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub container: String,
    pub packet_size: Option<u64>,
    pub route_code: String,
    pub route: String,
    pub service: String,
    pub tracks: Vec<Track>,
    pub broadcast: BroadcastMetadata,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastMetadata {
    #[serde(alias = "network_name")]
    pub network_name: Option<String>,
    #[serde(alias = "programme_name")]
    pub programme_name: Option<String>,
    #[serde(alias = "programme_description")]
    pub programme_description: Option<String>,
    #[serde(alias = "broadcast_time_utc")]
    pub broadcast_time_utc: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub label: String,
    pub detail: String,
    pub pid: Option<String>,
    pub kind: String,
    pub ordinal: usize,
    pub service_id: Option<u16>,
    pub language: Option<String>,
    pub service_name: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrcsGlyph {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub alternative_text: String,
    pub image: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrcsMappingInput {
    pub id: String,
    pub text: String,
    pub action: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreservation {
    #[serde(default = "enabled")]
    pub position: bool,
    #[serde(default = "enabled")]
    pub color: bool,
    #[serde(default = "enabled")]
    pub ruby: bool,
    #[serde(default = "enabled")]
    pub drcs: bool,
    #[serde(default = "enabled")]
    pub gaiji: bool,
    #[serde(default = "enabled")]
    pub accessibility: bool,
}

fn enabled() -> bool {
    true
}

impl Default for ExportPreservation {
    fn default() -> Self {
        Self {
            position: true,
            color: true,
            ruby: true,
            drcs: true,
            gaiji: true,
            accessibility: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSelection {
    #[serde(default = "default_export_formats")]
    pub formats: Vec<String>,
    #[serde(default)]
    pub preservation: ExportPreservation,
}

fn default_export_formats() -> Vec<String> {
    vec!["ASS".into()]
}

impl Default for ExportSelection {
    fn default() -> Self {
        Self {
            formats: default_export_formats(),
            preservation: ExportPreservation::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryRecord {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub container: String,
    pub status: String,
    pub time: String,
    pub warnings: u64,
    pub captions: u64,
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub job_id: Option<String>,
    pub kind: String,
    pub code: String,
    pub parameters: BTreeMap<String, Value>,
    pub message: String,
    pub bytes_read: Option<u64>,
    pub captions: Option<u64>,
    pub warnings: Option<u64>,
    pub output: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRecord {
    pub timestamp: u64,
    pub severity: String,
    pub code: String,
    pub parameters: BTreeMap<String, Value>,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub kind: String,
    pub path: String,
    pub temporary_path: String,
    pub status: String,
    #[serde(default)]
    pub existed_before_start: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRecord {
    pub job_id: String,
    pub source: String,
    pub output: String,
    pub bytes_read: u64,
    pub captions: u64,
    pub warnings: u64,
    pub strategy: String,
    pub updated_at: u64,
    #[serde(default)]
    pub source_size: Option<u64>,
    #[serde(default)]
    pub source_modified: Option<u64>,
    #[serde(default)]
    pub source_fingerprint: Option<String>,
    #[serde(default)]
    pub track_id: Option<u16>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionRenderSnapshot {
    pub source: String,
    pub time_ms: i64,
    pub intervals: Vec<Value>,
    pub resource_previews: Vec<Value>,
    pub plane_width: Option<u32>,
    pub plane_height: Option<u32>,
    pub composed_png_base64: Option<String>,
    pub active_layer_count: usize,
    pub caption_plane_mode: String,
    pub missing_glyph_count: usize,
    pub rendered_ruby_count: usize,
    pub render_profile: CaptionRenderProfile,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionRenderProfile {
    pub renderer: String,
    pub font_family: String,
    pub preserve_character_cells: bool,
    pub ruby_scale: f32,
    pub background_alpha_from_source: bool,
    pub stroke_from_source: bool,
    pub drcs_policy: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewOverlaySyncResult {
    pub action: String,
    pub media_time_ms: Option<i64>,
    pub project_time_ms: Option<i64>,
    pub snapshot: Option<CaptionRenderSnapshot>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackTimeMapping {
    pub segment_id: String,
    pub media_anchor_ms: i64,
    pub project_anchor_ms: i64,
    pub rate_numerator: i32,
    pub rate_denominator: i32,
}

impl Default for PlaybackTimeMapping {
    fn default() -> Self {
        Self {
            segment_id: "recording-origin".into(),
            media_anchor_ms: 0,
            project_anchor_ms: 0,
            rate_numerator: 1,
            rate_denominator: 1,
        }
    }
}

impl PlaybackTimeMapping {
    pub fn project_time_ms(&self, media_time_ms: i64) -> Result<i64, String> {
        if self.rate_numerator <= 0 || self.rate_denominator <= 0 {
            return Err("Playback time mapping rate must be positive.".into());
        }
        // Convert to a wide signed type before subtracting.  Saturating the
        // `i64` values first would clamp timestamps before the anchor to zero,
        // making the desktop mapping disagree with the WebView's signed math.
        let delta = (media_time_ms as i128) - (self.media_anchor_ms as i128);
        let scaled =
            delta.saturating_mul(self.rate_numerator as i128) / self.rate_denominator as i128;
        Ok((self.project_anchor_ms as i128)
            .saturating_add(scaled)
            .clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }

    pub fn media_time_ms(&self, project_time_ms: i64) -> Result<i64, String> {
        if self.rate_numerator <= 0 || self.rate_denominator <= 0 {
            return Err("Playback time mapping rate must be positive.".into());
        }
        let delta = (project_time_ms as i128) - (self.project_anchor_ms as i128);
        let scaled =
            delta.saturating_mul(self.rate_denominator as i128) / self.rate_numerator as i128;
        Ok((self.media_anchor_ms as i128)
            .saturating_add(scaled)
            .clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::{BroadcastMetadata, PlaybackTimeMapping};

    #[test]
    fn maps_media_time_through_a_segment_offset_and_rate() {
        let mapping = PlaybackTimeMapping {
            segment_id: "programme-2".into(),
            media_anchor_ms: 10_000,
            project_anchor_ms: 25_000,
            rate_numerator: 1001,
            rate_denominator: 1000,
        };
        assert_eq!(mapping.project_time_ms(11_000).unwrap(), 26_001);
        assert_eq!(mapping.media_time_ms(26_001).unwrap(), 11_000);
    }

    #[test]
    fn preserves_signed_offsets_before_each_anchor() {
        let mapping = PlaybackTimeMapping {
            segment_id: "offset-segment".into(),
            media_anchor_ms: 10_000,
            project_anchor_ms: 25_000,
            rate_numerator: 2,
            rate_denominator: 1,
        };

        // A timestamp before the media anchor must map before the project
        // anchor instead of being clamped to the anchor itself.
        assert_eq!(mapping.project_time_ms(9_500).unwrap(), 24_000);
        assert_eq!(mapping.media_time_ms(24_000).unwrap(), 9_500);
    }

    #[test]
    fn maps_both_directions_with_non_default_rate_and_anchors() {
        let mapping = PlaybackTimeMapping {
            segment_id: "programme-3".into(),
            media_anchor_ms: 123_456,
            project_anchor_ms: -20_000,
            rate_numerator: 3,
            rate_denominator: 2,
        };

        assert_eq!(mapping.project_time_ms(124_456).unwrap(), -18_500);
        assert_eq!(mapping.media_time_ms(-18_500).unwrap(), 124_456);
    }

    #[test]
    fn rejects_invalid_mapping_rates() {
        let mapping = PlaybackTimeMapping {
            rate_denominator: 0,
            ..Default::default()
        };
        assert!(mapping.project_time_ms(0).is_err());
    }

    #[test]
    fn accepts_worker_snake_case_broadcast_metadata() {
        let metadata: BroadcastMetadata = serde_json::from_value(serde_json::json!({
            "network_name": "関東広域3",
            "programme_name": "Nスタ",
            "programme_description": "番組案内",
            "broadcast_time_utc": "2026-02-23 18:59:00 UTC"
        }))
        .unwrap();
        assert_eq!(metadata.network_name.as_deref(), Some("関東広域3"));
        assert_eq!(metadata.programme_name.as_deref(), Some("Nスタ"));
        assert_eq!(metadata.programme_description.as_deref(), Some("番組案内"));
        assert_eq!(
            metadata.broadcast_time_utc.as_deref(),
            Some("2026-02-23 18:59:00 UTC")
        );
    }
}

#[derive(Deserialize)]
pub struct WorkerProbe {
    pub probe: WorkerProbeInfo,
    pub inspection: WorkerInspection,
    pub b24_track: Option<WorkerB24Track>,
    #[serde(default)]
    pub b24_tracks: Vec<WorkerB24Track>,
    #[serde(default)]
    pub mpeg_ts_data_tracks: Option<WorkerDataTracks>,
    #[serde(default)]
    pub m2ts_data_tracks: Option<WorkerDataTracks>,
}
#[derive(Deserialize)]
pub struct WorkerProbeInfo {
    pub kind: String,
    pub packet_size: Option<u64>,
}
#[derive(Deserialize)]
pub struct WorkerInspection {
    pub route_code: String,
    pub route: String,
    pub service: String,
    #[serde(default)]
    pub broadcast: BroadcastMetadata,
}
#[derive(Deserialize)]
pub struct WorkerB24Track {
    pub caption_pid: u16,
    #[serde(default)]
    pub service_id: Option<u16>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub service_name: Option<String>,
}
#[derive(Deserialize)]
pub struct WorkerDataTracks {
    pub pids: Vec<u16>,
    #[serde(default)]
    pub caption_pids: Vec<u16>,
    #[serde(default)]
    pub superimpose_pids: Vec<u16>,
}

#[derive(Deserialize)]
pub struct DrcsReport {
    pub glyphs: Vec<DrcsReportGlyph>,
}
#[derive(Deserialize)]
pub struct DrcsReportGlyph {
    pub asset: String,
    pub width: u32,
    pub height: u32,
    pub depth_bits: u8,
    pub drcs_code: u32,
    #[serde(default)]
    pub alternative_text: String,
}
