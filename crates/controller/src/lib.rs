//! MIDI controller mapping bundles for the engine cmd/evt bus.

mod action;
mod bundle;
mod catalog;
mod device;
mod error;
mod map_file;
mod midi;
mod session;

pub mod check;
pub mod script;

pub use action::resolve_action;
pub use bundle::{load_bundle, Bundle};
pub use catalog::{is_closed_input_alias, is_known_action, ActionName};
pub use check::check_bundle_dir;
pub use device::{
    AudioHints, DeviceFile, SectionName, SECTION_CUSTOM, SECTION_MASTER, SECTION_SAMPLER,
};
pub use error::{LoadError, MidiPortError, RuntimeError};
pub use map_file::{
    InputBinding, MapFile, OutputBinding, OutputTarget, RawBinding, SoftTakeoverDefault,
};
pub use midi::{
    match_device, norm_from_cc14, parse_short, CcField, Direction, MidiEndpoint, MidiIdentity,
    MidiMsgType, ParsedMidi, ShortMsg,
};
pub use session::{BusPublish, MappingSession, MidiOut, MidiPort};
