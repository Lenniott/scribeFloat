use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Carbon `kVK_ANSI_V` — physical key used for the standard Paste shortcut (⌘V).
#[cfg(target_os = "macos")]
const MACOS_KEYCODE_ANSI_V: u16 = 0x09;

/// Simulate Cmd+V (macOS) or Ctrl+V (Windows) to paste clipboard contents
/// into the currently focused application.
///
/// The caller is responsible for writing the text to the clipboard before
/// calling this function. Requires Accessibility permission on macOS.
#[cfg(target_os = "macos")]
pub fn paste_text() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| e.to_string())?;
    // Use layout-independent keycode + enigo's tracked Command flag (not Unicode),
    // so paste is a real shortcut rather than typing "v".
    enigo
        .raw(MACOS_KEYCODE_ANSI_V, Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn paste_text() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Simulate pressing Enter in the focused application.
pub fn send_enter() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Return, Direction::Click)
        .map_err(|e| e.to_string())?;
    Ok(())
}
