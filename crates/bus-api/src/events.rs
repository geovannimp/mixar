//! Every engine event, as one closed [`Event`] enum plus one payload struct per row.
//!
//! The table is the single source of truth: the `Topic` enum, the payload structs,
//! `EventSpec` (policy, key, fold) and `BusMessage` for `Event` are all generated
//! from it. Adding an event means adding a row — engine-core matches the enum and
//! Flutter receives it as a generated sealed class.
//!
//! Command rows carry the `deck` (or mixer scope) the old `Origin` used to carry.
//! Policy assignment follows the rules in `docs/event-bus-design.md`: relative
//! amounts fold (`Delta`), booleans flip (`Toggle`), absolute targets replace
//! (`Update`), press/release edges never merge (`Signal`).

use crate::library::BeatGrid;
use crate::library::HotCue;
use crate::library::SavedLoop;
use crate::library::TrackSummary;
use crate::payload::DeckEq;
use crate::payload::DeckHotCue;
use crate::payload::DeckSavedLoop;
use crate::payload::DeckSnapshot;
use crate::payload::EngineStatus;
use crate::payload::EqBand;
use crate::payload::FxKind;
use crate::payload::FxSlotState;
use crate::payload::JogMode;
use crate::payload::LoopRegion;
use crate::payload::PadMode;
use crate::payload::SamplerSlotInfo;
use crate::payload::SyncMode;
use audio_core::CrossfaderAssign;
use event_bus::bus_events;
use event_bus::Entity;
use event_bus::Part;

bus_events! {
    // ── deck transport ───────────────────────────────────────────────────────
    Play: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    Pause: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    TogglePlay: Toggle, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    Seek: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, position_ms: i32 };
    Unload: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };

    // ── deck levels and mixer strip ──────────────────────────────────────────
    SetVolume: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, volume: f32, soft_takeover: bool };
    SetEq: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, low: f32, mid: f32, high: f32 };
    SetEqBand: Update, Entity::Deck(*deck), Part::Index(*band as u8),
        { deck: u16, band: EqBand, gain: f32, soft_takeover: bool };
    SetFilter: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, filter: f32, soft_takeover: bool };
    SetGainTrim: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, gain_trim: f32, soft_takeover: bool };
    SetHeadphoneCue: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, enabled: bool };
    ToggleHeadphoneCue: Toggle, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    SetCrossfaderAssign: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, assignment: CrossfaderAssign };

    // ── deck tempo / pitch ───────────────────────────────────────────────────
    SetSpeed: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, speed: f32, soft_takeover: bool };
    SetTempoRange: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, tempo_range: f32 };
    SetKeyLock: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, enabled: bool };
    SetKeyShift: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, semitones: f32 };
    SetKeyboardRoot: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    SetKeyboardPage: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, page: u8 };
    SetKeyShiftPage: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, page: u8 };
    ToggleSync: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, beat_sync: bool };
    SetMasterDeck: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };

    // ── cue / loop ───────────────────────────────────────────────────────────
    SetCuePoint: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    BeginCueHold: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    EndCueHold: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    SetQuantize: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, enabled: bool };
    ToggleQuantize: Toggle, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    SetSlip: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, enabled: bool };
    ToggleSlip: Toggle, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    SetAutoLoop: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, beats: f32 };
    LoopIn: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, position_ms: i32 };
    LoopOut: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, position_ms: i32 };
    ExitLoop: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    RecallSavedLoop: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, in_ms: i32, out_ms: i32 };
    BeginLoopRoll: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, beats: f32 };
    EndLoopRoll: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };
    /// Relative beat jump: repeats within one batch add up.
    BeatJump: Delta, Entity::Deck(*deck), Part::Whole,
        { deck: u16, beats: f32 }, fold: beats;

    // ── pads ─────────────────────────────────────────────────────────────────
    SetPadMode: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, mode: PadMode };
    PadPress: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8, shift: bool };
    PadRelease: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    HotCuePadPress: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8, shift: bool };
    HotCuePadRelease: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    LoopRollPadPress: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    LoopRollPadRelease: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    BeatJumpPadPress: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    BeatJumpPadRelease: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    SamplerPadPress: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8, shift: bool };
    SamplerPadRelease: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    KeyboardPadPress: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8, shift: bool };
    KeyboardPadRelease: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    KeyShiftPadPress: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8, shift: bool };
    KeyShiftPadRelease: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, slot: u8 };
    /// A retrigger is a new hit; two different cues in one batch must both fire.
    TriggerHotCue: Signal, Entity::Deck(*deck), Part::Whole,
        { deck: u16, position_ms: i32 };

    // ── hot cues and loops on the deck ───────────────────────────────────────
    SaveDeckHotCue: Update, Entity::Deck(*deck), Part::Index(*slot),
        { deck: u16, slot: u8 };
    DeleteDeckHotCue: Update, Entity::Deck(*deck), Part::Index(*slot),
        { deck: u16, slot: u8 };

    // ── sampler slots ────────────────────────────────────────────────────────
    AssignSampler: Update, Entity::Mixer, Part::Index(*slot),
        { deck: u16, slot: u8, path: String };
    AssignSamplerTrack: Update, Entity::Mixer, Part::Index(*slot),
        { deck: u16, slot: u8, track_id: String };
    ClearSampler: Update, Entity::Mixer, Part::Index(*slot),
        { deck: u16, slot: u8 };

    // ── jog ──────────────────────────────────────────────────────────────────
    JogTouch: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, touching: bool };
    /// Relative platter ticks: repeats within one batch add up.
    JogTurn: Delta, Entity::Deck(*deck), Part::Whole,
        { deck: u16, delta: i32 }, fold: delta;
    SetJogMode: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, top: JogMode, outer: JogMode };

    // ── mixer ────────────────────────────────────────────────────────────────
    SetCrossfader: Update, Entity::Mixer, Part::Whole,
        { position: f32, soft_takeover: bool };
    SetMasterVolume: Update, Entity::Mixer, Part::Whole,
        { volume: f32, soft_takeover: bool };
    SetCueVolume: Update, Entity::Mixer, Part::Whole,
        { volume: f32, soft_takeover: bool };
    SetCueMix: Update, Entity::Mixer, Part::Whole,
        { mix: f32, soft_takeover: bool };
    SetMasterCue: Update, Entity::Mixer, Part::Whole,
        { enabled: bool };
    ToggleMasterCue: Toggle, Entity::Mixer, Part::Whole,
        { };

    // ── shared FX unit ───────────────────────────────────────────────────────
    SetFxUnitSlot: Update, Entity::Mixer, Part::Index(*slot),
        { slot: u8, kind: FxKind };
    SetFxUnitSlotEnabled: Update, Entity::Mixer, Part::Index(*slot),
        { slot: u8, enabled: bool };
    SetFxUnitWet: Update, Entity::Mixer, Part::Index(*slot),
        { slot: u8, wet: f32, soft_takeover: bool };
    SetFxUnitParam: Update, Entity::Mixer, Part::Pair(*slot, *param),
        { slot: u8, param: u8, value: f32 };
    SetDeckFxRouted: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, routed: bool };

    // ── engine egress ────────────────────────────────────────────────────────
    DeckUpdated: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, track: Option<String>, track_id: Option<String>,
          title: Option<String>, artist: Option<String>, album: Option<String>,
          isrc: Option<String>, bpm: Option<f64>, key: Option<String>,
          playing: bool, crossfader_assign: CrossfaderAssign,
          volume: f32, speed: f32, tempo_range: f32, key_lock: bool,
          key_shift: f32, keyboard_page: u8, key_shift_page: u8,
          keyboard_root_hot_cue: u8, eq: DeckEq, filter: f32, gain_trim: f32,
          fx_routed: bool, fx: Vec<FxSlotState>,
          headphone_cue: bool, sync_mode: SyncMode, cue_point_ms: Option<i32>,
          quantize: bool, active_loop: Option<LoopRegion>,
          pending_loop_in_ms: Option<i32>, slip_enabled: bool,
          slip_shadow_position_ms: Option<i32>, pad_mode: PadMode,
          stems_ready: bool, stems_live: bool, stem_mute: Vec<bool>,
          stem_isolate: Option<u8>, position_ms: Option<i32>,
          duration_ms: Option<i32>, hot_cues: Vec<DeckHotCue>,
          saved_loops: Vec<DeckSavedLoop>, loudness_lufs: Option<f64>,
          auto_gain_db: f32, active_sampler_bank_id: Option<String>,
          top_jog_mode: JogMode, outer_jog_mode: JogMode,
          jog_touching: bool, sampler_slots: Vec<SamplerSlotInfo> };
    /// Playhead tick for one deck.
    DeckPosition: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16, position_ms: i32, slip_shadow_position_ms: Option<i32> };
    DeckLevels: Update,
        match deck { Some(id) => Entity::Deck(*id), None => Entity::Mixer }, Part::Whole,
        { deck: Option<u16>, peak_l: f32, peak_r: f32,
          peak_hold_l: f32, peak_hold_r: f32 };
    /// Whole-engine snapshot: mixer state plus every deck.
    EngineSnapshot: Update, Entity::Mixer, Part::Whole,
        { status: EngineStatus };
    /// Two errors are two problems to report; never folded.
    Error: Signal, Entity::Ui, Part::Whole,
        { message: String, track_id: Option<String>, deck: Option<u16> };
    Notice: Signal, Entity::Ui, Part::Whole,
        { message: String };

    // ── library commands ─────────────────────────────────────────────────────
    AnalyzeTrack: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String, force: bool };
    GenerateStems: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String };
    RefreshTrack: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String };
    SaveBeatGrid: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String, bpm: f64, first_beat_secs: f32 };
    SaveTrackHotCue: Update, Entity::Track(track_id.clone()), Part::Index(*slot),
        { track_id: String, slot: u8, position_ms: i32,
          loop_length_beats: Option<i32>, color: Option<String>,
          label: Option<String> };
    DeleteTrackHotCue: Update, Entity::Track(track_id.clone()), Part::Index(*slot),
        { track_id: String, slot: u8 };
    SaveTrackLoop: Update, Entity::Track(track_id.clone()), Part::Index(*slot),
        { track_id: String, slot: u8, in_ms: i32, out_ms: i32,
          label: Option<String>, color: Option<String> };
    DeleteTrackLoop: Update, Entity::Track(track_id.clone()), Part::Index(*slot),
        { track_id: String, slot: u8 };

    // ── library events ───────────────────────────────────────────────────────
    TrackAnalyzed: Update, Entity::Track(track.id.clone()), Part::Whole,
        { track: TrackSummary };
    TrackUpdated: Update, Entity::Track(track.id.clone()), Part::Whole,
        { track: TrackSummary };
    /// Coarse progress for analysis / stem jobs (`phase` is a stable snake_case token).
    TrackProgress: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String, phase: String, fraction: Option<f32> };
    HotCuesChanged: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String, hot_cues: Vec<HotCue> };
    LoopsChanged: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String, loops: Vec<SavedLoop> };
    BeatGridChanged: Update, Entity::Track(track_id.clone()), Part::Whole,
        { track_id: String, beat_grid: BeatGrid };
    /// Open history session entries or boundaries changed (thin signal).
    HistorySessionUpdated: Update,
        match *session_id { Some(ref id) => Entity::Session(id.clone()), None => Entity::Library },
        Part::Whole,
        { session_id: Option<String> };
    /// UI-only: move library table focus by `delta` (signed row steps).
    Navigate: Delta, Entity::Ui, Part::Whole,
        { delta: i32 }, fold: delta;
    /// UI-only: step browser focus to another pane (`+1` next, `-1` previous).
    Focus: Delta, Entity::Ui, Part::Whole,
        { delta: i32 }, fold: delta;
    /// UI-only: load the focused library row onto `deck` (0-based).
    Load: Update, Entity::Deck(*deck), Part::Whole,
        { deck: u16 };

    // ── controller events ────────────────────────────────────────────────────
    /// A mapped controller is plugged in but not trusted yet: offer it once.
    MappingOffer: Create, Entity::ControllerPort(port_name.clone()), Part::Whole,
        { mapping_id: String, device_id: String, device_name: String,
          port_name: String };
    MappingAttached: Update, Entity::ControllerPort(port_name.clone()), Part::Whole,
        { mapping_id: String, port_name: String };
    MappingDetached: Update, Entity::ControllerPort(port_name.clone()), Part::Whole,
        { mapping_id: String, port_name: String };
}

impl From<DeckSnapshot> for DeckUpdated {
    fn from(snap: DeckSnapshot) -> Self {
        Self {
            deck: snap.id,
            track: snap.track,
            track_id: snap.track_id,
            title: snap.title,
            artist: snap.artist,
            album: snap.album,
            isrc: snap.isrc,
            bpm: snap.bpm,
            key: snap.key,
            playing: snap.playing,
            crossfader_assign: snap.crossfader_assign,
            volume: snap.volume,
            speed: snap.speed,
            tempo_range: snap.tempo_range,
            key_lock: snap.key_lock,
            key_shift: snap.key_shift,
            keyboard_page: snap.keyboard_page,
            key_shift_page: snap.key_shift_page,
            keyboard_root_hot_cue: snap.keyboard_root_hot_cue,
            eq: snap.eq,
            filter: snap.filter,
            gain_trim: snap.gain_trim,
            fx_routed: snap.fx_routed,
            fx: snap.fx,
            headphone_cue: snap.headphone_cue,
            sync_mode: snap.sync_mode,
            cue_point_ms: snap.cue_point_ms,
            quantize: snap.quantize,
            active_loop: snap.active_loop,
            pending_loop_in_ms: snap.pending_loop_in_ms,
            slip_enabled: snap.slip_enabled,
            slip_shadow_position_ms: snap.slip_shadow_position_ms,
            pad_mode: snap.pad_mode,
            stems_ready: snap.stems_ready,
            stems_live: snap.stems_live,
            stem_mute: snap.stem_mute,
            stem_isolate: snap.stem_isolate,
            position_ms: snap.position_ms,
            duration_ms: snap.duration_ms,
            hot_cues: snap.hot_cues,
            saved_loops: snap.saved_loops,
            loudness_lufs: snap.loudness_lufs,
            auto_gain_db: snap.auto_gain_db,
            active_sampler_bank_id: snap.active_sampler_bank_id,
            top_jog_mode: snap.top_jog_mode,
            outer_jog_mode: snap.outer_jog_mode,
            jog_touching: snap.jog_touching,
            // Filled by the host from its ephemeral sampler pad chrome.
            sampler_slots: Vec::new(),
        }
    }
}
