//! Archive support: detection, listing, extraction, and compression.

mod compress;
mod detect;
mod tar_ops;
mod zip_ops;

use anyhow::Result;
use std::path::Path;

pub use compress::compress_to_zip;
pub use detect::{is_archive, ArchiveEntry};

use detect::{classify, ArchiveKind};

/// List contents of an archive (zip, tar, tar.gz, tgz).
pub fn list_archive(path: &Path) -> Result<Vec<ArchiveEntry>> {
    match classify(path) {
        Some(ArchiveKind::Zip) => zip_ops::list_zip(path),
        Some(ArchiveKind::Tar) => tar_ops::list_tar(path, false),
        Some(ArchiveKind::TarGz) => tar_ops::list_tar(path, true),
        None => anyhow::bail!("Not a supported archive: {}", path.display()),
    }
}

/// Extract an archive to a destination directory.
pub fn extract_archive(path: &Path, dest: &Path) -> Result<usize> {
    let kind = classify(path).ok_or_else(|| anyhow::anyhow!("Not a supported archive"))?;
    std::fs::create_dir_all(dest)?;
    match kind {
        ArchiveKind::Zip => zip_ops::extract_zip(path, dest),
        ArchiveKind::Tar => tar_ops::extract_tar(path, dest, false),
        ArchiveKind::TarGz => tar_ops::extract_tar(path, dest, true),
    }
}

#[cfg(test)]
mod tests;
