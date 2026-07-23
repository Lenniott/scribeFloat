//! Static guards for least-privilege IPC (ticket 16 / B+).
//!
//! These tests do not boot a webview. They assert capability files and the
//! `generate_handler!` list stay aligned with the ACL design.

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    /// High-impact commands satellites must never be granted.
    const SATELLITE_DENY_LIST: &[&str] = &[
        "history_delete",
        "scribe_start",
        "scribe_stop_and_save",
        "scribe_cancel",
        "transcribe_start",
        "transcribe_inspect_inputs",
        "transcribe_open_output",
        "settings_set_open_with_app_path",
        "settings_set_output_path",
        "note_create_empty",
        "update_check",
    ];

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read(rel: &str) -> String {
        fs::read_to_string(manifest_dir().join(rel)).unwrap_or_else(|e| {
            panic!("read {}: {e}", manifest_dir().join(rel).display())
        })
    }

    /// Permission ids are `allow-<kebab-cmd>`; return snake_case command names.
    fn allows_in_toml(toml: &str) -> BTreeSet<String> {
        toml.lines()
            .filter_map(|line| {
                let line = line.trim().trim_end_matches(',');
                let rest = line.strip_prefix('"')?.strip_suffix('"')?;
                let kebab = rest.strip_prefix("allow-")?;
                Some(kebab.replace('-', "_"))
            })
            .collect()
    }

    #[test]
    fn satellite_capabilities_omit_deny_list_commands() {
        let dictate = allows_in_toml(&read("permissions/sets/dictate-overlay.toml"));
        let onboarding = allows_in_toml(&read("permissions/sets/onboarding.toml"));
        for cmd in SATELLITE_DENY_LIST {
            assert!(
                !dictate.contains(*cmd),
                "dictate-overlay must not allow {cmd}"
            );
            assert!(
                !onboarding.contains(*cmd),
                "onboarding set must not allow {cmd}"
            );
        }
    }

    #[test]
    fn satellite_capabilities_do_not_reference_main_shell_or_plugins() {
        for name in ["dictate.json", "onboarding.json"] {
            let body = read(&format!("capabilities/{name}"));
            assert!(
                !body.contains("main-shell"),
                "{name} must not include main-shell"
            );
            assert!(
                !body.contains("opener:default"),
                "{name} must not include opener:default"
            );
            assert!(
                !body.contains("dialog:default"),
                "{name} must not include dialog:default"
            );
            assert!(
                !body.contains("clipboard-manager"),
                "{name} must not include clipboard-manager"
            );
        }
    }

    #[test]
    fn shell_capability_targets_history_window_only() {
        let body = read("capabilities/shell.json");
        assert!(body.contains("\"history\""));
        assert!(!body.contains("\"dictate\""));
        assert!(!body.contains("\"onboarding\""));
        assert!(body.contains("main-shell"));
    }

    #[test]
    fn build_rs_app_commands_match_generate_handler() {
        let lib = read("src/lib.rs");
        let start = lib
            .find("generate_handler![")
            .expect("generate_handler in lib.rs");
        let rest = &lib[start..];
        let end = rest.find(']').expect("closing ]");
        let block = &rest[..end];
        let mut handler: BTreeSet<String> = BTreeSet::new();
        for line in block.lines() {
            if let Some(name) = line.split("::").last() {
                let name = name.trim().trim_end_matches(',');
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_')
                {
                    handler.insert(name.to_string());
                }
            }
        }

        let build = read("build.rs");
        let start = build
            .find("const APP_COMMANDS")
            .expect("APP_COMMANDS in build.rs");
        let rest = &build[start..];
        // Skip past `&[&str] = &[` so we do not stop at the type's `]`.
        let list_start = rest
            .find("= &[")
            .expect("APP_COMMANDS array literal")
            + 4;
        let list = &rest[list_start..];
        let end = list.find(']').expect("closing ] for APP_COMMANDS");
        let block = &list[..end];
        let mut app: BTreeSet<String> = BTreeSet::new();
        for line in block.lines() {
            let line = line.trim().trim_end_matches(',');
            if let Some(inner) = line.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                if inner
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_')
                {
                    app.insert(inner.to_string());
                }
            }
        }

        assert_eq!(
            handler, app,
            "APP_COMMANDS in build.rs must match generate_handler! names.\nonly in handler: {:?}\nonly in build.rs: {:?}",
            handler.difference(&app).collect::<Vec<_>>(),
            app.difference(&handler).collect::<Vec<_>>()
        );
    }

    #[test]
    fn main_shell_set_includes_every_handler_command() {
        let lib = read("src/lib.rs");
        let start = lib.find("generate_handler![").unwrap();
        let rest = &lib[start..];
        let end = rest.find(']').unwrap();
        let block = &rest[..end];
        let mut handler: BTreeSet<String> = BTreeSet::new();
        for line in block.lines() {
            if let Some(name) = line.split("::").last() {
                let name = name.trim().trim_end_matches(',');
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_')
                {
                    handler.insert(name.to_string());
                }
            }
        }
        let shell = allows_in_toml(&read("permissions/sets/main-shell.toml"));
        assert_eq!(
            handler, shell,
            "main-shell permission set must list every handler command"
        );
    }
}
