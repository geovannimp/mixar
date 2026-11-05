use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::config::{APP_LAUNCHED_EVENT, APP_VERSION};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use uuid::Uuid;

mod client;
pub mod config;
mod queue;

use client::start_worker;
use queue::Queue;
#[cfg(test)]
use queue::{BATCH_SIZE, QUEUE_CAPACITY};

/// PostHog's error-tracking event name. The error itself travels in
/// `properties.$exception_list`, which is what the product reads.
const EXCEPTION_EVENT: &str = "$exception";

/// Dart package prefix whose frames are this app's own code. Keep it in one
/// place: a rename in `pubspec.yaml` otherwise silently stops marking frames
/// as `in_app`.
const IN_APP_PACKAGE_PREFIX: &str = "package:gui_flutter/";

/// How long an identical `(kind, message)` is suppressed after being queued.
const ERROR_DEDUPE_WINDOW: Duration = Duration::from_secs(60);

/// Signatures remembered for [`ERROR_DEDUPE_WINDOW`]; the oldest is dropped
/// once this many distinct errors are in flight.
const ERROR_DEDUPE_CAPACITY: usize = 32;

#[derive(Clone, Debug)]
pub struct CaptureEvent {
    event: &'static str,
    distinct_id: String,
    timestamp: String,
    properties: Map<String, Value>,
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
            event: APP_LAUNCHED_EVENT,
            distinct_id,
            timestamp: timestamp.into(),
            properties: app_properties(app_version, os, os_version, arch, build_mode),
        }
    }

    pub fn current_app_launch(distinct_id: String) -> Self {
        Self::app_launched(
            distinct_id,
            APP_VERSION,
            std::env::consts::OS,
            "unknown",
            std::env::consts::ARCH,
            build_mode(),
            &now(),
        )
    }

    /// One PostHog error-tracking event.
    ///
    /// `source` is where the error was caught (`flutter` for framework errors,
    /// `dart` for uncaught asynchronous ones). `kind` is the error's runtime
    /// type, which PostHog groups issues by. `stack` is a Dart VM stack trace
    /// when the caller has one.
    pub fn exception(
        distinct_id: String,
        source: &str,
        kind: &str,
        message: &str,
        stack: Option<&str>,
    ) -> Self {
        let mut properties = app_properties(
            APP_VERSION,
            std::env::consts::OS,
            "unknown",
            std::env::consts::ARCH,
            build_mode(),
        );
        let mut exception = json!({
            "type": kind,
            "value": redact(message),
            // Nobody in the app caught these: both handlers are last-resort.
            "mechanism": { "handled": false, "synthetic": false },
        });
        if let Some(frames) = stack.map(stack_frames).filter(|frames| !frames.is_empty()) {
            exception["stacktrace"] = json!({ "type": "raw", "frames": frames });
        }
        properties.insert("$exception_list".into(), Value::Array(vec![exception]));
        properties.insert("$exception_level".into(), Value::from("error"));
        properties.insert("error_source".into(), Value::from(source));
        Self {
            event: EXCEPTION_EVENT,
            distinct_id,
            timestamp: now(),
            properties,
        }
    }

    /// Kernel release, which only the flush worker resolves.
    pub(crate) fn set_os_version(&mut self, os_version: &str) {
        self.properties
            .insert("os_version".into(), Value::from(os_version));
    }

    pub fn to_posthog_json(self) -> Value {
        json!({
            "event": self.event,
            "distinct_id": self.distinct_id,
            "timestamp": self.timestamp,
            "properties": self.properties,
        })
    }
}

/// `debug` or `release`, from the build that compiled this crate.
fn build_mode() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// RFC 3339 timestamp with second precision, which is what PostHog expects.
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// The properties every Mixar event carries: build identity and nothing else.
fn app_properties(
    app_version: &str,
    os: &str,
    os_version: &str,
    arch: &str,
    mode: &str,
) -> Map<String, Value> {
    [
        ("app_version", app_version),
        ("os", os),
        ("os_version", os_version),
        ("arch", arch),
        ("build_mode", mode),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), Value::from(value)))
    .collect()
}

/// Raw stack frames for PostHog, parsed from a Dart VM stack trace:
/// `#0      main (package:gui_flutter/main.dart:12:3)`.
///
/// Frames without a `#` (e.g. `<asynchronous suspension>`) are not frames.
fn stack_frames(stack: &str) -> Vec<Value> {
    stack.lines().filter_map(stack_frame).collect()
}

fn stack_frame(line: &str) -> Option<Value> {
    let rest = line.trim().strip_prefix('#')?;
    let rest = rest
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start();
    // Split at the first ` (`: Dart function names never contain one, while a
    // location can, because a directory may (e.g. `Music (live)`).
    let (function, location) = match rest.find(" (") {
        Some(index) => (
            &rest[..index],
            rest[index + 2..]
                .strip_suffix(')')
                .unwrap_or(&rest[index + 2..]),
        ),
        None => (rest, ""),
    };
    let (filename, lineno, colno) = split_location(location);
    let mut frame = json!({
        "platform": "custom",
        "lang": "dart",
        "function": redact(if function.is_empty() { rest } else { function }),
        "in_app": filename.starts_with(IN_APP_PACKAGE_PREFIX),
    });
    if !filename.is_empty() {
        frame["filename"] = Value::from(redact(filename));
    }
    if let Some(lineno) = lineno {
        frame["lineno"] = Value::from(lineno);
    }
    if let Some(colno) = colno {
        frame["colno"] = Value::from(colno);
    }
    Some(frame)
}

/// `name:line:col`, or the whole string as the name when it carries no line.
fn split_location(location: &str) -> (&str, Option<u64>, Option<u64>) {
    let mut parts = location.rsplitn(3, ':');
    let colno = parts.next().and_then(|part| part.parse().ok());
    let lineno = parts.next().and_then(|part| part.parse().ok());
    match (parts.next(), lineno) {
        (Some(filename), Some(lineno)) => (filename, Some(lineno), colno),
        _ => (location, None, None),
    }
}

/// Replace an absolute filesystem path, and the rest of its line, with
/// `<path>`.
///
/// User-facing errors already label files by basename
/// (`library_core::path_label`), but SDK messages and native stack frames can
/// carry full paths, and the privacy page promises no paths or file names leave
/// the machine. A path may contain spaces, so there is no reliable end marker:
/// keeping a tail would leak the folder or file name. Everything before the
/// path on that line survives, as do lines without a path.
fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match absolute_path_start(line) {
            Some(start) => {
                out.push_str(&line[..start]);
                out.push_str("<path>");
            }
            None => out.push_str(line),
        }
    }
    out
}

/// Byte offset of the first absolute path in `line`, if any.
///
/// A path has to start a token: at the line start or after a byte that cannot
/// continue a token. That keeps the `/` in `dart:async/zone.dart` or `48/2`
/// from reading as a path, while still catching one after `(`, `;`, `:`, `>`.
fn absolute_path_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    (0..bytes.len()).find(|&index| {
        let starts_token = index == 0
            || !(bytes[index - 1].is_ascii_alphanumeric()
                || matches!(bytes[index - 1], b'_' | b'/'));
        starts_token && starts_path(&bytes[index..])
    })
}

fn starts_path(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"file://") || bytes.starts_with(b"\\\\") {
        return true;
    }
    match bytes.first() {
        // A single leading slash is a POSIX path; `//` is a URI's authority and
        // stays readable, which is what keeps `https://…` in an error message.
        Some(b'/') => bytes.len() > 1 && bytes[1] != b'/' && !bytes[1].is_ascii_whitespace(),
        Some(first) if first.is_ascii_alphabetic() => {
            bytes.len() > 2 && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/')
        }
        _ => false,
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
    /// Recent `(kind, redacted message, queued at)` signatures, so a widget that
    /// throws on every frame cannot fill the queue or the wire.
    recent_errors: Vec<(String, String, Instant)>,
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
            recent_errors: Vec::new(),
        })
    }

    pub fn app_launched(&self) {
        self.queue
            .push(CaptureEvent::current_app_launch(self.distinct_id.clone()));
    }

    /// Queue one error report. Same gate as every other event: with analytics
    /// off, the queue drops it.
    ///
    /// Repeats of the same kind and redacted message within
    /// [`ERROR_DEDUPE_WINDOW`] are dropped, so a widget that throws on every
    /// frame — or a scan failing on file after file — sends one report, not
    /// thousands. The key is the redacted value on purpose: two messages that
    /// differ only inside a path serialize identically.
    ///
    /// Rust panics are deliberately not wired in: release builds use
    /// `panic = "abort"`, so the process dies before a queued batch can flush.
    pub fn error(&mut self, source: &str, kind: &str, message: &str, stack: Option<&str>) {
        let queued_at = Instant::now();
        let value = redact(message);
        self.recent_errors
            .retain(|(_, _, at)| queued_at.duration_since(*at) < ERROR_DEDUPE_WINDOW);
        if self
            .recent_errors
            .iter()
            .any(|(seen_kind, seen_value, _)| seen_kind == kind && seen_value == &value)
        {
            return;
        }
        if self.recent_errors.len() == ERROR_DEDUPE_CAPACITY {
            self.recent_errors.remove(0);
        }
        self.recent_errors
            .push((kind.to_string(), value, queued_at));
        self.queue.push(CaptureEvent::exception(
            self.distinct_id.clone(),
            source,
            kind,
            message,
            stack,
        ));
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

    /// An enabled client over `queue`, for tests that never reach the network.
    fn test_analytics(queue: &Queue) -> Analytics {
        Analytics {
            queue: queue.clone(),
            identity_path: std::env::temp_dir().join("unused-analytics-id"),
            distinct_id: "client-id".into(),
            recent_errors: Vec::new(),
        }
    }

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
    fn exception_payload_uses_the_posthog_error_tracking_shape() {
        let payload = CaptureEvent::exception(
            "550e8400-e29b-41d4-a716-446655440000".into(),
            "flutter",
            "StateError",
            "Bad state: no engine",
            Some(
                "#0      EngineTransport.play (package:gui_flutter/providers/mixer/engine_providers.dart:10:5)\n\
                 #1      _rootRun (dart:async/zone.dart:1399:13)\n\
                 <asynchronous suspension>",
            ),
        )
        .to_posthog_json();

        assert_eq!(payload["event"], EXCEPTION_EVENT);
        assert_eq!(payload["properties"]["$exception_level"], "error");
        assert_eq!(payload["properties"]["error_source"], "flutter");

        let exceptions = payload["properties"]["$exception_list"].as_array().unwrap();
        assert_eq!(exceptions.len(), 1);
        let exception = &exceptions[0];
        assert_eq!(exception["type"], "StateError");
        assert_eq!(exception["value"], "Bad state: no engine");
        assert_eq!(exception["mechanism"]["handled"], false);
        assert_eq!(exception["mechanism"]["synthetic"], false);

        let frames = exception["stacktrace"]["frames"].as_array().unwrap();
        assert_eq!(frames.len(), 2, "the suspension line is not a frame");
        assert_eq!(
            frames[0]["filename"],
            "package:gui_flutter/providers/mixer/engine_providers.dart"
        );
        assert_eq!(frames[0]["function"], "EngineTransport.play");
        assert_eq!(frames[0]["lineno"], 10);
        assert_eq!(frames[0]["colno"], 5);
        assert_eq!(frames[0]["in_app"], true);
        assert_eq!(frames[0]["platform"], "custom");
        assert_eq!(frames[0]["lang"], "dart");
        assert_eq!(frames[1]["filename"], "dart:async/zone.dart");
        assert_eq!(frames[1]["in_app"], false);
    }

    #[test]
    fn exception_redacts_absolute_paths_from_messages_and_frames() {
        let payload = CaptureEvent::exception(
            "id".into(),
            "dart",
            "FileSystemException",
            "Cannot open file (/home/ana/Music/secret.flac) for reading and writing",
            Some(
                "#0      read (file:///home/ana/Music/secret.flac:1:2)\n\
                 #1      main (C:\\Users\\ana\\tracks\\secret.flac:3:4)",
            ),
        )
        .to_posthog_json();

        let exception = &payload["properties"]["$exception_list"][0];
        assert_eq!(exception["value"], "Cannot open file (<path>");
        let frames = exception["stacktrace"]["frames"].as_array().unwrap();
        assert_eq!(frames[0]["filename"], "<path>");
        assert_eq!(frames[1]["filename"], "<path>");
    }

    #[test]
    fn redaction_swallows_paths_with_spaces_and_keeps_other_lines() {
        assert_eq!(
            redact("decode /home/ana/My Music/Artist - Title.flac failed"),
            "decode <path>"
        );
        assert_eq!(
            redact("read /home/ana/x.flac (os error 30)\nthen continue"),
            "read <path>\nthen continue"
        );
    }

    #[test]
    fn redaction_catches_unc_and_paths_after_punctuation() {
        assert_eq!(redact(r"open \\server\share\mix\track.flac"), "open <path>");
        assert_eq!(redact("a;/home/ana/x"), "a;<path>");
        assert_eq!(redact("path:/home/ana/x"), "path:<path>");
    }

    #[test]
    fn redaction_leaves_urls_readable_and_survives_multibyte_prefixes() {
        assert_eq!(
            redact("GET https://eu.i.posthog.com/batch/ failed"),
            "GET https://eu.i.posthog.com/batch/ failed"
        );
        // The `«` is two bytes; slicing at the path start must not split it.
        assert_eq!(redact("«/home/ana/x.flac»"), "«<path>");
    }

    #[test]
    fn frame_with_parenthesized_directory_keeps_function_and_redacts_path() {
        let payload = CaptureEvent::exception(
            "id".into(),
            "dart",
            "StateError",
            "boom",
            Some("#0      read (file:///home/ana/Music (live)/Alias - Track.flac:1:2)"),
        )
        .to_posthog_json();

        let frame = &payload["properties"]["$exception_list"][0]["stacktrace"]["frames"][0];
        assert_eq!(frame["function"], "read");
        assert_eq!(frame["filename"], "<path>");
        assert_eq!(frame["lineno"], 1);
        assert_eq!(frame["colno"], 2);
    }

    #[test]
    fn relative_and_package_paths_are_left_alone() {
        assert_eq!(
            redact("panicked at crates/analytics/src/lib.rs:77"),
            "panicked at crates/analytics/src/lib.rs:77"
        );
        assert_eq!(
            redact("package:gui_flutter/main.dart:12:3"),
            "package:gui_flutter/main.dart:12:3"
        );
        assert_eq!(redact("ratio 48/2"), "ratio 48/2");
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
    fn clearing_queue_drops_pending_error_reports() {
        let queue = Queue::new();
        queue.push(CaptureEvent::exception(
            "before".into(),
            "dart",
            "StateError",
            "boom",
            None,
        ));
        queue.clear();
        assert!(queue.drain().is_empty());
    }

    #[test]
    fn error_reports_use_the_client_identity_and_are_dropped_once_disabled() {
        let queue = Queue::new();
        let mut analytics = test_analytics(&queue);

        analytics.error("dart", "StateError", "boom", None);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].distinct_id, "client-id");
        let payload = drained.into_iter().next().unwrap().to_posthog_json();
        assert_eq!(payload["event"], EXCEPTION_EVENT);
        assert_eq!(payload["properties"]["error_source"], "dart");

        analytics.disable();
        analytics.error("dart", "StateError", "after opt-out", None);
        assert!(queue.drain().is_empty());
    }

    #[test]
    fn repeated_errors_are_reported_once_per_window() {
        let queue = Queue::new();
        let mut analytics = test_analytics(&queue);

        analytics.error("dart", "StateError", "boom", None);
        analytics.error("dart", "StateError", "boom", None);
        analytics.error("flutter", "StateError", "boom", None);
        analytics.error("dart", "StateError", "different", None);

        let drained = queue.drain();
        assert_eq!(drained.len(), 2, "one duplicate and one distinct message");
    }

    #[test]
    fn messages_differing_only_in_a_path_are_one_report() {
        let queue = Queue::new();
        let mut analytics = test_analytics(&queue);

        analytics.error(
            "dart",
            "FileSystemException",
            "cannot open /home/a/A.flac",
            None,
        );
        analytics.error(
            "dart",
            "FileSystemException",
            "cannot open /home/a/B.flac",
            None,
        );

        assert_eq!(queue.drain().len(), 1, "both serialize to the same value");
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
