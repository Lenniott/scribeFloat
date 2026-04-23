use crate::platform::permissions_impl;
use crate::types::PermissionStatus;
use anyhow::Result;
use std::sync::Arc;

pub struct PermissionsService;

impl PermissionsService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn statuses(&self) -> Vec<PermissionStatus> {
        ["microphone", "accessibility", "input_monitoring"]
            .iter()
            .map(|kind| PermissionStatus {
                kind: kind.to_string(),
                granted: permissions_impl::permission_granted(kind),
                can_request: permissions_impl::permission_can_request(kind),
            })
            .collect()
    }

    pub fn open_settings(&self, kind: &str) -> Result<bool> {
        permissions_impl::open_permission_settings(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::PermissionsService;

    #[test]
    fn statuses_have_expected_kinds() {
        let service = PermissionsService;
        let statuses = service.statuses();
        let kinds: Vec<_> = statuses.iter().map(|p| p.kind.as_str()).collect();
        assert_eq!(
            kinds,
            vec!["microphone", "accessibility", "input_monitoring"]
        );
    }

    #[test]
    fn open_settings_unknown_kind_returns_false() {
        let service = PermissionsService;
        let opened = service
            .open_settings("not_real")
            .expect("unknown permission should not error");
        assert!(!opened);
    }
}
