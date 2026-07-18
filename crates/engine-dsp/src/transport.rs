//! Transport events raised by deck playback (end-of-track, etc.).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckTransportEvent {
    TrackEnded,
}
