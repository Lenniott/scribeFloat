//! Puts `scribefloat-cli` (bundled inside the app as a Tauri externalBin sidecar — see
//! `tauri.conf.json` `bundle.externalBin` and `scripts/prepare-cli-sidecar.sh`) on the
//! user's PATH as `scribefloat`, the same way VS Code's `code` command or Docker
//! Desktop's `docker` CLI get installed: a symlink created by the GUI app itself, since
//! installers can't write into system PATH directories without privilege escalation.

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// Tauri removes the target suffix when packaging; development staging keeps it.
pub fn resolve_sidecar_at(
    executable_dir: &Path,
    staging_dir: &Path,
    triple: &str,
    name: &str,
) -> Option<PathBuf> {
    [
        executable_dir.join(name),
        staging_dir.join(format!("{name}-{triple}")),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// Where the symlink should live. `~/.local/bin` needs no sudo and is on PATH in
/// some shell setups. Users whose PATH omits it must add this directory themselves.
pub fn link_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/bin"))
}

/// What to do with the `scribefloat` symlink at `link_path`, given the sidecar binary this
/// install would point it at (`cli_target`).
#[derive(Debug, PartialEq, Eq)]
pub enum LinkAction {
    /// Nothing at `link_path` — create the symlink.
    Create,
    /// A symlink already exists and points here or at another scribefloat-cli sidecar
    /// (e.g. a previous app version) — safe to replace.
    Replace,
    /// `link_path` exists but isn't a scribefloat-managed symlink (a real file, or a
    /// symlink to something else entirely) — leave it alone rather than clobber whatever
    /// the user put there.
    Skip,
}

/// Decide what `ensure_cli_symlink` should do at `link_path`, given the sidecar binary this
/// install would point it at (`cli_target`) and what (if anything) already exists there.
///
/// `existing_link_target` is `Some(path)` if `link_path` is itself a symlink (resolved one
/// level, not canonicalized) and `None` if it doesn't exist or isn't a symlink at all —
/// `std::fs::read_link` returns `Err` for both "missing" and "not a symlink" cases, so the
/// caller collapses both into `None`.
///
/// A real file at `link_path` is left alone unconditionally: it's not ours to overwrite,
/// even if it happens to already point at a scribefloat-cli binary via some other means.
/// Only a *symlink* we can attribute to a previous run of this same function — one whose
/// target is the current CLI or an app-bundled `scribefloat-cli` (including the
/// old target-suffixed name) — is
/// treated as ours to replace (covers version upgrades, moved/renamed .app, and dangling
/// links left behind by an uninstalled older build).
pub fn plan_cli_symlink(
    link_path: &Path,
    cli_target: &Path,
    existing_link_target: Option<&Path>,
) -> LinkAction {
    match existing_link_target {
        None if !link_path.exists() => LinkAction::Create,
        None => LinkAction::Skip, // exists and isn't a symlink: a real file, leave it
        Some(prev) => {
            let cli_name = prev
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == "scribefloat-cli" || n.starts_with("scribefloat-cli-"));
            let app_binary = prev.parent().is_some_and(|dir| {
                dir.file_name().is_some_and(|name| name == "MacOS")
                    && dir.parent().is_some_and(|contents| {
                        contents.file_name().is_some_and(|name| name == "Contents")
                            && contents
                                .parent()
                                .is_some_and(|app| app.extension().is_some_and(|ext| ext == "app"))
                    })
            });
            let is_ours = prev == cli_target || (cli_name && app_binary);
            if is_ours {
                LinkAction::Replace
            } else {
                LinkAction::Skip
            }
        }
    }
}

/// Resolve the bundled `scribefloat-cli` sidecar path from the running app, run
/// `plan_cli_symlink`, and apply it. Best-effort: failures are logged, never fatal to app
/// startup (a user without a symlink still has a fully working GUI app).
#[cfg(target_os = "macos")]
pub fn ensure_cli_symlink(cli_target: &Path) {
    let Some(dir) = link_dir() else {
        tracing::warn!("scribefloat: no HOME; skipping CLI symlink setup");
        return;
    };
    if let Err(err) = std::fs::create_dir_all(&dir) {
        tracing::warn!("scribefloat: failed to create {}: {err}", dir.display());
        return;
    }
    let link_path = dir.join("scribefloat");
    let existing_link_target = std::fs::read_link(&link_path).ok();

    match plan_cli_symlink(&link_path, cli_target, existing_link_target.as_deref()) {
        LinkAction::Skip => {}
        LinkAction::Create => {
            if let Err(err) = symlink(cli_target, &link_path) {
                tracing::warn!(
                    "scribefloat: failed to create CLI symlink at {}: {err}",
                    link_path.display()
                );
            }
        }
        LinkAction::Replace => {
            if let Err(err) = std::fs::remove_file(&link_path) {
                tracing::warn!(
                    "scribefloat: failed to remove stale CLI symlink at {}: {err}",
                    link_path.display()
                );
                return;
            }
            if let Err(err) = symlink(cli_target, &link_path) {
                tracing::warn!(
                    "scribefloat: failed to recreate CLI symlink at {}: {err}",
                    link_path.display()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_packaged_name_without_source_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("scribefloat-cli");
        std::fs::write(&bundled, b"binary").unwrap();
        assert_eq!(
            resolve_sidecar_at(
                tmp.path(),
                &tmp.path().join("missing"),
                "test-target",
                "scribefloat-cli"
            ),
            Some(bundled)
        );
    }

    #[test]
    fn resolves_packaged_audio_helper_without_source_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        let helper = tmp.path().join("set-default-output");
        std::fs::write(&helper, b"binary").unwrap();
        assert_eq!(
            resolve_sidecar_at(
                tmp.path(),
                &tmp.path().join("missing"),
                "test-target",
                "set-default-output"
            ),
            Some(helper)
        );
    }

    #[test]
    fn resolves_staged_name_in_development() {
        let tmp = tempfile::tempdir().unwrap();
        let staged = tmp.path().join("scribefloat-cli-test-target");
        std::fs::write(&staged, b"binary").unwrap();
        assert_eq!(
            resolve_sidecar_at(
                &tmp.path().join("missing"),
                tmp.path(),
                "test-target",
                "scribefloat-cli"
            ),
            Some(staged)
        );
    }

    #[test]
    fn upgrades_unsuffixed_app_link_but_preserves_unrelated_same_name() {
        let link = Path::new("/unused/scribefloat");
        let target = Path::new("/Applications/New.app/Contents/MacOS/scribefloat-cli");
        assert_eq!(
            plan_cli_symlink(
                link,
                target,
                Some(Path::new(
                    "/Applications/Old.app/Contents/MacOS/scribefloat-cli"
                ))
            ),
            LinkAction::Replace
        );
        assert_eq!(
            plan_cli_symlink(link, target, Some(Path::new("/custom/bin/scribefloat-cli"))),
            LinkAction::Skip
        );
        assert_eq!(
            plan_cli_symlink(link, target, Some(target)),
            LinkAction::Replace
        );
    }

    #[test]
    fn creates_when_nothing_exists() {
        let link = PathBuf::from("/Users/x/.local/bin/scribefloat");
        let target = PathBuf::from(
            "/Applications/ScribeFloat.app/Contents/MacOS/scribefloat-cli-aarch64-apple-darwin",
        );
        assert_eq!(plan_cli_symlink(&link, &target, None), LinkAction::Create);
    }

    #[test]
    fn replaces_a_stale_scribefloat_symlink() {
        let link = PathBuf::from("/Users/x/.local/bin/scribefloat");
        let old_target = PathBuf::from(
            "/Applications/ScribeFloat.app/Contents/MacOS/scribefloat-cli-aarch64-apple-darwin",
        );
        let new_target = PathBuf::from(
            "/Applications/ScribeFloat 2.app/Contents/MacOS/scribefloat-cli-aarch64-apple-darwin",
        );
        assert_eq!(
            plan_cli_symlink(&link, &new_target, Some(&old_target)),
            LinkAction::Replace
        );
    }

    #[test]
    fn skips_a_symlink_pointing_somewhere_unrelated() {
        let link = PathBuf::from("/Users/x/.local/bin/scribefloat");
        let target = PathBuf::from("/Applications/ScribeFloat.app/Contents/MacOS/scribefloat-cli");
        let unrelated = PathBuf::from("/Users/x/bin/my-own-scribefloat-script");
        assert_eq!(
            plan_cli_symlink(&link, &target, Some(&unrelated)),
            LinkAction::Skip
        );
    }
}
