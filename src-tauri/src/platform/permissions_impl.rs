use anyhow::Result;

#[cfg(target_os = "macos")]
pub fn permission_granted(kind: &str) -> bool {
    match kind {
        "microphone" => macos::microphone_granted(),
        "accessibility" => macos::accessibility_granted(),
        "input_monitoring" => macos::input_monitoring_granted(),
        "speaker_capture" => macos::speaker_capture_ready(),
        _ => false,
    }
}

#[cfg(target_os = "windows")]
pub fn permission_granted(kind: &str) -> bool {
    match kind {
        "microphone" => windows::microphone_granted(),
        // No explicit OS grant prompt exists for these in current flow.
        "accessibility" | "input_monitoring" => true,
        // WASAPI loopback is built into Windows and needs no extra driver.
        "speaker_capture" => true,
        _ => false,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn permission_granted(_kind: &str) -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn permission_can_request(kind: &str) -> bool {
    kind != "speaker_capture" && permission_settings_url(kind).is_some()
}

#[cfg(target_os = "windows")]
pub fn permission_can_request(kind: &str) -> bool {
    matches!(kind, "microphone")
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn permission_can_request(kind: &str) -> bool {
    kind != "speaker_capture" && permission_settings_url(kind).is_some()
}

#[cfg(target_os = "macos")]
pub fn permission_settings_url(kind: &str) -> Option<&'static str> {
    match kind {
        "microphone" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        }
        "accessibility" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        }
        "input_monitoring" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        }
        "speaker_capture" => None,
        _ => None,
    }
}

#[cfg(target_os = "windows")]
pub fn permission_settings_url(kind: &str) -> Option<&'static str> {
    match kind {
        "microphone" => Some("ms-settings:privacy-microphone"),
        "accessibility" => Some("ms-settings:easeofaccess-display"),
        "input_monitoring" => Some("ms-settings:privacy"),
        "speaker_capture" => None,
        _ => None,
    }
}

#[cfg(target_os = "macos")]
pub fn permission_hint(kind: &str) -> Option<String> {
    match kind {
        "speaker_capture" => Some(
            "Speaker capture requires BlackHole 2ch installed and selected in your audio routing."
                .to_string(),
        ),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
pub fn permission_hint(kind: &str) -> Option<String> {
    match kind {
        "speaker_capture" => Some(
            "Windows speaker capture uses WASAPI loopback and does not require BlackHole."
                .to_string(),
        ),
        _ => None,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn permission_hint(_kind: &str) -> Option<String> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn permission_settings_url(_kind: &str) -> Option<&'static str> {
    None
}

/// Actively request the permission — triggers the OS dialog where available,
/// or opens the relevant System Settings pane as a fallback.
///
/// For microphone on macOS this MUST be called from a non-main thread (spawn_blocking).
pub fn request_permission(kind: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        if kind == "input_monitoring" {
            macos::request_listen_event_access();
            return Ok(());
        }
        if kind == "microphone" && macos::microphone_not_determined() {
            // Status is NotDetermined — show the native TCC dialog directly.
            // If Denied, fall through to open System Settings below.
            macos::request_microphone_access();
            return Ok(());
        }
    }
    open_permission_settings(kind)?;
    Ok(())
}

pub fn open_permission_settings(kind: &str) -> Result<bool> {
    let Some(url) = permission_settings_url(kind) else {
        return Ok(false);
    };
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).status()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        return Ok(false);
    }
    Ok(true)
}

#[cfg(target_os = "macos")]
mod macos {
    use cpal::traits::{DeviceTrait, HostTrait};
    use std::ffi::{c_char, c_int, c_long, c_void};

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
    }

    #[link(name = "AVFoundation", kind = "framework")]
    unsafe extern "C" {}

    #[link(name = "objc")]
    #[allow(clashing_extern_declarations)]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn objc_msg_send_id_cstr(
            receiver: *mut c_void,
            op: *mut c_void,
            arg: *const c_char,
        ) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn objc_msg_send_isize_arg(
            receiver: *mut c_void,
            op: *mut c_void,
            arg: *mut c_void,
        ) -> isize;
        // Used by request_microphone_access: class method with id + block arguments.
        #[link_name = "objc_msgSend"]
        fn objc_msg_send_request_access(
            receiver: *mut c_void,
            sel: *mut c_void,
            media_type: *mut c_void,
            block: *mut MicAccessBlock,
        );
    }

    // GCD semaphore — lets a spawn_blocking thread wait for the async TCC callback
    // that fires on the main dispatch queue without blocking the main thread.
    extern "C" {
        fn dispatch_semaphore_create(value: c_long) -> *mut c_void;
        fn dispatch_semaphore_wait(sema: *mut c_void, timeout: u64) -> c_long;
        fn dispatch_semaphore_signal(sema: *mut c_void) -> c_long;
    }

    // ISA pointer for stack-allocated Objective-C blocks.
    extern "C" {
        static _NSConcreteStackBlock: c_void;
    }

    // Minimal Objective-C block literal matching the published ABI.
    // Captures only raw pointers (no ObjC objects), so no copy/dispose helpers needed.
    #[repr(C)]
    struct BlockDescriptor {
        reserved: usize,
        size: usize,
    }

    static MIC_BLOCK_DESC: BlockDescriptor = BlockDescriptor {
        reserved: 0,
        size: std::mem::size_of::<MicAccessBlock>(),
    };

    #[repr(C)]
    pub(super) struct MicAccessBlock {
        isa: *const c_void,
        flags: c_int,
        _reserved: c_int,
        invoke: unsafe extern "C" fn(*mut MicAccessBlock, bool),
        descriptor: *const BlockDescriptor,
        // Captured:
        sema: *mut c_void,
        result: *mut bool,
    }

    // SAFETY: the block is only used while the calling stack frame is live (we
    // block on the semaphore until the callback fires).
    unsafe impl Send for MicAccessBlock {}
    unsafe impl Sync for MicAccessBlock {}

    unsafe extern "C" fn mic_access_invoke(block: *mut MicAccessBlock, granted: bool) {
        unsafe {
            *(*block).result = granted;
            dispatch_semaphore_signal((*block).sema);
        }
    }

    pub fn accessibility_granted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn input_monitoring_granted() -> bool {
        unsafe { CGPreflightListenEventAccess() }
    }

    pub fn request_listen_event_access() -> bool {
        unsafe { CGRequestListenEventAccess() }
    }

    pub fn microphone_granted() -> bool {
        // AVAuthorizationStatusAuthorized = 3
        microphone_auth_status() == 3
    }

    pub fn microphone_not_determined() -> bool {
        // AVAuthorizationStatusNotDetermined = 0
        microphone_auth_status() == 0
    }

    /// Request microphone access via the native TCC dialog.
    ///
    /// MUST be called from a non-main thread — the TCC completion block fires on
    /// the main dispatch queue, and we block on a semaphore here waiting for it.
    /// Calling from the main thread deadlocks. Use spawn_blocking.
    pub fn request_microphone_access() -> bool {
        // Only request when status is NotDetermined; Denied needs System Settings.
        if !microphone_not_determined() {
            return microphone_granted();
        }
        unsafe {
            let sema = dispatch_semaphore_create(0);
            if sema.is_null() {
                return false;
            }
            let mut result = false;
            let mut block = MicAccessBlock {
                isa: &_NSConcreteStackBlock as *const c_void,
                flags: 0,
                _reserved: 0,
                invoke: mic_access_invoke,
                descriptor: &MIC_BLOCK_DESC,
                sema,
                result: &mut result,
            };

            let av_cls = objc_getClass(c"AVCaptureDevice".as_ptr());
            let ns_cls = objc_getClass(c"NSString".as_ptr());
            let req_sel =
                sel_registerName(c"requestAccessForMediaType:completionHandler:".as_ptr());
            let str_sel = sel_registerName(c"stringWithUTF8String:".as_ptr());

            if !av_cls.is_null()
                && !ns_cls.is_null()
                && !req_sel.is_null()
                && !str_sel.is_null()
            {
                let media_type = objc_msg_send_id_cstr(ns_cls, str_sel, c"soun".as_ptr());
                if !media_type.is_null() {
                    objc_msg_send_request_access(av_cls, req_sel, media_type, &mut block);
                    // Block until the completion handler signals us.
                    dispatch_semaphore_wait(sema, u64::MAX);
                }
            }
            result
        }
    }

    pub fn speaker_capture_ready() -> bool {
        if !microphone_granted() {
            return false;
        }
        // Use devices() not input_devices(): input_devices() lazily filters via
        // supports_input() which calls audio_unit_from_device() per device, creating
        // an AudioUnit and blocking on CoreAudio IPC (mach_msg2_trap). We only need
        // device names; devices() reads those without probing device capabilities.
        let host = cpal::default_host();
        let Ok(devices) = host.devices() else {
            return false;
        };
        devices
            .filter_map(|device| device.name().ok())
            .any(|name| looks_like_blackhole_name(&name))
    }

    fn microphone_auth_status() -> isize {
        unsafe {
            let ns_string_cls = objc_getClass(c"NSString".as_ptr());
            let av_capture_cls = objc_getClass(c"AVCaptureDevice".as_ptr());
            if ns_string_cls.is_null() || av_capture_cls.is_null() {
                return 0;
            }

            let string_sel = sel_registerName(c"stringWithUTF8String:".as_ptr());
            let status_sel = sel_registerName(c"authorizationStatusForMediaType:".as_ptr());
            if string_sel.is_null() || status_sel.is_null() {
                return 0;
            }

            let media_type =
                objc_msg_send_id_cstr(ns_string_cls, string_sel, c"soun".as_ptr());
            if media_type.is_null() {
                return 0;
            }

            objc_msg_send_isize_arg(av_capture_cls, status_sel, media_type)
        }
    }

    fn looks_like_blackhole_name(name: &str) -> bool {
        name.to_ascii_lowercase().contains("blackhole")
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Without this, every `reg query` flashes a black cmd window — the permissions
    // screen polls every 10s and on every focus change, so it was very visible.
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    pub fn microphone_granted() -> bool {
        let output = Command::new("reg")
            .args([
                "query",
                r#"HKCU\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone"#,
                "/v",
                "Value",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let Ok(output) = output else {
            return false;
        };
        if !output.status.success() {
            return false;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        // ConsentStore values are typically Allow / Deny / Prompt.
        text.lines().any(|line| line.contains("Allow"))
    }
}

#[cfg(test)]
mod tests {
    use super::{open_permission_settings, permission_hint, permission_settings_url};

    #[test]
    fn unknown_permission_has_no_settings_target() {
        assert!(permission_settings_url("not_real").is_none());
    }

    #[test]
    fn open_settings_returns_false_for_unknown_kind() {
        let opened =
            open_permission_settings("not_real").expect("unknown permission should not error");
        assert!(!opened);
    }

    #[test]
    fn speaker_capture_does_not_have_settings_deeplink() {
        assert!(permission_settings_url("speaker_capture").is_none());
    }

    #[test]
    fn speaker_capture_has_platform_specific_hint() {
        let hint = permission_hint("speaker_capture");
        assert!(hint.is_some());
    }
}
