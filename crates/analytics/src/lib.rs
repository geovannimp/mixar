use std::path::{Path, PathBuf};

use crate::config::{APP_LAUNCHED_EVENT, APP_VERSION};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

mod client;
pub mod config;
mod queue;

use client::start_worker;
use queue::Queue;
#[cfg(test)]
use queue::{BATCH_SIZE, QUEUE_CAPACITY};

#[derive(Clone, Debug)]
pub struct CaptureEvent {
    distinct_id: String,
    app_version: String,
    os: String,
    os_version: String,
    arch: String,
    build_mode: String,
    timestamp: String,
}

impl CaptureEvent {
    pub fn app_launched(
        distinct_id: String,
        app_version: &str,
        os: &str,
        os_version: &str,
        arch: &str,
        build_mode: &str,
        timestamp: &str,
    ) -> Self {
        Self {
            distinct_id,
            app_version: app_version.into(),
            os: os.into(),
            os_version: os_version.into(),
            arch: arch.into(),
            build_mode: build_mode.into(),
            timestamp: timestamp.into(),
        }
    }

    pub fn current_app_launch(distinct_id: String) -> Self {
        Self::app_launched(
            distinct_id,
            APP_VERSION,
            std::env::consts::OS,
            "unknown",
            std::env::consts::ARCH,
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
            &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        )
    }

    pub fn to_posthog_json(&self) -> Value {
        json!({
            "event": APP_LAUNCHED_EVENT,
            "distinct_id": self.distinct_id,
            "timestamp": self.timestamp,
            "properties": {
                "app_version": self.app_version,
                "os": self.os,
                "os_version": self.os_version,
                "arch": self.arch,
                "build_mode": self.build_mode,
            }
        })
    }
}

fn load_or_create_identity(path: &Path) -> Result<String, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let distinct_id = std::fs::read_to_string(path)
        .ok()
        .map(|id| id.trim().to_string())
        .filter(|id| Uuid::parse_str(id).is_ok())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    std::fs::write(path, &distinct_id).map_err(|error| error.to_string())?;
    Ok(distinct_id)
}

pub struct Analytics {
    queue: Queue,
    identity_path: PathBuf,
    distinct_id: String,
}

impl Analytics {
    pub fn enable(state_path: impl AsRef<Path>) -> Result<Self, String> {
        let token = config::token();
        if token.trim().is_empty() {
            return Err("PostHog token is not configured".into());
        }
        let identity_path = state_path.as_ref().to_path_buf();
        let distinct_id = load_or_create_identity(&identity_path)?;
        let queue = Queue::new();
        start_worker(queue.clone(), token.to_string());
        Ok(Self {
            queue,
            identity_path,
            distinct_id,
        })
    }

    pub fn app_launched(&self) {
        self.queue
            .push(CaptureEvent::current_app_launch(self.distinct_id.clone()));
    }

    pub fn disable(&self) {
        self.queue.clear();
        let _ = std::fs::remove_file(&self.identity_path);
    }
}

impl Drop for Analytics {
    /// Deactivating the queue wakes the flush thread, which then returns. We do
    /// not join: the thread may be inside a 5 s HTTP request, and this can drop
    /// while the transport's command thread is shutting down.
    fn drop(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_payload_contains_only_allowlisted_properties() {
        let payload = CaptureEvent::app_launched(
            "550e8400-e29b-41d4-a716-446655440000".into(),
            "0.0.18",
            "linux",
            "6.8",
            "x86_64",
            "release",
            "2026-10-23T00:00:00Z",
        )
        .to_posthog_json();

        let properties = payload["properties"].as_object().unwrap();
        let top_level_keys: std::collections::BTreeSet<_> = payload
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            top_level_keys,
            ["distinct_id", "event", "properties", "timestamp"]
                .into_iter()
                .collect()
        );
        let keys: std::collections::BTreeSet<_> = properties.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["app_version", "arch", "build_mode", "os", "os_version"]
                .into_iter()
                .collect()
        );
        assert_eq!(payload["event"], APP_LAUNCHED_EVENT);
        assert_eq!(
            payload["distinct_id"],
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(payload["timestamp"], "2026-10-23T00:00:00Z");
    }
    #[test]
    fn bounded_queue_discards_oldest_event() {
        let queue = Queue::new();
        for index in 0..=QUEUE_CAPACITY {
            queue.push(CaptureEvent::app_launched(
                index.to_string(),
                "0.0.18",
                "linux",
                "test",
                "x86_64",
                "test",
                "2026-10-23T00:00:00Z",
            ));
        }
        let drained = queue.drain();
        assert_eq!(drained.len(), BATCH_SIZE);
        assert_eq!(drained[0].distinct_id, "1");
    }

    #[test]
    fn app_version_matches_flutter_pubspec() {
        let version = include_str!("../../../apps/gui-flutter/pubspec.yaml")
            .lines()
            .find_map(|line| line.strip_prefix("version: "))
            .unwrap();
        assert_eq!(APP_VERSION, version);
    }
    #[test]
    fn clearing_queue_drops_pending_and_future_events() {
        let queue = Queue::new();
        queue.push(CaptureEvent::app_launched(
            "before".into(),
            "0.0.18",
            "linux",
            "test",
            "x86_64",
            "test",
            "2026-10-23T00:00:00Z",
        ));
        queue.clear();
        queue.push(CaptureEvent::app_launched(
            "after".into(),
            "0.0.18",
            "linux",
            "test",
            "x86_64",
            "test",
            "2026-10-23T00:00:00Z",
        ));
        assert!(queue.drain().is_empty());
    }
    #[test]
    fn deleting_identity_makes_reenable_use_a_new_uuid() {
        let dir = std::env::temp_dir().join(format!("mixar-analytics-{}", Uuid::new_v4()));
        let path = dir.join("analytics-id");
        let first = load_or_create_identity(&path).unwrap();
        assert_eq!(first, load_or_create_identity(&path).unwrap());
        std::fs::remove_file(&path).unwrap();
        let second = load_or_create_identity(&path).unwrap();
        assert_ne!(first, second);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
