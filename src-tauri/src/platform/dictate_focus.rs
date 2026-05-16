//! Remember which app had keyboard focus before the dictate HUD appears, then restore
//! it before simulating Paste. Fixes first-invocation races where WKWebView's first
//! `show()` makes our process frontmost and macOS restores the wrong window after hide.

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_char;
    use std::ffi::c_void;

    type NSInteger = isize;

    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {}

    #[link(name = "objc")]
    #[allow(clashing_extern_declarations)]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;

        #[link_name = "objc_msgSend"]
        fn objc_msg_send_id(receiver: *mut c_void, op: *mut c_void) -> *mut c_void;

        #[link_name = "objc_msgSend"]
        fn objc_msg_send_nsinteger(receiver: *mut c_void, op: *mut c_void) -> NSInteger;

        #[link_name = "objc_msgSend"]
        fn objc_msg_send_id_nsinteger(
            receiver: *mut c_void,
            op: *mut c_void,
            arg: NSInteger,
        ) -> *mut c_void;

        #[link_name = "objc_msgSend"]
        fn objc_msg_send_bool_u64(receiver: *mut c_void, op: *mut c_void, opts: u64) -> bool;
    }

    fn sel_lit(name_with_nul: &'static [u8]) -> *mut c_void {
        unsafe { sel_registerName(name_with_nul.as_ptr() as *const c_char) }
    }

    pub fn capture_frontmost_pid_excluding_self() -> Option<i32> {
        let my_pid = std::process::id() as i32;
        unsafe {
            let ws_class = objc_getClass(c"NSWorkspace".as_ptr());
            let ra_check = objc_getClass(c"NSRunningApplication".as_ptr());
            if ws_class.is_null() || ra_check.is_null() {
                return None;
            }

            let sel_shared = sel_lit(b"sharedWorkspace\0");
            let sel_front = sel_lit(b"frontmostApplication\0");
            let sel_pid = sel_lit(b"processIdentifier\0");
            if sel_shared.is_null() || sel_front.is_null() || sel_pid.is_null() {
                return None;
            }

            let workspace = objc_msg_send_id(ws_class, sel_shared);
            if workspace.is_null() {
                return None;
            }

            let front = objc_msg_send_id(workspace, sel_front);
            if front.is_null() {
                return None;
            }

            let pid = objc_msg_send_nsinteger(front, sel_pid);
            if pid <= 0 || pid as i32 == my_pid {
                return None;
            }
            Some(pid as i32)
        }
    }

    pub fn activate_pid(pid: i32) -> Result<(), String> {
        if pid <= 0 {
            return Err("invalid pid".to_string());
        }
        unsafe {
            let ra_class = objc_getClass(c"NSRunningApplication".as_ptr());
            if ra_class.is_null() {
                return Err("NSRunningApplication unavailable".to_string());
            }
            let sel_running =
                sel_lit(b"runningApplicationWithProcessIdentifier:\0");
            let sel_activate = sel_lit(b"activateWithOptions:\0");
            if sel_running.is_null() || sel_activate.is_null() {
                return Err("Objective-C selectors unavailable".to_string());
            }

            let app = objc_msg_send_id_nsinteger(ra_class, sel_running, pid as NSInteger);
            if app.is_null() {
                return Err(format!(
                    "no NSRunningApplication for pid {pid} (process may have exited)"
                ));
            }
            // NSApplicationActivateAllWindows | NSApplicationActivateIgnoringOtherApps
            let ok = objc_msg_send_bool_u64(app, sel_activate, 3);
            if ok {
                Ok(())
            } else {
                Err(format!(
                    "activateWithOptions returned false for pid {pid}"
                ))
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub fn capture_frontmost_pid_excluding_self() -> Option<i32> {
    macos::capture_frontmost_pid_excluding_self()
}

#[cfg(target_os = "macos")]
pub fn activate_pid_for_paste(pid: i32) -> Result<(), String> {
    macos::activate_pid(pid)
}

#[cfg(not(target_os = "macos"))]
pub fn capture_frontmost_pid_excluding_self() -> Option<i32> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn activate_pid_for_paste(_pid: i32) -> Result<(), String> {
    Ok(())
}
