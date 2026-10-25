use std::{thread, time::Duration};

use serde_json::json;

use crate::{config, queue::Queue, CaptureEvent};

pub(crate) fn start_worker(queue: Queue, token: String) {
    thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                tracing::debug!(%error, "analytics client initialization failed");
                return;
            }
        };
        loop {
            let Some(mut batch) = queue.wait_batch() else {
                return;
            };
            if batch.is_empty() {
                continue;
            }
            let os_version = os_version();
            for event in &mut batch {
                event.os_version.clone_from(&os_version);
            }
            let payload = json!({
                "api_key": token,
                "batch": batch.iter().map(CaptureEvent::to_posthog_json).collect::<Vec<_>>()
            });
            if !queue.is_active() {
                return;
            }
            if let Err(error) = client
                .post(config::capture_url())
                .json(&payload)
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
            {
                tracing::debug!(%error, "analytics flush failed; dropping batch");
            }
        }
    });
}

fn os_version() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .unwrap_or_else(|_| "unknown".into())
            .trim()
            .to_string()
    }
    #[cfg(not(target_os = "linux"))]
    {
        "unknown".into()
    }
}
