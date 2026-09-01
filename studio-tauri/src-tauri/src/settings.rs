use crate::{
    models::{AppSettings, LanguagePack},
    state::AppState,
    storage::write_atomic,
};
use std::{fs, path::PathBuf, process::Command};
use tauri::{AppHandle, Manager, State};

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("settings.json"))
}

fn normalize(mut settings: AppSettings) -> AppSettings {
    if !matches!(settings.ui_font.as_str(), "system" | "cjk" | "arib") {
        settings.ui_font = "system".into();
    }
    if !matches!(settings.caption_font.as_str(), "arib" | "system") {
        settings.caption_font = "arib".into();
    }
    if !matches!(
        settings.default_format.as_str(),
        "ASS" | "TTML" | "JSON" | "Raw Data"
    ) {
        settings.default_format = "ASS".into();
    }
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
        settings.theme = "system".into();
    }
    if settings.locale.trim().is_empty() {
        settings.locale = "system".into();
    }
    settings.workspace_layout.source_width = settings.workspace_layout.source_width.clamp(220, 320);
    settings.workspace_layout.output_width = settings.workspace_layout.output_width.clamp(280, 380);
    settings
}

fn language_pack_directory(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve application data directory: {error}"))?
        .join("language-packs"))
}

#[derive(serde::Deserialize)]
struct LanguagePackFile {
    locale: String,
    name: String,
    messages: std::collections::BTreeMap<String, String>,
}

/// Reads only bounded JSON language packs from the application-owned folder.
/// The WebView never receives an arbitrary directory capability or parses
/// local files directly.
#[tauri::command]
pub fn list_language_packs(app: AppHandle) -> Result<Vec<LanguagePack>, String> {
    let directory = language_pack_directory(&app)?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create language pack directory: {error}"))?;
    let mut packs = Vec::new();
    let entries = fs::read_dir(&directory)
        .map_err(|error| format!("Could not read language directory: {error}"))?;
    for entry in entries.take(64) {
        let entry = entry.map_err(|error| format!("Could not inspect language file: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Could not inspect language file metadata: {error}"))?;
        if metadata.len() > 2 * 1024 * 1024 {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("Could not read language pack {}: {error}", path.display()))?;
        let parsed: LanguagePackFile = match serde_json::from_slice(&bytes) {
            Ok(pack) => pack,
            Err(_) => continue,
        };
        if parsed.locale.trim().is_empty()
            || parsed.name.trim().is_empty()
            || parsed.messages.len() > 2_000
        {
            continue;
        }
        packs.push(LanguagePack {
            locale: parsed.locale,
            name: parsed.name,
            messages: parsed.messages,
        });
    }
    Ok(packs)
}

#[tauri::command]
pub fn open_language_pack_directory(app: AppHandle) -> Result<(), String> {
    let directory = language_pack_directory(&app)?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create language pack directory: {error}"))?;
    open_directory(&directory)
}

#[cfg(target_os = "windows")]
fn open_directory(directory: &std::path::Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open language pack directory: {error}"))
}

#[cfg(target_os = "macos")]
fn open_directory(directory: &std::path::Path) -> Result<(), String> {
    Command::new("open")
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open language pack directory: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_directory(directory: &std::path::Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open language pack directory: {error}"))
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    match fs::read(settings_path(&app)?) {
        Ok(bytes) => serde_json::from_slice::<AppSettings>(&bytes)
            .map(normalize)
            .map_err(|error| format!("Could not decode settings: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(error) => Err(format!("Could not read settings: {error}")),
    }
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, std::sync::Arc<AppState>>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = normalize(settings);
    let path = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create settings directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(&settings)
        .map_err(|error| format!("Could not encode settings: {error}"))?;
    write_atomic(&path, &bytes).map_err(|error| format!("Could not publish settings: {error}"))?;
    *state
        .caption_font
        .lock()
        .map_err(|_| "Caption font state is unavailable.".to_string())? =
        settings.caption_font.clone();
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, normalize};

    #[test]
    fn keeps_supported_persisted_setting_values() {
        let settings = normalize(AppSettings {
            ui_font: "cjk".into(),
            caption_font: "system".into(),
            default_format: "TTML".into(),
            locale: "ja".into(),
            theme: "dark".into(),
            workspace_layout: Default::default(),
            onboarding_version: 1,
        });
        assert_eq!(settings.ui_font, "cjk");
        assert_eq!(settings.caption_font, "system");
        assert_eq!(settings.default_format, "TTML");
        assert_eq!(settings.locale, "ja");
        assert_eq!(settings.theme, "dark");
    }

    #[test]
    fn replaces_untrusted_setting_values_with_documented_defaults() {
        let settings = normalize(AppSettings {
            ui_font: "not-a-font-profile".into(),
            caption_font: "untrusted-font".into(),
            default_format: "SRT".into(),
            locale: "".into(),
            theme: "neon".into(),
            workspace_layout: crate::models::WorkspaceLayoutSettings {
                source_width: 12,
                output_width: 900,
                source_collapsed: true,
                output_collapsed: false,
            },
            onboarding_version: 0,
        });
        assert_eq!(settings.ui_font, "system");
        assert_eq!(settings.caption_font, "arib");
        assert_eq!(settings.default_format, "ASS");
        assert_eq!(settings.locale, "system");
        assert_eq!(settings.theme, "system");
        assert_eq!(settings.workspace_layout.source_width, 220);
        assert_eq!(settings.workspace_layout.output_width, 380);
        assert!(settings.workspace_layout.source_collapsed);
    }

    #[test]
    fn old_settings_without_onboarding_version_require_the_guide_once() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "uiFont": "system",
            "captionFont": "arib",
            "defaultFormat": "ASS",
            "locale": "system",
            "theme": "system",
            "workspaceLayout": {
                "sourceWidth": 240,
                "outputWidth": 300,
                "sourceCollapsed": false,
                "outputCollapsed": false
            }
        }))
        .expect("legacy settings");
        assert_eq!(settings.onboarding_version, 0);
    }
}
