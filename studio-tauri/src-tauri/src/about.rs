#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    product_name: &'static str,
    description: &'static str,
    version: &'static str,
    channel: &'static str,
    platform: String,
    architecture: &'static str,
    release_tier: String,
    build_tag: Option<String>,
    build_commit: Option<String>,
    signing_declaration: &'static str,
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectLink {
    Source,
    Releases,
    Issues,
}

fn project_url(target: ProjectLink) -> &'static str {
    match target {
        ProjectLink::Source => "https://github.com/margueriteaya/ResubWinny",
        ProjectLink::Releases => "https://github.com/margueriteaya/ResubWinny/releases",
        ProjectLink::Issues => "https://github.com/margueriteaya/ResubWinny/issues",
    }
}

fn optional_build_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn build_about_info() -> AboutInfo {
    let version = env!("CARGO_PKG_VERSION");
    let release_tier = option_env!("RESUBWINNY_RELEASE_TIER")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Development")
        .to_owned();
    let signing_declaration = match release_tier.as_str() {
        "UnsignedWindowsAlpha" => "unsigned-alpha",
        "SignedStable" => "declared-signed",
        _ => "development",
    };
    AboutInfo {
        product_name: "ResubWinny",
        description: env!("CARGO_PKG_DESCRIPTION"),
        version,
        channel: if version.contains("alpha") {
            "Alpha"
        } else {
            "Stable"
        },
        platform: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH,
        release_tier,
        build_tag: optional_build_value(option_env!("RESUBWINNY_BUILD_TAG")),
        build_commit: optional_build_value(option_env!("RESUBWINNY_BUILD_COMMIT")),
        signing_declaration,
    }
}

#[tauri::command]
pub fn get_about_info() -> AboutInfo {
    build_about_info()
}

#[tauri::command]
pub fn open_project_link(target: ProjectLink) -> Result<(), String> {
    open_external_url(project_url(target))
}

#[cfg(target_os = "windows")]
fn open_external_url(url: &str) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open project link: {error}"))
}

#[cfg(target_os = "macos")]
fn open_external_url(url: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open project link: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_external_url(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open project link: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{ProjectLink, build_about_info, optional_build_value, project_url};

    #[test]
    fn blank_optional_build_values_are_omitted() {
        assert_eq!(optional_build_value(Some("  ")), None);
        assert_eq!(optional_build_value(None), None);
        assert_eq!(
            optional_build_value(Some(" v0.2.2-alpha.1 ")).as_deref(),
            Some("v0.2.2-alpha.1")
        );
    }

    #[test]
    fn development_build_has_safe_provenance_fallbacks() {
        let info = build_about_info();
        assert_eq!(info.product_name, "ResubWinny");
        assert!(!info.version.is_empty());
        assert!(!info.release_tier.is_empty());
    }

    #[test]
    fn project_links_are_fixed_to_the_repository() {
        for target in [
            ProjectLink::Source,
            ProjectLink::Releases,
            ProjectLink::Issues,
        ] {
            assert!(project_url(target).starts_with("https://github.com/margueriteaya/ResubWinny"));
        }
    }
}
