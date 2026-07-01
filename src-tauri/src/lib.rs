// lib.rs — Tauri application entry point for Vispa Flow
//
// Wires together: db, ipc, hotkey, tray, injector modules.
// Exposes Tauri commands: get_setting, set_setting, re_register_hotkey.

mod db;
mod hotkey;
mod injector;
mod ipc;
mod tray;

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use log::{info, warn};
use tauri::{AppHandle, Manager};

/// App-level state accessible from Tauri commands.
pub struct AppState {
    pub python_sender: ipc::PythonSender,
}

const AUTOSTART_REG_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const AUTOSTART_VALUE_NAME: &str = "Vispa Flow";

/// Enable or disable autostart by writing directly to the Windows registry.
/// Falls back to `reg.exe` if the windows crate API fails.
fn set_autostart_registry(enable: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let exe_str = exe.to_string_lossy();

    if enable {
        // reg add HKCU\... /v "Vispa Flow" /d "\"C:\path\"" /f
        let value = format!("\"{}\"", exe_str);
        let output = std::process::Command::new("reg")
            .args(["add", AUTOSTART_REG_KEY, "/v", AUTOSTART_VALUE_NAME, "/d", &value, "/f"])
            .output()
            .map_err(|e| format!("reg add failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("reg add error: {}", stderr));
        }
        info!("Autostart REGISTRY: enabled -> {}", exe_str);
    } else {
        // reg delete HKCU\... /v "Vispa Flow" /f
        let output = std::process::Command::new("reg")
            .args(["delete", AUTOSTART_REG_KEY, "/v", AUTOSTART_VALUE_NAME, "/f"])
            .output()
            .map_err(|e| format!("reg delete failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "not found" errors on disable (already disabled)
            if !stderr.contains("not found") && !stderr.contains("does not exist") {
                return Err(format!("reg delete error: {}", stderr));
            }
        }
        info!("Autostart REGISTRY: disabled");
    }
    Ok(())
}

/// Sync autostart with the DB setting: verify the registry matches the DB.
/// Called at startup during setup.
fn sync_autostart() {
    let should_autostart = db::get("autostart") == "true";
    info!("Autostart sync: DB says should_autostart={}", should_autostart);

    match set_autostart_registry(should_autostart) {
        Ok(()) => info!("Autostart sync complete."),
        Err(e) => warn!("Autostart sync failed: {}", e),
    }
}

/// Install a panic hook that writes to crash.log on fatal panic.
/// Filters out known-benign tao shutdown panics.
fn install_crash_log() {
    let crash_log_path = crate::db::crash_log_path();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.to_string();
        // Suppress the well-known tao shutdown panic (benign, not actionable)
        if payload.contains("cannot move state from Destroyed") {
            return;
        }
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let msg = format!(
            "{} | PANIC: {}\n",
            timestamp,
            payload,
        );
        eprint!("{}", msg);
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&crash_log_path) {
            let _ = f.write_all(msg.as_bytes());
            let _ = f.write_all(b"\n");
        }
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_crash_log();

    tauri::Builder::default()
        // --- Plugins ---
        // Single-instance MUST be first to prevent duplicate processes
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").map(|win| win.set_focus());
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(Vec::new()),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .max_file_size(5_000_000)  // 5 MB
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .targets([
                    tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::LogDir {
                            file_name: Some("vispa".to_string()),
                        },
                    ),
                    tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Stdout,
                    ),
                ])
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        // --- Commands ---
        .invoke_handler(tauri::generate_handler![
            db::get_setting,
            db::set_setting,
            cmd_re_register_hotkey,
            cmd_apply_setting,
            cmd_toggle_autostart,
        ])
        // --- Setup ---
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 1. Ensure DB exists with defaults
            if let Err(e) = db::open() {
                log::error!("Failed to open database: {}", e);
            }

            // 2. Sync autostart via direct registry write (most reliable method)
            sync_autostart();

            // 3. Start Python sidecar + TCP connection
            let sender = ipc::start_python_sidecar(app_handle.clone());

            // 4. Register app state
            app.manage(Arc::new(Mutex::new(AppState {
                python_sender: sender.clone(),
            })));

            // 5. Set up tray icon
            tray::setup_tray(&app_handle)?;

            // 6. Register global hotkey (slight delay to allow sidecar to connect)
            let handle_for_hotkey = app_handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                hotkey::register_hotkey(&handle_for_hotkey, sender);
            });

            info!("Vispa Flow started.");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ---------------------------------------------------------------------------
// Additional Tauri commands
// ---------------------------------------------------------------------------

/// Re-register the global hotkey (called after hotkey setting is saved).
#[tauri::command]
async fn cmd_re_register_hotkey(app: AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<Mutex<AppState>>>();
    let sender = state.lock().unwrap().python_sender.clone();
    hotkey::register_hotkey(&app, sender);
    Ok(())
}

/// Apply a setting change immediately to the Python sidecar (no restart needed).
/// Also handles the pseudo-command __open_settings__ from TrayPopup.
#[tauri::command]
async fn cmd_apply_setting(
    app: AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    // Pseudo-command: open settings window from TrayPopup
    if key == "__open_settings__" {
        tray::show_settings_window_pub(&app);
        return Ok(());
    }

    // Persist to DB
    db::set_setting(key.clone(), value.clone())?;

    // Forward to Python
    let state = app.state::<Arc<Mutex<AppState>>>();
    let sender = state.lock().unwrap().python_sender.clone();
    ipc::send_config(&sender, &key, &value);

    // If hotkey changed, re-register
    if key == "hotkey" {
        hotkey::register_hotkey(&app, sender);
    }

    Ok(())
}

/// Toggle autostart on/off via direct registry write.
/// Persists the preference to DB and updates the Windows registry immediately.
#[tauri::command]
async fn cmd_toggle_autostart(enabled: bool) -> Result<(), String> {
    // Persist to DB
    db::set_setting("autostart".to_string(), if enabled { "true" } else { "false" }.to_string())?;

    // Write directly to registry
    set_autostart_registry(enabled)?;

    info!("Autostart toggled: enabled={}", enabled);
    Ok(())
}
