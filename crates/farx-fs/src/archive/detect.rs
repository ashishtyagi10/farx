use std::path::Path;

/// Entry in an archive listing.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Supported archive kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveKind {
    Zip,
    Tar,
    TarGz,
}

/// Classify a path into a supported archive kind, if any.
pub(crate) fn classify(path: &Path) -> Option<ArchiveKind> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let name = path.to_string_lossy().to_lowercase();

    match ext.as_deref() {
        Some("zip") | Some("jar") => Some(ArchiveKind::Zip),
        Some("tar") => Some(ArchiveKind::Tar),
        Some("gz") | Some("tgz") => {
            if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
                Some(ArchiveKind::TarGz)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if a path is a supported archive.
pub fn is_archive(path: &Path) -> bool {
    classify(path).is_some()
}
