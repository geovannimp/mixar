//! Smart-playlist evaluation over the library pool.

use super::*;

impl LibraryManager {
    /// Host-reported deck references; the last set stays in effect until replaced.
    ///
    /// Deck-relative rules read the values the host computed (tempo fader and
    /// Key Shift already applied), so this crate needs no engine dependency.
    pub fn set_deck_references(&mut self, decks: Vec<DeckReference>) {
        self.deck_references = decks;
    }

    /// Evaluate `rules` over the file pool using the current deck references.
    pub fn smart_playlist_tracks(&self, rules: &SmartPlaylistRules) -> Result<Vec<AudioSource>> {
        rules.validate().map_err(smart_rules_error)?;
        let now = unix_secs_string().parse::<i64>().unwrap_or(0);
        let mut facts = self.store().list_smart_playlist_facts()?;
        facts.retain(|fact| rules.matches(fact, now, &self.deck_references));
        rules.sort_facts(&mut facts);

        let ids: Vec<TrackId> = facts
            .into_iter()
            .map(|facts| TrackId::new(facts.id))
            .collect();
        let by_id = self.store().get_tracks_by_ids(&ids)?;
        Ok(ids
            .iter()
            .filter_map(|id| by_id.get(id.as_str()).cloned())
            .collect())
    }

    /// Stored rule document for a smart playlist (`Err` for other kinds).
    pub fn smart_playlist_rules_json(&self, id: &CollectionId) -> Result<String> {
        let collection = self.require_collection(id)?;
        match &collection.config {
            CollectionConfig::SmartPlaylist { rules_json } => Ok(rules_json.clone()),
            _ => Err(LibraryError::WrongCollectionType {
                expected: CollectionType::SmartPlaylist.as_str(),
                got: collection.collection_type().as_str(),
            }),
        }
    }
}

/// Validation failures surface as a backend error so callers get one error type.
pub(crate) fn smart_rules_error(message: String) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: format!("smart playlist rules: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use library_core::{
        SmartCondition, SmartField, SmartMatch, SmartOp, SmartRule, SmartRuleGroup, SmartSort,
        SmartSortField,
    };

    fn condition(field: SmartField, op: SmartOp) -> SmartCondition {
        SmartCondition {
            field,
            op,
            min: None,
            max: None,
            values: Vec::new(),
            deck: None,
        }
    }

    fn rules(conditions: Vec<SmartCondition>, sort: Option<SmartSort>) -> SmartPlaylistRules {
        SmartPlaylistRules {
            sort,
            root: SmartRuleGroup {
                match_mode: SmartMatch::All,
                rules: conditions.into_iter().map(SmartRule::Condition).collect(),
            },
            ..Default::default()
        }
    }

    fn bpm_between(min: f64, max: f64) -> SmartCondition {
        SmartCondition {
            min: Some(min),
            max: Some(max),
            ..condition(SmartField::Bpm, SmartOp::Between)
        }
    }

    fn manager() -> LibraryManager {
        LibraryManager::open_in_memory(LibraryConfig::default()).expect("open in-memory library")
    }

    fn import(
        library: &mut LibraryManager,
        name: &str,
        bpm: Option<f64>,
        key: Option<&str>,
    ) -> TrackId {
        let path = PathBuf::from(format!("/music/{name}.wav"));
        let metadata = TrackMetadata {
            title: Some(name.to_string()),
            bpm,
            key: key.map(str::to_string),
            ..TrackMetadata::default()
        };
        library
            .import_track(&path, &metadata)
            .expect("import track")
            .source
            .id()
            .clone()
    }

    fn create(
        library: &mut LibraryManager,
        name: &str,
        rules: &SmartPlaylistRules,
    ) -> CollectionId {
        library
            .add_collection(&NewCollection::smart_playlist(name, rules.to_json()))
            .expect("create smart playlist")
            .id
    }

    fn ids(sources: &[AudioSource]) -> Vec<String> {
        sources
            .iter()
            .map(|source| source.id().as_str().to_string())
            .collect()
    }

    #[test]
    fn smart_playlist_membership_follows_metadata_edits() {
        let mut library = manager();
        let slow = import(&mut library, "slow", Some(120.0), None);
        let peak = import(&mut library, "peak", Some(128.0), None);
        let fast = import(&mut library, "fast", Some(134.0), None);

        let playlist = create(
            &mut library,
            "Peak",
            &rules(vec![bpm_between(126.0, 130.0)], None),
        );
        let tracks = library
            .list_collection_tracks(&playlist)
            .expect("list tracks");
        assert_eq!(ids(&tracks), vec![peak.as_str().to_string()]);

        // Editing a track's BPM changes membership without touching the playlist.
        library
            .store()
            .update_track_metadata(
                &slow,
                &TrackMetadataEdit {
                    title: Some("slow".into()),
                    bpm: Some(128.0),
                    ..TrackMetadataEdit::default()
                },
                &unix_secs_string(),
            )
            .expect("edit metadata");
        let tracks = library
            .list_collection_tracks(&playlist)
            .expect("list tracks");
        let mut matched = ids(&tracks);
        matched.sort();
        let mut expected = vec![slow.as_str().to_string(), peak.as_str().to_string()];
        expected.sort();
        assert_eq!(matched, expected);

        // …and the untouched track stays out.
        assert!(!matched.contains(&fast.as_str().to_string()));
    }

    #[test]
    fn smart_playlist_deck_rules_track_reference() {
        let mut library = manager();
        let matching = import(&mut library, "matching", Some(128.0), Some("Am"));
        let other = import(&mut library, "other", Some(140.0), Some("F#m"));

        let mut window = bpm_between(-6.0, 6.0);
        window.field = SmartField::DeckBpmDelta;
        let mut harmonic = condition(SmartField::DeckKey, SmartOp::HarmonicallyCompatible);
        harmonic.deck = None;
        let playlist = create(
            &mut library,
            "±6% + harmonic key",
            &rules(vec![window, harmonic], None),
        );

        // No playing deck: deck-relative rules match nothing.
        assert!(library
            .list_collection_tracks(&playlist)
            .expect("list tracks")
            .is_empty());

        library.set_deck_references(vec![DeckReference {
            deck_id: 0,
            playing: true,
            bpm: Some(128.0),
            key: Some("Am".into()),
        }]);
        assert_eq!(
            ids(&library.list_collection_tracks(&playlist).expect("list")),
            vec![matching.as_str().to_string()]
        );

        // Replacing the reference widens the window onto the other track.
        library.set_deck_references(vec![DeckReference {
            deck_id: 0,
            playing: true,
            bpm: Some(140.0),
            key: Some("F#m".into()),
        }]);
        assert_eq!(
            ids(&library.list_collection_tracks(&playlist).expect("list")),
            vec![other.as_str().to_string()]
        );
    }

    #[test]
    fn smart_playlist_delete_keeps_tracks() {
        let mut library = manager();
        let first = import(&mut library, "one", Some(128.0), None);
        let second = import(&mut library, "two", Some(120.0), None);
        let playlist = create(
            &mut library,
            "Everything",
            &rules(
                vec![condition(SmartField::Title, SmartOp::IsNotEmpty)],
                None,
            ),
        );
        assert_eq!(
            library
                .list_collection_tracks(&playlist)
                .expect("list")
                .len(),
            2
        );

        library.delete_collection(&playlist).expect("delete");
        assert!(library.get_collection(&playlist).expect("get").is_none());
        assert!(library.get_track(&first).expect("get").is_some());
        assert!(library.get_track(&second).expect("get").is_some());
    }

    #[test]
    fn smart_playlist_rejects_membership_edits() {
        let mut library = manager();
        let track = import(&mut library, "one", Some(128.0), None);
        let playlist = create(
            &mut library,
            "Rules",
            &rules(
                vec![condition(SmartField::Title, SmartOp::IsNotEmpty)],
                None,
            ),
        );

        let error = library
            .add_collection_entry(&playlist, &track, None)
            .expect_err("membership is computed");
        assert!(matches!(
            error,
            LibraryError::WrongCollectionType {
                expected: "playlist",
                got: "smart_playlist"
            }
        ));
        assert!(library.list_collection_entries(&playlist).is_err());
    }

    #[test]
    fn smart_playlist_loudness_requires_analysis() {
        use crate::entity::track_analysis;
        use sea_orm::{ActiveModelTrait, Set};

        let mut library = manager();
        let analyzed = import(&mut library, "analyzed", None, None);
        let plain = import(&mut library, "plain", None, None);

        let mut loudness = condition(SmartField::LoudnessLufs, SmartOp::Between);
        loudness.min = Some(-9.0);
        let playlist = create(&mut library, "Loud", &rules(vec![loudness], None));
        assert!(library
            .list_collection_tracks(&playlist)
            .expect("list")
            .is_empty());

        track_analysis::ActiveModel {
            track_id: Set(analyzed.as_str().to_string()),
            backend: Set("test".into()),
            backend_version: Set("0".into()),
            analyzed_at: Set(unix_secs_string()),
            sample_rate: Set(48_000),
            duration_analyzed_ms: Set(1_000),
            loudness_lufs: Set(Some(-6.0)),
            ..Default::default()
        }
        .insert(library.db.conn().expect("conn").as_connection())
        .expect("insert analysis");

        let matched = ids(&library.list_collection_tracks(&playlist).expect("list"));
        assert_eq!(matched, vec![analyzed.as_str().to_string()]);
        assert!(!matched.contains(&plain.as_str().to_string()));
    }

    #[test]
    fn smart_playlist_update_replaces_rules() {
        let mut library = manager();
        let quiet = import(&mut library, "quiet", Some(120.0), None);
        let peak = import(&mut library, "peak", Some(128.0), None);
        let playlist = create(
            &mut library,
            "Set",
            &rules(vec![bpm_between(126.0, 130.0)], None),
        );
        assert_eq!(
            ids(&library.list_collection_tracks(&playlist).expect("list")),
            vec![peak.as_str().to_string()]
        );

        let widened = rules(vec![bpm_between(100.0, 200.0)], None);
        library
            .update_collection(
                &playlist,
                &UpdateCollection {
                    name: Some("Wide".into()),
                    config: Some(CollectionConfigUpdate::SmartPlaylist {
                        rules_json: widened.to_json(),
                    }),
                },
            )
            .expect("update rules");

        let collection = library
            .get_collection(&playlist)
            .expect("get")
            .expect("exists");
        assert_eq!(collection.name, "Wide");
        assert_eq!(
            library
                .smart_playlist_rules_json(&playlist)
                .expect("stored rules"),
            widened.to_json()
        );
        let mut matched = ids(&library.list_collection_tracks(&playlist).expect("list"));
        matched.sort();
        let mut expected = vec![quiet.as_str().to_string(), peak.as_str().to_string()];
        expected.sort();
        assert_eq!(matched, expected);

        // A playable-but-invalid document is rejected, and nothing about the
        // collection changes — including a name sent alongside it.
        assert!(library
            .update_collection(
                &playlist,
                &UpdateCollection {
                    name: Some("Rejected".into()),
                    config: Some(CollectionConfigUpdate::SmartPlaylist {
                        rules_json: "{not json".into(),
                    }),
                },
            )
            .is_err());
        assert_eq!(
            library
                .smart_playlist_rules_json(&playlist)
                .expect("stored rules"),
            widened.to_json()
        );
        assert_eq!(
            library
                .get_collection(&playlist)
                .expect("get")
                .expect("exists")
                .name,
            "Wide"
        );
    }

    #[test]
    fn smart_playlist_rules_persist_across_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("library.db");
        let rules = rules(
            vec![bpm_between(126.0, 130.0)],
            Some(SmartSort {
                field: SmartSortField::Bpm,
                ascending: true,
            }),
        );
        let playlist = {
            let mut library =
                LibraryManager::open(&db_path, LibraryConfig::default()).expect("open library");
            let playlist = create(&mut library, "Peak", &rules);
            let stored = library
                .smart_playlist_rules_json(&playlist)
                .expect("stored rules");
            assert_eq!(stored, rules.to_json());
            playlist
        };

        let library = LibraryManager::open(&db_path, LibraryConfig::default()).expect("reopen");
        let reopened = library
            .smart_playlist_rules_json(&playlist)
            .expect("stored rules after reopen");
        assert_eq!(reopened, rules.to_json());
        assert_eq!(
            library
                .get_collection(&playlist)
                .expect("get")
                .expect("exists")
                .collection_type(),
            CollectionType::SmartPlaylist
        );
    }
}
