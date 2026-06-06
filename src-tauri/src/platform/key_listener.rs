//! Low-level modifier-key listener.
//!
//! On macOS we use a CGEventTap directly rather than `rdev` because rdev calls
//! `TSMGetInputSourceProperty` (string_from_code) for every key event on a
//! background thread, which triggers a dispatch-queue assertion on macOS 13+
//! and crashes the process.
//!
//! We only need raw keydown/keyup for Left Control (keycode 59), so we skip
//! string conversion entirely.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    Down,
    Up,
}

pub struct KeyEvent {
    pub kind: KeyEventKind,
}

/// Spawn a background thread that calls `callback` for every Left Control (macOS)
/// or Left Alt (Windows) keydown / keyup event.
pub fn start_modifier_listener<F>(callback: F)
where
    F: Fn(KeyEvent) + Send + 'static,
{
    #[cfg(target_os = "macos")]
    macos::start(callback);

    #[cfg(target_os = "windows")]
    windows::start(callback);

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = callback;
}

// ── macOS ────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::{KeyEvent, KeyEventKind};
    use std::ffi::c_void;

    // Left Control keycode on macOS (Carbon kVK_Control = 59).
    const KEYCODE_CTRL_LEFT: i64 = 59;

    // CGEventType: modifier-key press/release fires as flagsChanged (12), not keyDown/keyUp.
    const K_CGEVENT_FLAGS_CHANGED: u32 = 12;

    // CGEventField: kCGKeyboardEventKeycode = 9
    const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

    // kCGEventFlagMaskControl = 0x40000 — set when any Control key is held.
    const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x40000;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: *const c_void,
            user_info: *mut c_void,
        ) -> *mut c_void;
        fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
        fn CGEventGetFlags(event: *mut c_void) -> u64;
        fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            tap: *mut c_void,
            order: isize,
        ) -> *mut c_void;
        fn CFRunLoopGetCurrent() -> *mut c_void;
        fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *mut c_void);
        fn CGEventTapEnable(tap: *mut c_void, enable: bool);
        fn CFRunLoopRun();
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopDefaultMode: *mut c_void;
    }

    // kCGSessionEventTap = 1, kCGHeadInsertEventTap = 0, kCGEventTapOptionListenOnly = 1
    const K_CG_SESSION_EVENT_TAP: u32 = 1;
    const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;

    // Only subscribe to flagsChanged — that's what macOS fires for modifier keys.
    const EVENT_MASK: u64 = 1 << K_CGEVENT_FLAGS_CHANGED;

    // The callback receives a raw pointer to our Box<dyn Fn>.
    extern "C" fn tap_callback(
        _proxy: *mut c_void,
        event_type: u32,
        event: *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void {
        if event_type != K_CGEVENT_FLAGS_CHANGED {
            return event;
        }
        let keycode = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) };
        if keycode != KEYCODE_CTRL_LEFT {
            return event;
        }
        let flags = unsafe { CGEventGetFlags(event) };
        let kind = if flags & K_CG_EVENT_FLAG_MASK_CONTROL != 0 {
            KeyEventKind::Down
        } else {
            KeyEventKind::Up
        };
        // Safety: user_info is a leaked Box<Box<dyn Fn(KeyEvent)>> pointer.
        let cb = unsafe { &*(user_info as *const Box<dyn Fn(KeyEvent) + Send + 'static>) };
        cb(KeyEvent { kind });
        event
    }

    pub fn start<F: Fn(KeyEvent) + Send + 'static>(callback: F) {
        // Double-box so the fat pointer fits in a *mut c_void.
        let cb: Box<Box<dyn Fn(KeyEvent) + Send + 'static>> = Box::new(Box::new(callback));
        // Transmit the pointer as usize so it is Send; restore it inside the thread.
        let user_info_addr: usize = Box::into_raw(cb) as usize;

        std::thread::spawn(move || unsafe {
            let user_info = user_info_addr as *mut c_void;
            let tap = CGEventTapCreate(
                K_CG_SESSION_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_LISTEN_ONLY,
                EVENT_MASK,
                tap_callback as *const c_void,
                user_info,
            );
            if tap.is_null() {
                tracing::warn!("CGEventTapCreate failed — Input Monitoring permission may not be granted");
                return;
            }
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            let rl = CFRunLoopGetCurrent();
            CFRunLoopAddSource(rl, source, kCFRunLoopDefaultMode);
            CGEventTapEnable(tap, true);
            CFRunLoopRun(); // blocks this thread forever
        });
    }
}

// ── Windows ──────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod windows {
    use super::{KeyEvent, KeyEventKind};

    pub fn start<F: Fn(KeyEvent) + Send + 'static>(callback: F) {
        // On Windows we still use rdev since the TSM issue is macOS-only.
        std::thread::spawn(move || {
            if let Err(e) = rdev::listen(move |event| {
                let kind = match event.event_type {
                    rdev::EventType::KeyPress(rdev::Key::Alt) => KeyEventKind::Down,
                    rdev::EventType::KeyRelease(rdev::Key::Alt) => KeyEventKind::Up,
                    _ => return,
                };
                callback(KeyEvent { kind });
            }) {
                tracing::warn!(error = ?e, "rdev listener stopped");
            }
        });
    }
}
