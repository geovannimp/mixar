//! The control direction: every command the host, the controller and the
//! engine send to a worker.
//!
//! A command is owned: exactly one worker inbox receives it, which is why
//! [`CommandTarget`] is explicit per variant here and not on the wire.

use crate::messages::{
    AnalyzeTrack, AssignSampler, AssignSamplerTrack, BeatJump, BeatJumpPadPress,
    BeatJumpPadRelease, BeginCueHold, BeginLoopRoll, ClearSampler, DeleteDeckHotCue,
    DeleteTrackHotCue, DeleteTrackLoop, EndCueHold, EndLoopRoll, ExitLoop, FaderStart, FaderStop,
    GenerateStems, HotCuePadPress, HotCuePadRelease, JogTouch, JogTurn, KeyShiftPadPress,
    KeyShiftPadRelease, KeyboardPadPress, KeyboardPadRelease, LoopIn, LoopOut, LoopRollPadPress,
    LoopRollPadRelease, PadPress, PadRelease, Pause, Play, RecallSavedLoop, RefreshTrack,
    SamplerPadPress, SamplerPadRelease, SaveBeatGrid, SaveDeckHotCue, SaveTrackCue,
    SaveTrackHotCue, SaveTrackLoop, Seek, SetAutoLoop, SetChannelFaderCurve, SetCrossfader,
    SetCrossfaderAssign, SetCrossfaderBehavior, SetCrossfaderCurve, SetCrossfaderCurveAmount,
    SetCueMix, SetCuePoint, SetCueVolume, SetCurveLibrary, SetDeckFxRouted, SetEq, SetEqBand,
    SetFaderStartNative, SetFilter, SetFxLevelCurve, SetFxUnitParam, SetFxUnitSlot,
    SetFxUnitSlotEnabled, SetFxUnitWet, SetGainTrim, SetHeadphoneCue, SetJogMode, SetKeyLock,
    SetKeyShift, SetKeyShiftPage, SetKeyboardPage, SetKeyboardRoot, SetMasterCue, SetMasterDeck,
    SetMasterVolume, SetPadMode, SetQuantize, SetSlip, SetSpeed, SetStemEqBand, SetStemEqEnabled,
    SetTempoCurve, SetTempoRange, SetVolume, ToggleHeadphoneCue, ToggleMasterCue, TogglePlay,
    ToggleQuantize, ToggleSlip, ToggleStemEq, ToggleSync, TriggerHotCue, Unload,
};
use event_bus::Message;

/// The worker that owns a command's inbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandTarget {
    Engine,
    Library,
}

/// Every command that travels on the bus.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Play(Play),
    Pause(Pause),
    TogglePlay(TogglePlay),
    Seek(Seek),
    Unload(Unload),
    SetVolume(SetVolume),
    SetEq(SetEq),
    SetEqBand(SetEqBand),
    SetStemEqBand(SetStemEqBand),
    SetStemEqEnabled(SetStemEqEnabled),
    ToggleStemEq(ToggleStemEq),
    SetFilter(SetFilter),
    SetGainTrim(SetGainTrim),
    SetHeadphoneCue(SetHeadphoneCue),
    ToggleHeadphoneCue(ToggleHeadphoneCue),
    SetCrossfaderAssign(SetCrossfaderAssign),
    SetSpeed(SetSpeed),
    SetTempoRange(SetTempoRange),
    SetTempoCurve(SetTempoCurve),
    SetKeyLock(SetKeyLock),
    SetKeyShift(SetKeyShift),
    SetKeyboardRoot(SetKeyboardRoot),
    SetKeyboardPage(SetKeyboardPage),
    SetKeyShiftPage(SetKeyShiftPage),
    ToggleSync(ToggleSync),
    SetMasterDeck(SetMasterDeck),
    SetCuePoint(SetCuePoint),
    BeginCueHold(BeginCueHold),
    EndCueHold(EndCueHold),
    FaderStart(FaderStart),
    FaderStop(FaderStop),
    SetFaderStartNative(SetFaderStartNative),
    SetQuantize(SetQuantize),
    ToggleQuantize(ToggleQuantize),
    SetSlip(SetSlip),
    ToggleSlip(ToggleSlip),
    SetAutoLoop(SetAutoLoop),
    LoopIn(LoopIn),
    LoopOut(LoopOut),
    ExitLoop(ExitLoop),
    RecallSavedLoop(RecallSavedLoop),
    BeginLoopRoll(BeginLoopRoll),
    EndLoopRoll(EndLoopRoll),
    BeatJump(BeatJump),
    SetPadMode(SetPadMode),
    PadPress(PadPress),
    PadRelease(PadRelease),
    HotCuePadPress(HotCuePadPress),
    HotCuePadRelease(HotCuePadRelease),
    LoopRollPadPress(LoopRollPadPress),
    LoopRollPadRelease(LoopRollPadRelease),
    BeatJumpPadPress(BeatJumpPadPress),
    BeatJumpPadRelease(BeatJumpPadRelease),
    SamplerPadPress(SamplerPadPress),
    SamplerPadRelease(SamplerPadRelease),
    KeyboardPadPress(KeyboardPadPress),
    KeyboardPadRelease(KeyboardPadRelease),
    KeyShiftPadPress(KeyShiftPadPress),
    KeyShiftPadRelease(KeyShiftPadRelease),
    TriggerHotCue(TriggerHotCue),
    SaveDeckHotCue(SaveDeckHotCue),
    DeleteDeckHotCue(DeleteDeckHotCue),
    AssignSampler(AssignSampler),
    AssignSamplerTrack(AssignSamplerTrack),
    ClearSampler(ClearSampler),
    JogTouch(JogTouch),
    JogTurn(JogTurn),
    SetJogMode(SetJogMode),
    SetCrossfader(SetCrossfader),
    SetMasterVolume(SetMasterVolume),
    SetCueVolume(SetCueVolume),
    SetCueMix(SetCueMix),
    SetMasterCue(SetMasterCue),
    ToggleMasterCue(ToggleMasterCue),
    SetCurveLibrary(SetCurveLibrary),
    SetCrossfaderCurve(SetCrossfaderCurve),
    SetCrossfaderCurveAmount(SetCrossfaderCurveAmount),
    SetCrossfaderBehavior(SetCrossfaderBehavior),
    SetChannelFaderCurve(SetChannelFaderCurve),
    SetFxUnitSlot(SetFxUnitSlot),
    SetFxUnitSlotEnabled(SetFxUnitSlotEnabled),
    SetFxUnitWet(SetFxUnitWet),
    SetFxLevelCurve(SetFxLevelCurve),
    SetFxUnitParam(SetFxUnitParam),
    SetDeckFxRouted(SetDeckFxRouted),
    AnalyzeTrack(AnalyzeTrack),
    GenerateStems(GenerateStems),
    RefreshTrack(RefreshTrack),
    SaveBeatGrid(SaveBeatGrid),
    SaveTrackCue(SaveTrackCue),
    SaveTrackHotCue(SaveTrackHotCue),
    DeleteTrackHotCue(DeleteTrackHotCue),
    SaveTrackLoop(SaveTrackLoop),
    DeleteTrackLoop(DeleteTrackLoop),
}

impl Command {
    /// The worker that owns this command. Every variant is listed, so a new
    /// command cannot inherit a target by accident.
    pub fn target(&self) -> CommandTarget {
        match self {
            // library-owned
            Self::AnalyzeTrack(_)
            | Self::GenerateStems(_)
            | Self::RefreshTrack(_)
            | Self::SaveBeatGrid(_)
            | Self::SaveTrackCue(_)
            | Self::SaveTrackHotCue(_)
            | Self::DeleteTrackHotCue(_)
            | Self::SaveTrackLoop(_)
            | Self::DeleteTrackLoop(_) => CommandTarget::Library,
            // engine-owned
            Self::Play(_)
            | Self::Pause(_)
            | Self::TogglePlay(_)
            | Self::Seek(_)
            | Self::Unload(_)
            | Self::SetVolume(_)
            | Self::SetEq(_)
            | Self::SetEqBand(_)
            | Self::SetStemEqBand(_)
            | Self::SetStemEqEnabled(_)
            | Self::ToggleStemEq(_)
            | Self::SetFilter(_)
            | Self::SetGainTrim(_)
            | Self::SetHeadphoneCue(_)
            | Self::ToggleHeadphoneCue(_)
            | Self::SetCrossfaderAssign(_)
            | Self::SetSpeed(_)
            | Self::SetTempoRange(_)
            | Self::SetTempoCurve(_)
            | Self::SetKeyLock(_)
            | Self::SetKeyShift(_)
            | Self::SetKeyboardRoot(_)
            | Self::SetKeyboardPage(_)
            | Self::SetKeyShiftPage(_)
            | Self::ToggleSync(_)
            | Self::SetMasterDeck(_)
            | Self::SetCuePoint(_)
            | Self::BeginCueHold(_)
            | Self::EndCueHold(_)
            | Self::FaderStart(_)
            | Self::FaderStop(_)
            | Self::SetFaderStartNative(_)
            | Self::SetQuantize(_)
            | Self::ToggleQuantize(_)
            | Self::SetSlip(_)
            | Self::ToggleSlip(_)
            | Self::SetAutoLoop(_)
            | Self::LoopIn(_)
            | Self::LoopOut(_)
            | Self::ExitLoop(_)
            | Self::RecallSavedLoop(_)
            | Self::BeginLoopRoll(_)
            | Self::EndLoopRoll(_)
            | Self::BeatJump(_)
            | Self::SetPadMode(_)
            | Self::PadPress(_)
            | Self::PadRelease(_)
            | Self::HotCuePadPress(_)
            | Self::HotCuePadRelease(_)
            | Self::LoopRollPadPress(_)
            | Self::LoopRollPadRelease(_)
            | Self::BeatJumpPadPress(_)
            | Self::BeatJumpPadRelease(_)
            | Self::SamplerPadPress(_)
            | Self::SamplerPadRelease(_)
            | Self::KeyboardPadPress(_)
            | Self::KeyboardPadRelease(_)
            | Self::KeyShiftPadPress(_)
            | Self::KeyShiftPadRelease(_)
            | Self::TriggerHotCue(_)
            | Self::SaveDeckHotCue(_)
            | Self::DeleteDeckHotCue(_)
            | Self::AssignSampler(_)
            | Self::AssignSamplerTrack(_)
            | Self::ClearSampler(_)
            | Self::JogTouch(_)
            | Self::JogTurn(_)
            | Self::SetJogMode(_)
            | Self::SetCrossfader(_)
            | Self::SetMasterVolume(_)
            | Self::SetCueVolume(_)
            | Self::SetCueMix(_)
            | Self::SetMasterCue(_)
            | Self::ToggleMasterCue(_)
            | Self::SetCurveLibrary(_)
            | Self::SetCrossfaderCurve(_)
            | Self::SetCrossfaderCurveAmount(_)
            | Self::SetCrossfaderBehavior(_)
            | Self::SetChannelFaderCurve(_)
            | Self::SetFxUnitSlot(_)
            | Self::SetFxUnitSlotEnabled(_)
            | Self::SetFxUnitWet(_)
            | Self::SetFxLevelCurve(_)
            | Self::SetFxUnitParam(_)
            | Self::SetDeckFxRouted(_) => CommandTarget::Engine,
        }
    }
}

impl Message for Command {
    type Topic = CommandTarget;

    fn topic(&self) -> Self::Topic {
        Command::target(self)
    }
}

impl From<Play> for Command {
    fn from(payload: Play) -> Self {
        Self::Play(payload)
    }
}

impl From<Pause> for Command {
    fn from(payload: Pause) -> Self {
        Self::Pause(payload)
    }
}

impl From<TogglePlay> for Command {
    fn from(payload: TogglePlay) -> Self {
        Self::TogglePlay(payload)
    }
}

impl From<Seek> for Command {
    fn from(payload: Seek) -> Self {
        Self::Seek(payload)
    }
}

impl From<Unload> for Command {
    fn from(payload: Unload) -> Self {
        Self::Unload(payload)
    }
}

impl From<SetVolume> for Command {
    fn from(payload: SetVolume) -> Self {
        Self::SetVolume(payload)
    }
}

impl From<SetEq> for Command {
    fn from(payload: SetEq) -> Self {
        Self::SetEq(payload)
    }
}

impl From<SetEqBand> for Command {
    fn from(payload: SetEqBand) -> Self {
        Self::SetEqBand(payload)
    }
}

impl From<SetStemEqBand> for Command {
    fn from(payload: SetStemEqBand) -> Self {
        Self::SetStemEqBand(payload)
    }
}

impl From<SetStemEqEnabled> for Command {
    fn from(payload: SetStemEqEnabled) -> Self {
        Self::SetStemEqEnabled(payload)
    }
}

impl From<ToggleStemEq> for Command {
    fn from(payload: ToggleStemEq) -> Self {
        Self::ToggleStemEq(payload)
    }
}

impl From<SetFilter> for Command {
    fn from(payload: SetFilter) -> Self {
        Self::SetFilter(payload)
    }
}

impl From<SetGainTrim> for Command {
    fn from(payload: SetGainTrim) -> Self {
        Self::SetGainTrim(payload)
    }
}

impl From<SetHeadphoneCue> for Command {
    fn from(payload: SetHeadphoneCue) -> Self {
        Self::SetHeadphoneCue(payload)
    }
}

impl From<ToggleHeadphoneCue> for Command {
    fn from(payload: ToggleHeadphoneCue) -> Self {
        Self::ToggleHeadphoneCue(payload)
    }
}

impl From<SetCrossfaderAssign> for Command {
    fn from(payload: SetCrossfaderAssign) -> Self {
        Self::SetCrossfaderAssign(payload)
    }
}

impl From<SetSpeed> for Command {
    fn from(payload: SetSpeed) -> Self {
        Self::SetSpeed(payload)
    }
}

impl From<SetTempoRange> for Command {
    fn from(payload: SetTempoRange) -> Self {
        Self::SetTempoRange(payload)
    }
}

impl From<SetTempoCurve> for Command {
    fn from(payload: SetTempoCurve) -> Self {
        Self::SetTempoCurve(payload)
    }
}

impl From<SetKeyLock> for Command {
    fn from(payload: SetKeyLock) -> Self {
        Self::SetKeyLock(payload)
    }
}

impl From<SetKeyShift> for Command {
    fn from(payload: SetKeyShift) -> Self {
        Self::SetKeyShift(payload)
    }
}

impl From<SetKeyboardRoot> for Command {
    fn from(payload: SetKeyboardRoot) -> Self {
        Self::SetKeyboardRoot(payload)
    }
}

impl From<SetKeyboardPage> for Command {
    fn from(payload: SetKeyboardPage) -> Self {
        Self::SetKeyboardPage(payload)
    }
}

impl From<SetKeyShiftPage> for Command {
    fn from(payload: SetKeyShiftPage) -> Self {
        Self::SetKeyShiftPage(payload)
    }
}

impl From<ToggleSync> for Command {
    fn from(payload: ToggleSync) -> Self {
        Self::ToggleSync(payload)
    }
}

impl From<SetMasterDeck> for Command {
    fn from(payload: SetMasterDeck) -> Self {
        Self::SetMasterDeck(payload)
    }
}

impl From<SetCuePoint> for Command {
    fn from(payload: SetCuePoint) -> Self {
        Self::SetCuePoint(payload)
    }
}

impl From<BeginCueHold> for Command {
    fn from(payload: BeginCueHold) -> Self {
        Self::BeginCueHold(payload)
    }
}

impl From<EndCueHold> for Command {
    fn from(payload: EndCueHold) -> Self {
        Self::EndCueHold(payload)
    }
}

impl From<FaderStart> for Command {
    fn from(payload: FaderStart) -> Self {
        Self::FaderStart(payload)
    }
}

impl From<FaderStop> for Command {
    fn from(payload: FaderStop) -> Self {
        Self::FaderStop(payload)
    }
}

impl From<SetFaderStartNative> for Command {
    fn from(payload: SetFaderStartNative) -> Self {
        Self::SetFaderStartNative(payload)
    }
}

impl From<SetQuantize> for Command {
    fn from(payload: SetQuantize) -> Self {
        Self::SetQuantize(payload)
    }
}

impl From<ToggleQuantize> for Command {
    fn from(payload: ToggleQuantize) -> Self {
        Self::ToggleQuantize(payload)
    }
}

impl From<SetSlip> for Command {
    fn from(payload: SetSlip) -> Self {
        Self::SetSlip(payload)
    }
}

impl From<ToggleSlip> for Command {
    fn from(payload: ToggleSlip) -> Self {
        Self::ToggleSlip(payload)
    }
}

impl From<SetAutoLoop> for Command {
    fn from(payload: SetAutoLoop) -> Self {
        Self::SetAutoLoop(payload)
    }
}

impl From<LoopIn> for Command {
    fn from(payload: LoopIn) -> Self {
        Self::LoopIn(payload)
    }
}

impl From<LoopOut> for Command {
    fn from(payload: LoopOut) -> Self {
        Self::LoopOut(payload)
    }
}

impl From<ExitLoop> for Command {
    fn from(payload: ExitLoop) -> Self {
        Self::ExitLoop(payload)
    }
}

impl From<RecallSavedLoop> for Command {
    fn from(payload: RecallSavedLoop) -> Self {
        Self::RecallSavedLoop(payload)
    }
}

impl From<BeginLoopRoll> for Command {
    fn from(payload: BeginLoopRoll) -> Self {
        Self::BeginLoopRoll(payload)
    }
}

impl From<EndLoopRoll> for Command {
    fn from(payload: EndLoopRoll) -> Self {
        Self::EndLoopRoll(payload)
    }
}

impl From<BeatJump> for Command {
    fn from(payload: BeatJump) -> Self {
        Self::BeatJump(payload)
    }
}

impl From<SetPadMode> for Command {
    fn from(payload: SetPadMode) -> Self {
        Self::SetPadMode(payload)
    }
}

impl From<PadPress> for Command {
    fn from(payload: PadPress) -> Self {
        Self::PadPress(payload)
    }
}

impl From<PadRelease> for Command {
    fn from(payload: PadRelease) -> Self {
        Self::PadRelease(payload)
    }
}

impl From<HotCuePadPress> for Command {
    fn from(payload: HotCuePadPress) -> Self {
        Self::HotCuePadPress(payload)
    }
}

impl From<HotCuePadRelease> for Command {
    fn from(payload: HotCuePadRelease) -> Self {
        Self::HotCuePadRelease(payload)
    }
}

impl From<LoopRollPadPress> for Command {
    fn from(payload: LoopRollPadPress) -> Self {
        Self::LoopRollPadPress(payload)
    }
}

impl From<LoopRollPadRelease> for Command {
    fn from(payload: LoopRollPadRelease) -> Self {
        Self::LoopRollPadRelease(payload)
    }
}

impl From<BeatJumpPadPress> for Command {
    fn from(payload: BeatJumpPadPress) -> Self {
        Self::BeatJumpPadPress(payload)
    }
}

impl From<BeatJumpPadRelease> for Command {
    fn from(payload: BeatJumpPadRelease) -> Self {
        Self::BeatJumpPadRelease(payload)
    }
}

impl From<SamplerPadPress> for Command {
    fn from(payload: SamplerPadPress) -> Self {
        Self::SamplerPadPress(payload)
    }
}

impl From<SamplerPadRelease> for Command {
    fn from(payload: SamplerPadRelease) -> Self {
        Self::SamplerPadRelease(payload)
    }
}

impl From<KeyboardPadPress> for Command {
    fn from(payload: KeyboardPadPress) -> Self {
        Self::KeyboardPadPress(payload)
    }
}

impl From<KeyboardPadRelease> for Command {
    fn from(payload: KeyboardPadRelease) -> Self {
        Self::KeyboardPadRelease(payload)
    }
}

impl From<KeyShiftPadPress> for Command {
    fn from(payload: KeyShiftPadPress) -> Self {
        Self::KeyShiftPadPress(payload)
    }
}

impl From<KeyShiftPadRelease> for Command {
    fn from(payload: KeyShiftPadRelease) -> Self {
        Self::KeyShiftPadRelease(payload)
    }
}

impl From<TriggerHotCue> for Command {
    fn from(payload: TriggerHotCue) -> Self {
        Self::TriggerHotCue(payload)
    }
}

impl From<SaveDeckHotCue> for Command {
    fn from(payload: SaveDeckHotCue) -> Self {
        Self::SaveDeckHotCue(payload)
    }
}

impl From<DeleteDeckHotCue> for Command {
    fn from(payload: DeleteDeckHotCue) -> Self {
        Self::DeleteDeckHotCue(payload)
    }
}

impl From<AssignSampler> for Command {
    fn from(payload: AssignSampler) -> Self {
        Self::AssignSampler(payload)
    }
}

impl From<AssignSamplerTrack> for Command {
    fn from(payload: AssignSamplerTrack) -> Self {
        Self::AssignSamplerTrack(payload)
    }
}

impl From<ClearSampler> for Command {
    fn from(payload: ClearSampler) -> Self {
        Self::ClearSampler(payload)
    }
}

impl From<JogTouch> for Command {
    fn from(payload: JogTouch) -> Self {
        Self::JogTouch(payload)
    }
}

impl From<JogTurn> for Command {
    fn from(payload: JogTurn) -> Self {
        Self::JogTurn(payload)
    }
}

impl From<SetJogMode> for Command {
    fn from(payload: SetJogMode) -> Self {
        Self::SetJogMode(payload)
    }
}

impl From<SetCrossfader> for Command {
    fn from(payload: SetCrossfader) -> Self {
        Self::SetCrossfader(payload)
    }
}

impl From<SetMasterVolume> for Command {
    fn from(payload: SetMasterVolume) -> Self {
        Self::SetMasterVolume(payload)
    }
}

impl From<SetCueVolume> for Command {
    fn from(payload: SetCueVolume) -> Self {
        Self::SetCueVolume(payload)
    }
}

impl From<SetCueMix> for Command {
    fn from(payload: SetCueMix) -> Self {
        Self::SetCueMix(payload)
    }
}

impl From<SetMasterCue> for Command {
    fn from(payload: SetMasterCue) -> Self {
        Self::SetMasterCue(payload)
    }
}

impl From<ToggleMasterCue> for Command {
    fn from(payload: ToggleMasterCue) -> Self {
        Self::ToggleMasterCue(payload)
    }
}

impl From<SetCurveLibrary> for Command {
    fn from(payload: SetCurveLibrary) -> Self {
        Self::SetCurveLibrary(payload)
    }
}

impl From<SetCrossfaderCurve> for Command {
    fn from(payload: SetCrossfaderCurve) -> Self {
        Self::SetCrossfaderCurve(payload)
    }
}

impl From<SetCrossfaderCurveAmount> for Command {
    fn from(payload: SetCrossfaderCurveAmount) -> Self {
        Self::SetCrossfaderCurveAmount(payload)
    }
}

impl From<SetCrossfaderBehavior> for Command {
    fn from(payload: SetCrossfaderBehavior) -> Self {
        Self::SetCrossfaderBehavior(payload)
    }
}

impl From<SetChannelFaderCurve> for Command {
    fn from(payload: SetChannelFaderCurve) -> Self {
        Self::SetChannelFaderCurve(payload)
    }
}

impl From<SetFxUnitSlot> for Command {
    fn from(payload: SetFxUnitSlot) -> Self {
        Self::SetFxUnitSlot(payload)
    }
}

impl From<SetFxUnitSlotEnabled> for Command {
    fn from(payload: SetFxUnitSlotEnabled) -> Self {
        Self::SetFxUnitSlotEnabled(payload)
    }
}

impl From<SetFxUnitWet> for Command {
    fn from(payload: SetFxUnitWet) -> Self {
        Self::SetFxUnitWet(payload)
    }
}

impl From<SetFxLevelCurve> for Command {
    fn from(payload: SetFxLevelCurve) -> Self {
        Self::SetFxLevelCurve(payload)
    }
}

impl From<SetFxUnitParam> for Command {
    fn from(payload: SetFxUnitParam) -> Self {
        Self::SetFxUnitParam(payload)
    }
}

impl From<SetDeckFxRouted> for Command {
    fn from(payload: SetDeckFxRouted) -> Self {
        Self::SetDeckFxRouted(payload)
    }
}

impl From<AnalyzeTrack> for Command {
    fn from(payload: AnalyzeTrack) -> Self {
        Self::AnalyzeTrack(payload)
    }
}

impl From<GenerateStems> for Command {
    fn from(payload: GenerateStems) -> Self {
        Self::GenerateStems(payload)
    }
}

impl From<RefreshTrack> for Command {
    fn from(payload: RefreshTrack) -> Self {
        Self::RefreshTrack(payload)
    }
}

impl From<SaveBeatGrid> for Command {
    fn from(payload: SaveBeatGrid) -> Self {
        Self::SaveBeatGrid(payload)
    }
}

impl From<SaveTrackCue> for Command {
    fn from(payload: SaveTrackCue) -> Self {
        Self::SaveTrackCue(payload)
    }
}

impl From<SaveTrackHotCue> for Command {
    fn from(payload: SaveTrackHotCue) -> Self {
        Self::SaveTrackHotCue(payload)
    }
}

impl From<DeleteTrackHotCue> for Command {
    fn from(payload: DeleteTrackHotCue) -> Self {
        Self::DeleteTrackHotCue(payload)
    }
}

impl From<SaveTrackLoop> for Command {
    fn from(payload: SaveTrackLoop) -> Self {
        Self::SaveTrackLoop(payload)
    }
}

impl From<DeleteTrackLoop> for Command {
    fn from(payload: DeleteTrackLoop) -> Self {
        Self::DeleteTrackLoop(payload)
    }
}
