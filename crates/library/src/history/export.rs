//! Derived export formats for history sessions.

use std::fs;
use std::path::Path;

use library_core::{LibraryError, Result};

use super::xspf::{HistoryDocument, HistoryEntry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryExportFormat {
    Csv,
    M3u8,
    Txt,
}

pub fn export_document(
    doc: &HistoryDocument,
    format: HistoryExportFormat,
    dest: &Path,
) -> Result<()> {
    let body = match format {
        HistoryExportFormat::Csv => render_csv(doc),
        HistoryExportFormat::M3u8 => render_m3u8(doc),
        HistoryExportFormat::Txt => render_txt(doc),
    };
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    fs::write(dest, body).map_err(io_err)
}

fn render_csv(doc: &HistoryDocument) -> String {
    let mut out =
        String::from("position,started_at,ended_at,played_duration_ms,deck,title,artist,album,bpm,key,isrc,file_path,track_id\n");
    for (i, entry) in doc.entries.iter().enumerate() {
        out.push_str(&csv_row(i + 1, entry));
        out.push('\n');
    }
    out
}

fn csv_row(position: usize, entry: &HistoryEntry) -> String {
    [
        position.to_string(),
        csv_field(&entry.started_at),
        csv_field(entry.ended_at.as_deref().unwrap_or("")),
        entry
            .played_duration_ms
            .map(|v| v.to_string())
            .unwrap_or_default(),
        entry.deck.to_string(),
        csv_field(entry.title.as_deref().unwrap_or("")),
        csv_field(entry.artist.as_deref().unwrap_or("")),
        csv_field(entry.album.as_deref().unwrap_or("")),
        entry.bpm.map(|v| v.to_string()).unwrap_or_default(),
        csv_field(entry.key.as_deref().unwrap_or("")),
        csv_field(entry.isrc.as_deref().unwrap_or("")),
        csv_field(&entry.location),
        csv_field(entry.track_id.as_deref().unwrap_or("")),
    ]
    .join(",")
}

fn csv_field(raw: &str) -> String {
    let raw = if matches!(raw.as_bytes().first(), Some(b'=' | b'+' | b'-' | b'@')) {
        format!("'{raw}")
    } else {
        raw.to_owned()
    };
    if raw.contains(',') || raw.contains('"') || raw.contains('\n') {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw
    }
}

fn render_m3u8(doc: &HistoryDocument) -> String {
    let mut out = String::from("#EXTM3U\n");
    for entry in &doc.entries {
        let duration = entry.duration_sec.unwrap_or(-1);
        let title = display_title(entry);
        out.push_str(&format!("#EXTINF:{duration},{title}\n"));
        out.push_str(&entry.location);
        out.push('\n');
    }
    out
}

fn render_txt(doc: &HistoryDocument) -> String {
    let lines: Vec<String> = doc
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| format!("{}. {}", i + 1, display_title(entry)))
        .collect();
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    }
}

fn display_title(entry: &HistoryEntry) -> String {
    match (&entry.title, &entry.artist) {
        (Some(t), Some(a)) => format!("{t} - {a}"),
        (Some(t), None) => t.clone(),
        (None, Some(a)) => a.clone(),
        (None, None) => entry.location.clone(),
    }
}

fn io_err(e: std::io::Error) -> LibraryError {
    LibraryError::Backend {
        backend: "history",
        message: e.to_string(),
    }
}
