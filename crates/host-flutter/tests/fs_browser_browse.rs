//! Browse smoke: host FRB wrappers over fs-browser.

use std::fs;

use host_flutter::api::fs_browser::browse_fs_directory;
use tempfile::tempdir;

#[test]
fn browse_temp_directory_finds_subdir_and_wav() {
    let root = tempdir().expect("temp dir");
    let nested = root.path().join("nested");
    fs::create_dir(&nested).expect("nested dir");
    fs::write(root.path().join("track.wav"), b"RIFF").expect("wav file");
    fs::write(root.path().join("readme.txt"), b"notes").expect("text file");

    let listing =
        browse_fs_directory(root.path().to_string_lossy().into_owned()).expect("browse temp dir");

    assert_eq!(listing.directories.len(), 1);
    assert_eq!(listing.directories[0].name, "nested");
    assert_eq!(listing.audio_files.len(), 1);
    assert_eq!(listing.audio_files[0].name, "track.wav");
}
