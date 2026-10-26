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
    #[cfg(target_os = "linux")]
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        relaunch_appimage(&appimage);
        app.exit(0);
        return Ok(());
    }
    app.restart();
}

// The AppImage's AppRun points LD_LIBRARY_PATH, PATH, GTK_*, GST_*… at its own mount. A plain
// restart hands them to the new AppImage, whose hooks then run the host bash against the bundled
// (older) libreadline and die before the app starts ("undefined symbol: rl_trim_arg_from_keyseq"
// on Arch). So every entry under the old mount is dropped; the new AppRun sets its own.
#[cfg(target_os = "linux")]
fn relaunch_appimage(appimage: &std::ffi::OsStr) {
    let appdir = std::env::var("APPDIR").unwrap_or_default();
    let mut cmd = std::process::Command::new(appimage);
    cmd.env_clear();
    for (key, value) in std::env::vars_os() {
        match value.to_str() {
            Some(text) if !appdir.is_empty() && text.contains(&appdir) => {
                let kept: Vec<&str> = text
                    .split(':')
                    .filter(|part| !part.is_empty() && !part.starts_with(&appdir))
                    .collect();
                if !kept.is_empty() {
                    cmd.env(key, kept.join(":"));
                }
            }
            _ => {
                cmd.env(key, value);
            }
        }
    }
    if let Err(err) = cmd.spawn() {
        eprintln!("updater: relaunch failed: {err}");
    }
}
