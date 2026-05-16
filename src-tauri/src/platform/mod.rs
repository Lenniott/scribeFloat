pub mod dictate_focus;
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

/// Returns the default modifier key used to activate push-to-talk dictation.
/// Windows uses Alt to avoid conflicting with common Ctrl shortcuts.
/// macOS uses Ctrl, which is rarely bound by apps and works well as a hold key.
#[cfg(target_os = "windows")]
pub fn default_dictate_activation_key() -> &'static str {
    "Alt"
}

#[cfg(not(target_os = "windows"))]
pub fn default_dictate_activation_key() -> &'static str {
    "Ctrl"
}

/// Returns the device and stream config to use for loopback (speaker) capture.
///
/// Windows — WASAPI loopback: open the selected (or default) output device and
/// build an input stream on it. The output config is used because the loopback
/// format mirrors the output device's native format.
///
/// macOS — virtual loopback input: open the named input device (e.g. BlackHole 2ch).
/// The caller is responsible for having already routed system audio through it.
#[cfg(target_os = "windows")]
pub fn loopback_device_and_config(
    preferred_name: Option<&str>,
) -> Result<(cpal::Device, cpal::SupportedStreamConfig), String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let device = match preferred_name.filter(|n| !n.trim().is_empty()) {
        Some(name) => host
            .output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .ok_or_else(|| format!("output device `{name}` not found for loopback capture"))?,
        None => host
            .default_output_device()
            .ok_or_else(|| "no default output device available for loopback capture".to_string())?,
    };
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    Ok((device, config))
}

#[cfg(not(target_os = "windows"))]
pub fn loopback_device_and_config(
    preferred_name: Option<&str>,
) -> Result<(cpal::Device, cpal::SupportedStreamConfig), String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let name = preferred_name
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| {
            "no loopback input device configured — select a virtual audio device (e.g. BlackHole 2ch) as the speaker source".to_string()
        })?;
    let host = cpal::default_host();
    let device = host
        .input_devices()
        .map_err(|e| e.to_string())?
        .find(|d| d.name().map(|n| n == name).unwrap_or(false))
        .ok_or_else(|| format!("loopback input device `{name}` not found"))?;
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    Ok((device, config))
}
