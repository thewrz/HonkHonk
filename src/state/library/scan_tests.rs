use std::fs;
use std::path::PathBuf;

use super::*;

#[test]
fn scan_records_file_modification_time_in_epoch_milliseconds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("timestamped.wav");
    fs::write(&path, b"data").unwrap();

    let scan = Library::scan(&[dir.path().to_path_buf()]).unwrap();

    assert!(scan.entries[0].modified_ms.is_some());
}

#[test]
fn scan_marks_missing_root_as_incomplete() {
    let scan = Library::scan(&[PathBuf::from("/nonexistent/path/12345")]).unwrap();

    assert!(scan.entries.is_empty());
    assert!(!scan.complete);
}

#[test]
fn scan_marks_non_directory_root_as_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-directory");
    fs::write(&file, b"data").unwrap();

    let scan = Library::scan(&[file]).unwrap();

    assert!(scan.entries.is_empty());
    assert!(!scan.complete);
}

#[test]
fn time_before_epoch_has_no_timestamp() {
    let before_epoch = std::time::UNIX_EPOCH - std::time::Duration::from_millis(1);

    assert_eq!(system_time_to_epoch_ms(before_epoch), None);
}

#[test]
fn epoch_milliseconds_are_converted_without_loss() {
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_millis(42);
    assert_eq!(system_time_to_epoch_ms(time), Some(42));
}
