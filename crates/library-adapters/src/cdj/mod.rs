//! Pioneer CDJ/XDJ USB export.
//!
//! Writes the layout Rekordbox puts on a USB drive, so CDJ/XDJ players can
//! browse and play a Mixar library without Rekordbox:
//!
//! ```text
//! <destination>/
//!   Contents/<Artist>/<Album>/<file>            audio (folder collections mirror their tree)
//!   PIONEER/rekordbox/export.pdb                DeviceSQL database: tracks + playlists
//!   PIONEER/USBANLZ/P###/########/ANLZ0000.DAT  beat grid, preview waveforms, cue lists
//!   PIONEER/USBANLZ/P###/########/ANLZ0000.EXT  detail waveforms, extended cue lists
//! ```
//!
//! The database is the legacy DeviceSQL format every CDJ/XDJ generation reads;
//! the analysis directories are derived from the on-drive audio path with the
//! algorithm the hardware itself uses ([`CdjExporter::analysis_dir`]).
//!
//! # Fidelity
//!
//! Written: title, artist, album, genre, BPM, key, duration, bitrate, sample
//! rate, bit depth, file type, date added, playlists (device playlist tree),
//! beat grid, waveforms (preview + detail), hot cues, memory cues, saved loops,
//! and one cover thumbnail per distinct artwork (`PIONEER/Artwork/<id>/a1.jpg`,
//! scaled to the 80 px the device database expects).
//!
//! Dropped, because Mixar's library model has no equivalent or the format would
//! need data the export source does not expose:
//!
//! - the higher-resolution `_m` artwork variant newer Rekordbox releases add
//!   next to the primary thumbnail;
//! - track rating, release year, track/disc numbers, comments, color labels;
//! - the OneLibrary format (`exportLibrary.db`) and the `.2EX` 3-band waveform
//!   files that newer players (CDJ-3000X, XDJ-AZ, OPUS-QUAD, OMNIS-DUO) prefer;
//!   those units read the legacy format, they just show lower-resolution
//!   waveforms;
//! - device settings files (`MYSETTING.DAT` and friends), which belong to the
//!   player, not the export;
//! - stems and sampler banks, which have no device representation.
//!
//! Analysis is recomputed at export time when needed: the detail waveform tier
//! (150 entries per second) is a different resolution from the one the library
//! stores for its own displays, so each exported track is decoded once.
//!
//! # Verification
//!
//! The writer is checked the way a reviewer can repeat: every file is
//! re-parsed with `rekordcrate` (an independent implementation of both
//! formats) in `tests/cdj_export.rs`, the fixed tables are byte-compared with
//! two Rekordbox-written exports during development, and the layout rules are
//! pinned by unit tests. Physical CDJ/XDJ validation has **not** been done for
//! this implementation — the format details come from public
//! reverse-engineering work, credited in `pdb.rs`, `anlz.rs` and `path_hash.rs`.
//!
//! # Modules
//!
//! [`CdjExporter`] is the entry point embedders want: it plans, copies and
//! writes a whole drive. The format writers below are public so a tool can
//! build one artifact without a drive — [`pdb::build`] for the database,
//! `anlz::dat_bytes` / `anlz::ext_bytes` for a track's analysis files (all
//! taking the flat [`CdjTrack`] / [`CdjPlaylist`] / [`AnlzInput`] inputs).

pub mod anlz;
mod device_sql;
mod keys;
mod layout;
mod manifest;
mod model;
mod path_hash;
pub mod pdb;
mod plan;
pub mod waveform;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub use model::{AnlzInput, CdjArtwork, CdjFileType, CdjPlaylist, CdjTrack};
pub use pdb::PdbError;

use library_core::{
    path_label, ExportCue, ExportOptions, ExportReport, ExportSource, Exportable, LibraryError,
    Result, TrackExportData, EXPORT_WAVEFORM_ENTRIES_PER_SECOND,
};

/// Relative path of the device database inside an export root.
pub const PDB_PATH: &str = "PIONEER/rekordbox/export.pdb";

/// Writer for the Pioneer DeviceSQL export layout.
#[derive(Clone, Copy, Debug, Default)]
pub struct CdjExporter;

impl CdjExporter {
    /// Analysis directory (relative to the export root, starting with a slash)
    /// that CDJ/XDJ hardware derives for an on-drive audio path.
    pub fn analysis_dir(device_audio_path: &str) -> String {
        path_hash::anlz_dir(device_audio_path)
    }
}

impl Exportable for CdjExporter {
    fn name(&self) -> &'static str {
        "cdj-usb"
    }

    fn export(
        &self,
        source: &dyn ExportSource,
        destination: &Path,
        options: &ExportOptions,
    ) -> Result<ExportReport> {
        let mut report = ExportReport::default();
        let mut manifest = manifest::Manifest::load(destination, &mut report);
        let plan = plan::build(source, options, &manifest.previous_placement())?;
        report.folders = plan.folders;
        report.warnings.extend(plan.warnings.clone());

        // Plan order is database order; skipped tracks keep no slot, so
        // playlist entries are remapped through `writer_index`.
        let mut run_tracks: Vec<manifest::ManifestTrack> = Vec::with_capacity(plan.tracks.len());
        let mut writer_index: Vec<Option<usize>> = Vec::with_capacity(plan.tracks.len());
        let mut written_paths: Vec<String> = Vec::with_capacity(plan.tracks.len());
        // One write per distinct cover per run: an album's tracks share it.
        let mut written_artwork: HashSet<String> = HashSet::new();
        for track in &plan.tracks {
            match export_track(
                source,
                destination,
                options,
                track,
                &mut manifest,
                &mut written_artwork,
                &mut report,
            ) {
                Ok(Some(cdj_track)) => {
                    writer_index.push(Some(run_tracks.len()));
                    written_paths.push(cdj_track.device_audio_path.clone());
                    run_tracks.push(manifest::ManifestTrack {
                        source_id: track.track_id.to_string(),
                        track: cdj_track,
                    });
                }
                Ok(None) => writer_index.push(None),
                Err(err) => {
                    report
                        .errors
                        .push(format!("{}: {err}", path_label(&track.source_path)));
                    writer_index.push(None);
                }
            }
        }

        // This run's playlists, by on-drive path so the merge never depends on
        // track indices.
        let run_playlists: Vec<(String, Option<String>, Vec<String>)> = plan
            .playlists
            .iter()
            .map(|playlist| {
                let entries = playlist
                    .entries
                    .iter()
                    .filter_map(|index| {
                        writer_index[*index].map(|slot| written_paths[slot].clone())
                    })
                    .collect();
                (playlist.name.clone(), playlist.source.clone(), entries)
            })
            .collect();

        // Merge into what the drive already carries: the UI exports one
        // collection at a time, and a run must not drop the others.
        let written = run_tracks.len();
        manifest.merge_tracks(run_tracks);
        for (name, source_id, entries) in &run_playlists {
            manifest.record_playlist(name, source_id.as_deref(), entries.clone());
        }

        let (tracks, playlists, artworks) = manifest.writer_inputs();
        let database = pdb::build(&tracks, &playlists, &artworks, &today()).map_err(|err| {
            LibraryError::Backend {
                backend: "cdj-export",
                message: err.to_string(),
            }
        })?;
        write_file(&destination.join(PDB_PATH), &database)?;
        let _ = manifest.save(destination, &database, &mut report);

        report.tracks = written;
        report.playlists = run_playlists.len();
        Ok(report)
    }
}

/// Export one planned track: copy the audio file and write its analysis files.
///
/// Returns `Ok(None)` for tracks the device cannot play; those are reported as
/// warnings, not failures.
fn export_track(
    source: &dyn ExportSource,
    destination: &Path,
    options: &ExportOptions,
    track: &plan::PlanTrack,
    manifest: &mut manifest::Manifest,
    written_artwork: &mut HashSet<String>,
    report: &mut ExportReport,
) -> Result<Option<CdjTrack>> {
    let label = path_label(&track.source_path);
    let extension = track
        .source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let file_type = CdjFileType::from_extension(extension);
    if file_type == CdjFileType::Unknown {
        report
            .warnings
            .push(format!("{label}: not a file type CDJ/XDJ players play"));
        return Ok(None);
    }

    let file_size = fs::metadata(&track.source_path)
        .map_err(|err| export_error(&track.source_path, err))?
        .len();
    let device_path = destination.join(relative_path(&track.device_audio_path));
    copy_audio_file(
        &track.source_path,
        &device_path,
        file_size,
        options.refresh_files,
        report,
    )?;

    let data = match source.export_data(&track.track_id) {
        Ok(data) => data,
        Err(err) => {
            report
                .warnings
                .push(format!("{label}: analysis unavailable ({err})"));
            None
        }
    };
    write_analysis_files(destination, track, data.as_ref(), options, report)?;
    let artwork_id = match data.as_ref().and_then(|data| data.artwork.as_ref()) {
        Some(artwork) => {
            // Ids come from the drive's state, so a cover keeps its id (and its
            // file) across runs instead of restarting at 1.
            let (id, path) = manifest.artwork_for(&artwork.jpeg);
            if !written_artwork.insert(path.clone()) {
                id
            } else {
                match write_artwork_file(destination, &path, &artwork.jpeg, options.refresh_files) {
                    Ok(copied) => {
                        report.bytes_copied += copied;
                        id
                    }
                    Err(err) => {
                        report
                            .warnings
                            .push(format!("{label}: artwork not written ({err})"));
                        0
                    }
                }
            }
        }
        None => 0,
    };

    let metadata = &track.metadata;
    let bpm = data
        .as_ref()
        .and_then(|data| data.bpm)
        .or(metadata.bpm)
        .filter(|bpm| bpm.is_finite() && *bpm > 0.0);
    Ok(Some(CdjTrack {
        title: metadata.title.clone().unwrap_or_else(|| label.clone()),
        artist: metadata.artist.clone().unwrap_or_default(),
        album: metadata.album.clone().unwrap_or_default(),
        genre: metadata
            .genre
            .clone()
            .filter(|genre| !genre.trim().is_empty()),
        bpm,
        key_id: metadata.key.as_deref().and_then(keys::id_for_musical),
        duration_secs: duration_secs(metadata.duration_ms, data.as_ref()),
        file_size,
        bitrate_kbps: metadata.bitrate_kbps,
        sample_rate_hz: metadata.sample_rate.unwrap_or(44_100).max(1),
        bit_depth: data
            .as_ref()
            .and_then(|data| data.bit_depth)
            .unwrap_or(16)
            .max(1),
        file_type,
        artwork_id,
        date_added: data.as_ref().and_then(|data| data.added_at.clone()),
        device_audio_path: track.device_audio_path.clone(),
        analyze_path: track.analyze_path.clone(),
    }))
}

/// On-drive path of an artwork thumbnail, following Rekordbox's layout.
fn artwork_path(id: u32) -> String {
    format!("/PIONEER/Artwork/{id:05}/a1.jpg")
}

/// Write one cover file, reusing an identical one already on the drive unless
/// `refresh` is set. Returns the bytes copied.
fn write_artwork_file(
    destination: &Path,
    device_path: &str,
    jpeg: &[u8],
    refresh: bool,
) -> Result<u64> {
    let path = destination.join(relative_path(device_path));
    // Artwork ids identify a slot, not an image: compare the bytes so a
    // re-export never leaves a stale cover under a reused id.
    let already_there = !refresh
        && fs::read(&path)
            .map(|on_drive| on_drive == jpeg)
            .unwrap_or(false);
    if already_there {
        return Ok(0);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| export_error(parent, err))?;
    }
    fs::write(&path, jpeg).map_err(|err| export_error(&path, err))?;
    Ok(jpeg.len() as u64)
}

/// Copy `source` to `device_path` unless an identically sized file is already
/// there and `refresh` is false.
fn copy_audio_file(
    source: &Path,
    device_path: &Path,
    file_size: u64,
    refresh: bool,
    report: &mut ExportReport,
) -> Result<()> {
    let already_there = !refresh
        && fs::metadata(device_path)
            .map(|metadata| metadata.len() == file_size)
            .unwrap_or(false);
    if already_there {
        report.tracks_reused += 1;
        return Ok(());
    }

    if let Some(parent) = device_path.parent() {
        fs::create_dir_all(parent).map_err(|err| export_error(parent, err))?;
    }
    fs::copy(source, device_path).map_err(|err| export_error(device_path, err))?;
    report.bytes_copied += file_size;
    Ok(())
}

/// Write the track's `.DAT`/`.EXT` analysis files, reusing existing ones
/// unless `refresh_files` asks for a rewrite.
fn write_analysis_files(
    destination: &Path,
    track: &plan::PlanTrack,
    data: Option<&TrackExportData>,
    options: &ExportOptions,
    report: &mut ExportReport,
) -> Result<()> {
    let dir = destination.join(relative_path(&CdjExporter::analysis_dir(
        &track.device_audio_path,
    )));
    let dat = dir.join("ANLZ0000.DAT");
    let ext = dir.join("ANLZ0000.EXT");
    if !options.refresh_files && dat.is_file() && ext.is_file() {
        return Ok(());
    }

    let Some(data) = data else {
        report.warnings.push(format!(
            "{}: no analysis data; the player analyzes it on first load",
            path_label(&track.source_path)
        ));
        return Ok(());
    };

    let (hot_cues, memory_cues) = split_cues(&data.cues);
    let input = AnlzInput {
        device_audio_path: &track.device_audio_path,
        bpm: data.bpm.or(track.metadata.bpm),
        duration_ms: duration_ms(&track.metadata, data),
        beat_grid: data.beat_grid.as_ref(),
        waveform_overview: &data.waveform_overview,
        waveform_detail: &data.waveform_detail,
        hot_cues: &hot_cues,
        memory_cues: &memory_cues,
    };
    fs::create_dir_all(&dir).map_err(|err| export_error(&dir, err))?;
    fs::write(&dat, anlz::dat_bytes(&input)).map_err(|err| export_error(&dat, err))?;
    fs::write(&ext, anlz::ext_bytes(&input)).map_err(|err| export_error(&ext, err))?;
    Ok(())
}

/// Split cues into the device's hot-cue and memory-cue lists, each in slot
/// order (hot cues first, then memory cues and loops).
fn split_cues(cues: &[ExportCue]) -> (Vec<ExportCue>, Vec<ExportCue>) {
    let mut hot: Vec<ExportCue> = Vec::new();
    let mut memory: Vec<ExportCue> = Vec::new();
    for cue in cues {
        if cue.hot_cue_slot.is_some() {
            hot.push(cue.clone());
        } else {
            memory.push(cue.clone());
        }
    }
    hot.sort_by_key(|cue| cue.hot_cue_slot);
    memory.sort_by_key(|cue| (cue.position_ms, cue.loop_end_ms));
    (hot, memory)
}

/// Whole seconds of audio, from tag metadata with the analysis tier as a
/// fallback.
fn duration_secs(duration_ms: Option<i32>, data: Option<&TrackExportData>) -> u32 {
    let from_tags = duration_ms.unwrap_or_default().max(0) as u32 / 1_000;
    if from_tags > 0 {
        return from_tags;
    }
    match data {
        Some(data) if !data.waveform_detail.is_empty() => u32::try_from(
            data.waveform_detail.len() as u64 / u64::from(EXPORT_WAVEFORM_ENTRIES_PER_SECOND),
        )
        .unwrap_or(u32::MAX),
        _ => 0,
    }
}

/// Milliseconds of audio to describe in the analysis files.
fn duration_ms(metadata: &library_core::TrackMetadata, data: &TrackExportData) -> u32 {
    match metadata.duration_ms {
        Some(duration) if duration > 0 => duration as u32,
        _ => {
            let entries = data.waveform_detail.len() as u64;
            let from_waveform =
                entries.saturating_mul(1_000) / u64::from(EXPORT_WAVEFORM_ENTRIES_PER_SECOND);
            u32::try_from(from_waveform).unwrap_or(u32::MAX)
        }
    }
}

/// Strip the leading slash from an on-drive path so it can be joined onto the
/// export root.
fn relative_path(device_path: &str) -> PathBuf {
    PathBuf::from(device_path.trim_start_matches('/'))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| export_error(parent, err))?;
    }
    fs::write(path, bytes).map_err(|err| export_error(path, err))
}

fn export_error(path: &Path, err: std::io::Error) -> LibraryError {
    LibraryError::Backend {
        backend: "cdj-export",
        message: format!("{}: {err}", path.display()),
    }
}

/// Today's date in the `YYYY-MM-DD` form the device database stores.
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
