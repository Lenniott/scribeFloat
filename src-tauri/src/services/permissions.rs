use crate::platform::permissions_impl;
use crate::types::PermissionStatus;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SPEAKER_TTL: Duration = Duration::from_secs(30);

struct SpeakerCache {
    value: bool,
    mic_was_granted: bool,
    at: Instant,
}

pub struct PermissionsService {
    speaker_cache: Mutex<Option<SpeakerCache>>,
}

impl PermissionsService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            speaker_cache: Mutex::new(None),
        })
    }

    pub fn statuses(&self) -> Vec<PermissionStatus> {
        let speaker_granted = self.speaker_capture_granted();
        [
            "microphone",
            "accessibility",
            "input_monitoring",
            "speaker_capture",
        ]
        .iter()
        .map(|kind| {
            let granted = if *kind == "speaker_capture" {
                speaker_granted
            } else {
                permissions_impl::permission_granted(kind)
            };
            PermissionStatus {
                kind: kind.to_string(),
                granted,
                can_request: permissions_impl::permission_can_request(kind),
                hint: permissions_impl::permission_hint(kind),
            }
        })
        .collect()
    }

    pub fn open_settings(&self, kind: &str) -> Result<bool> {
        permissions_impl::open_permission_settings(kind)
    }

    pub fn request_permission(&self, kind: &str) -> Result<()> {
        permissions_impl::request_permission(kind)
    }

    /// True when a BlackHole (or similarly named) loopback input device is present.
    pub fn blackhole_device_detected(&self) -> bool {
        permissions_impl::blackhole_device_detected()
    }

    fn speaker_capture_granted(&self) -> bool {
        let mic_now = permissions_impl::permission_granted("microphone");
        let mut cache = self.speaker_cache.lock().unwrap();

        if let Some(ref entry) = *cache {
            let mic_transitioned = !entry.mic_was_granted && mic_now;
            let expired = entry.at.elapsed() > SPEAKER_TTL;
            if !mic_transitioned && !expired {
                return entry.value;
            }
        }

        let value = permissions_impl::permission_granted("speaker_capture");
        *cache = Some(SpeakerCache {
            value,
            mic_was_granted: mic_now,
            at: Instant::now(),
        });
        value
    }
}

#[cfg(test)]
mod tests {
    use super::PermissionsService;

    #[test]
    fn statuses_have_expected_kinds() {
        let service = PermissionsService::new();
        let statuses = service.statuses();
        let kinds: Vec<_> = statuses.iter().map(|p| p.kind.as_str()).collect();
        assert_eq!(
            kinds,
            vec![
                "microphone",
                "accessibility",
                "input_monitoring",
                "speaker_capture"
            ]
        );
    }

    #[test]
    fn open_settings_unknown_kind_returns_false() {
        let service = PermissionsService::new();
        let opened = service
            .open_settings("not_real")
            .expect("unknown permission should not error");
        assert!(!opened);
    }
}
