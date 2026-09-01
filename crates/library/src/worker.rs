//! Library worker thread: drains cmd bus and publishes evt.

use crate::bus::LibraryBus;
use crate::{HotCueRecord, LibraryError, LibraryManager, LoopRecord};
use library_api::{
    decode_cmd_body, encode_evt_body, BeatGrid, CmdBody, EvtBody, HotCue, Kind, Origin, SavedLoop,
    TrackSummary,
};
use library_core::{AnalysisDurationMode, AnalyzeTrackOptions, AudioSource, Library, TrackId};
use omnibus::Event;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const RECV_TIMEOUT: Duration = Duration::from_millis(100);

/// Host-owned library cmd worker (JoinHandle must not live inside `Mutex<LibraryManager>`).
pub struct LibraryWorker {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for LibraryWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn the cmd-draining worker using buses attached to `library`.
///
/// Errors if [`LibraryManager::set_buses`] was not called, or if the worker
/// fails to subscribe to the cmd bus.
pub fn spawn_library_worker(library: Arc<Mutex<LibraryManager>>) -> crate::Result<LibraryWorker> {
    let buses = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.buses().ok_or_else(|| LibraryError::Backend {
            backend: "library",
            message: "library buses not attached; call set_buses before spawn_library_worker"
                .into(),
        })?
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    let cmd = buses.cmd_bus();
    let evt = buses.evt_bus();
    let duration_handle = buses.analysis_duration_arc();
    let revision_handle = buses.revision_arc();
    let library_handle = Arc::clone(&library);
    let shutdown_flag = Arc::clone(&shutdown);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<std::result::Result<(), String>>();
    let handle = thread::spawn(move || {
        worker_thread_loop(
            cmd,
            evt,
            library_handle,
            duration_handle,
            revision_handle,
            shutdown_flag,
            ready_tx,
        );
    });
    ready_rx
        .recv()
        .map_err(|_| LibraryError::Io(std::io::Error::other("library worker failed to start")))?
        .map_err(|e| LibraryError::Io(std::io::Error::other(e)))?;

    Ok(LibraryWorker {
        shutdown,
        handle: Some(handle),
    })
}

pub(crate) fn worker_thread_loop(
    cmd_bus: LibraryBus,
    evt_bus: LibraryBus,
    library: Arc<Mutex<LibraryManager>>,
    analysis_duration: Arc<Mutex<AnalysisDurationMode>>,
    revision: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    ready: std::sync::mpsc::Sender<Result<(), String>>,
) {
    let rx = match cmd_bus.subscribe(omnibus::Filter::Any, omnibus::Filter::Any) {
        Ok(rx) => rx,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let _ = ready.send(Ok(()));

    while !shutdown.load(Ordering::Relaxed) {
        match rx.recv_timeout(RECV_TIMEOUT) {
            Ok(Some(event)) => {
                handle_cmd(&event, &library, &analysis_duration, &evt_bus, &revision)
            }
            Ok(None) => continue,
            Err(_) => break,
        }
    }
}

fn handle_cmd(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    analysis_duration: &Arc<Mutex<AnalysisDurationMode>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    match event.kind() {
        Kind::AnalyzeTrack => handle_analyze(event, library, analysis_duration, evt_bus, revision),
        Kind::RefreshTrack => handle_refresh_track(event, library, evt_bus, revision),
        Kind::SaveHotCue => handle_save_hot_cue(event, library, evt_bus, revision),
        Kind::DeleteHotCue => handle_delete_hot_cue(event, library, evt_bus, revision),
        Kind::SaveLoop => handle_save_loop(event, library, evt_bus, revision),
        Kind::DeleteLoop => handle_delete_loop(event, library, evt_bus, revision),
        Kind::SaveBeatGrid => handle_save_beat_grid(event, library, evt_bus, revision),
        Kind::TrackAnalyzed
        | Kind::TrackUpdated
        | Kind::HotCuesChanged
        | Kind::LoopsChanged
        | Kind::BeatGridChanged
        | Kind::HistorySessionUpdated
        | Kind::Navigate
        | Kind::Load
        | Kind::Error
        | Kind::Notice => {
            // Ignore evt kinds published onto cmd by mistake.
        }
    }
}

fn handle_refresh_track(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    let track_id = match decode_cmd_body(event.payload()) {
        Ok(CmdBody::RefreshTrack { track_id }) => track_id,
        Ok(_) => {
            publish_error(
                evt_bus,
                revision,
                Origin::Library,
                "refresh_track body mismatch".into(),
                None,
            );
            return;
        }
        Err(err) => {
            publish_error(evt_bus, revision, Origin::Library, err.to_string(), None);
            return;
        }
    };

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
            let _ = publish_evt(
                evt_bus,
                revision,
                Origin::Track(track_id.clone()),
                Kind::TrackUpdated,
                EvtBody::TrackUpdated { track },
            );
            match cues {
                Ok(rows) => {
                    let _ = publish_evt(
                        evt_bus,
                        revision,
                        Origin::Track(track_id.clone()),
                        Kind::HotCuesChanged,
                        EvtBody::HotCuesChanged {
                            track_id: track_id.clone(),
                            hot_cues: rows.into_iter().map(hot_cue_from_record).collect(),
                        },
                    );
                }
                Err(err) => publish_error(
                    evt_bus,
                    revision,
                    Origin::Track(track_id.clone()),
                    err.to_string(),
                    Some(track_id.clone()),
                ),
            }
            match loops {
                Ok(rows) => {
                    let _ = publish_evt(
                        evt_bus,
                        revision,
                        Origin::Track(track_id.clone()),
                        Kind::LoopsChanged,
                        EvtBody::LoopsChanged {
                            track_id: track_id.clone(),
                            loops: rows.into_iter().map(saved_loop_from_record).collect(),
                        },
                    );
                }
                Err(err) => publish_error(
                    evt_bus,
                    revision,
                    Origin::Track(track_id.clone()),
                    err.to_string(),
                    Some(track_id),
                ),
            }
        }
        Err(message) => publish_error(
            evt_bus,
            revision,
            Origin::Track(track_id.clone()),
            message,
            Some(track_id),
        ),
    }
}

fn handle_analyze(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    analysis_duration: &Arc<Mutex<AnalysisDurationMode>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    let body = match decode_cmd_body(event.payload()) {
        Ok(CmdBody::AnalyzeTrack { track_id, force }) => (track_id, force),
        Ok(_) => {
            publish_error(
                evt_bus,
                revision,
                Origin::Library,
                "analyze_track body mismatch".into(),
                None,
            );
            return;
        }
        Err(err) => {
            publish_error(evt_bus, revision, Origin::Library, err.to_string(), None);
            return;
        }
    };
    let (track_id, force) = body;
    let duration = *analysis_duration.lock().unwrap_or_else(|e| e.into_inner());
    let options = AnalyzeTrackOptions {
        force,
        analysis_duration: duration,
    };
    let result =
        LibraryManager::analyze_track_off_mutex(library, &TrackId::new(track_id.clone()), options);

    match result {
        Ok(source) => match track_summary(&source) {
            Some(track) => {
                let _ = publish_evt(
                    evt_bus,
                    revision,
                    Origin::Track(track.id.clone()),
                    Kind::TrackAnalyzed,
                    EvtBody::TrackAnalyzed { track },
                );
            }
            None => publish_error(
                evt_bus,
                revision,
                Origin::Track(track_id.clone()),
                "Only file tracks can be analyzed.".into(),
                Some(track_id),
            ),
        },
        Err(err) => publish_error(
            evt_bus,
            revision,
            Origin::Track(track_id.clone()),
            err.to_string(),
            Some(track_id),
        ),
    }
}

fn handle_save_hot_cue(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    let (track_id, slot, position_ms, loop_length_beats, color, label) =
        match decode_cmd_body(event.payload()) {
            Ok(CmdBody::SaveHotCue {
                track_id,
                slot,
                position_ms,
                loop_length_beats,
                color,
                label,
            }) => (track_id, slot, position_ms, loop_length_beats, color, label),
            Ok(_) => {
                publish_error(
                    evt_bus,
                    revision,
                    Origin::Library,
                    "save_hot_cue body mismatch".into(),
                    None,
                );
                return;
            }
            Err(err) => {
                publish_error(evt_bus, revision, Origin::Library, err.to_string(), None);
                return;
            }
        };

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.save_track_hot_cue(&id, slot, position_ms, loop_length_beats, color, label)
            .and_then(|_| lib.list_track_hot_cues(&id))
    };
    publish_hot_cues_result(evt_bus, revision, track_id, result);
}

fn handle_delete_hot_cue(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    let (track_id, slot) = match decode_cmd_body(event.payload()) {
        Ok(CmdBody::DeleteHotCue { track_id, slot }) => (track_id, slot),
        Ok(_) => {
            publish_error(
                evt_bus,
                revision,
                Origin::Library,
                "delete_hot_cue body mismatch".into(),
                None,
            );
            return;
        }
        Err(err) => {
            publish_error(evt_bus, revision, Origin::Library, err.to_string(), None);
            return;
        }
    };

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.delete_track_hot_cue(&id, slot)
            .and_then(|_| lib.list_track_hot_cues(&id))
    };
    publish_hot_cues_result(evt_bus, revision, track_id, result);
}

fn handle_save_loop(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    let (track_id, slot, in_ms, out_ms, label, color) = match decode_cmd_body(event.payload()) {
        Ok(CmdBody::SaveLoop {
            track_id,
            slot,
            in_ms,
            out_ms,
            label,
            color,
        }) => (track_id, slot, in_ms, out_ms, label, color),
        Ok(_) => {
            publish_error(
                evt_bus,
                revision,
                Origin::Library,
                "save_loop body mismatch".into(),
                None,
            );
            return;
        }
        Err(err) => {
            publish_error(evt_bus, revision, Origin::Library, err.to_string(), None);
            return;
        }
    };

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.save_track_loop(&id, slot, in_ms, out_ms, label, color)
            .and_then(|_| lib.list_track_loops(&id))
    };
    publish_loops_result(evt_bus, revision, track_id, result);
}

fn handle_delete_loop(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    let (track_id, slot) = match decode_cmd_body(event.payload()) {
        Ok(CmdBody::DeleteLoop { track_id, slot }) => (track_id, slot),
        Ok(_) => {
            publish_error(
                evt_bus,
                revision,
                Origin::Library,
                "delete_loop body mismatch".into(),
                None,
            );
            return;
        }
        Err(err) => {
            publish_error(evt_bus, revision, Origin::Library, err.to_string(), None);
            return;
        }
    };

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.delete_track_loop(&id, slot)
            .and_then(|_| lib.list_track_loops(&id))
    };
    publish_loops_result(evt_bus, revision, track_id, result);
}

fn handle_save_beat_grid(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    library: &Arc<Mutex<LibraryManager>>,
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
) {
    let (track_id, bpm, first_beat_secs) = match decode_cmd_body(event.payload()) {
        Ok(CmdBody::SaveBeatGrid {
            track_id,
            bpm,
            first_beat_secs,
        }) => (track_id, bpm, first_beat_secs),
        Ok(_) => {
            publish_error(
                evt_bus,
                revision,
                Origin::Library,
                "save_beat_grid body mismatch".into(),
                None,
            );
            return;
        }
        Err(err) => {
            publish_error(evt_bus, revision, Origin::Library, err.to_string(), None);
            return;
        }
    };

    let id = TrackId::new(track_id.clone());
    let result = {
        let lib = library.lock().unwrap_or_else(|e| e.into_inner());
        lib.save_track_beat_grid(&id, bpm, first_beat_secs)
            .and_then(|grid| lib.get_track(&id).map(|source| (grid, source)))
    };
    publish_beat_grid_result(evt_bus, revision, track_id, result);
}

fn publish_beat_grid_result(
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
    track_id: String,
    result: crate::Result<(crate::waveform::BeatGridSnapshot, Option<AudioSource>)>,
) {
    match result {
        Ok((grid, source)) => {
            let bpm = grid.bpm.unwrap_or(0.0);
            let _ = publish_evt(
                evt_bus,
                revision,
                Origin::Track(track_id.clone()),
                Kind::BeatGridChanged,
                EvtBody::BeatGridChanged {
                    track_id: track_id.clone(),
                    beat_grid: BeatGrid {
                        beats: grid.beats,
                        downbeats: grid.downbeats,
                        bpm,
                    },
                },
            );
            if let Some(source) = source {
                if let Some(track) = track_summary(&source) {
                    let _ = publish_evt(
                        evt_bus,
                        revision,
                        Origin::Track(track_id),
                        Kind::TrackUpdated,
                        EvtBody::TrackUpdated { track },
                    );
                }
            }
        }
        Err(err) => publish_error(
            evt_bus,
            revision,
            Origin::Track(track_id.clone()),
            err.to_string(),
            Some(track_id),
        ),
    }
}

fn publish_hot_cues_result(
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
    track_id: String,
    result: crate::Result<Vec<HotCueRecord>>,
) {
    match result {
        Ok(rows) => {
            let _ = publish_evt(
                evt_bus,
                revision,
                Origin::Track(track_id.clone()),
                Kind::HotCuesChanged,
                EvtBody::HotCuesChanged {
                    track_id,
                    hot_cues: rows.into_iter().map(hot_cue_from_record).collect(),
                },
            );
        }
        Err(err) => publish_error(
            evt_bus,
            revision,
            Origin::Track(track_id.clone()),
            err.to_string(),
            Some(track_id),
        ),
    }
}

fn publish_loops_result(
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
    track_id: String,
    result: crate::Result<Vec<LoopRecord>>,
) {
    match result {
        Ok(rows) => {
            let _ = publish_evt(
                evt_bus,
                revision,
                Origin::Track(track_id.clone()),
                Kind::LoopsChanged,
                EvtBody::LoopsChanged {
                    track_id,
                    loops: rows.into_iter().map(saved_loop_from_record).collect(),
                },
            );
        }
        Err(err) => publish_error(
            evt_bus,
            revision,
            Origin::Track(track_id.clone()),
            err.to_string(),
            Some(track_id),
        ),
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

fn publish_error(
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
    origin: Origin,
    message: String,
    track_id: Option<String>,
) {
    let _ = publish_evt(
        evt_bus,
        revision,
        origin,
        Kind::Error,
        EvtBody::Error { message, track_id },
    );
}

fn publish_evt(
    evt_bus: &LibraryBus,
    revision: &Arc<AtomicU64>,
    origin: Origin,
    kind: Kind,
    body: EvtBody,
) -> Result<(), String> {
    let bytes = encode_evt_body(&body).map_err(|e| e.to_string())?;
    revision.fetch_add(1, Ordering::Relaxed);
    evt_bus
        .publish(Event::new(origin, kind, Arc::from(bytes)))
        .map_err(|e| e.to_string())?;
    Ok(())
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
    })
}
