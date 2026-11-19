//! Schema invariants: which direction a payload travels and who owns it.
//!
//! Two independent expectations are held against the schema:
//!
//! * the `expected_target` / `expected_topic` matches below — exhaustive, with
//!   no wildcard, so a new variant forces an owner decision here as well as in
//!   both worker dispatchers;
//! * one runtime fixture per variant, built through `From`, so a mis-routed arm
//!   or a missing conversion fails the test instead of only failing to compile.
//!
//! The fixtures are ordinary values: a payload that grows a field still compiles
//! here, which is the point — this file pins direction and ownership, not shape.

use bus_api::Command;
use bus_api::CommandTarget;
use bus_api::CrossfaderAssign;
use bus_api::CurveLibrarySpec;
use bus_api::DeckEq;
use bus_api::EngineStatus;
use bus_api::EqBand;
use bus_api::Event;
use bus_api::EventTopic;
use bus_api::FxKind;
use bus_api::JogMode;
use bus_api::PadMode;
use bus_api::SyncMode;
use bus_api::TrackSummary;
use event_bus::Message;

/// The nine commands the library worker owns.
const LIBRARY_COMMANDS: [&str; 9] = [
    "AnalyzeTrack",
    "GenerateStems",
    "RefreshTrack",
    "SaveBeatGrid",
    "SaveTrackCue",
    "SaveTrackHotCue",
    "DeleteTrackHotCue",
    "SaveTrackLoop",
    "DeleteTrackLoop",
];

/// Every remaining command, owned by the engine.
const ENGINE_COMMANDS: [&str; 85] = [
    "Play",
    "Pause",
    "TogglePlay",
    "Seek",
    "Unload",
    "SetVolume",
    "SetEq",
    "SetEqBand",
    "SetStemEqBand",
    "SetStemEqEnabled",
    "ToggleStemEq",
    "SetFilter",
    "SetGainTrim",
    "SetHeadphoneCue",
    "ToggleHeadphoneCue",
    "SetCrossfaderAssign",
    "SetSpeed",
    "SetTempoRange",
    "SetTempoCurve",
    "SetKeyLock",
    "SetKeyShift",
    "SetKeyboardRoot",
    "SetKeyboardPage",
    "SetKeyShiftPage",
    "ToggleSync",
    "SetMasterDeck",
    "SetCuePoint",
    "BeginCueHold",
    "EndCueHold",
    "FaderStart",
    "FaderStop",
    "SetFaderStartNative",
    "SetQuantize",
    "ToggleQuantize",
    "SetSlip",
    "ToggleSlip",
    "SetAutoLoop",
    "LoopIn",
    "LoopOut",
    "ExitLoop",
    "RecallSavedLoop",
    "BeginLoopRoll",
    "EndLoopRoll",
    "BeatJump",
    "SetPadMode",
    "PadPress",
    "PadRelease",
    "HotCuePadPress",
    "HotCuePadRelease",
    "LoopRollPadPress",
    "LoopRollPadRelease",
    "BeatJumpPadPress",
    "BeatJumpPadRelease",
    "SamplerPadPress",
    "SamplerPadRelease",
    "KeyboardPadPress",
    "KeyboardPadRelease",
    "KeyShiftPadPress",
    "KeyShiftPadRelease",
    "TriggerHotCue",
    "SaveDeckHotCue",
    "DeleteDeckHotCue",
    "AssignSampler",
    "AssignSamplerTrack",
    "ClearSampler",
    "JogTouch",
    "JogTurn",
    "SetJogMode",
    "SetCrossfader",
    "SetMasterVolume",
    "SetCueVolume",
    "SetCueMix",
    "SetMasterCue",
    "ToggleMasterCue",
    "SetCurveLibrary",
    "SetCrossfaderCurve",
    "SetCrossfaderCurveAmount",
    "SetCrossfaderBehavior",
    "SetChannelFaderCurve",
    "SetFxUnitSlot",
    "SetFxUnitSlotEnabled",
    "SetFxUnitWet",
    "SetFxLevelCurve",
    "SetFxUnitParam",
    "SetDeckFxRouted",
];

/// Every event, in catalog order.
const EVENT_NAMES: [&str; 19] = [
    "DeckUpdated",
    "DeckPosition",
    "DeckLevels",
    "EngineSnapshot",
    "Error",
    "Notice",
    "TrackAnalyzed",
    "TrackUpdated",
    "TrackProgress",
    "HotCuesChanged",
    "LoopsChanged",
    "BeatGridChanged",
    "HistorySessionUpdated",
    "Navigate",
    "Focus",
    "Load",
    "MappingOffer",
    "MappingAttached",
    "MappingDetached",
];

/// The owner each command variant declares.
fn expected_target(command: &Command) -> CommandTarget {
    match command {
        // ── library-owned ───────────────────────────────────────────────────
        Command::AnalyzeTrack(_) => CommandTarget::Library,
        Command::GenerateStems(_) => CommandTarget::Library,
        Command::RefreshTrack(_) => CommandTarget::Library,
        Command::SaveBeatGrid(_) => CommandTarget::Library,
        Command::SaveTrackCue(_) => CommandTarget::Library,
        Command::SaveTrackHotCue(_) => CommandTarget::Library,
        Command::DeleteTrackHotCue(_) => CommandTarget::Library,
        Command::SaveTrackLoop(_) => CommandTarget::Library,
        Command::DeleteTrackLoop(_) => CommandTarget::Library,

        // ── engine-owned ────────────────────────────────────────────────────
        Command::Play(_) => CommandTarget::Engine,
        Command::Pause(_) => CommandTarget::Engine,
        Command::TogglePlay(_) => CommandTarget::Engine,
        Command::Seek(_) => CommandTarget::Engine,
        Command::Unload(_) => CommandTarget::Engine,
        Command::SetVolume(_) => CommandTarget::Engine,
        Command::SetEq(_) => CommandTarget::Engine,
        Command::SetEqBand(_) => CommandTarget::Engine,
        Command::SetStemEqBand(_) => CommandTarget::Engine,
        Command::SetStemEqEnabled(_) => CommandTarget::Engine,
        Command::ToggleStemEq(_) => CommandTarget::Engine,
        Command::SetFilter(_) => CommandTarget::Engine,
        Command::SetGainTrim(_) => CommandTarget::Engine,
        Command::SetHeadphoneCue(_) => CommandTarget::Engine,
        Command::ToggleHeadphoneCue(_) => CommandTarget::Engine,
        Command::SetCrossfaderAssign(_) => CommandTarget::Engine,
        Command::SetSpeed(_) => CommandTarget::Engine,
        Command::SetTempoRange(_) => CommandTarget::Engine,
        Command::SetTempoCurve(_) => CommandTarget::Engine,
        Command::SetKeyLock(_) => CommandTarget::Engine,
        Command::SetKeyShift(_) => CommandTarget::Engine,
        Command::SetKeyboardRoot(_) => CommandTarget::Engine,
        Command::SetKeyboardPage(_) => CommandTarget::Engine,
        Command::SetKeyShiftPage(_) => CommandTarget::Engine,
        Command::ToggleSync(_) => CommandTarget::Engine,
        Command::SetMasterDeck(_) => CommandTarget::Engine,
        Command::SetCuePoint(_) => CommandTarget::Engine,
        Command::BeginCueHold(_) => CommandTarget::Engine,
        Command::EndCueHold(_) => CommandTarget::Engine,
        Command::FaderStart(_) => CommandTarget::Engine,
        Command::FaderStop(_) => CommandTarget::Engine,
        Command::SetFaderStartNative(_) => CommandTarget::Engine,
        Command::SetQuantize(_) => CommandTarget::Engine,
        Command::ToggleQuantize(_) => CommandTarget::Engine,
        Command::SetSlip(_) => CommandTarget::Engine,
        Command::ToggleSlip(_) => CommandTarget::Engine,
        Command::SetAutoLoop(_) => CommandTarget::Engine,
        Command::LoopIn(_) => CommandTarget::Engine,
        Command::LoopOut(_) => CommandTarget::Engine,
        Command::ExitLoop(_) => CommandTarget::Engine,
        Command::RecallSavedLoop(_) => CommandTarget::Engine,
        Command::BeginLoopRoll(_) => CommandTarget::Engine,
        Command::EndLoopRoll(_) => CommandTarget::Engine,
        Command::BeatJump(_) => CommandTarget::Engine,
        Command::SetPadMode(_) => CommandTarget::Engine,
        Command::PadPress(_) => CommandTarget::Engine,
        Command::PadRelease(_) => CommandTarget::Engine,
        Command::HotCuePadPress(_) => CommandTarget::Engine,
        Command::HotCuePadRelease(_) => CommandTarget::Engine,
        Command::LoopRollPadPress(_) => CommandTarget::Engine,
        Command::LoopRollPadRelease(_) => CommandTarget::Engine,
        Command::BeatJumpPadPress(_) => CommandTarget::Engine,
        Command::BeatJumpPadRelease(_) => CommandTarget::Engine,
        Command::SamplerPadPress(_) => CommandTarget::Engine,
        Command::SamplerPadRelease(_) => CommandTarget::Engine,
        Command::KeyboardPadPress(_) => CommandTarget::Engine,
        Command::KeyboardPadRelease(_) => CommandTarget::Engine,
        Command::KeyShiftPadPress(_) => CommandTarget::Engine,
        Command::KeyShiftPadRelease(_) => CommandTarget::Engine,
        Command::TriggerHotCue(_) => CommandTarget::Engine,
        Command::SaveDeckHotCue(_) => CommandTarget::Engine,
        Command::DeleteDeckHotCue(_) => CommandTarget::Engine,
        Command::AssignSampler(_) => CommandTarget::Engine,
        Command::AssignSamplerTrack(_) => CommandTarget::Engine,
        Command::ClearSampler(_) => CommandTarget::Engine,
        Command::JogTouch(_) => CommandTarget::Engine,
        Command::JogTurn(_) => CommandTarget::Engine,
        Command::SetJogMode(_) => CommandTarget::Engine,
        Command::SetCrossfader(_) => CommandTarget::Engine,
        Command::SetMasterVolume(_) => CommandTarget::Engine,
        Command::SetCueVolume(_) => CommandTarget::Engine,
        Command::SetCueMix(_) => CommandTarget::Engine,
        Command::SetMasterCue(_) => CommandTarget::Engine,
        Command::ToggleMasterCue(_) => CommandTarget::Engine,
        Command::SetCurveLibrary(_) => CommandTarget::Engine,
        Command::SetCrossfaderCurve(_) => CommandTarget::Engine,
        Command::SetCrossfaderCurveAmount(_) => CommandTarget::Engine,
        Command::SetCrossfaderBehavior(_) => CommandTarget::Engine,
        Command::SetChannelFaderCurve(_) => CommandTarget::Engine,
        Command::SetFxUnitSlot(_) => CommandTarget::Engine,
        Command::SetFxUnitSlotEnabled(_) => CommandTarget::Engine,
        Command::SetFxUnitWet(_) => CommandTarget::Engine,
        Command::SetFxLevelCurve(_) => CommandTarget::Engine,
        Command::SetFxUnitParam(_) => CommandTarget::Engine,
        Command::SetDeckFxRouted(_) => CommandTarget::Engine,
    }
}

/// The topic each event variant declares.
fn expected_topic(event: &Event) -> EventTopic {
    match event {
        Event::DeckUpdated(_) => EventTopic::DeckUpdated,
        Event::DeckPosition(_) => EventTopic::DeckPosition,
        Event::DeckLevels(_) => EventTopic::DeckLevels,
        Event::EngineSnapshot(_) => EventTopic::EngineSnapshot,
        Event::Error(_) => EventTopic::Error,
        Event::Notice(_) => EventTopic::Notice,
        Event::TrackAnalyzed(_) => EventTopic::TrackAnalyzed,
        Event::TrackUpdated(_) => EventTopic::TrackUpdated,
        Event::TrackProgress(_) => EventTopic::TrackProgress,
        Event::HotCuesChanged(_) => EventTopic::HotCuesChanged,
        Event::LoopsChanged(_) => EventTopic::LoopsChanged,
        Event::BeatGridChanged(_) => EventTopic::BeatGridChanged,
        Event::HistorySessionUpdated(_) => EventTopic::HistorySessionUpdated,
        Event::Navigate(_) => EventTopic::Navigate,
        Event::Focus(_) => EventTopic::Focus,
        Event::Load(_) => EventTopic::Load,
        Event::MappingOffer(_) => EventTopic::MappingOffer,
        Event::MappingAttached(_) => EventTopic::MappingAttached,
        Event::MappingDetached(_) => EventTopic::MappingDetached,
    }
}

fn track_summary() -> TrackSummary {
    TrackSummary {
        id: String::new(),
        display_name: String::new(),
        artist: None,
        title: None,
        album: None,
        genre: None,
        bpm: None,
        key: None,
        duration_ms: None,
        path: String::new(),
        isrc: None,
        rating: None,
        comment: None,
        year: None,
        track_number: None,
        disc_number: None,
        color_label: None,
    }
}

fn engine_status() -> EngineStatus {
    EngineStatus {
        running: false,
        sample_rate: 48_000,
        crossfader: 0.5,
        master_volume: 0.8,
        cue_volume: 0.5,
        cue_mix: 0.5,
        master_cue: false,
        master_deck: 0,
        crossfader_curve: Default::default(),
        decks: Vec::new(),
    }
}

fn beat_grid() -> bus_api::BeatGrid {
    bus_api::BeatGrid {
        source: String::new(),
        covered_to_ms: None,
        segments: Vec::new(),
    }
}

/// One instance of every command, built through its `From` conversion.
fn all_commands() -> Vec<Command> {
    vec![
        bus_api::Play { deck: 0 }.into(),
        bus_api::Pause { deck: 0 }.into(),
        bus_api::TogglePlay { deck: 0 }.into(),
        bus_api::Seek {
            deck: 0,
            position_ms: 0,
        }
        .into(),
        bus_api::Unload { deck: 0 }.into(),
        bus_api::SetVolume {
            deck: 0,
            volume: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetEq {
            deck: 0,
            low: 0.5,
            mid: 0.5,
            high: 0.5,
        }
        .into(),
        bus_api::SetEqBand {
            deck: 0,
            band: EqBand::Mid,
            gain: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetStemEqBand {
            deck: 0,
            band: EqBand::Mid,
            gain: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetStemEqEnabled {
            deck: 0,
            enabled: false,
        }
        .into(),
        bus_api::ToggleStemEq { deck: 0 }.into(),
        bus_api::SetFilter {
            deck: 0,
            filter: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetGainTrim {
            deck: 0,
            gain_trim: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetHeadphoneCue {
            deck: 0,
            enabled: false,
        }
        .into(),
        bus_api::ToggleHeadphoneCue { deck: 0 }.into(),
        bus_api::SetCrossfaderAssign {
            deck: 0,
            assignment: CrossfaderAssign::A,
        }
        .into(),
        bus_api::SetSpeed {
            deck: 0,
            speed: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetTempoRange {
            deck: 0,
            tempo_range: 0.5,
        }
        .into(),
        bus_api::SetTempoCurve {
            deck: 0,
            id: String::new(),
        }
        .into(),
        bus_api::SetKeyLock {
            deck: 0,
            enabled: false,
        }
        .into(),
        bus_api::SetKeyShift {
            deck: 0,
            semitones: 0.5,
        }
        .into(),
        bus_api::SetKeyboardRoot { deck: 0, slot: 0 }.into(),
        bus_api::SetKeyboardPage { deck: 0, page: 0 }.into(),
        bus_api::SetKeyShiftPage { deck: 0, page: 0 }.into(),
        bus_api::ToggleSync {
            deck: 0,
            beat_sync: false,
        }
        .into(),
        bus_api::SetMasterDeck { deck: 0 }.into(),
        bus_api::SetCuePoint { deck: 0 }.into(),
        bus_api::BeginCueHold { deck: 0 }.into(),
        bus_api::EndCueHold { deck: 0 }.into(),
        bus_api::FaderStart { deck: 0 }.into(),
        bus_api::FaderStop { deck: 0 }.into(),
        bus_api::SetFaderStartNative {
            deck: 0,
            native: false,
        }
        .into(),
        bus_api::SetQuantize {
            deck: 0,
            enabled: false,
        }
        .into(),
        bus_api::ToggleQuantize { deck: 0 }.into(),
        bus_api::SetSlip {
            deck: 0,
            enabled: false,
        }
        .into(),
        bus_api::ToggleSlip { deck: 0 }.into(),
        bus_api::SetAutoLoop {
            deck: 0,
            beats: 0.5,
        }
        .into(),
        bus_api::LoopIn {
            deck: 0,
            position_ms: 0,
        }
        .into(),
        bus_api::LoopOut {
            deck: 0,
            position_ms: 0,
        }
        .into(),
        bus_api::ExitLoop { deck: 0 }.into(),
        bus_api::RecallSavedLoop {
            deck: 0,
            in_ms: 0,
            out_ms: 0,
        }
        .into(),
        bus_api::BeginLoopRoll {
            deck: 0,
            beats: 0.5,
        }
        .into(),
        bus_api::EndLoopRoll { deck: 0 }.into(),
        bus_api::BeatJump {
            deck: 0,
            beats: 0.5,
        }
        .into(),
        bus_api::SetPadMode {
            deck: 0,
            mode: PadMode::HotCue,
        }
        .into(),
        bus_api::PadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }
        .into(),
        bus_api::PadRelease { deck: 0, slot: 0 }.into(),
        bus_api::HotCuePadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }
        .into(),
        bus_api::HotCuePadRelease { deck: 0, slot: 0 }.into(),
        bus_api::LoopRollPadPress { deck: 0, slot: 0 }.into(),
        bus_api::LoopRollPadRelease { deck: 0, slot: 0 }.into(),
        bus_api::BeatJumpPadPress { deck: 0, slot: 0 }.into(),
        bus_api::BeatJumpPadRelease { deck: 0, slot: 0 }.into(),
        bus_api::SamplerPadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }
        .into(),
        bus_api::SamplerPadRelease { deck: 0, slot: 0 }.into(),
        bus_api::KeyboardPadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }
        .into(),
        bus_api::KeyboardPadRelease { deck: 0, slot: 0 }.into(),
        bus_api::KeyShiftPadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }
        .into(),
        bus_api::KeyShiftPadRelease { deck: 0, slot: 0 }.into(),
        bus_api::TriggerHotCue {
            deck: 0,
            position_ms: 0,
        }
        .into(),
        bus_api::SaveDeckHotCue { deck: 0, slot: 0 }.into(),
        bus_api::DeleteDeckHotCue { deck: 0, slot: 0 }.into(),
        bus_api::AssignSampler {
            deck: 0,
            slot: 0,
            path: String::new(),
        }
        .into(),
        bus_api::AssignSamplerTrack {
            deck: 0,
            slot: 0,
            track_id: String::new(),
        }
        .into(),
        bus_api::ClearSampler { deck: 0, slot: 0 }.into(),
        bus_api::JogTouch {
            deck: 0,
            touching: false,
        }
        .into(),
        bus_api::JogTurn { deck: 0, delta: 0 }.into(),
        bus_api::SetJogMode {
            deck: 0,
            top: JogMode::Vinyl,
            outer: JogMode::Vinyl,
        }
        .into(),
        bus_api::SetCrossfader {
            position: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetMasterVolume {
            volume: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetCueVolume {
            volume: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetCueMix {
            mix: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetMasterCue { enabled: false }.into(),
        bus_api::ToggleMasterCue {}.into(),
        bus_api::SetCurveLibrary {
            library: CurveLibrarySpec {
                groups: Vec::new(),
                sets: Vec::new(),
            },
        }
        .into(),
        bus_api::SetCrossfaderCurve { id: String::new() }.into(),
        bus_api::SetCrossfaderCurveAmount {
            set: String::new(),
            amount: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetCrossfaderBehavior {
            hamster: false,
            disabled: false,
        }
        .into(),
        bus_api::SetChannelFaderCurve { id: String::new() }.into(),
        bus_api::SetFxUnitSlot {
            slot: 0,
            kind: FxKind::Echo,
        }
        .into(),
        bus_api::SetFxUnitSlotEnabled {
            slot: 0,
            enabled: false,
        }
        .into(),
        bus_api::SetFxUnitWet {
            slot: 0,
            wet: 0.5,
            soft_takeover: false,
        }
        .into(),
        bus_api::SetFxLevelCurve { id: String::new() }.into(),
        bus_api::SetFxUnitParam {
            slot: 0,
            param: 0,
            value: 0.5,
        }
        .into(),
        bus_api::SetDeckFxRouted {
            deck: 0,
            routed: false,
        }
        .into(),
        bus_api::AnalyzeTrack {
            track_id: String::new(),
            force: false,
        }
        .into(),
        bus_api::GenerateStems {
            track_id: String::new(),
        }
        .into(),
        bus_api::RefreshTrack {
            track_id: String::new(),
        }
        .into(),
        bus_api::SaveBeatGrid {
            track_id: String::new(),
            bpm: 120.0,
            first_beat_secs: 0.5,
            replace: false,
        }
        .into(),
        bus_api::SaveTrackCue {
            track_id: String::new(),
            position_ms: 0,
        }
        .into(),
        bus_api::SaveTrackHotCue {
            track_id: String::new(),
            slot: 0,
            position_ms: 0,
            loop_length_beats: None,
            color: None,
            label: None,
        }
        .into(),
        bus_api::DeleteTrackHotCue {
            track_id: String::new(),
            slot: 0,
        }
        .into(),
        bus_api::SaveTrackLoop {
            track_id: String::new(),
            slot: 0,
            in_ms: 0,
            out_ms: 0,
            label: None,
            color: None,
        }
        .into(),
        bus_api::DeleteTrackLoop {
            track_id: String::new(),
            slot: 0,
        }
        .into(),
    ]
}

/// One instance of every event, built through its `From` conversion.
fn all_events() -> Vec<Event> {
    vec![
        bus_api::DeckUpdated {
            deck: 0,
            track: None,
            track_id: None,
            title: None,
            artist: None,
            album: None,
            isrc: None,
            bpm: None,
            key: None,
            playing: false,
            crossfader_assign: CrossfaderAssign::A,
            volume: 0.5,
            speed: 0.5,
            tempo_range: 0.5,
            playback_ratio: 0.5,
            key_lock: false,
            key_shift: 0.5,
            keyboard_page: 0,
            key_shift_page: 0,
            keyboard_root_hot_cue: 0,
            eq: DeckEq {
                low: 0.5,
                mid: 0.5,
                high: 0.5,
            },
            filter: 0.5,
            gain_trim: 0.5,
            fx_routed: false,
            fx: Vec::new(),
            headphone_cue: false,
            sync_mode: SyncMode::Off,
            cue_point_ms: None,
            quantize: false,
            active_loop: None,
            pending_loop_in_ms: None,
            slip_enabled: false,
            slip_shadow_position_ms: None,
            pad_mode: PadMode::HotCue,
            stems_ready: false,
            stems_audible: false,
            stems_live: false,
            stem_mute: Vec::new(),
            stem_isolate: None,
            stem_eq: Vec::new(),
            stem_eq_enabled: false,
            position_ms: None,
            duration_ms: None,
            hot_cues: Vec::new(),
            saved_loops: Vec::new(),
            loudness_lufs: None,
            auto_gain_db: 0.5,
            active_sampler_bank_id: None,
            top_jog_mode: JogMode::Vinyl,
            outer_jog_mode: JogMode::Vinyl,
            jog_touching: false,
            sampler_slots: Vec::new(),
        }
        .into(),
        bus_api::DeckPosition {
            deck: 0,
            position_ms: 0,
            slip_shadow_position_ms: None,
        }
        .into(),
        bus_api::DeckLevels {
            deck: None,
            peak_l: 0.5,
            peak_r: 0.5,
            peak_hold_l: 0.5,
            peak_hold_r: 0.5,
        }
        .into(),
        bus_api::EngineSnapshot {
            status: engine_status(),
        }
        .into(),
        bus_api::Error {
            message: String::new(),
            track_id: None,
            deck: None,
        }
        .into(),
        bus_api::Notice {
            message: String::new(),
        }
        .into(),
        bus_api::TrackAnalyzed {
            track: track_summary(),
        }
        .into(),
        bus_api::TrackUpdated {
            track: track_summary(),
        }
        .into(),
        bus_api::TrackProgress {
            track_id: String::new(),
            phase: String::new(),
            fraction: None,
        }
        .into(),
        bus_api::HotCuesChanged {
            track_id: String::new(),
            hot_cues: Vec::new(),
        }
        .into(),
        bus_api::LoopsChanged {
            track_id: String::new(),
            loops: Vec::new(),
        }
        .into(),
        bus_api::BeatGridChanged {
            track_id: String::new(),
            beat_grid: beat_grid(),
        }
        .into(),
        bus_api::HistorySessionUpdated { session_id: None }.into(),
        bus_api::Navigate { delta: 0 }.into(),
        bus_api::Focus { delta: 0 }.into(),
        bus_api::Load { deck: 0 }.into(),
        bus_api::MappingOffer {
            mapping_id: String::new(),
            device_id: String::new(),
            device_name: String::new(),
            port_name: String::new(),
        }
        .into(),
        bus_api::MappingAttached {
            mapping_id: String::new(),
            port_name: String::new(),
        }
        .into(),
        bus_api::MappingDetached {
            mapping_id: String::new(),
            port_name: String::new(),
        }
        .into(),
    ]
}

#[test]
fn the_catalog_is_nine_plus_eighty_five_commands_and_nineteen_events() {
    assert_eq!(LIBRARY_COMMANDS.len() + ENGINE_COMMANDS.len(), 94);
    assert_eq!(EVENT_NAMES.len(), 19);

    let mut all: Vec<&str> = LIBRARY_COMMANDS
        .iter()
        .chain(ENGINE_COMMANDS.iter())
        .copied()
        .collect();
    let total = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), total, "a command is listed twice");

    for library in LIBRARY_COMMANDS {
        assert!(
            !ENGINE_COMMANDS.contains(&library),
            "{library} cannot be owned by both workers"
        );
    }
}

#[test]
fn every_command_variant_routes_as_declared() {
    let commands = all_commands();
    assert_eq!(
        commands.len(),
        LIBRARY_COMMANDS.len() + ENGINE_COMMANDS.len(),
        "one fixture per command variant"
    );
    for command in &commands {
        let expected = expected_target(command);
        assert_eq!(command.target(), expected, "{command:?}");
        assert_eq!(Message::topic(command), expected, "{command:?}");
    }
}

#[test]
fn every_event_variant_maps_to_its_topic() {
    let events = all_events();
    assert_eq!(
        events.len(),
        EVENT_NAMES.len(),
        "one fixture per event variant"
    );
    for event in &events {
        let expected = expected_topic(event);
        assert_eq!(event.topic(), expected, "{event:?}");
        assert_eq!(Message::topic(event), expected, "{event:?}");
    }
}

#[test]
fn every_library_command_routes_to_the_library() {
    let commands: Vec<Command> = vec![
        bus_api::AnalyzeTrack {
            track_id: String::new(),
            force: false,
        }
        .into(),
        bus_api::GenerateStems {
            track_id: String::new(),
        }
        .into(),
        bus_api::RefreshTrack {
            track_id: String::new(),
        }
        .into(),
        bus_api::SaveBeatGrid {
            track_id: String::new(),
            bpm: 120.0,
            first_beat_secs: 0.0,
            replace: false,
        }
        .into(),
        bus_api::SaveTrackCue {
            track_id: String::new(),
            position_ms: 0,
        }
        .into(),
        bus_api::SaveTrackHotCue {
            track_id: String::new(),
            slot: 0,
            position_ms: 0,
            loop_length_beats: None,
            color: None,
            label: None,
        }
        .into(),
        bus_api::DeleteTrackHotCue {
            track_id: String::new(),
            slot: 0,
        }
        .into(),
        bus_api::SaveTrackLoop {
            track_id: String::new(),
            slot: 0,
            in_ms: 0,
            out_ms: 0,
            label: None,
            color: None,
        }
        .into(),
        bus_api::DeleteTrackLoop {
            track_id: String::new(),
            slot: 0,
        }
        .into(),
    ];
    assert_eq!(commands.len(), LIBRARY_COMMANDS.len());
    for command in &commands {
        assert_eq!(command.target(), CommandTarget::Library, "{command:?}");
        assert_eq!(expected_target(command), CommandTarget::Library);
        assert_eq!(Message::topic(command), CommandTarget::Library);
    }
}

#[test]
fn payloads_convert_into_their_direction() {
    let command: Command = bus_api::Play { deck: 2 }.into();
    assert_eq!(command, Command::Play(bus_api::Play { deck: 2 }));

    let event: Event = bus_api::Notice {
        message: "hi".into(),
    }
    .into();
    assert_eq!(
        event,
        Event::Notice(bus_api::Notice {
            message: "hi".into()
        })
    );
}
