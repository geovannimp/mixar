//! Artwork is scaled once, at the source: the UI gets one thumbnail edge, the
//! export another, and neither ever sees the user's full-resolution cover.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::io::Write;
use std::path::Path;

use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use library::{read_artwork, LibraryConfig, LibraryManager, ARTWORK_UI_EDGE_PX};
use library_core::{ExportSource, EXPORT_ARTWORK_EDGE_PX};
use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Tag, TagType};

/// 1s of 16-bit mono PCM at 8 kHz: decodable by our decoder, which `export_data`
/// needs (it computes the detail waveform off a full decode).
fn write_wav(path: &Path) {
    let sample_rate = 8_000u32;
    let pcm = vec![0u8; sample_rate as usize * 2];
    let data_size = pcm.len() as u32;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&(sample_rate * 2).to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&pcm).unwrap();
}

/// Embed `cover` as the front cover of the WAV at `path` (`id3 ` chunk → ID3v2
/// APIC) and assert lofty reads it back — a container that silently dropped the
/// picture would otherwise look like "no artwork".
fn write_wav_with_cover(path: &Path, mime: MimeType, cover: Vec<u8>) {
    write_wav(path);

    let mut tagged = Probe::open(path).unwrap().read().unwrap();
    if tagged.primary_tag().is_none() {
        tagged.insert_tag(Tag::new(TagType::Id3v2));
    }
    tagged.primary_tag_mut().expect("id3v2 tag").set_picture(
        0,
        Picture::unchecked(cover)
            .pic_type(PictureType::CoverFront)
            .mime_type(mime)
            .build(),
    );
    tagged
        .save_to_path(path, WriteOptions::default())
        .expect("save artwork tag");

    let reread = Probe::open(path).unwrap().read().unwrap();
    assert_eq!(
        reread.primary_tag().expect("id3v2 tag").pictures().len(),
        1,
        "lofty must read back the picture it wrote"
    );
}

/// 300×200 PNG: larger than both thumbnail edges, so both must resize it.
fn cover_png() -> Vec<u8> {
    let rgb = RgbImage::from_fn(300, 200, |x, y| Rgb([(x % 256) as u8, (y % 256) as u8, 64]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(rgb)
        .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("encode cover");
    bytes
}

fn jpeg_dimensions(bytes: &[u8]) -> (u32, u32) {
    let decoded = image::load_from_memory(bytes).expect("thumbnail must decode");
    (decoded.width(), decoded.height())
}

#[test]
fn embedded_cover_is_thumbnailed_for_the_ui_and_for_export() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("covered.wav");
    write_wav_with_cover(&path, MimeType::Png, cover_png());

    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&path).unwrap();

    let ui = lib
        .get_track_artwork(track.id())
        .unwrap()
        .expect("UI artwork");
    assert_eq!(ui[..2], [0xFF, 0xD8], "UI artwork is JPEG");
    let (width, height) = jpeg_dimensions(&ui);
    assert_eq!(width.max(height), ARTWORK_UI_EDGE_PX);
    // The manager and the crate-level reader agree on the UI thumbnail.
    assert_eq!(read_artwork(&path).unwrap().as_deref(), Some(&ui[..]));

    let data = lib.export_data(track.id()).unwrap().expect("export data");
    let artwork = data.artwork.expect("export artwork");
    assert_eq!(artwork.jpeg[..2], [0xFF, 0xD8], "export artwork is JPEG");
    let (width, height) = jpeg_dimensions(&artwork.jpeg);
    assert_eq!(width.max(height), EXPORT_ARTWORK_EDGE_PX);
}

#[test]
fn track_without_cover_has_no_artwork() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bare.wav");
    write_wav(&path);

    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&path).unwrap();

    assert!(lib.get_track_artwork(track.id()).unwrap().is_none());
    let data = lib.export_data(track.id()).unwrap().expect("export data");
    assert!(data.artwork.is_none());
}
