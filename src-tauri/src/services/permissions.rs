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
                can_request: permissions_impl::permission_settings_url(kind).is_some(),
            })
            .collect()
    }

    pub fn open_settings(&self, kind: &str) -> Result<bool> {
        permissions_impl::open_permission_settings(kind)
    }
}
