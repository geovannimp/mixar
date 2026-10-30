use crate::entity::Entity;
use crate::message::BusMessage;
use crate::subscriber::Subscriber;
use crate::BusError;
use omnibus::Filter;

/// One process-wide cmd/evt pair over a closed event enum.
///
/// `omnibus` publishes into every matching subscriber channel; `EventBus` adds
/// the typed payload (no encode/decode), the topic filter shorthand and the
/// merge-policy pending queue inside [`Subscriber`].
///
/// The omnibus *origin* dimension is unused: it is always [`Entity::Ui`] and
/// subscriptions discriminate by topic. Keeping the dimension typed costs
/// nothing and leaves room to split by origin later.
///
/// Cheap to clone — every clone shares both buses.
pub struct EventBus<M: BusMessage> {
    cmd: omnibus::Bus<Entity, M::Topic, M>,
    evt: omnibus::Bus<Entity, M::Topic, M>,
}

impl<M: BusMessage> Clone for EventBus<M> {
    fn clone(&self) -> Self {
        Self {
            cmd: self.cmd.clone(),
            evt: self.evt.clone(),
        }
    }
}

impl<M: BusMessage> EventBus<M> {
    /// Create a matched cmd/evt pair. `cmd_capacity` / `evt_capacity` are the
    /// per-subscriber channel depths: overflow drops the newest event for that
    /// subscriber, so the evt side wants headroom for high-rate position/levels.
    pub fn new(cmd_capacity: usize, evt_capacity: usize) -> Self {
        Self {
            cmd: omnibus::Bus::with_capacity(cmd_capacity),
            evt: omnibus::Bus::with_capacity(evt_capacity),
        }
    }

    /// Fire-and-forget publish of a command.
    pub fn publish_cmd(&self, event: M) -> Result<(), BusError> {
        self.publish(&self.cmd, event)
    }

    /// Fire-and-forget publish of an egress event.
    pub fn publish_evt(&self, event: M) -> Result<(), BusError> {
        self.publish(&self.evt, event)
    }

    /// Subscribe to every command (unfiltered, so `Toggle` folding applies).
    pub fn subscribe_cmd(&self) -> Result<Subscriber<M>, BusError> {
        self.subscriber(&self.cmd, false, Filter::Any)
    }

    /// Subscribe to every egress event (unfiltered, so `Toggle` folding applies).
    pub fn subscribe_evt(&self) -> Result<Subscriber<M>, BusError> {
        self.subscriber(&self.evt, false, Filter::Any)
    }

    /// Subscribe to one topic. A filtered subscriber never folds `Toggle`:
    /// events that would separate two toggles are filtered out, so a pair looks
    /// adjacent when it is not.
    pub fn subscribe_evt_topic(&self, topic: M::Topic) -> Result<Subscriber<M>, BusError> {
        self.subscriber(&self.evt, true, Filter::Is(topic))
    }

    fn publish(&self, bus: &omnibus::Bus<Entity, M::Topic, M>, event: M) -> Result<(), BusError> {
        bus.publish(omnibus::Event::new(Entity::Ui, event.topic(), event))?;
        Ok(())
    }

    fn subscriber(
        &self,
        bus: &omnibus::Bus<Entity, M::Topic, M>,
        filtered: bool,
        topic: Filter<M::Topic>,
    ) -> Result<Subscriber<M>, BusError> {
        Ok(Subscriber::new(
            bus.subscribe(Filter::Any, topic)?,
            filtered,
        ))
    }
}
