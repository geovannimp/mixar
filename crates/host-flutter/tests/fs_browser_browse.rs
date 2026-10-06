//! Browse shim: FRB DTO field mapping + error mapping over fs-browser.
//!
//! The bare `fs-browser` unit tests already cover the dir/audio split, so this
//! asserts what only the host shim adds: the `FsEntry`/`FsDirectoryListing`
//! field mapping (full paths, `parent`) and error passthrough.

use std::fs;

use host_flutter::api::fs_browser::browse_fs_directory;
use tempfile::tempdir;

#[test]
fn browse_maps_full_paths_into_frb_dto() {
    let root = tempdir().expect("temp dir");
    let nested = root.path().join("nested");
    fs::create_dir(&nested).expect("nested dir");
    fs::write(root.path().join("track.wav"), b"RIFF").expect("wav file");

    let listing =
        browse_fs_directory(root.path().to_string_lossy().into_owned()).expect("browse temp dir");

    // Host DTO carries the canonical directory path and its parent.
    let canonical_root = fs::canonicalize(root.path()).expect("canonical root");
    assert_eq!(listing.path, canonical_root.to_string_lossy());
    assert_eq!(
        listing.parent.as_deref(),
        canonical_root
            .parent()
            .map(|p| p.to_string_lossy())
            .as_deref()
    );

    // Each FsEntry carries the full path, not just the display name.
    assert_eq!(listing.directories.len(), 1);
    assert_eq!(listing.directories[0].name, "nested");
    assert_eq!(
        listing.directories[0].path,
        canonical_root.join("nested").to_string_lossy()
    );

    assert_eq!(listing.audio_files.len(), 1);
    assert_eq!(listing.audio_files[0].name, "track.wav");
    assert_eq!(
        listing.audio_files[0].path,
        canonical_root.join("track.wav").to_string_lossy()
    );
}

#[test]
fn browse_missing_directory_maps_error() {
    let missing = tempdir().expect("temp dir").path().join("no-such-dir");

    let error = browse_fs_directory(missing.to_string_lossy().into_owned())
        .expect_err("missing directory must map to Err");
    assert!(
        error.contains("Not a directory"),
        "unexpected shim error: {error}"
    );
}
