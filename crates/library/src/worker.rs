//! Library worker thread: drains cmd bus and publishes evt.

use crate::bus::LibraryBus;
use crate::LibraryManager;
use library_api::{decode_cmd_body, encode_evt_body, CmdBody, EvtBody, Kind, Origin, TrackSummary};
use library_core::{
    AnalysisDurationMode, AnalyzeTrackOptions, AudioSource, TrackId, WritableLibrary,
};
use omnibus::Event;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RECV_TIMEOUT: Duration = Duration::from_millis(100);

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
        Kind::AnalyzeTrack => {
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
            let result = library
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .analyze_track(&TrackId::new(track_id.clone()), options);

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
        Kind::TrackAnalyzed | Kind::Error | Kind::Notice => {
            // Ignore evt kinds published onto cmd by mistake.
        }
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
    })
}
