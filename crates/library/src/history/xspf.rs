//! XSPF read/write for performance history sessions.

use std::fs;
use std::path::Path;

use library_core::{LibraryError, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use uuid::Uuid;

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
    fs::write(&tmp, render_document(doc)).map_err(io_err)?;
    fs::rename(&tmp, path).map_err(io_err)?;
    Ok(())
}

pub fn read_document(path: &Path) -> Result<HistoryDocument> {
    let bytes = fs::read(path).map_err(io_err)?;
    parse_document(&bytes).map_err(parse_err)
}

fn render_document(doc: &HistoryDocument) -> String {
    let date = doc
        .session
        .started_at
        .get(..10)
        .unwrap_or(&doc.session.started_at);
    let closed = if doc.session.closed { "true" } else { "false" };
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<playlist version=\"1\" xmlns=\"http://xspf.org/ns/0/\">\n");
    out.push_str(&format!(
        "  <title>{}</title>\n",
        xml_escape(&doc.session.title)
    ));
    out.push_str("  <creator>Mixar</creator>\n");
    out.push_str(&format!("  <date>{date}</date>\n"));
    out.push_str("  <info>Mixar performance session</info>\n");
    out.push_str(&format!(
        "  <extension application=\"{MIXAR_NS}\">\n    <session id=\"{}\" started_at=\"{}\" last_activity_at=\"{}\" closed=\"{closed}\"/>\n  </extension>\n",
        xml_escape(&doc.session.id),
        xml_escape(&doc.session.started_at),
        xml_escape(&doc.session.last_activity_at),
    ));
    out.push_str("  <trackList>\n");
    for entry in &doc.entries {
        out.push_str("    <track>\n");
        out.push_str(&format!(
            "      <location>{}</location>\n",
            xml_escape(&entry.location)
        ));
        if let Some(title) = &entry.title {
            out.push_str(&format!("      <title>{}</title>\n", xml_escape(title)));
        }
        if let Some(artist) = &entry.artist {
            out.push_str(&format!(
                "      <creator>{}</creator>\n",
                xml_escape(artist)
            ));
        }
        if let Some(album) = &entry.album {
            out.push_str(&format!("      <album>{}</album>\n", xml_escape(album)));
        }
        if let Some(duration) = entry.duration_sec {
            out.push_str(&format!("      <duration>{duration}</duration>\n"));
        }
        let ended = entry
            .ended_at
            .as_deref()
            .map(xml_escape)
            .unwrap_or_default();
        let played_ms = entry
            .played_duration_ms
            .map(|v| v.to_string())
            .unwrap_or_default();
        let track_id = entry
            .track_id
            .as_deref()
            .map(xml_escape)
            .unwrap_or_default();
        let bpm = entry.bpm.map(|v| v.to_string()).unwrap_or_default();
        let key = entry.key.as_deref().map(xml_escape).unwrap_or_default();
        let isrc = entry.isrc.as_deref().map(xml_escape).unwrap_or_default();
        out.push_str(&format!("      <extension application=\"{MIXAR_NS}\">\n"));
        out.push_str(&format!(
            "        <entry id=\"{}\" deck=\"{}\" track_id=\"{track_id}\" started_at=\"{}\" ended_at=\"{ended}\" played_duration_ms=\"{played_ms}\" bpm=\"{bpm}\" key=\"{key}\" isrc=\"{isrc}\"/>\n",
            xml_escape(&entry.id),
            entry.deck,
            xml_escape(&entry.started_at),
        ));
        out.push_str("      </extension>\n");
        out.push_str("    </track>\n");
    }
    out.push_str("  </trackList>\n");
    out.push_str("</playlist>\n");
    out
}

fn parse_document(bytes: &[u8]) -> std::result::Result<HistoryDocument, String> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut doc = HistoryDocument::new_session("Session");
    doc.entries.clear();
    doc.session = SessionMeta::default();

    let mut buf = Vec::new();
    let mut in_track = false;
    let mut current_entry: Option<HistoryEntry> = None;
    let mut text_target: Option<TextTarget> = None;
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"session" => apply_session_attrs(&e, &mut doc.session),
                b"track" => {
                    in_track = true;
                    current_entry = Some(empty_history_entry());
                }
                b"entry" if in_track => {
                    if let Some(entry) = current_entry.as_mut() {
                        apply_entry_attrs(&e, entry);
                    }
                }
                b"title" if !in_track => text_target = Some(TextTarget::SessionTitle),
                b"title" if in_track => text_target = Some(TextTarget::TrackTitle),
                b"creator" if in_track => text_target = Some(TextTarget::TrackArtist),
                b"album" if in_track => text_target = Some(TextTarget::TrackAlbum),
                b"location" if in_track => text_target = Some(TextTarget::TrackLocation),
                b"duration" if in_track => text_target = Some(TextTarget::TrackDuration),
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                b"session" => apply_session_attrs(&e, &mut doc.session),
                b"entry" if in_track => {
                    if let Some(entry) = current_entry.as_mut() {
                        apply_entry_attrs(&e, entry);
                    }
                }
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if text_target.is_some() {
                    text_buf.push_str(&e.decode().unwrap_or_default());
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"title" if text_target == Some(TextTarget::SessionTitle) => {
                    doc.session.title = text_buf.clone();
                    clear_text(&mut text_target, &mut text_buf);
                }
                b"title" if text_target == Some(TextTarget::TrackTitle) => {
                    if let Some(entry) = current_entry.as_mut() {
                        entry.title = Some(text_buf.clone());
                    }
                    clear_text(&mut text_target, &mut text_buf);
                }
                b"creator" if text_target == Some(TextTarget::TrackArtist) => {
                    if let Some(entry) = current_entry.as_mut() {
                        entry.artist = Some(text_buf.clone());
                    }
                    clear_text(&mut text_target, &mut text_buf);
                }
                b"album" if text_target == Some(TextTarget::TrackAlbum) => {
                    if let Some(entry) = current_entry.as_mut() {
                        entry.album = Some(text_buf.clone());
                    }
                    clear_text(&mut text_target, &mut text_buf);
                }
                b"location" if text_target == Some(TextTarget::TrackLocation) => {
                    if let Some(entry) = current_entry.as_mut() {
                        entry.location = text_buf.clone();
                    }
                    clear_text(&mut text_target, &mut text_buf);
                }
                b"duration" if text_target == Some(TextTarget::TrackDuration) => {
                    if let Some(entry) = current_entry.as_mut() {
                        entry.duration_sec = text_buf.parse().ok();
                    }
                    clear_text(&mut text_target, &mut text_buf);
                }
                b"track" => {
                    if let Some(entry) = current_entry.take() {
                        doc.entries.push(entry);
                    }
                    in_track = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
        buf.clear();
    }

    if doc.session.id.is_empty() {
        doc.session.id = Uuid::new_v4().to_string();
    }
    if doc.session.title.is_empty() {
        doc.session.title = "Session".into();
    }
    Ok(doc)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextTarget {
    SessionTitle,
    TrackTitle,
    TrackArtist,
    TrackAlbum,
    TrackLocation,
    TrackDuration,
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

fn apply_session_attrs(e: &BytesStart, session: &mut SessionMeta) {
    session.id = attr_string(e, b"id").unwrap_or_else(|| session.id.clone());
    session.started_at =
        attr_string(e, b"started_at").unwrap_or_else(|| session.started_at.clone());
    session.last_activity_at =
        attr_string(e, b"last_activity_at").unwrap_or_else(|| session.last_activity_at.clone());
    session.closed = attr_string(e, b"closed").is_some_and(|v| v == "true");
}

fn apply_entry_attrs(e: &BytesStart, entry: &mut HistoryEntry) {
    entry.id = attr_string(e, b"id").unwrap_or_else(|| entry.id.clone());
    entry.deck = attr_string(e, b"deck")
        .and_then(|v| v.parse().ok())
        .unwrap_or(entry.deck);
    entry.track_id = non_empty_attr(e, b"track_id");
    entry.started_at = attr_string(e, b"started_at").unwrap_or_else(|| entry.started_at.clone());
    entry.ended_at = non_empty_attr(e, b"ended_at");
    entry.played_duration_ms = attr_string(e, b"played_duration_ms").and_then(|v| v.parse().ok());
    entry.bpm = attr_string(e, b"bpm").and_then(|v| v.parse().ok());
    entry.key = non_empty_attr(e, b"key");
    entry.isrc = non_empty_attr(e, b"isrc");
}

fn clear_text(target: &mut Option<TextTarget>, buf: &mut String) {
    *target = None;
    buf.clear();
}

fn attr_string(e: &BytesStart, key: &[u8]) -> Option<String> {
    e.try_get_attribute(key)
        .ok()
        .flatten()
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}

fn non_empty_attr(e: &BytesStart, key: &[u8]) -> Option<String> {
    attr_string(e, key).filter(|s| !s.is_empty())
}

fn xml_escape(raw: &str) -> String {
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

fn parse_err(msg: String) -> LibraryError {
    LibraryError::Backend {
        backend: "history",
        message: msg,
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
            title: Some("Title".into()),
            artist: Some("Artist".into()),
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
        assert_eq!(
            read.entries[0].ended_at.as_deref(),
            Some("2026-08-27T14:36:12Z")
        );
    }
}
