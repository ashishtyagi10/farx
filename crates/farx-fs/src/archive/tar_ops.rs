use anyhow::Result;
use std::path::Path;

use super::detect::ArchiveEntry;

/// List contents of a tar (optionally gzipped) archive.
pub(crate) fn list_tar(path: &Path, gzipped: bool) -> Result<Vec<ArchiveEntry>> {
    let file = std::fs::File::open(path)?;
    let mut entries = Vec::new();
    if gzipped {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries()? {
            let entry = entry?;
            let is_dir = entry.header().entry_type().is_dir();
            entries.push(ArchiveEntry {
                name: entry.path()?.to_string_lossy().to_string(),
                is_dir,
                size: entry.size(),
            });
        }
    } else {
        let mut archive = tar::Archive::new(file);
        for entry in archive.entries()? {
            let entry = entry?;
            let is_dir = entry.header().entry_type().is_dir();
            entries.push(ArchiveEntry {
                name: entry.path()?.to_string_lossy().to_string(),
                is_dir,
                size: entry.size(),
            });
        }
    }
    Ok(entries)
}

/// Extract a tar (optionally gzipped) archive to a destination directory.
pub(crate) fn extract_tar(path: &Path, dest: &Path, gzipped: bool) -> Result<usize> {
    let file = std::fs::File::open(path)?;
    let mut count = 0;
    if gzipped {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(dest)?;
        // Count entries by re-reading
        let file2 = std::fs::File::open(path)?;
        let decoder2 = flate2::read::GzDecoder::new(file2);
        let mut archive2 = tar::Archive::new(decoder2);
        for entry in archive2.entries()? {
            let _ = entry?;
            count += 1;
        }
    } else {
        let mut archive = tar::Archive::new(file);
        archive.unpack(dest)?;
        let file2 = std::fs::File::open(path)?;
        let mut archive2 = tar::Archive::new(file2);
        for entry in archive2.entries()? {
            let _ = entry?;
            count += 1;
        }
    }
    Ok(count)
}
