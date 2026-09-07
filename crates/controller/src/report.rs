//! Rate-limited controller diagnostics (MIDI / Rhai).

use std::time::{Duration, Instant};

/// Suppress repeat failure spam; always log first failure and recovery.
#[derive(Debug, Default)]
pub(crate) struct FailureGate {
    failing: bool,
    last_logged: Option<Instant>,
}

impl FailureGate {
    /// Log on first failure and again at most once per `interval` while still failing.
    pub(crate) fn on_err(&mut self, interval: Duration, log: impl FnOnce()) {
        let now = Instant::now();
        let due = !self.failing
            || self
                .last_logged
                .is_none_or(|t| now.duration_since(t) >= interval);
        if due {
            log();
            self.last_logged = Some(now);
            self.failing = true;
        }
    }

    /// Log once when leaving a failing streak.
    pub(crate) fn on_ok(&mut self, log: impl FnOnce()) {
        if self.failing {
            log();
            self.failing = false;
            self.last_logged = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn is_failing(&self) -> bool {
        self.failing
    }
}

/// Compact hex for MIDI payloads (capped so logs stay readable).
pub(crate) fn midi_bytes_hex(bytes: &[u8]) -> String {
    const MAX: usize = 32;
    let mut out = String::with_capacity(bytes.len().min(MAX) * 3);
    for (i, b) in bytes.iter().take(MAX).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{b:02X}"));
    }
    if bytes.len() > MAX {
        out.push_str(" …");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn failure_gate_logs_first_then_rate_limits() {
        let mut gate = FailureGate::default();
        let n = AtomicUsize::new(0);
        gate.on_err(Duration::from_secs(60), || {
            n.fetch_add(1, Ordering::Relaxed);
        });
        gate.on_err(Duration::from_secs(60), || {
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.load(Ordering::Relaxed), 1);
        assert!(gate.is_failing());
    }

    #[test]
    fn failure_gate_logs_recovery_once() {
        let mut gate = FailureGate::default();
        let n = AtomicUsize::new(0);
        gate.on_err(Duration::from_secs(60), || {});
        gate.on_ok(|| {
            n.fetch_add(1, Ordering::Relaxed);
        });
        gate.on_ok(|| {
            n.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(n.load(Ordering::Relaxed), 1);
        assert!(!gate.is_failing());
    }

    #[test]
    fn midi_bytes_hex_formats_and_caps() {
        assert_eq!(midi_bytes_hex(&[0xF0, 0x00, 0x7F]), "F0 00 7F");
        let long: Vec<u8> = (0..40).map(|i| i as u8).collect();
        let s = midi_bytes_hex(&long);
        assert!(s.ends_with('…'));
        assert!(s.starts_with("00 01 02"));
    }
}
