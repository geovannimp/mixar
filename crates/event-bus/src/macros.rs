//! The `bus_events!` table macro.
//!
//! One row per event. The macro emits the `Topic` enum, one payload struct per
//! row, `EventSpec` for each payload, the closed `Event` enum and
//! `BusMessage`/`From` impls for it. Entity and part are ordinary expressions
//! evaluated against `&self` inside `key()`, so a row can compute its key
//! conditionally (`match self.session_id { … }`) without special-casing the
//! macro.
//!
//! ```ignore
//! bus_events! {
//!     Volume: Update, Entity::Deck(*deck), Part::Whole, { deck: u16, volume: f32 };
//!     Jog:    Delta,  Entity::Deck(*deck), Part::Whole, { deck: u16, delta: i32 }, fold: delta;
//! }
//! ```
//!
//! `$entity`/`$part` are expressions written against the row's own field names:
//! the macro destructures the payload with those idents, so `deck` is `&u16`
//! and a row binds the value with `*deck` (or clones a heap field). Identifiers
//! the macro never received cannot resolve — `self.deck` is not available here.
//!
//! The generated names (`Topic`, `Event`) are fixed: one invocation per crate.
//!
//! A `Delta` row must name the field that accumulates (`fold:`); without it
//! `merge` folds nothing and the deltas are lost, so every `Delta` row in the
//! table carries one.

#[macro_export]
macro_rules! bus_events {
    (
        $(
            $(#[$meta:meta])*
            $name:ident : $policy:ident ,
            $entity:expr ,
            $part:expr ,
            { $( $field:ident : $ty:ty ),* $(,)? }
            $(, fold: $fold:ident)?
        );
        * $(;)?
    ) => {
        /// Discriminator of every event type. Used by topic-filtered subscriptions.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Topic {
            $( $name ),*
        }

        $(
            $(#[$meta])*
            #[derive(Clone, Debug, PartialEq)]
            pub struct $name {
                $( pub $field: $ty ),*
            }

            impl $crate::EventSpec for $name {
                const POLICY: $crate::Policy = $crate::Policy::$policy;
                type Topic = Topic;
                const TOPIC: Topic = Topic::$name;

                fn key(&self) -> ::core::option::Option<$crate::EventKey<Topic>> {
                    // Destructure with the row's own field idents: captured
                    // expressions resolve at the call site, so the row's
                    // `deck`/`band`/… only resolve to a binding carrying the
                    // same token. Fields not used in `$entity`/`$part` are
                    // simply unused here.
                    #[allow(unused_variables)]
                    let Self { $( $field ),* } = self;
                    ::core::option::Option::Some($crate::EventKey::new(
                        $entity,
                        Topic::$name,
                        $part,
                    ))
                }

                fn merge(&mut self, later: &Self) {
                    if ::core::matches!($crate::Policy::$policy, $crate::Policy::Delta) {
                        $( self.$fold += later.$fold; )?
                    } else {
                        *self = later.clone();
                    }
                }
            }
        )*

        /// Every event that can travel on the bus.
        #[derive(Clone, Debug, PartialEq)]
        pub enum Event {
            $( $name($name) ),*
        }

        impl $crate::BusMessage for Event {
            type Topic = Topic;

            fn topic(&self) -> Topic {
                match self {
                    $( Self::$name(_) => Topic::$name, )*
                }
            }

            fn policy(&self) -> $crate::Policy {
                match self {
                    $( Self::$name(_) => $crate::Policy::$policy, )*
                }
            }

            fn key(&self) -> ::core::option::Option<$crate::EventKey<Topic>> {
                match self {
                    $( Self::$name(event) => $crate::EventSpec::key(event), )*
                }
            }

            fn merge(&mut self, later: &Self) {
                match (self, later) {
                    $(
                        ( Self::$name(pending), Self::$name(newer) ) => {
                            $crate::EventSpec::merge(pending, newer)
                        }
                    )*
                    // A key carries its topic, so only same-variant merges reach
                    // `merge`; folding across variants is not a real case.
                    _ => {}
                }
            }
        }

        $(
            impl ::core::convert::From<$name> for Event {
                fn from(event: $name) -> Self {
                    Self::$name(event)
                }
            }
        )*
    };
}
