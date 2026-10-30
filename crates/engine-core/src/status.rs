use crate::sync::DeckControlState;
use audio_core::secs_to_ms;
use bus_api::{DeckEq, DeckHotCue, DeckSnapshot, EngineStatus, LoopRegion};
use engine_dsp::{DeckState, DspEngine};

use crate::engine::{DeckTransportState, Engine};

impl Engine {
    /// Read one deck from the running DSP engine.
    ///
    /// Returns `None` when the engine is not running or the deck id is out of
    /// range, so callers do not repeat the lock-and-lookup sequence.
    fn with_dsp_deck<R>(
        &self,
        deck_id: usize,
        read: impl FnOnce(&engine_dsp::Deck) -> R,
    ) -> Option<R> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        Some(read(deck))
    }

    /// Snapshot playback positions for all decks that currently have loaded audio.
    pub fn deck_playback_snapshot(&self) -> Vec<(usize, i32, i32)> {
        let Some(dsp_engine) = self.dsp_engine.as_ref() else {
            return Vec::new();
        };
        let dsp = match dsp_engine.lock() {
            Ok(dsp) => dsp,
            Err(_) => return Vec::new(),
        };

        let mut snapshot = Vec::new();
        for deck_id in 0..dsp.num_decks() {
            let Some(deck) = dsp.deck(deck_id) else {
                continue;
            };
            let Some(duration_secs) = deck.duration_seconds() else {
                continue;
            };
            let position = deck.position_ms().unwrap_or(0);
            snapshot.push((deck_id, position, secs_to_ms(duration_secs)));
        }
        snapshot
    }

    /// Cue point and loop region for status mirroring.
    pub fn deck_transport_state(&self, deck_id: usize) -> Option<DeckTransportState> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        Some((deck.cue_point_ms(), deck.loop_region_ms()))
    }

    /// Whether a deck is currently playing.
    pub fn deck_is_playing(&self, deck_id: usize) -> Option<bool> {
        self.with_dsp_deck(deck_id, |deck| {
            matches!(deck.state(), engine_dsp::DeckState::Playing)
        })
    }

    /// Whether stem audio is attached and usable (live: first window published).
    pub fn deck_stems_ready(&self, deck_id: usize) -> Option<bool> {
        self.with_dsp_deck(deck_id, |deck| deck.stems_ready())
    }

    /// Whether jog may be moving the playhead (scratch / bend / release).
    pub fn deck_jog_driving_audio(&self, deck_id: usize) -> Option<bool> {
        self.with_dsp_deck(deck_id, |deck| deck.jog_driving_audio())
    }

    /// Playback position and duration for a deck (milliseconds), when the engine is running.
    pub fn deck_playback_ms(&self, deck_id: usize) -> Option<(i32, i32)> {
        self.with_dsp_deck(deck_id, |deck| {
            let duration = secs_to_ms(deck.duration_seconds()?);
            let position = deck.position_ms().unwrap_or(0);
            Some((position, duration))
        })
        .flatten()
    }

    /// Whether a deck has loaded audio when the engine is running.
    pub fn deck_has_audio_loaded(&self, deck_id: usize) -> Option<bool> {
        self.with_dsp_deck(deck_id, |deck| deck.has_audio_loaded())
    }

    /// Slim deck snapshot for bus `Updated` events.
    pub fn deck_snapshot(&self, deck_id: usize) -> Option<DeckSnapshot> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let control = self.deck_control.get(deck_id)?;
        deck_snapshot_from_dsp(&dsp, deck_id, control)
    }

    /// Full engine snapshot for bus `Status` events.
    pub fn engine_status_snapshot(&self) -> Option<EngineStatus> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let mut decks = Vec::with_capacity(dsp.num_decks());
        let default_control = DeckControlState::default();
        for deck_id in 0..dsp.num_decks() {
            let control = self.deck_control.get(deck_id).unwrap_or(&default_control);
            if let Some(snapshot) = deck_snapshot_from_dsp(&dsp, deck_id, control) {
                decks.push(snapshot);
            }
        }
        Some(EngineStatus {
            running: true,
            sample_rate: dsp.sample_rate(),
            crossfader: dsp.mixer().crossfader(),
            master_volume: dsp.mixer().master_volume(),
            cue_volume: dsp.mixer().cue_volume(),
            cue_mix: dsp.mixer().cue_mix(),
            master_cue: dsp.mixer().master_cue(),
            master_deck: self.master_deck as u16,
            decks,
        })
    }
}

fn deck_snapshot_from_dsp(
    dsp: &DspEngine,
    deck_id: usize,
    control: &DeckControlState,
) -> Option<DeckSnapshot> {
    let deck = dsp.deck(deck_id)?;
    let channel = dsp.mixer().channel(deck_id)?;
    let eq = channel.eq_gains();
    let (position_ms, duration_ms) = match deck.duration_seconds() {
        Some(duration) => (
            Some(deck.position_ms().unwrap_or(0)),
            Some(secs_to_ms(duration)),
        ),
        None => (None, None),
    };
    let active_loop = deck.loop_region_ms().map(|(in_ms, out_ms)| LoopRegion {
        in_ms,
        out_ms,
        active: true,
    });
    Some(DeckSnapshot {
        id: deck_id as u16,
        track: control.track_path.clone(),
        track_id: control.track_id.as_ref().map(|id| id.as_str().to_string()),
        title: control.title.clone(),
        artist: control.artist.clone(),
        album: control.album.clone(),
        isrc: control.isrc.clone(),
        bpm: control.bpm,
        key: control.key.clone(),
        playing: matches!(deck.state(), DeckState::Playing),
        crossfader_assign: dsp.mixer().crossfader_assign(deck_id)?,
        volume: channel.volume(),
        speed: deck.speed(),
        tempo_range: deck.tempo_range(),
        key_lock: deck.key_lock(),
        key_shift: control.pitch_pad.key_shift_semitones,
        keyboard_page: control.pitch_pad.keyboard_page,
        key_shift_page: control.pitch_pad.key_shift_page,
        keyboard_root_hot_cue: control.pitch_pad.keyboard_root_hot_cue,
        eq: DeckEq {
            low: crate::control_norm::strip_db_to_norm(eq.low_db),
            mid: crate::control_norm::strip_db_to_norm(eq.mid_db),
            high: crate::control_norm::strip_db_to_norm(eq.high_db),
        },
        filter: crate::control_norm::strip_db_to_norm(channel.filter_db()),
        gain_trim: crate::control_norm::strip_db_to_norm(channel.gain_trim_db()),
        fx_routed: channel.fx_routed(),
        fx: (0..bus_api::FX_SLOT_COUNT)
            .map(|slot| fx_slot_state(channel.fx(), slot))
            .collect(),
        headphone_cue: channel.headphone_cue(),
        sync_mode: control.sync_mode,
        cue_point_ms: deck.cue_point_ms(),
        quantize: control.quantize,
        active_loop,
        pending_loop_in_ms: control.pending_loop_in_ms,
        slip_enabled: deck.slip_enabled(),
        slip_shadow_position_ms: deck.shadow_position_ms(),
        pad_mode: control.pad_mode,
        stems_ready: deck.stems_ready(),
        stems_live: deck.stems_live(),
        stem_mute: deck.stem_mute().to_vec(),
        stem_isolate: deck.stem_isolate(),
        position_ms,
        duration_ms,
        hot_cues: control_hot_cues(control),
        saved_loops: Vec::new(),
        loudness_lufs: None,
        auto_gain_db: 0.0,
        active_sampler_bank_id: control.active_sampler_bank_id.clone(),
        top_jog_mode: map_jog_mode_to_api(deck.top_jog_mode()),
        outer_jog_mode: map_jog_mode_to_api(deck.outer_jog_mode()),
        jog_touching: deck.jog_touching(),
    })
}

fn control_hot_cues(control: &DeckControlState) -> Vec<DeckHotCue> {
    control
        .hot_cues
        .iter()
        .enumerate()
        .filter_map(|(slot, pos)| {
            Some(DeckHotCue {
                slot: slot as u8,
                position_ms: (*pos)?,
                loop_length_beats: None,
                color: None,
                label: None,
            })
        })
        .collect()
}

pub(crate) fn map_jog_mode(mode: bus_api::JogMode) -> engine_dsp::JogMode {
    match mode {
        bus_api::JogMode::Vinyl => engine_dsp::JogMode::Vinyl,
        bus_api::JogMode::PitchBend => engine_dsp::JogMode::PitchBend,
        bus_api::JogMode::Ignore => engine_dsp::JogMode::Ignore,
    }
}

fn fx_slot_state(fx: &engine_dsp::FxChain, slot: usize) -> bus_api::FxSlotState {
    let fx_slot = fx.slot(slot).expect("FX slot index is in range");
    bus_api::FxSlotState {
        kind: map_fx_kind_to_api(fx_slot.kind()),
        enabled: fx_slot.enabled(),
        wet: fx_slot.wet(),
        params: fx_slot.params().to_vec(),
    }
}

pub(crate) fn map_fx_kind(kind: bus_api::FxKind) -> engine_dsp::FxKind {
    match kind {
        bus_api::FxKind::Off => engine_dsp::FxKind::Off,
        bus_api::FxKind::Echo => engine_dsp::FxKind::Echo,
        bus_api::FxKind::Reverb => engine_dsp::FxKind::Reverb,
    }
}

fn map_fx_kind_to_api(kind: engine_dsp::FxKind) -> bus_api::FxKind {
    match kind {
        engine_dsp::FxKind::Off => bus_api::FxKind::Off,
        engine_dsp::FxKind::Echo => bus_api::FxKind::Echo,
        engine_dsp::FxKind::Reverb => bus_api::FxKind::Reverb,
    }
}

fn map_jog_mode_to_api(mode: engine_dsp::JogMode) -> bus_api::JogMode {
    match mode {
        engine_dsp::JogMode::Vinyl => bus_api::JogMode::Vinyl,
        engine_dsp::JogMode::PitchBend => bus_api::JogMode::PitchBend,
        engine_dsp::JogMode::Ignore => bus_api::JogMode::Ignore,
    }
}
