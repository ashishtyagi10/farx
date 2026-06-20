use std::path::Path;

use anyhow::Result;

/// Create a directory
pub fn create_directory(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

/// Rename/move a file
pub fn rename_entry(from: &Path, to: &Path) -> Result<()> {
    std::fs::rename(from, to)?;
    Ok(())
}

/// Create a symbolic link at `link_path` pointing to `target`.
pub fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)?;
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link_path)?;
        } else {
            std::os::windows::fs::symlink_file(target, link_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_directory_makes_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b/c");
        create_directory(&nested).unwrap();
        assert!(nested.is_dir());
        // Idempotent: create_dir_all on an existing path is Ok.
        create_directory(&nested).unwrap();
    }

    #[test]
    fn rename_entry_moves_file() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("old.txt");
        let to = dir.path().join("new.txt");
        std::fs::write(&from, b"hi").unwrap();
        rename_entry(&from, &to).unwrap();
        assert!(!from.exists());
        assert_eq!(std::fs::read(&to).unwrap(), b"hi");
    }

    #[cfg(unix)]
    #[test]
    fn create_symlink_points_at_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        std::fs::write(&target, b"data").unwrap();
        create_symlink(&target, &link).unwrap();
        assert_eq!(std::fs::read(&link).unwrap(), b"data");
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
