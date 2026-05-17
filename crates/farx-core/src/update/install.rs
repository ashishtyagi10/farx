//! Blocking install flow: pick the right release asset, download it,
//! extract the `farx` binary, and install it without ever invoking sudo —
//! first by replacing the running binary in place when possible, otherwise
//! by writing to `~/.local/bin/farx`.

use anyhow::Result;
use self_update::cargo_crate_version;
use semver::Version;
use std::path::{Path, PathBuf};

use super::asset::{extract_binary, make_executable, select_asset};
use super::{REPO_NAME, REPO_OWNER};

/// Perform the actual update: download the latest release and install
/// without sudo. Tries an in-place replace of the running binary first;
/// falls back to `~/.local/bin/farx`.
pub fn perform_update() -> Result<self_update::Status> {
    let current = cargo_crate_version!();

    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;

    let latest = releases
        .first()
        .ok_or_else(|| anyhow::anyhow!("No releases found"))?;

    let latest_ver = latest.version.trim_start_matches('v');
    let current_ver = Version::parse(current)?;
    let remote_ver = Version::parse(latest_ver)?;

    if remote_ver <= current_ver {
        return Ok(self_update::Status::UpToDate(current.to_string()));
    }

    let asset = select_asset(&latest.assets)?;

    println!("Downloading {}...", asset.name);
    let tmp_dir = tempfile::tempdir()?;
    let tmp_archive = tmp_dir.path().join(&asset.name);

    let response = reqwest::blocking::get(&asset.download_url)?;
    let bytes = response.bytes()?;
    std::fs::write(&tmp_archive, &bytes)?;

    let tmp_bin = tmp_dir.path().join("farx");
    extract_binary(&asset.name, &tmp_archive, &tmp_bin)?;

    if !tmp_bin.exists() {
        anyhow::bail!("Could not find 'farx' binary in the release archive");
    }

    make_executable(&tmp_bin)?;

    // 1. Try replacing the running binary atomically in-place. Works
    //    without sudo whenever the user has write access to the directory
    //    containing the current executable (e.g. ~/.local/bin, ~/bin,
    //    /opt/homebrew/bin if owned by the user, etc.).
    if let Some(current_exe) = current_exe_if_writable() {
        match self_replace::self_replace(&tmp_bin) {
            Ok(()) => {
                println!("Replaced {} in place.", current_exe.display());
                return Ok(self_update::Status::Updated(remote_ver.to_string()));
            }
            Err(e) => {
                println!(
                    "In-place replace of {} failed ({}). Falling back to ~/.local/bin.",
                    current_exe.display(),
                    e
                );
            }
        }
    }

    // 2. Fall back: install to ~/.local/bin (a user-writable location).
    let local_bin = install_to_local_bin(&tmp_bin)?;
    warn_path_and_shadow(&local_bin);

    Ok(self_update::Status::Updated(remote_ver.to_string()))
}

/// Return `current_exe()` only if the directory containing it is writable
/// by the current user (so `self_replace` can succeed without sudo).
fn current_exe_if_writable() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    let probe = parent.join(".farx-write-probe");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Some(exe)
        }
        Err(_) => None,
    }
}

/// Copy `tmp_bin` to `~/.local/bin/farx` and return the install directory.
fn install_to_local_bin(tmp_bin: &Path) -> Result<PathBuf> {
    let local_bin = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join(".local")
        .join("bin");
    std::fs::create_dir_all(&local_bin)?;

    let dest = local_bin.join("farx");
    std::fs::copy(tmp_bin, &dest)?;

    println!("Installed to {}", dest.display());
    Ok(local_bin)
}

/// Print guidance if `~/.local/bin` is not on PATH or another copy shadows it.
fn warn_path_and_shadow(local_bin: &Path) {
    if let Ok(path) = std::env::var("PATH") {
        let local_str = local_bin.to_string_lossy();
        if !path.split(':').any(|p| p == local_str.as_ref()) {
            println!();
            println!("NOTE: {} is not in your PATH. Add it:", local_bin.display());
            println!(
                "  echo 'export PATH=\"{}:$PATH\"' >> ~/.zshrc && source ~/.zshrc",
                local_bin.display()
            );
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if !current_exe.starts_with(local_bin) {
            println!();
            println!(
                "NOTE: The currently running farx is at {} and may shadow the",
                current_exe.display()
            );
            println!("      update on your PATH. Two ways to fix without sudo:");
            println!(
                "        1. Put {} earlier in your PATH, or",
                local_bin.display()
            );
            println!(
                "        2. Delete the shadowing copy yourself: rm {}",
                current_exe.display()
            );
        }
    }
}
