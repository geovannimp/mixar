# History MVP acceptance evidence (§14)

Tracks [history-spec §14](history-spec.md#14--acceptance-criteria) against automated and manual evidence for [#249](https://github.com/geovannimp/mixar/issues/249).

**Completion rule:** do not mark History MVP complete while any row lacks evidence, fails, or depends on unresolved work. Related: #242 (export format tests — closed).

| # | Criterion | Automated evidence | Manual | Status |
|---|-----------|--------------------|--------|--------|
| 1 | Qualifying play commits entry (`started_at`, deck, metadata) | `qualifying_play_commits_entry_with_metadata_under_history_dir` | — | Pass |
| 2 | End play → `ended_at` + `played_duration_ms` | `ending_play_sets_ended_at_and_played_duration` | — | Pass |
| 3 | Below duration or effective-output → no entry | `below_duration_or_volume_produces_no_entry` | — | Pass |
| 4 | Re-play same track → second entry | `replaying_same_track_appends_second_entry` | — | Pass |
| 5 | Sampler / PFL never logged | `only_deck_snapshots_drive_logging…` + `non_deck_events_do_not_drive_history` | — | Pass |
| 6 | Idle timeout → next play opens new session | `after_idle_timeout_next_play_opens_new_session` | — | Pass |
| 7 | Manual New session / Resume | `manual_new_session_and_resume_before_successor` | — | Pass |
| 8 | Crossfader full cut blocks logging | `crossfader_full_cut_prevents_logging` + gain helper | — | Pass |
| 9 | Restart in idle window → prompt; decline → fresh | `restore_prompt_on_bootstrap_decline_starts_fresh` | — | Pass |
| 10 | Four settings persist and affect behavior | `history_settings_round_trip_survives_reload` + `settings_enabled_and_min_volume_affect_logging` | — | Pass |
| 11 | ISRC in history entry snapshots | ISRC asserted in AC1 commit test | Track-detail UI spot-check | Pass |
| 12 | CSV / M3U8 open in external tools | #242 export render/write tests | Manual smoke below | Manual + auto format |
| 13 | OBS can read `{appSupport}/history/*.xspf` | AC1 asserts live path under `history_dir_for_db` | Manual smoke below | Auto path + manual |

Run automated suite:

```bash
cargo test --manifest-path crates/Cargo.toml -p library --lib history
cargo test --manifest-path crates/Cargo.toml -p host_flutter --lib -- history_settings_round_trip non_deck_events_do_not_drive_history
```

## Manual smoke

### AC12 — CSV / M3U8 in external tools

1. Export a history session as CSV and as M3U8 from the History UI.
2. Open CSV in LibreOffice Calc / Excel → rows match session order; header includes `isrc`.
3. Open M3U8 in VLC / foobar2000 → playlist lists exported tracks.

**Pass when:** files open without corruption and content matches the session.

### AC13 — OBS / live XSPF

1. Note app-support `history/` (beside `library.db`).
2. Play a qualifying track so an active `.xspf` appears/updates.
3. Point OBS Media Source / a text reader (or `cat`) at that `.xspf`.

**Pass when:** file is valid XSPF and updates as plays commit.
