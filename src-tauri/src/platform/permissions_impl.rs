use anyhow::Result;

pub fn permission_granted(_kind: &str) -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn permission_settings_url(kind: &str) -> Option<&'static str> {
    match kind {
        "microphone" => Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"),
        "accessibility" => Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"),
        "input_monitoring" => {
            Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        }
        _ => None,
    }
}

#[cfg(target_os = "windows")]
pub fn permission_settings_url(kind: &str) -> Option<&'static str> {
    match kind {
        "microphone" => Some("ms-settings:privacy-microphone"),
        "accessibility" => Some("ms-settings:easeofaccess-display"),
        "input_monitoring" => Some("ms-settings:privacy"),
        _ => None,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn permission_settings_url(_kind: &str) -> Option<&'static str> {
    None
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
