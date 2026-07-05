pub mod dictate_focus;
pub mod key_listener;
pub mod paste_impl;
pub mod permissions_impl;
pub mod window_impl;

const VOICE_CRYPTO_KEY_SERVICE: &str = "com.benjamin.scribefloat-v8.voice-embeddings-key";
const VOICE_CRYPTO_KEY_ACCOUNT: &str = "default";

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
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
static SET_DEFAULT_OUTPUT_HELPER: OnceLock<PathBuf> = OnceLock::new();

/// Path to the bundled `set-default-output` helper (compiled in build.rs, registered as externalBin).
#[cfg(target_os = "macos")]
pub fn init_set_default_output_helper(path: PathBuf) {
    let _ = SET_DEFAULT_OUTPUT_HELPER.set(path);
}

#[cfg(target_os = "macos")]
fn set_default_output_helper_path() -> Result<&'static Path, String> {
    SET_DEFAULT_OUTPUT_HELPER
        .get()
        .map(|p| p.as_path())
        .ok_or_else(|| "set-default-output helper not initialized".to_string())
}

#[cfg(target_os = "macos")]
pub fn resolve_set_default_output_helper() -> Option<PathBuf> {
    if let Some(path) = option_env!("SCRIBEFLOAT_SET_DEFAULT_OUTPUT_HELPER") {
        let helper = PathBuf::from(path);
        if helper.is_file() {
            return Some(helper);
        }
    }

    let triple = env!("SCRIBEFLOAT_TARGET_TRIPLE");
    let binary_name = format!("set-default-output-{triple}");

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let bundled = dir.join(&binary_name);
            if bundled.is_file() {
                return Some(bundled);
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev = manifest.join("binaries").join(&binary_name);
    if dev.is_file() {
        return Some(dev);
    }
    None
}

#[cfg(target_os = "macos")]
fn run_set_default_output_helper(args: &[&str]) -> Result<std::process::Output, String> {
    let bin = set_default_output_helper_path()?;
    std::process::Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run set-default-output helper: {e}"))
}

#[cfg(target_os = "macos")]
pub fn get_default_output_device() -> Result<String, String> {
    let output = run_set_default_output_helper(&["get-default-output"])?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("failed to query current output device: {stderr}"))
}

#[cfg(target_os = "macos")]
pub fn set_default_output_device(device_name: &str) -> Result<(), String> {
    let output = run_set_default_output_helper(&["set-default-output", device_name])?;
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

#[cfg(target_os = "macos")]
pub fn get_or_create_voice_crypto_key() -> Result<String, String> {
    match read_keychain_password(VOICE_CRYPTO_KEY_SERVICE, VOICE_CRYPTO_KEY_ACCOUNT) {
        Ok(value) if !value.trim().is_empty() => return Ok(value),
        Ok(_) => {}
        Err(err) if !err.contains("could not be found") => return Err(err),
        Err(_) => {}
    }

    let key = generate_base64_key()?;
    write_keychain_password(
        VOICE_CRYPTO_KEY_SERVICE,
        VOICE_CRYPTO_KEY_ACCOUNT,
        &key,
    )?;
    Ok(key)
}

#[cfg(not(target_os = "macos"))]
pub fn get_or_create_voice_crypto_key() -> Result<String, String> {
    Err("voice embedding encryption key storage is only implemented for macOS Keychain".to_string())
}

#[cfg(target_os = "macos")]
fn read_keychain_password(service: &str, account: &str) -> Result<String, String> {
    let output = std::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", service, "-a", account, "-w"])
        .output()
        .map_err(|e| format!("failed to read macOS Keychain: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(target_os = "macos")]
fn write_keychain_password(service: &str, account: &str, password: &str) -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-U",
            "-s",
            service,
            "-a",
            account,
            "-w",
            password,
        ])
        .status()
        .map_err(|e| format!("failed to write macOS Keychain: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("macOS Keychain write failed with status {status}"))
    }
}

#[cfg(target_os = "macos")]
fn generate_base64_key() -> Result<String, String> {
    use base64::Engine;
    use ring::rand::{SecureRandom, SystemRandom};

    let random = SystemRandom::new();
    let mut key = [0u8; 32];
    random
        .fill(&mut key)
        .map_err(|_| "failed to generate voice encryption key".to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(key))
}

#[cfg(target_os = "windows")]
pub fn default_open_scribe_hotkey() -> &'static str {
    "Alt+Shift+L"
}

#[cfg(not(target_os = "windows"))]
pub fn default_open_scribe_hotkey() -> &'static str {
    "CmdOrCtrl+Shift+L"
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
    let host = cpal::default_host();
    let mut inputs = host.input_devices().map_err(|e| e.to_string())?;
    let device = match preferred_name.filter(|n| !n.trim().is_empty()) {
        Some(name) => inputs
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .ok_or_else(|| format!("loopback input device `{name}` not found"))?,
        None => inputs
            .find(|d| {
                d.name()
                    .map(|n| n.to_ascii_lowercase().contains("blackhole"))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                "no BlackHole input device found — install BlackHole 2ch for speaker capture"
                    .to_string()
            })?,
    };
    let config = device.default_input_config().map_err(|e| e.to_string())?;
    Ok((device, config))
}

/// True when a persisted save folder should be rewritten on Windows startup.
#[cfg(target_os = "windows")]
pub fn windows_save_folder_needs_migration(path: &str) -> bool {
    path == "/tmp/transcripts_scribefloat"
        || path.starts_with("/tmp/")
        || (!path.is_empty() && !std::path::Path::new(path).has_root())
}

#[cfg(not(target_os = "windows"))]
pub fn windows_save_folder_needs_migration(_path: &str) -> bool {
    false
}

#[cfg(test)]
mod save_folder_tests {
    use super::windows_save_folder_needs_migration;

    #[test]
    fn legacy_unix_tmp_path_migration_flag() {
        #[cfg(target_os = "windows")]
        {
            assert!(windows_save_folder_needs_migration(
                "/tmp/transcripts_scribefloat"
            ));
            assert!(!windows_save_folder_needs_migration(
                r"C:\Users\me\Documents\transcripts_scribefloat"
            ));
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert!(!windows_save_folder_needs_migration(
                "/tmp/transcripts_scribefloat"
            ));
        }
    }
}
