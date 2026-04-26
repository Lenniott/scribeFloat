pub mod permissions_impl;

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
