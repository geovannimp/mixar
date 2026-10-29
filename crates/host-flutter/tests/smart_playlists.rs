//! Smart playlists through the FRB transport: create, list, evaluate, edit.

use std::path::PathBuf;

use host_flutter::api::library::{LibraryDeckReference, LibraryTransport};
use library::{TrackMetadata, WritableLibrary};

/// Rules document for `bpm between min..=max`; the library stores this exact
/// canonical form (compact JSON, float bounds).
fn bpm_between_rules(min: f64, max: f64) -> String {
    format!(
        r#"{{"version":1,"root":{{"match":"all","rules":[{{"kind":"condition","field":"bpm","op":"between","min":{min},"max":{max}}}]}}}}"#
    )
}

fn deck_rules() -> String {
    r#"{"version":1,"root":{"match":"all","rules":[
        {"kind":"condition","field":"deck_bpm_delta","op":"between","min":-6,"max":6},
        {"kind":"condition","field":"deck_key","op":"harmonically_compatible"}]}}"#
        .to_string()
}

/// In-memory library holding one 128 BPM Am track and one 140 BPM F#m track.
fn transport() -> LibraryTransport {
    let transport = LibraryTransport::open_in_memory().expect("open transport");
    let manager = transport.library_arc();
    {
        let mut library = manager.lock().expect("library lock");
        for (name, bpm, key) in [("peak", 128.0, "Am"), ("other", 140.0, "F#m")] {
            library
                .import_track(
                    &PathBuf::from(format!("/music/{name}.wav")),
                    &TrackMetadata {
                        title: Some(name.to_string()),
                        bpm: Some(bpm),
                        key: Some(key.to_string()),
                        ..TrackMetadata::default()
                    },
                )
                .expect("import track");
        }
    }
    transport
}

#[test]
fn create_smart_playlist_lists_it_without_a_track_count() {
    let transport = transport();

    let created = transport
        .create_smart_playlist("Peak".into(), bpm_between_rules(126.0, 130.0))
        .expect("create smart playlist");
    assert_eq!(created.kind, "smart_playlist");
    assert_eq!(created.name, "Peak");
    assert_eq!(created.track_count, None);

    let collections = transport.list_collections().expect("list collections");
    let row = collections
        .iter()
        .find(|collection| collection.id == created.id)
        .expect("created row is listed");
    assert_eq!(row.kind, "smart_playlist");
    assert_eq!(row.track_count, None);

    let tracks = transport
        .list_collection_entries(created.id.clone())
        .expect("list entries");
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title.as_deref(), Some("peak"));
    assert_eq!(tracks[0].entry_id, None, "computed membership has no rows");
}

#[test]
fn smart_playlist_deck_reference_reshapes_membership() {
    let transport = transport();
    let created = transport
        .create_smart_playlist("±6% + key".into(), deck_rules())
        .expect("create smart playlist");

    // No reference yet: deck-relative conditions match nothing.
    assert!(transport
        .list_collection_entries(created.id.clone())
        .expect("entries")
        .is_empty());

    transport
        .set_deck_reference(vec![LibraryDeckReference {
            deck_id: 0,
            playing: true,
            bpm: Some(128.0),
            key: Some("Am".into()),
        }])
        .expect("set deck reference");
    let titles: Vec<String> = transport
        .list_collection_entries(created.id.clone())
        .expect("entries")
        .into_iter()
        .filter_map(|track| track.title)
        .collect();
    assert_eq!(titles, ["peak"]);

    transport
        .set_deck_reference(vec![LibraryDeckReference {
            deck_id: 0,
            playing: true,
            bpm: Some(140.0),
            key: Some("F#m".into()),
        }])
        .expect("replace deck reference");
    let titles: Vec<String> = transport
        .list_collection_entries(created.id.clone())
        .expect("entries")
        .into_iter()
        .filter_map(|track| track.title)
        .collect();
    assert_eq!(titles, ["other"]);
}

#[test]
fn update_smart_playlist_renames_and_changes_membership() {
    let transport = transport();
    let created = transport
        .create_smart_playlist("Narrow".into(), bpm_between_rules(126.0, 130.0))
        .expect("create smart playlist");
    assert_eq!(
        transport
            .list_collection_entries(created.id.clone())
            .expect("entries")
            .len(),
        1
    );

    let updated = transport
        .update_smart_playlist(
            created.id.clone(),
            "Wide".into(),
            bpm_between_rules(100.0, 200.0),
        )
        .expect("update smart playlist");
    assert_eq!(updated.name, "Wide");
    assert_eq!(updated.kind, "smart_playlist");
    assert_eq!(updated.track_count, None);
    assert_eq!(
        transport
            .list_collection_entries(created.id.clone())
            .expect("entries")
            .len(),
        2
    );

    // The stored document is the canonical form of what was sent.
    let stored = transport
        .smart_playlist_rules(created.id.clone())
        .expect("stored rules");
    assert_eq!(
        stored,
        r#"{"version":1,"root":{"match":"all","rules":[{"kind":"condition","field":"bpm","op":"between","min":100.0,"max":200.0}]}}"#
    );
}

#[test]
fn smart_playlist_invalid_rules_are_rejected() {
    let transport = transport();

    let error = transport
        .create_smart_playlist("Broken".into(), "{not json".into())
        .expect_err("invalid document");
    assert!(error.contains("smart playlist rules:"), "{error}");

    let created = transport
        .create_smart_playlist("Fine".into(), bpm_between_rules(126.0, 130.0))
        .expect("create smart playlist");
    let error = transport
        .update_smart_playlist(
            created.id,
            "Still fine".into(),
            // Structurally valid JSON, semantically invalid rule.
            r#"{"version":1,"root":{"match":"all","rules":[
                {"kind":"condition","field":"title","op":"between","min":1}]}}"#
                .into(),
        )
        .expect_err("invalid operator");
    assert!(
        error.contains("smart playlist rules: op between is not allowed on title"),
        "{error}"
    );
}

#[test]
fn smart_playlist_delete_keeps_tracks() {
    let transport = transport();
    let created = transport
        .create_smart_playlist("Peak".into(), bpm_between_rules(126.0, 130.0))
        .expect("create smart playlist");
    let tracks = transport
        .list_collection_entries(created.id.clone())
        .expect("entries");
    let track_id = tracks[0].id.clone();

    transport
        .delete_collection(created.id.clone())
        .expect("delete smart playlist");
    assert!(transport
        .list_collections()
        .expect("collections")
        .iter()
        .all(|collection| collection.id != created.id));
    assert!(
        transport.get_track(track_id).expect("get track").is_some(),
        "deleting a smart playlist must not delete its members"
    );
}
