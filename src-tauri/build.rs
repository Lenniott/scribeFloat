fn main() {
    #[cfg(target_os = "macos")]
    build_set_default_output_helper();

    // Listing commands here disables Tauri's default "every window may invoke every
    // command" and autogenerates allow-<cmd> / deny-<cmd> permissions. Grant those
    // via permission sets under permissions/sets/ and capabilities/*.json.
    // Keep this list in sync with generate_handler! in src/lib.rs (enforced by test).
    const APP_COMMANDS: &[&str] = &[
"scribe_start",
    "scribe_stop_and_save",
    "scribe_save_recording_only",
    "scribe_abort_transcription",
    "scribe_destroy_window",
    "scribe_cancel",
    "scribe_set_attach_note",
    "scribe_add_note",
    "scribe_get_include_timestamps",
    "scribe_set_include_timestamps",
    "scribe_list_input_devices",
    "scribe_list_output_devices",
    "scribe_read_transcript",
    "scribe_list_recovery_sessions",
    "scribe_list_transcripts",
    "scribe_switch_mic",
    "scribe_toggle_speaker_capture",
    "model_vad_status",
    "settings_get_output_path",
    "settings_set_output_path",
    "settings_get_hotkeys",
    "settings_set_hotkeys",
    "settings_get_input_labels",
    "settings_set_input_labels",
    "settings_get_preferred_audio_devices",
    "settings_set_preferred_audio_devices",
    "settings_list_output_devices",
    "settings_speaker_capture_requires_device_name",
    "settings_blackhole_detected",
    "settings_get_scribe_capture_speaker",
    "settings_set_scribe_capture_speaker",
    "settings_get_open_with_app_path",
    "settings_set_open_with_app_path",
    "settings_open_transcript",
    "settings_get_theme_mode",
    "settings_set_theme_mode",
    "settings_permissions_status",
    "settings_permissions_open",
    "settings_permissions_request",
    "settings_onboarding_status",
    "settings_complete_onboarding",
    "settings_reset_onboarding",
    "settings_show_window",
    "settings_show_onboarding_window",
    "settings_get_platform",
    "settings_open_scribe_window",
    "settings_get_dictate_auto_paste",
    "settings_set_dictate_auto_paste",
    "settings_get_dictate_auto_enter",
    "settings_set_dictate_auto_enter",
    "settings_get_keep_wav",
    "settings_set_keep_wav",
    "settings_get_save_transcripts_as_markdown",
    "settings_set_save_transcripts_as_markdown",
    "settings_get_user_display_name",
    "settings_set_user_display_name",
    "dictate_cancel",
    "dictate_dismiss",
    "dictate_get_history",
    "dictate_trigger",
    "dictate_get_state",
    "history_list",
    "history_get_detail",
    "history_render_markdown",
    "history_export_markdown",
    "history_delete",
    "history_read_legacy",
    "get_dashboard_stats",
    "history_tag_vocabulary",
    "note_create_empty",
    "note_save_written_content",
    "note_save_title",
    "note_is_empty",
    "note_has_metadata",
    "note_set_tags",
    "note_relabel_speaker",
    "speaker_names_list",
    "speaker_name_save",
    "speaker_name_delete",
    "note_attach_transcript",
    "note_render_transcript_html",
    "transcribe_inspect_inputs",
    "transcribe_start",
    "transcribe_open_output",
    "transcribe_show_window",
    "update_check",
    ];

    println!("cargo:rerun-if-changed=permissions");
    println!("cargo:rerun-if-changed=capabilities");

    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("failed to run tauri-build");
}

#[cfg(target_os = "macos")]
fn build_set_default_output_helper() {
    use std::path::PathBuf;
    use std::process::Command;

    let target = std::env::var("TARGET").expect("TARGET");
    println!("cargo:rustc-env=SCRIBEFLOAT_TARGET_TRIPLE={target}");
    println!("cargo:rerun-if-changed=Swift/SetDefaultOutput/main.swift");

    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("Swift/SetDefaultOutput/main.swift");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let dest = out_dir.join("set-default-output");

    let status = Command::new("swiftc")
        .args(["-O", "-o"])
        .arg(&dest)
        .arg(&src)
        .status()
        .expect("run swiftc");
    if !status.success() {
        panic!(
            "failed to compile set-default-output helper (is Xcode command line tools installed?)"
        );
    }

    // Runtime path for dev/debug builds (under target/, not src-tauri/binaries/).
    println!(
        "cargo:rustc-env=SCRIBEFLOAT_SET_DEFAULT_OUTPUT_HELPER={}",
        dest.display()
    );

    // Tauri `externalBin` validates triple-suffixed binaries at build time (dev and release).
    // Copy only when missing or stale so `tauri dev` does not rewrite an unchanged file every
    // rebuild and retrigger the file watcher in a loop.
    let bundle_dir = manifest_dir.join("binaries");
    std::fs::create_dir_all(&bundle_dir).expect("create binaries dir");
    let bundle_dest = bundle_dir.join(format!("set-default-output-{target}"));
    copy_helper_if_changed(&dest, &bundle_dest);
}

#[cfg(target_os = "macos")]
fn copy_helper_if_changed(src: &std::path::Path, dest: &std::path::Path) {
    use std::io::Read;

    let src_bytes = std::fs::read(src).expect("read compiled set-default-output helper");
    if dest.is_file() {
        let mut dest_bytes = Vec::new();
        std::fs::File::open(dest)
            .and_then(|mut f| f.read_to_end(&mut dest_bytes))
            .expect("read existing set-default-output bundle binary");
        if dest_bytes == src_bytes {
            return;
        }
    }
    std::fs::write(dest, src_bytes).expect("write set-default-output bundle binary");
}
