mod commands;
mod controllers;
mod platform;
mod services;
mod types;

use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

/// One-shot startup log. Captures the CPU, core counts, RAM, arch, and app version so
/// user-reported performance issues can be correlated with their hardware without us
/// having to play 20 questions. Stays in stderr — no telemetry leaves the device.
fn log_system_info() {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let version = env!("CARGO_PKG_VERSION");
    let physical = num_cpus::get_physical();
    let logical = num_cpus::get();
    let cpu_brand = cpu_brand_string().unwrap_or_else(|| "unknown".to_string());
    let ram_gb = total_ram_bytes()
        .map(|b| format!("{:.1} GB", b as f64 / 1_073_741_824.0))
        .unwrap_or_else(|| "?".to_string());

    tracing::info!(
        version, os, arch, cpu = cpu_brand, cores_physical = physical, cores_logical = logical,
        ram = ram_gb, "scribefloat startup"
    );
}

#[cfg(target_os = "macos")]
fn cpu_brand_string() -> Option<String> {
    sysctl_string(c"machdep.cpu.brand_string")
}

#[cfg(target_os = "macos")]
fn total_ram_bytes() -> Option<u64> {
    sysctl_u64(c"hw.memsize")
}

#[cfg(target_os = "macos")]
fn sysctl_string(name: &std::ffi::CStr) -> Option<String> {
    let mut len: libc::size_t = 0;
    // First call: ask for the length.
    // SAFETY: passing null buffer to sysctlbyname is the documented way to query length.
    let rc = unsafe { libc::sysctlbyname(name.as_ptr(), std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) };
    if rc != 0 || len == 0 {
        return None;
    }
    let mut buf = vec![0u8; len];
    // SAFETY: buf has `len` bytes; sysctlbyname fills it and updates `len`.
    let rc = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return None;
    }
    // sysctl strings include a trailing NUL.
    if let Some(&0) = buf.last() {
        buf.pop();
    }
    String::from_utf8(buf).ok()
}

#[cfg(target_os = "macos")]
fn sysctl_u64(name: &std::ffi::CStr) -> Option<u64> {
    let mut value: u64 = 0;
    let mut len: libc::size_t = std::mem::size_of::<u64>();
    // SAFETY: `value` is properly sized for an integer sysctl read.
    let rc = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut value as *mut u64 as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc == 0 {
        Some(value)
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
fn cpu_brand_string() -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
fn total_ram_bytes() -> Option<u64> {
    None
}

pub(crate) const SCRIBE_WINDOW_LABEL: &str = "scribe";
const TRANSCRIBE_WINDOW_LABEL: &str = "transcribe";
const SETTINGS_WINDOW_LABEL: &str = "settings";
pub(crate) const DICTATE_WINDOW_LABEL: &str = "dictate";
const HISTORY_WINDOW_LABEL: &str = "history";

const OPEN_SCRIBE_MENU_ID: &str = "open_scribe";
const OPEN_TRANSCRIBE_MENU_ID: &str = "open_transcribe";
const OPEN_HISTORY_MENU_ID: &str = "open_history";
const OPEN_SETTINGS_MENU_ID: &str = "open_settings";
const QUIT_MENU_ID: &str = "quit";

const SCRIBE_WINDOW_W: f64 = 800.0;
const SCRIBE_WINDOW_H: f64 = 600.0;
const TRANSCRIBE_WINDOW_W: f64 = 800.0;
const TRANSCRIBE_WINDOW_H: f64 = 600.0;
const SETTINGS_WINDOW_W: f64 = 960.0;
const SETTINGS_WINDOW_H: f64 = 680.0;
const HISTORY_WINDOW_W: f64 = 480.0;
const HISTORY_WINDOW_H: f64 = 600.0;
const DICTATE_WINDOW_W: f64 = 240.0;
const DICTATE_WINDOW_H: f64 = 48.0;
/// Margin from the right and top edge of the primary monitor.
const DICTATE_MARGIN_RIGHT: f64 = 16.0;
const DICTATE_MARGIN_TOP: f64 = 28.0;

fn resolve_icon_path(app: &tauri::AppHandle, file_name: &str) -> Option<std::path::PathBuf> {
    let resource_icon = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("icons").join(file_name));
    let current_dir = std::env::current_dir().ok();
    let cwd_src_icon = current_dir
        .as_ref()
        .map(|dir| dir.join("src-tauri/icons").join(file_name));
    let cwd_icon = current_dir
        .as_ref()
        .map(|dir| dir.join("icons").join(file_name));

    [resource_icon, cwd_src_icon, cwd_icon]
        .into_iter()
        .flatten()
        .find(|path| path.exists())
}

fn load_icon(app: &tauri::AppHandle, file_name: &str) -> Option<Image<'static>> {
    let path = resolve_icon_path(app, file_name)?;
    Image::from_path(path).ok()
}

fn create_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let open_scribe =
        MenuItem::with_id(app, OPEN_SCRIBE_MENU_ID, "Scribe", true, None::<&str>)?;
    let open_transcribe = MenuItem::with_id(
        app,
        OPEN_TRANSCRIBE_MENU_ID,
        "Transcribe",
        true,
        None::<&str>,
    )?;
    let open_history = MenuItem::with_id(
        app,
        OPEN_HISTORY_MENU_ID,
        "History",
        true,
        None::<&str>,
    )?;
    let open_settings = MenuItem::with_id(
        app,
        OPEN_SETTINGS_MENU_ID,
        "Settings",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open_scribe, &open_transcribe, &open_history, &open_settings, &quit],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_SCRIBE_MENU_ID => {
                if let Err(err) = open_scribe_window(app) {
                    tracing::warn!(error = %err, "failed to open scribe window");
                }
            }
            OPEN_TRANSCRIBE_MENU_ID => {
                if let Err(err) = open_transcribe_window(app) {
                    tracing::warn!(error = %err, "failed to open transcribe window");
                }
            }
            OPEN_HISTORY_MENU_ID => {
                if let Err(err) = open_history_window(app) {
                    tracing::warn!(error = %err, "failed to open history window");
                }
            }
            OPEN_SETTINGS_MENU_ID => {
                if let Err(err) = open_settings_window(app) {
                    tracing::warn!(error = %err, "failed to open settings window");
                }
            }
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        });

    #[cfg(not(target_os = "macos"))]
    {
        let preferred_tray_icon = "sf_Transparent_tray_32x32.png";
        if let Some(icon) =
            load_icon(app.handle(), preferred_tray_icon).or_else(|| app.default_window_icon().cloned())
        {
            tray = tray.icon(icon.clone());
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Prefer the generated app icon so tray updates immediately after icon regeneration.
        if let Some(icon) =
            load_icon(app.handle(), "icon.png").or_else(|| app.default_window_icon().cloned())
        {
            tray = tray.icon(icon);
        }
        // Keep original colors; template mode can mask icon changes.
        tray = tray.icon_as_template(false);
    }

    tray.build(app)?;
    Ok(())
}

fn prewarm_scribe_window(app: &AppHandle) {
    let result: tauri::Result<()> = (|| {
        let url = WebviewUrl::App("index.html".into());
        let mut builder = WebviewWindowBuilder::new(app, SCRIBE_WINDOW_LABEL, url)
            .title("Scribe")
            .inner_size(SCRIBE_WINDOW_W, SCRIBE_WINDOW_H)
            .visible(false);
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }
        let window = builder.build()?;
        // `visible(false)` alone can still leave the webview reported visible on macOS until hide().
        let _ = window.hide();
        Ok(())
    })();
    if let Err(err) = result {
        tracing::debug!(error = %err, "failed to prewarm scribe window");
    }
}

fn prewarm_transcribe_window(app: &AppHandle) {
    let result: tauri::Result<()> = (|| {
        let url = WebviewUrl::App("?view=transcribe".into());
        let mut builder = WebviewWindowBuilder::new(app, TRANSCRIBE_WINDOW_LABEL, url)
            .title("Transcribe")
            .inner_size(TRANSCRIBE_WINDOW_W, TRANSCRIBE_WINDOW_H)
            .visible(false);
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }
        builder.build()?;
        Ok(())
    })();
    if let Err(err) = result {
        tracing::debug!(error = %err, "failed to prewarm transcribe window");
    }
}

fn prewarm_dictate_window(app: &AppHandle) {
    let result: tauri::Result<()> = (|| {
        let (x, y) = primary_monitor_dictate_position(app);
        WebviewWindowBuilder::new(
            app,
            DICTATE_WINDOW_LABEL,
            WebviewUrl::App("?view=dictate".into()),
        )
        .inner_size(DICTATE_WINDOW_W, DICTATE_WINDOW_H)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(true)
        .position(x, y)
        .visible(false)
        .build()?;
        Ok(())
    })();
    if let Err(err) = result {
        tracing::debug!(error = %err, "failed to prewarm dictate window");
    }
}

pub(crate) fn open_scribe_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let window = open_or_focus_window(
        app,
        SCRIBE_WINDOW_LABEL,
        "Scribe",
        WebviewUrl::App("index.html".into()),
        SCRIBE_WINDOW_W,
        SCRIBE_WINDOW_H,
    )?;
    Ok(window)
}

pub(crate) fn open_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    open_or_focus_window(
        app,
        SETTINGS_WINDOW_LABEL,
        "Settings",
        WebviewUrl::App("?view=settings".into()),
        SETTINGS_WINDOW_W,
        SETTINGS_WINDOW_H,
    )
}

pub(crate) fn open_transcribe_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    open_or_focus_window(
        app,
        TRANSCRIBE_WINDOW_LABEL,
        "Transcribe",
        WebviewUrl::App("?view=transcribe".into()),
        TRANSCRIBE_WINDOW_W,
        TRANSCRIBE_WINDOW_H,
    )
}

fn open_history_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    open_or_focus_window(
        app,
        HISTORY_WINDOW_LABEL,
        "History",
        WebviewUrl::App("?view=history".into()),
        HISTORY_WINDOW_W,
        HISTORY_WINDOW_H,
    )
}

pub(crate) fn open_dictate_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(DICTATE_WINDOW_LABEL) {
        let (x, y) = primary_monitor_dictate_position(app);
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        window.show()?;
        return Ok(window);
    }

    let (x, y) = primary_monitor_dictate_position(app);
    let window = WebviewWindowBuilder::new(
        app,
        DICTATE_WINDOW_LABEL,
        WebviewUrl::App("?view=dictate".into()),
    )
    .inner_size(DICTATE_WINDOW_W, DICTATE_WINDOW_H)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(true)
    .position(x, y)
    .visible(true)
    .build()?;
    Ok(window)
}

fn primary_monitor_dictate_position(app: &AppHandle) -> (f64, f64) {
    let (width, scale) = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let size = m.size();
            let sf = m.scale_factor();
            (size.width as f64 / sf, sf)
        })
        .unwrap_or((1440.0, 1.0));
    let _ = scale;
    let x = width - DICTATE_WINDOW_W - DICTATE_MARGIN_RIGHT;
    let y = DICTATE_MARGIN_TOP;
    (x, y)
}


/// Show, restore, and focus. On Windows, `show()` applies visibility asynchronously; a deferred
/// `set_focus` runs after so Tao sees `VISIBLE` and can call `SetForegroundWindow`.
fn raise_webview_window(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;

    #[cfg(target_os = "windows")]
    {
        let label = window.label().to_string();
        let app_handle = app.clone();
        app.run_on_main_thread(move || {
            if let Some(w) = app_handle.get_webview_window(&label) {
                let _ = w.set_focus();
            }
        })?;
    }

    #[cfg(not(target_os = "windows"))]
    let _ = app;

    Ok(())
}

fn open_or_focus_window(
    app: &AppHandle,
    label: &str,
    title: &str,
    url: WebviewUrl,
    width: f64,
    height: f64,
) -> tauri::Result<WebviewWindow> {
    let window = if let Some(window) = app.get_webview_window(label) {
        if let Some(icon) = app.default_window_icon() {
            window.set_icon(icon.clone())?;
        }
        raise_webview_window(app, &window)?;
        window
    } else {
        let mut builder = WebviewWindowBuilder::new(app, label, url)
            .title(title)
            .inner_size(width, height);

        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }

        let window = builder.build()?;
        raise_webview_window(app, &window)?;
        window
    };

    platform::window_impl::set_has_visible_windows(app, true);
    Ok(window)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    log_system_info();

    let audio = services::audio::AudioService::new();
    let output = services::output::OutputService::new();
    let history = services::history::HistoryService::new();
    let permissions = services::permissions::PermissionsService::new();
    let transcribe_input = services::transcribe_input::TranscribeInputService::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let Some(config) =
                        app.try_state::<Arc<services::config::ConfigService>>()
                    else {
                        return;
                    };
                    let scribe_str = config.get().open_scribe_hotkey.clone();
                    if let Ok(scribe_sc) =
                        scribe_str.parse::<tauri_plugin_global_shortcut::Shortcut>()
                    {
                        if shortcut.id() == scribe_sc.id() {
                            let handle = app.clone();
                            let _ = app.run_on_main_thread(move || {
                                open_scribe_window(&handle).ok();
                            });
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            create_tray(app)?;

            let data_dir = app.path().app_data_dir()?;
            #[cfg(target_os = "macos")]
            if let Some(helper) = platform::resolve_set_default_output_helper() {
                platform::init_set_default_output_helper(helper);
            } else {
                tracing::warn!("set-default-output helper missing; speaker capture output restore may fail");
            }
            let config = services::config::ConfigService::load(data_dir.join("config.json"))?;
            {
                let save_folder = config.get().save_folder;
                if platform::windows_save_folder_needs_migration(&save_folder) {
                    let migrated = crate::types::Config::default().save_folder;
                    if let Ok(normalized) =
                        output.ensure_output_dir(std::path::Path::new(&migrated))
                    {
                        let normalized = normalized.to_string_lossy().to_string();
                        config
                            .update(|cfg| cfg.save_folder = normalized)
                            .map_err(|e| format!("failed to migrate save folder: {e}"))?;
                    }
                }
            }
            let hotkeys = services::hotkeys::HotkeyService::new(
                services::hotkeys::TauriHotkeyRegistrar::new(app.handle().clone()),
            );

            let is_first_run = !config.get().onboarding_complete;

            let models_dir = data_dir.join("models");
            std::fs::create_dir_all(&models_dir)?;
            // Seed the bundled base model into the user's models dir on first install.
            // Silently skipped in dev builds where the resource file isn't present.
            let base_dest = models_dir.join("ggml-base.en-q5_1.bin");
            if !base_dest.exists() {
                if let Ok(resource_dir) = app.path().resource_dir() {
                    let bundled = resource_dir.join("ggml-base.en-q5_1.bin");
                    if bundled.is_file() {
                        let _ = std::fs::copy(&bundled, &base_dest);
                    }
                }
            }
            let model = services::model::ModelService::new(models_dir);
            let model_ctrl =
                controllers::model::ModelController::new(Arc::clone(&model), Arc::clone(&config));
            let settings_ctrl = controllers::settings::SettingsController::new(
                Arc::clone(&config),
                Arc::clone(&hotkeys),
                Arc::clone(&output),
                Arc::clone(&permissions),
                Arc::clone(&audio),
            );
            if let Err(err) = settings_ctrl.rehydrate_hotkeys() {
                tracing::debug!(error = %err, "hotkey rehydration skipped");
            }

            let ctrl = controllers::scribe::ScribeController::new(
                Arc::clone(&audio),
                Arc::clone(&model),
                Arc::clone(&output),
                Arc::clone(&history),
                Arc::clone(&config),
                app.handle().clone(),
            );

            let dictate_ctrl = controllers::dictate::DictateController::new(
                Arc::clone(&audio),
                Arc::clone(&model),
                Arc::clone(&output),
                Arc::clone(&history),
                Arc::clone(&config),
                app.handle().clone(),
            );
            let transcribe_ctrl = controllers::transcribe::TranscribeController::new(
                Arc::clone(&transcribe_input),
                Arc::clone(&model),
                Arc::clone(&output),
                Arc::clone(&history),
                Arc::clone(&config),
                app.handle().clone(),
            );
            let history_ctrl = controllers::history::HistoryController::new(
                Arc::clone(&history),
                Arc::clone(&output),
                Arc::clone(&config),
            );

            let update = services::update::UpdateService::new();

            app.manage(model); // shared model service
            app.manage(config); // shared config service
            app.manage(model_ctrl); // model command orchestration
            app.manage(settings_ctrl); // settings orchestration
            app.manage(ctrl); // for scribe commands
            app.manage(Arc::clone(&dictate_ctrl)); // for dictate commands
            app.manage(Arc::clone(&transcribe_ctrl)); // for transcribe commands
            app.manage(history_ctrl); // for history commands
            app.manage(update);

            dictate_ctrl.start_key_listener();

            if is_first_run {
                open_settings_window(app.handle())?;
                app.state::<Arc<controllers::settings::SettingsController>>()
                    .complete_onboarding()
                    .ok();
            }
            prewarm_scribe_window(app.handle());
            prewarm_transcribe_window(app.handle());
            prewarm_dictate_window(app.handle());
            // Tao applies Regular activation at launch; `set_dock_visibility(false)` only runs when we
            // call it. Sync once after prewarm so a tray-only start hides the Dock (plist LSUIElement
            // is not sufficient on its own).
            platform::window_impl::sync_activation_policy(app.handle());

            let save_folder = app.state::<Arc<services::config::ConfigService>>().get().save_folder;
            // Run compaction and recovery scans in the background so they never block
            // the Tauri event loop at startup (large histories can take 100-500ms).
            let history_bg = Arc::clone(&history);
            let output_bg = Arc::clone(&output);
            let save_folder_bg = save_folder.clone();
            let temp_dir_bg = app.path().app_local_data_dir().ok().map(|d| d.join("dictate_temp"));
            tokio::spawn(async move {
                if let Err(e) = history_bg.compact(&save_folder_bg) {
                    tracing::warn!(error = %e, "startup history compaction skipped");
                }
                match output_bg.scan_incomplete_scribe_sessions(&save_folder_bg) {
                    Ok(sessions) => {
                        for info in &sessions {
                            tracing::info!(
                                session_dir = %info.session_dir, state = %info.state,
                                "incomplete scribe session found at startup"
                            );
                        }
                    }
                    Err(e) => tracing::warn!(error = %e, "scribe session scan failed"),
                }
                if let Some(temp_dir) = temp_dir_bg {
                    match output_bg.scan_and_salvage_dictate_temp_wavs(&temp_dir, &save_folder_bg) {
                        Ok(salvaged) => {
                            for path in salvaged {
                                tracing::info!(path = %path.display(), "salvaged dictate wav");
                            }
                        }
                        Err(e) => tracing::warn!(error = %e, "dictate temp scan failed"),
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == SCRIBE_WINDOW_LABEL && matches!(event, WindowEvent::Destroyed) {
                // Release mic/speaker streams if the webview is torn down before invoke(`scribe_cancel`)
                // completes (crash or exceptional teardown — normal close uses hide, not destroy).
                if let Some(ctrl) = window
                    .app_handle()
                    .try_state::<Arc<controllers::scribe::ScribeController>>()
                {
                    let _ = ctrl.cancel();
                }
                platform::window_impl::sync_activation_policy(window.app_handle());
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == SCRIBE_WINDOW_LABEL {
                    api.prevent_close();
                    let _ = window.emit(
                        "scribe://native-close-requested",
                        serde_json::json!({}),
                    );
                    platform::window_impl::sync_activation_policy(window.app_handle());
                    return;
                }

                api.prevent_close();
                if let Err(err) = window.hide() {
                    tracing::debug!(label = window.label(), error = %err, "failed to hide window");
                }
                // Dictate is a HUD overlay — its close should never affect Dock visibility.
                if window.label() != DICTATE_WINDOW_LABEL {
                    platform::window_impl::sync_activation_policy(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::scribe::scribe_start,
            commands::scribe::scribe_stop_and_save,
            commands::scribe::scribe_save_recording_only,
            commands::scribe::scribe_abort_transcription,
            commands::scribe::scribe_destroy_window,
            commands::scribe::scribe_cancel,
            commands::scribe::scribe_add_note,
            commands::scribe::scribe_get_include_timestamps,
            commands::scribe::scribe_set_include_timestamps,
            commands::scribe::scribe_list_input_devices,
            commands::scribe::scribe_list_output_devices,
            commands::scribe::scribe_read_transcript,
            commands::scribe::scribe_list_recovery_sessions,
            commands::scribe::scribe_list_transcripts,
            commands::scribe::scribe_toggle_speaker_capture,
            commands::model::model_setup_status,
            commands::model::model_list,
            commands::model::model_download,
            commands::model::model_select,
            commands::model::model_remove,
            commands::model::model_vad_status,
            commands::model::model_vad_download,
            commands::model::model_vad_remove,
            commands::settings::settings_get_output_path,
            commands::settings::settings_set_output_path,
            commands::settings::settings_get_hotkeys,
            commands::settings::settings_set_hotkeys,
            commands::settings::settings_get_input_labels,
            commands::settings::settings_set_input_labels,
            commands::settings::settings_get_preferred_audio_devices,
            commands::settings::settings_set_preferred_audio_devices,
            commands::settings::settings_list_output_devices,
            commands::settings::settings_speaker_capture_requires_device_name,
            commands::settings::settings_blackhole_detected,
            commands::settings::settings_get_scribe_capture_speaker,
            commands::settings::settings_set_scribe_capture_speaker,
            commands::settings::settings_get_open_with_app_path,
            commands::settings::settings_set_open_with_app_path,
            commands::settings::settings_open_transcript,
            commands::settings::settings_get_theme_mode,
            commands::settings::settings_set_theme_mode,
            commands::settings::settings_permissions_status,
            commands::settings::settings_permissions_open,
            commands::settings::settings_permissions_request,
            commands::settings::settings_onboarding_status,
            commands::settings::settings_complete_onboarding,
            commands::settings::settings_reset_onboarding,
            commands::settings::settings_show_window,
            commands::settings::settings_get_dictate_auto_paste,
            commands::settings::settings_set_dictate_auto_paste,
            commands::settings::settings_get_dictate_auto_enter,
            commands::settings::settings_set_dictate_auto_enter,
            commands::settings::settings_get_keep_wav,
            commands::settings::settings_set_keep_wav,
            commands::settings::settings_get_save_transcripts_as_markdown,
            commands::settings::settings_set_save_transcripts_as_markdown,
            commands::settings::settings_get_dictate_model_id,
            commands::settings::settings_set_dictate_model_id,
            commands::settings::settings_get_replacement_rules,
            commands::settings::settings_add_replacement_rule,
            commands::settings::settings_update_replacement_rule,
            commands::settings::settings_delete_replacement_rule,
            commands::dictate::dictate_cancel,
            commands::dictate::dictate_dismiss,
            commands::dictate::dictate_get_history,
            commands::history::history_list,
            commands::history::history_get_detail,
            commands::history::history_render_markdown,
            commands::history::history_export_markdown,
            commands::history::history_delete,
            commands::history::history_read_legacy,
            commands::transcribe::transcribe_inspect_inputs,
            commands::transcribe::transcribe_start,
            commands::transcribe::transcribe_open_output,
            commands::transcribe::transcribe_show_window,
            commands::update::update_check,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { code, api, .. } = event {
                if let Some(ctrl) =
                    app_handle.try_state::<Arc<controllers::scribe::ScribeController>>()
                {
                    ctrl.finalize_capture_on_shutdown();
                }
                if let Some(ctrl) =
                    app_handle.try_state::<Arc<controllers::dictate::DictateController>>()
                {
                    ctrl.finalize_capture_on_shutdown();
                }
                // Tray-backed app: default event loop exits when the last window is torn down.
                // Programmatic `app.exit(n)` uses `Some(n)` and must not be prevented.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
