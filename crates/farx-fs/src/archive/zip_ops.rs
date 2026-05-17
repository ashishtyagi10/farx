use anyhow::Result;
use std::path::Path;

use super::detect::ArchiveEntry;

/// List contents of a zip archive.
pub(crate) fn list_zip(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        entries.push(ArchiveEntry {
            name: entry.name().to_string(),
            is_dir: entry.is_dir(),
            size: entry.size(),
        });
    }
    Ok(entries)
}

/// Extract a zip archive to a destination directory.
pub(crate) fn extract_zip(path: &Path, dest: &Path) -> Result<usize> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let count = archive.len();
    archive.extract(dest)?;
    Ok(count)
}
