// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'event_mirrors.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$Event {

 Object get field0;



@override
bool operator ==(Object other) {
  final _this = this as Event;
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Event&&const DeepCollectionEquality().equals(other.field0, _this.field0));
}


@override
int get hashCode {
  final _this = this as Event;
  return Object.hash(runtimeType,const DeepCollectionEquality().hash(_this.field0));
}

@override
String toString() {
  final _this = this as Event;
  return 'Event(field0: ${_this.field0})';
}


}

/// @nodoc
class $EventCopyWith<$Res>  {
$EventCopyWith(Event _, $Res Function(Event) __);
}


/// Adds pattern-matching-related methods to [Event].
extension EventPatterns on Event {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Event_Play value)?  play,TResult Function( Event_Pause value)?  pause,TResult Function( Event_TogglePlay value)?  togglePlay,TResult Function( Event_Seek value)?  seek,TResult Function( Event_Unload value)?  unload,TResult Function( Event_SetVolume value)?  setVolume,TResult Function( Event_SetEq value)?  setEq,TResult Function( Event_SetEqBand value)?  setEqBand,TResult Function( Event_SetFilter value)?  setFilter,TResult Function( Event_SetGainTrim value)?  setGainTrim,TResult Function( Event_SetHeadphoneCue value)?  setHeadphoneCue,TResult Function( Event_ToggleHeadphoneCue value)?  toggleHeadphoneCue,TResult Function( Event_SetCrossfaderAssign value)?  setCrossfaderAssign,TResult Function( Event_SetSpeed value)?  setSpeed,TResult Function( Event_SetTempoRange value)?  setTempoRange,TResult Function( Event_SetKeyLock value)?  setKeyLock,TResult Function( Event_SetKeyShift value)?  setKeyShift,TResult Function( Event_SetKeyboardRoot value)?  setKeyboardRoot,TResult Function( Event_SetKeyboardPage value)?  setKeyboardPage,TResult Function( Event_SetKeyShiftPage value)?  setKeyShiftPage,TResult Function( Event_ToggleSync value)?  toggleSync,TResult Function( Event_SetMasterDeck value)?  setMasterDeck,TResult Function( Event_SetCuePoint value)?  setCuePoint,TResult Function( Event_BeginCueHold value)?  beginCueHold,TResult Function( Event_EndCueHold value)?  endCueHold,TResult Function( Event_SetQuantize value)?  setQuantize,TResult Function( Event_ToggleQuantize value)?  toggleQuantize,TResult Function( Event_SetSlip value)?  setSlip,TResult Function( Event_ToggleSlip value)?  toggleSlip,TResult Function( Event_SetAutoLoop value)?  setAutoLoop,TResult Function( Event_LoopIn value)?  loopIn,TResult Function( Event_LoopOut value)?  loopOut,TResult Function( Event_ExitLoop value)?  exitLoop,TResult Function( Event_RecallSavedLoop value)?  recallSavedLoop,TResult Function( Event_BeginLoopRoll value)?  beginLoopRoll,TResult Function( Event_EndLoopRoll value)?  endLoopRoll,TResult Function( Event_BeatJump value)?  beatJump,TResult Function( Event_SetPadMode value)?  setPadMode,TResult Function( Event_PadPress value)?  padPress,TResult Function( Event_PadRelease value)?  padRelease,TResult Function( Event_HotCuePadPress value)?  hotCuePadPress,TResult Function( Event_HotCuePadRelease value)?  hotCuePadRelease,TResult Function( Event_LoopRollPadPress value)?  loopRollPadPress,TResult Function( Event_LoopRollPadRelease value)?  loopRollPadRelease,TResult Function( Event_BeatJumpPadPress value)?  beatJumpPadPress,TResult Function( Event_BeatJumpPadRelease value)?  beatJumpPadRelease,TResult Function( Event_SamplerPadPress value)?  samplerPadPress,TResult Function( Event_SamplerPadRelease value)?  samplerPadRelease,TResult Function( Event_KeyboardPadPress value)?  keyboardPadPress,TResult Function( Event_KeyboardPadRelease value)?  keyboardPadRelease,TResult Function( Event_KeyShiftPadPress value)?  keyShiftPadPress,TResult Function( Event_KeyShiftPadRelease value)?  keyShiftPadRelease,TResult Function( Event_TriggerHotCue value)?  triggerHotCue,TResult Function( Event_SaveDeckHotCue value)?  saveDeckHotCue,TResult Function( Event_DeleteDeckHotCue value)?  deleteDeckHotCue,TResult Function( Event_AssignSampler value)?  assignSampler,TResult Function( Event_AssignSamplerTrack value)?  assignSamplerTrack,TResult Function( Event_ClearSampler value)?  clearSampler,TResult Function( Event_JogTouch value)?  jogTouch,TResult Function( Event_JogTurn value)?  jogTurn,TResult Function( Event_SetJogMode value)?  setJogMode,TResult Function( Event_SetCrossfader value)?  setCrossfader,TResult Function( Event_SetMasterVolume value)?  setMasterVolume,TResult Function( Event_SetCueVolume value)?  setCueVolume,TResult Function( Event_SetCueMix value)?  setCueMix,TResult Function( Event_SetMasterCue value)?  setMasterCue,TResult Function( Event_ToggleMasterCue value)?  toggleMasterCue,TResult Function( Event_SetFxUnitSlot value)?  setFxUnitSlot,TResult Function( Event_SetFxUnitSlotEnabled value)?  setFxUnitSlotEnabled,TResult Function( Event_SetFxUnitWet value)?  setFxUnitWet,TResult Function( Event_SetFxUnitParam value)?  setFxUnitParam,TResult Function( Event_SetDeckFxRouted value)?  setDeckFxRouted,TResult Function( Event_DeckUpdated value)?  deckUpdated,TResult Function( Event_DeckPosition value)?  deckPosition,TResult Function( Event_DeckLevels value)?  deckLevels,TResult Function( Event_EngineSnapshot value)?  engineSnapshot,TResult Function( Event_Error value)?  error,TResult Function( Event_Notice value)?  notice,TResult Function( Event_AnalyzeTrack value)?  analyzeTrack,TResult Function( Event_GenerateStems value)?  generateStems,TResult Function( Event_RefreshTrack value)?  refreshTrack,TResult Function( Event_SaveBeatGrid value)?  saveBeatGrid,TResult Function( Event_SaveTrackHotCue value)?  saveTrackHotCue,TResult Function( Event_DeleteTrackHotCue value)?  deleteTrackHotCue,TResult Function( Event_SaveTrackLoop value)?  saveTrackLoop,TResult Function( Event_DeleteTrackLoop value)?  deleteTrackLoop,TResult Function( Event_TrackAnalyzed value)?  trackAnalyzed,TResult Function( Event_TrackUpdated value)?  trackUpdated,TResult Function( Event_TrackProgress value)?  trackProgress,TResult Function( Event_HotCuesChanged value)?  hotCuesChanged,TResult Function( Event_LoopsChanged value)?  loopsChanged,TResult Function( Event_BeatGridChanged value)?  beatGridChanged,TResult Function( Event_HistorySessionUpdated value)?  historySessionUpdated,TResult Function( Event_Navigate value)?  navigate,TResult Function( Event_Focus value)?  focus,TResult Function( Event_Load value)?  load,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Event_Play() when play != null:
return play(_that);case Event_Pause() when pause != null:
return pause(_that);case Event_TogglePlay() when togglePlay != null:
return togglePlay(_that);case Event_Seek() when seek != null:
return seek(_that);case Event_Unload() when unload != null:
return unload(_that);case Event_SetVolume() when setVolume != null:
return setVolume(_that);case Event_SetEq() when setEq != null:
return setEq(_that);case Event_SetEqBand() when setEqBand != null:
return setEqBand(_that);case Event_SetFilter() when setFilter != null:
return setFilter(_that);case Event_SetGainTrim() when setGainTrim != null:
return setGainTrim(_that);case Event_SetHeadphoneCue() when setHeadphoneCue != null:
return setHeadphoneCue(_that);case Event_ToggleHeadphoneCue() when toggleHeadphoneCue != null:
return toggleHeadphoneCue(_that);case Event_SetCrossfaderAssign() when setCrossfaderAssign != null:
return setCrossfaderAssign(_that);case Event_SetSpeed() when setSpeed != null:
return setSpeed(_that);case Event_SetTempoRange() when setTempoRange != null:
return setTempoRange(_that);case Event_SetKeyLock() when setKeyLock != null:
return setKeyLock(_that);case Event_SetKeyShift() when setKeyShift != null:
return setKeyShift(_that);case Event_SetKeyboardRoot() when setKeyboardRoot != null:
return setKeyboardRoot(_that);case Event_SetKeyboardPage() when setKeyboardPage != null:
return setKeyboardPage(_that);case Event_SetKeyShiftPage() when setKeyShiftPage != null:
return setKeyShiftPage(_that);case Event_ToggleSync() when toggleSync != null:
return toggleSync(_that);case Event_SetMasterDeck() when setMasterDeck != null:
return setMasterDeck(_that);case Event_SetCuePoint() when setCuePoint != null:
return setCuePoint(_that);case Event_BeginCueHold() when beginCueHold != null:
return beginCueHold(_that);case Event_EndCueHold() when endCueHold != null:
return endCueHold(_that);case Event_SetQuantize() when setQuantize != null:
return setQuantize(_that);case Event_ToggleQuantize() when toggleQuantize != null:
return toggleQuantize(_that);case Event_SetSlip() when setSlip != null:
return setSlip(_that);case Event_ToggleSlip() when toggleSlip != null:
return toggleSlip(_that);case Event_SetAutoLoop() when setAutoLoop != null:
return setAutoLoop(_that);case Event_LoopIn() when loopIn != null:
return loopIn(_that);case Event_LoopOut() when loopOut != null:
return loopOut(_that);case Event_ExitLoop() when exitLoop != null:
return exitLoop(_that);case Event_RecallSavedLoop() when recallSavedLoop != null:
return recallSavedLoop(_that);case Event_BeginLoopRoll() when beginLoopRoll != null:
return beginLoopRoll(_that);case Event_EndLoopRoll() when endLoopRoll != null:
return endLoopRoll(_that);case Event_BeatJump() when beatJump != null:
return beatJump(_that);case Event_SetPadMode() when setPadMode != null:
return setPadMode(_that);case Event_PadPress() when padPress != null:
return padPress(_that);case Event_PadRelease() when padRelease != null:
return padRelease(_that);case Event_HotCuePadPress() when hotCuePadPress != null:
return hotCuePadPress(_that);case Event_HotCuePadRelease() when hotCuePadRelease != null:
return hotCuePadRelease(_that);case Event_LoopRollPadPress() when loopRollPadPress != null:
return loopRollPadPress(_that);case Event_LoopRollPadRelease() when loopRollPadRelease != null:
return loopRollPadRelease(_that);case Event_BeatJumpPadPress() when beatJumpPadPress != null:
return beatJumpPadPress(_that);case Event_BeatJumpPadRelease() when beatJumpPadRelease != null:
return beatJumpPadRelease(_that);case Event_SamplerPadPress() when samplerPadPress != null:
return samplerPadPress(_that);case Event_SamplerPadRelease() when samplerPadRelease != null:
return samplerPadRelease(_that);case Event_KeyboardPadPress() when keyboardPadPress != null:
return keyboardPadPress(_that);case Event_KeyboardPadRelease() when keyboardPadRelease != null:
return keyboardPadRelease(_that);case Event_KeyShiftPadPress() when keyShiftPadPress != null:
return keyShiftPadPress(_that);case Event_KeyShiftPadRelease() when keyShiftPadRelease != null:
return keyShiftPadRelease(_that);case Event_TriggerHotCue() when triggerHotCue != null:
return triggerHotCue(_that);case Event_SaveDeckHotCue() when saveDeckHotCue != null:
return saveDeckHotCue(_that);case Event_DeleteDeckHotCue() when deleteDeckHotCue != null:
return deleteDeckHotCue(_that);case Event_AssignSampler() when assignSampler != null:
return assignSampler(_that);case Event_AssignSamplerTrack() when assignSamplerTrack != null:
return assignSamplerTrack(_that);case Event_ClearSampler() when clearSampler != null:
return clearSampler(_that);case Event_JogTouch() when jogTouch != null:
return jogTouch(_that);case Event_JogTurn() when jogTurn != null:
return jogTurn(_that);case Event_SetJogMode() when setJogMode != null:
return setJogMode(_that);case Event_SetCrossfader() when setCrossfader != null:
return setCrossfader(_that);case Event_SetMasterVolume() when setMasterVolume != null:
return setMasterVolume(_that);case Event_SetCueVolume() when setCueVolume != null:
return setCueVolume(_that);case Event_SetCueMix() when setCueMix != null:
return setCueMix(_that);case Event_SetMasterCue() when setMasterCue != null:
return setMasterCue(_that);case Event_ToggleMasterCue() when toggleMasterCue != null:
return toggleMasterCue(_that);case Event_SetFxUnitSlot() when setFxUnitSlot != null:
return setFxUnitSlot(_that);case Event_SetFxUnitSlotEnabled() when setFxUnitSlotEnabled != null:
return setFxUnitSlotEnabled(_that);case Event_SetFxUnitWet() when setFxUnitWet != null:
return setFxUnitWet(_that);case Event_SetFxUnitParam() when setFxUnitParam != null:
return setFxUnitParam(_that);case Event_SetDeckFxRouted() when setDeckFxRouted != null:
return setDeckFxRouted(_that);case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that);case Event_Error() when error != null:
return error(_that);case Event_Notice() when notice != null:
return notice(_that);case Event_AnalyzeTrack() when analyzeTrack != null:
return analyzeTrack(_that);case Event_GenerateStems() when generateStems != null:
return generateStems(_that);case Event_RefreshTrack() when refreshTrack != null:
return refreshTrack(_that);case Event_SaveBeatGrid() when saveBeatGrid != null:
return saveBeatGrid(_that);case Event_SaveTrackHotCue() when saveTrackHotCue != null:
return saveTrackHotCue(_that);case Event_DeleteTrackHotCue() when deleteTrackHotCue != null:
return deleteTrackHotCue(_that);case Event_SaveTrackLoop() when saveTrackLoop != null:
return saveTrackLoop(_that);case Event_DeleteTrackLoop() when deleteTrackLoop != null:
return deleteTrackLoop(_that);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that);case Event_Navigate() when navigate != null:
return navigate(_that);case Event_Focus() when focus != null:
return focus(_that);case Event_Load() when load != null:
return load(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Event_Play value)  play,required TResult Function( Event_Pause value)  pause,required TResult Function( Event_TogglePlay value)  togglePlay,required TResult Function( Event_Seek value)  seek,required TResult Function( Event_Unload value)  unload,required TResult Function( Event_SetVolume value)  setVolume,required TResult Function( Event_SetEq value)  setEq,required TResult Function( Event_SetEqBand value)  setEqBand,required TResult Function( Event_SetFilter value)  setFilter,required TResult Function( Event_SetGainTrim value)  setGainTrim,required TResult Function( Event_SetHeadphoneCue value)  setHeadphoneCue,required TResult Function( Event_ToggleHeadphoneCue value)  toggleHeadphoneCue,required TResult Function( Event_SetCrossfaderAssign value)  setCrossfaderAssign,required TResult Function( Event_SetSpeed value)  setSpeed,required TResult Function( Event_SetTempoRange value)  setTempoRange,required TResult Function( Event_SetKeyLock value)  setKeyLock,required TResult Function( Event_SetKeyShift value)  setKeyShift,required TResult Function( Event_SetKeyboardRoot value)  setKeyboardRoot,required TResult Function( Event_SetKeyboardPage value)  setKeyboardPage,required TResult Function( Event_SetKeyShiftPage value)  setKeyShiftPage,required TResult Function( Event_ToggleSync value)  toggleSync,required TResult Function( Event_SetMasterDeck value)  setMasterDeck,required TResult Function( Event_SetCuePoint value)  setCuePoint,required TResult Function( Event_BeginCueHold value)  beginCueHold,required TResult Function( Event_EndCueHold value)  endCueHold,required TResult Function( Event_SetQuantize value)  setQuantize,required TResult Function( Event_ToggleQuantize value)  toggleQuantize,required TResult Function( Event_SetSlip value)  setSlip,required TResult Function( Event_ToggleSlip value)  toggleSlip,required TResult Function( Event_SetAutoLoop value)  setAutoLoop,required TResult Function( Event_LoopIn value)  loopIn,required TResult Function( Event_LoopOut value)  loopOut,required TResult Function( Event_ExitLoop value)  exitLoop,required TResult Function( Event_RecallSavedLoop value)  recallSavedLoop,required TResult Function( Event_BeginLoopRoll value)  beginLoopRoll,required TResult Function( Event_EndLoopRoll value)  endLoopRoll,required TResult Function( Event_BeatJump value)  beatJump,required TResult Function( Event_SetPadMode value)  setPadMode,required TResult Function( Event_PadPress value)  padPress,required TResult Function( Event_PadRelease value)  padRelease,required TResult Function( Event_HotCuePadPress value)  hotCuePadPress,required TResult Function( Event_HotCuePadRelease value)  hotCuePadRelease,required TResult Function( Event_LoopRollPadPress value)  loopRollPadPress,required TResult Function( Event_LoopRollPadRelease value)  loopRollPadRelease,required TResult Function( Event_BeatJumpPadPress value)  beatJumpPadPress,required TResult Function( Event_BeatJumpPadRelease value)  beatJumpPadRelease,required TResult Function( Event_SamplerPadPress value)  samplerPadPress,required TResult Function( Event_SamplerPadRelease value)  samplerPadRelease,required TResult Function( Event_KeyboardPadPress value)  keyboardPadPress,required TResult Function( Event_KeyboardPadRelease value)  keyboardPadRelease,required TResult Function( Event_KeyShiftPadPress value)  keyShiftPadPress,required TResult Function( Event_KeyShiftPadRelease value)  keyShiftPadRelease,required TResult Function( Event_TriggerHotCue value)  triggerHotCue,required TResult Function( Event_SaveDeckHotCue value)  saveDeckHotCue,required TResult Function( Event_DeleteDeckHotCue value)  deleteDeckHotCue,required TResult Function( Event_AssignSampler value)  assignSampler,required TResult Function( Event_AssignSamplerTrack value)  assignSamplerTrack,required TResult Function( Event_ClearSampler value)  clearSampler,required TResult Function( Event_JogTouch value)  jogTouch,required TResult Function( Event_JogTurn value)  jogTurn,required TResult Function( Event_SetJogMode value)  setJogMode,required TResult Function( Event_SetCrossfader value)  setCrossfader,required TResult Function( Event_SetMasterVolume value)  setMasterVolume,required TResult Function( Event_SetCueVolume value)  setCueVolume,required TResult Function( Event_SetCueMix value)  setCueMix,required TResult Function( Event_SetMasterCue value)  setMasterCue,required TResult Function( Event_ToggleMasterCue value)  toggleMasterCue,required TResult Function( Event_SetFxUnitSlot value)  setFxUnitSlot,required TResult Function( Event_SetFxUnitSlotEnabled value)  setFxUnitSlotEnabled,required TResult Function( Event_SetFxUnitWet value)  setFxUnitWet,required TResult Function( Event_SetFxUnitParam value)  setFxUnitParam,required TResult Function( Event_SetDeckFxRouted value)  setDeckFxRouted,required TResult Function( Event_DeckUpdated value)  deckUpdated,required TResult Function( Event_DeckPosition value)  deckPosition,required TResult Function( Event_DeckLevels value)  deckLevels,required TResult Function( Event_EngineSnapshot value)  engineSnapshot,required TResult Function( Event_Error value)  error,required TResult Function( Event_Notice value)  notice,required TResult Function( Event_AnalyzeTrack value)  analyzeTrack,required TResult Function( Event_GenerateStems value)  generateStems,required TResult Function( Event_RefreshTrack value)  refreshTrack,required TResult Function( Event_SaveBeatGrid value)  saveBeatGrid,required TResult Function( Event_SaveTrackHotCue value)  saveTrackHotCue,required TResult Function( Event_DeleteTrackHotCue value)  deleteTrackHotCue,required TResult Function( Event_SaveTrackLoop value)  saveTrackLoop,required TResult Function( Event_DeleteTrackLoop value)  deleteTrackLoop,required TResult Function( Event_TrackAnalyzed value)  trackAnalyzed,required TResult Function( Event_TrackUpdated value)  trackUpdated,required TResult Function( Event_TrackProgress value)  trackProgress,required TResult Function( Event_HotCuesChanged value)  hotCuesChanged,required TResult Function( Event_LoopsChanged value)  loopsChanged,required TResult Function( Event_BeatGridChanged value)  beatGridChanged,required TResult Function( Event_HistorySessionUpdated value)  historySessionUpdated,required TResult Function( Event_Navigate value)  navigate,required TResult Function( Event_Focus value)  focus,required TResult Function( Event_Load value)  load,}){
final _that = this;
switch (_that) {
case Event_Play():
return play(_that);case Event_Pause():
return pause(_that);case Event_TogglePlay():
return togglePlay(_that);case Event_Seek():
return seek(_that);case Event_Unload():
return unload(_that);case Event_SetVolume():
return setVolume(_that);case Event_SetEq():
return setEq(_that);case Event_SetEqBand():
return setEqBand(_that);case Event_SetFilter():
return setFilter(_that);case Event_SetGainTrim():
return setGainTrim(_that);case Event_SetHeadphoneCue():
return setHeadphoneCue(_that);case Event_ToggleHeadphoneCue():
return toggleHeadphoneCue(_that);case Event_SetCrossfaderAssign():
return setCrossfaderAssign(_that);case Event_SetSpeed():
return setSpeed(_that);case Event_SetTempoRange():
return setTempoRange(_that);case Event_SetKeyLock():
return setKeyLock(_that);case Event_SetKeyShift():
return setKeyShift(_that);case Event_SetKeyboardRoot():
return setKeyboardRoot(_that);case Event_SetKeyboardPage():
return setKeyboardPage(_that);case Event_SetKeyShiftPage():
return setKeyShiftPage(_that);case Event_ToggleSync():
return toggleSync(_that);case Event_SetMasterDeck():
return setMasterDeck(_that);case Event_SetCuePoint():
return setCuePoint(_that);case Event_BeginCueHold():
return beginCueHold(_that);case Event_EndCueHold():
return endCueHold(_that);case Event_SetQuantize():
return setQuantize(_that);case Event_ToggleQuantize():
return toggleQuantize(_that);case Event_SetSlip():
return setSlip(_that);case Event_ToggleSlip():
return toggleSlip(_that);case Event_SetAutoLoop():
return setAutoLoop(_that);case Event_LoopIn():
return loopIn(_that);case Event_LoopOut():
return loopOut(_that);case Event_ExitLoop():
return exitLoop(_that);case Event_RecallSavedLoop():
return recallSavedLoop(_that);case Event_BeginLoopRoll():
return beginLoopRoll(_that);case Event_EndLoopRoll():
return endLoopRoll(_that);case Event_BeatJump():
return beatJump(_that);case Event_SetPadMode():
return setPadMode(_that);case Event_PadPress():
return padPress(_that);case Event_PadRelease():
return padRelease(_that);case Event_HotCuePadPress():
return hotCuePadPress(_that);case Event_HotCuePadRelease():
return hotCuePadRelease(_that);case Event_LoopRollPadPress():
return loopRollPadPress(_that);case Event_LoopRollPadRelease():
return loopRollPadRelease(_that);case Event_BeatJumpPadPress():
return beatJumpPadPress(_that);case Event_BeatJumpPadRelease():
return beatJumpPadRelease(_that);case Event_SamplerPadPress():
return samplerPadPress(_that);case Event_SamplerPadRelease():
return samplerPadRelease(_that);case Event_KeyboardPadPress():
return keyboardPadPress(_that);case Event_KeyboardPadRelease():
return keyboardPadRelease(_that);case Event_KeyShiftPadPress():
return keyShiftPadPress(_that);case Event_KeyShiftPadRelease():
return keyShiftPadRelease(_that);case Event_TriggerHotCue():
return triggerHotCue(_that);case Event_SaveDeckHotCue():
return saveDeckHotCue(_that);case Event_DeleteDeckHotCue():
return deleteDeckHotCue(_that);case Event_AssignSampler():
return assignSampler(_that);case Event_AssignSamplerTrack():
return assignSamplerTrack(_that);case Event_ClearSampler():
return clearSampler(_that);case Event_JogTouch():
return jogTouch(_that);case Event_JogTurn():
return jogTurn(_that);case Event_SetJogMode():
return setJogMode(_that);case Event_SetCrossfader():
return setCrossfader(_that);case Event_SetMasterVolume():
return setMasterVolume(_that);case Event_SetCueVolume():
return setCueVolume(_that);case Event_SetCueMix():
return setCueMix(_that);case Event_SetMasterCue():
return setMasterCue(_that);case Event_ToggleMasterCue():
return toggleMasterCue(_that);case Event_SetFxUnitSlot():
return setFxUnitSlot(_that);case Event_SetFxUnitSlotEnabled():
return setFxUnitSlotEnabled(_that);case Event_SetFxUnitWet():
return setFxUnitWet(_that);case Event_SetFxUnitParam():
return setFxUnitParam(_that);case Event_SetDeckFxRouted():
return setDeckFxRouted(_that);case Event_DeckUpdated():
return deckUpdated(_that);case Event_DeckPosition():
return deckPosition(_that);case Event_DeckLevels():
return deckLevels(_that);case Event_EngineSnapshot():
return engineSnapshot(_that);case Event_Error():
return error(_that);case Event_Notice():
return notice(_that);case Event_AnalyzeTrack():
return analyzeTrack(_that);case Event_GenerateStems():
return generateStems(_that);case Event_RefreshTrack():
return refreshTrack(_that);case Event_SaveBeatGrid():
return saveBeatGrid(_that);case Event_SaveTrackHotCue():
return saveTrackHotCue(_that);case Event_DeleteTrackHotCue():
return deleteTrackHotCue(_that);case Event_SaveTrackLoop():
return saveTrackLoop(_that);case Event_DeleteTrackLoop():
return deleteTrackLoop(_that);case Event_TrackAnalyzed():
return trackAnalyzed(_that);case Event_TrackUpdated():
return trackUpdated(_that);case Event_TrackProgress():
return trackProgress(_that);case Event_HotCuesChanged():
return hotCuesChanged(_that);case Event_LoopsChanged():
return loopsChanged(_that);case Event_BeatGridChanged():
return beatGridChanged(_that);case Event_HistorySessionUpdated():
return historySessionUpdated(_that);case Event_Navigate():
return navigate(_that);case Event_Focus():
return focus(_that);case Event_Load():
return load(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Event_Play value)?  play,TResult? Function( Event_Pause value)?  pause,TResult? Function( Event_TogglePlay value)?  togglePlay,TResult? Function( Event_Seek value)?  seek,TResult? Function( Event_Unload value)?  unload,TResult? Function( Event_SetVolume value)?  setVolume,TResult? Function( Event_SetEq value)?  setEq,TResult? Function( Event_SetEqBand value)?  setEqBand,TResult? Function( Event_SetFilter value)?  setFilter,TResult? Function( Event_SetGainTrim value)?  setGainTrim,TResult? Function( Event_SetHeadphoneCue value)?  setHeadphoneCue,TResult? Function( Event_ToggleHeadphoneCue value)?  toggleHeadphoneCue,TResult? Function( Event_SetCrossfaderAssign value)?  setCrossfaderAssign,TResult? Function( Event_SetSpeed value)?  setSpeed,TResult? Function( Event_SetTempoRange value)?  setTempoRange,TResult? Function( Event_SetKeyLock value)?  setKeyLock,TResult? Function( Event_SetKeyShift value)?  setKeyShift,TResult? Function( Event_SetKeyboardRoot value)?  setKeyboardRoot,TResult? Function( Event_SetKeyboardPage value)?  setKeyboardPage,TResult? Function( Event_SetKeyShiftPage value)?  setKeyShiftPage,TResult? Function( Event_ToggleSync value)?  toggleSync,TResult? Function( Event_SetMasterDeck value)?  setMasterDeck,TResult? Function( Event_SetCuePoint value)?  setCuePoint,TResult? Function( Event_BeginCueHold value)?  beginCueHold,TResult? Function( Event_EndCueHold value)?  endCueHold,TResult? Function( Event_SetQuantize value)?  setQuantize,TResult? Function( Event_ToggleQuantize value)?  toggleQuantize,TResult? Function( Event_SetSlip value)?  setSlip,TResult? Function( Event_ToggleSlip value)?  toggleSlip,TResult? Function( Event_SetAutoLoop value)?  setAutoLoop,TResult? Function( Event_LoopIn value)?  loopIn,TResult? Function( Event_LoopOut value)?  loopOut,TResult? Function( Event_ExitLoop value)?  exitLoop,TResult? Function( Event_RecallSavedLoop value)?  recallSavedLoop,TResult? Function( Event_BeginLoopRoll value)?  beginLoopRoll,TResult? Function( Event_EndLoopRoll value)?  endLoopRoll,TResult? Function( Event_BeatJump value)?  beatJump,TResult? Function( Event_SetPadMode value)?  setPadMode,TResult? Function( Event_PadPress value)?  padPress,TResult? Function( Event_PadRelease value)?  padRelease,TResult? Function( Event_HotCuePadPress value)?  hotCuePadPress,TResult? Function( Event_HotCuePadRelease value)?  hotCuePadRelease,TResult? Function( Event_LoopRollPadPress value)?  loopRollPadPress,TResult? Function( Event_LoopRollPadRelease value)?  loopRollPadRelease,TResult? Function( Event_BeatJumpPadPress value)?  beatJumpPadPress,TResult? Function( Event_BeatJumpPadRelease value)?  beatJumpPadRelease,TResult? Function( Event_SamplerPadPress value)?  samplerPadPress,TResult? Function( Event_SamplerPadRelease value)?  samplerPadRelease,TResult? Function( Event_KeyboardPadPress value)?  keyboardPadPress,TResult? Function( Event_KeyboardPadRelease value)?  keyboardPadRelease,TResult? Function( Event_KeyShiftPadPress value)?  keyShiftPadPress,TResult? Function( Event_KeyShiftPadRelease value)?  keyShiftPadRelease,TResult? Function( Event_TriggerHotCue value)?  triggerHotCue,TResult? Function( Event_SaveDeckHotCue value)?  saveDeckHotCue,TResult? Function( Event_DeleteDeckHotCue value)?  deleteDeckHotCue,TResult? Function( Event_AssignSampler value)?  assignSampler,TResult? Function( Event_AssignSamplerTrack value)?  assignSamplerTrack,TResult? Function( Event_ClearSampler value)?  clearSampler,TResult? Function( Event_JogTouch value)?  jogTouch,TResult? Function( Event_JogTurn value)?  jogTurn,TResult? Function( Event_SetJogMode value)?  setJogMode,TResult? Function( Event_SetCrossfader value)?  setCrossfader,TResult? Function( Event_SetMasterVolume value)?  setMasterVolume,TResult? Function( Event_SetCueVolume value)?  setCueVolume,TResult? Function( Event_SetCueMix value)?  setCueMix,TResult? Function( Event_SetMasterCue value)?  setMasterCue,TResult? Function( Event_ToggleMasterCue value)?  toggleMasterCue,TResult? Function( Event_SetFxUnitSlot value)?  setFxUnitSlot,TResult? Function( Event_SetFxUnitSlotEnabled value)?  setFxUnitSlotEnabled,TResult? Function( Event_SetFxUnitWet value)?  setFxUnitWet,TResult? Function( Event_SetFxUnitParam value)?  setFxUnitParam,TResult? Function( Event_SetDeckFxRouted value)?  setDeckFxRouted,TResult? Function( Event_DeckUpdated value)?  deckUpdated,TResult? Function( Event_DeckPosition value)?  deckPosition,TResult? Function( Event_DeckLevels value)?  deckLevels,TResult? Function( Event_EngineSnapshot value)?  engineSnapshot,TResult? Function( Event_Error value)?  error,TResult? Function( Event_Notice value)?  notice,TResult? Function( Event_AnalyzeTrack value)?  analyzeTrack,TResult? Function( Event_GenerateStems value)?  generateStems,TResult? Function( Event_RefreshTrack value)?  refreshTrack,TResult? Function( Event_SaveBeatGrid value)?  saveBeatGrid,TResult? Function( Event_SaveTrackHotCue value)?  saveTrackHotCue,TResult? Function( Event_DeleteTrackHotCue value)?  deleteTrackHotCue,TResult? Function( Event_SaveTrackLoop value)?  saveTrackLoop,TResult? Function( Event_DeleteTrackLoop value)?  deleteTrackLoop,TResult? Function( Event_TrackAnalyzed value)?  trackAnalyzed,TResult? Function( Event_TrackUpdated value)?  trackUpdated,TResult? Function( Event_TrackProgress value)?  trackProgress,TResult? Function( Event_HotCuesChanged value)?  hotCuesChanged,TResult? Function( Event_LoopsChanged value)?  loopsChanged,TResult? Function( Event_BeatGridChanged value)?  beatGridChanged,TResult? Function( Event_HistorySessionUpdated value)?  historySessionUpdated,TResult? Function( Event_Navigate value)?  navigate,TResult? Function( Event_Focus value)?  focus,TResult? Function( Event_Load value)?  load,}){
final _that = this;
switch (_that) {
case Event_Play() when play != null:
return play(_that);case Event_Pause() when pause != null:
return pause(_that);case Event_TogglePlay() when togglePlay != null:
return togglePlay(_that);case Event_Seek() when seek != null:
return seek(_that);case Event_Unload() when unload != null:
return unload(_that);case Event_SetVolume() when setVolume != null:
return setVolume(_that);case Event_SetEq() when setEq != null:
return setEq(_that);case Event_SetEqBand() when setEqBand != null:
return setEqBand(_that);case Event_SetFilter() when setFilter != null:
return setFilter(_that);case Event_SetGainTrim() when setGainTrim != null:
return setGainTrim(_that);case Event_SetHeadphoneCue() when setHeadphoneCue != null:
return setHeadphoneCue(_that);case Event_ToggleHeadphoneCue() when toggleHeadphoneCue != null:
return toggleHeadphoneCue(_that);case Event_SetCrossfaderAssign() when setCrossfaderAssign != null:
return setCrossfaderAssign(_that);case Event_SetSpeed() when setSpeed != null:
return setSpeed(_that);case Event_SetTempoRange() when setTempoRange != null:
return setTempoRange(_that);case Event_SetKeyLock() when setKeyLock != null:
return setKeyLock(_that);case Event_SetKeyShift() when setKeyShift != null:
return setKeyShift(_that);case Event_SetKeyboardRoot() when setKeyboardRoot != null:
return setKeyboardRoot(_that);case Event_SetKeyboardPage() when setKeyboardPage != null:
return setKeyboardPage(_that);case Event_SetKeyShiftPage() when setKeyShiftPage != null:
return setKeyShiftPage(_that);case Event_ToggleSync() when toggleSync != null:
return toggleSync(_that);case Event_SetMasterDeck() when setMasterDeck != null:
return setMasterDeck(_that);case Event_SetCuePoint() when setCuePoint != null:
return setCuePoint(_that);case Event_BeginCueHold() when beginCueHold != null:
return beginCueHold(_that);case Event_EndCueHold() when endCueHold != null:
return endCueHold(_that);case Event_SetQuantize() when setQuantize != null:
return setQuantize(_that);case Event_ToggleQuantize() when toggleQuantize != null:
return toggleQuantize(_that);case Event_SetSlip() when setSlip != null:
return setSlip(_that);case Event_ToggleSlip() when toggleSlip != null:
return toggleSlip(_that);case Event_SetAutoLoop() when setAutoLoop != null:
return setAutoLoop(_that);case Event_LoopIn() when loopIn != null:
return loopIn(_that);case Event_LoopOut() when loopOut != null:
return loopOut(_that);case Event_ExitLoop() when exitLoop != null:
return exitLoop(_that);case Event_RecallSavedLoop() when recallSavedLoop != null:
return recallSavedLoop(_that);case Event_BeginLoopRoll() when beginLoopRoll != null:
return beginLoopRoll(_that);case Event_EndLoopRoll() when endLoopRoll != null:
return endLoopRoll(_that);case Event_BeatJump() when beatJump != null:
return beatJump(_that);case Event_SetPadMode() when setPadMode != null:
return setPadMode(_that);case Event_PadPress() when padPress != null:
return padPress(_that);case Event_PadRelease() when padRelease != null:
return padRelease(_that);case Event_HotCuePadPress() when hotCuePadPress != null:
return hotCuePadPress(_that);case Event_HotCuePadRelease() when hotCuePadRelease != null:
return hotCuePadRelease(_that);case Event_LoopRollPadPress() when loopRollPadPress != null:
return loopRollPadPress(_that);case Event_LoopRollPadRelease() when loopRollPadRelease != null:
return loopRollPadRelease(_that);case Event_BeatJumpPadPress() when beatJumpPadPress != null:
return beatJumpPadPress(_that);case Event_BeatJumpPadRelease() when beatJumpPadRelease != null:
return beatJumpPadRelease(_that);case Event_SamplerPadPress() when samplerPadPress != null:
return samplerPadPress(_that);case Event_SamplerPadRelease() when samplerPadRelease != null:
return samplerPadRelease(_that);case Event_KeyboardPadPress() when keyboardPadPress != null:
return keyboardPadPress(_that);case Event_KeyboardPadRelease() when keyboardPadRelease != null:
return keyboardPadRelease(_that);case Event_KeyShiftPadPress() when keyShiftPadPress != null:
return keyShiftPadPress(_that);case Event_KeyShiftPadRelease() when keyShiftPadRelease != null:
return keyShiftPadRelease(_that);case Event_TriggerHotCue() when triggerHotCue != null:
return triggerHotCue(_that);case Event_SaveDeckHotCue() when saveDeckHotCue != null:
return saveDeckHotCue(_that);case Event_DeleteDeckHotCue() when deleteDeckHotCue != null:
return deleteDeckHotCue(_that);case Event_AssignSampler() when assignSampler != null:
return assignSampler(_that);case Event_AssignSamplerTrack() when assignSamplerTrack != null:
return assignSamplerTrack(_that);case Event_ClearSampler() when clearSampler != null:
return clearSampler(_that);case Event_JogTouch() when jogTouch != null:
return jogTouch(_that);case Event_JogTurn() when jogTurn != null:
return jogTurn(_that);case Event_SetJogMode() when setJogMode != null:
return setJogMode(_that);case Event_SetCrossfader() when setCrossfader != null:
return setCrossfader(_that);case Event_SetMasterVolume() when setMasterVolume != null:
return setMasterVolume(_that);case Event_SetCueVolume() when setCueVolume != null:
return setCueVolume(_that);case Event_SetCueMix() when setCueMix != null:
return setCueMix(_that);case Event_SetMasterCue() when setMasterCue != null:
return setMasterCue(_that);case Event_ToggleMasterCue() when toggleMasterCue != null:
return toggleMasterCue(_that);case Event_SetFxUnitSlot() when setFxUnitSlot != null:
return setFxUnitSlot(_that);case Event_SetFxUnitSlotEnabled() when setFxUnitSlotEnabled != null:
return setFxUnitSlotEnabled(_that);case Event_SetFxUnitWet() when setFxUnitWet != null:
return setFxUnitWet(_that);case Event_SetFxUnitParam() when setFxUnitParam != null:
return setFxUnitParam(_that);case Event_SetDeckFxRouted() when setDeckFxRouted != null:
return setDeckFxRouted(_that);case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that);case Event_Error() when error != null:
return error(_that);case Event_Notice() when notice != null:
return notice(_that);case Event_AnalyzeTrack() when analyzeTrack != null:
return analyzeTrack(_that);case Event_GenerateStems() when generateStems != null:
return generateStems(_that);case Event_RefreshTrack() when refreshTrack != null:
return refreshTrack(_that);case Event_SaveBeatGrid() when saveBeatGrid != null:
return saveBeatGrid(_that);case Event_SaveTrackHotCue() when saveTrackHotCue != null:
return saveTrackHotCue(_that);case Event_DeleteTrackHotCue() when deleteTrackHotCue != null:
return deleteTrackHotCue(_that);case Event_SaveTrackLoop() when saveTrackLoop != null:
return saveTrackLoop(_that);case Event_DeleteTrackLoop() when deleteTrackLoop != null:
return deleteTrackLoop(_that);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that);case Event_Navigate() when navigate != null:
return navigate(_that);case Event_Focus() when focus != null:
return focus(_that);case Event_Load() when load != null:
return load(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( Play field0)?  play,TResult Function( Pause field0)?  pause,TResult Function( TogglePlay field0)?  togglePlay,TResult Function( Seek field0)?  seek,TResult Function( Unload field0)?  unload,TResult Function( SetVolume field0)?  setVolume,TResult Function( SetEq field0)?  setEq,TResult Function( SetEqBand field0)?  setEqBand,TResult Function( SetFilter field0)?  setFilter,TResult Function( SetGainTrim field0)?  setGainTrim,TResult Function( SetHeadphoneCue field0)?  setHeadphoneCue,TResult Function( ToggleHeadphoneCue field0)?  toggleHeadphoneCue,TResult Function( SetCrossfaderAssign field0)?  setCrossfaderAssign,TResult Function( SetSpeed field0)?  setSpeed,TResult Function( SetTempoRange field0)?  setTempoRange,TResult Function( SetKeyLock field0)?  setKeyLock,TResult Function( SetKeyShift field0)?  setKeyShift,TResult Function( SetKeyboardRoot field0)?  setKeyboardRoot,TResult Function( SetKeyboardPage field0)?  setKeyboardPage,TResult Function( SetKeyShiftPage field0)?  setKeyShiftPage,TResult Function( ToggleSync field0)?  toggleSync,TResult Function( SetMasterDeck field0)?  setMasterDeck,TResult Function( SetCuePoint field0)?  setCuePoint,TResult Function( BeginCueHold field0)?  beginCueHold,TResult Function( EndCueHold field0)?  endCueHold,TResult Function( SetQuantize field0)?  setQuantize,TResult Function( ToggleQuantize field0)?  toggleQuantize,TResult Function( SetSlip field0)?  setSlip,TResult Function( ToggleSlip field0)?  toggleSlip,TResult Function( SetAutoLoop field0)?  setAutoLoop,TResult Function( LoopIn field0)?  loopIn,TResult Function( LoopOut field0)?  loopOut,TResult Function( ExitLoop field0)?  exitLoop,TResult Function( RecallSavedLoop field0)?  recallSavedLoop,TResult Function( BeginLoopRoll field0)?  beginLoopRoll,TResult Function( EndLoopRoll field0)?  endLoopRoll,TResult Function( BeatJump field0)?  beatJump,TResult Function( SetPadMode field0)?  setPadMode,TResult Function( PadPress field0)?  padPress,TResult Function( PadRelease field0)?  padRelease,TResult Function( HotCuePadPress field0)?  hotCuePadPress,TResult Function( HotCuePadRelease field0)?  hotCuePadRelease,TResult Function( LoopRollPadPress field0)?  loopRollPadPress,TResult Function( LoopRollPadRelease field0)?  loopRollPadRelease,TResult Function( BeatJumpPadPress field0)?  beatJumpPadPress,TResult Function( BeatJumpPadRelease field0)?  beatJumpPadRelease,TResult Function( SamplerPadPress field0)?  samplerPadPress,TResult Function( SamplerPadRelease field0)?  samplerPadRelease,TResult Function( KeyboardPadPress field0)?  keyboardPadPress,TResult Function( KeyboardPadRelease field0)?  keyboardPadRelease,TResult Function( KeyShiftPadPress field0)?  keyShiftPadPress,TResult Function( KeyShiftPadRelease field0)?  keyShiftPadRelease,TResult Function( TriggerHotCue field0)?  triggerHotCue,TResult Function( SaveDeckHotCue field0)?  saveDeckHotCue,TResult Function( DeleteDeckHotCue field0)?  deleteDeckHotCue,TResult Function( AssignSampler field0)?  assignSampler,TResult Function( AssignSamplerTrack field0)?  assignSamplerTrack,TResult Function( ClearSampler field0)?  clearSampler,TResult Function( JogTouch field0)?  jogTouch,TResult Function( JogTurn field0)?  jogTurn,TResult Function( SetJogMode field0)?  setJogMode,TResult Function( SetCrossfader field0)?  setCrossfader,TResult Function( SetMasterVolume field0)?  setMasterVolume,TResult Function( SetCueVolume field0)?  setCueVolume,TResult Function( SetCueMix field0)?  setCueMix,TResult Function( SetMasterCue field0)?  setMasterCue,TResult Function( ToggleMasterCue field0)?  toggleMasterCue,TResult Function( SetFxUnitSlot field0)?  setFxUnitSlot,TResult Function( SetFxUnitSlotEnabled field0)?  setFxUnitSlotEnabled,TResult Function( SetFxUnitWet field0)?  setFxUnitWet,TResult Function( SetFxUnitParam field0)?  setFxUnitParam,TResult Function( SetDeckFxRouted field0)?  setDeckFxRouted,TResult Function( DeckUpdated field0)?  deckUpdated,TResult Function( DeckPosition field0)?  deckPosition,TResult Function( DeckLevels field0)?  deckLevels,TResult Function( EngineSnapshot field0)?  engineSnapshot,TResult Function( Error field0)?  error,TResult Function( Notice field0)?  notice,TResult Function( AnalyzeTrack field0)?  analyzeTrack,TResult Function( GenerateStems field0)?  generateStems,TResult Function( RefreshTrack field0)?  refreshTrack,TResult Function( SaveBeatGrid field0)?  saveBeatGrid,TResult Function( SaveTrackHotCue field0)?  saveTrackHotCue,TResult Function( DeleteTrackHotCue field0)?  deleteTrackHotCue,TResult Function( SaveTrackLoop field0)?  saveTrackLoop,TResult Function( DeleteTrackLoop field0)?  deleteTrackLoop,TResult Function( TrackAnalyzed field0)?  trackAnalyzed,TResult Function( TrackUpdated field0)?  trackUpdated,TResult Function( TrackProgress field0)?  trackProgress,TResult Function( HotCuesChanged field0)?  hotCuesChanged,TResult Function( LoopsChanged field0)?  loopsChanged,TResult Function( BeatGridChanged field0)?  beatGridChanged,TResult Function( HistorySessionUpdated field0)?  historySessionUpdated,TResult Function( Navigate field0)?  navigate,TResult Function( Focus field0)?  focus,TResult Function( Load field0)?  load,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Event_Play() when play != null:
return play(_that.field0);case Event_Pause() when pause != null:
return pause(_that.field0);case Event_TogglePlay() when togglePlay != null:
return togglePlay(_that.field0);case Event_Seek() when seek != null:
return seek(_that.field0);case Event_Unload() when unload != null:
return unload(_that.field0);case Event_SetVolume() when setVolume != null:
return setVolume(_that.field0);case Event_SetEq() when setEq != null:
return setEq(_that.field0);case Event_SetEqBand() when setEqBand != null:
return setEqBand(_that.field0);case Event_SetFilter() when setFilter != null:
return setFilter(_that.field0);case Event_SetGainTrim() when setGainTrim != null:
return setGainTrim(_that.field0);case Event_SetHeadphoneCue() when setHeadphoneCue != null:
return setHeadphoneCue(_that.field0);case Event_ToggleHeadphoneCue() when toggleHeadphoneCue != null:
return toggleHeadphoneCue(_that.field0);case Event_SetCrossfaderAssign() when setCrossfaderAssign != null:
return setCrossfaderAssign(_that.field0);case Event_SetSpeed() when setSpeed != null:
return setSpeed(_that.field0);case Event_SetTempoRange() when setTempoRange != null:
return setTempoRange(_that.field0);case Event_SetKeyLock() when setKeyLock != null:
return setKeyLock(_that.field0);case Event_SetKeyShift() when setKeyShift != null:
return setKeyShift(_that.field0);case Event_SetKeyboardRoot() when setKeyboardRoot != null:
return setKeyboardRoot(_that.field0);case Event_SetKeyboardPage() when setKeyboardPage != null:
return setKeyboardPage(_that.field0);case Event_SetKeyShiftPage() when setKeyShiftPage != null:
return setKeyShiftPage(_that.field0);case Event_ToggleSync() when toggleSync != null:
return toggleSync(_that.field0);case Event_SetMasterDeck() when setMasterDeck != null:
return setMasterDeck(_that.field0);case Event_SetCuePoint() when setCuePoint != null:
return setCuePoint(_that.field0);case Event_BeginCueHold() when beginCueHold != null:
return beginCueHold(_that.field0);case Event_EndCueHold() when endCueHold != null:
return endCueHold(_that.field0);case Event_SetQuantize() when setQuantize != null:
return setQuantize(_that.field0);case Event_ToggleQuantize() when toggleQuantize != null:
return toggleQuantize(_that.field0);case Event_SetSlip() when setSlip != null:
return setSlip(_that.field0);case Event_ToggleSlip() when toggleSlip != null:
return toggleSlip(_that.field0);case Event_SetAutoLoop() when setAutoLoop != null:
return setAutoLoop(_that.field0);case Event_LoopIn() when loopIn != null:
return loopIn(_that.field0);case Event_LoopOut() when loopOut != null:
return loopOut(_that.field0);case Event_ExitLoop() when exitLoop != null:
return exitLoop(_that.field0);case Event_RecallSavedLoop() when recallSavedLoop != null:
return recallSavedLoop(_that.field0);case Event_BeginLoopRoll() when beginLoopRoll != null:
return beginLoopRoll(_that.field0);case Event_EndLoopRoll() when endLoopRoll != null:
return endLoopRoll(_that.field0);case Event_BeatJump() when beatJump != null:
return beatJump(_that.field0);case Event_SetPadMode() when setPadMode != null:
return setPadMode(_that.field0);case Event_PadPress() when padPress != null:
return padPress(_that.field0);case Event_PadRelease() when padRelease != null:
return padRelease(_that.field0);case Event_HotCuePadPress() when hotCuePadPress != null:
return hotCuePadPress(_that.field0);case Event_HotCuePadRelease() when hotCuePadRelease != null:
return hotCuePadRelease(_that.field0);case Event_LoopRollPadPress() when loopRollPadPress != null:
return loopRollPadPress(_that.field0);case Event_LoopRollPadRelease() when loopRollPadRelease != null:
return loopRollPadRelease(_that.field0);case Event_BeatJumpPadPress() when beatJumpPadPress != null:
return beatJumpPadPress(_that.field0);case Event_BeatJumpPadRelease() when beatJumpPadRelease != null:
return beatJumpPadRelease(_that.field0);case Event_SamplerPadPress() when samplerPadPress != null:
return samplerPadPress(_that.field0);case Event_SamplerPadRelease() when samplerPadRelease != null:
return samplerPadRelease(_that.field0);case Event_KeyboardPadPress() when keyboardPadPress != null:
return keyboardPadPress(_that.field0);case Event_KeyboardPadRelease() when keyboardPadRelease != null:
return keyboardPadRelease(_that.field0);case Event_KeyShiftPadPress() when keyShiftPadPress != null:
return keyShiftPadPress(_that.field0);case Event_KeyShiftPadRelease() when keyShiftPadRelease != null:
return keyShiftPadRelease(_that.field0);case Event_TriggerHotCue() when triggerHotCue != null:
return triggerHotCue(_that.field0);case Event_SaveDeckHotCue() when saveDeckHotCue != null:
return saveDeckHotCue(_that.field0);case Event_DeleteDeckHotCue() when deleteDeckHotCue != null:
return deleteDeckHotCue(_that.field0);case Event_AssignSampler() when assignSampler != null:
return assignSampler(_that.field0);case Event_AssignSamplerTrack() when assignSamplerTrack != null:
return assignSamplerTrack(_that.field0);case Event_ClearSampler() when clearSampler != null:
return clearSampler(_that.field0);case Event_JogTouch() when jogTouch != null:
return jogTouch(_that.field0);case Event_JogTurn() when jogTurn != null:
return jogTurn(_that.field0);case Event_SetJogMode() when setJogMode != null:
return setJogMode(_that.field0);case Event_SetCrossfader() when setCrossfader != null:
return setCrossfader(_that.field0);case Event_SetMasterVolume() when setMasterVolume != null:
return setMasterVolume(_that.field0);case Event_SetCueVolume() when setCueVolume != null:
return setCueVolume(_that.field0);case Event_SetCueMix() when setCueMix != null:
return setCueMix(_that.field0);case Event_SetMasterCue() when setMasterCue != null:
return setMasterCue(_that.field0);case Event_ToggleMasterCue() when toggleMasterCue != null:
return toggleMasterCue(_that.field0);case Event_SetFxUnitSlot() when setFxUnitSlot != null:
return setFxUnitSlot(_that.field0);case Event_SetFxUnitSlotEnabled() when setFxUnitSlotEnabled != null:
return setFxUnitSlotEnabled(_that.field0);case Event_SetFxUnitWet() when setFxUnitWet != null:
return setFxUnitWet(_that.field0);case Event_SetFxUnitParam() when setFxUnitParam != null:
return setFxUnitParam(_that.field0);case Event_SetDeckFxRouted() when setDeckFxRouted != null:
return setDeckFxRouted(_that.field0);case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that.field0);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that.field0);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that.field0);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that.field0);case Event_Error() when error != null:
return error(_that.field0);case Event_Notice() when notice != null:
return notice(_that.field0);case Event_AnalyzeTrack() when analyzeTrack != null:
return analyzeTrack(_that.field0);case Event_GenerateStems() when generateStems != null:
return generateStems(_that.field0);case Event_RefreshTrack() when refreshTrack != null:
return refreshTrack(_that.field0);case Event_SaveBeatGrid() when saveBeatGrid != null:
return saveBeatGrid(_that.field0);case Event_SaveTrackHotCue() when saveTrackHotCue != null:
return saveTrackHotCue(_that.field0);case Event_DeleteTrackHotCue() when deleteTrackHotCue != null:
return deleteTrackHotCue(_that.field0);case Event_SaveTrackLoop() when saveTrackLoop != null:
return saveTrackLoop(_that.field0);case Event_DeleteTrackLoop() when deleteTrackLoop != null:
return deleteTrackLoop(_that.field0);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that.field0);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that.field0);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that.field0);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that.field0);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that.field0);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that.field0);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that.field0);case Event_Navigate() when navigate != null:
return navigate(_that.field0);case Event_Focus() when focus != null:
return focus(_that.field0);case Event_Load() when load != null:
return load(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( Play field0)  play,required TResult Function( Pause field0)  pause,required TResult Function( TogglePlay field0)  togglePlay,required TResult Function( Seek field0)  seek,required TResult Function( Unload field0)  unload,required TResult Function( SetVolume field0)  setVolume,required TResult Function( SetEq field0)  setEq,required TResult Function( SetEqBand field0)  setEqBand,required TResult Function( SetFilter field0)  setFilter,required TResult Function( SetGainTrim field0)  setGainTrim,required TResult Function( SetHeadphoneCue field0)  setHeadphoneCue,required TResult Function( ToggleHeadphoneCue field0)  toggleHeadphoneCue,required TResult Function( SetCrossfaderAssign field0)  setCrossfaderAssign,required TResult Function( SetSpeed field0)  setSpeed,required TResult Function( SetTempoRange field0)  setTempoRange,required TResult Function( SetKeyLock field0)  setKeyLock,required TResult Function( SetKeyShift field0)  setKeyShift,required TResult Function( SetKeyboardRoot field0)  setKeyboardRoot,required TResult Function( SetKeyboardPage field0)  setKeyboardPage,required TResult Function( SetKeyShiftPage field0)  setKeyShiftPage,required TResult Function( ToggleSync field0)  toggleSync,required TResult Function( SetMasterDeck field0)  setMasterDeck,required TResult Function( SetCuePoint field0)  setCuePoint,required TResult Function( BeginCueHold field0)  beginCueHold,required TResult Function( EndCueHold field0)  endCueHold,required TResult Function( SetQuantize field0)  setQuantize,required TResult Function( ToggleQuantize field0)  toggleQuantize,required TResult Function( SetSlip field0)  setSlip,required TResult Function( ToggleSlip field0)  toggleSlip,required TResult Function( SetAutoLoop field0)  setAutoLoop,required TResult Function( LoopIn field0)  loopIn,required TResult Function( LoopOut field0)  loopOut,required TResult Function( ExitLoop field0)  exitLoop,required TResult Function( RecallSavedLoop field0)  recallSavedLoop,required TResult Function( BeginLoopRoll field0)  beginLoopRoll,required TResult Function( EndLoopRoll field0)  endLoopRoll,required TResult Function( BeatJump field0)  beatJump,required TResult Function( SetPadMode field0)  setPadMode,required TResult Function( PadPress field0)  padPress,required TResult Function( PadRelease field0)  padRelease,required TResult Function( HotCuePadPress field0)  hotCuePadPress,required TResult Function( HotCuePadRelease field0)  hotCuePadRelease,required TResult Function( LoopRollPadPress field0)  loopRollPadPress,required TResult Function( LoopRollPadRelease field0)  loopRollPadRelease,required TResult Function( BeatJumpPadPress field0)  beatJumpPadPress,required TResult Function( BeatJumpPadRelease field0)  beatJumpPadRelease,required TResult Function( SamplerPadPress field0)  samplerPadPress,required TResult Function( SamplerPadRelease field0)  samplerPadRelease,required TResult Function( KeyboardPadPress field0)  keyboardPadPress,required TResult Function( KeyboardPadRelease field0)  keyboardPadRelease,required TResult Function( KeyShiftPadPress field0)  keyShiftPadPress,required TResult Function( KeyShiftPadRelease field0)  keyShiftPadRelease,required TResult Function( TriggerHotCue field0)  triggerHotCue,required TResult Function( SaveDeckHotCue field0)  saveDeckHotCue,required TResult Function( DeleteDeckHotCue field0)  deleteDeckHotCue,required TResult Function( AssignSampler field0)  assignSampler,required TResult Function( AssignSamplerTrack field0)  assignSamplerTrack,required TResult Function( ClearSampler field0)  clearSampler,required TResult Function( JogTouch field0)  jogTouch,required TResult Function( JogTurn field0)  jogTurn,required TResult Function( SetJogMode field0)  setJogMode,required TResult Function( SetCrossfader field0)  setCrossfader,required TResult Function( SetMasterVolume field0)  setMasterVolume,required TResult Function( SetCueVolume field0)  setCueVolume,required TResult Function( SetCueMix field0)  setCueMix,required TResult Function( SetMasterCue field0)  setMasterCue,required TResult Function( ToggleMasterCue field0)  toggleMasterCue,required TResult Function( SetFxUnitSlot field0)  setFxUnitSlot,required TResult Function( SetFxUnitSlotEnabled field0)  setFxUnitSlotEnabled,required TResult Function( SetFxUnitWet field0)  setFxUnitWet,required TResult Function( SetFxUnitParam field0)  setFxUnitParam,required TResult Function( SetDeckFxRouted field0)  setDeckFxRouted,required TResult Function( DeckUpdated field0)  deckUpdated,required TResult Function( DeckPosition field0)  deckPosition,required TResult Function( DeckLevels field0)  deckLevels,required TResult Function( EngineSnapshot field0)  engineSnapshot,required TResult Function( Error field0)  error,required TResult Function( Notice field0)  notice,required TResult Function( AnalyzeTrack field0)  analyzeTrack,required TResult Function( GenerateStems field0)  generateStems,required TResult Function( RefreshTrack field0)  refreshTrack,required TResult Function( SaveBeatGrid field0)  saveBeatGrid,required TResult Function( SaveTrackHotCue field0)  saveTrackHotCue,required TResult Function( DeleteTrackHotCue field0)  deleteTrackHotCue,required TResult Function( SaveTrackLoop field0)  saveTrackLoop,required TResult Function( DeleteTrackLoop field0)  deleteTrackLoop,required TResult Function( TrackAnalyzed field0)  trackAnalyzed,required TResult Function( TrackUpdated field0)  trackUpdated,required TResult Function( TrackProgress field0)  trackProgress,required TResult Function( HotCuesChanged field0)  hotCuesChanged,required TResult Function( LoopsChanged field0)  loopsChanged,required TResult Function( BeatGridChanged field0)  beatGridChanged,required TResult Function( HistorySessionUpdated field0)  historySessionUpdated,required TResult Function( Navigate field0)  navigate,required TResult Function( Focus field0)  focus,required TResult Function( Load field0)  load,}) {final _that = this;
switch (_that) {
case Event_Play():
return play(_that.field0);case Event_Pause():
return pause(_that.field0);case Event_TogglePlay():
return togglePlay(_that.field0);case Event_Seek():
return seek(_that.field0);case Event_Unload():
return unload(_that.field0);case Event_SetVolume():
return setVolume(_that.field0);case Event_SetEq():
return setEq(_that.field0);case Event_SetEqBand():
return setEqBand(_that.field0);case Event_SetFilter():
return setFilter(_that.field0);case Event_SetGainTrim():
return setGainTrim(_that.field0);case Event_SetHeadphoneCue():
return setHeadphoneCue(_that.field0);case Event_ToggleHeadphoneCue():
return toggleHeadphoneCue(_that.field0);case Event_SetCrossfaderAssign():
return setCrossfaderAssign(_that.field0);case Event_SetSpeed():
return setSpeed(_that.field0);case Event_SetTempoRange():
return setTempoRange(_that.field0);case Event_SetKeyLock():
return setKeyLock(_that.field0);case Event_SetKeyShift():
return setKeyShift(_that.field0);case Event_SetKeyboardRoot():
return setKeyboardRoot(_that.field0);case Event_SetKeyboardPage():
return setKeyboardPage(_that.field0);case Event_SetKeyShiftPage():
return setKeyShiftPage(_that.field0);case Event_ToggleSync():
return toggleSync(_that.field0);case Event_SetMasterDeck():
return setMasterDeck(_that.field0);case Event_SetCuePoint():
return setCuePoint(_that.field0);case Event_BeginCueHold():
return beginCueHold(_that.field0);case Event_EndCueHold():
return endCueHold(_that.field0);case Event_SetQuantize():
return setQuantize(_that.field0);case Event_ToggleQuantize():
return toggleQuantize(_that.field0);case Event_SetSlip():
return setSlip(_that.field0);case Event_ToggleSlip():
return toggleSlip(_that.field0);case Event_SetAutoLoop():
return setAutoLoop(_that.field0);case Event_LoopIn():
return loopIn(_that.field0);case Event_LoopOut():
return loopOut(_that.field0);case Event_ExitLoop():
return exitLoop(_that.field0);case Event_RecallSavedLoop():
return recallSavedLoop(_that.field0);case Event_BeginLoopRoll():
return beginLoopRoll(_that.field0);case Event_EndLoopRoll():
return endLoopRoll(_that.field0);case Event_BeatJump():
return beatJump(_that.field0);case Event_SetPadMode():
return setPadMode(_that.field0);case Event_PadPress():
return padPress(_that.field0);case Event_PadRelease():
return padRelease(_that.field0);case Event_HotCuePadPress():
return hotCuePadPress(_that.field0);case Event_HotCuePadRelease():
return hotCuePadRelease(_that.field0);case Event_LoopRollPadPress():
return loopRollPadPress(_that.field0);case Event_LoopRollPadRelease():
return loopRollPadRelease(_that.field0);case Event_BeatJumpPadPress():
return beatJumpPadPress(_that.field0);case Event_BeatJumpPadRelease():
return beatJumpPadRelease(_that.field0);case Event_SamplerPadPress():
return samplerPadPress(_that.field0);case Event_SamplerPadRelease():
return samplerPadRelease(_that.field0);case Event_KeyboardPadPress():
return keyboardPadPress(_that.field0);case Event_KeyboardPadRelease():
return keyboardPadRelease(_that.field0);case Event_KeyShiftPadPress():
return keyShiftPadPress(_that.field0);case Event_KeyShiftPadRelease():
return keyShiftPadRelease(_that.field0);case Event_TriggerHotCue():
return triggerHotCue(_that.field0);case Event_SaveDeckHotCue():
return saveDeckHotCue(_that.field0);case Event_DeleteDeckHotCue():
return deleteDeckHotCue(_that.field0);case Event_AssignSampler():
return assignSampler(_that.field0);case Event_AssignSamplerTrack():
return assignSamplerTrack(_that.field0);case Event_ClearSampler():
return clearSampler(_that.field0);case Event_JogTouch():
return jogTouch(_that.field0);case Event_JogTurn():
return jogTurn(_that.field0);case Event_SetJogMode():
return setJogMode(_that.field0);case Event_SetCrossfader():
return setCrossfader(_that.field0);case Event_SetMasterVolume():
return setMasterVolume(_that.field0);case Event_SetCueVolume():
return setCueVolume(_that.field0);case Event_SetCueMix():
return setCueMix(_that.field0);case Event_SetMasterCue():
return setMasterCue(_that.field0);case Event_ToggleMasterCue():
return toggleMasterCue(_that.field0);case Event_SetFxUnitSlot():
return setFxUnitSlot(_that.field0);case Event_SetFxUnitSlotEnabled():
return setFxUnitSlotEnabled(_that.field0);case Event_SetFxUnitWet():
return setFxUnitWet(_that.field0);case Event_SetFxUnitParam():
return setFxUnitParam(_that.field0);case Event_SetDeckFxRouted():
return setDeckFxRouted(_that.field0);case Event_DeckUpdated():
return deckUpdated(_that.field0);case Event_DeckPosition():
return deckPosition(_that.field0);case Event_DeckLevels():
return deckLevels(_that.field0);case Event_EngineSnapshot():
return engineSnapshot(_that.field0);case Event_Error():
return error(_that.field0);case Event_Notice():
return notice(_that.field0);case Event_AnalyzeTrack():
return analyzeTrack(_that.field0);case Event_GenerateStems():
return generateStems(_that.field0);case Event_RefreshTrack():
return refreshTrack(_that.field0);case Event_SaveBeatGrid():
return saveBeatGrid(_that.field0);case Event_SaveTrackHotCue():
return saveTrackHotCue(_that.field0);case Event_DeleteTrackHotCue():
return deleteTrackHotCue(_that.field0);case Event_SaveTrackLoop():
return saveTrackLoop(_that.field0);case Event_DeleteTrackLoop():
return deleteTrackLoop(_that.field0);case Event_TrackAnalyzed():
return trackAnalyzed(_that.field0);case Event_TrackUpdated():
return trackUpdated(_that.field0);case Event_TrackProgress():
return trackProgress(_that.field0);case Event_HotCuesChanged():
return hotCuesChanged(_that.field0);case Event_LoopsChanged():
return loopsChanged(_that.field0);case Event_BeatGridChanged():
return beatGridChanged(_that.field0);case Event_HistorySessionUpdated():
return historySessionUpdated(_that.field0);case Event_Navigate():
return navigate(_that.field0);case Event_Focus():
return focus(_that.field0);case Event_Load():
return load(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( Play field0)?  play,TResult? Function( Pause field0)?  pause,TResult? Function( TogglePlay field0)?  togglePlay,TResult? Function( Seek field0)?  seek,TResult? Function( Unload field0)?  unload,TResult? Function( SetVolume field0)?  setVolume,TResult? Function( SetEq field0)?  setEq,TResult? Function( SetEqBand field0)?  setEqBand,TResult? Function( SetFilter field0)?  setFilter,TResult? Function( SetGainTrim field0)?  setGainTrim,TResult? Function( SetHeadphoneCue field0)?  setHeadphoneCue,TResult? Function( ToggleHeadphoneCue field0)?  toggleHeadphoneCue,TResult? Function( SetCrossfaderAssign field0)?  setCrossfaderAssign,TResult? Function( SetSpeed field0)?  setSpeed,TResult? Function( SetTempoRange field0)?  setTempoRange,TResult? Function( SetKeyLock field0)?  setKeyLock,TResult? Function( SetKeyShift field0)?  setKeyShift,TResult? Function( SetKeyboardRoot field0)?  setKeyboardRoot,TResult? Function( SetKeyboardPage field0)?  setKeyboardPage,TResult? Function( SetKeyShiftPage field0)?  setKeyShiftPage,TResult? Function( ToggleSync field0)?  toggleSync,TResult? Function( SetMasterDeck field0)?  setMasterDeck,TResult? Function( SetCuePoint field0)?  setCuePoint,TResult? Function( BeginCueHold field0)?  beginCueHold,TResult? Function( EndCueHold field0)?  endCueHold,TResult? Function( SetQuantize field0)?  setQuantize,TResult? Function( ToggleQuantize field0)?  toggleQuantize,TResult? Function( SetSlip field0)?  setSlip,TResult? Function( ToggleSlip field0)?  toggleSlip,TResult? Function( SetAutoLoop field0)?  setAutoLoop,TResult? Function( LoopIn field0)?  loopIn,TResult? Function( LoopOut field0)?  loopOut,TResult? Function( ExitLoop field0)?  exitLoop,TResult? Function( RecallSavedLoop field0)?  recallSavedLoop,TResult? Function( BeginLoopRoll field0)?  beginLoopRoll,TResult? Function( EndLoopRoll field0)?  endLoopRoll,TResult? Function( BeatJump field0)?  beatJump,TResult? Function( SetPadMode field0)?  setPadMode,TResult? Function( PadPress field0)?  padPress,TResult? Function( PadRelease field0)?  padRelease,TResult? Function( HotCuePadPress field0)?  hotCuePadPress,TResult? Function( HotCuePadRelease field0)?  hotCuePadRelease,TResult? Function( LoopRollPadPress field0)?  loopRollPadPress,TResult? Function( LoopRollPadRelease field0)?  loopRollPadRelease,TResult? Function( BeatJumpPadPress field0)?  beatJumpPadPress,TResult? Function( BeatJumpPadRelease field0)?  beatJumpPadRelease,TResult? Function( SamplerPadPress field0)?  samplerPadPress,TResult? Function( SamplerPadRelease field0)?  samplerPadRelease,TResult? Function( KeyboardPadPress field0)?  keyboardPadPress,TResult? Function( KeyboardPadRelease field0)?  keyboardPadRelease,TResult? Function( KeyShiftPadPress field0)?  keyShiftPadPress,TResult? Function( KeyShiftPadRelease field0)?  keyShiftPadRelease,TResult? Function( TriggerHotCue field0)?  triggerHotCue,TResult? Function( SaveDeckHotCue field0)?  saveDeckHotCue,TResult? Function( DeleteDeckHotCue field0)?  deleteDeckHotCue,TResult? Function( AssignSampler field0)?  assignSampler,TResult? Function( AssignSamplerTrack field0)?  assignSamplerTrack,TResult? Function( ClearSampler field0)?  clearSampler,TResult? Function( JogTouch field0)?  jogTouch,TResult? Function( JogTurn field0)?  jogTurn,TResult? Function( SetJogMode field0)?  setJogMode,TResult? Function( SetCrossfader field0)?  setCrossfader,TResult? Function( SetMasterVolume field0)?  setMasterVolume,TResult? Function( SetCueVolume field0)?  setCueVolume,TResult? Function( SetCueMix field0)?  setCueMix,TResult? Function( SetMasterCue field0)?  setMasterCue,TResult? Function( ToggleMasterCue field0)?  toggleMasterCue,TResult? Function( SetFxUnitSlot field0)?  setFxUnitSlot,TResult? Function( SetFxUnitSlotEnabled field0)?  setFxUnitSlotEnabled,TResult? Function( SetFxUnitWet field0)?  setFxUnitWet,TResult? Function( SetFxUnitParam field0)?  setFxUnitParam,TResult? Function( SetDeckFxRouted field0)?  setDeckFxRouted,TResult? Function( DeckUpdated field0)?  deckUpdated,TResult? Function( DeckPosition field0)?  deckPosition,TResult? Function( DeckLevels field0)?  deckLevels,TResult? Function( EngineSnapshot field0)?  engineSnapshot,TResult? Function( Error field0)?  error,TResult? Function( Notice field0)?  notice,TResult? Function( AnalyzeTrack field0)?  analyzeTrack,TResult? Function( GenerateStems field0)?  generateStems,TResult? Function( RefreshTrack field0)?  refreshTrack,TResult? Function( SaveBeatGrid field0)?  saveBeatGrid,TResult? Function( SaveTrackHotCue field0)?  saveTrackHotCue,TResult? Function( DeleteTrackHotCue field0)?  deleteTrackHotCue,TResult? Function( SaveTrackLoop field0)?  saveTrackLoop,TResult? Function( DeleteTrackLoop field0)?  deleteTrackLoop,TResult? Function( TrackAnalyzed field0)?  trackAnalyzed,TResult? Function( TrackUpdated field0)?  trackUpdated,TResult? Function( TrackProgress field0)?  trackProgress,TResult? Function( HotCuesChanged field0)?  hotCuesChanged,TResult? Function( LoopsChanged field0)?  loopsChanged,TResult? Function( BeatGridChanged field0)?  beatGridChanged,TResult? Function( HistorySessionUpdated field0)?  historySessionUpdated,TResult? Function( Navigate field0)?  navigate,TResult? Function( Focus field0)?  focus,TResult? Function( Load field0)?  load,}) {final _that = this;
switch (_that) {
case Event_Play() when play != null:
return play(_that.field0);case Event_Pause() when pause != null:
return pause(_that.field0);case Event_TogglePlay() when togglePlay != null:
return togglePlay(_that.field0);case Event_Seek() when seek != null:
return seek(_that.field0);case Event_Unload() when unload != null:
return unload(_that.field0);case Event_SetVolume() when setVolume != null:
return setVolume(_that.field0);case Event_SetEq() when setEq != null:
return setEq(_that.field0);case Event_SetEqBand() when setEqBand != null:
return setEqBand(_that.field0);case Event_SetFilter() when setFilter != null:
return setFilter(_that.field0);case Event_SetGainTrim() when setGainTrim != null:
return setGainTrim(_that.field0);case Event_SetHeadphoneCue() when setHeadphoneCue != null:
return setHeadphoneCue(_that.field0);case Event_ToggleHeadphoneCue() when toggleHeadphoneCue != null:
return toggleHeadphoneCue(_that.field0);case Event_SetCrossfaderAssign() when setCrossfaderAssign != null:
return setCrossfaderAssign(_that.field0);case Event_SetSpeed() when setSpeed != null:
return setSpeed(_that.field0);case Event_SetTempoRange() when setTempoRange != null:
return setTempoRange(_that.field0);case Event_SetKeyLock() when setKeyLock != null:
return setKeyLock(_that.field0);case Event_SetKeyShift() when setKeyShift != null:
return setKeyShift(_that.field0);case Event_SetKeyboardRoot() when setKeyboardRoot != null:
return setKeyboardRoot(_that.field0);case Event_SetKeyboardPage() when setKeyboardPage != null:
return setKeyboardPage(_that.field0);case Event_SetKeyShiftPage() when setKeyShiftPage != null:
return setKeyShiftPage(_that.field0);case Event_ToggleSync() when toggleSync != null:
return toggleSync(_that.field0);case Event_SetMasterDeck() when setMasterDeck != null:
return setMasterDeck(_that.field0);case Event_SetCuePoint() when setCuePoint != null:
return setCuePoint(_that.field0);case Event_BeginCueHold() when beginCueHold != null:
return beginCueHold(_that.field0);case Event_EndCueHold() when endCueHold != null:
return endCueHold(_that.field0);case Event_SetQuantize() when setQuantize != null:
return setQuantize(_that.field0);case Event_ToggleQuantize() when toggleQuantize != null:
return toggleQuantize(_that.field0);case Event_SetSlip() when setSlip != null:
return setSlip(_that.field0);case Event_ToggleSlip() when toggleSlip != null:
return toggleSlip(_that.field0);case Event_SetAutoLoop() when setAutoLoop != null:
return setAutoLoop(_that.field0);case Event_LoopIn() when loopIn != null:
return loopIn(_that.field0);case Event_LoopOut() when loopOut != null:
return loopOut(_that.field0);case Event_ExitLoop() when exitLoop != null:
return exitLoop(_that.field0);case Event_RecallSavedLoop() when recallSavedLoop != null:
return recallSavedLoop(_that.field0);case Event_BeginLoopRoll() when beginLoopRoll != null:
return beginLoopRoll(_that.field0);case Event_EndLoopRoll() when endLoopRoll != null:
return endLoopRoll(_that.field0);case Event_BeatJump() when beatJump != null:
return beatJump(_that.field0);case Event_SetPadMode() when setPadMode != null:
return setPadMode(_that.field0);case Event_PadPress() when padPress != null:
return padPress(_that.field0);case Event_PadRelease() when padRelease != null:
return padRelease(_that.field0);case Event_HotCuePadPress() when hotCuePadPress != null:
return hotCuePadPress(_that.field0);case Event_HotCuePadRelease() when hotCuePadRelease != null:
return hotCuePadRelease(_that.field0);case Event_LoopRollPadPress() when loopRollPadPress != null:
return loopRollPadPress(_that.field0);case Event_LoopRollPadRelease() when loopRollPadRelease != null:
return loopRollPadRelease(_that.field0);case Event_BeatJumpPadPress() when beatJumpPadPress != null:
return beatJumpPadPress(_that.field0);case Event_BeatJumpPadRelease() when beatJumpPadRelease != null:
return beatJumpPadRelease(_that.field0);case Event_SamplerPadPress() when samplerPadPress != null:
return samplerPadPress(_that.field0);case Event_SamplerPadRelease() when samplerPadRelease != null:
return samplerPadRelease(_that.field0);case Event_KeyboardPadPress() when keyboardPadPress != null:
return keyboardPadPress(_that.field0);case Event_KeyboardPadRelease() when keyboardPadRelease != null:
return keyboardPadRelease(_that.field0);case Event_KeyShiftPadPress() when keyShiftPadPress != null:
return keyShiftPadPress(_that.field0);case Event_KeyShiftPadRelease() when keyShiftPadRelease != null:
return keyShiftPadRelease(_that.field0);case Event_TriggerHotCue() when triggerHotCue != null:
return triggerHotCue(_that.field0);case Event_SaveDeckHotCue() when saveDeckHotCue != null:
return saveDeckHotCue(_that.field0);case Event_DeleteDeckHotCue() when deleteDeckHotCue != null:
return deleteDeckHotCue(_that.field0);case Event_AssignSampler() when assignSampler != null:
return assignSampler(_that.field0);case Event_AssignSamplerTrack() when assignSamplerTrack != null:
return assignSamplerTrack(_that.field0);case Event_ClearSampler() when clearSampler != null:
return clearSampler(_that.field0);case Event_JogTouch() when jogTouch != null:
return jogTouch(_that.field0);case Event_JogTurn() when jogTurn != null:
return jogTurn(_that.field0);case Event_SetJogMode() when setJogMode != null:
return setJogMode(_that.field0);case Event_SetCrossfader() when setCrossfader != null:
return setCrossfader(_that.field0);case Event_SetMasterVolume() when setMasterVolume != null:
return setMasterVolume(_that.field0);case Event_SetCueVolume() when setCueVolume != null:
return setCueVolume(_that.field0);case Event_SetCueMix() when setCueMix != null:
return setCueMix(_that.field0);case Event_SetMasterCue() when setMasterCue != null:
return setMasterCue(_that.field0);case Event_ToggleMasterCue() when toggleMasterCue != null:
return toggleMasterCue(_that.field0);case Event_SetFxUnitSlot() when setFxUnitSlot != null:
return setFxUnitSlot(_that.field0);case Event_SetFxUnitSlotEnabled() when setFxUnitSlotEnabled != null:
return setFxUnitSlotEnabled(_that.field0);case Event_SetFxUnitWet() when setFxUnitWet != null:
return setFxUnitWet(_that.field0);case Event_SetFxUnitParam() when setFxUnitParam != null:
return setFxUnitParam(_that.field0);case Event_SetDeckFxRouted() when setDeckFxRouted != null:
return setDeckFxRouted(_that.field0);case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that.field0);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that.field0);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that.field0);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that.field0);case Event_Error() when error != null:
return error(_that.field0);case Event_Notice() when notice != null:
return notice(_that.field0);case Event_AnalyzeTrack() when analyzeTrack != null:
return analyzeTrack(_that.field0);case Event_GenerateStems() when generateStems != null:
return generateStems(_that.field0);case Event_RefreshTrack() when refreshTrack != null:
return refreshTrack(_that.field0);case Event_SaveBeatGrid() when saveBeatGrid != null:
return saveBeatGrid(_that.field0);case Event_SaveTrackHotCue() when saveTrackHotCue != null:
return saveTrackHotCue(_that.field0);case Event_DeleteTrackHotCue() when deleteTrackHotCue != null:
return deleteTrackHotCue(_that.field0);case Event_SaveTrackLoop() when saveTrackLoop != null:
return saveTrackLoop(_that.field0);case Event_DeleteTrackLoop() when deleteTrackLoop != null:
return deleteTrackLoop(_that.field0);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that.field0);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that.field0);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that.field0);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that.field0);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that.field0);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that.field0);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that.field0);case Event_Navigate() when navigate != null:
return navigate(_that.field0);case Event_Focus() when focus != null:
return focus(_that.field0);case Event_Load() when load != null:
return load(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class Event_Play extends Event {
  const Event_Play(this.field0): super._();
  

@override final  Play field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_PlayCopyWith<Event_Play> get copyWith => _$Event_PlayCopyWithImpl<Event_Play>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Play&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.play(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_PlayCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_PlayCopyWith(Event_Play value, $Res Function(Event_Play) _then) = _$Event_PlayCopyWithImpl;
@useResult
$Res call({
 Play field0
});




}
/// @nodoc
class _$Event_PlayCopyWithImpl<$Res>
    implements $Event_PlayCopyWith<$Res> {
  _$Event_PlayCopyWithImpl(this._self, this._then);

  final Event_Play _self;
  final $Res Function(Event_Play) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Play(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Play,
  ));
}


}

/// @nodoc


class Event_Pause extends Event {
  const Event_Pause(this.field0): super._();
  

@override final  Pause field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_PauseCopyWith<Event_Pause> get copyWith => _$Event_PauseCopyWithImpl<Event_Pause>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Pause&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.pause(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_PauseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_PauseCopyWith(Event_Pause value, $Res Function(Event_Pause) _then) = _$Event_PauseCopyWithImpl;
@useResult
$Res call({
 Pause field0
});




}
/// @nodoc
class _$Event_PauseCopyWithImpl<$Res>
    implements $Event_PauseCopyWith<$Res> {
  _$Event_PauseCopyWithImpl(this._self, this._then);

  final Event_Pause _self;
  final $Res Function(Event_Pause) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Pause(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Pause,
  ));
}


}

/// @nodoc


class Event_TogglePlay extends Event {
  const Event_TogglePlay(this.field0): super._();
  

@override final  TogglePlay field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_TogglePlayCopyWith<Event_TogglePlay> get copyWith => _$Event_TogglePlayCopyWithImpl<Event_TogglePlay>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_TogglePlay&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.togglePlay(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_TogglePlayCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_TogglePlayCopyWith(Event_TogglePlay value, $Res Function(Event_TogglePlay) _then) = _$Event_TogglePlayCopyWithImpl;
@useResult
$Res call({
 TogglePlay field0
});




}
/// @nodoc
class _$Event_TogglePlayCopyWithImpl<$Res>
    implements $Event_TogglePlayCopyWith<$Res> {
  _$Event_TogglePlayCopyWithImpl(this._self, this._then);

  final Event_TogglePlay _self;
  final $Res Function(Event_TogglePlay) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_TogglePlay(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as TogglePlay,
  ));
}


}

/// @nodoc


class Event_Seek extends Event {
  const Event_Seek(this.field0): super._();
  

@override final  Seek field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SeekCopyWith<Event_Seek> get copyWith => _$Event_SeekCopyWithImpl<Event_Seek>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Seek&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.seek(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SeekCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SeekCopyWith(Event_Seek value, $Res Function(Event_Seek) _then) = _$Event_SeekCopyWithImpl;
@useResult
$Res call({
 Seek field0
});




}
/// @nodoc
class _$Event_SeekCopyWithImpl<$Res>
    implements $Event_SeekCopyWith<$Res> {
  _$Event_SeekCopyWithImpl(this._self, this._then);

  final Event_Seek _self;
  final $Res Function(Event_Seek) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Seek(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Seek,
  ));
}


}

/// @nodoc


class Event_Unload extends Event {
  const Event_Unload(this.field0): super._();
  

@override final  Unload field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_UnloadCopyWith<Event_Unload> get copyWith => _$Event_UnloadCopyWithImpl<Event_Unload>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Unload&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.unload(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_UnloadCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_UnloadCopyWith(Event_Unload value, $Res Function(Event_Unload) _then) = _$Event_UnloadCopyWithImpl;
@useResult
$Res call({
 Unload field0
});




}
/// @nodoc
class _$Event_UnloadCopyWithImpl<$Res>
    implements $Event_UnloadCopyWith<$Res> {
  _$Event_UnloadCopyWithImpl(this._self, this._then);

  final Event_Unload _self;
  final $Res Function(Event_Unload) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Unload(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Unload,
  ));
}


}

/// @nodoc


class Event_SetVolume extends Event {
  const Event_SetVolume(this.field0): super._();
  

@override final  SetVolume field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetVolumeCopyWith<Event_SetVolume> get copyWith => _$Event_SetVolumeCopyWithImpl<Event_SetVolume>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetVolume&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setVolume(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetVolumeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetVolumeCopyWith(Event_SetVolume value, $Res Function(Event_SetVolume) _then) = _$Event_SetVolumeCopyWithImpl;
@useResult
$Res call({
 SetVolume field0
});




}
/// @nodoc
class _$Event_SetVolumeCopyWithImpl<$Res>
    implements $Event_SetVolumeCopyWith<$Res> {
  _$Event_SetVolumeCopyWithImpl(this._self, this._then);

  final Event_SetVolume _self;
  final $Res Function(Event_SetVolume) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetVolume(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetVolume,
  ));
}


}

/// @nodoc


class Event_SetEq extends Event {
  const Event_SetEq(this.field0): super._();
  

@override final  SetEq field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetEqCopyWith<Event_SetEq> get copyWith => _$Event_SetEqCopyWithImpl<Event_SetEq>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetEq&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setEq(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetEqCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetEqCopyWith(Event_SetEq value, $Res Function(Event_SetEq) _then) = _$Event_SetEqCopyWithImpl;
@useResult
$Res call({
 SetEq field0
});




}
/// @nodoc
class _$Event_SetEqCopyWithImpl<$Res>
    implements $Event_SetEqCopyWith<$Res> {
  _$Event_SetEqCopyWithImpl(this._self, this._then);

  final Event_SetEq _self;
  final $Res Function(Event_SetEq) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetEq(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetEq,
  ));
}


}

/// @nodoc


class Event_SetEqBand extends Event {
  const Event_SetEqBand(this.field0): super._();
  

@override final  SetEqBand field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetEqBandCopyWith<Event_SetEqBand> get copyWith => _$Event_SetEqBandCopyWithImpl<Event_SetEqBand>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetEqBand&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setEqBand(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetEqBandCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetEqBandCopyWith(Event_SetEqBand value, $Res Function(Event_SetEqBand) _then) = _$Event_SetEqBandCopyWithImpl;
@useResult
$Res call({
 SetEqBand field0
});




}
/// @nodoc
class _$Event_SetEqBandCopyWithImpl<$Res>
    implements $Event_SetEqBandCopyWith<$Res> {
  _$Event_SetEqBandCopyWithImpl(this._self, this._then);

  final Event_SetEqBand _self;
  final $Res Function(Event_SetEqBand) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetEqBand(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetEqBand,
  ));
}


}

/// @nodoc


class Event_SetFilter extends Event {
  const Event_SetFilter(this.field0): super._();
  

@override final  SetFilter field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetFilterCopyWith<Event_SetFilter> get copyWith => _$Event_SetFilterCopyWithImpl<Event_SetFilter>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetFilter&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setFilter(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetFilterCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetFilterCopyWith(Event_SetFilter value, $Res Function(Event_SetFilter) _then) = _$Event_SetFilterCopyWithImpl;
@useResult
$Res call({
 SetFilter field0
});




}
/// @nodoc
class _$Event_SetFilterCopyWithImpl<$Res>
    implements $Event_SetFilterCopyWith<$Res> {
  _$Event_SetFilterCopyWithImpl(this._self, this._then);

  final Event_SetFilter _self;
  final $Res Function(Event_SetFilter) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetFilter(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetFilter,
  ));
}


}

/// @nodoc


class Event_SetGainTrim extends Event {
  const Event_SetGainTrim(this.field0): super._();
  

@override final  SetGainTrim field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetGainTrimCopyWith<Event_SetGainTrim> get copyWith => _$Event_SetGainTrimCopyWithImpl<Event_SetGainTrim>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetGainTrim&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setGainTrim(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetGainTrimCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetGainTrimCopyWith(Event_SetGainTrim value, $Res Function(Event_SetGainTrim) _then) = _$Event_SetGainTrimCopyWithImpl;
@useResult
$Res call({
 SetGainTrim field0
});




}
/// @nodoc
class _$Event_SetGainTrimCopyWithImpl<$Res>
    implements $Event_SetGainTrimCopyWith<$Res> {
  _$Event_SetGainTrimCopyWithImpl(this._self, this._then);

  final Event_SetGainTrim _self;
  final $Res Function(Event_SetGainTrim) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetGainTrim(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetGainTrim,
  ));
}


}

/// @nodoc


class Event_SetHeadphoneCue extends Event {
  const Event_SetHeadphoneCue(this.field0): super._();
  

@override final  SetHeadphoneCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetHeadphoneCueCopyWith<Event_SetHeadphoneCue> get copyWith => _$Event_SetHeadphoneCueCopyWithImpl<Event_SetHeadphoneCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetHeadphoneCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setHeadphoneCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetHeadphoneCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetHeadphoneCueCopyWith(Event_SetHeadphoneCue value, $Res Function(Event_SetHeadphoneCue) _then) = _$Event_SetHeadphoneCueCopyWithImpl;
@useResult
$Res call({
 SetHeadphoneCue field0
});




}
/// @nodoc
class _$Event_SetHeadphoneCueCopyWithImpl<$Res>
    implements $Event_SetHeadphoneCueCopyWith<$Res> {
  _$Event_SetHeadphoneCueCopyWithImpl(this._self, this._then);

  final Event_SetHeadphoneCue _self;
  final $Res Function(Event_SetHeadphoneCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetHeadphoneCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetHeadphoneCue,
  ));
}


}

/// @nodoc


class Event_ToggleHeadphoneCue extends Event {
  const Event_ToggleHeadphoneCue(this.field0): super._();
  

@override final  ToggleHeadphoneCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ToggleHeadphoneCueCopyWith<Event_ToggleHeadphoneCue> get copyWith => _$Event_ToggleHeadphoneCueCopyWithImpl<Event_ToggleHeadphoneCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_ToggleHeadphoneCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.toggleHeadphoneCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ToggleHeadphoneCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ToggleHeadphoneCueCopyWith(Event_ToggleHeadphoneCue value, $Res Function(Event_ToggleHeadphoneCue) _then) = _$Event_ToggleHeadphoneCueCopyWithImpl;
@useResult
$Res call({
 ToggleHeadphoneCue field0
});




}
/// @nodoc
class _$Event_ToggleHeadphoneCueCopyWithImpl<$Res>
    implements $Event_ToggleHeadphoneCueCopyWith<$Res> {
  _$Event_ToggleHeadphoneCueCopyWithImpl(this._self, this._then);

  final Event_ToggleHeadphoneCue _self;
  final $Res Function(Event_ToggleHeadphoneCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_ToggleHeadphoneCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ToggleHeadphoneCue,
  ));
}


}

/// @nodoc


class Event_SetCrossfaderAssign extends Event {
  const Event_SetCrossfaderAssign(this.field0): super._();
  

@override final  SetCrossfaderAssign field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetCrossfaderAssignCopyWith<Event_SetCrossfaderAssign> get copyWith => _$Event_SetCrossfaderAssignCopyWithImpl<Event_SetCrossfaderAssign>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetCrossfaderAssign&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setCrossfaderAssign(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetCrossfaderAssignCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetCrossfaderAssignCopyWith(Event_SetCrossfaderAssign value, $Res Function(Event_SetCrossfaderAssign) _then) = _$Event_SetCrossfaderAssignCopyWithImpl;
@useResult
$Res call({
 SetCrossfaderAssign field0
});




}
/// @nodoc
class _$Event_SetCrossfaderAssignCopyWithImpl<$Res>
    implements $Event_SetCrossfaderAssignCopyWith<$Res> {
  _$Event_SetCrossfaderAssignCopyWithImpl(this._self, this._then);

  final Event_SetCrossfaderAssign _self;
  final $Res Function(Event_SetCrossfaderAssign) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetCrossfaderAssign(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetCrossfaderAssign,
  ));
}


}

/// @nodoc


class Event_SetSpeed extends Event {
  const Event_SetSpeed(this.field0): super._();
  

@override final  SetSpeed field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetSpeedCopyWith<Event_SetSpeed> get copyWith => _$Event_SetSpeedCopyWithImpl<Event_SetSpeed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetSpeed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setSpeed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetSpeedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetSpeedCopyWith(Event_SetSpeed value, $Res Function(Event_SetSpeed) _then) = _$Event_SetSpeedCopyWithImpl;
@useResult
$Res call({
 SetSpeed field0
});




}
/// @nodoc
class _$Event_SetSpeedCopyWithImpl<$Res>
    implements $Event_SetSpeedCopyWith<$Res> {
  _$Event_SetSpeedCopyWithImpl(this._self, this._then);

  final Event_SetSpeed _self;
  final $Res Function(Event_SetSpeed) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetSpeed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetSpeed,
  ));
}


}

/// @nodoc


class Event_SetTempoRange extends Event {
  const Event_SetTempoRange(this.field0): super._();
  

@override final  SetTempoRange field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetTempoRangeCopyWith<Event_SetTempoRange> get copyWith => _$Event_SetTempoRangeCopyWithImpl<Event_SetTempoRange>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetTempoRange&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setTempoRange(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetTempoRangeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetTempoRangeCopyWith(Event_SetTempoRange value, $Res Function(Event_SetTempoRange) _then) = _$Event_SetTempoRangeCopyWithImpl;
@useResult
$Res call({
 SetTempoRange field0
});




}
/// @nodoc
class _$Event_SetTempoRangeCopyWithImpl<$Res>
    implements $Event_SetTempoRangeCopyWith<$Res> {
  _$Event_SetTempoRangeCopyWithImpl(this._self, this._then);

  final Event_SetTempoRange _self;
  final $Res Function(Event_SetTempoRange) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetTempoRange(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetTempoRange,
  ));
}


}

/// @nodoc


class Event_SetKeyLock extends Event {
  const Event_SetKeyLock(this.field0): super._();
  

@override final  SetKeyLock field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetKeyLockCopyWith<Event_SetKeyLock> get copyWith => _$Event_SetKeyLockCopyWithImpl<Event_SetKeyLock>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetKeyLock&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setKeyLock(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetKeyLockCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetKeyLockCopyWith(Event_SetKeyLock value, $Res Function(Event_SetKeyLock) _then) = _$Event_SetKeyLockCopyWithImpl;
@useResult
$Res call({
 SetKeyLock field0
});




}
/// @nodoc
class _$Event_SetKeyLockCopyWithImpl<$Res>
    implements $Event_SetKeyLockCopyWith<$Res> {
  _$Event_SetKeyLockCopyWithImpl(this._self, this._then);

  final Event_SetKeyLock _self;
  final $Res Function(Event_SetKeyLock) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetKeyLock(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetKeyLock,
  ));
}


}

/// @nodoc


class Event_SetKeyShift extends Event {
  const Event_SetKeyShift(this.field0): super._();
  

@override final  SetKeyShift field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetKeyShiftCopyWith<Event_SetKeyShift> get copyWith => _$Event_SetKeyShiftCopyWithImpl<Event_SetKeyShift>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetKeyShift&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setKeyShift(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetKeyShiftCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetKeyShiftCopyWith(Event_SetKeyShift value, $Res Function(Event_SetKeyShift) _then) = _$Event_SetKeyShiftCopyWithImpl;
@useResult
$Res call({
 SetKeyShift field0
});




}
/// @nodoc
class _$Event_SetKeyShiftCopyWithImpl<$Res>
    implements $Event_SetKeyShiftCopyWith<$Res> {
  _$Event_SetKeyShiftCopyWithImpl(this._self, this._then);

  final Event_SetKeyShift _self;
  final $Res Function(Event_SetKeyShift) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetKeyShift(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetKeyShift,
  ));
}


}

/// @nodoc


class Event_SetKeyboardRoot extends Event {
  const Event_SetKeyboardRoot(this.field0): super._();
  

@override final  SetKeyboardRoot field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetKeyboardRootCopyWith<Event_SetKeyboardRoot> get copyWith => _$Event_SetKeyboardRootCopyWithImpl<Event_SetKeyboardRoot>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetKeyboardRoot&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setKeyboardRoot(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetKeyboardRootCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetKeyboardRootCopyWith(Event_SetKeyboardRoot value, $Res Function(Event_SetKeyboardRoot) _then) = _$Event_SetKeyboardRootCopyWithImpl;
@useResult
$Res call({
 SetKeyboardRoot field0
});




}
/// @nodoc
class _$Event_SetKeyboardRootCopyWithImpl<$Res>
    implements $Event_SetKeyboardRootCopyWith<$Res> {
  _$Event_SetKeyboardRootCopyWithImpl(this._self, this._then);

  final Event_SetKeyboardRoot _self;
  final $Res Function(Event_SetKeyboardRoot) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetKeyboardRoot(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetKeyboardRoot,
  ));
}


}

/// @nodoc


class Event_SetKeyboardPage extends Event {
  const Event_SetKeyboardPage(this.field0): super._();
  

@override final  SetKeyboardPage field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetKeyboardPageCopyWith<Event_SetKeyboardPage> get copyWith => _$Event_SetKeyboardPageCopyWithImpl<Event_SetKeyboardPage>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetKeyboardPage&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setKeyboardPage(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetKeyboardPageCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetKeyboardPageCopyWith(Event_SetKeyboardPage value, $Res Function(Event_SetKeyboardPage) _then) = _$Event_SetKeyboardPageCopyWithImpl;
@useResult
$Res call({
 SetKeyboardPage field0
});




}
/// @nodoc
class _$Event_SetKeyboardPageCopyWithImpl<$Res>
    implements $Event_SetKeyboardPageCopyWith<$Res> {
  _$Event_SetKeyboardPageCopyWithImpl(this._self, this._then);

  final Event_SetKeyboardPage _self;
  final $Res Function(Event_SetKeyboardPage) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetKeyboardPage(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetKeyboardPage,
  ));
}


}

/// @nodoc


class Event_SetKeyShiftPage extends Event {
  const Event_SetKeyShiftPage(this.field0): super._();
  

@override final  SetKeyShiftPage field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetKeyShiftPageCopyWith<Event_SetKeyShiftPage> get copyWith => _$Event_SetKeyShiftPageCopyWithImpl<Event_SetKeyShiftPage>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetKeyShiftPage&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setKeyShiftPage(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetKeyShiftPageCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetKeyShiftPageCopyWith(Event_SetKeyShiftPage value, $Res Function(Event_SetKeyShiftPage) _then) = _$Event_SetKeyShiftPageCopyWithImpl;
@useResult
$Res call({
 SetKeyShiftPage field0
});




}
/// @nodoc
class _$Event_SetKeyShiftPageCopyWithImpl<$Res>
    implements $Event_SetKeyShiftPageCopyWith<$Res> {
  _$Event_SetKeyShiftPageCopyWithImpl(this._self, this._then);

  final Event_SetKeyShiftPage _self;
  final $Res Function(Event_SetKeyShiftPage) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetKeyShiftPage(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetKeyShiftPage,
  ));
}


}

/// @nodoc


class Event_ToggleSync extends Event {
  const Event_ToggleSync(this.field0): super._();
  

@override final  ToggleSync field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ToggleSyncCopyWith<Event_ToggleSync> get copyWith => _$Event_ToggleSyncCopyWithImpl<Event_ToggleSync>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_ToggleSync&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.toggleSync(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ToggleSyncCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ToggleSyncCopyWith(Event_ToggleSync value, $Res Function(Event_ToggleSync) _then) = _$Event_ToggleSyncCopyWithImpl;
@useResult
$Res call({
 ToggleSync field0
});




}
/// @nodoc
class _$Event_ToggleSyncCopyWithImpl<$Res>
    implements $Event_ToggleSyncCopyWith<$Res> {
  _$Event_ToggleSyncCopyWithImpl(this._self, this._then);

  final Event_ToggleSync _self;
  final $Res Function(Event_ToggleSync) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_ToggleSync(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ToggleSync,
  ));
}


}

/// @nodoc


class Event_SetMasterDeck extends Event {
  const Event_SetMasterDeck(this.field0): super._();
  

@override final  SetMasterDeck field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetMasterDeckCopyWith<Event_SetMasterDeck> get copyWith => _$Event_SetMasterDeckCopyWithImpl<Event_SetMasterDeck>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetMasterDeck&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setMasterDeck(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetMasterDeckCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetMasterDeckCopyWith(Event_SetMasterDeck value, $Res Function(Event_SetMasterDeck) _then) = _$Event_SetMasterDeckCopyWithImpl;
@useResult
$Res call({
 SetMasterDeck field0
});




}
/// @nodoc
class _$Event_SetMasterDeckCopyWithImpl<$Res>
    implements $Event_SetMasterDeckCopyWith<$Res> {
  _$Event_SetMasterDeckCopyWithImpl(this._self, this._then);

  final Event_SetMasterDeck _self;
  final $Res Function(Event_SetMasterDeck) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetMasterDeck(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetMasterDeck,
  ));
}


}

/// @nodoc


class Event_SetCuePoint extends Event {
  const Event_SetCuePoint(this.field0): super._();
  

@override final  SetCuePoint field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetCuePointCopyWith<Event_SetCuePoint> get copyWith => _$Event_SetCuePointCopyWithImpl<Event_SetCuePoint>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetCuePoint&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setCuePoint(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetCuePointCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetCuePointCopyWith(Event_SetCuePoint value, $Res Function(Event_SetCuePoint) _then) = _$Event_SetCuePointCopyWithImpl;
@useResult
$Res call({
 SetCuePoint field0
});




}
/// @nodoc
class _$Event_SetCuePointCopyWithImpl<$Res>
    implements $Event_SetCuePointCopyWith<$Res> {
  _$Event_SetCuePointCopyWithImpl(this._self, this._then);

  final Event_SetCuePoint _self;
  final $Res Function(Event_SetCuePoint) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetCuePoint(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetCuePoint,
  ));
}


}

/// @nodoc


class Event_BeginCueHold extends Event {
  const Event_BeginCueHold(this.field0): super._();
  

@override final  BeginCueHold field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_BeginCueHoldCopyWith<Event_BeginCueHold> get copyWith => _$Event_BeginCueHoldCopyWithImpl<Event_BeginCueHold>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_BeginCueHold&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.beginCueHold(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_BeginCueHoldCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_BeginCueHoldCopyWith(Event_BeginCueHold value, $Res Function(Event_BeginCueHold) _then) = _$Event_BeginCueHoldCopyWithImpl;
@useResult
$Res call({
 BeginCueHold field0
});




}
/// @nodoc
class _$Event_BeginCueHoldCopyWithImpl<$Res>
    implements $Event_BeginCueHoldCopyWith<$Res> {
  _$Event_BeginCueHoldCopyWithImpl(this._self, this._then);

  final Event_BeginCueHold _self;
  final $Res Function(Event_BeginCueHold) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_BeginCueHold(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as BeginCueHold,
  ));
}


}

/// @nodoc


class Event_EndCueHold extends Event {
  const Event_EndCueHold(this.field0): super._();
  

@override final  EndCueHold field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_EndCueHoldCopyWith<Event_EndCueHold> get copyWith => _$Event_EndCueHoldCopyWithImpl<Event_EndCueHold>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_EndCueHold&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.endCueHold(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_EndCueHoldCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_EndCueHoldCopyWith(Event_EndCueHold value, $Res Function(Event_EndCueHold) _then) = _$Event_EndCueHoldCopyWithImpl;
@useResult
$Res call({
 EndCueHold field0
});




}
/// @nodoc
class _$Event_EndCueHoldCopyWithImpl<$Res>
    implements $Event_EndCueHoldCopyWith<$Res> {
  _$Event_EndCueHoldCopyWithImpl(this._self, this._then);

  final Event_EndCueHold _self;
  final $Res Function(Event_EndCueHold) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_EndCueHold(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as EndCueHold,
  ));
}


}

/// @nodoc


class Event_SetQuantize extends Event {
  const Event_SetQuantize(this.field0): super._();
  

@override final  SetQuantize field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetQuantizeCopyWith<Event_SetQuantize> get copyWith => _$Event_SetQuantizeCopyWithImpl<Event_SetQuantize>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetQuantize&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setQuantize(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetQuantizeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetQuantizeCopyWith(Event_SetQuantize value, $Res Function(Event_SetQuantize) _then) = _$Event_SetQuantizeCopyWithImpl;
@useResult
$Res call({
 SetQuantize field0
});




}
/// @nodoc
class _$Event_SetQuantizeCopyWithImpl<$Res>
    implements $Event_SetQuantizeCopyWith<$Res> {
  _$Event_SetQuantizeCopyWithImpl(this._self, this._then);

  final Event_SetQuantize _self;
  final $Res Function(Event_SetQuantize) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetQuantize(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetQuantize,
  ));
}


}

/// @nodoc


class Event_ToggleQuantize extends Event {
  const Event_ToggleQuantize(this.field0): super._();
  

@override final  ToggleQuantize field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ToggleQuantizeCopyWith<Event_ToggleQuantize> get copyWith => _$Event_ToggleQuantizeCopyWithImpl<Event_ToggleQuantize>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_ToggleQuantize&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.toggleQuantize(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ToggleQuantizeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ToggleQuantizeCopyWith(Event_ToggleQuantize value, $Res Function(Event_ToggleQuantize) _then) = _$Event_ToggleQuantizeCopyWithImpl;
@useResult
$Res call({
 ToggleQuantize field0
});




}
/// @nodoc
class _$Event_ToggleQuantizeCopyWithImpl<$Res>
    implements $Event_ToggleQuantizeCopyWith<$Res> {
  _$Event_ToggleQuantizeCopyWithImpl(this._self, this._then);

  final Event_ToggleQuantize _self;
  final $Res Function(Event_ToggleQuantize) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_ToggleQuantize(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ToggleQuantize,
  ));
}


}

/// @nodoc


class Event_SetSlip extends Event {
  const Event_SetSlip(this.field0): super._();
  

@override final  SetSlip field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetSlipCopyWith<Event_SetSlip> get copyWith => _$Event_SetSlipCopyWithImpl<Event_SetSlip>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetSlip&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setSlip(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetSlipCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetSlipCopyWith(Event_SetSlip value, $Res Function(Event_SetSlip) _then) = _$Event_SetSlipCopyWithImpl;
@useResult
$Res call({
 SetSlip field0
});




}
/// @nodoc
class _$Event_SetSlipCopyWithImpl<$Res>
    implements $Event_SetSlipCopyWith<$Res> {
  _$Event_SetSlipCopyWithImpl(this._self, this._then);

  final Event_SetSlip _self;
  final $Res Function(Event_SetSlip) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetSlip(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetSlip,
  ));
}


}

/// @nodoc


class Event_ToggleSlip extends Event {
  const Event_ToggleSlip(this.field0): super._();
  

@override final  ToggleSlip field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ToggleSlipCopyWith<Event_ToggleSlip> get copyWith => _$Event_ToggleSlipCopyWithImpl<Event_ToggleSlip>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_ToggleSlip&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.toggleSlip(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ToggleSlipCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ToggleSlipCopyWith(Event_ToggleSlip value, $Res Function(Event_ToggleSlip) _then) = _$Event_ToggleSlipCopyWithImpl;
@useResult
$Res call({
 ToggleSlip field0
});




}
/// @nodoc
class _$Event_ToggleSlipCopyWithImpl<$Res>
    implements $Event_ToggleSlipCopyWith<$Res> {
  _$Event_ToggleSlipCopyWithImpl(this._self, this._then);

  final Event_ToggleSlip _self;
  final $Res Function(Event_ToggleSlip) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_ToggleSlip(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ToggleSlip,
  ));
}


}

/// @nodoc


class Event_SetAutoLoop extends Event {
  const Event_SetAutoLoop(this.field0): super._();
  

@override final  SetAutoLoop field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetAutoLoopCopyWith<Event_SetAutoLoop> get copyWith => _$Event_SetAutoLoopCopyWithImpl<Event_SetAutoLoop>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetAutoLoop&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setAutoLoop(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetAutoLoopCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetAutoLoopCopyWith(Event_SetAutoLoop value, $Res Function(Event_SetAutoLoop) _then) = _$Event_SetAutoLoopCopyWithImpl;
@useResult
$Res call({
 SetAutoLoop field0
});




}
/// @nodoc
class _$Event_SetAutoLoopCopyWithImpl<$Res>
    implements $Event_SetAutoLoopCopyWith<$Res> {
  _$Event_SetAutoLoopCopyWithImpl(this._self, this._then);

  final Event_SetAutoLoop _self;
  final $Res Function(Event_SetAutoLoop) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetAutoLoop(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetAutoLoop,
  ));
}


}

/// @nodoc


class Event_LoopIn extends Event {
  const Event_LoopIn(this.field0): super._();
  

@override final  LoopIn field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_LoopInCopyWith<Event_LoopIn> get copyWith => _$Event_LoopInCopyWithImpl<Event_LoopIn>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_LoopIn&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.loopIn(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_LoopInCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_LoopInCopyWith(Event_LoopIn value, $Res Function(Event_LoopIn) _then) = _$Event_LoopInCopyWithImpl;
@useResult
$Res call({
 LoopIn field0
});




}
/// @nodoc
class _$Event_LoopInCopyWithImpl<$Res>
    implements $Event_LoopInCopyWith<$Res> {
  _$Event_LoopInCopyWithImpl(this._self, this._then);

  final Event_LoopIn _self;
  final $Res Function(Event_LoopIn) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_LoopIn(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as LoopIn,
  ));
}


}

/// @nodoc


class Event_LoopOut extends Event {
  const Event_LoopOut(this.field0): super._();
  

@override final  LoopOut field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_LoopOutCopyWith<Event_LoopOut> get copyWith => _$Event_LoopOutCopyWithImpl<Event_LoopOut>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_LoopOut&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.loopOut(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_LoopOutCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_LoopOutCopyWith(Event_LoopOut value, $Res Function(Event_LoopOut) _then) = _$Event_LoopOutCopyWithImpl;
@useResult
$Res call({
 LoopOut field0
});




}
/// @nodoc
class _$Event_LoopOutCopyWithImpl<$Res>
    implements $Event_LoopOutCopyWith<$Res> {
  _$Event_LoopOutCopyWithImpl(this._self, this._then);

  final Event_LoopOut _self;
  final $Res Function(Event_LoopOut) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_LoopOut(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as LoopOut,
  ));
}


}

/// @nodoc


class Event_ExitLoop extends Event {
  const Event_ExitLoop(this.field0): super._();
  

@override final  ExitLoop field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ExitLoopCopyWith<Event_ExitLoop> get copyWith => _$Event_ExitLoopCopyWithImpl<Event_ExitLoop>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_ExitLoop&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.exitLoop(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ExitLoopCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ExitLoopCopyWith(Event_ExitLoop value, $Res Function(Event_ExitLoop) _then) = _$Event_ExitLoopCopyWithImpl;
@useResult
$Res call({
 ExitLoop field0
});




}
/// @nodoc
class _$Event_ExitLoopCopyWithImpl<$Res>
    implements $Event_ExitLoopCopyWith<$Res> {
  _$Event_ExitLoopCopyWithImpl(this._self, this._then);

  final Event_ExitLoop _self;
  final $Res Function(Event_ExitLoop) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_ExitLoop(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ExitLoop,
  ));
}


}

/// @nodoc


class Event_RecallSavedLoop extends Event {
  const Event_RecallSavedLoop(this.field0): super._();
  

@override final  RecallSavedLoop field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_RecallSavedLoopCopyWith<Event_RecallSavedLoop> get copyWith => _$Event_RecallSavedLoopCopyWithImpl<Event_RecallSavedLoop>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_RecallSavedLoop&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.recallSavedLoop(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_RecallSavedLoopCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_RecallSavedLoopCopyWith(Event_RecallSavedLoop value, $Res Function(Event_RecallSavedLoop) _then) = _$Event_RecallSavedLoopCopyWithImpl;
@useResult
$Res call({
 RecallSavedLoop field0
});




}
/// @nodoc
class _$Event_RecallSavedLoopCopyWithImpl<$Res>
    implements $Event_RecallSavedLoopCopyWith<$Res> {
  _$Event_RecallSavedLoopCopyWithImpl(this._self, this._then);

  final Event_RecallSavedLoop _self;
  final $Res Function(Event_RecallSavedLoop) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_RecallSavedLoop(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as RecallSavedLoop,
  ));
}


}

/// @nodoc


class Event_BeginLoopRoll extends Event {
  const Event_BeginLoopRoll(this.field0): super._();
  

@override final  BeginLoopRoll field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_BeginLoopRollCopyWith<Event_BeginLoopRoll> get copyWith => _$Event_BeginLoopRollCopyWithImpl<Event_BeginLoopRoll>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_BeginLoopRoll&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.beginLoopRoll(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_BeginLoopRollCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_BeginLoopRollCopyWith(Event_BeginLoopRoll value, $Res Function(Event_BeginLoopRoll) _then) = _$Event_BeginLoopRollCopyWithImpl;
@useResult
$Res call({
 BeginLoopRoll field0
});




}
/// @nodoc
class _$Event_BeginLoopRollCopyWithImpl<$Res>
    implements $Event_BeginLoopRollCopyWith<$Res> {
  _$Event_BeginLoopRollCopyWithImpl(this._self, this._then);

  final Event_BeginLoopRoll _self;
  final $Res Function(Event_BeginLoopRoll) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_BeginLoopRoll(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as BeginLoopRoll,
  ));
}


}

/// @nodoc


class Event_EndLoopRoll extends Event {
  const Event_EndLoopRoll(this.field0): super._();
  

@override final  EndLoopRoll field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_EndLoopRollCopyWith<Event_EndLoopRoll> get copyWith => _$Event_EndLoopRollCopyWithImpl<Event_EndLoopRoll>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_EndLoopRoll&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.endLoopRoll(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_EndLoopRollCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_EndLoopRollCopyWith(Event_EndLoopRoll value, $Res Function(Event_EndLoopRoll) _then) = _$Event_EndLoopRollCopyWithImpl;
@useResult
$Res call({
 EndLoopRoll field0
});




}
/// @nodoc
class _$Event_EndLoopRollCopyWithImpl<$Res>
    implements $Event_EndLoopRollCopyWith<$Res> {
  _$Event_EndLoopRollCopyWithImpl(this._self, this._then);

  final Event_EndLoopRoll _self;
  final $Res Function(Event_EndLoopRoll) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_EndLoopRoll(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as EndLoopRoll,
  ));
}


}

/// @nodoc


class Event_BeatJump extends Event {
  const Event_BeatJump(this.field0): super._();
  

@override final  BeatJump field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_BeatJumpCopyWith<Event_BeatJump> get copyWith => _$Event_BeatJumpCopyWithImpl<Event_BeatJump>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_BeatJump&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.beatJump(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_BeatJumpCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_BeatJumpCopyWith(Event_BeatJump value, $Res Function(Event_BeatJump) _then) = _$Event_BeatJumpCopyWithImpl;
@useResult
$Res call({
 BeatJump field0
});




}
/// @nodoc
class _$Event_BeatJumpCopyWithImpl<$Res>
    implements $Event_BeatJumpCopyWith<$Res> {
  _$Event_BeatJumpCopyWithImpl(this._self, this._then);

  final Event_BeatJump _self;
  final $Res Function(Event_BeatJump) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_BeatJump(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as BeatJump,
  ));
}


}

/// @nodoc


class Event_SetPadMode extends Event {
  const Event_SetPadMode(this.field0): super._();
  

@override final  SetPadMode field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetPadModeCopyWith<Event_SetPadMode> get copyWith => _$Event_SetPadModeCopyWithImpl<Event_SetPadMode>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetPadMode&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setPadMode(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetPadModeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetPadModeCopyWith(Event_SetPadMode value, $Res Function(Event_SetPadMode) _then) = _$Event_SetPadModeCopyWithImpl;
@useResult
$Res call({
 SetPadMode field0
});




}
/// @nodoc
class _$Event_SetPadModeCopyWithImpl<$Res>
    implements $Event_SetPadModeCopyWith<$Res> {
  _$Event_SetPadModeCopyWithImpl(this._self, this._then);

  final Event_SetPadMode _self;
  final $Res Function(Event_SetPadMode) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetPadMode(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetPadMode,
  ));
}


}

/// @nodoc


class Event_PadPress extends Event {
  const Event_PadPress(this.field0): super._();
  

@override final  PadPress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_PadPressCopyWith<Event_PadPress> get copyWith => _$Event_PadPressCopyWithImpl<Event_PadPress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_PadPress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.padPress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_PadPressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_PadPressCopyWith(Event_PadPress value, $Res Function(Event_PadPress) _then) = _$Event_PadPressCopyWithImpl;
@useResult
$Res call({
 PadPress field0
});




}
/// @nodoc
class _$Event_PadPressCopyWithImpl<$Res>
    implements $Event_PadPressCopyWith<$Res> {
  _$Event_PadPressCopyWithImpl(this._self, this._then);

  final Event_PadPress _self;
  final $Res Function(Event_PadPress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_PadPress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PadPress,
  ));
}


}

/// @nodoc


class Event_PadRelease extends Event {
  const Event_PadRelease(this.field0): super._();
  

@override final  PadRelease field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_PadReleaseCopyWith<Event_PadRelease> get copyWith => _$Event_PadReleaseCopyWithImpl<Event_PadRelease>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_PadRelease&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.padRelease(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_PadReleaseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_PadReleaseCopyWith(Event_PadRelease value, $Res Function(Event_PadRelease) _then) = _$Event_PadReleaseCopyWithImpl;
@useResult
$Res call({
 PadRelease field0
});




}
/// @nodoc
class _$Event_PadReleaseCopyWithImpl<$Res>
    implements $Event_PadReleaseCopyWith<$Res> {
  _$Event_PadReleaseCopyWithImpl(this._self, this._then);

  final Event_PadRelease _self;
  final $Res Function(Event_PadRelease) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_PadRelease(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PadRelease,
  ));
}


}

/// @nodoc


class Event_HotCuePadPress extends Event {
  const Event_HotCuePadPress(this.field0): super._();
  

@override final  HotCuePadPress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_HotCuePadPressCopyWith<Event_HotCuePadPress> get copyWith => _$Event_HotCuePadPressCopyWithImpl<Event_HotCuePadPress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_HotCuePadPress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.hotCuePadPress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_HotCuePadPressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_HotCuePadPressCopyWith(Event_HotCuePadPress value, $Res Function(Event_HotCuePadPress) _then) = _$Event_HotCuePadPressCopyWithImpl;
@useResult
$Res call({
 HotCuePadPress field0
});




}
/// @nodoc
class _$Event_HotCuePadPressCopyWithImpl<$Res>
    implements $Event_HotCuePadPressCopyWith<$Res> {
  _$Event_HotCuePadPressCopyWithImpl(this._self, this._then);

  final Event_HotCuePadPress _self;
  final $Res Function(Event_HotCuePadPress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_HotCuePadPress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as HotCuePadPress,
  ));
}


}

/// @nodoc


class Event_HotCuePadRelease extends Event {
  const Event_HotCuePadRelease(this.field0): super._();
  

@override final  HotCuePadRelease field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_HotCuePadReleaseCopyWith<Event_HotCuePadRelease> get copyWith => _$Event_HotCuePadReleaseCopyWithImpl<Event_HotCuePadRelease>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_HotCuePadRelease&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.hotCuePadRelease(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_HotCuePadReleaseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_HotCuePadReleaseCopyWith(Event_HotCuePadRelease value, $Res Function(Event_HotCuePadRelease) _then) = _$Event_HotCuePadReleaseCopyWithImpl;
@useResult
$Res call({
 HotCuePadRelease field0
});




}
/// @nodoc
class _$Event_HotCuePadReleaseCopyWithImpl<$Res>
    implements $Event_HotCuePadReleaseCopyWith<$Res> {
  _$Event_HotCuePadReleaseCopyWithImpl(this._self, this._then);

  final Event_HotCuePadRelease _self;
  final $Res Function(Event_HotCuePadRelease) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_HotCuePadRelease(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as HotCuePadRelease,
  ));
}


}

/// @nodoc


class Event_LoopRollPadPress extends Event {
  const Event_LoopRollPadPress(this.field0): super._();
  

@override final  LoopRollPadPress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_LoopRollPadPressCopyWith<Event_LoopRollPadPress> get copyWith => _$Event_LoopRollPadPressCopyWithImpl<Event_LoopRollPadPress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_LoopRollPadPress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.loopRollPadPress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_LoopRollPadPressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_LoopRollPadPressCopyWith(Event_LoopRollPadPress value, $Res Function(Event_LoopRollPadPress) _then) = _$Event_LoopRollPadPressCopyWithImpl;
@useResult
$Res call({
 LoopRollPadPress field0
});




}
/// @nodoc
class _$Event_LoopRollPadPressCopyWithImpl<$Res>
    implements $Event_LoopRollPadPressCopyWith<$Res> {
  _$Event_LoopRollPadPressCopyWithImpl(this._self, this._then);

  final Event_LoopRollPadPress _self;
  final $Res Function(Event_LoopRollPadPress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_LoopRollPadPress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as LoopRollPadPress,
  ));
}


}

/// @nodoc


class Event_LoopRollPadRelease extends Event {
  const Event_LoopRollPadRelease(this.field0): super._();
  

@override final  LoopRollPadRelease field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_LoopRollPadReleaseCopyWith<Event_LoopRollPadRelease> get copyWith => _$Event_LoopRollPadReleaseCopyWithImpl<Event_LoopRollPadRelease>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_LoopRollPadRelease&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.loopRollPadRelease(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_LoopRollPadReleaseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_LoopRollPadReleaseCopyWith(Event_LoopRollPadRelease value, $Res Function(Event_LoopRollPadRelease) _then) = _$Event_LoopRollPadReleaseCopyWithImpl;
@useResult
$Res call({
 LoopRollPadRelease field0
});




}
/// @nodoc
class _$Event_LoopRollPadReleaseCopyWithImpl<$Res>
    implements $Event_LoopRollPadReleaseCopyWith<$Res> {
  _$Event_LoopRollPadReleaseCopyWithImpl(this._self, this._then);

  final Event_LoopRollPadRelease _self;
  final $Res Function(Event_LoopRollPadRelease) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_LoopRollPadRelease(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as LoopRollPadRelease,
  ));
}


}

/// @nodoc


class Event_BeatJumpPadPress extends Event {
  const Event_BeatJumpPadPress(this.field0): super._();
  

@override final  BeatJumpPadPress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_BeatJumpPadPressCopyWith<Event_BeatJumpPadPress> get copyWith => _$Event_BeatJumpPadPressCopyWithImpl<Event_BeatJumpPadPress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_BeatJumpPadPress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.beatJumpPadPress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_BeatJumpPadPressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_BeatJumpPadPressCopyWith(Event_BeatJumpPadPress value, $Res Function(Event_BeatJumpPadPress) _then) = _$Event_BeatJumpPadPressCopyWithImpl;
@useResult
$Res call({
 BeatJumpPadPress field0
});




}
/// @nodoc
class _$Event_BeatJumpPadPressCopyWithImpl<$Res>
    implements $Event_BeatJumpPadPressCopyWith<$Res> {
  _$Event_BeatJumpPadPressCopyWithImpl(this._self, this._then);

  final Event_BeatJumpPadPress _self;
  final $Res Function(Event_BeatJumpPadPress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_BeatJumpPadPress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as BeatJumpPadPress,
  ));
}


}

/// @nodoc


class Event_BeatJumpPadRelease extends Event {
  const Event_BeatJumpPadRelease(this.field0): super._();
  

@override final  BeatJumpPadRelease field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_BeatJumpPadReleaseCopyWith<Event_BeatJumpPadRelease> get copyWith => _$Event_BeatJumpPadReleaseCopyWithImpl<Event_BeatJumpPadRelease>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_BeatJumpPadRelease&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.beatJumpPadRelease(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_BeatJumpPadReleaseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_BeatJumpPadReleaseCopyWith(Event_BeatJumpPadRelease value, $Res Function(Event_BeatJumpPadRelease) _then) = _$Event_BeatJumpPadReleaseCopyWithImpl;
@useResult
$Res call({
 BeatJumpPadRelease field0
});




}
/// @nodoc
class _$Event_BeatJumpPadReleaseCopyWithImpl<$Res>
    implements $Event_BeatJumpPadReleaseCopyWith<$Res> {
  _$Event_BeatJumpPadReleaseCopyWithImpl(this._self, this._then);

  final Event_BeatJumpPadRelease _self;
  final $Res Function(Event_BeatJumpPadRelease) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_BeatJumpPadRelease(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as BeatJumpPadRelease,
  ));
}


}

/// @nodoc


class Event_SamplerPadPress extends Event {
  const Event_SamplerPadPress(this.field0): super._();
  

@override final  SamplerPadPress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SamplerPadPressCopyWith<Event_SamplerPadPress> get copyWith => _$Event_SamplerPadPressCopyWithImpl<Event_SamplerPadPress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SamplerPadPress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.samplerPadPress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SamplerPadPressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SamplerPadPressCopyWith(Event_SamplerPadPress value, $Res Function(Event_SamplerPadPress) _then) = _$Event_SamplerPadPressCopyWithImpl;
@useResult
$Res call({
 SamplerPadPress field0
});




}
/// @nodoc
class _$Event_SamplerPadPressCopyWithImpl<$Res>
    implements $Event_SamplerPadPressCopyWith<$Res> {
  _$Event_SamplerPadPressCopyWithImpl(this._self, this._then);

  final Event_SamplerPadPress _self;
  final $Res Function(Event_SamplerPadPress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SamplerPadPress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SamplerPadPress,
  ));
}


}

/// @nodoc


class Event_SamplerPadRelease extends Event {
  const Event_SamplerPadRelease(this.field0): super._();
  

@override final  SamplerPadRelease field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SamplerPadReleaseCopyWith<Event_SamplerPadRelease> get copyWith => _$Event_SamplerPadReleaseCopyWithImpl<Event_SamplerPadRelease>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SamplerPadRelease&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.samplerPadRelease(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SamplerPadReleaseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SamplerPadReleaseCopyWith(Event_SamplerPadRelease value, $Res Function(Event_SamplerPadRelease) _then) = _$Event_SamplerPadReleaseCopyWithImpl;
@useResult
$Res call({
 SamplerPadRelease field0
});




}
/// @nodoc
class _$Event_SamplerPadReleaseCopyWithImpl<$Res>
    implements $Event_SamplerPadReleaseCopyWith<$Res> {
  _$Event_SamplerPadReleaseCopyWithImpl(this._self, this._then);

  final Event_SamplerPadRelease _self;
  final $Res Function(Event_SamplerPadRelease) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SamplerPadRelease(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SamplerPadRelease,
  ));
}


}

/// @nodoc


class Event_KeyboardPadPress extends Event {
  const Event_KeyboardPadPress(this.field0): super._();
  

@override final  KeyboardPadPress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_KeyboardPadPressCopyWith<Event_KeyboardPadPress> get copyWith => _$Event_KeyboardPadPressCopyWithImpl<Event_KeyboardPadPress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_KeyboardPadPress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.keyboardPadPress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_KeyboardPadPressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_KeyboardPadPressCopyWith(Event_KeyboardPadPress value, $Res Function(Event_KeyboardPadPress) _then) = _$Event_KeyboardPadPressCopyWithImpl;
@useResult
$Res call({
 KeyboardPadPress field0
});




}
/// @nodoc
class _$Event_KeyboardPadPressCopyWithImpl<$Res>
    implements $Event_KeyboardPadPressCopyWith<$Res> {
  _$Event_KeyboardPadPressCopyWithImpl(this._self, this._then);

  final Event_KeyboardPadPress _self;
  final $Res Function(Event_KeyboardPadPress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_KeyboardPadPress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as KeyboardPadPress,
  ));
}


}

/// @nodoc


class Event_KeyboardPadRelease extends Event {
  const Event_KeyboardPadRelease(this.field0): super._();
  

@override final  KeyboardPadRelease field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_KeyboardPadReleaseCopyWith<Event_KeyboardPadRelease> get copyWith => _$Event_KeyboardPadReleaseCopyWithImpl<Event_KeyboardPadRelease>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_KeyboardPadRelease&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.keyboardPadRelease(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_KeyboardPadReleaseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_KeyboardPadReleaseCopyWith(Event_KeyboardPadRelease value, $Res Function(Event_KeyboardPadRelease) _then) = _$Event_KeyboardPadReleaseCopyWithImpl;
@useResult
$Res call({
 KeyboardPadRelease field0
});




}
/// @nodoc
class _$Event_KeyboardPadReleaseCopyWithImpl<$Res>
    implements $Event_KeyboardPadReleaseCopyWith<$Res> {
  _$Event_KeyboardPadReleaseCopyWithImpl(this._self, this._then);

  final Event_KeyboardPadRelease _self;
  final $Res Function(Event_KeyboardPadRelease) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_KeyboardPadRelease(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as KeyboardPadRelease,
  ));
}


}

/// @nodoc


class Event_KeyShiftPadPress extends Event {
  const Event_KeyShiftPadPress(this.field0): super._();
  

@override final  KeyShiftPadPress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_KeyShiftPadPressCopyWith<Event_KeyShiftPadPress> get copyWith => _$Event_KeyShiftPadPressCopyWithImpl<Event_KeyShiftPadPress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_KeyShiftPadPress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.keyShiftPadPress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_KeyShiftPadPressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_KeyShiftPadPressCopyWith(Event_KeyShiftPadPress value, $Res Function(Event_KeyShiftPadPress) _then) = _$Event_KeyShiftPadPressCopyWithImpl;
@useResult
$Res call({
 KeyShiftPadPress field0
});




}
/// @nodoc
class _$Event_KeyShiftPadPressCopyWithImpl<$Res>
    implements $Event_KeyShiftPadPressCopyWith<$Res> {
  _$Event_KeyShiftPadPressCopyWithImpl(this._self, this._then);

  final Event_KeyShiftPadPress _self;
  final $Res Function(Event_KeyShiftPadPress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_KeyShiftPadPress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as KeyShiftPadPress,
  ));
}


}

/// @nodoc


class Event_KeyShiftPadRelease extends Event {
  const Event_KeyShiftPadRelease(this.field0): super._();
  

@override final  KeyShiftPadRelease field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_KeyShiftPadReleaseCopyWith<Event_KeyShiftPadRelease> get copyWith => _$Event_KeyShiftPadReleaseCopyWithImpl<Event_KeyShiftPadRelease>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_KeyShiftPadRelease&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.keyShiftPadRelease(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_KeyShiftPadReleaseCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_KeyShiftPadReleaseCopyWith(Event_KeyShiftPadRelease value, $Res Function(Event_KeyShiftPadRelease) _then) = _$Event_KeyShiftPadReleaseCopyWithImpl;
@useResult
$Res call({
 KeyShiftPadRelease field0
});




}
/// @nodoc
class _$Event_KeyShiftPadReleaseCopyWithImpl<$Res>
    implements $Event_KeyShiftPadReleaseCopyWith<$Res> {
  _$Event_KeyShiftPadReleaseCopyWithImpl(this._self, this._then);

  final Event_KeyShiftPadRelease _self;
  final $Res Function(Event_KeyShiftPadRelease) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_KeyShiftPadRelease(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as KeyShiftPadRelease,
  ));
}


}

/// @nodoc


class Event_TriggerHotCue extends Event {
  const Event_TriggerHotCue(this.field0): super._();
  

@override final  TriggerHotCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_TriggerHotCueCopyWith<Event_TriggerHotCue> get copyWith => _$Event_TriggerHotCueCopyWithImpl<Event_TriggerHotCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_TriggerHotCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.triggerHotCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_TriggerHotCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_TriggerHotCueCopyWith(Event_TriggerHotCue value, $Res Function(Event_TriggerHotCue) _then) = _$Event_TriggerHotCueCopyWithImpl;
@useResult
$Res call({
 TriggerHotCue field0
});




}
/// @nodoc
class _$Event_TriggerHotCueCopyWithImpl<$Res>
    implements $Event_TriggerHotCueCopyWith<$Res> {
  _$Event_TriggerHotCueCopyWithImpl(this._self, this._then);

  final Event_TriggerHotCue _self;
  final $Res Function(Event_TriggerHotCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_TriggerHotCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as TriggerHotCue,
  ));
}


}

/// @nodoc


class Event_SaveDeckHotCue extends Event {
  const Event_SaveDeckHotCue(this.field0): super._();
  

@override final  SaveDeckHotCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SaveDeckHotCueCopyWith<Event_SaveDeckHotCue> get copyWith => _$Event_SaveDeckHotCueCopyWithImpl<Event_SaveDeckHotCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SaveDeckHotCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.saveDeckHotCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SaveDeckHotCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SaveDeckHotCueCopyWith(Event_SaveDeckHotCue value, $Res Function(Event_SaveDeckHotCue) _then) = _$Event_SaveDeckHotCueCopyWithImpl;
@useResult
$Res call({
 SaveDeckHotCue field0
});




}
/// @nodoc
class _$Event_SaveDeckHotCueCopyWithImpl<$Res>
    implements $Event_SaveDeckHotCueCopyWith<$Res> {
  _$Event_SaveDeckHotCueCopyWithImpl(this._self, this._then);

  final Event_SaveDeckHotCue _self;
  final $Res Function(Event_SaveDeckHotCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SaveDeckHotCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SaveDeckHotCue,
  ));
}


}

/// @nodoc


class Event_DeleteDeckHotCue extends Event {
  const Event_DeleteDeckHotCue(this.field0): super._();
  

@override final  DeleteDeckHotCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_DeleteDeckHotCueCopyWith<Event_DeleteDeckHotCue> get copyWith => _$Event_DeleteDeckHotCueCopyWithImpl<Event_DeleteDeckHotCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_DeleteDeckHotCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.deleteDeckHotCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_DeleteDeckHotCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_DeleteDeckHotCueCopyWith(Event_DeleteDeckHotCue value, $Res Function(Event_DeleteDeckHotCue) _then) = _$Event_DeleteDeckHotCueCopyWithImpl;
@useResult
$Res call({
 DeleteDeckHotCue field0
});




}
/// @nodoc
class _$Event_DeleteDeckHotCueCopyWithImpl<$Res>
    implements $Event_DeleteDeckHotCueCopyWith<$Res> {
  _$Event_DeleteDeckHotCueCopyWithImpl(this._self, this._then);

  final Event_DeleteDeckHotCue _self;
  final $Res Function(Event_DeleteDeckHotCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_DeleteDeckHotCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DeleteDeckHotCue,
  ));
}


}

/// @nodoc


class Event_AssignSampler extends Event {
  const Event_AssignSampler(this.field0): super._();
  

@override final  AssignSampler field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_AssignSamplerCopyWith<Event_AssignSampler> get copyWith => _$Event_AssignSamplerCopyWithImpl<Event_AssignSampler>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_AssignSampler&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.assignSampler(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_AssignSamplerCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_AssignSamplerCopyWith(Event_AssignSampler value, $Res Function(Event_AssignSampler) _then) = _$Event_AssignSamplerCopyWithImpl;
@useResult
$Res call({
 AssignSampler field0
});




}
/// @nodoc
class _$Event_AssignSamplerCopyWithImpl<$Res>
    implements $Event_AssignSamplerCopyWith<$Res> {
  _$Event_AssignSamplerCopyWithImpl(this._self, this._then);

  final Event_AssignSampler _self;
  final $Res Function(Event_AssignSampler) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_AssignSampler(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as AssignSampler,
  ));
}


}

/// @nodoc


class Event_AssignSamplerTrack extends Event {
  const Event_AssignSamplerTrack(this.field0): super._();
  

@override final  AssignSamplerTrack field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_AssignSamplerTrackCopyWith<Event_AssignSamplerTrack> get copyWith => _$Event_AssignSamplerTrackCopyWithImpl<Event_AssignSamplerTrack>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_AssignSamplerTrack&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.assignSamplerTrack(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_AssignSamplerTrackCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_AssignSamplerTrackCopyWith(Event_AssignSamplerTrack value, $Res Function(Event_AssignSamplerTrack) _then) = _$Event_AssignSamplerTrackCopyWithImpl;
@useResult
$Res call({
 AssignSamplerTrack field0
});




}
/// @nodoc
class _$Event_AssignSamplerTrackCopyWithImpl<$Res>
    implements $Event_AssignSamplerTrackCopyWith<$Res> {
  _$Event_AssignSamplerTrackCopyWithImpl(this._self, this._then);

  final Event_AssignSamplerTrack _self;
  final $Res Function(Event_AssignSamplerTrack) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_AssignSamplerTrack(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as AssignSamplerTrack,
  ));
}


}

/// @nodoc


class Event_ClearSampler extends Event {
  const Event_ClearSampler(this.field0): super._();
  

@override final  ClearSampler field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ClearSamplerCopyWith<Event_ClearSampler> get copyWith => _$Event_ClearSamplerCopyWithImpl<Event_ClearSampler>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_ClearSampler&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.clearSampler(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ClearSamplerCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ClearSamplerCopyWith(Event_ClearSampler value, $Res Function(Event_ClearSampler) _then) = _$Event_ClearSamplerCopyWithImpl;
@useResult
$Res call({
 ClearSampler field0
});




}
/// @nodoc
class _$Event_ClearSamplerCopyWithImpl<$Res>
    implements $Event_ClearSamplerCopyWith<$Res> {
  _$Event_ClearSamplerCopyWithImpl(this._self, this._then);

  final Event_ClearSampler _self;
  final $Res Function(Event_ClearSampler) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_ClearSampler(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ClearSampler,
  ));
}


}

/// @nodoc


class Event_JogTouch extends Event {
  const Event_JogTouch(this.field0): super._();
  

@override final  JogTouch field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_JogTouchCopyWith<Event_JogTouch> get copyWith => _$Event_JogTouchCopyWithImpl<Event_JogTouch>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_JogTouch&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.jogTouch(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_JogTouchCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_JogTouchCopyWith(Event_JogTouch value, $Res Function(Event_JogTouch) _then) = _$Event_JogTouchCopyWithImpl;
@useResult
$Res call({
 JogTouch field0
});




}
/// @nodoc
class _$Event_JogTouchCopyWithImpl<$Res>
    implements $Event_JogTouchCopyWith<$Res> {
  _$Event_JogTouchCopyWithImpl(this._self, this._then);

  final Event_JogTouch _self;
  final $Res Function(Event_JogTouch) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_JogTouch(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as JogTouch,
  ));
}


}

/// @nodoc


class Event_JogTurn extends Event {
  const Event_JogTurn(this.field0): super._();
  

@override final  JogTurn field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_JogTurnCopyWith<Event_JogTurn> get copyWith => _$Event_JogTurnCopyWithImpl<Event_JogTurn>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_JogTurn&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.jogTurn(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_JogTurnCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_JogTurnCopyWith(Event_JogTurn value, $Res Function(Event_JogTurn) _then) = _$Event_JogTurnCopyWithImpl;
@useResult
$Res call({
 JogTurn field0
});




}
/// @nodoc
class _$Event_JogTurnCopyWithImpl<$Res>
    implements $Event_JogTurnCopyWith<$Res> {
  _$Event_JogTurnCopyWithImpl(this._self, this._then);

  final Event_JogTurn _self;
  final $Res Function(Event_JogTurn) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_JogTurn(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as JogTurn,
  ));
}


}

/// @nodoc


class Event_SetJogMode extends Event {
  const Event_SetJogMode(this.field0): super._();
  

@override final  SetJogMode field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetJogModeCopyWith<Event_SetJogMode> get copyWith => _$Event_SetJogModeCopyWithImpl<Event_SetJogMode>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetJogMode&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setJogMode(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetJogModeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetJogModeCopyWith(Event_SetJogMode value, $Res Function(Event_SetJogMode) _then) = _$Event_SetJogModeCopyWithImpl;
@useResult
$Res call({
 SetJogMode field0
});




}
/// @nodoc
class _$Event_SetJogModeCopyWithImpl<$Res>
    implements $Event_SetJogModeCopyWith<$Res> {
  _$Event_SetJogModeCopyWithImpl(this._self, this._then);

  final Event_SetJogMode _self;
  final $Res Function(Event_SetJogMode) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetJogMode(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetJogMode,
  ));
}


}

/// @nodoc


class Event_SetCrossfader extends Event {
  const Event_SetCrossfader(this.field0): super._();
  

@override final  SetCrossfader field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetCrossfaderCopyWith<Event_SetCrossfader> get copyWith => _$Event_SetCrossfaderCopyWithImpl<Event_SetCrossfader>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetCrossfader&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setCrossfader(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetCrossfaderCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetCrossfaderCopyWith(Event_SetCrossfader value, $Res Function(Event_SetCrossfader) _then) = _$Event_SetCrossfaderCopyWithImpl;
@useResult
$Res call({
 SetCrossfader field0
});




}
/// @nodoc
class _$Event_SetCrossfaderCopyWithImpl<$Res>
    implements $Event_SetCrossfaderCopyWith<$Res> {
  _$Event_SetCrossfaderCopyWithImpl(this._self, this._then);

  final Event_SetCrossfader _self;
  final $Res Function(Event_SetCrossfader) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetCrossfader(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetCrossfader,
  ));
}


}

/// @nodoc


class Event_SetMasterVolume extends Event {
  const Event_SetMasterVolume(this.field0): super._();
  

@override final  SetMasterVolume field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetMasterVolumeCopyWith<Event_SetMasterVolume> get copyWith => _$Event_SetMasterVolumeCopyWithImpl<Event_SetMasterVolume>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetMasterVolume&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setMasterVolume(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetMasterVolumeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetMasterVolumeCopyWith(Event_SetMasterVolume value, $Res Function(Event_SetMasterVolume) _then) = _$Event_SetMasterVolumeCopyWithImpl;
@useResult
$Res call({
 SetMasterVolume field0
});




}
/// @nodoc
class _$Event_SetMasterVolumeCopyWithImpl<$Res>
    implements $Event_SetMasterVolumeCopyWith<$Res> {
  _$Event_SetMasterVolumeCopyWithImpl(this._self, this._then);

  final Event_SetMasterVolume _self;
  final $Res Function(Event_SetMasterVolume) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetMasterVolume(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetMasterVolume,
  ));
}


}

/// @nodoc


class Event_SetCueVolume extends Event {
  const Event_SetCueVolume(this.field0): super._();
  

@override final  SetCueVolume field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetCueVolumeCopyWith<Event_SetCueVolume> get copyWith => _$Event_SetCueVolumeCopyWithImpl<Event_SetCueVolume>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetCueVolume&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setCueVolume(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetCueVolumeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetCueVolumeCopyWith(Event_SetCueVolume value, $Res Function(Event_SetCueVolume) _then) = _$Event_SetCueVolumeCopyWithImpl;
@useResult
$Res call({
 SetCueVolume field0
});




}
/// @nodoc
class _$Event_SetCueVolumeCopyWithImpl<$Res>
    implements $Event_SetCueVolumeCopyWith<$Res> {
  _$Event_SetCueVolumeCopyWithImpl(this._self, this._then);

  final Event_SetCueVolume _self;
  final $Res Function(Event_SetCueVolume) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetCueVolume(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetCueVolume,
  ));
}


}

/// @nodoc


class Event_SetCueMix extends Event {
  const Event_SetCueMix(this.field0): super._();
  

@override final  SetCueMix field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetCueMixCopyWith<Event_SetCueMix> get copyWith => _$Event_SetCueMixCopyWithImpl<Event_SetCueMix>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetCueMix&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setCueMix(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetCueMixCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetCueMixCopyWith(Event_SetCueMix value, $Res Function(Event_SetCueMix) _then) = _$Event_SetCueMixCopyWithImpl;
@useResult
$Res call({
 SetCueMix field0
});




}
/// @nodoc
class _$Event_SetCueMixCopyWithImpl<$Res>
    implements $Event_SetCueMixCopyWith<$Res> {
  _$Event_SetCueMixCopyWithImpl(this._self, this._then);

  final Event_SetCueMix _self;
  final $Res Function(Event_SetCueMix) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetCueMix(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetCueMix,
  ));
}


}

/// @nodoc


class Event_SetMasterCue extends Event {
  const Event_SetMasterCue(this.field0): super._();
  

@override final  SetMasterCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetMasterCueCopyWith<Event_SetMasterCue> get copyWith => _$Event_SetMasterCueCopyWithImpl<Event_SetMasterCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetMasterCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setMasterCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetMasterCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetMasterCueCopyWith(Event_SetMasterCue value, $Res Function(Event_SetMasterCue) _then) = _$Event_SetMasterCueCopyWithImpl;
@useResult
$Res call({
 SetMasterCue field0
});




}
/// @nodoc
class _$Event_SetMasterCueCopyWithImpl<$Res>
    implements $Event_SetMasterCueCopyWith<$Res> {
  _$Event_SetMasterCueCopyWithImpl(this._self, this._then);

  final Event_SetMasterCue _self;
  final $Res Function(Event_SetMasterCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetMasterCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetMasterCue,
  ));
}


}

/// @nodoc


class Event_ToggleMasterCue extends Event {
  const Event_ToggleMasterCue(this.field0): super._();
  

@override final  ToggleMasterCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ToggleMasterCueCopyWith<Event_ToggleMasterCue> get copyWith => _$Event_ToggleMasterCueCopyWithImpl<Event_ToggleMasterCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_ToggleMasterCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.toggleMasterCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ToggleMasterCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ToggleMasterCueCopyWith(Event_ToggleMasterCue value, $Res Function(Event_ToggleMasterCue) _then) = _$Event_ToggleMasterCueCopyWithImpl;
@useResult
$Res call({
 ToggleMasterCue field0
});




}
/// @nodoc
class _$Event_ToggleMasterCueCopyWithImpl<$Res>
    implements $Event_ToggleMasterCueCopyWith<$Res> {
  _$Event_ToggleMasterCueCopyWithImpl(this._self, this._then);

  final Event_ToggleMasterCue _self;
  final $Res Function(Event_ToggleMasterCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_ToggleMasterCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ToggleMasterCue,
  ));
}


}

/// @nodoc


class Event_SetFxUnitSlot extends Event {
  const Event_SetFxUnitSlot(this.field0): super._();
  

@override final  SetFxUnitSlot field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetFxUnitSlotCopyWith<Event_SetFxUnitSlot> get copyWith => _$Event_SetFxUnitSlotCopyWithImpl<Event_SetFxUnitSlot>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetFxUnitSlot&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setFxUnitSlot(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetFxUnitSlotCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetFxUnitSlotCopyWith(Event_SetFxUnitSlot value, $Res Function(Event_SetFxUnitSlot) _then) = _$Event_SetFxUnitSlotCopyWithImpl;
@useResult
$Res call({
 SetFxUnitSlot field0
});




}
/// @nodoc
class _$Event_SetFxUnitSlotCopyWithImpl<$Res>
    implements $Event_SetFxUnitSlotCopyWith<$Res> {
  _$Event_SetFxUnitSlotCopyWithImpl(this._self, this._then);

  final Event_SetFxUnitSlot _self;
  final $Res Function(Event_SetFxUnitSlot) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetFxUnitSlot(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetFxUnitSlot,
  ));
}


}

/// @nodoc


class Event_SetFxUnitSlotEnabled extends Event {
  const Event_SetFxUnitSlotEnabled(this.field0): super._();
  

@override final  SetFxUnitSlotEnabled field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetFxUnitSlotEnabledCopyWith<Event_SetFxUnitSlotEnabled> get copyWith => _$Event_SetFxUnitSlotEnabledCopyWithImpl<Event_SetFxUnitSlotEnabled>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetFxUnitSlotEnabled&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setFxUnitSlotEnabled(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetFxUnitSlotEnabledCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetFxUnitSlotEnabledCopyWith(Event_SetFxUnitSlotEnabled value, $Res Function(Event_SetFxUnitSlotEnabled) _then) = _$Event_SetFxUnitSlotEnabledCopyWithImpl;
@useResult
$Res call({
 SetFxUnitSlotEnabled field0
});




}
/// @nodoc
class _$Event_SetFxUnitSlotEnabledCopyWithImpl<$Res>
    implements $Event_SetFxUnitSlotEnabledCopyWith<$Res> {
  _$Event_SetFxUnitSlotEnabledCopyWithImpl(this._self, this._then);

  final Event_SetFxUnitSlotEnabled _self;
  final $Res Function(Event_SetFxUnitSlotEnabled) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetFxUnitSlotEnabled(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetFxUnitSlotEnabled,
  ));
}


}

/// @nodoc


class Event_SetFxUnitWet extends Event {
  const Event_SetFxUnitWet(this.field0): super._();
  

@override final  SetFxUnitWet field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetFxUnitWetCopyWith<Event_SetFxUnitWet> get copyWith => _$Event_SetFxUnitWetCopyWithImpl<Event_SetFxUnitWet>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetFxUnitWet&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setFxUnitWet(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetFxUnitWetCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetFxUnitWetCopyWith(Event_SetFxUnitWet value, $Res Function(Event_SetFxUnitWet) _then) = _$Event_SetFxUnitWetCopyWithImpl;
@useResult
$Res call({
 SetFxUnitWet field0
});




}
/// @nodoc
class _$Event_SetFxUnitWetCopyWithImpl<$Res>
    implements $Event_SetFxUnitWetCopyWith<$Res> {
  _$Event_SetFxUnitWetCopyWithImpl(this._self, this._then);

  final Event_SetFxUnitWet _self;
  final $Res Function(Event_SetFxUnitWet) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetFxUnitWet(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetFxUnitWet,
  ));
}


}

/// @nodoc


class Event_SetFxUnitParam extends Event {
  const Event_SetFxUnitParam(this.field0): super._();
  

@override final  SetFxUnitParam field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetFxUnitParamCopyWith<Event_SetFxUnitParam> get copyWith => _$Event_SetFxUnitParamCopyWithImpl<Event_SetFxUnitParam>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetFxUnitParam&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setFxUnitParam(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetFxUnitParamCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetFxUnitParamCopyWith(Event_SetFxUnitParam value, $Res Function(Event_SetFxUnitParam) _then) = _$Event_SetFxUnitParamCopyWithImpl;
@useResult
$Res call({
 SetFxUnitParam field0
});




}
/// @nodoc
class _$Event_SetFxUnitParamCopyWithImpl<$Res>
    implements $Event_SetFxUnitParamCopyWith<$Res> {
  _$Event_SetFxUnitParamCopyWithImpl(this._self, this._then);

  final Event_SetFxUnitParam _self;
  final $Res Function(Event_SetFxUnitParam) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetFxUnitParam(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetFxUnitParam,
  ));
}


}

/// @nodoc


class Event_SetDeckFxRouted extends Event {
  const Event_SetDeckFxRouted(this.field0): super._();
  

@override final  SetDeckFxRouted field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SetDeckFxRoutedCopyWith<Event_SetDeckFxRouted> get copyWith => _$Event_SetDeckFxRoutedCopyWithImpl<Event_SetDeckFxRouted>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SetDeckFxRouted&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.setDeckFxRouted(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SetDeckFxRoutedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SetDeckFxRoutedCopyWith(Event_SetDeckFxRouted value, $Res Function(Event_SetDeckFxRouted) _then) = _$Event_SetDeckFxRoutedCopyWithImpl;
@useResult
$Res call({
 SetDeckFxRouted field0
});




}
/// @nodoc
class _$Event_SetDeckFxRoutedCopyWithImpl<$Res>
    implements $Event_SetDeckFxRoutedCopyWith<$Res> {
  _$Event_SetDeckFxRoutedCopyWithImpl(this._self, this._then);

  final Event_SetDeckFxRouted _self;
  final $Res Function(Event_SetDeckFxRouted) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SetDeckFxRouted(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SetDeckFxRouted,
  ));
}


}

/// @nodoc


class Event_DeckUpdated extends Event {
  const Event_DeckUpdated(this.field0): super._();
  

@override final  DeckUpdated field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_DeckUpdatedCopyWith<Event_DeckUpdated> get copyWith => _$Event_DeckUpdatedCopyWithImpl<Event_DeckUpdated>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_DeckUpdated&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.deckUpdated(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_DeckUpdatedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_DeckUpdatedCopyWith(Event_DeckUpdated value, $Res Function(Event_DeckUpdated) _then) = _$Event_DeckUpdatedCopyWithImpl;
@useResult
$Res call({
 DeckUpdated field0
});




}
/// @nodoc
class _$Event_DeckUpdatedCopyWithImpl<$Res>
    implements $Event_DeckUpdatedCopyWith<$Res> {
  _$Event_DeckUpdatedCopyWithImpl(this._self, this._then);

  final Event_DeckUpdated _self;
  final $Res Function(Event_DeckUpdated) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_DeckUpdated(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DeckUpdated,
  ));
}


}

/// @nodoc


class Event_DeckPosition extends Event {
  const Event_DeckPosition(this.field0): super._();
  

@override final  DeckPosition field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_DeckPositionCopyWith<Event_DeckPosition> get copyWith => _$Event_DeckPositionCopyWithImpl<Event_DeckPosition>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_DeckPosition&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.deckPosition(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_DeckPositionCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_DeckPositionCopyWith(Event_DeckPosition value, $Res Function(Event_DeckPosition) _then) = _$Event_DeckPositionCopyWithImpl;
@useResult
$Res call({
 DeckPosition field0
});




}
/// @nodoc
class _$Event_DeckPositionCopyWithImpl<$Res>
    implements $Event_DeckPositionCopyWith<$Res> {
  _$Event_DeckPositionCopyWithImpl(this._self, this._then);

  final Event_DeckPosition _self;
  final $Res Function(Event_DeckPosition) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_DeckPosition(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DeckPosition,
  ));
}


}

/// @nodoc


class Event_DeckLevels extends Event {
  const Event_DeckLevels(this.field0): super._();
  

@override final  DeckLevels field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_DeckLevelsCopyWith<Event_DeckLevels> get copyWith => _$Event_DeckLevelsCopyWithImpl<Event_DeckLevels>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_DeckLevels&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.deckLevels(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_DeckLevelsCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_DeckLevelsCopyWith(Event_DeckLevels value, $Res Function(Event_DeckLevels) _then) = _$Event_DeckLevelsCopyWithImpl;
@useResult
$Res call({
 DeckLevels field0
});




}
/// @nodoc
class _$Event_DeckLevelsCopyWithImpl<$Res>
    implements $Event_DeckLevelsCopyWith<$Res> {
  _$Event_DeckLevelsCopyWithImpl(this._self, this._then);

  final Event_DeckLevels _self;
  final $Res Function(Event_DeckLevels) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_DeckLevels(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DeckLevels,
  ));
}


}

/// @nodoc


class Event_EngineSnapshot extends Event {
  const Event_EngineSnapshot(this.field0): super._();
  

@override final  EngineSnapshot field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_EngineSnapshotCopyWith<Event_EngineSnapshot> get copyWith => _$Event_EngineSnapshotCopyWithImpl<Event_EngineSnapshot>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_EngineSnapshot&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.engineSnapshot(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_EngineSnapshotCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_EngineSnapshotCopyWith(Event_EngineSnapshot value, $Res Function(Event_EngineSnapshot) _then) = _$Event_EngineSnapshotCopyWithImpl;
@useResult
$Res call({
 EngineSnapshot field0
});




}
/// @nodoc
class _$Event_EngineSnapshotCopyWithImpl<$Res>
    implements $Event_EngineSnapshotCopyWith<$Res> {
  _$Event_EngineSnapshotCopyWithImpl(this._self, this._then);

  final Event_EngineSnapshot _self;
  final $Res Function(Event_EngineSnapshot) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_EngineSnapshot(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as EngineSnapshot,
  ));
}


}

/// @nodoc


class Event_Error extends Event {
  const Event_Error(this.field0): super._();
  

@override final  Error field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_ErrorCopyWith<Event_Error> get copyWith => _$Event_ErrorCopyWithImpl<Event_Error>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Error&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.error(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_ErrorCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_ErrorCopyWith(Event_Error value, $Res Function(Event_Error) _then) = _$Event_ErrorCopyWithImpl;
@useResult
$Res call({
 Error field0
});




}
/// @nodoc
class _$Event_ErrorCopyWithImpl<$Res>
    implements $Event_ErrorCopyWith<$Res> {
  _$Event_ErrorCopyWithImpl(this._self, this._then);

  final Event_Error _self;
  final $Res Function(Event_Error) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Error(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Error,
  ));
}


}

/// @nodoc


class Event_Notice extends Event {
  const Event_Notice(this.field0): super._();
  

@override final  Notice field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_NoticeCopyWith<Event_Notice> get copyWith => _$Event_NoticeCopyWithImpl<Event_Notice>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Notice&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.notice(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_NoticeCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_NoticeCopyWith(Event_Notice value, $Res Function(Event_Notice) _then) = _$Event_NoticeCopyWithImpl;
@useResult
$Res call({
 Notice field0
});




}
/// @nodoc
class _$Event_NoticeCopyWithImpl<$Res>
    implements $Event_NoticeCopyWith<$Res> {
  _$Event_NoticeCopyWithImpl(this._self, this._then);

  final Event_Notice _self;
  final $Res Function(Event_Notice) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Notice(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Notice,
  ));
}


}

/// @nodoc


class Event_AnalyzeTrack extends Event {
  const Event_AnalyzeTrack(this.field0): super._();
  

@override final  AnalyzeTrack field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_AnalyzeTrackCopyWith<Event_AnalyzeTrack> get copyWith => _$Event_AnalyzeTrackCopyWithImpl<Event_AnalyzeTrack>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_AnalyzeTrack&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.analyzeTrack(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_AnalyzeTrackCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_AnalyzeTrackCopyWith(Event_AnalyzeTrack value, $Res Function(Event_AnalyzeTrack) _then) = _$Event_AnalyzeTrackCopyWithImpl;
@useResult
$Res call({
 AnalyzeTrack field0
});




}
/// @nodoc
class _$Event_AnalyzeTrackCopyWithImpl<$Res>
    implements $Event_AnalyzeTrackCopyWith<$Res> {
  _$Event_AnalyzeTrackCopyWithImpl(this._self, this._then);

  final Event_AnalyzeTrack _self;
  final $Res Function(Event_AnalyzeTrack) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_AnalyzeTrack(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as AnalyzeTrack,
  ));
}


}

/// @nodoc


class Event_GenerateStems extends Event {
  const Event_GenerateStems(this.field0): super._();
  

@override final  GenerateStems field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_GenerateStemsCopyWith<Event_GenerateStems> get copyWith => _$Event_GenerateStemsCopyWithImpl<Event_GenerateStems>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_GenerateStems&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.generateStems(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_GenerateStemsCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_GenerateStemsCopyWith(Event_GenerateStems value, $Res Function(Event_GenerateStems) _then) = _$Event_GenerateStemsCopyWithImpl;
@useResult
$Res call({
 GenerateStems field0
});




}
/// @nodoc
class _$Event_GenerateStemsCopyWithImpl<$Res>
    implements $Event_GenerateStemsCopyWith<$Res> {
  _$Event_GenerateStemsCopyWithImpl(this._self, this._then);

  final Event_GenerateStems _self;
  final $Res Function(Event_GenerateStems) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_GenerateStems(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as GenerateStems,
  ));
}


}

/// @nodoc


class Event_RefreshTrack extends Event {
  const Event_RefreshTrack(this.field0): super._();
  

@override final  RefreshTrack field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_RefreshTrackCopyWith<Event_RefreshTrack> get copyWith => _$Event_RefreshTrackCopyWithImpl<Event_RefreshTrack>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_RefreshTrack&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.refreshTrack(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_RefreshTrackCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_RefreshTrackCopyWith(Event_RefreshTrack value, $Res Function(Event_RefreshTrack) _then) = _$Event_RefreshTrackCopyWithImpl;
@useResult
$Res call({
 RefreshTrack field0
});




}
/// @nodoc
class _$Event_RefreshTrackCopyWithImpl<$Res>
    implements $Event_RefreshTrackCopyWith<$Res> {
  _$Event_RefreshTrackCopyWithImpl(this._self, this._then);

  final Event_RefreshTrack _self;
  final $Res Function(Event_RefreshTrack) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_RefreshTrack(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as RefreshTrack,
  ));
}


}

/// @nodoc


class Event_SaveBeatGrid extends Event {
  const Event_SaveBeatGrid(this.field0): super._();
  

@override final  SaveBeatGrid field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SaveBeatGridCopyWith<Event_SaveBeatGrid> get copyWith => _$Event_SaveBeatGridCopyWithImpl<Event_SaveBeatGrid>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SaveBeatGrid&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.saveBeatGrid(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SaveBeatGridCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SaveBeatGridCopyWith(Event_SaveBeatGrid value, $Res Function(Event_SaveBeatGrid) _then) = _$Event_SaveBeatGridCopyWithImpl;
@useResult
$Res call({
 SaveBeatGrid field0
});




}
/// @nodoc
class _$Event_SaveBeatGridCopyWithImpl<$Res>
    implements $Event_SaveBeatGridCopyWith<$Res> {
  _$Event_SaveBeatGridCopyWithImpl(this._self, this._then);

  final Event_SaveBeatGrid _self;
  final $Res Function(Event_SaveBeatGrid) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SaveBeatGrid(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SaveBeatGrid,
  ));
}


}

/// @nodoc


class Event_SaveTrackHotCue extends Event {
  const Event_SaveTrackHotCue(this.field0): super._();
  

@override final  SaveTrackHotCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SaveTrackHotCueCopyWith<Event_SaveTrackHotCue> get copyWith => _$Event_SaveTrackHotCueCopyWithImpl<Event_SaveTrackHotCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SaveTrackHotCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.saveTrackHotCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SaveTrackHotCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SaveTrackHotCueCopyWith(Event_SaveTrackHotCue value, $Res Function(Event_SaveTrackHotCue) _then) = _$Event_SaveTrackHotCueCopyWithImpl;
@useResult
$Res call({
 SaveTrackHotCue field0
});




}
/// @nodoc
class _$Event_SaveTrackHotCueCopyWithImpl<$Res>
    implements $Event_SaveTrackHotCueCopyWith<$Res> {
  _$Event_SaveTrackHotCueCopyWithImpl(this._self, this._then);

  final Event_SaveTrackHotCue _self;
  final $Res Function(Event_SaveTrackHotCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SaveTrackHotCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SaveTrackHotCue,
  ));
}


}

/// @nodoc


class Event_DeleteTrackHotCue extends Event {
  const Event_DeleteTrackHotCue(this.field0): super._();
  

@override final  DeleteTrackHotCue field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_DeleteTrackHotCueCopyWith<Event_DeleteTrackHotCue> get copyWith => _$Event_DeleteTrackHotCueCopyWithImpl<Event_DeleteTrackHotCue>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_DeleteTrackHotCue&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.deleteTrackHotCue(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_DeleteTrackHotCueCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_DeleteTrackHotCueCopyWith(Event_DeleteTrackHotCue value, $Res Function(Event_DeleteTrackHotCue) _then) = _$Event_DeleteTrackHotCueCopyWithImpl;
@useResult
$Res call({
 DeleteTrackHotCue field0
});




}
/// @nodoc
class _$Event_DeleteTrackHotCueCopyWithImpl<$Res>
    implements $Event_DeleteTrackHotCueCopyWith<$Res> {
  _$Event_DeleteTrackHotCueCopyWithImpl(this._self, this._then);

  final Event_DeleteTrackHotCue _self;
  final $Res Function(Event_DeleteTrackHotCue) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_DeleteTrackHotCue(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DeleteTrackHotCue,
  ));
}


}

/// @nodoc


class Event_SaveTrackLoop extends Event {
  const Event_SaveTrackLoop(this.field0): super._();
  

@override final  SaveTrackLoop field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_SaveTrackLoopCopyWith<Event_SaveTrackLoop> get copyWith => _$Event_SaveTrackLoopCopyWithImpl<Event_SaveTrackLoop>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_SaveTrackLoop&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.saveTrackLoop(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_SaveTrackLoopCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_SaveTrackLoopCopyWith(Event_SaveTrackLoop value, $Res Function(Event_SaveTrackLoop) _then) = _$Event_SaveTrackLoopCopyWithImpl;
@useResult
$Res call({
 SaveTrackLoop field0
});




}
/// @nodoc
class _$Event_SaveTrackLoopCopyWithImpl<$Res>
    implements $Event_SaveTrackLoopCopyWith<$Res> {
  _$Event_SaveTrackLoopCopyWithImpl(this._self, this._then);

  final Event_SaveTrackLoop _self;
  final $Res Function(Event_SaveTrackLoop) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_SaveTrackLoop(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SaveTrackLoop,
  ));
}


}

/// @nodoc


class Event_DeleteTrackLoop extends Event {
  const Event_DeleteTrackLoop(this.field0): super._();
  

@override final  DeleteTrackLoop field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_DeleteTrackLoopCopyWith<Event_DeleteTrackLoop> get copyWith => _$Event_DeleteTrackLoopCopyWithImpl<Event_DeleteTrackLoop>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_DeleteTrackLoop&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.deleteTrackLoop(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_DeleteTrackLoopCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_DeleteTrackLoopCopyWith(Event_DeleteTrackLoop value, $Res Function(Event_DeleteTrackLoop) _then) = _$Event_DeleteTrackLoopCopyWithImpl;
@useResult
$Res call({
 DeleteTrackLoop field0
});




}
/// @nodoc
class _$Event_DeleteTrackLoopCopyWithImpl<$Res>
    implements $Event_DeleteTrackLoopCopyWith<$Res> {
  _$Event_DeleteTrackLoopCopyWithImpl(this._self, this._then);

  final Event_DeleteTrackLoop _self;
  final $Res Function(Event_DeleteTrackLoop) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_DeleteTrackLoop(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DeleteTrackLoop,
  ));
}


}

/// @nodoc


class Event_TrackAnalyzed extends Event {
  const Event_TrackAnalyzed(this.field0): super._();
  

@override final  TrackAnalyzed field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_TrackAnalyzedCopyWith<Event_TrackAnalyzed> get copyWith => _$Event_TrackAnalyzedCopyWithImpl<Event_TrackAnalyzed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_TrackAnalyzed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.trackAnalyzed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_TrackAnalyzedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_TrackAnalyzedCopyWith(Event_TrackAnalyzed value, $Res Function(Event_TrackAnalyzed) _then) = _$Event_TrackAnalyzedCopyWithImpl;
@useResult
$Res call({
 TrackAnalyzed field0
});




}
/// @nodoc
class _$Event_TrackAnalyzedCopyWithImpl<$Res>
    implements $Event_TrackAnalyzedCopyWith<$Res> {
  _$Event_TrackAnalyzedCopyWithImpl(this._self, this._then);

  final Event_TrackAnalyzed _self;
  final $Res Function(Event_TrackAnalyzed) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_TrackAnalyzed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as TrackAnalyzed,
  ));
}


}

/// @nodoc


class Event_TrackUpdated extends Event {
  const Event_TrackUpdated(this.field0): super._();
  

@override final  TrackUpdated field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_TrackUpdatedCopyWith<Event_TrackUpdated> get copyWith => _$Event_TrackUpdatedCopyWithImpl<Event_TrackUpdated>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_TrackUpdated&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.trackUpdated(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_TrackUpdatedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_TrackUpdatedCopyWith(Event_TrackUpdated value, $Res Function(Event_TrackUpdated) _then) = _$Event_TrackUpdatedCopyWithImpl;
@useResult
$Res call({
 TrackUpdated field0
});




}
/// @nodoc
class _$Event_TrackUpdatedCopyWithImpl<$Res>
    implements $Event_TrackUpdatedCopyWith<$Res> {
  _$Event_TrackUpdatedCopyWithImpl(this._self, this._then);

  final Event_TrackUpdated _self;
  final $Res Function(Event_TrackUpdated) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_TrackUpdated(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as TrackUpdated,
  ));
}


}

/// @nodoc


class Event_TrackProgress extends Event {
  const Event_TrackProgress(this.field0): super._();
  

@override final  TrackProgress field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_TrackProgressCopyWith<Event_TrackProgress> get copyWith => _$Event_TrackProgressCopyWithImpl<Event_TrackProgress>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_TrackProgress&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.trackProgress(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_TrackProgressCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_TrackProgressCopyWith(Event_TrackProgress value, $Res Function(Event_TrackProgress) _then) = _$Event_TrackProgressCopyWithImpl;
@useResult
$Res call({
 TrackProgress field0
});




}
/// @nodoc
class _$Event_TrackProgressCopyWithImpl<$Res>
    implements $Event_TrackProgressCopyWith<$Res> {
  _$Event_TrackProgressCopyWithImpl(this._self, this._then);

  final Event_TrackProgress _self;
  final $Res Function(Event_TrackProgress) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_TrackProgress(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as TrackProgress,
  ));
}


}

/// @nodoc


class Event_HotCuesChanged extends Event {
  const Event_HotCuesChanged(this.field0): super._();
  

@override final  HotCuesChanged field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_HotCuesChangedCopyWith<Event_HotCuesChanged> get copyWith => _$Event_HotCuesChangedCopyWithImpl<Event_HotCuesChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_HotCuesChanged&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.hotCuesChanged(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_HotCuesChangedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_HotCuesChangedCopyWith(Event_HotCuesChanged value, $Res Function(Event_HotCuesChanged) _then) = _$Event_HotCuesChangedCopyWithImpl;
@useResult
$Res call({
 HotCuesChanged field0
});




}
/// @nodoc
class _$Event_HotCuesChangedCopyWithImpl<$Res>
    implements $Event_HotCuesChangedCopyWith<$Res> {
  _$Event_HotCuesChangedCopyWithImpl(this._self, this._then);

  final Event_HotCuesChanged _self;
  final $Res Function(Event_HotCuesChanged) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_HotCuesChanged(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as HotCuesChanged,
  ));
}


}

/// @nodoc


class Event_LoopsChanged extends Event {
  const Event_LoopsChanged(this.field0): super._();
  

@override final  LoopsChanged field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_LoopsChangedCopyWith<Event_LoopsChanged> get copyWith => _$Event_LoopsChangedCopyWithImpl<Event_LoopsChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_LoopsChanged&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.loopsChanged(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_LoopsChangedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_LoopsChangedCopyWith(Event_LoopsChanged value, $Res Function(Event_LoopsChanged) _then) = _$Event_LoopsChangedCopyWithImpl;
@useResult
$Res call({
 LoopsChanged field0
});




}
/// @nodoc
class _$Event_LoopsChangedCopyWithImpl<$Res>
    implements $Event_LoopsChangedCopyWith<$Res> {
  _$Event_LoopsChangedCopyWithImpl(this._self, this._then);

  final Event_LoopsChanged _self;
  final $Res Function(Event_LoopsChanged) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_LoopsChanged(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as LoopsChanged,
  ));
}


}

/// @nodoc


class Event_BeatGridChanged extends Event {
  const Event_BeatGridChanged(this.field0): super._();
  

@override final  BeatGridChanged field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_BeatGridChangedCopyWith<Event_BeatGridChanged> get copyWith => _$Event_BeatGridChangedCopyWithImpl<Event_BeatGridChanged>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_BeatGridChanged&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.beatGridChanged(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_BeatGridChangedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_BeatGridChangedCopyWith(Event_BeatGridChanged value, $Res Function(Event_BeatGridChanged) _then) = _$Event_BeatGridChangedCopyWithImpl;
@useResult
$Res call({
 BeatGridChanged field0
});




}
/// @nodoc
class _$Event_BeatGridChangedCopyWithImpl<$Res>
    implements $Event_BeatGridChangedCopyWith<$Res> {
  _$Event_BeatGridChangedCopyWithImpl(this._self, this._then);

  final Event_BeatGridChanged _self;
  final $Res Function(Event_BeatGridChanged) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_BeatGridChanged(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as BeatGridChanged,
  ));
}


}

/// @nodoc


class Event_HistorySessionUpdated extends Event {
  const Event_HistorySessionUpdated(this.field0): super._();
  

@override final  HistorySessionUpdated field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_HistorySessionUpdatedCopyWith<Event_HistorySessionUpdated> get copyWith => _$Event_HistorySessionUpdatedCopyWithImpl<Event_HistorySessionUpdated>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_HistorySessionUpdated&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.historySessionUpdated(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_HistorySessionUpdatedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_HistorySessionUpdatedCopyWith(Event_HistorySessionUpdated value, $Res Function(Event_HistorySessionUpdated) _then) = _$Event_HistorySessionUpdatedCopyWithImpl;
@useResult
$Res call({
 HistorySessionUpdated field0
});




}
/// @nodoc
class _$Event_HistorySessionUpdatedCopyWithImpl<$Res>
    implements $Event_HistorySessionUpdatedCopyWith<$Res> {
  _$Event_HistorySessionUpdatedCopyWithImpl(this._self, this._then);

  final Event_HistorySessionUpdated _self;
  final $Res Function(Event_HistorySessionUpdated) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_HistorySessionUpdated(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as HistorySessionUpdated,
  ));
}


}

/// @nodoc


class Event_Navigate extends Event {
  const Event_Navigate(this.field0): super._();
  

@override final  Navigate field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_NavigateCopyWith<Event_Navigate> get copyWith => _$Event_NavigateCopyWithImpl<Event_Navigate>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Navigate&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.navigate(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_NavigateCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_NavigateCopyWith(Event_Navigate value, $Res Function(Event_Navigate) _then) = _$Event_NavigateCopyWithImpl;
@useResult
$Res call({
 Navigate field0
});




}
/// @nodoc
class _$Event_NavigateCopyWithImpl<$Res>
    implements $Event_NavigateCopyWith<$Res> {
  _$Event_NavigateCopyWithImpl(this._self, this._then);

  final Event_Navigate _self;
  final $Res Function(Event_Navigate) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Navigate(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Navigate,
  ));
}


}

/// @nodoc


class Event_Focus extends Event {
  const Event_Focus(this.field0): super._();
  

@override final  Focus field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_FocusCopyWith<Event_Focus> get copyWith => _$Event_FocusCopyWithImpl<Event_Focus>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Focus&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.focus(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_FocusCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_FocusCopyWith(Event_Focus value, $Res Function(Event_Focus) _then) = _$Event_FocusCopyWithImpl;
@useResult
$Res call({
 Focus field0
});




}
/// @nodoc
class _$Event_FocusCopyWithImpl<$Res>
    implements $Event_FocusCopyWith<$Res> {
  _$Event_FocusCopyWithImpl(this._self, this._then);

  final Event_Focus _self;
  final $Res Function(Event_Focus) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Focus(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Focus,
  ));
}


}

/// @nodoc


class Event_Load extends Event {
  const Event_Load(this.field0): super._();
  

@override final  Load field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_LoadCopyWith<Event_Load> get copyWith => _$Event_LoadCopyWithImpl<Event_Load>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_Load&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.load(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_LoadCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_LoadCopyWith(Event_Load value, $Res Function(Event_Load) _then) = _$Event_LoadCopyWithImpl;
@useResult
$Res call({
 Load field0
});




}
/// @nodoc
class _$Event_LoadCopyWithImpl<$Res>
    implements $Event_LoadCopyWith<$Res> {
  _$Event_LoadCopyWithImpl(this._self, this._then);

  final Event_Load _self;
  final $Res Function(Event_Load) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_Load(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Load,
  ));
}


}

// dart format on
