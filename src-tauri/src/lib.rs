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
use log::info;
use tauri::{AppHandle, Manager};

/// App-level state accessible from Tauri commands.
pub struct AppState {
    pub python_sender: ipc::PythonSender,
}

/// Install a panic hook that writes to crash.log on fatal panic.
fn install_crash_log() {
    let crash_log_path = crate::db::crash_log_path();
    std::panic::set_hook(Box::new(move |info| {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let msg = format!(
            "{} | PANIC: {}\n",
            timestamp,
            info
        );
        // Also print to stderr for debugging
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
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .max_file_size(5_000_000)  // 5 MB
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
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
        ])
        // --- Setup ---
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 1. Ensure DB exists with defaults
            if let Err(e) = db::open() {
                log::error!("Failed to open database: {}", e);
            }

            // 2. Start Python sidecar + TCP connection
            let sender = ipc::start_python_sidecar(app_handle.clone());

            // 3. Register app state
            app.manage(Arc::new(Mutex::new(AppState {
                python_sender: sender.clone(),
            })));

            // 4. Set up tray icon
            tray::setup_tray(&app_handle)?;

            // 5. Register global hotkey (slight delay to allow sidecar to connect)
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
