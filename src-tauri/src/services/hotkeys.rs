use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub trait HotkeyRegistrar: Send + Sync {
    fn rebind(&self, open_scribe: &str, dictate: &str) -> Result<(), String>;
}

pub struct TauriHotkeyRegistrar {
    app: AppHandle,
    registered: Mutex<Vec<String>>,
}

impl TauriHotkeyRegistrar {
    pub fn new(app: AppHandle) -> Arc<dyn HotkeyRegistrar> {
        Arc::new(Self {
            app,
            registered: Mutex::new(Vec::new()),
        })
    }
}

impl HotkeyRegistrar for TauriHotkeyRegistrar {
    fn rebind(&self, open_scribe: &str, dictate: &str) -> Result<(), String> {
        let mut registered = self.registered.lock().unwrap();
        let previous = registered.clone();
        let _ = self.app.global_shortcut().unregister_all();

        if let Err(err) = self.app.global_shortcut().register(open_scribe) {
            for shortcut in &previous {
                let _ = self.app.global_shortcut().register(shortcut.as_str());
            }
            return Err(format!(
                "failed to register Open Scribe hotkey `{open_scribe}`: {err}"
            ));
        }

        let should_register_dictate = hotkey_has_non_modifier_key(dictate);
        if should_register_dictate {
            if let Err(err) = self.app.global_shortcut().register(dictate) {
                let _ = self.app.global_shortcut().unregister_all();
                for shortcut in &previous {
                    let _ = self.app.global_shortcut().register(shortcut.as_str());
                }
                return Err(format!(
                    "failed to register Dictate hotkey `{dictate}`: {err}"
                ));
            }
        }

        *registered = if should_register_dictate {
            vec![open_scribe.to_string(), dictate.to_string()]
        } else {
            vec![open_scribe.to_string()]
        };
        Ok(())
    }
}

pub struct HotkeyService {
    registrar: Arc<dyn HotkeyRegistrar>,
}

impl HotkeyService {
    pub fn new(registrar: Arc<dyn HotkeyRegistrar>) -> Arc<Self> {
        Arc::new(Self { registrar })
    }

    pub fn validate_pair(
        &self,
        open_scribe: &str,
        dictate: &str,
    ) -> Result<(String, String), String> {
        let open_scribe = validate_hotkey("Open Scribe", open_scribe, true)?;
        let dictate = validate_hotkey("Dictate", dictate, false)?;
        if open_scribe.conflict_key == dictate.conflict_key {
            return Err(
                "Open Scribe and Dictate hotkeys conflict after normalization; choose distinct shortcuts."
                    .to_string(),
            );
        }
        Ok((open_scribe.canonical, dictate.canonical))
    }

    pub fn rebind(&self, open_scribe: &str, dictate: &str) -> Result<(), String> {
        self.registrar.rebind(open_scribe, dictate)
    }
}

struct ValidatedHotkey {
    canonical: String,
    conflict_key: String,
}

fn validate_hotkey(
    label: &str,
    raw: &str,
    require_non_modifier_key: bool,
) -> Result<ValidatedHotkey, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} hotkey cannot be empty."));
    }

    let parts = trimmed.split('+').map(str::trim).collect::<Vec<_>>();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        return Err(format!(
            "{label} hotkey `{trimmed}` is invalid: empty hotkey segment."
        ));
    }

    let mut modifiers: BTreeSet<&'static str> = BTreeSet::new();
    let mut key: Option<String> = None;

    for part in &parts {
        if let Some(modifier) = parse_modifier(part) {
            if !modifiers.insert(modifier) {
                return Err(format!(
                    "{label} hotkey `{trimmed}` is invalid: duplicate modifier `{part}`."
                ));
            }
            continue;
        }

        if key.is_some() {
            return Err(format!(
                "{label} hotkey `{trimmed}` is invalid: include only one non-modifier key."
            ));
        }

        if !is_valid_key(part) {
            return Err(format!(
                "{label} hotkey `{trimmed}` is invalid: unsupported key `{part}`."
            ));
        }
        key = Some(part.to_ascii_uppercase());
    }

    let key = if let Some(key) = key {
        key
    } else if !require_non_modifier_key && !modifiers.is_empty() {
        String::new()
    } else {
        return Err(format!(
            "{label} hotkey `{trimmed}` is invalid: missing non-modifier key."
        ));
    };

    let mut canonical_parts = modifiers
        .iter()
        .map(|m| (*m).to_string())
        .collect::<Vec<_>>();
    if !key.is_empty() {
        canonical_parts.push(key.clone());
    }
    let canonical = canonical_parts.join("+");

    Ok(ValidatedHotkey {
        canonical: canonical.clone(),
        conflict_key: canonical,
    })
}

fn hotkey_has_non_modifier_key(hotkey: &str) -> bool {
    hotkey
        .split('+')
        .map(str::trim)
        .any(|part| !part.is_empty() && parse_modifier(part).is_none())
}

fn parse_modifier(token: &str) -> Option<&'static str> {
    let lower = token.to_ascii_lowercase();
    match lower.as_str() {
        "cmdorctrl" => Some("CmdOrCtrl"),
        "cmd" | "command" | "meta" | "super" => Some("Command"),
        "ctrl" | "control" => Some("Ctrl"),
        "alt" | "option" => Some("Alt"),
        "shift" => Some("Shift"),
        _ => None,
    }
}

fn is_valid_key(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        return byte.is_ascii_alphanumeric();
    }

    matches!(
        upper.as_str(),
        "SPACE"
            | "TAB"
            | "ENTER"
            | "RETURN"
            | "ESC"
            | "ESCAPE"
            | "BACKSPACE"
            | "DELETE"
            | "UP"
            | "DOWN"
            | "LEFT"
            | "RIGHT"
            | "HOME"
            | "END"
            | "PAGEUP"
            | "PAGEDOWN"
    ) || upper
        .strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n))
}

#[cfg(test)]
mod tests {
    use super::hotkey_has_non_modifier_key;

    #[test]
    fn detects_modifier_only_hotkey() {
        assert!(!hotkey_has_non_modifier_key("Ctrl"));
        assert!(!hotkey_has_non_modifier_key("Command+Shift"));
        assert!(hotkey_has_non_modifier_key("CmdOrCtrl+P"));
    }
}
