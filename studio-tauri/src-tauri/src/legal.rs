use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const MAX_LEGAL_DOCUMENT_BYTES: u64 = 512 * 1024;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalDocumentSummary {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    license: &'static str,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalDocumentContent {
    id: &'static str,
    title: &'static str,
    content: String,
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LegalDocumentId {
    ProjectLicense,
    UnsignedAlphaNotice,
    ThirdPartyNotices,
    DependencyLicenses,
    LibaribcaptionLicense,
    LibmpvLicense,
    LibmpvCopyright,
    AribFontLicense,
}

struct LegalDocumentDescriptor {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    license: &'static str,
    relative_path: &'static str,
    source_relative_path: &'static str,
}

const DOCUMENTS: [LegalDocumentDescriptor; 8] = [
    LegalDocumentDescriptor {
        id: "project-license",
        title: "ResubWinny",
        category: "Project",
        license: "MPL-2.0",
        relative_path: "LICENSE",
        source_relative_path: "LICENSE",
    },
    LegalDocumentDescriptor {
        id: "unsigned-alpha-notice",
        title: "Unsigned Windows Alpha notice",
        category: "Distribution",
        license: "Release notice",
        relative_path: "UNSIGNED-WINDOWS-ALPHA.txt",
        source_relative_path: "UNSIGNED-WINDOWS-ALPHA.txt",
    },
    LegalDocumentDescriptor {
        id: "third-party-notices",
        title: "Third-party notices",
        category: "Project",
        license: "Multiple",
        relative_path: "THIRD_PARTY_NOTICES.md",
        source_relative_path: "THIRD_PARTY_NOTICES.md",
    },
    LegalDocumentDescriptor {
        id: "dependency-licenses",
        title: "Rust and npm dependency inventory",
        category: "Dependencies",
        license: "Multiple",
        relative_path: "licenses/dependency-licenses.md",
        source_relative_path: "docs/dependency-licenses.md",
    },
    LegalDocumentDescriptor {
        id: "libaribcaption-license",
        title: "libaribcaption",
        category: "Bundled runtime",
        license: "MIT",
        relative_path: "licenses/libaribcaption-MIT.txt",
        source_relative_path: "third_party/libaribcaption/LICENSE",
    },
    LegalDocumentDescriptor {
        id: "libmpv-license",
        title: "libmpv",
        category: "Bundled runtime",
        license: "LGPL-2.1-or-later",
        relative_path: "licenses/libmpv-LGPL-2.1.txt",
        source_relative_path: "third_party/libmpv/LICENSE.LGPL",
    },
    LegalDocumentDescriptor {
        id: "libmpv-copyright",
        title: "libmpv copyright notice",
        category: "Bundled runtime",
        license: "Multiple",
        relative_path: "licenses/libmpv-COPYRIGHT.txt",
        source_relative_path: "third_party/libmpv/COPYRIGHT.mpv",
    },
    LegalDocumentDescriptor {
        id: "arib-font-license",
        title: "Rounded M+ 1m for ARIB",
        category: "Bundled asset",
        license: "M+ FONT LICENSE",
        relative_path: "fonts/LICENSE.rounded-mplus-1m-arib.txt",
        source_relative_path: "third_party/rounded-mplus-1m-arib/LICENSE.txt",
    },
];

impl LegalDocumentId {
    fn descriptor(self) -> &'static LegalDocumentDescriptor {
        match self {
            Self::ProjectLicense => &DOCUMENTS[0],
            Self::UnsignedAlphaNotice => &DOCUMENTS[1],
            Self::ThirdPartyNotices => &DOCUMENTS[2],
            Self::DependencyLicenses => &DOCUMENTS[3],
            Self::LibaribcaptionLicense => &DOCUMENTS[4],
            Self::LibmpvLicense => &DOCUMENTS[5],
            Self::LibmpvCopyright => &DOCUMENTS[6],
            Self::AribFontLicense => &DOCUMENTS[7],
        }
    }
}

fn development_resource_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn resource_root(app: &AppHandle) -> Result<(PathBuf, bool), String> {
    let packaged = app
        .path()
        .resource_dir()
        .map_err(|error| format!("Could not resolve application resources: {error}"))?;
    Ok(if packaged.join("LICENSE").is_file() {
        (packaged, false)
    } else {
        (development_resource_root(), true)
    })
}

fn read_document(
    root: &Path,
    source_tree: bool,
    document: LegalDocumentId,
) -> Result<LegalDocumentContent, String> {
    let descriptor = document.descriptor();
    let path = root.join(if source_tree {
        descriptor.source_relative_path
    } else {
        descriptor.relative_path
    });
    let metadata = fs::metadata(&path).map_err(|error| {
        format!(
            "Could not inspect bundled license {}: {error}",
            descriptor.id
        )
    })?;
    if metadata.len() > MAX_LEGAL_DOCUMENT_BYTES {
        return Err(format!(
            "Bundled license {} is too large to display.",
            descriptor.id
        ));
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read bundled license {}: {error}", descriptor.id))?;
    Ok(LegalDocumentContent {
        id: descriptor.id,
        title: descriptor.title,
        content,
    })
}

#[tauri::command]
pub fn list_legal_documents() -> Vec<LegalDocumentSummary> {
    DOCUMENTS
        .iter()
        .map(|document| LegalDocumentSummary {
            id: document.id,
            title: document.title,
            category: document.category,
            license: document.license,
        })
        .collect()
}

#[tauri::command]
pub fn get_legal_document(
    app: AppHandle,
    id: LegalDocumentId,
) -> Result<LegalDocumentContent, String> {
    let (root, source_tree) = resource_root(&app)?;
    read_document(&root, source_tree, id)
}

#[cfg(test)]
mod tests {
    use super::{DOCUMENTS, LegalDocumentId, development_resource_root, read_document};
    use std::collections::BTreeSet;

    #[test]
    fn legal_documents_have_unique_fixed_identifiers() {
        let ids: BTreeSet<_> = DOCUMENTS.iter().map(|document| document.id).collect();
        assert_eq!(ids.len(), DOCUMENTS.len());
        assert!(ids.iter().all(|id| !id.contains("..")));
    }

    #[test]
    fn every_document_is_readable_from_the_source_distribution() {
        let root = development_resource_root();
        for document in [
            LegalDocumentId::ProjectLicense,
            LegalDocumentId::UnsignedAlphaNotice,
            LegalDocumentId::ThirdPartyNotices,
            LegalDocumentId::DependencyLicenses,
            LegalDocumentId::LibaribcaptionLicense,
            LegalDocumentId::LibmpvLicense,
            LegalDocumentId::LibmpvCopyright,
            LegalDocumentId::AribFontLicense,
        ] {
            let content = read_document(&root, true, document).expect("bundled legal document");
            assert!(!content.content.trim().is_empty());
        }
    }
}
