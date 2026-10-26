// In-app updates from the GitHub releases (latest.json, signed with the key in tauri.conf.json).
// The update is downloaded in the background and only installed once the user accepts the restart,
// so nobody loses what they were typing. "Later" asks again on the next check.
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

const FIRST_CHECK: Duration = Duration::from_secs(20);
// The app usually lives in the tray for days, so it keeps checking.
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            if let Err(err) = tauri::async_runtime::block_on(check_and_offer(&app)) {
                eprintln!("updater: {err}");
            }
            std::thread::sleep(CHECK_INTERVAL);
        }
    });
}

async fn check_and_offer(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    let bytes = update.download(|_, _| {}, || {}).await?;

    let restart = app
        .dialog()
        .message(format!(
            "La versión {} de TeurTask está lista. ¿Reiniciar ahora para actualizar?",
            update.version
        ))
        .title("Actualización disponible")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Reiniciar ahora".into(),
            "Más tarde".into(),
        ))
        .blocking_show();
    if !restart {
        return Ok(());
    }

    // On Windows this hands over to the installer, which relaunches the app itself.
    update.install(bytes)?;
    app.restart();
}
