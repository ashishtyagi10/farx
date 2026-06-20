use std::path::Path;

use anyhow::Result;

/// Delete a file or directory.
/// If `use_trash` is true, moves to the system trash/recycle bin.
pub fn delete_entry(path: &Path, use_trash: bool) -> Result<()> {
    if use_trash {
        trash::delete(path).map_err(|e| anyhow::anyhow!("Trash: {}", e))
    } else {
        delete_permanent(path)
    }
}

fn delete_permanent(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permanent_delete_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("doomed.txt");
        std::fs::write(&file, b"bye").unwrap();
        delete_entry(&file, false).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn permanent_delete_removes_directory_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("inner.txt"), b"x").unwrap();
        delete_entry(&sub, false).unwrap();
        assert!(!sub.exists());
    }

    #[test]
    fn permanent_delete_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.txt");
        assert!(delete_entry(&missing, false).is_err());
    }
}
