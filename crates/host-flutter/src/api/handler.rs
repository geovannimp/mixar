//! FRB handler: log panics that cross the bridge.
//!
//! Errors returned as `Err(..)` are logged by [`crate::api::bridge::BridgeResult`] at
//! the conversion point. Panics never reach that code — FRB catches them — so the
//! handler is the only place they can be recorded. Defining `FLUTTER_RUST_BRIDGE_HANDLER`
//! here makes the generator use it instead of the default one (the default listener
//! discards errors entirely, which is why a Rust panic used to leave no trace).
//!
//! `HandlerError::CustomError` carries no message (FRB drops the value), so nothing is
//! logged for it; the returned error itself is logged where it is converted.

use flutter_rust_bridge::for_generated::SimpleThreadPool;
use flutter_rust_bridge::handler::{ErrorListener, HandlerError, SimpleExecutor, SimpleHandler};
use flutter_rust_bridge::rust_async::SimpleAsyncRuntime;

#[derive(Clone, Copy)]
pub struct LoggingErrorListener;

impl ErrorListener for LoggingErrorListener {
    fn on_error(&self, error: HandlerError) {
        if matches!(&error, HandlerError::Panic(_)) {
            tracing::error!(
                target: "host_flutter::bridge",
                "rust panic crossed the bridge: {}",
                error.message()
            );
        }
    }
}

flutter_rust_bridge::for_generated::lazy_static! {
    pub static ref FLUTTER_RUST_BRIDGE_HANDLER: SimpleHandler<
        SimpleExecutor<LoggingErrorListener, SimpleThreadPool, SimpleAsyncRuntime>,
        LoggingErrorListener,
    > = SimpleHandler::new(
        SimpleExecutor::new(
            LoggingErrorListener,
            SimpleThreadPool::default(),
            SimpleAsyncRuntime::default(),
        ),
        LoggingErrorListener,
    );
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

    fn capture(f: impl FnOnce()) -> String {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .with_ansi(false)
            .with_writer(captured.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        let bytes = captured.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn panics_are_logged_with_their_message() {
        // The default listener drops these, which is why a Rust panic used to leave
        // nothing behind in `mixar.log`.
        let logged = capture(|| {
            LoggingErrorListener.on_error(HandlerError::Panic(Box::new("index out of bounds")));
        });
        assert!(logged.contains("rust panic crossed the bridge"), "{logged}");
        assert!(logged.contains("index out of bounds"), "{logged}");
    }

    #[test]
    fn custom_errors_are_not_reported_here() {
        // Returned `Err`s are logged once, by `BridgeResult`; this listener has no message
        // for them, so logging here would only be a guess.
        let logged = capture(|| {
            LoggingErrorListener.on_error(HandlerError::CustomError);
        });
        assert!(logged.is_empty(), "{logged}");
    }
}
