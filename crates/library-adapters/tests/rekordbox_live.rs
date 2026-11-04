//! Opt-in smoke test against a **real** Rekordbox library.
//!
//! The fixture suite builds synthetic databases, which proves the SQL and the
//! mapping but not that the shipped SQLCipher passphrase decrypts what
//! Rekordbox actually writes. This test closes that gap without any database
//! entering the repository:
//!
//! ```bash
//! REKORDBOX_FIXTURE="$HOME/Library/Pioneer/rekordbox/master.db" \
//!   cargo test -p library-adapters --features rekordbox \
//!   --test rekordbox_live -- --ignored --nocapture
//! ```
//!
//! Ignored by default, so CI and `cargo nextest run --workspace` skip it.
#![cfg(feature = "rekordbox")]
#![allow(clippy::unwrap_used)] // test crate: unwrap on a fixture the caller supplied

mod support;

use std::path::PathBuf;

use library_adapters::rekordbox::RekordboxLibrary;

#[test]
#[ignore = "requires REKORDBOX_FIXTURE pointing at a real Rekordbox 6/7 master.db"]
fn a_real_library_opens_migrates_and_stays_untouched() {
    let Some(path) = std::env::var_os("REKORDBOX_FIXTURE").map(PathBuf::from) else {
        eprintln!("REKORDBOX_FIXTURE unset; nothing to smoke-test");
        return;
    };

    // Decrypting a database Rekordbox wrote is the point: it fails here if the
    // shipped passphrase no longer matches (set REKORDBOX_DATABASE_KEY to the
    // current one and see `rekordbox::KEY_ENV`).
    let library = RekordboxLibrary::open(&path).unwrap();
    let preview = library.preview();
    println!("preview: {preview:#?}");

    assert!(
        preview.track_count > 0,
        "opened the database but found no tracks: {preview:?}"
    );
    assert!(preview.missing_file_count <= preview.track_count);

    let (first, second) = support::migrate_twice(&library, &path);
    println!("first import: {first:#?}");
    println!("re-import: {second:#?}");
}
