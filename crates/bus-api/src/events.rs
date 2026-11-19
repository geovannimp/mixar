//! The egress direction: every event the engine publishes.
//!
//! A command cannot be published here and an event cannot be published as a
//! command — the two enums are disjoint, so a wrong-direction publication does
//! not compile.

use crate::messages::{
    BeatGridChanged, DeckLevels, DeckPosition, DeckUpdated, EngineSnapshot, Error, Focus,
    HistorySessionUpdated, HotCuesChanged, Load, LoopsChanged, MappingAttached, MappingDetached,
    MappingOffer, Navigate, Notice, TrackAnalyzed, TrackProgress, TrackUpdated,
};
use event_bus::Message;

/// Discriminator of every event type: what a topic-filtered subscription selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventTopic {
    DeckUpdated,
    DeckPosition,
    DeckLevels,
    EngineSnapshot,
    Error,
    Notice,
    TrackAnalyzed,
    TrackUpdated,
    TrackProgress,
    HotCuesChanged,
    LoopsChanged,
    BeatGridChanged,
    HistorySessionUpdated,
    Navigate,
    Focus,
    Load,
    MappingOffer,
    MappingAttached,
    MappingDetached,
}

/// Every event that can travel on the bus.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    DeckUpdated(DeckUpdated),
    DeckPosition(DeckPosition),
    DeckLevels(DeckLevels),
    EngineSnapshot(EngineSnapshot),
    Error(Error),
    Notice(Notice),
    TrackAnalyzed(TrackAnalyzed),
    TrackUpdated(TrackUpdated),
    TrackProgress(TrackProgress),
    HotCuesChanged(HotCuesChanged),
    LoopsChanged(LoopsChanged),
    BeatGridChanged(BeatGridChanged),
    HistorySessionUpdated(HistorySessionUpdated),
    Navigate(Navigate),
    Focus(Focus),
    Load(Load),
    MappingOffer(MappingOffer),
    MappingAttached(MappingAttached),
    MappingDetached(MappingDetached),
}

impl Event {
    /// The topic this event belongs to.
    pub fn topic(&self) -> EventTopic {
        match self {
            Self::DeckUpdated(_) => EventTopic::DeckUpdated,
            Self::DeckPosition(_) => EventTopic::DeckPosition,
            Self::DeckLevels(_) => EventTopic::DeckLevels,
            Self::EngineSnapshot(_) => EventTopic::EngineSnapshot,
            Self::Error(_) => EventTopic::Error,
            Self::Notice(_) => EventTopic::Notice,
            Self::TrackAnalyzed(_) => EventTopic::TrackAnalyzed,
            Self::TrackUpdated(_) => EventTopic::TrackUpdated,
            Self::TrackProgress(_) => EventTopic::TrackProgress,
            Self::HotCuesChanged(_) => EventTopic::HotCuesChanged,
            Self::LoopsChanged(_) => EventTopic::LoopsChanged,
            Self::BeatGridChanged(_) => EventTopic::BeatGridChanged,
            Self::HistorySessionUpdated(_) => EventTopic::HistorySessionUpdated,
            Self::Navigate(_) => EventTopic::Navigate,
            Self::Focus(_) => EventTopic::Focus,
            Self::Load(_) => EventTopic::Load,
            Self::MappingOffer(_) => EventTopic::MappingOffer,
            Self::MappingAttached(_) => EventTopic::MappingAttached,
            Self::MappingDetached(_) => EventTopic::MappingDetached,
        }
    }
}

impl Message for Event {
    type Topic = EventTopic;

    fn topic(&self) -> Self::Topic {
        Event::topic(self)
    }
}

impl From<DeckUpdated> for Event {
    fn from(payload: DeckUpdated) -> Self {
        Self::DeckUpdated(payload)
    }
}

impl From<DeckPosition> for Event {
    fn from(payload: DeckPosition) -> Self {
        Self::DeckPosition(payload)
    }
}

impl From<DeckLevels> for Event {
    fn from(payload: DeckLevels) -> Self {
        Self::DeckLevels(payload)
    }
}

impl From<EngineSnapshot> for Event {
    fn from(payload: EngineSnapshot) -> Self {
        Self::EngineSnapshot(payload)
    }
}

impl From<Error> for Event {
    fn from(payload: Error) -> Self {
        Self::Error(payload)
    }
}

impl From<Notice> for Event {
    fn from(payload: Notice) -> Self {
        Self::Notice(payload)
    }
}

impl From<TrackAnalyzed> for Event {
    fn from(payload: TrackAnalyzed) -> Self {
        Self::TrackAnalyzed(payload)
    }
}

impl From<TrackUpdated> for Event {
    fn from(payload: TrackUpdated) -> Self {
        Self::TrackUpdated(payload)
    }
}

impl From<TrackProgress> for Event {
    fn from(payload: TrackProgress) -> Self {
        Self::TrackProgress(payload)
    }
}

impl From<HotCuesChanged> for Event {
    fn from(payload: HotCuesChanged) -> Self {
        Self::HotCuesChanged(payload)
    }
}

impl From<LoopsChanged> for Event {
    fn from(payload: LoopsChanged) -> Self {
        Self::LoopsChanged(payload)
    }
}

impl From<BeatGridChanged> for Event {
    fn from(payload: BeatGridChanged) -> Self {
        Self::BeatGridChanged(payload)
    }
}

impl From<HistorySessionUpdated> for Event {
    fn from(payload: HistorySessionUpdated) -> Self {
        Self::HistorySessionUpdated(payload)
    }
}

impl From<Navigate> for Event {
    fn from(payload: Navigate) -> Self {
        Self::Navigate(payload)
    }
}

impl From<Focus> for Event {
    fn from(payload: Focus) -> Self {
        Self::Focus(payload)
    }
}

impl From<Load> for Event {
    fn from(payload: Load) -> Self {
        Self::Load(payload)
    }
}

impl From<MappingOffer> for Event {
    fn from(payload: MappingOffer) -> Self {
        Self::MappingOffer(payload)
    }
}

impl From<MappingAttached> for Event {
    fn from(payload: MappingAttached) -> Self {
        Self::MappingAttached(payload)
    }
}

impl From<MappingDetached> for Event {
    fn from(payload: MappingDetached) -> Self {
        Self::MappingDetached(payload)
    }
}
