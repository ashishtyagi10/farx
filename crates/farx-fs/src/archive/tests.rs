use super::*;

#[test]
fn zip_archive_roundtrip_list_and_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let src_file = tmp.path().join("a.txt");
    std::fs::write(&src_file, "hello zip").unwrap();

    let zip_path = tmp.path().join("test.zip");
    let count = compress_to_zip(&[src_file.as_path()], &zip_path).unwrap();
    assert_eq!(count, 1);
    assert!(is_archive(&zip_path));

    let entries = list_archive(&zip_path).unwrap();
    assert!(entries.iter().any(|e| e.name.ends_with("a.txt")));

    let out = tmp.path().join("out");
    let extracted = extract_archive(&zip_path, &out).unwrap();
    assert_eq!(extracted, entries.len());
    assert_eq!(
        std::fs::read_to_string(out.join("a.txt")).unwrap(),
        "hello zip"
    );
}

#[test]
fn tar_gz_archive_list_and_extract() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("payload");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("b.txt"), "hello tar").unwrap();

    let tar_gz = tmp.path().join("test.tar.gz");
    let file = std::fs::File::create(&tar_gz).unwrap();
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(enc);
    builder.append_dir_all("payload", &src).unwrap();
    builder.finish().unwrap();
    let enc = builder.into_inner().unwrap();
    let _file = enc.finish().unwrap();

    assert!(is_archive(&tar_gz));
    let entries = list_archive(&tar_gz).unwrap();
    assert!(entries.iter().any(|e| e.name.contains("payload")));

    let out = tmp.path().join("untar");
    let extracted = extract_archive(&tar_gz, &out).unwrap();
    assert!(extracted >= 1);
    assert_eq!(
        std::fs::read_to_string(out.join("payload").join("b.txt")).unwrap(),
        "hello tar"
    );
}

#[test]
fn non_archive_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let plain = tmp.path().join("plain.txt");
    std::fs::write(&plain, "x").unwrap();
    assert!(!is_archive(&plain));
    assert!(list_archive(&plain).is_err());
    assert!(extract_archive(&plain, tmp.path()).is_err());
}
