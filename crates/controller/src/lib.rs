//! MIDI controller mapping bundles for the engine cmd/evt bus.

mod action;
mod action_id;
mod bundle;
mod catalog;
mod device;
mod engine;
mod error;
mod map_file;
mod midi;
mod session;

pub mod check;
pub mod script;

pub use action::{resolve_action, ControlValue, RoutedAction, HOT_CUE_SLOT_COUNT};
pub use action_id::{
    bind_origin, format_bound_action, parse_action_id, ActionArgs, ArgValue, BoundOrigin,
    OriginTemplate,
};
pub use bundle::{load_bundle, MappingBundle};
pub use catalog::{is_closed_input_alias, is_known_action, ActionName};
pub use check::check_bundle_dir;
pub use device::{
    AudioHints, DeviceFile, SectionName, TomlSchemaRef, SECTION_CUSTOM, SECTION_MASTER,
    SECTION_SAMPLER,
};
pub use engine::{
    ControllerEngine, ControllerEvent, DeviceDirection, DeviceInfo, EngineError, MappingInfo,
};
pub use error::{LoadError, MidiPortError, RuntimeError};
pub use map_file::{
    InputBinding, LifecycleHooks, MapFile, OutputBinding, OutputTarget, RawBinding,
    SoftTakeoverDefault,
};
pub use midi::{
    decode_relative, match_device, norm_from_cc14, parse_short, CcField, Direction, MidiEndpoint,
    MidiIdentity, MidiMsgType, ParsedMidi, RelativeMode, ShortMsg,
};
pub use session::{ActionPublish, BusPublish, MappingSession, MidiOut, MidiPort};
