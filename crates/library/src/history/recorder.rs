//! Performance history recorder — qualifying-play gates, sessions, XSPF persistence.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use library_core::{Result, TrackId};
use uuid::Uuid;

use crate::db::Db;
use crate::history::store::{
    self, delete_session_index, history_dir_for_db, load_document, save_document,
    upsert_session_index, HistorySessionRow,
};
use crate::history::xspf::{self, utc_now_rfc3339, HistoryDocument, HistoryEntry};
use crate::store::Store;

#[derive(Clone, Debug, PartialEq)]
pub struct HistorySettings {
    pub enabled: bool,
    pub session_idle_minutes: u32,
    pub min_play_seconds: u32,
    pub min_deck_volume: f32,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            session_idle_minutes: 5,
            min_play_seconds: 5,
            min_deck_volume: 0.05,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckPlaySnapshot {
    pub playing: bool,
    pub volume: f32,
    pub track_id: Option<String>,
    pub track_path: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub isrc: Option<String>,
    pub duration_ms: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryRestorePrompt {
    pub session_id: String,
    pub title: String,
    pub last_activity_at: String,
}

struct PendingPlay {
    started_at: String,
    started_instant: Instant,
    deck: u16,
    snapshot: DeckPlaySnapshot,
    entry_id: String,
    committed: bool,
}

impl PendingPlay {
    fn snapshot_for_commit(&self) -> Self {
        Self {
            started_at: self.started_at.clone(),
            started_instant: self.started_instant,
            deck: self.deck,
            snapshot: self.snapshot.clone(),
            entry_id: self.entry_id.clone(),
            committed: true,
        }
    }
}

struct DeckRuntime {
    snapshot: DeckPlaySnapshot,
    pending: Option<PendingPlay>,
}

pub struct HistoryRecorder {
    history_dir: PathBuf,
    settings: HistorySettings,
    crossfader: f32,
    decks: Vec<DeckRuntime>,
    document: Option<HistoryDocument>,
    xspf_path: Option<PathBuf>,
    manual_close_pending: bool,
    restore_prompt: Option<HistoryRestorePrompt>,
    restore_declined: bool,
    /// Set when open-session entries or boundaries change; cleared via [`Self::take_session_updated`].
    session_updated: bool,
}

impl HistoryRecorder {
    pub fn new(db_path: &Path, settings: HistorySettings) -> Result<Self> {
        let history_dir = history_dir_for_db(db_path);
        std::fs::create_dir_all(&history_dir).map_err(history_io)?;
        Ok(Self {
            history_dir,
            settings,
            crossfader: 0.5,
            decks: Vec::new(),
            document: None,
            xspf_path: None,
            manual_close_pending: false,
            restore_prompt: None,
            restore_declined: false,
            session_updated: false,
        })
    }

    /// Consume the dirty flag set by persist / session boundary mutations.
    pub fn take_session_updated(&mut self) -> bool {
        std::mem::take(&mut self.session_updated)
    }

    /// Re-arm the dirty flag after a failed `HistorySessionUpdated` publish.
    pub(crate) fn mark_session_updated(&mut self) {
        self.session_updated = true;
    }

    pub fn bootstrap(&mut self, db: &Db) -> Result<()> {
        if let Some(row) = store::active_session(db)? {
            if self.restore_declined {
                return Ok(());
            }
            let elapsed = minutes_since(&row.last_activity_at);
            if elapsed >= self.settings.session_idle_minutes as f64 {
                self.close_session_by_id(db, &row.id)?;
            } else {
                self.restore_prompt = Some(HistoryRestorePrompt {
                    session_id: row.id,
                    title: row.title,
                    last_activity_at: row.last_activity_at,
                });
            }
        }
        Ok(())
    }

    pub fn settings(&self) -> &HistorySettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: HistorySettings) {
        self.settings = settings;
    }

    pub fn restore_prompt(&self) -> Option<&HistoryRestorePrompt> {
        self.restore_prompt.as_ref()
    }

    pub fn can_resume_session(&self, db: &Db) -> bool {
        self.document.is_none()
            && self.xspf_path.is_none()
            && store::list_sessions(db)
                .ok()
                .and_then(|rows| rows.into_iter().find(|r| r.closed))
                .is_some()
    }

    pub fn on_crossfader(&mut self, db: &Db, crossfader: f32) -> Result<()> {
        self.crossfader = crossfader.clamp(0.0, 1.0);
        self.sync_deck_states(db)
    }

    pub fn on_deck_updated(
        &mut self,
        db: &Db,
        deck_id: usize,
        snapshot: DeckPlaySnapshot,
    ) -> Result<()> {
        while self.decks.len() <= deck_id {
            self.decks.push(DeckRuntime {
                snapshot: DeckPlaySnapshot {
                    playing: false,
                    volume: 1.0,
                    track_id: None,
                    track_path: None,
                    title: None,
                    artist: None,
                    album: None,
                    bpm: None,
                    key: None,
                    isrc: None,
                    duration_ms: None,
                },
                pending: None,
            });
        }
        self.decks[deck_id].snapshot =
            merge_deck_play_snapshot(&self.decks[deck_id].snapshot, snapshot);
        let deck_snapshot = self.decks[deck_id].snapshot.clone();
        if let Some(pending) = &mut self.decks[deck_id].pending {
            pending.snapshot = merge_deck_play_snapshot(&pending.snapshot, deck_snapshot);
        }
        self.sync_deck_states(db)
    }

    pub fn tick(&mut self, db: &Db) -> Result<()> {
        if !self.settings.enabled {
            return Ok(());
        }
        self.commit_ready_pending(db)?;
        self.check_idle_timeout(db)
    }

    pub fn restore_session(&mut self, db: &Db, session_id: &str) -> Result<()> {
        let row =
            store::get_session(db, session_id)?.ok_or_else(|| history_err("session not found"))?;
        let path = PathBuf::from(&row.xspf_path);
        let mut doc = load_document(&path)?;
        doc.session.closed = false;
        self.document = Some(doc);
        self.xspf_path = Some(path);
        self.restore_prompt = None;
        self.restore_declined = false;
        self.manual_close_pending = false;
        self.persist(db)
    }

    pub fn decline_restore(&mut self, db: &Db) -> Result<()> {
        if let Some(prompt) = self.restore_prompt.take() {
            self.close_session_by_id(db, &prompt.session_id)?;
        }
        self.restore_declined = true;
        Ok(())
    }

    pub fn new_session(&mut self, db: &Db) -> Result<()> {
        let Some(doc) = self.document.as_ref() else {
            return Err(history_err("no active session"));
        };
        if doc.entries.is_empty() {
            return Err(history_err("session has no entries"));
        }
        self.close_active_session(db)?;
        self.manual_close_pending = true;
        Ok(())
    }

    pub fn resume_session(&mut self, db: &Db) -> Result<()> {
        if self.document.is_some() || self.xspf_path.is_some() {
            return Err(history_err("successor session already exists"));
        }
        let closed = store::list_sessions(db)?.into_iter().find(|r| r.closed);
        let Some(row) = closed else {
            return Err(history_err("no closed session to resume"));
        };
        self.restore_session(db, &row.id)
    }

    pub fn rename_session(&mut self, db: &Db, session_id: &str, title: &str) -> Result<()> {
        let row =
            store::get_session(db, session_id)?.ok_or_else(|| history_err("session not found"))?;
        let path = PathBuf::from(&row.xspf_path);
        let mut doc = load_document(&path)?;
        doc.session.title = title.to_string();
        save_document(&path, &doc)?;
        upsert_session_index(db, &doc, &path)?;
        self.mark_session_updated();
        if self
            .document
            .as_ref()
            .is_some_and(|d| d.session.id == session_id)
        {
            if let Some(active) = self.document.as_mut() {
                active.session.title = title.to_string();
            }
        }
        Ok(())
    }

    pub fn delete_session(&mut self, db: &Db, session_id: &str) -> Result<()> {
        if self
            .document
            .as_ref()
            .is_some_and(|d| d.session.id == session_id)
        {
            self.document = None;
            self.xspf_path = None;
            self.decks.iter_mut().for_each(|d| d.pending = None);
        }
        if let Some(row) = store::get_session(db, session_id)? {
            let _ = std::fs::remove_file(&row.xspf_path);
            delete_session_index(db, session_id)?;
            self.mark_session_updated();
        }
        Ok(())
    }

    pub fn list_sessions(&self, db: &Db) -> Result<Vec<HistorySessionRow>> {
        store::list_sessions(db)
    }

    pub fn session_entries(&self, db: &Db, session_id: &str) -> Result<Vec<HistoryEntry>> {
        let row =
            store::get_session(db, session_id)?.ok_or_else(|| history_err("session not found"))?;
        let mut entries = load_document(Path::new(&row.xspf_path))?.entries;
        for entry in &mut entries {
            enrich_history_entry(db, entry);
        }
        Ok(entries)
    }

    pub fn history_dir(&self) -> &Path {
        &self.history_dir
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.document.as_ref().map(|d| d.session.id.as_str())
    }

    fn sync_deck_states(&mut self, db: &Db) -> Result<()> {
        if !self.settings.enabled {
            return Ok(());
        }
        for deck_id in 0..self.decks.len() {
            let qualifying = self.is_qualifying(deck_id);
            let had_pending = self.decks[deck_id].pending.is_some();
            if qualifying {
                if self.decks[deck_id].pending.is_none() {
                    self.start_pending(deck_id)?;
                }
            } else if had_pending {
                self.finish_pending(db, deck_id)?;
            }
        }
        Ok(())
    }

    fn start_pending(&mut self, deck_id: usize) -> Result<()> {
        let snap = self.decks[deck_id].snapshot.clone();
        self.decks[deck_id].pending = Some(PendingPlay {
            started_at: utc_now_rfc3339(),
            started_instant: Instant::now(),
            deck: deck_id as u16,
            snapshot: snap,
            entry_id: Uuid::new_v4().to_string(),
            committed: false,
        });
        Ok(())
    }

    fn finish_pending(&mut self, db: &Db, deck_id: usize) -> Result<()> {
        let Some(pending) = self.decks[deck_id].pending.take() else {
            return Ok(());
        };
        if !pending.committed {
            return Ok(());
        }
        let ended_at = utc_now_rfc3339();
        let played_ms =
            parse_rfc3339_ms(&ended_at).saturating_sub(parse_rfc3339_ms(&pending.started_at));
        self.ensure_active_session()?;
        if let Some(doc) = self.document.as_mut() {
            if let Some(entry) = doc.entries.iter_mut().find(|e| e.id == pending.entry_id) {
                entry.ended_at = Some(ended_at);
                entry.played_duration_ms = Some(played_ms.max(0));
            }
            doc.session.last_activity_at = utc_now_rfc3339();
            self.persist(db)?;
        }
        Ok(())
    }

    fn commit_ready_pending(&mut self, db: &Db) -> Result<()> {
        let min_duration = Duration::from_secs(self.settings.min_play_seconds as u64);
        for deck_id in 0..self.decks.len() {
            let should_commit = self.decks[deck_id]
                .pending
                .as_ref()
                .is_some_and(|p| !p.committed && p.started_instant.elapsed() >= min_duration)
                && self.is_qualifying(deck_id);
            if !should_commit {
                continue;
            }
            let pending_snapshot = {
                let pending = self.decks[deck_id].pending.as_mut().expect("pending");
                pending.committed = true;
                pending.snapshot_for_commit()
            };
            self.ensure_active_session()?;
            let entry = self.build_entry(db, &pending_snapshot)?;
            if let Some(doc) = self.document.as_mut() {
                doc.entries.push(entry);
                doc.session.last_activity_at = utc_now_rfc3339();
                self.persist(db)?;
            }
        }
        Ok(())
    }

    fn build_entry(&self, db: &Db, pending: &PendingPlay) -> Result<HistoryEntry> {
        let mut snap = pending.snapshot.clone();
        enrich_snapshot_from_library(db, &mut snap);
        Ok(HistoryEntry {
            id: pending.entry_id.clone(),
            deck: pending.deck,
            track_id: snap.track_id.clone(),
            location: file_uri(snap.track_path.as_deref()),
            title: snap.title.clone(),
            artist: snap.artist.clone(),
            album: snap.album.clone(),
            duration_sec: snap.duration_ms.map(|ms| ms / 1000),
            bpm: snap.bpm,
            key: snap.key.clone(),
            isrc: snap.isrc.clone(),
            started_at: pending.started_at.clone(),
            ended_at: None,
            played_duration_ms: None,
        })
    }

    fn ensure_active_session(&mut self) -> Result<()> {
        if self.document.is_some() {
            return Ok(());
        }
        if self.manual_close_pending {
            self.manual_close_pending = false;
        }
        let now = utc_now_rfc3339();
        let mut doc = HistoryDocument::new_session(local_session_title(&now));
        doc.session.started_at = now.clone();
        doc.session.last_activity_at = now;
        let path = self.history_dir.join(xspf::session_filename(
            &doc.session.started_at,
            &doc.session.id,
        ));
        self.document = Some(doc);
        self.xspf_path = Some(path);
        Ok(())
    }

    fn persist(&mut self, db: &Db) -> Result<()> {
        let Some(doc) = self.document.clone() else {
            return Ok(());
        };
        let path = self
            .xspf_path
            .clone()
            .ok_or_else(|| history_err("missing xspf path"))?;
        save_document(&path, &doc)?;
        upsert_session_index(db, &doc, &path)?;
        self.mark_session_updated();
        Ok(())
    }

    fn check_idle_timeout(&mut self, db: &Db) -> Result<()> {
        if self.any_deck_qualifying() {
            return Ok(());
        }
        let timeout_minutes = self.settings.session_idle_minutes as f64;
        if let Some(doc) = self.document.as_ref() {
            if minutes_since(&doc.session.last_activity_at) >= timeout_minutes {
                self.close_active_session(db)?;
            }
            return Ok(());
        }
        if let Some(row) = store::active_session(db)? {
            if minutes_since(&row.last_activity_at) >= timeout_minutes {
                self.close_session_by_id(db, &row.id)?;
            }
        }
        Ok(())
    }

    fn close_active_session(&mut self, db: &Db) -> Result<()> {
        for deck_id in 0..self.decks.len() {
            self.finish_pending(db, deck_id)?;
        }
        if let Some(mut doc) = self.document.take() {
            let now = utc_now_rfc3339();
            for entry in &mut doc.entries {
                if entry.ended_at.is_none() {
                    entry.ended_at = Some(now.clone());
                    entry.played_duration_ms = Some(
                        parse_rfc3339_ms(&now).saturating_sub(parse_rfc3339_ms(&entry.started_at)),
                    );
                }
            }
            doc.session.closed = true;
            doc.session.last_activity_at = now;
            if let Some(path) = self.xspf_path.clone() {
                save_document(&path, &doc)?;
                upsert_session_index(db, &doc, &path)?;
            }
            self.mark_session_updated();
        }
        self.xspf_path = None;
        self.decks.iter_mut().for_each(|d| d.pending = None);
        Ok(())
    }

    fn close_session_by_id(&mut self, db: &Db, session_id: &str) -> Result<()> {
        if self
            .document
            .as_ref()
            .is_some_and(|d| d.session.id == session_id)
        {
            return self.close_active_session(db);
        }
        let row =
            store::get_session(db, session_id)?.ok_or_else(|| history_err("session not found"))?;
        let path = PathBuf::from(&row.xspf_path);
        let mut doc = load_document(&path)?;
        let now = utc_now_rfc3339();
        for entry in &mut doc.entries {
            if entry.ended_at.is_none() {
                entry.ended_at = Some(now.clone());
                entry.played_duration_ms = Some(
                    parse_rfc3339_ms(&now).saturating_sub(parse_rfc3339_ms(&entry.started_at)),
                );
            }
        }
        doc.session.closed = true;
        doc.session.last_activity_at = now;
        save_document(&path, &doc)?;
        upsert_session_index(db, &doc, &path)?;
        self.mark_session_updated();
        if self
            .restore_prompt
            .as_ref()
            .is_some_and(|p| p.session_id == session_id)
        {
            self.restore_prompt = None;
        }
        Ok(())
    }

    fn is_qualifying(&self, deck_id: usize) -> bool {
        let Some(deck) = self.decks.get(deck_id) else {
            return false;
        };
        let loaded = deck.snapshot.track_id.is_some() || deck.snapshot.track_path.is_some();
        deck.snapshot.playing
            && loaded
            && effective_output(deck_id, deck.snapshot.volume, self.crossfader)
                >= self.settings.min_deck_volume
    }

    fn any_deck_qualifying(&self) -> bool {
        (0..self.decks.len()).any(|id| self.is_qualifying(id))
    }
}

pub fn crossfader_gain(deck_id: usize, crossfader: f32) -> f32 {
    let t = crossfader.clamp(0.0, 1.0);
    let angle = t * std::f32::consts::FRAC_PI_2;
    match deck_id {
        0 => angle.cos(),
        1 => angle.sin(),
        _ => 1.0,
    }
}

pub fn effective_output(deck_id: usize, volume: f32, crossfader: f32) -> f32 {
    volume * crossfader_gain(deck_id, crossfader)
}

fn merge_deck_play_snapshot(prev: &DeckPlaySnapshot, next: DeckPlaySnapshot) -> DeckPlaySnapshot {
    DeckPlaySnapshot {
        playing: next.playing,
        volume: next.volume,
        track_id: next.track_id.or_else(|| prev.track_id.clone()),
        track_path: next.track_path.or_else(|| prev.track_path.clone()),
        title: next.title.or_else(|| prev.title.clone()),
        artist: next.artist.or_else(|| prev.artist.clone()),
        album: next.album.or_else(|| prev.album.clone()),
        bpm: next.bpm.or(prev.bpm),
        key: next.key.or_else(|| prev.key.clone()),
        isrc: next.isrc.or_else(|| prev.isrc.clone()),
        duration_ms: next.duration_ms.or(prev.duration_ms),
    }
}

fn enrich_history_entry(db: &Db, entry: &mut HistoryEntry) {
    let Some(track_id) = entry.track_id.as_ref().filter(|id| !id.is_empty()) else {
        return;
    };
    let Ok(Some(source)) = Store::new(db).get_track(&TrackId::new(track_id)) else {
        return;
    };
    let meta = source.metadata();
    if entry.location.is_empty() {
        entry.location = file_uri(Some(&source.source_ref()));
    }
    if entry.title.as_ref().is_none_or(|title| title.is_empty()) {
        entry.title = meta.title.clone().filter(|title| !title.is_empty());
    }
    if entry.artist.as_ref().is_none_or(|artist| artist.is_empty()) {
        entry.artist = meta.artist.clone().filter(|artist| !artist.is_empty());
    }
    if entry.album.as_ref().is_none_or(|album| album.is_empty()) {
        entry.album = meta.album.clone().filter(|album| !album.is_empty());
    }
    if entry.key.as_ref().is_none_or(|key| key.is_empty()) {
        entry.key = meta.key.clone().filter(|key| !key.is_empty());
    }
    if entry.bpm.is_none() {
        entry.bpm = meta.bpm;
    }
    if entry.isrc.as_ref().is_none_or(|isrc| isrc.is_empty()) {
        entry.isrc = meta.isrc.clone().filter(|isrc| !isrc.is_empty());
    }
}

fn enrich_snapshot_from_library(db: &Db, snap: &mut DeckPlaySnapshot) {
    let Some(track_id) = snap.track_id.as_ref().filter(|id| !id.is_empty()) else {
        return;
    };
    let Ok(Some(source)) = Store::new(db).get_track(&TrackId::new(track_id)) else {
        return;
    };
    let meta = source.metadata();
    if snap.track_path.as_ref().is_none_or(|path| path.is_empty()) {
        snap.track_path = Some(source.source_ref());
    }
    if snap.title.as_ref().is_none_or(|title| title.is_empty()) {
        snap.title = meta.title.clone().filter(|title| !title.is_empty());
    }
    if snap.artist.as_ref().is_none_or(|artist| artist.is_empty()) {
        snap.artist = meta.artist.clone().filter(|artist| !artist.is_empty());
    }
    if snap.album.as_ref().is_none_or(|album| album.is_empty()) {
        snap.album = meta.album.clone().filter(|album| !album.is_empty());
    }
    if snap.key.as_ref().is_none_or(|key| key.is_empty()) {
        snap.key = meta.key.clone().filter(|key| !key.is_empty());
    }
    if snap.bpm.is_none() {
        snap.bpm = meta.bpm;
    }
    if snap.isrc.as_ref().is_none_or(|isrc| isrc.is_empty()) {
        snap.isrc = meta.isrc.clone().filter(|isrc| !isrc.is_empty());
    }
    if snap.duration_ms.is_none() {
        snap.duration_ms = meta.duration_ms;
    }
}

fn file_uri(path: Option<&str>) -> String {
    match path {
        Some(p) if p.starts_with("file://") => p.to_string(),
        Some(p) => format!("file://{}", p),
        None => String::new(),
    }
}

fn local_session_title(started_at: &str) -> String {
    started_at
        .replace('T', " ")
        .trim_end_matches('Z')
        .to_string()
}

fn minutes_since(iso: &str) -> f64 {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    ((now_ms - parse_rfc3339_ms(iso)).max(0) as f64) / 60_000.0
}

fn parse_rfc3339_ms(iso: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

fn history_err(msg: &str) -> library_core::LibraryError {
    library_core::LibraryError::Backend {
        backend: "history",
        message: msg.into(),
    }
}

fn history_io(e: std::io::Error) -> library_core::LibraryError {
    library_core::LibraryError::Backend {
        backend: "history",
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open;
    use crate::history::store::{
        active_session, get_session, history_dir_for_db, upsert_session_index,
    };
    use std::fs;

    /// Instant commits via `tick` (host settings reject 0; library allows it for tests).
    fn fast_settings() -> HistorySettings {
        HistorySettings {
            enabled: true,
            session_idle_minutes: 60,
            min_play_seconds: 0,
            min_deck_volume: 0.05,
        }
    }

    fn snap(path: &str, playing: bool, volume: f32) -> DeckPlaySnapshot {
        DeckPlaySnapshot {
            playing,
            volume,
            track_id: Some("track-1".into()),
            track_path: Some(path.into()),
            title: Some("Title".into()),
            artist: Some("Artist".into()),
            album: Some("Album".into()),
            bpm: Some(128.0),
            key: Some("Am".into()),
            isrc: Some("USXXX1234567".into()),
            duration_ms: Some(180_000),
        }
    }

    fn stopped(path: &str) -> DeckPlaySnapshot {
        snap(path, false, 1.0)
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        db_path: PathBuf,
        db: Db,
        recorder: HistoryRecorder,
    }

    fn fixture(settings: HistorySettings) -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("library.db");
        let db = open(&db_path).expect("db");
        let recorder = HistoryRecorder::new(&db_path, settings).expect("recorder");
        Fixture {
            _dir: dir,
            db_path,
            db,
            recorder,
        }
    }

    fn commit_play(fx: &mut Fixture, deck: usize, path: &str) {
        fx.recorder
            .on_deck_updated(&fx.db, deck, snap(path, true, 1.0))
            .expect("deck");
        fx.recorder.tick(&fx.db).expect("tick");
    }

    fn end_play(fx: &mut Fixture, deck: usize, path: &str) {
        fx.recorder
            .on_deck_updated(&fx.db, deck, stopped(path))
            .expect("stop");
    }

    #[test]
    fn crossfader_gains_match_mixer() {
        assert!((crossfader_gain(0, 0.0) - 1.0).abs() < f32::EPSILON);
        assert!((crossfader_gain(1, 1.0) - 1.0).abs() < f32::EPSILON);
        assert!(effective_output(0, 0.1, 0.0) >= 0.05);
        assert!(effective_output(1, 0.1, 1.0) >= 0.05);
        assert!(effective_output(0, 0.1, 1.0) < 0.05);
    }

    #[test]
    fn merge_deck_play_snapshot_keeps_track_metadata() {
        let prev = DeckPlaySnapshot {
            playing: true,
            volume: 0.8,
            track_id: Some("track-1".into()),
            track_path: Some("/music/a.flac".into()),
            title: Some("Title".into()),
            artist: Some("Artist".into()),
            album: None,
            bpm: Some(128.0),
            key: None,
            isrc: None,
            duration_ms: Some(180_000),
        };
        let next = DeckPlaySnapshot {
            playing: false,
            volume: 0.5,
            track_id: None,
            track_path: None,
            title: None,
            artist: None,
            album: None,
            bpm: None,
            key: None,
            isrc: None,
            duration_ms: None,
        };
        let merged = merge_deck_play_snapshot(&prev, next);
        assert_eq!(merged.track_path.as_deref(), Some("/music/a.flac"));
        assert_eq!(merged.title.as_deref(), Some("Title"));
        assert!(!merged.playing);
        assert!((merged.volume - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn idle_timeout_closes_unloaded_session_from_last_activity_at() {
        let mut fx = fixture(HistorySettings {
            session_idle_minutes: 1,
            ..Default::default()
        });
        let mut doc = HistoryDocument::new_session("test");
        doc.session.last_activity_at = "2020-01-01T00:00:00Z".into();
        let path = fx.recorder.history_dir().join("test.xspf");
        save_document(&path, &doc).expect("write xspf");
        upsert_session_index(&fx.db, &doc, &path).expect("index");

        fx.recorder.tick(&fx.db).expect("tick");

        let row = get_session(&fx.db, &doc.session.id)
            .expect("get")
            .expect("row");
        assert!(row.closed);
    }

    #[test]
    fn session_updated_flag_take_and_restore() {
        let mut fx = fixture(HistorySettings::default());
        assert!(!fx.recorder.take_session_updated());
        fx.recorder.mark_session_updated();
        assert!(fx.recorder.take_session_updated());
        assert!(!fx.recorder.take_session_updated());
        fx.recorder.mark_session_updated();
        assert!(fx.recorder.take_session_updated());
    }

    /// AC1 + AC11 + AC13: qualifying play commits with metadata/ISRC under history/.
    #[test]
    fn qualifying_play_commits_entry_with_metadata_under_history_dir() {
        let mut fx = fixture(fast_settings());
        commit_play(&mut fx, 0, "/music/a.flac");

        let session_id = fx.recorder.active_session_id().expect("active").to_string();
        let entries = fx
            .recorder
            .session_entries(&fx.db, &session_id)
            .expect("entries");
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.deck, 0);
        assert!(!e.started_at.is_empty());
        assert_eq!(e.title.as_deref(), Some("Title"));
        assert_eq!(e.artist.as_deref(), Some("Artist"));
        assert_eq!(e.isrc.as_deref(), Some("USXXX1234567"));
        assert!(e.ended_at.is_none());

        let row = active_session(&fx.db).expect("active").expect("row");
        let xspf = PathBuf::from(&row.xspf_path);
        assert!(
            xspf.starts_with(fx.recorder.history_dir()),
            "xspf not under history dir: {xspf:?}"
        );
        assert!(fs::metadata(&xspf).is_ok(), "live xspf missing at {xspf:?}");
        assert_eq!(
            fx.recorder.history_dir(),
            history_dir_for_db(&fx.db_path).as_path()
        );
    }

    /// AC2: leaving qualifying state fills ended_at / played_duration_ms.
    #[test]
    fn ending_play_sets_ended_at_and_played_duration() {
        let mut fx = fixture(fast_settings());
        commit_play(&mut fx, 0, "/music/a.flac");
        std::thread::sleep(Duration::from_millis(5));
        end_play(&mut fx, 0, "/music/a.flac");

        let session_id = fx.recorder.active_session_id().expect("active").to_string();
        let entries = fx
            .recorder
            .session_entries(&fx.db, &session_id)
            .expect("entries");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].ended_at.is_some());
        assert!(entries[0].played_duration_ms.unwrap_or(0) >= 0);
    }

    /// AC3: below duration or effective output → no entry.
    #[test]
    fn below_duration_or_volume_produces_no_entry() {
        let mut fx = fixture(HistorySettings {
            min_play_seconds: 60,
            ..fast_settings()
        });
        fx.recorder
            .on_deck_updated(&fx.db, 0, snap("/music/a.flac", true, 1.0))
            .expect("deck");
        end_play(&mut fx, 0, "/music/a.flac");
        assert!(fx.recorder.active_session_id().is_none());

        let mut fx = fixture(fast_settings());
        fx.recorder
            .on_deck_updated(&fx.db, 0, snap("/music/a.flac", true, 0.01))
            .expect("quiet");
        fx.recorder.tick(&fx.db).expect("tick");
        assert!(fx.recorder.active_session_id().is_none());
    }

    /// AC4: re-playing the same track appends a second entry.
    #[test]
    fn replaying_same_track_appends_second_entry() {
        let mut fx = fixture(fast_settings());
        commit_play(&mut fx, 0, "/music/a.flac");
        end_play(&mut fx, 0, "/music/a.flac");
        commit_play(&mut fx, 0, "/music/a.flac");
        end_play(&mut fx, 0, "/music/a.flac");

        let session_id = fx.recorder.active_session_id().expect("active").to_string();
        let entries = fx
            .recorder
            .session_entries(&fx.db, &session_id)
            .expect("entries");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].track_id.as_deref(), Some("track-1"));
        assert_eq!(entries[1].track_id.as_deref(), Some("track-1"));
        assert_ne!(entries[0].id, entries[1].id);
    }

    /// AC5: recorder only logs deck snapshots (no sampler/PFL inputs on the type).
    #[test]
    fn only_deck_snapshots_drive_logging_no_sampler_or_pfl_api() {
        let mut fx = fixture(fast_settings());
        // No HistoryRecorder sampler / headphone API — empty decks never invent entries.
        fx.recorder.tick(&fx.db).expect("tick");
        assert!(fx.recorder.active_session_id().is_none());
        // Quiet but "playing" still fails effective-output gate (PFL cannot raise it).
        fx.recorder
            .on_deck_updated(&fx.db, 0, snap("/music/a.flac", true, 0.0))
            .expect("deck");
        fx.recorder.tick(&fx.db).expect("tick");
        assert!(fx.recorder.active_session_id().is_none());
    }

    /// AC6: after idle close, next qualifying play opens a new session.
    #[test]
    fn after_idle_timeout_next_play_opens_new_session() {
        let mut fx = fixture(HistorySettings {
            session_idle_minutes: 1,
            ..fast_settings()
        });
        let mut doc = HistoryDocument::new_session("stale");
        doc.session.last_activity_at = "2020-01-01T00:00:00Z".into();
        let path = fx.recorder.history_dir().join("stale.xspf");
        save_document(&path, &doc).expect("write");
        upsert_session_index(&fx.db, &doc, &path).expect("index");
        let old_id = doc.session.id.clone();

        fx.recorder.tick(&fx.db).expect("idle close");
        assert!(
            get_session(&fx.db, &old_id)
                .expect("get")
                .expect("row")
                .closed
        );

        commit_play(&mut fx, 0, "/music/b.flac");
        let new_id = fx
            .recorder
            .active_session_id()
            .expect("new session")
            .to_string();
        assert_ne!(old_id, new_id);
    }

    /// AC7: manual new session + resume only before a successor exists.
    #[test]
    fn manual_new_session_and_resume_before_successor() {
        let mut fx = fixture(fast_settings());
        commit_play(&mut fx, 0, "/music/a.flac");
        end_play(&mut fx, 0, "/music/a.flac");
        let first = fx.recorder.active_session_id().expect("active").to_string();

        fx.recorder.new_session(&fx.db).expect("new");
        assert!(fx.recorder.active_session_id().is_none());
        assert!(fx.recorder.can_resume_session(&fx.db));
        fx.recorder.resume_session(&fx.db).expect("resume");
        assert_eq!(fx.recorder.active_session_id(), Some(first.as_str()));

        fx.recorder.new_session(&fx.db).expect("new again");
        assert!(fx.recorder.can_resume_session(&fx.db));
        commit_play(&mut fx, 0, "/music/b.flac");
        assert!(!fx.recorder.can_resume_session(&fx.db));
        assert!(fx.recorder.resume_session(&fx.db).is_err());
    }

    /// AC8: crossfader fully cut prevents logging while deck is playing.
    #[test]
    fn crossfader_full_cut_prevents_logging() {
        let mut fx = fixture(fast_settings());
        fx.recorder.on_crossfader(&fx.db, 1.0).expect("xf");
        fx.recorder
            .on_deck_updated(&fx.db, 0, snap("/music/a.flac", true, 1.0))
            .expect("deck");
        fx.recorder.tick(&fx.db).expect("tick");
        assert!(fx.recorder.active_session_id().is_none());
        assert!(effective_output(0, 1.0, 1.0) < 0.05);
    }

    /// AC9: bootstrap restore prompt; decline starts fresh on next play.
    #[test]
    fn restore_prompt_on_bootstrap_decline_starts_fresh() {
        let mut fx = fixture(fast_settings());
        commit_play(&mut fx, 0, "/music/a.flac");
        end_play(&mut fx, 0, "/music/a.flac");
        let old_id = fx.recorder.active_session_id().expect("active").to_string();

        let mut restarted = HistoryRecorder::new(&fx.db_path, fast_settings()).expect("restart");
        restarted.bootstrap(&fx.db).expect("bootstrap");
        let prompt = restarted.restore_prompt().expect("prompt");
        assert_eq!(prompt.session_id, old_id);

        restarted.decline_restore(&fx.db).expect("decline");
        assert!(restarted.restore_prompt().is_none());
        assert!(
            get_session(&fx.db, &old_id)
                .expect("get")
                .expect("row")
                .closed
        );

        restarted
            .on_deck_updated(&fx.db, 0, snap("/music/b.flac", true, 1.0))
            .expect("deck");
        restarted.tick(&fx.db).expect("tick");
        let new_id = restarted.active_session_id().expect("fresh").to_string();
        assert_ne!(old_id, new_id);
    }

    /// AC10: settings changes affect recorder gates.
    #[test]
    fn settings_enabled_and_min_volume_affect_logging() {
        let mut fx = fixture(HistorySettings {
            enabled: false,
            ..fast_settings()
        });
        commit_play(&mut fx, 0, "/music/a.flac");
        assert!(fx.recorder.active_session_id().is_none());

        fx.recorder.set_settings(HistorySettings {
            enabled: true,
            min_deck_volume: 0.9,
            ..fast_settings()
        });
        fx.recorder
            .on_deck_updated(&fx.db, 0, snap("/music/a.flac", true, 0.5))
            .expect("deck");
        fx.recorder.tick(&fx.db).expect("tick");
        assert!(fx.recorder.active_session_id().is_none());

        fx.recorder.set_settings(fast_settings());
        commit_play(&mut fx, 0, "/music/a.flac");
        assert!(fx.recorder.active_session_id().is_some());
    }
}
