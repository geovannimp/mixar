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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Event_DeckUpdated value)?  deckUpdated,TResult Function( Event_DeckPosition value)?  deckPosition,TResult Function( Event_DeckLevels value)?  deckLevels,TResult Function( Event_EngineSnapshot value)?  engineSnapshot,TResult Function( Event_Error value)?  error,TResult Function( Event_Notice value)?  notice,TResult Function( Event_TrackAnalyzed value)?  trackAnalyzed,TResult Function( Event_TrackUpdated value)?  trackUpdated,TResult Function( Event_TrackProgress value)?  trackProgress,TResult Function( Event_HotCuesChanged value)?  hotCuesChanged,TResult Function( Event_LoopsChanged value)?  loopsChanged,TResult Function( Event_BeatGridChanged value)?  beatGridChanged,TResult Function( Event_HistorySessionUpdated value)?  historySessionUpdated,TResult Function( Event_Navigate value)?  navigate,TResult Function( Event_Focus value)?  focus,TResult Function( Event_Load value)?  load,TResult Function( Event_MappingOffer value)?  mappingOffer,TResult Function( Event_MappingAttached value)?  mappingAttached,TResult Function( Event_MappingDetached value)?  mappingDetached,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that);case Event_Error() when error != null:
return error(_that);case Event_Notice() when notice != null:
return notice(_that);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that);case Event_Navigate() when navigate != null:
return navigate(_that);case Event_Focus() when focus != null:
return focus(_that);case Event_Load() when load != null:
return load(_that);case Event_MappingOffer() when mappingOffer != null:
return mappingOffer(_that);case Event_MappingAttached() when mappingAttached != null:
return mappingAttached(_that);case Event_MappingDetached() when mappingDetached != null:
return mappingDetached(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Event_DeckUpdated value)  deckUpdated,required TResult Function( Event_DeckPosition value)  deckPosition,required TResult Function( Event_DeckLevels value)  deckLevels,required TResult Function( Event_EngineSnapshot value)  engineSnapshot,required TResult Function( Event_Error value)  error,required TResult Function( Event_Notice value)  notice,required TResult Function( Event_TrackAnalyzed value)  trackAnalyzed,required TResult Function( Event_TrackUpdated value)  trackUpdated,required TResult Function( Event_TrackProgress value)  trackProgress,required TResult Function( Event_HotCuesChanged value)  hotCuesChanged,required TResult Function( Event_LoopsChanged value)  loopsChanged,required TResult Function( Event_BeatGridChanged value)  beatGridChanged,required TResult Function( Event_HistorySessionUpdated value)  historySessionUpdated,required TResult Function( Event_Navigate value)  navigate,required TResult Function( Event_Focus value)  focus,required TResult Function( Event_Load value)  load,required TResult Function( Event_MappingOffer value)  mappingOffer,required TResult Function( Event_MappingAttached value)  mappingAttached,required TResult Function( Event_MappingDetached value)  mappingDetached,}){
final _that = this;
switch (_that) {
case Event_DeckUpdated():
return deckUpdated(_that);case Event_DeckPosition():
return deckPosition(_that);case Event_DeckLevels():
return deckLevels(_that);case Event_EngineSnapshot():
return engineSnapshot(_that);case Event_Error():
return error(_that);case Event_Notice():
return notice(_that);case Event_TrackAnalyzed():
return trackAnalyzed(_that);case Event_TrackUpdated():
return trackUpdated(_that);case Event_TrackProgress():
return trackProgress(_that);case Event_HotCuesChanged():
return hotCuesChanged(_that);case Event_LoopsChanged():
return loopsChanged(_that);case Event_BeatGridChanged():
return beatGridChanged(_that);case Event_HistorySessionUpdated():
return historySessionUpdated(_that);case Event_Navigate():
return navigate(_that);case Event_Focus():
return focus(_that);case Event_Load():
return load(_that);case Event_MappingOffer():
return mappingOffer(_that);case Event_MappingAttached():
return mappingAttached(_that);case Event_MappingDetached():
return mappingDetached(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Event_DeckUpdated value)?  deckUpdated,TResult? Function( Event_DeckPosition value)?  deckPosition,TResult? Function( Event_DeckLevels value)?  deckLevels,TResult? Function( Event_EngineSnapshot value)?  engineSnapshot,TResult? Function( Event_Error value)?  error,TResult? Function( Event_Notice value)?  notice,TResult? Function( Event_TrackAnalyzed value)?  trackAnalyzed,TResult? Function( Event_TrackUpdated value)?  trackUpdated,TResult? Function( Event_TrackProgress value)?  trackProgress,TResult? Function( Event_HotCuesChanged value)?  hotCuesChanged,TResult? Function( Event_LoopsChanged value)?  loopsChanged,TResult? Function( Event_BeatGridChanged value)?  beatGridChanged,TResult? Function( Event_HistorySessionUpdated value)?  historySessionUpdated,TResult? Function( Event_Navigate value)?  navigate,TResult? Function( Event_Focus value)?  focus,TResult? Function( Event_Load value)?  load,TResult? Function( Event_MappingOffer value)?  mappingOffer,TResult? Function( Event_MappingAttached value)?  mappingAttached,TResult? Function( Event_MappingDetached value)?  mappingDetached,}){
final _that = this;
switch (_that) {
case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that);case Event_Error() when error != null:
return error(_that);case Event_Notice() when notice != null:
return notice(_that);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that);case Event_Navigate() when navigate != null:
return navigate(_that);case Event_Focus() when focus != null:
return focus(_that);case Event_Load() when load != null:
return load(_that);case Event_MappingOffer() when mappingOffer != null:
return mappingOffer(_that);case Event_MappingAttached() when mappingAttached != null:
return mappingAttached(_that);case Event_MappingDetached() when mappingDetached != null:
return mappingDetached(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( DeckUpdated field0)?  deckUpdated,TResult Function( DeckPosition field0)?  deckPosition,TResult Function( DeckLevels field0)?  deckLevels,TResult Function( EngineSnapshot field0)?  engineSnapshot,TResult Function( Error field0)?  error,TResult Function( Notice field0)?  notice,TResult Function( TrackAnalyzed field0)?  trackAnalyzed,TResult Function( TrackUpdated field0)?  trackUpdated,TResult Function( TrackProgress field0)?  trackProgress,TResult Function( HotCuesChanged field0)?  hotCuesChanged,TResult Function( LoopsChanged field0)?  loopsChanged,TResult Function( BeatGridChanged field0)?  beatGridChanged,TResult Function( HistorySessionUpdated field0)?  historySessionUpdated,TResult Function( Navigate field0)?  navigate,TResult Function( Focus field0)?  focus,TResult Function( Load field0)?  load,TResult Function( MappingOffer field0)?  mappingOffer,TResult Function( MappingAttached field0)?  mappingAttached,TResult Function( MappingDetached field0)?  mappingDetached,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that.field0);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that.field0);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that.field0);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that.field0);case Event_Error() when error != null:
return error(_that.field0);case Event_Notice() when notice != null:
return notice(_that.field0);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that.field0);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that.field0);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that.field0);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that.field0);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that.field0);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that.field0);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that.field0);case Event_Navigate() when navigate != null:
return navigate(_that.field0);case Event_Focus() when focus != null:
return focus(_that.field0);case Event_Load() when load != null:
return load(_that.field0);case Event_MappingOffer() when mappingOffer != null:
return mappingOffer(_that.field0);case Event_MappingAttached() when mappingAttached != null:
return mappingAttached(_that.field0);case Event_MappingDetached() when mappingDetached != null:
return mappingDetached(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( DeckUpdated field0)  deckUpdated,required TResult Function( DeckPosition field0)  deckPosition,required TResult Function( DeckLevels field0)  deckLevels,required TResult Function( EngineSnapshot field0)  engineSnapshot,required TResult Function( Error field0)  error,required TResult Function( Notice field0)  notice,required TResult Function( TrackAnalyzed field0)  trackAnalyzed,required TResult Function( TrackUpdated field0)  trackUpdated,required TResult Function( TrackProgress field0)  trackProgress,required TResult Function( HotCuesChanged field0)  hotCuesChanged,required TResult Function( LoopsChanged field0)  loopsChanged,required TResult Function( BeatGridChanged field0)  beatGridChanged,required TResult Function( HistorySessionUpdated field0)  historySessionUpdated,required TResult Function( Navigate field0)  navigate,required TResult Function( Focus field0)  focus,required TResult Function( Load field0)  load,required TResult Function( MappingOffer field0)  mappingOffer,required TResult Function( MappingAttached field0)  mappingAttached,required TResult Function( MappingDetached field0)  mappingDetached,}) {final _that = this;
switch (_that) {
case Event_DeckUpdated():
return deckUpdated(_that.field0);case Event_DeckPosition():
return deckPosition(_that.field0);case Event_DeckLevels():
return deckLevels(_that.field0);case Event_EngineSnapshot():
return engineSnapshot(_that.field0);case Event_Error():
return error(_that.field0);case Event_Notice():
return notice(_that.field0);case Event_TrackAnalyzed():
return trackAnalyzed(_that.field0);case Event_TrackUpdated():
return trackUpdated(_that.field0);case Event_TrackProgress():
return trackProgress(_that.field0);case Event_HotCuesChanged():
return hotCuesChanged(_that.field0);case Event_LoopsChanged():
return loopsChanged(_that.field0);case Event_BeatGridChanged():
return beatGridChanged(_that.field0);case Event_HistorySessionUpdated():
return historySessionUpdated(_that.field0);case Event_Navigate():
return navigate(_that.field0);case Event_Focus():
return focus(_that.field0);case Event_Load():
return load(_that.field0);case Event_MappingOffer():
return mappingOffer(_that.field0);case Event_MappingAttached():
return mappingAttached(_that.field0);case Event_MappingDetached():
return mappingDetached(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( DeckUpdated field0)?  deckUpdated,TResult? Function( DeckPosition field0)?  deckPosition,TResult? Function( DeckLevels field0)?  deckLevels,TResult? Function( EngineSnapshot field0)?  engineSnapshot,TResult? Function( Error field0)?  error,TResult? Function( Notice field0)?  notice,TResult? Function( TrackAnalyzed field0)?  trackAnalyzed,TResult? Function( TrackUpdated field0)?  trackUpdated,TResult? Function( TrackProgress field0)?  trackProgress,TResult? Function( HotCuesChanged field0)?  hotCuesChanged,TResult? Function( LoopsChanged field0)?  loopsChanged,TResult? Function( BeatGridChanged field0)?  beatGridChanged,TResult? Function( HistorySessionUpdated field0)?  historySessionUpdated,TResult? Function( Navigate field0)?  navigate,TResult? Function( Focus field0)?  focus,TResult? Function( Load field0)?  load,TResult? Function( MappingOffer field0)?  mappingOffer,TResult? Function( MappingAttached field0)?  mappingAttached,TResult? Function( MappingDetached field0)?  mappingDetached,}) {final _that = this;
switch (_that) {
case Event_DeckUpdated() when deckUpdated != null:
return deckUpdated(_that.field0);case Event_DeckPosition() when deckPosition != null:
return deckPosition(_that.field0);case Event_DeckLevels() when deckLevels != null:
return deckLevels(_that.field0);case Event_EngineSnapshot() when engineSnapshot != null:
return engineSnapshot(_that.field0);case Event_Error() when error != null:
return error(_that.field0);case Event_Notice() when notice != null:
return notice(_that.field0);case Event_TrackAnalyzed() when trackAnalyzed != null:
return trackAnalyzed(_that.field0);case Event_TrackUpdated() when trackUpdated != null:
return trackUpdated(_that.field0);case Event_TrackProgress() when trackProgress != null:
return trackProgress(_that.field0);case Event_HotCuesChanged() when hotCuesChanged != null:
return hotCuesChanged(_that.field0);case Event_LoopsChanged() when loopsChanged != null:
return loopsChanged(_that.field0);case Event_BeatGridChanged() when beatGridChanged != null:
return beatGridChanged(_that.field0);case Event_HistorySessionUpdated() when historySessionUpdated != null:
return historySessionUpdated(_that.field0);case Event_Navigate() when navigate != null:
return navigate(_that.field0);case Event_Focus() when focus != null:
return focus(_that.field0);case Event_Load() when load != null:
return load(_that.field0);case Event_MappingOffer() when mappingOffer != null:
return mappingOffer(_that.field0);case Event_MappingAttached() when mappingAttached != null:
return mappingAttached(_that.field0);case Event_MappingDetached() when mappingDetached != null:
return mappingDetached(_that.field0);case _:
  return null;

}
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

/// @nodoc


class Event_MappingOffer extends Event {
  const Event_MappingOffer(this.field0): super._();
  

@override final  MappingOffer field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_MappingOfferCopyWith<Event_MappingOffer> get copyWith => _$Event_MappingOfferCopyWithImpl<Event_MappingOffer>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_MappingOffer&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.mappingOffer(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_MappingOfferCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_MappingOfferCopyWith(Event_MappingOffer value, $Res Function(Event_MappingOffer) _then) = _$Event_MappingOfferCopyWithImpl;
@useResult
$Res call({
 MappingOffer field0
});




}
/// @nodoc
class _$Event_MappingOfferCopyWithImpl<$Res>
    implements $Event_MappingOfferCopyWith<$Res> {
  _$Event_MappingOfferCopyWithImpl(this._self, this._then);

  final Event_MappingOffer _self;
  final $Res Function(Event_MappingOffer) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_MappingOffer(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MappingOffer,
  ));
}


}

/// @nodoc


class Event_MappingAttached extends Event {
  const Event_MappingAttached(this.field0): super._();
  

@override final  MappingAttached field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_MappingAttachedCopyWith<Event_MappingAttached> get copyWith => _$Event_MappingAttachedCopyWithImpl<Event_MappingAttached>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_MappingAttached&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.mappingAttached(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_MappingAttachedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_MappingAttachedCopyWith(Event_MappingAttached value, $Res Function(Event_MappingAttached) _then) = _$Event_MappingAttachedCopyWithImpl;
@useResult
$Res call({
 MappingAttached field0
});




}
/// @nodoc
class _$Event_MappingAttachedCopyWithImpl<$Res>
    implements $Event_MappingAttachedCopyWith<$Res> {
  _$Event_MappingAttachedCopyWithImpl(this._self, this._then);

  final Event_MappingAttached _self;
  final $Res Function(Event_MappingAttached) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_MappingAttached(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MappingAttached,
  ));
}


}

/// @nodoc


class Event_MappingDetached extends Event {
  const Event_MappingDetached(this.field0): super._();
  

@override final  MappingDetached field0;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Event_MappingDetachedCopyWith<Event_MappingDetached> get copyWith => _$Event_MappingDetachedCopyWithImpl<Event_MappingDetached>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Event_MappingDetached&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Event.mappingDetached(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Event_MappingDetachedCopyWith<$Res> implements $EventCopyWith<$Res> {
  factory $Event_MappingDetachedCopyWith(Event_MappingDetached value, $Res Function(Event_MappingDetached) _then) = _$Event_MappingDetachedCopyWithImpl;
@useResult
$Res call({
 MappingDetached field0
});




}
/// @nodoc
class _$Event_MappingDetachedCopyWithImpl<$Res>
    implements $Event_MappingDetachedCopyWith<$Res> {
  _$Event_MappingDetachedCopyWithImpl(this._self, this._then);

  final Event_MappingDetached _self;
  final $Res Function(Event_MappingDetached) _then;

/// Create a copy of Event
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Event_MappingDetached(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MappingDetached,
  ));
}


}

// dart format on
