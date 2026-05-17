use anyhow::Result;
use std::path::Path;

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
