use tauri::Manager;

/// Keep the app's Dock/taskbar presence in sync with visible windows.
pub fn sync_activation_policy(app: &tauri::AppHandle) {
    let has_visible_window = app
        .webview_windows()
        .iter()
        .filter(|(label, _)| label.as_str() != crate::DICTATE_WINDOW_LABEL)
        .any(|(_, w)| w.is_visible().unwrap_or(false));

    set_has_visible_windows(app, has_visible_window);
}

#[cfg(target_os = "macos")]
pub fn set_has_visible_windows(app: &tauri::AppHandle, has_visible_window: bool) {
    // The dictate HUD is intentionally excluded from Dock visibility — it is a
    // floating overlay, not a user-facing content window. Only Scribe/Settings
    // windows should cause the Dock icon to appear.
    // LSUIElement=true in Info.plist keeps us menu-bar-only by default; this
    // call makes the Dock icon appear/disappear as real windows open/close.
    if let Err(err) = app.set_dock_visibility(has_visible_window) {
        eprintln!("failed to update macOS Dock visibility: {err}");
    }
    if has_visible_window {
        refresh_dock_icon(app);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_has_visible_windows(_app: &tauri::AppHandle, _has_visible_window: bool) {}

#[cfg(target_os = "macos")]
fn refresh_dock_icon(app: &tauri::AppHandle) {
    let Some(icon_path) = find_icon_path(app) else {
        return;
    };
    macos::set_application_icon(&icon_path);
}

#[cfg(target_os = "macos")]
fn find_icon_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let resource_icon = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("icons/icon.icns"));
    let current_dir = std::env::current_dir().ok();
    let cwd_src_icon = current_dir
        .as_ref()
        .map(|dir| dir.join("src-tauri/icons/icon.icns"));
    let cwd_icon = current_dir.as_ref().map(|dir| dir.join("icons/icon.icns"));

    [resource_icon, cwd_src_icon, cwd_icon]
        .into_iter()
        .flatten()
        .find(|path| path.exists())
}

#[cfg(target_os = "macos")]
mod macos {
    use std::{
        ffi::{c_char, c_void, CString},
        path::Path,
    };

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
        fn objc_msg_send_id_cstr(
            receiver: *mut c_void,
            op: *mut c_void,
            arg: *const c_char,
        ) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn objc_msg_send_id_id(
            receiver: *mut c_void,
            op: *mut c_void,
            arg: *mut c_void,
        ) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn objc_msg_send_void_id(receiver: *mut c_void, op: *mut c_void, arg: *mut c_void);
    }

    pub fn set_application_icon(path: &Path) {
        let Some(path) = path.to_str().and_then(|path| CString::new(path).ok()) else {
            return;
        };

        unsafe {
            let app_cls = objc_getClass(b"NSApplication\0".as_ptr() as *const c_char);
            let image_cls = objc_getClass(b"NSImage\0".as_ptr() as *const c_char);
            let string_cls = objc_getClass(b"NSString\0".as_ptr() as *const c_char);
            if app_cls.is_null() || image_cls.is_null() || string_cls.is_null() {
                return;
            }

            let shared_sel = sel_registerName(b"sharedApplication\0".as_ptr() as *const c_char);
            let alloc_sel = sel_registerName(b"alloc\0".as_ptr() as *const c_char);
            let init_file_sel =
                sel_registerName(b"initWithContentsOfFile:\0".as_ptr() as *const c_char);
            let string_sel = sel_registerName(b"stringWithUTF8String:\0".as_ptr() as *const c_char);
            let set_icon_sel =
                sel_registerName(b"setApplicationIconImage:\0".as_ptr() as *const c_char);
            if shared_sel.is_null()
                || alloc_sel.is_null()
                || init_file_sel.is_null()
                || string_sel.is_null()
                || set_icon_sel.is_null()
            {
                return;
            }

            let app = objc_msg_send_id(app_cls, shared_sel);
            let path_string = objc_msg_send_id_cstr(string_cls, string_sel, path.as_ptr());
            let image_alloc = objc_msg_send_id(image_cls, alloc_sel);
            let image = objc_msg_send_id_id(image_alloc, init_file_sel, path_string);
            if !app.is_null() && !image.is_null() {
                objc_msg_send_void_id(app, set_icon_sel, image);
            }
        }
    }
}
