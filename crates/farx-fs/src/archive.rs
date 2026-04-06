use anyhow::Result;
use std::path::Path;

/// Entry in an archive listing.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// List contents of a zip archive.
fn list_zip(path: &Path) -> Result<Vec<ArchiveEntry>> {
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

/// List contents of a tar (optionally gzipped) archive.
fn list_tar(path: &Path, gzipped: bool) -> Result<Vec<ArchiveEntry>> {
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

/// List contents of an archive (zip, tar, tar.gz, tgz).
pub fn list_archive(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let name = path.to_string_lossy().to_lowercase();

    match ext.as_deref() {
        Some("zip") | Some("jar") => list_zip(path),
        Some("tar") => list_tar(path, false),
        Some("gz") | Some("tgz") => {
            if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
                list_tar(path, true)
            } else {
                anyhow::bail!("Not a supported archive: {}", path.display())
            }
        }
        _ => anyhow::bail!("Not a supported archive: {}", path.display()),
    }
}

/// Extract an archive to a destination directory.
pub fn extract_archive(path: &Path, dest: &Path) -> Result<usize> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let name = path.to_string_lossy().to_lowercase();

    std::fs::create_dir_all(dest)?;

    match ext.as_deref() {
        Some("zip") | Some("jar") => extract_zip(path, dest),
        Some("tar") => extract_tar(path, dest, false),
        Some("gz") | Some("tgz") => {
            if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
                extract_tar(path, dest, true)
            } else {
                anyhow::bail!("Not a supported archive")
            }
        }
        _ => anyhow::bail!("Not a supported archive"),
    }
}

fn extract_zip(path: &Path, dest: &Path) -> Result<usize> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let count = archive.len();
    archive.extract(dest)?;
    Ok(count)
}

fn extract_tar(path: &Path, dest: &Path, gzipped: bool) -> Result<usize> {
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

/// Compress files into a zip archive.
pub fn compress_to_zip(files: &[&Path], archive_path: &Path) -> Result<usize> {
    let file = std::fs::File::create(archive_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0;

    for &path in files {
        if path.is_file() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            zip.start_file(&name, options)?;
            let data = std::fs::read(path)?;
            std::io::Write::write_all(&mut zip, &data)?;
            count += 1;
        } else if path.is_dir() {
            count += compress_dir_to_zip(&mut zip, path, path, options)?;
        }
    }
    zip.finish()?;
    Ok(count)
}

fn compress_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base.parent().unwrap_or(base))
            .unwrap_or(&path);
        let name = rel.to_string_lossy().to_string();

        if path.is_dir() {
            zip.add_directory(&name, options)?;
            count += compress_dir_to_zip(zip, base, &path, options)?;
        } else {
            zip.start_file(&name, options)?;
            let data = std::fs::read(&path)?;
            std::io::Write::write_all(zip, &data)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Check if a path is a supported archive.
pub fn is_archive(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let name = path.to_string_lossy().to_lowercase();
    matches!(ext.as_deref(), Some("zip") | Some("jar") | Some("tar"))
        || name.ends_with(".tar.gz")
        || name.ends_with(".tgz")
}
