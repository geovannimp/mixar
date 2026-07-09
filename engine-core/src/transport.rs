//! Engine-level transport notifications (track ended, etc.).

use engine_dsp::DeckTransportEvent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportEvent {
    TrackEnded { deck_id: usize },
}

impl TransportEvent {
    pub fn from_deck(deck_id: usize, event: DeckTransportEvent) -> Self {
        match event {
            DeckTransportEvent::TrackEnded => Self::TrackEnded { deck_id },
        }
    }
}
