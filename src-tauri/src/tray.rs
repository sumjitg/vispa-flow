// tray.rs — System tray icon with three visual states
//
// States per agent.md:
//   Idle        — grey icon
//   Recording   — red icon
//   Transcribing — amber icon
//
// Right-click menu: Settings | Status: <state> (non-clickable) | Quit
// Left-click: show small TrayPopup React window.


use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};
use log::info;

// Pre-embed tinted tray icons at compile time
const ICON_IDLE:         &[u8] = include_bytes!("../icons/tray-idle.png");
const ICON_RECORDING:    &[u8] = include_bytes!("../icons/tray-recording.png");
const ICON_TRANSCRIBING: &[u8] = include_bytes!("../icons/tray-transcribing.png");

#[derive(Debug, Clone, PartialEq)]
pub enum TrayState {
    Idle,
    Recording,
    Transcribing,
}

impl TrayState {
    pub fn label(&self) -> &'static str {
        match self {
            TrayState::Idle         => "idle",
            TrayState::Recording    => "recording",
            TrayState::Transcribing => "transcribing",
        }
    }
}


/// Build the tray icon and attach event handlers.
pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(app, "status", "Status: idle", false, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&settings_item, &status_item, &sep, &quit_item])?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Vispa Flow — Hold Ctrl+Alt+Space to dictate")
        .on_menu_event({
            let app_handle = app.clone();
            move |_tray, event| {
                match event.id().as_ref() {
                    "settings" => {
                        show_settings_window(&app_handle);
                    }
                    "quit" => {
                        info!("Quit from tray menu.");
                        app_handle.exit(0);
                    }
                    _ => {}
                }
            }
        })
        .on_tray_icon_event({
            let app_handle = app.clone();
            move |_tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_tray_popup(&app_handle);
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Update the tray icon and tooltip to reflect the new state.
pub fn set_state(app: &AppHandle, state: TrayState) {
    info!("Tray state -> {:?}", state);

    if let Some(tray) = app.tray_by_id("main") {
        let (icon_bytes, tooltip) = match state {
            TrayState::Idle => (
                ICON_IDLE,
                "Vispa Flow - Hold Ctrl+Alt+Space to dictate",
            ),
            TrayState::Recording => (
                ICON_RECORDING,
                "Vispa Flow - Recording...",
            ),
            TrayState::Transcribing => (
                ICON_TRANSCRIBING,
                "Vispa Flow - Transcribing...",
            ),
        };

        tray.set_tooltip(Some(tooltip)).ok();

        if let Ok(icon) = Image::from_bytes(icon_bytes) {
            tray.set_icon(Some(icon)).ok();
        }
    }

    // Emit state to frontend for TrayPopup
    if let Some(main_win) = app.get_webview_window("main") {
        main_win.emit("tray-state", state.label()).ok();
    }
    // Also emit to popup window if open
    if let Some(popup_win) = app.get_webview_window("popup") {
        popup_win.emit("tray-state", state.label()).ok();
    }
}

fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("settings") {
        win.show().ok();
        win.set_focus().ok();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            app,
            "settings",
            tauri::WebviewUrl::App("/#/settings".into()),
        )
        .title("Vispa Flow — Settings")
        .inner_size(520.0, 620.0)
        .resizable(false)
        .build();
    }
}

pub fn show_settings_window_pub<R: Runtime>(app: &AppHandle<R>) {
    show_settings_window(app);
}

fn show_tray_popup<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("popup") {
        if win.is_visible().unwrap_or(false) {
            win.hide().ok();
        } else {
            win.show().ok();
            win.set_focus().ok();
        }
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            app,
            "popup",
            tauri::WebviewUrl::App("/#/popup".into()),
        )
        .title("Vispa Flow")
        .inner_size(280.0, 180.0)
        .decorations(false)
        .always_on_top(true)
        .resizable(false)
        .build();
    }
}
