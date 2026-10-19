//! On-drive state so repeated per-collection exports accumulate.
//!
//! The export dialog acts on one collection at a time, while the device database
//! has to describe *everything* on the drive. After each run the exporter
//! therefore rewrites `PIONEER/rekordbox/mixar-export.json` with what the drive
//! now holds — tracks, playlists and cover thumbnails — and the next run merges
//! its collection into that state instead of replacing the database. Players
//! ignore the file.
//!
//! The manifest also carries a fingerprint of the `export.pdb` it describes. If
//! the database on the drive is not the one we wrote (Rekordbox touched the
//! drive, or someone deleted it), the manifest is discarded and the drive is
//! rebuilt from the current selection, so stale entries never leak into a
//! database we no longer own.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::model::{CdjArtwork, CdjPlaylist, CdjTrack};
use library_core::ExportReport;

/// Path of the manifest inside the export root.
pub(super) const MANIFEST_PATH: &str = "PIONEER/rekordbox/mixar-export.json";

/// Manifest schema version. A manifest written by a newer exporter is ignored
/// rather than misread.
const MANIFEST_VERSION: u32 = 1;

/// One track a previous run wrote, plus the source track it came from so a
/// re-export can keep the same on-drive path even if its metadata changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct ManifestTrack {
    /// Library track id (`AudioSource::id`) the row was exported from.
    pub source_id: String,
    /// The writer input that describes it.
    pub track: CdjTrack,
}

/// One playlist a previous run wrote, by on-drive track path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct ManifestPlaylist {
    /// Playlist name shown on the device.
    pub name: String,
    /// Library collection id this playlist came from, when it was a playlist
    /// (folder collections also produce a playlist, keyed by their id). Used to
    /// replace the previous version of the same collection instead of
    /// duplicating it.
    #[serde(default)]
    pub source: Option<String>,
    /// Member tracks by on-drive path, in play order.
    pub entries: Vec<String>,
}

/// One cover thumbnail a previous run wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct ManifestArtwork {
    /// Artwork-table id.
    pub id: u32,
    /// On-drive path of the thumbnail.
    pub path: String,
    /// Fingerprint of the JPEG bytes, so the same cover keeps its id.
    pub fingerprint: u64,
    /// JPEG length, a cheap guard against a truncated file.
    pub len: u64,
}

/// Everything the drive currently carries, from the exporter's point of view.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Manifest {
    /// Schema version.
    pub version: u32,
    /// Fingerprint of the `export.pdb` bytes this manifest describes.
    #[serde(default)]
    pub pdb_fingerprint: u64,
    /// Length of that database in bytes.
    #[serde(default)]
    pub pdb_len: u64,
    /// Tracks registered on the drive, in database order.
    #[serde(default)]
    pub tracks: Vec<ManifestTrack>,
    /// Playlists on the drive, in database order.
    #[serde(default)]
    pub playlists: Vec<ManifestPlaylist>,
    /// Cover thumbnails on the drive.
    #[serde(default)]
    pub artworks: Vec<ManifestArtwork>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: MANIFEST_VERSION,
            pdb_fingerprint: 0,
            pdb_len: 0,
            tracks: Vec::new(),
            playlists: Vec::new(),
            artworks: Vec::new(),
        }
    }
}

impl Manifest {
    /// Load the drive's state, or start fresh when there is none.
    ///
    /// A manifest that cannot be read, was written by a newer exporter, or does
    /// not describe the database currently on the drive is discarded with a
    /// warning: merging against state we cannot trust would resurrect playlists
    /// the drive no longer has.
    pub fn load(destination: &Path, report: &mut ExportReport) -> Self {
        let path = destination.join(MANIFEST_PATH);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            // No state of ours. A database beside it was written by something
            // else — rekordbox, an older Mixar, a partly deleted drive — and
            // this run replaces it, so say so instead of silently dropping
            // playlists that cannot be read back.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if read_database(destination).is_some() {
                    report.warnings.push(
                        "device database has no Mixar export state beside it; rebuilding it from this export"
                            .to_string(),
                    );
                }
                return Self::default();
            }
            Err(err) => {
                report.warnings.push(format!(
                    "export state unreadable, rebuilding the drive ({err})"
                ));
                return Self::default();
            }
        };
        let manifest: Self = match serde_json::from_slice(&bytes) {
            Ok(manifest) => manifest,
            Err(err) => {
                report.warnings.push(format!(
                    "export state unreadable, rebuilding the drive ({err})"
                ));
                return Self::default();
            }
        };
        if manifest.version > MANIFEST_VERSION {
            report.warnings.push(
                "export state was written by a newer Mixar; rebuilding the drive".to_string(),
            );
            return Self::default();
        }
        if !manifest.paths_are_inside_the_destination() {
            report.warnings.push(
                "export state names files outside the destination; rebuilding the drive from this export"
                    .to_string(),
            );
            return Self::default();
        }

        match read_database(destination) {
            Some(database) => {
                if database.len() as u64 != manifest.pdb_len
                    || fingerprint(&database) != manifest.pdb_fingerprint
                {
                    report.warnings.push(
                        "device database changed outside Mixar; rebuilding it from this export"
                            .to_string(),
                    );
                    return Self::default();
                }
                manifest
            }
            None => {
                report
                    .warnings
                    .push("device database missing; rebuilding it from this export".to_string());
                Self::default()
            }
        }
    }

    /// Whether every path in the state stays under the export root.
    ///
    /// Checked once on load, which is what makes the paths handed out by
    /// [`Self::previous_placement`], [`Self::artwork_for`] and
    /// [`Self::writer_inputs`] safe to join onto the destination.
    fn paths_are_inside_the_destination(&self) -> bool {
        self.tracks.iter().all(|entry| {
            is_device_path(&entry.track.device_audio_path)
                && is_device_path(&entry.track.analyze_path)
        }) && self
            .artworks
            .iter()
            .all(|artwork| is_device_path(&artwork.path))
    }

    /// Source track id → on-drive path for everything a previous run wrote.
    ///
    /// The planner reuses these paths for the same track so re-exporting a
    /// collection never duplicates its files, and reserves the rest so a new
    /// track cannot take a path an unselected track already occupies. Every path
    /// was checked against the destination root when the state was loaded.
    pub fn previous_placement(&self) -> HashMap<String, String> {
        self.tracks
            .iter()
            .map(|entry| {
                (
                    entry.source_id.clone(),
                    entry.track.device_audio_path.clone(),
                )
            })
            .collect()
    }

    /// Id and path for a cover, reusing the previous entry with identical bytes.
    ///
    /// Does no file I/O: the caller writes (or verifies) the file at `path`.
    pub fn artwork_for(&mut self, jpeg: &[u8]) -> (u32, String) {
        let digest = fingerprint(jpeg);
        let len = jpeg.len() as u64;
        if let Some(existing) = self
            .artworks
            .iter()
            .find(|artwork| artwork.fingerprint == digest && artwork.len == len)
        {
            return (existing.id, existing.path.clone());
        }

        let id = self
            .artworks
            .iter()
            .map(|artwork| artwork.id)
            .max()
            .unwrap_or(0)
            + 1;
        let path = super::artwork_path(id);
        self.artworks.push(ManifestArtwork {
            id,
            path: path.clone(),
            fingerprint: digest,
            len,
        });
        (id, path)
    }

    /// Register this run's tracks, replacing previous rows for the same source
    /// track (by on-drive path) and keeping everything else.
    pub fn merge_tracks(&mut self, run: Vec<ManifestTrack>) {
        // One pass over the state instead of a scan per run track: a drive
        // accumulates thousands of tracks and each run rewrites all of them.
        let mut positions: HashMap<String, usize> = self
            .tracks
            .iter()
            .enumerate()
            .map(|(position, entry)| (entry.track.device_audio_path.clone(), position))
            .collect();
        for entry in run {
            match positions.get(&entry.track.device_audio_path).copied() {
                Some(position) => self.tracks[position] = entry,
                None => {
                    positions.insert(entry.track.device_audio_path.clone(), self.tracks.len());
                    self.tracks.push(entry);
                }
            }
        }
    }

    /// Replace the previous version of a collection's playlist, or add it.
    pub fn record_playlist(&mut self, name: &str, source: Option<&str>, entries: Vec<String>) {
        let matches = |existing: &ManifestPlaylist| match source {
            Some(source) => existing.source.as_deref() == Some(source),
            None => existing.source.is_none() && existing.name == name,
        };
        match self.playlists.iter_mut().find(|existing| matches(existing)) {
            Some(existing) => {
                existing.name = name.to_string();
                existing.entries = entries;
            }
            None => self.playlists.push(ManifestPlaylist {
                name: name.to_string(),
                source: source.map(str::to_string),
                entries,
            }),
        }
    }

    /// Writer inputs for the merged state: tracks in manifest order, playlists
    /// resolved to track indices (entries whose track is gone are dropped), and
    /// the drive's covers.
    pub fn writer_inputs(&self) -> (Vec<CdjTrack>, Vec<CdjPlaylist>, Vec<CdjArtwork>) {
        let index: HashMap<&str, usize> = self
            .tracks
            .iter()
            .enumerate()
            .map(|(position, entry)| (entry.track.device_audio_path.as_str(), position))
            .collect();

        let tracks = self
            .tracks
            .iter()
            .map(|entry| entry.track.clone())
            .collect();
        let playlists = self
            .playlists
            .iter()
            .map(|playlist| CdjPlaylist {
                name: playlist.name.clone(),
                entries: playlist
                    .entries
                    .iter()
                    .filter_map(|path| index.get(path.as_str()).copied())
                    .collect(),
            })
            .collect();
        let artworks = self
            .artworks
            .iter()
            .map(|artwork| CdjArtwork {
                id: artwork.id,
                path: artwork.path.clone(),
            })
            .collect();
        (tracks, playlists, artworks)
    }

    /// Record the database we just wrote and persist the manifest.
    ///
    /// A manifest write failure is a warning: the database is valid, the next
    /// run just rebuilds instead of merging.
    pub fn save(
        &mut self,
        destination: &Path,
        database: &[u8],
        report: &mut ExportReport,
    ) -> std::io::Result<()> {
        self.version = MANIFEST_VERSION;
        self.pdb_fingerprint = fingerprint(database);
        self.pdb_len = database.len() as u64;

        let path = destination.join(MANIFEST_PATH);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        if let Err(err) = std::fs::write(&path, bytes) {
            report
                .warnings
                .push(format!("export state not written ({err})"));
            return Err(err);
        }
        Ok(())
    }
}

/// The database currently on the drive, when there is one.
fn read_database(destination: &Path) -> Option<Vec<u8>> {
    std::fs::read(destination.join(super::PDB_PATH)).ok()
}

/// FNV-1a over the bytes: stable across runs and cheap enough to hash a database
/// and a thumbnail, with no extra dependency.
/// Whether a path from the state file stays inside the export root.
///
/// The state file lives on a drive the user plugs in, so everything it names is
/// as untrusted as the rest of the drive: a `device_audio_path` of
/// `/../../.config/autostart/x` would otherwise be joined onto the destination
/// and written outside it. The fingerprint guard is no defence — whoever
/// rewrote the manifest controls the database it fingerprints too. Absolute
/// (leading `/`), forward slashes only, no `.`/`..` and no empty component.
fn is_device_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .skip(1)
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

pub(super) fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdj::model::CdjFileType;

    fn track(source: &str, path: &str) -> ManifestTrack {
        ManifestTrack {
            source_id: source.to_string(),
            track: CdjTrack {
                title: source.to_string(),
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                genre: None,
                bpm: Some(128.0),
                key_id: None,
                duration_secs: 200,
                file_size: 1_000,
                bitrate_kbps: None,
                sample_rate_hz: 44_100,
                bit_depth: 16,
                file_type: CdjFileType::Wav,
                artwork_id: 0,
                date_added: None,
                device_audio_path: path.to_string(),
                analyze_path: "/PIONEER/USBANLZ/P000/0000000/ANLZ0000.DAT".to_string(),
            },
        }
    }

    #[test]
    fn tracks_merge_in_place_and_append() {
        let mut manifest = Manifest::default();
        manifest.merge_tracks(vec![track("a", "/Contents/a.wav")]);
        let mut updated = track("a", "/Contents/a.wav");
        updated.track.title = "Renamed".to_string();
        manifest.merge_tracks(vec![updated, track("b", "/Contents/b.wav")]);

        assert_eq!(manifest.tracks.len(), 2);
        assert_eq!(manifest.tracks[0].track.title, "Renamed");
        assert_eq!(manifest.tracks[1].source_id, "b");
    }

    #[test]
    fn a_playlist_replaces_the_same_collection_but_keeps_others() {
        let mut manifest = Manifest::default();
        manifest.record_playlist("Warmup", Some("pl-1"), vec!["/a".into()]);
        manifest.record_playlist("Sets", Some("fo-1"), vec!["/b".into()]);
        manifest.record_playlist("Warmup", Some("pl-1"), vec!["/c".into()]);

        assert_eq!(manifest.playlists.len(), 2);
        assert_eq!(manifest.playlists[0].entries, vec!["/c".to_string()]);
        assert_eq!(manifest.playlists[1].name, "Sets");
    }

    #[test]
    fn identical_covers_reuse_their_id() {
        let mut manifest = Manifest::default();
        let (first, path) = manifest.artwork_for(b"jpeg-one");
        let (again, again_path) = manifest.artwork_for(b"jpeg-one");
        let (second, _) = manifest.artwork_for(b"jpeg-two");

        assert_eq!(first, 1);
        assert_eq!(again, 1);
        assert_eq!(again_path, path);
        assert_eq!(second, 2);
        assert_eq!(manifest.artworks.len(), 2);
    }

    #[test]
    fn writer_inputs_resolve_paths_and_drop_missing_members() {
        let mut manifest = Manifest::default();
        manifest.merge_tracks(vec![
            track("a", "/Contents/a.wav"),
            track("b", "/Contents/b.wav"),
        ]);
        manifest.record_playlist(
            "Mix",
            Some("pl-1"),
            vec!["/Contents/b.wav".into(), "/Contents/gone.wav".into()],
        );

        let (tracks, playlists, artworks) = manifest.writer_inputs();
        assert_eq!(tracks.len(), 2);
        assert!(artworks.is_empty());
        assert_eq!(playlists[0].entries, vec![1]);
    }

    #[test]
    fn a_manifest_without_its_database_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let mut report = ExportReport::default();
        let mut manifest = Manifest::default();
        manifest.merge_tracks(vec![track("a", "/Contents/a.wav")]);
        manifest
            .save(dir.path(), b"database", &mut report)
            .expect("save manifest");

        // Saved with a database that is not on the drive.
        let loaded = Manifest::load(dir.path(), &mut report);
        assert!(loaded.tracks.is_empty());
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn a_database_changed_outside_mixar_discards_the_state() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("PIONEER/rekordbox")).unwrap();
        std::fs::write(dir.path().join(crate::cdj::PDB_PATH), b"first").unwrap();

        let mut report = ExportReport::default();
        let mut manifest = Manifest::default();
        manifest.merge_tracks(vec![track("a", "/Contents/a.wav")]);
        manifest.save(dir.path(), b"first", &mut report).unwrap();
        assert_eq!(Manifest::load(dir.path(), &mut report).tracks.len(), 1);

        // Rekordbox (or anything else) rewrote the database.
        std::fs::write(dir.path().join(crate::cdj::PDB_PATH), b"second").unwrap();
        let loaded = Manifest::load(dir.path(), &mut report);
        assert!(loaded.tracks.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("changed outside Mixar")));
    }

    #[test]
    fn a_corrupt_manifest_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("PIONEER/rekordbox")).unwrap();
        std::fs::write(dir.path().join(MANIFEST_PATH), b"{not json").unwrap();

        let mut report = ExportReport::default();
        let manifest = Manifest::load(dir.path(), &mut report);
        assert!(manifest.tracks.is_empty());
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn a_database_without_our_state_is_reported_before_being_rebuilt() {
        let dir = tempfile::tempdir().unwrap();
        let mut report = ExportReport::default();
        // A drive with no database at all is a plain first export.
        assert!(Manifest::load(dir.path(), &mut report).tracks.is_empty());
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);

        // A database somebody else wrote (rekordbox, an older Mixar) is about to
        // be replaced, which the report has to say.
        std::fs::create_dir_all(dir.path().join("PIONEER/rekordbox")).unwrap();
        std::fs::write(
            dir.path().join(crate::cdj::PDB_PATH),
            b"rekordbox wrote this",
        )
        .unwrap();
        let manifest = Manifest::load(dir.path(), &mut report);

        assert!(manifest.tracks.is_empty());
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("no Mixar export state")),
            "warnings: {:?}",
            report.warnings
        );
    }

    #[test]
    fn state_that_names_files_outside_the_destination_is_discarded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("PIONEER/rekordbox")).unwrap();
        std::fs::write(dir.path().join(crate::cdj::PDB_PATH), b"database").unwrap();

        // What a tampered drive could carry. The fingerprint matches the
        // database it ships with, so only the path check stands between this
        // and a file written outside the export root.
        for evil in [
            "/../../tmp/mixar-escaped.wav",
            "/Contents/../outside.wav",
            "/Contents\\..\\outside.wav",
            "Contents/no-leading-slash.wav",
            "/Contents//double-slash.wav",
        ] {
            let mut report = ExportReport::default();
            let mut manifest = Manifest::default();
            manifest.merge_tracks(vec![track("evil", evil)]);
            manifest.save(dir.path(), b"database", &mut report).unwrap();

            let mut report = ExportReport::default();
            let loaded = Manifest::load(dir.path(), &mut report);

            assert!(loaded.tracks.is_empty(), "{evil} was accepted");
            assert!(
                loaded.previous_placement().is_empty(),
                "{evil} was accepted"
            );
            assert!(
                report
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("outside the destination")),
                "{evil}: {:?}",
                report.warnings
            );
        }
    }

    #[test]
    fn fingerprints_are_stable_and_distinguish_bytes() {
        assert_eq!(fingerprint(b"same"), fingerprint(b"same"));
        assert_ne!(fingerprint(b"same"), fingerprint(b"diff"));
    }
}
