pub mod paste_impl;
pub mod permissions_impl;
pub mod window_impl;

/// Returns true when `event` is the key press or release used to trigger Dictate.
/// macOS uses Left Control (modifier-only, avoids conflicting with global shortcuts).
/// Windows uses Alt.
#[cfg(target_os = "macos")]
pub fn dictate_key_matches(event: &rdev::Event) -> bool {
    matches!(
        event.event_type,
        rdev::EventType::KeyPress(rdev::Key::ControlLeft)
            | rdev::EventType::KeyRelease(rdev::Key::ControlLeft)
    )
}

#[cfg(target_os = "windows")]
pub fn dictate_key_matches(event: &rdev::Event) -> bool {
    matches!(
        event.event_type,
        rdev::EventType::KeyPress(rdev::Key::Alt) | rdev::EventType::KeyRelease(rdev::Key::Alt)
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn dictate_key_matches(_event: &rdev::Event) -> bool {
    false
}

/// Open a file, optionally with a specific application.
/// On macOS `app` is either a bare app name ("Obsidian") or a full path ("/Applications/Obsidian.app").
/// On Windows `app` is the full path to the executable.
#[cfg(target_os = "macos")]
pub fn open_file(path: &str, app: Option<&str>) -> Result<(), String> {
    let mut cmd = std::process::Command::new("open");
    if let Some(a) = app {
        if !a.trim().is_empty() {
            cmd.arg("-a").arg(a);
        }
    }
    cmd.arg(path);
    let status = cmd
        .status()
        .map_err(|e| format!("failed to launch open: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("open exited with status {status}"))
    }
}

#[cfg(target_os = "windows")]
pub fn open_file(path: &str, app: Option<&str>) -> Result<(), String> {
    if let Some(a) = app {
        if !a.trim().is_empty() {
            let status = std::process::Command::new(a)
                .arg(path)
                .status()
                .map_err(|e| format!("failed to launch {a}: {e}"))?;
            return if status.success() {
                Ok(())
            } else {
                Err(format!("{a} exited with status {status}"))
            };
        }
    }
    // Fall back to shell default
    std::process::Command::new("cmd")
        .args(["/c", "start", "", path])
        .status()
        .map(|_| ())
        .map_err(|e| format!("failed to open file: {e}"))
}

#[cfg(target_os = "macos")]
pub fn get_default_output_device() -> Result<String, String> {
    let output = std::process::Command::new("/opt/homebrew/bin/SwitchAudioSource")
        .args(["-c", "-t", "output"])
        .output()
        .map_err(|e| format!("failed to query current output device: {e}"))?;
    if !output.status.success() {
        return Err("failed to query current output device".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "macos")]
pub fn set_default_output_device(device_name: &str) -> Result<(), String> {
    let status = std::process::Command::new("/opt/homebrew/bin/SwitchAudioSource")
        .args(["-s", device_name, "-t", "output"])
        .status()
        .map_err(|e| format!("failed to set output device `{device_name}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "switching output device to `{device_name}` failed with status {status}"
        ))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_default_output_device() -> Result<String, String> {
    Ok(String::new())
}

#[cfg(not(target_os = "macos"))]
pub fn set_default_output_device(_device_name: &str) -> Result<(), String> {
    Ok(())
}

/// Returns true if the named output device exists on the system.
/// Uses `system_profiler` on macOS; returns false on other platforms.
#[cfg(target_os = "macos")]
pub fn output_device_exists(device_name: &str) -> bool {
    let Ok(output) = std::process::Command::new("system_profiler")
        .args(["SPAudioDataType", "-json"])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).contains(device_name)
}

#[cfg(not(target_os = "macos"))]
pub fn output_device_exists(_device_name: &str) -> bool {
    false
}
