// update.rs — auto-update via tauri-plugin-updater with a prompt-then-install flow.
//
// On startup (after a short delay) and on the "Check for updates" tray item,
// we query the GitHub Releases manifest. If a newer version is available we
// show a native yes/no dialog; on confirm we download, install, and relaunch.

use log::{error, info, warn};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

/// Check for an update. When `silent` is true (startup check), stay quiet if
/// the app is already up to date or the check fails; only speak up when there
/// is actually a newer version. When false (manual check from the tray), also
/// report "you're up to date" and errors so the user gets feedback.
pub fn check_for_updates(app: &AppHandle, silent: bool) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let updater = match app.updater() {
            Ok(u) => u,
            Err(e) => {
                warn!("Updater unavailable: {}", e);
                if !silent {
                    notify_error(&app, &format!("Update check failed: {}", e));
                }
                return;
            }
        };

        match updater.check().await {
            Ok(Some(update)) => {
                let version = update.version.clone();
                info!("Update available: {} -> {}", update.current_version, version);
                prompt_and_install(&app, update).await;
            }
            Ok(None) => {
                info!("No update available (already on latest).");
                if !silent {
                    app.dialog()
                        .message("You're on the latest version of VoiceType.")
                        .title("No updates")
                        .kind(MessageDialogKind::Info)
                        .blocking_show();
                }
            }
            Err(e) => {
                error!("Update check failed: {}", e);
                if !silent {
                    notify_error(&app, &format!("Could not check for updates: {}", e));
                }
            }
        }
    });
}

/// Show the confirmation dialog, then download+install+relaunch if accepted.
async fn prompt_and_install(app: &AppHandle, update: tauri_plugin_updater::Update) {
    let version = update.version.clone();
    let body = update.body.clone().unwrap_or_default();

    let mut message = format!("VoiceType {} is available.", version);
    if !body.trim().is_empty() {
        message.push_str("\n\n");
        message.push_str(body.trim());
    }
    message.push_str("\n\nInstall now and restart?");

    let confirmed = app
        .dialog()
        .message(message)
        .title("Update available")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Install".to_string(),
            "Later".to_string(),
        ))
        .blocking_show();

    if !confirmed {
        info!("User deferred update to {}.", version);
        return;
    }

    info!("Downloading and installing update {}...", version);
    match update.download_and_install(|_chunk, _total| {}, || {}).await {
        Ok(()) => {
            info!("Update {} installed. Relaunching.", version);
            app.restart();
        }
        Err(e) => {
            error!("Update install failed: {}", e);
            notify_error(app, &format!("Update failed to install: {}", e));
        }
    }
}

fn notify_error(app: &AppHandle, msg: &str) {
    app.dialog()
        .message(msg)
        .title("VoiceType update")
        .kind(MessageDialogKind::Error)
        .blocking_show();
}
