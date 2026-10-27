//! The single conversion from Rust errors to the bridge's `String` errors.
//!
//! Every `Result` crossing FRB used to spell `map_err(|e| e.to_string())`, which
//! turned the cause into a UI notice and left `mixar.log` with nothing — a user
//! reporting "the engine failed to start and I have no logs" had no way to see why.
//! `bridge()` logs once, here, and keeps the message Dart receives.
//!
//! Panics never reach this path (FRB catches them before the return value exists);
//! [`crate::api::handler`] logs those.

/// Convert a `Result` into the bridge's error type, logging the cause first.
pub(crate) trait BridgeResult<T> {
    /// Log the error at ERROR and convert it to the message the bridge returns.
    fn bridge(self) -> Result<T, String>;
}

impl<T, E> BridgeResult<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn bridge(self) -> Result<T, String> {
        self.map_err(|error| {
            tracing::error!(error = %error, "bridge call failed");
            error.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<u8>>>);

    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Sink {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Captured {
        type Writer = Sink;
        fn make_writer(&'a self) -> Self::Writer {
            Sink(Arc::clone(&self.0))
        }
    }

    #[test]
    fn bridge_preserves_the_message_and_logs_it() {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .with_ansi(false)
            .with_writer(captured.clone())
            .finish();

        let outcome = tracing::subscriber::with_default(subscriber, || {
            Err::<(), _>(anyhow::anyhow!(
                "Device 'Remote Audio' rejected 48000 Hz F32"
            ))
            .bridge()
        });

        assert_eq!(
            outcome.unwrap_err(),
            "Device 'Remote Audio' rejected 48000 Hz F32",
            "Dart must keep getting the same message"
        );
        let logged = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        assert!(logged.contains("bridge call failed"), "{logged}");
        assert!(logged.contains("rejected 48000 Hz F32"), "{logged}");
    }

    #[test]
    fn bridge_leaves_ok_untouched_and_logs_nothing() {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .with_ansi(false)
            .with_writer(captured.clone())
            .finish();

        let outcome = tracing::subscriber::with_default(subscriber, || Ok::<_, String>(7).bridge());

        assert_eq!(outcome.unwrap(), 7);
        assert!(captured.0.lock().unwrap().is_empty());
    }
}
