//! Bookkeeping for the Android JNI context handoff.
//!
//! [`crate::android`] does the JNI work; this module owns the decision state, so
//! the retry and reporting rules are testable without a JVM. It is compiled for
//! Android and for tests only: the host build has no JNI context to track, and
//! keeping it out of that target leaves nothing dead behind.
//!
//! `ndk-context` asserts it is initialized exactly once, so a success must stop
//! every later attempt. A failure must not: `MainActivity.onCreate` runs again
//! when the activity is recreated, and that second call is the only chance to
//! recover from a JVM that was not ready the first time.

/// How one attempt to hand the context to `ndk-context` ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextInit {
    /// The JavaVM and the Application context were stored for `ndk-context`.
    Initialized,
    /// Nothing usable reached `ndk-context`; the JNI-backed hosts will panic.
    Failed(&'static str),
}

/// What [`crate::android`] has learned so far.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ContextState {
    /// Set only after `ndk-context` accepted the context.
    initialized: bool,
    /// The last attempt's answer, for the application log.
    outcome: Option<ContextInit>,
}

impl ContextState {
    /// Const constructor for the process-wide static.
    pub(crate) const fn new() -> Self {
        Self {
            initialized: false,
            outcome: None,
        }
    }

    /// Whether the context still has to be handed to `ndk-context`.
    pub(crate) fn needs_init(&self) -> bool {
        !self.initialized
    }

    /// Records one attempt: only a success closes the door on the next one.
    pub(crate) fn record(&mut self, outcome: ContextInit) {
        self.initialized = outcome == ContextInit::Initialized;
        self.outcome = Some(outcome);
    }

    /// The last attempt's answer, if there was one.
    pub(crate) fn outcome(&self) -> Option<ContextInit> {
        self.outcome
    }
}

/// What the application log should say about the handoff.
///
/// Kept out of [`crate::android`] so the mapping is decided — and tested — on
/// the host; that module only emits the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Report {
    /// The context reached `ndk-context`.
    Initialized,
    /// An attempt failed, with the reason.
    Missing(&'static str),
    /// `MainActivity` never called in, so nothing was attempted.
    NeverAttempted,
}

/// Decides which line [`crate::android::log_init_outcome`] writes.
pub(crate) fn report(outcome: Option<ContextInit>) -> Report {
    match outcome {
        Some(ContextInit::Initialized) => Report::Initialized,
        Some(ContextInit::Failed(reason)) => Report::Missing(reason),
        None => Report::NeverAttempted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_successful_attempt_is_never_repeated() {
        let mut state = ContextState::new();
        assert!(state.needs_init());

        state.record(ContextInit::Initialized);

        // `ndk-context` asserts on a second initialization, so this has to stay
        // false for the rest of the process.
        assert!(!state.needs_init());
    }

    #[test]
    fn a_failed_attempt_leaves_room_for_the_next_activity() {
        let mut state = ContextState::new();

        state.record(ContextInit::Failed("the JVM could not be resolved"));

        // A recreated activity must be able to try again instead of leaving the
        // process without a context for good.
        assert!(state.needs_init());
        assert_eq!(
            state.outcome(),
            Some(ContextInit::Failed("the JVM could not be resolved"))
        );
    }

    #[test]
    fn the_last_attempt_is_what_gets_reported() {
        let mut state = ContextState::new();

        state.record(ContextInit::Failed("the context could not be pinned"));
        state.record(ContextInit::Initialized);

        // A first failure followed by a success must not be reported as a
        // failure in `mixar.log`.
        assert_eq!(state.outcome(), Some(ContextInit::Initialized));
        assert!(!state.needs_init());
    }

    #[test]
    fn a_success_is_reported_as_initialized() {
        assert_eq!(report(Some(ContextInit::Initialized)), Report::Initialized);
    }

    #[test]
    fn a_failure_is_reported_with_its_reason() {
        assert_eq!(
            report(Some(ContextInit::Failed("the JVM could not be resolved"))),
            Report::Missing("the JVM could not be resolved")
        );
    }

    #[test]
    fn no_attempt_is_reported_as_never_attempted() {
        // Distinct from a failure: nothing was tried, so `MainActivity` is the
        // place to look, not the JVM.
        assert_eq!(report(None), Report::NeverAttempted);
    }
}
