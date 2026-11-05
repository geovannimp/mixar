use std::{
    path::Path,
    sync::mpsc::{self, Sender},
};

use analytics::Analytics;

enum Command {
    SetEnabled(bool),
    AppLaunched,
    ReportError {
        source: String,
        kind: String,
        message: String,
        stack: Option<String>,
    },
}

#[flutter_rust_bridge::frb(opaque)]
pub struct AnalyticsTransport {
    commands: Sender<Command>,
}

impl AnalyticsTransport {
    /// Open the analytics host against `{app-support}` (identity file included).
    pub fn open() -> Self {
        Self::open_at(&crate::app_paths::support())
    }

    /// Open against an explicit app-support directory (tests: temp directories).
    pub(crate) fn open_at(app_support: &Path) -> Self {
        let (commands, receiver) = mpsc::channel();
        let identity_path = app_support.join("analytics-id");
        std::thread::spawn(move || {
            let mut client: Option<Analytics> = None;
            while let Ok(command) = receiver.recv() {
                match command {
                    Command::SetEnabled(true) => {
                        if client.is_none() && !analytics::config::token().trim().is_empty() {
                            match Analytics::enable(&identity_path) {
                                Ok(analytics) => client = Some(analytics),
                                Err(error) => {
                                    tracing::debug!(%error, "analytics enable failed");
                                }
                            }
                        }
                    }
                    Command::SetEnabled(false) => {
                        if let Some(analytics) = client.take() {
                            analytics.disable();
                        } else {
                            let _ = std::fs::remove_file(&identity_path);
                        }
                    }
                    Command::AppLaunched => {
                        if let Some(analytics) = client.as_ref() {
                            analytics.app_launched();
                        }
                    }
                    Command::ReportError {
                        source,
                        kind,
                        message,
                        stack,
                    } => {
                        if let Some(analytics) = client.as_mut() {
                            analytics.error(&source, &kind, &message, stack.as_deref());
                        }
                    }
                }
            }
        });
        Self { commands }
    }

    /// Deliver one command, reporting a dead command thread instead of dropping
    /// the failure silently. Analytics still never changes app behaviour.
    fn send(&self, command: Command) {
        if let Err(error) = self.commands.send(command) {
            tracing::debug!(%error, "analytics command dropped");
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.send(Command::SetEnabled(enabled));
    }

    pub fn app_launched(&self) {
        self.send(Command::AppLaunched);
    }

    /// Report one error. Dropped on the Rust side when analytics is off, so the
    /// caller does not have to track the switch.
    pub fn report_error(
        &self,
        source: String,
        kind: String,
        message: String,
        stack: Option<String>,
    ) {
        self.send(Command::ReportError {
            source,
            kind,
            message,
            stack,
        });
    }
}
