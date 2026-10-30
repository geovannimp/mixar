//! Library worker thread: drains cmd bus and publishes evt.

use crate::bus::LibraryBuses;
use crate::{HotCueRecord, LibraryError, LibraryManager, LoopRecord};
use bus_api::AnalyzeTrack;
use bus_api::BeatGrid;
use bus_api::BeatGridChanged;
use bus_api::DeleteTrackHotCue;
use bus_api::DeleteTrackLoop;
use bus_api::Error;
use bus_api::Event;
use bus_api::GenerateStems;
use bus_api::HotCue;
use bus_api::HotCuesChanged;
use bus_api::LoopsChanged;
use bus_api::RefreshTrack;
use bus_api::SaveBeatGrid;
use bus_api::SaveTrackHotCue;
use bus_api::SaveTrackLoop;
use bus_api::SavedLoop;
use bus_api::TrackAnalyzed;
use bus_api::TrackProgress;
use bus_api::TrackSummary;
use bus_api::TrackUpdated;
use library_core::{AnalyzeTrackOptions, AudioSource, Library, TrackId};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// How long each worker blocks (on its cmd bus or its stem queue) before
/// re-checking the shutdown flag.
///
/// This bounds how long `Drop` waits to join an idle worker, and every
/// `LibraryTransport` drop pays it (15+ per `host-flutter` test run), so it is
/// kept short. The timeout is a no-op poll in both loops, which differ in how it
/// is reported: the cmd loop's subscriber returns `Ok(None)`, the stem loop's
/// `std::sync::mpsc` receiver returns `Err(RecvTimeoutError::Timeout)`.
const RECV_TIMEOUT: Duration = Duration::from_millis(25);

/// Host-owned library cmd worker (JoinHandle must not live inside `Mutex<LibraryManager>`).
pub struct LibraryWorker {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    stem_handle: Option<JoinHandle<()>>,
}

impl Drop for LibraryWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        // Join the serial stem thread too so a mid-shutdown Demucs pass finishes
        // writing its cache instead of leaving a partial file. No active job in
        // the common case means this returns immediately.
        if let Some(handle) = self.stem_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn the cmd-draining worker using buses attached to `library`.
///
/// Errors if [`LibraryManager::set_buses`] was not called, or if the worker
/// fails to subscribe to the cmd bus.
pub fn spawn_library_worker(library: Arc<Mutex<LibraryManager>>) -> crate::Result<LibraryWorker> {
    let buses = attach_buses(&library)?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let library_handle = Arc::clone(&library);
    let shutdown_flag = Arc::clone(&shutdown);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<std::result::Result<(), String>>();
    let handle = thread::spawn(move || {
        worker_thread_loop(buses, library_handle, shutdown_flag, ready_tx);
    });
    ready_rx
        .recv()
        .map_err(|_| LibraryError::Io(std::io::Error::other("library worker failed to start")))?
        .map_err(|e| LibraryError::Io(std::io::Error::other(e)))?;

    // Serial stem-job thread: one Demucs/ONNX pass at a time. Bulk stem queues
    // (collection-wide "generate stems") would otherwise spawn one thread per
    // cmd and OOM.
    let (stem_tx, stem_rx) = std::sync::mpsc::channel::<String>();
    attach_buses(&library)?.set_stem_queue(stem_tx);
    let stem_library = Arc::clone(&library);
    let stem_buses = attach_buses(&library)?;
    let stem_shutdown = Arc::clone(&shutdown);
    let stem_handle = thread::Builder::new()
        .name("library-stems".into())
        .spawn(move || stem_thread_loop(stem_rx, stem_library, stem_buses, stem_shutdown))
        .map_err(|e| LibraryError::Io(std::io::Error::other(e.to_string())))?;

    Ok(LibraryWorker {
        shutdown,
        handle: Some(handle),
        stem_handle: Some(stem_handle),
    })
}

/// Clone the buses attached to `library`, or explain why there are none.
fn attach_buses(library: &Arc<Mutex<LibraryManager>>) -> crate::Result<LibraryBuses> {
    let lib = library.lock().unwrap_or_else(|e| e.into_inner());
    lib.buses().ok_or_else(|| LibraryError::Backend {
        backend: "library",
        message: "library buses not attached; call set_buses before spawn_library_worker".into(),
    })
}

pub(crate) fn worker_thread_loop(
    buses: LibraryBuses,
    library: Arc<Mutex<LibraryManager>>,
    shutdown: Arc<AtomicBool>,
    ready: std::sync::mpsc::Sender<Result<(), String>>,
) {
    let mut cmd = match buses.subscribe_cmd() {
        Ok(cmd) => cmd,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let _ = ready.send(Ok(()));

    while !shutdown.load(Ordering::Relaxed) {
        match cmd.recv_timeout(RECV_TIMEOUT) {
            Ok(Some(event)) => handle_cmd(event, &library, &buses),
            Ok(None) => continue,
            Err(_) => break,
        }
    }
}

fn handle_cmd(event: Event, library: &Arc<Mutex<LibraryManager>>, buses: &LibraryBuses) {
    match event {
        Event::AnalyzeTrack(cmd) => handle_analyze(cmd, library, buses),
        Event::GenerateStems(cmd) => handle_generate_stems(cmd, library, buses),
        Event::RefreshTrack(cmd) => handle_refresh_track(cmd, library, buses),
        Event::SaveTrackHotCue(cmd) => handle_save_hot_cue(cmd, library, buses),
        Event::DeleteTrackHotCue(cmd) => handle_delete_hot_cue(cmd, library, buses),
        Event::SaveTrackLoop(cmd) => handle_save_loop(cmd, library, buses),
        Event::DeleteTrackLoop(cmd) => handle_delete_loop(cmd, library, buses),
        Event::SaveBeatGrid(cmd) => handle_save_beat_grid(cmd, library, buses),
        // Egress events published onto cmd by mistake, plus engine and controller
        // commands that are not the library worker's business.
        _ => {}
    }
}

fn handle_refresh_track(
    cmd: RefreshTrack,
    library: &Arc<Mutex<LibraryManager>>,
    buses: &LibraryBuses,
) {
    let track_id = cmd.track_id;
    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        match lib.get_track(&id) {
            Ok(Some(source)) => match track_summary(&source) {
                Some(track) => {
                    let cues = lib.list_track_hot_cues(&id);
                    let loops = lib.list_track_loops(&id);
                    Ok((track, cues, loops))
                }
                None => Err("Only file tracks can be refreshed.".to_string()),
            },
            Ok(None) => Err(format!("Track not found: {track_id}")),
            Err(err) => Err(err.to_string()),
        }
    };

    match result {
        Ok((track, cues, loops)) => {
            let _ = buses.publish_evt(Event::TrackUpdated(TrackUpdated { track }));
            match cues {
                Ok(rows) => {
                    let _ = buses.publish_evt(Event::HotCuesChanged(HotCuesChanged {
                        track_id: track_id.clone(),
                        hot_cues: rows.into_iter().map(hot_cue_from_record).collect(),
                    }));
                }
                Err(err) => publish_error(buses, err.to_string(), Some(track_id.clone())),
            }
            match loops {
                Ok(rows) => {
                    let _ = buses.publish_evt(Event::LoopsChanged(LoopsChanged {
                        track_id: track_id.clone(),
                        loops: rows.into_iter().map(saved_loop_from_record).collect(),
                    }));
                }
                Err(err) => publish_error(buses, err.to_string(), Some(track_id)),
            }
        }
        Err(message) => publish_error(buses, message, Some(track_id)),
    }
}

fn handle_analyze(cmd: AnalyzeTrack, library: &Arc<Mutex<LibraryManager>>, buses: &LibraryBuses) {
    let AnalyzeTrack { track_id, force } = cmd;
    let duration = *buses
        .analysis_duration_arc()
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let options = AnalyzeTrackOptions {
        force,
        analysis_duration: duration,
    };

    let _ = buses.publish_evt(Event::TrackProgress(TrackProgress {
        track_id: track_id.clone(),
        phase: "analyze".into(),
        fraction: None,
    }));

    let result =
        LibraryManager::analyze_track_off_mutex(library, &TrackId::new(track_id.clone()), options);

    match result {
        Ok(source) => match track_summary(&source) {
            Some(track) => {
                let _ = buses.publish_evt(Event::TrackAnalyzed(TrackAnalyzed { track }));
            }
            None => publish_error(
                buses,
                "Only file tracks can be analyzed.".into(),
                Some(track_id),
            ),
        },
        Err(err) => publish_error(buses, err.to_string(), Some(track_id)),
    }
}

/// Queue the standalone stem generation job for one track.
///
/// A no-op when a valid cache already exists, so the UI can fire this without
/// first checking whether the track already has stems.
fn handle_generate_stems(
    cmd: GenerateStems,
    library: &Arc<Mutex<LibraryManager>>,
    buses: &LibraryBuses,
) {
    let track_id = cmd.track_id;

    // Publish the queued phase before enqueueing so the UI shows a loader even
    // if the job turns out to be a no-op against an existing cache.
    let _ = buses.publish_evt(Event::TrackProgress(TrackProgress {
        track_id: track_id.clone(),
        phase: "stems_queued".into(),
        fraction: None,
    }));
    if !buses.queue_stem(track_id.clone()) {
        // No serial queue installed (worker spawned without stem thread, e.g. in
        // some unit-test harnesses): fall back to a one-off thread so the job
        // still runs.
        spawn_stem_ensure_job(Arc::clone(library), buses.clone(), track_id);
    }
}

/// Single stem job: decode/Demucs off the library mutex, then report ready/failed.
fn run_stem_ensure_job(library: &Arc<Mutex<LibraryManager>>, buses: &LibraryBuses, track_id: &str) {
    let stems_root = buses.stems_root();
    let models_root = buses.models_root();
    let format = buses.stems_format();
    let model = buses.stems_model();
    let track_id_for_cb = track_id.to_string();
    let buses_cb = buses.clone();
    let progress: crate::StemProgressFn = Arc::new(move |phase, fraction| {
        let _ = buses_cb.publish_evt(Event::TrackProgress(TrackProgress {
            track_id: track_id_for_cb.clone(),
            phase: phase.to_string(),
            fraction,
        }));
    });
    let result = crate::ensure_track_stems_with_progress(
        library,
        &TrackId::new(track_id),
        &stems_root,
        &models_root,
        &format,
        &model,
        Some(progress),
    );
    match result {
        Ok(()) => {
            let _ = buses.publish_evt(Event::TrackProgress(TrackProgress {
                track_id: track_id.to_string(),
                phase: "stems_ready".into(),
                fraction: Some(1.0),
            }));
        }
        Err(err) => {
            let message = err.to_string();
            let _ = buses.publish_evt(Event::TrackProgress(TrackProgress {
                track_id: track_id.to_string(),
                phase: "stems_failed".into(),
                fraction: None,
            }));
            publish_error(buses, message, Some(track_id.to_string()));
        }
    }
}

/// Drain the serial stem queue one job at a time.
fn stem_thread_loop(
    rx: std::sync::mpsc::Receiver<String>,
    library: Arc<Mutex<LibraryManager>>,
    buses: LibraryBuses,
    shutdown: Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        match rx.recv_timeout(RECV_TIMEOUT) {
            Ok(track_id) => run_stem_ensure_job(&library, &buses, &track_id),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Fallback one-off stem thread for contexts without the serial queue.
fn spawn_stem_ensure_job(
    library: Arc<Mutex<LibraryManager>>,
    buses: LibraryBuses,
    track_id: String,
) {
    let _ = thread::Builder::new()
        .name(format!("stems-ensure-{track_id}"))
        .spawn(move || {
            run_stem_ensure_job(&library, &buses, &track_id);
        });
}

fn handle_save_hot_cue(
    cmd: SaveTrackHotCue,
    library: &Arc<Mutex<LibraryManager>>,
    buses: &LibraryBuses,
) {
    let SaveTrackHotCue {
        track_id,
        slot,
        position_ms,
        loop_length_beats,
        color,
        label,
    } = cmd;

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.save_track_hot_cue(&id, slot, position_ms, loop_length_beats, color, label)
            .and_then(|_| lib.list_track_hot_cues(&id))
    };
    publish_hot_cues_result(buses, track_id, result);
}

fn handle_delete_hot_cue(
    cmd: DeleteTrackHotCue,
    library: &Arc<Mutex<LibraryManager>>,
    buses: &LibraryBuses,
) {
    let DeleteTrackHotCue { track_id, slot } = cmd;

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.delete_track_hot_cue(&id, slot)
            .and_then(|_| lib.list_track_hot_cues(&id))
    };
    publish_hot_cues_result(buses, track_id, result);
}

fn handle_save_loop(
    cmd: SaveTrackLoop,
    library: &Arc<Mutex<LibraryManager>>,
    buses: &LibraryBuses,
) {
    let SaveTrackLoop {
        track_id,
        slot,
        in_ms,
        out_ms,
        label,
        color,
    } = cmd;

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.save_track_loop(&id, slot, in_ms, out_ms, label, color)
            .and_then(|_| lib.list_track_loops(&id))
    };
    publish_loops_result(buses, track_id, result);
}

fn handle_delete_loop(
    cmd: DeleteTrackLoop,
    library: &Arc<Mutex<LibraryManager>>,
    buses: &LibraryBuses,
) {
    let DeleteTrackLoop { track_id, slot } = cmd;

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.delete_track_loop(&id, slot)
            .and_then(|_| lib.list_track_loops(&id))
    };
    publish_loops_result(buses, track_id, result);
}

fn handle_save_beat_grid(
    cmd: SaveBeatGrid,
    library: &Arc<Mutex<LibraryManager>>,
    buses: &LibraryBuses,
) {
    let SaveBeatGrid {
        track_id,
        bpm,
        first_beat_secs,
    } = cmd;

    let id = TrackId::new(track_id.clone());
    // Tag write-back is planned under the lock but executed after it is released:
    // a manual grid is the user telling us this BPM is the right one.
    let mut write_back = None;
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.save_track_beat_grid(&id, bpm, first_beat_secs)
            .and_then(|grid| {
                let source = lib.get_track(&id)?;
                let file = source.as_ref().and_then(AudioSource::file);
                let metadata = source.as_ref().map(AudioSource::metadata);
                if let (Some(file), Some(metadata)) = (file, metadata) {
                    // The manual grid is the user telling us this BPM is the right one.
                    let mut tags = crate::tags::app_managed_tags(metadata);
                    tags.bpm = Some(bpm);
                    write_back = crate::tags::TagWritePlan::new(
                        lib.config().tag_write_back,
                        file.path().to_path_buf(),
                        tags,
                        lib.tag_write_gate(),
                    );
                }
                Ok((grid, source))
            })
    };
    // File I/O for tag write-back runs after the manager lock is released.
    if let Some(plan) = write_back {
        plan.execute();
    }
    publish_beat_grid_result(buses, track_id, result);
}

fn publish_beat_grid_result(
    buses: &LibraryBuses,
    track_id: String,
    result: crate::Result<(crate::waveform::BeatGridSnapshot, Option<AudioSource>)>,
) {
    match result {
        Ok((grid, source)) => {
            let bpm = grid.bpm.unwrap_or(0.0);
            let _ = buses.publish_evt(Event::BeatGridChanged(BeatGridChanged {
                track_id: track_id.clone(),
                beat_grid: BeatGrid {
                    beats: grid.beats,
                    downbeats: grid.downbeats,
                    bpm,
                },
            }));
            if let Some(source) = source {
                if let Some(track) = track_summary(&source) {
                    let _ = buses.publish_evt(Event::TrackUpdated(TrackUpdated { track }));
                }
            }
        }
        Err(err) => publish_error(buses, err.to_string(), Some(track_id)),
    }
}

fn publish_hot_cues_result(
    buses: &LibraryBuses,
    track_id: String,
    result: crate::Result<Vec<HotCueRecord>>,
) {
    match result {
        Ok(rows) => {
            let _ = buses.publish_evt(Event::HotCuesChanged(HotCuesChanged {
                track_id,
                hot_cues: rows.into_iter().map(hot_cue_from_record).collect(),
            }));
        }
        Err(err) => publish_error(buses, err.to_string(), Some(track_id)),
    }
}

fn publish_loops_result(
    buses: &LibraryBuses,
    track_id: String,
    result: crate::Result<Vec<LoopRecord>>,
) {
    match result {
        Ok(rows) => {
            let _ = buses.publish_evt(Event::LoopsChanged(LoopsChanged {
                track_id,
                loops: rows.into_iter().map(saved_loop_from_record).collect(),
            }));
        }
        Err(err) => publish_error(buses, err.to_string(), Some(track_id)),
    }
}

fn hot_cue_from_record(record: HotCueRecord) -> HotCue {
    HotCue {
        slot: record.slot_index,
        position_ms: record.position_ms,
        loop_length_beats: record.loop_length_beats,
        color: record.color,
        label: record.label,
    }
}

fn saved_loop_from_record(record: LoopRecord) -> SavedLoop {
    SavedLoop {
        slot: record.slot_index,
        in_ms: record.in_ms,
        out_ms: record.out_ms,
        label: record.label,
        color: record.color,
    }
}

fn publish_error(buses: &LibraryBuses, message: String, track_id: Option<String>) {
    let _ = buses.publish_evt(Event::Error(Error {
        message,
        track_id,
        deck: None,
    }));
}

fn track_summary(source: &AudioSource) -> Option<TrackSummary> {
    let file = source.file()?;
    let metadata = source.metadata();
    let display_name = match (&metadata.artist, &metadata.title) {
        (Some(artist), Some(title)) => format!("{artist} — {title}"),
        (_, Some(title)) => title.clone(),
        _ => file
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| source.id().as_str().to_string()),
    };
    Some(TrackSummary {
        id: source.id().as_str().to_string(),
        display_name,
        artist: metadata.artist.clone(),
        title: metadata.title.clone(),
        album: metadata.album.clone(),
        genre: metadata.genre.clone(),
        bpm: metadata.bpm,
        key: metadata.key.clone(),
        duration_ms: metadata.duration_ms,
        path: file.path().to_string_lossy().into_owned(),
        isrc: metadata.isrc.clone(),
        rating: metadata.rating,
        comment: metadata.comment.clone(),
        year: metadata.year,
        track_number: metadata.track_number,
        disc_number: metadata.disc_number,
        color_label: metadata.color_label.map(|c| c.as_str().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use library_core::FileAudioSource;
    use std::path::PathBuf;

    #[test]
    fn track_summary_carries_new_metadata_fields() {
        let metadata = library_core::TrackMetadata {
            title: Some("Track".into()),
            rating: Some(4),
            comment: Some("Peak time".into()),
            year: Some(2024),
            track_number: Some(3),
            disc_number: Some(2),
            color_label: Some(library_core::ColorLabel::Aqua),
            ..library_core::TrackMetadata::default()
        };
        let source = AudioSource::File(FileAudioSource::new(
            TrackId::new("t1"),
            PathBuf::from("/music/track.flac"),
            metadata,
        ));

        let summary = track_summary(&source).expect("file track");
        assert_eq!(summary.rating, Some(4));
        assert_eq!(summary.comment.as_deref(), Some("Peak time"));
        assert_eq!(summary.year, Some(2024));
        assert_eq!(summary.track_number, Some(3));
        assert_eq!(summary.disc_number, Some(2));
        assert_eq!(summary.color_label.as_deref(), Some("aqua"));

        let empty = AudioSource::File(FileAudioSource::new(
            TrackId::new("t2"),
            PathBuf::from("/music/empty.flac"),
            library_core::TrackMetadata::default(),
        ));
        let empty = track_summary(&empty).expect("file track");
        assert_eq!(empty.rating, None);
        assert_eq!(empty.comment, None);
        assert_eq!(empty.year, None);
        assert_eq!(empty.track_number, None);
        assert_eq!(empty.disc_number, None);
        assert_eq!(empty.color_label, None);
    }
}
