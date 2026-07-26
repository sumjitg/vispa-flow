// hotkey.rs — Global hotkey registration for VoiceType
//
// Per agent.md:
//   - Register global hotkey (default: Ctrl+Alt+Space).
//   - On press: send {"cmd": "start_recording"} to Python sidecar.
//   - On release: send {"cmd": "stop_recording"} to Python sidecar.
//   - Hotkey combo read from SQLite at startup. Re-register on settings save.

use log::{error, info, warn};
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

use crate::ipc::PythonSender;

/// Parse a hotkey string like "ctrl+alt+space" into a Tauri Shortcut.
fn parse_hotkey(hotkey_str: &str) -> Option<Shortcut> {
    let mut modifiers = Modifiers::empty();
    let mut code: Option<Code> = None;

    for part in hotkey_str.to_lowercase().split('+') {
        let part = part.trim();
        match part {
            "ctrl"  | "control" => modifiers |= Modifiers::CONTROL,
            "alt"               => modifiers |= Modifiers::ALT,
            "shift"             => modifiers |= Modifiers::SHIFT,
            "meta"  | "super" | "win" => modifiers |= Modifiers::META,
            key => {
                code = parse_key_code(key);
            }
        }
    }

    let code = code?;
    Some(Shortcut::new(
        if modifiers.is_empty() { None } else { Some(modifiers) },
        code,
    ))
}

fn parse_key_code(key: &str) -> Option<Code> {
    match key {
        "space"  => Some(Code::Space),
        "enter"  => Some(Code::Enter),
        "tab"    => Some(Code::Tab),
        "escape" | "esc" => Some(Code::Escape),
        "a" => Some(Code::KeyA), "b" => Some(Code::KeyB),
        "c" => Some(Code::KeyC), "d" => Some(Code::KeyD),
        "e" => Some(Code::KeyE), "f" => Some(Code::KeyF),
        "g" => Some(Code::KeyG), "h" => Some(Code::KeyH),
        "i" => Some(Code::KeyI), "j" => Some(Code::KeyJ),
        "k" => Some(Code::KeyK), "l" => Some(Code::KeyL),
        "m" => Some(Code::KeyM), "n" => Some(Code::KeyN),
        "o" => Some(Code::KeyO), "p" => Some(Code::KeyP),
        "q" => Some(Code::KeyQ), "r" => Some(Code::KeyR),
        "s" => Some(Code::KeyS), "t" => Some(Code::KeyT),
        "u" => Some(Code::KeyU), "v" => Some(Code::KeyV),
        "w" => Some(Code::KeyW), "x" => Some(Code::KeyX),
        "y" => Some(Code::KeyY), "z" => Some(Code::KeyZ),
        "0" => Some(Code::Digit0), "1" => Some(Code::Digit1),
        "2" => Some(Code::Digit2), "3" => Some(Code::Digit3),
        "4" => Some(Code::Digit4), "5" => Some(Code::Digit5),
        "6" => Some(Code::Digit6), "7" => Some(Code::Digit7),
        "8" => Some(Code::Digit8), "9" => Some(Code::Digit9),
        "f1"  => Some(Code::F1),  "f2"  => Some(Code::F2),
        "f3"  => Some(Code::F3),  "f4"  => Some(Code::F4),
        "f5"  => Some(Code::F5),  "f6"  => Some(Code::F6),
        "f7"  => Some(Code::F7),  "f8"  => Some(Code::F8),
        "f9"  => Some(Code::F9),  "f10" => Some(Code::F10),
        "f11" => Some(Code::F11), "f12" => Some(Code::F12),
        _ => {
            warn!("Unknown key in hotkey string: '{}'", key);
            None
        }
    }
}

/// Register the global hotkey from the database setting.
/// Called once at startup and again after settings save.
pub fn register_hotkey<R: Runtime>(app: &AppHandle<R>, sender: PythonSender) {
    let hotkey_str = crate::db::get("hotkey");
    let hotkey_str = if hotkey_str.is_empty() {
        "ctrl+alt+space".to_string()
    } else {
        hotkey_str
    };

    let shortcut = match parse_hotkey(&hotkey_str) {
        Some(s) => s,
        None => {
            error!("Could not parse hotkey '{}'. Using default.", hotkey_str);
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space)
        }
    };

    info!("Registering hotkey: '{}'", hotkey_str);

    // Unregister any existing shortcut first
    let _ = app.global_shortcut().unregister_all();

    let sender_clone = sender.clone();
    if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |_app, _sc, event| {
        match event.state() {
            ShortcutState::Pressed  => crate::ipc::send_cmd(&sender_clone, "start_recording"),
            ShortcutState::Released => {
                crate::ipc::send_cmd_with_timing(&sender_clone, "stop_recording");
            }
        }
    }) {
        error!("Failed to register hotkey '{}': {}", hotkey_str, e);
    } else {
        info!("Hotkey '{}' registered successfully.", hotkey_str);
    }
}
