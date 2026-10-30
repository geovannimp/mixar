use std::sync::Arc;

/// Identity of the stateful thing an event talks about.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Entity {
    Engine,
    Mixer,
    Deck(u16),
    Library,
    Track(String),
    Session(String),
    Controller,
    ControllerPort(String),
    Ui,
}

/// Sub-target inside an entity. Keeps two setters for different slots, bands or
/// params from sharing one pending slot.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Whole,
    Index(u8),
    Pair(u8, u8),
    Name(Arc<str>),
}

/// Full pending-queue identity of one event.
///
/// Two events share a pending slot only when entity, topic and part all match,
/// so `SetEqBand(low)` and `SetEqBand(mid)` on the same deck are never folded
/// into each other.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventKey<T> {
    pub entity: Entity,
    pub topic: T,
    pub part: Part,
}

impl<T> EventKey<T> {
    pub fn new(entity: Entity, topic: T, part: Part) -> Self {
        Self {
            entity,
            topic,
            part,
        }
    }
}
