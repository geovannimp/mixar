//! XSPF read/write for performance history sessions.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use library_core::{LibraryError, Result};
use uuid::Uuid;
use xml::reader::XmlEvent;
use xspf::{Extension, Playlist, Track};

pub const MIXAR_NS: &str = "https://mixar.app/ns/history/1";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub last_activity_at: String,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    pub id: String,
    pub deck: u16,
    pub track_id: Option<String>,
    pub location: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_sec: Option<i32>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub isrc: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub played_duration_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryDocument {
    pub session: SessionMeta,
    pub entries: Vec<HistoryEntry>,
}

impl HistoryDocument {
    pub fn new_session(title: impl Into<String>) -> Self {
        let now = utc_now_rfc3339();
        Self {
            session: SessionMeta {
                id: Uuid::new_v4().to_string(),
                title: title.into(),
                started_at: now.clone(),
                last_activity_at: now,
                closed: false,
            },
            entries: Vec::new(),
        }
    }
}

pub fn utc_now_rfc3339() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn session_filename_from_started_at(started_at: &str) -> String {
    let compact = started_at
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();
    format!("{compact}.xspf")
}

pub fn write_document(path: &Path, doc: &HistoryDocument) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let tmp = path.with_extension("xspf.tmp");
    let playlist = doc_to_playlist(doc)?;
    fs::write(&tmp, playlist.to_string_pretty("  ")).map_err(io_err)?;
    fs::rename(&tmp, path).map_err(io_err)?;
    Ok(())
}

pub fn read_document(path: &Path) -> Result<HistoryDocument> {
    let playlist = Playlist::read_file(path).map_err(|e| parse_err(format!("{e:?}")))?;
    playlist_to_doc(playlist)
}

fn doc_to_playlist(doc: &HistoryDocument) -> Result<Playlist> {
    let mut playlist = Playlist::default();
    playlist.set_title(&doc.session.title);
    playlist.set_creator("Mixar");
    playlist.set_info("Mixar performance session");
    if let Some(date) = doc.session.started_at.get(..10) {
        playlist.set_date(date);
    }

    let closed = if doc.session.closed { "true" } else { "false" };
    let session_xml = format!(
        r#"<session id="{}" started_at="{}" last_activity_at="{}" closed="{closed}"/>"#,
        xml_attr(&doc.session.id),
        xml_attr(&doc.session.started_at),
        xml_attr(&doc.session.last_activity_at),
    );
    playlist.extension.push(mixar_extension(&session_xml)?);

    for entry in &doc.entries {
        let mut track = Track::default();
        if !entry.location.is_empty() {
            track.add_location(&entry.location);
        }
        if let Some(title) = &entry.title {
            track.set_title(title);
        }
        if let Some(artist) = &entry.artist {
            track.set_creator(artist);
        }
        if let Some(album) = &entry.album {
            track.set_album(album);
        }
        if let Some(duration) = entry.duration_sec.filter(|d| *d > 0) {
            track.set_duration(duration as u64 * 1000);
        }

        let ended = entry.ended_at.as_deref().map(xml_attr).unwrap_or_default();
        let played_ms = entry
            .played_duration_ms
            .map(|v| v.to_string())
            .unwrap_or_default();
        let track_id = entry.track_id.as_deref().map(xml_attr).unwrap_or_default();
        let bpm = entry.bpm.map(|v| v.to_string()).unwrap_or_default();
        let key = entry.key.as_deref().map(xml_attr).unwrap_or_default();
        let isrc = entry.isrc.as_deref().map(xml_attr).unwrap_or_default();
        let entry_xml = format!(
            r#"<entry id="{}" deck="{}" track_id="{track_id}" started_at="{}" ended_at="{ended}" played_duration_ms="{played_ms}" bpm="{bpm}" key="{key}" isrc="{isrc}"/>"#,
            xml_attr(&entry.id),
            entry.deck,
            xml_attr(&entry.started_at),
        );
        track.extension.push(mixar_extension(&entry_xml)?);
        playlist.add_track(&track);
    }

    Ok(playlist)
}

fn playlist_to_doc(playlist: Playlist) -> Result<HistoryDocument> {
    let mut doc = HistoryDocument::new_session("Session");
    doc.entries.clear();
    doc.session = SessionMeta::default();

    doc.session.title = playlist.title.unwrap_or_else(|| "Session".into());
    if let Some(ext) = mixar_extension_for(&playlist.extension) {
        apply_session_attrs(ext, &mut doc.session);
    }

    for track in playlist.track_list {
        let mut entry = empty_history_entry();
        if let Some(location) = track.location.into_iter().next() {
            entry.location = location;
        }
        entry.title = track.title;
        entry.artist = track.creator;
        entry.album = track.album;
        if let Some(duration_ms) = track.duration {
            entry.duration_sec = Some((duration_ms / 1000) as i32);
        }
        if let Some(ext) = mixar_extension_for(&track.extension) {
            apply_entry_attrs(ext, &mut entry);
        }
        doc.entries.push(entry);
    }

    if doc.session.id.is_empty() {
        doc.session.id = Uuid::new_v4().to_string();
    }
    if doc.session.title.is_empty() {
        doc.session.title = "Session".into();
    }
    Ok(doc)
}

fn mixar_extension(inner: &str) -> Result<Extension> {
    let wrapped = format!(
        r#"<?xml version="1.0"?><playlist xmlns="http://xspf.org/ns/0/"><extension application="{MIXAR_NS}">{inner}</extension></playlist>"#
    );
    let playlist: Playlist = wrapped
        .parse()
        .map_err(|e: xspf::parse::ParseError| parse_err(format!("{e:?}")))?;
    playlist
        .extension
        .into_iter()
        .next()
        .ok_or_else(|| parse_err("missing mixar extension"))
}

fn mixar_extension_for(extensions: &[Extension]) -> Option<&Extension> {
    extensions.iter().find(|ext| ext.application == MIXAR_NS)
}

fn apply_session_attrs(ext: &Extension, session: &mut SessionMeta) {
    let Some(attrs) = element_attrs(&ext.content, "session") else {
        return;
    };
    if let Some(id) = attrs.get("id").filter(|v| !v.is_empty()) {
        session.id.clone_from(id);
    }
    if let Some(started_at) = attrs.get("started_at").filter(|v| !v.is_empty()) {
        session.started_at.clone_from(started_at);
    }
    if let Some(last_activity_at) = attrs.get("last_activity_at").filter(|v| !v.is_empty()) {
        session.last_activity_at.clone_from(last_activity_at);
    }
    session.closed = attrs.get("closed").is_some_and(|v| v == "true");
}

fn apply_entry_attrs(ext: &Extension, entry: &mut HistoryEntry) {
    let Some(attrs) = element_attrs(&ext.content, "entry") else {
        return;
    };
    if let Some(id) = attrs.get("id").filter(|v| !v.is_empty()) {
        entry.id.clone_from(id);
    }
    if let Some(deck) = attrs.get("deck").and_then(|v| v.parse().ok()) {
        entry.deck = deck;
    }
    entry.track_id = non_empty_attr(&attrs, "track_id");
    if let Some(started_at) = attrs.get("started_at").filter(|v| !v.is_empty()) {
        entry.started_at.clone_from(started_at);
    }
    entry.ended_at = non_empty_attr(&attrs, "ended_at");
    entry.played_duration_ms = attrs.get("played_duration_ms").and_then(|v| v.parse().ok());
    entry.bpm = attrs.get("bpm").and_then(|v| v.parse().ok());
    entry.key = non_empty_attr(&attrs, "key");
    entry.isrc = non_empty_attr(&attrs, "isrc");
}

fn element_attrs(content: &[XmlEvent], local_name: &str) -> Option<HashMap<String, String>> {
    for ev in content {
        if let XmlEvent::StartElement {
            name, attributes, ..
        } = ev
        {
            if name.local_name == local_name {
                return Some(attrs_map(attributes));
            }
        }
    }
    None
}

fn attrs_map(attributes: &[xml::attribute::OwnedAttribute]) -> HashMap<String, String> {
    attributes
        .iter()
        .map(|attr| (attr.name.local_name.clone(), attr.value.clone()))
        .collect()
}

fn non_empty_attr(attrs: &HashMap<String, String>, key: &str) -> Option<String> {
    attrs.get(key).filter(|v| !v.is_empty()).cloned()
}

fn empty_history_entry() -> HistoryEntry {
    HistoryEntry {
        id: Uuid::new_v4().to_string(),
        deck: 0,
        track_id: None,
        location: String::new(),
        title: None,
        artist: None,
        album: None,
        duration_sec: None,
        bpm: None,
        key: None,
        isrc: None,
        started_at: utc_now_rfc3339(),
        ended_at: None,
        played_duration_ms: None,
    }
}

fn xml_attr(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn io_err(e: std::io::Error) -> LibraryError {
    LibraryError::Backend {
        backend: "history",
        message: e.to_string(),
    }
}

fn parse_err(msg: impl ToString) -> LibraryError {
    LibraryError::Backend {
        backend: "history",
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_document() {
        let mut doc = HistoryDocument::new_session("Test Set");
        doc.entries.push(HistoryEntry {
            id: "e1".into(),
            deck: 1,
            track_id: Some("t1".into()),
            location: "file:///music/a.flac".into(),
            title: Some("Rock & Roll".into()),
            artist: Some("Artist <live>".into()),
            album: None,
            duration_sec: Some(240),
            bpm: Some(128.0),
            key: Some("Am".into()),
            isrc: Some("USXXX".into()),
            started_at: "2026-08-27T14:31:05Z".into(),
            ended_at: Some("2026-08-27T14:36:12Z".into()),
            played_duration_ms: Some(307_000),
        });
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.xspf");
        write_document(&path, &doc).unwrap();
        let read = read_document(&path).unwrap();
        assert_eq!(read.session.title, "Test Set");
        assert_eq!(read.entries.len(), 1);
        assert_eq!(read.entries[0].deck, 1);
        assert_eq!(read.entries[0].title.as_deref(), Some("Rock & Roll"));
        assert_eq!(read.entries[0].artist.as_deref(), Some("Artist <live>"));
        assert_eq!(
            read.entries[0].ended_at.as_deref(),
            Some("2026-08-27T14:36:12Z")
        );
    }
}
