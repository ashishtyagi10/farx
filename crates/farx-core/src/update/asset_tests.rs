//! Tests for `asset.rs`. Split into a sibling `#[path]` module (a child of
//! `update::asset`, so it can reach the `pub(super)` functions) to keep
//! `asset.rs` within the file-size cap.

use super::*;
use std::io::Write;

fn asset(name: &str) -> ReleaseAsset {
    ReleaseAsset {
        name: name.to_string(),
        download_url: format!("https://example.com/{name}"),
    }
}

// The os/arch substrings select_asset falls back to, mirrored from the
// function so tests are portable across the dev (macOS) and CI (Linux) hosts.
fn os_arch() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "macos") {
        "apple"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "windows"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    (os, arch)
}

#[test]
fn select_asset_matches_exact_target() {
    let target = self_update::get_target();
    let assets = vec![
        asset("farx-other.tar.gz"),
        asset(&format!("farx-{target}.tar.gz")),
    ];
    let picked = select_asset(&assets).unwrap();
    assert!(picked.name.contains(target));
}

#[test]
fn select_asset_falls_back_to_os_and_arch() {
    let (os, arch) = os_arch();
    // Name carries os+arch but not the full target triple, forcing the fallback.
    let assets = vec![asset(&format!("farx_{os}_{arch}.zip"))];
    let picked = select_asset(&assets).unwrap();
    assert!(picked.name.contains(os) && picked.name.contains(arch));
}

#[test]
fn select_asset_errors_when_nothing_matches() {
    let assets = vec![asset("totally-unrelated.bin")];
    assert!(select_asset(&assets).is_err());
}

#[test]
fn extract_binary_from_tar_gz() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("farx.tar.gz");
    {
        let f = std::fs::File::create(&archive).unwrap();
        let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        let mut builder = tar::Builder::new(enc);
        let data = b"BINARYDATA";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append_data(&mut header, "farx", &data[..]).unwrap();
        builder.into_inner().unwrap().finish().unwrap();
    }
    let out = dir.path().join("farx");
    extract_binary("farx.tar.gz", &archive, &out).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"BINARYDATA");
}

#[test]
fn extract_binary_from_zip() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("farx.zip");
    {
        let f = std::fs::File::create(&archive).unwrap();
        let mut zw = zip::ZipWriter::new(f);
        zw.start_file("farx", zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(b"ZIPPED").unwrap();
        zw.finish().unwrap();
    }
    let out = dir.path().join("farx");
    extract_binary("farx.zip", &archive, &out).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"ZIPPED");
}

#[test]
fn extract_binary_plain_copy_for_unknown_extension() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("farx-raw");
    std::fs::write(&src, b"RAW").unwrap();
    let out = dir.path().join("farx");
    extract_binary("farx-raw", &src, &out).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"RAW");
}

#[test]
fn make_executable_sets_mode_on_unix() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("farx");
    std::fs::write(&bin, b"x").unwrap();
    make_executable(&bin).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&bin).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }
}
