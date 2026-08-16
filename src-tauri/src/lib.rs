mod commands;
mod controllers;
mod platform;
pub mod services;
pub mod types;

#[cfg(test)]
mod acl_capabilities_test;

use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
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
        version,
        os,
        arch,
        cpu = cpu_brand,
        cores_physical = physical,
        cores_logical = logical,
        ram = ram_gb,
        "scribefloat startup"
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
    let rc = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
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

pub(crate) const DICTATE_WINDOW_LABEL: &str = "dictate";
const HISTORY_WINDOW_LABEL: &str = "history";
const ONBOARDING_WINDOW_LABEL: &str = "onboarding";

const DICTATE_MENU_ID: &str = "dictate";
const NEW_NOTE_MENU_ID: &str = "new_note";
const OPEN_APP_MENU_ID: &str = "open_app";
const OPEN_SETTINGS_MENU_ID: &str = "open_settings";
const QUIT_MENU_ID: &str = "quit";

const SETTINGS_MENU_ACCELERATOR: &str = "CmdOrCtrl+,";
const QUIT_MENU_ACCELERATOR: &str = "CmdOrCtrl+Q";

struct TrayMenuState {
    new_note_item: MenuItem<tauri::Wry>,
}

const HISTORY_WINDOW_W: f64 = 980.0;
const HISTORY_WINDOW_H: f64 = 680.0;
const DICTATE_WINDOW_W: f64 = 240.0;
const DICTATE_WINDOW_H: f64 = 48.0;
const ONBOARDING_WINDOW_W: f64 = 680.0;
const ONBOARDING_WINDOW_H: f64 = 560.0;
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

fn build_tray_menu(
    app: &impl Manager<tauri::Wry>,
    open_hotkey: &str,
) -> tauri::Result<(MenuItem<tauri::Wry>, Menu<tauri::Wry>)> {
    let dictate_item = MenuItem::with_id(app, DICTATE_MENU_ID, "Dictate", true, None::<&str>)?;
    let new_note_item =
        MenuItem::with_id(app, NEW_NOTE_MENU_ID, "New note", true, Some(open_hotkey))?;
    let open_app_item = MenuItem::with_id(
        app,
        OPEN_APP_MENU_ID,
        "Open ScribeFloat",
        true,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(
        app,
        OPEN_SETTINGS_MENU_ID,
        "Settings",
        true,
        Some(SETTINGS_MENU_ACCELERATOR),
    )?;
    let quit_item = MenuItem::with_id(
        app,
        QUIT_MENU_ID,
        "Quit ScribeFloat",
        true,
        Some(QUIT_MENU_ACCELERATOR),
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &dictate_item,
            &new_note_item,
            &PredefinedMenuItem::separator(app)?,
            &open_app_item,
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    Ok((new_note_item, menu))
}

pub(crate) fn refresh_tray_accelerators(app: &AppHandle, open_hotkey: &str) {
    let Some(state) = app.try_state::<TrayMenuState>() else {
        return;
    };
    let _ = state.new_note_item.set_text("New note");
    let _ = state.new_note_item.set_accelerator(Some(open_hotkey));
}

fn create_tray(app: &mut tauri::App, open_hotkey: &str) -> tauri::Result<()> {
    let (new_note_item, menu) = build_tray_menu(app, open_hotkey)?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            DICTATE_MENU_ID => {
                if let Some(ctrl) = app.try_state::<Arc<controllers::dictate::DictateController>>()
                {
                    ctrl.trigger_toggle();
                }
            }
            NEW_NOTE_MENU_ID => {
                if let Err(err) = open_new_note(app) {
                    tracing::warn!(error = %err, "failed to open new note");
                }
            }
            OPEN_APP_MENU_ID => {
                if let Err(err) = navigate_history_path(app, "") {
                    tracing::warn!(error = %err, "failed to open scribefloat window");
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
        if let Some(icon) = load_icon(app.handle(), preferred_tray_icon)
            .or_else(|| app.default_window_icon().cloned())
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
    app.manage(TrayMenuState { new_note_item });
    Ok(())
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
        tracing::warn!(error = %err, "failed to prewarm dictate window");
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ShellNavigatePayload {
    route: String,
    settings_tab: Option<String>,
}

fn navigate_history_path(app: &AppHandle, path: &str) -> tauri::Result<WebviewWindow> {
    let path = path.trim_start_matches('/');
    if let Some(window) = app.get_webview_window(HISTORY_WINDOW_LABEL) {
        raise_webview_window(app, &window)?;
        let target = if path.is_empty() {
            "/?view=history".to_string()
        } else {
            format!("/{path}")
        };
        window.eval(format!("window.location.assign('{target}');"))?;
        return Ok(window);
    }

    let url = if path.is_empty() {
        WebviewUrl::App("?view=history".into())
    } else {
        WebviewUrl::App(path.into())
    };
    open_or_focus_window(
        app,
        HISTORY_WINDOW_LABEL,
        "ScribeFloat",
        url,
        HISTORY_WINDOW_W,
        HISTORY_WINDOW_H,
    )
}

fn shell_route_to_path(route: &str) -> &str {
    match route {
        "home" => "",
        "notes" => "notes",
        "upload" => "upload",
        "float" => "float",
        "settings" => "settings",
        "notes-new" | "notes/new" => "notes/new",
        // Legacy tray/IPC route — open the shell home.
        "scribe" => "",
        _ => "",
    }
}

fn navigate_shell(
    app: &AppHandle,
    route: &str,
    settings_tab: Option<&str>,
) -> tauri::Result<WebviewWindow> {
    let window = navigate_history_path(app, shell_route_to_path(route))?;
    if route == "settings" && settings_tab.is_some() {
        let payload = ShellNavigatePayload {
            route: route.to_string(),
            settings_tab: settings_tab.map(|s| s.to_string()),
        };
        let _ = window.emit("app://navigate", payload);
    }
    Ok(window)
}

pub(crate) fn open_scribe_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    navigate_shell(app, "scribe", None)
}

pub(crate) fn open_new_note(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    navigate_history_path(app, "notes/new")
}

pub(crate) fn open_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    navigate_history_path(app, "settings")
}

pub(crate) fn open_transcribe_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    navigate_shell(app, "upload", None)
}

pub(crate) fn open_onboarding_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let window = if let Some(w) = app.get_webview_window(ONBOARDING_WINDOW_LABEL) {
        raise_webview_window(app, &w)?;
        w
    } else {
        let mut builder = WebviewWindowBuilder::new(
            app,
            ONBOARDING_WINDOW_LABEL,
            WebviewUrl::App("?view=onboarding".into()),
        )
        .title("ScribeFloat Setup")
        .inner_size(ONBOARDING_WINDOW_W, ONBOARDING_WINDOW_H)
        .resizable(false)
        .center();
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }
        builder.build()?
    };
    platform::window_impl::set_has_visible_windows(app, true);
    Ok(window)
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
                    let open_hotkey = app
                        .try_state::<Arc<services::config::ConfigService>>()
                        .map(|config| config.get().open_scribe_hotkey.clone())
                        .unwrap_or_else(|| platform::default_open_scribe_hotkey().to_string());
                    if let Ok(scribe_sc) =
                        open_hotkey.parse::<tauri_plugin_global_shortcut::Shortcut>()
                    {
                        if shortcut.id() == scribe_sc.id() {
                            let handle = app.clone();
                            let _ = app.run_on_main_thread(move || {
                                open_new_note(&handle).ok();
                            });
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let data_dir = app.path().app_data_dir()?;
            #[cfg(target_os = "macos")]
            if let Some(helper) = platform::resolve_set_default_output_helper() {
                platform::init_set_default_output_helper(helper);
            } else {
                tracing::warn!(
                    "set-default-output helper missing; speaker capture output restore may fail"
                );
            }
            #[cfg(target_os = "macos")]
            if let Some(cli) = platform::resolve_cli_binary() {
                platform::cli_link::ensure_cli_symlink(&cli);
            }
            let config = services::config::ConfigService::load(data_dir.join("config.json"))?;
            app.manage(Arc::clone(&config));
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
            let resource_dir = app.path().resource_dir().ok();
            let diarization = services::diarization::DiarizationService::with_resource_dir(
                models_dir.join(services::diarization::SORTFORMER_MODEL_FILENAME),
                resource_dir.clone(),
            );
            app.manage(Arc::clone(&diarization));
            let model = services::model::ModelService::with_resource_dir(
                models_dir.clone(),
                resource_dir.clone(),
            );
            let model_ctrl = controllers::model::ModelController::new(Arc::clone(&model));

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

            let (open_hotkey, _) = settings_ctrl.get_hotkeys();
            create_tray(app, &open_hotkey)?;

            // Seed / offline-heal bundled models into the writable models dir, off the
            // startup critical path — the tray above no longer waits on this. Missing,
            // empty, or hash-mismatch copies are replaced from the installed app
            // resources when those files have real content (dev 0-byte placeholders are
            // skipped). Actual model use is always lazy (first Scribe/Dictate/Transcribe
            // session), so this has ample time to finish before anything needs it.
            //
            // Large models (Whisper ~181 MB, Sortformer ~469 MB) only get a
            // missing/empty check here — full SHA-256 runs on first use
            // (`ensure_model` / whisper load). Debug builds use soft SHA-256;
            // hashing Sortformer on the main thread was ~30–40s of 100% CPU
            // before the tray appeared.
            let model_seed_bg = Arc::clone(&model);
            tauri::async_runtime::spawn(async move {
                // Runs on a blocking-pool thread, not a core async-runtime worker: the
                // loop below does synchronous file I/O (existence checks, hashing, and
                // potentially copying up to ~650 MB across two models). Awaiting that
                // directly inside an async task would occupy a scheduler worker thread
                // for the whole blocking duration and could starve other IPC command
                // handlers sharing the same runtime — exactly the regression this once
                // caused (window IPC calls queued behind this task instead of running
                // concurrently with it).
                let _ = tokio::task::spawn_blocking(move || {
                    // Utility QoS so this thread yields CPU to window
                    // paint/JS init instead of racing them — see doc comment
                    // on `lower_thread_priority_for_background_work`.
                    platform::lower_thread_priority_for_background_work();
                    let seed_targets: &[(&str, &str, bool)] = &[
                        (
                            services::model::SMALL_MODEL_FILENAME,
                            services::model::SMALL_MODEL_SHA256,
                            false, // hash at use-time
                        ),
                        (
                            services::model::VAD_MODEL_FILENAME,
                            services::model::VAD_MODEL_SHA256,
                            true, // tiny — fine to pin every launch
                        ),
                        (
                            services::diarization::SORTFORMER_MODEL_FILENAME,
                            services::diarization::SORTFORMER_MODEL_SHA256,
                            false, // hash at use-time (same reason as Whisper)
                        ),
                    ];
                    for &(file_name, expected_sha, hash_at_startup) in seed_targets {
                        let dest = models_dir.join(file_name);
                        let needs = if hash_at_startup {
                            services::bundled_models::dest_needs_bundle_restore_cached(
                                &dest,
                                expected_sha,
                            )
                        } else {
                            !dest.exists()
                                || std::fs::metadata(&dest)
                                    .map(|m| !m.is_file() || m.len() == 0)
                                    .unwrap_or(true)
                        };
                        if needs {
                            if let Some(ref resource_dir) = resource_dir {
                                let _ = services::bundled_models::ensure_bundled_file(
                                    Some(resource_dir.as_path()),
                                    &dest,
                                    file_name,
                                    expected_sha,
                                );
                            }
                        }
                    }
                    if !model_seed_bg.bundled_model_available() {
                        tracing::warn!(
                            "bundled Whisper model missing at {}",
                            model_seed_bg.default_model_path().display()
                        );
                    }
                    if !model_seed_bg.bundled_vad_available() {
                        tracing::warn!(
                            "bundled VAD model missing or corrupt at {}",
                            model_seed_bg.vad_model_path().display()
                        );
                    }
                    // Warm the Whisper context now, once the model file is confirmed
                    // present/healed above, instead of waiting for the user's first
                    // Dictate/Scribe/Transcribe action. The context is cached for the
                    // app's lifetime (`ModelService::loaded_contexts`) and Small is
                    // ~181 MB — cheap enough on any machine this app targets to keep
                    // resident, and it removes the cold-load wait from every first
                    // capture of a session, not just from the ones after the first.
                    let default_path = model_seed_bg.default_model_path();
                    model_seed_bg.preload_context(&default_path);
                })
                .await;
            });

            let ctrl = controllers::scribe::ScribeController::new(
                Arc::clone(&audio),
                Arc::clone(&model),
                Arc::clone(&output),
                Arc::clone(&history),
                Arc::clone(&config),
                Arc::clone(&diarization),
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
                Arc::clone(&diarization),
                app.handle().clone(),
            );
            let speaker_names = services::speaker_names::SpeakerNameService::load(
                data_dir.join("speaker_names.json"),
            );
            // Voiceprint never shipped (exploration-only); this is local hygiene so a
            // machine that ran an earlier build doesn't keep an orphaned encryption key.
            if let Err(e) = platform::delete_voice_crypto_key() {
                tracing::warn!(error = %e, "could not delete legacy voice encryption key");
            }
            let history_ctrl = controllers::history::HistoryController::new(
                Arc::clone(&history),
                Arc::clone(&output),
                Arc::clone(&config),
                Arc::clone(&speaker_names),
            );
            app.manage(Arc::clone(&speaker_names));
            app.manage(controllers::speaker_names::SpeakerNamesController::new(
                Arc::clone(&speaker_names),
            ));

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

            dictate_ctrl.ensure_key_listener();

            if is_first_run {
                open_onboarding_window(app.handle())?;
            }
            prewarm_dictate_window(app.handle());
            // Tao applies Regular activation at launch; `set_dock_visibility(false)` only runs when we
            // call it. Sync once after prewarm so a tray-only start hides the Dock (plist LSUIElement
            // is not sufficient on its own).
            platform::window_impl::sync_activation_policy(app.handle());

            // On first run, the save folder hasn't been confirmed via onboarding yet, so don't
            // touch it here — that's what fires the Documents-folder TCC prompt before the
            // Permissions step is even shown. There's nothing to compact/recover on a fresh
            // install anyway; the scan runs on every later, non-first-run launch instead.
            if !is_first_run {
                let save_folder = app
                    .state::<Arc<services::config::ConfigService>>()
                    .get()
                    .save_folder;
                // Run compaction and recovery scans in the background so they never block
                // the Tauri event loop at startup (large histories can take 100-500ms).
                let history_bg = Arc::clone(&history);
                let output_bg = Arc::clone(&output);
                let save_folder_bg = save_folder.clone();
                let temp_dir_bg = app
                    .path()
                    .app_local_data_dir()
                    .ok()
                    .map(|d| d.join("dictate_temp"));
                tauri::async_runtime::spawn(async move {
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
                        match output_bg.scan_and_salvage_dictate_temp_wavs(&temp_dir, &save_folder_bg)
                        {
                            Ok(salvaged) => {
                                for path in salvaged {
                                    tracing::info!(path = %path.display(), "salvaged dictate wav");
                                }
                            }
                            Err(e) => tracing::warn!(error = %e, "dictate temp scan failed"),
                        }
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::Destroyed) {
                // Sync dock visibility after onboarding is fully destroyed (not on CloseRequested,
                // where is_visible() still returns true for the closing window).
                if window.label() == ONBOARDING_WINDOW_LABEL {
                    platform::window_impl::sync_activation_policy(window.app_handle());
                }
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                // The onboarding window is a one-time wizard: let it destroy normally.
                // The frontend calls settings_complete_onboarding before closing.
                if window.label() == ONBOARDING_WINDOW_LABEL {
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
            commands::scribe::scribe_set_attach_note,
            commands::scribe::scribe_add_note,
            commands::scribe::scribe_get_include_timestamps,
            commands::scribe::scribe_set_include_timestamps,
            commands::scribe::scribe_list_input_devices,
            commands::scribe::scribe_list_output_devices,
            commands::scribe::scribe_read_transcript,
            commands::scribe::scribe_list_recovery_sessions,
            commands::scribe::scribe_list_transcripts,
            commands::scribe::scribe_switch_mic,
            commands::scribe::scribe_toggle_speaker_capture,
            commands::model::model_vad_status,
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
            commands::settings::settings_get_onboarding_step,
            commands::settings::settings_set_onboarding_step,
            commands::settings::settings_complete_onboarding,
            commands::settings::settings_reset_onboarding,
            commands::settings::settings_show_window,
            commands::settings::settings_show_onboarding_window,
            commands::settings::settings_get_platform,
            commands::settings::settings_open_scribe_window,
            commands::settings::settings_get_dictate_auto_paste,
            commands::settings::settings_set_dictate_auto_paste,
            commands::settings::settings_get_dictate_auto_enter,
            commands::settings::settings_set_dictate_auto_enter,
            commands::settings::settings_get_keep_wav,
            commands::settings::settings_set_keep_wav,
            commands::settings::settings_get_save_transcripts_as_markdown,
            commands::settings::settings_set_save_transcripts_as_markdown,
            commands::settings::settings_get_user_display_name,
            commands::settings::settings_set_user_display_name,
            commands::dictate::dictate_cancel,
            commands::dictate::dictate_dismiss,
            commands::dictate::dictate_get_history,
            commands::dictate::dictate_trigger,
            commands::dictate::dictate_get_state,
            commands::history::history_list,
            commands::history::history_get_detail,
            commands::history::history_render_markdown,
            commands::history::history_export_markdown,
            commands::history::history_delete,
            commands::history::history_read_legacy,
            commands::history::get_dashboard_stats,
            commands::history::history_tag_vocabulary,
            commands::history::note_create_empty,
            commands::history::note_save_written_content,
            commands::history::note_save_title,
            commands::history::note_is_empty,
            commands::history::note_has_metadata,
            commands::history::note_set_tags,
            commands::history::note_relabel_speaker,
            commands::speaker_names::speaker_names_list,
            commands::speaker_names::speaker_name_save,
            commands::speaker_names::speaker_name_delete,
            commands::history::note_attach_transcript,
            commands::history::note_render_transcript_html,
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
                // Drop WhisperContext instances before audio teardown so Metal GPU
                // resources are freed while the Rust runtime is still fully live.
                // Without this, ggml-metal asserts during NSApplication terminate.
                if let Some(svc) = app_handle.try_state::<Arc<services::model::ModelService>>() {
                    svc.release_contexts();
                }
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
