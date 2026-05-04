pub mod key_listener;
pub mod paste_impl;
pub mod permissions_impl;
pub mod window_impl;

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
fn switch_audio_source_path() -> Result<std::path::PathBuf, String> {
    const CANDIDATES: &[&str] = &[
        "/opt/homebrew/bin/SwitchAudioSource", // Apple Silicon Homebrew
        "/usr/local/bin/SwitchAudioSource",    // Intel Homebrew
    ];
    for candidate in CANDIDATES {
        if std::path::Path::new(candidate).exists() {
            return Ok(std::path::PathBuf::from(candidate));
        }
    }
    Err("SwitchAudioSource not found (install via `brew install switchaudio-osx`)".to_string())
}

#[cfg(target_os = "macos")]
pub fn get_default_output_device() -> Result<String, String> {
    let bin = switch_audio_source_path()?;
    let output = std::process::Command::new(&bin)
        .args(["-c", "-t", "output"])
        .output()
        .map_err(|e| format!("failed to query current output device: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("failed to query current output device: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "macos")]
pub fn set_default_output_device(device_name: &str) -> Result<(), String> {
    let bin = switch_audio_source_path()?;
    let output = std::process::Command::new(&bin)
        .args(["-s", device_name, "-t", "output"])
        .output()
        .map_err(|e| format!("failed to set output device `{device_name}`: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "switching output device to `{device_name}` failed: {stderr}"
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
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return false;
    };
    // Walk SPAudioDataType[*].devices[*]._name for an exact match
    json["SPAudioDataType"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|section| section["devices"].as_array())
        .flatten()
        .filter_map(|device| device["_name"].as_str())
        .any(|name| name == device_name)
}

#[cfg(not(target_os = "macos"))]
pub fn output_device_exists(_device_name: &str) -> bool {
    false
}
