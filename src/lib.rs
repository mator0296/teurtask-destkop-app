use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_notification::NotificationExt;

static TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");
static TRAY_BADGE: &[u8] = include_bytes!("../icons/tray-badge.png");

#[tauri::command]
fn send_native_notification(app: tauri::AppHandle, title: String, body: String) {
    let icon_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("icons/128x128.png"))
        .filter(|p| p.exists());

    let result = if let Some(icon) = icon_path {
        app.notification()
            .builder()
            .title(&title)
            .body(&body)
            .icon(icon.to_string_lossy().into_owned())
            .show()
    } else {
        app.notification().builder().title(&title).body(&body).show()
    };

    // If icon caused failure, retry without icon
    if result.is_err() {
        let _ = app.notification().builder().title(&title).body(&body).show();
    }
}

#[tauri::command]
fn set_tray_badge(app: tauri::AppHandle, has_notifications: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let bytes = if has_notifications { TRAY_BADGE } else { TRAY_ICON };
        if let Ok(icon) = tauri::image::Image::from_bytes(bytes) {
            let _ = tray.set_icon(Some(icon));
        }
    }
}

// WebKitGTK ships with getUserMedia disabled and denies every permission request nobody
// handles; wry does neither, so voice notes failed on Linux with "No se pudo acceder al
// micrófono". Mic/camera are granted only to the teurtask origin (and the dev server).
#[cfg(target_os = "linux")]
fn enable_media_capture(window: &tauri::WebviewWindow) {
    use webkit2gtk::{
        glib::prelude::ObjectExt, PermissionRequestExt, SettingsExt, UserMediaPermissionRequest,
        WebViewExt,
    };

    let _ = window.with_webview(|webview| {
        let view = webview.inner();
        if let Some(settings) = WebViewExt::settings(&view) {
            settings.set_enable_media_stream(true);
            settings.set_enable_mediasource(true);
        }
        view.connect_permission_request(|view, request| {
            if !request.is::<UserMediaPermissionRequest>() {
                return false;
            }
            let trusted = view.uri().is_some_and(|uri| {
                let host = uri
                    .split("://")
                    .nth(1)
                    .and_then(|rest| rest.split(['/', ':']).next())
                    .unwrap_or("");
                host == "teurtask.com"
                    || host.ends_with(".teurtask.com")
                    || (cfg!(debug_assertions) && host == "localhost")
            });
            if trusted {
                request.allow();
            } else {
                request.deny();
            }
            true
        });
    });
}

// Picks the WebKitGTK renderer for this session and returns whether `backdrop-filter` will be
// painted (only the GPU-composited DMA-BUF renderer does it; otherwise the glass panels are bare
// see-through). On the NVIDIA proprietary driver that renderer is not usable: on X11 (which the
// AppImage forces) it paints a blank window, on Wayland it dies with "Error 71 (Protocol error)"
// and, with explicit sync turned off, the web process aborts as soon as a backdrop-filter panel
// opens. So NVIDIA gets it turned off and the page falls back to opaque glass. Forcing software
// rendering (what the old launchers did) cost ~120 MB and every animation frame on the CPU.
#[cfg(target_os = "linux")]
fn configure_linux_rendering() -> bool {
    use std::env;

    if env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
        && std::path::Path::new("/proc/driver/nvidia").exists()
    {
        env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let set = |key| env::var(key).is_ok_and(|v| v != "0");
    !(set("WEBKIT_DISABLE_DMABUF_RENDERER")
        || set("WEBKIT_DISABLE_COMPOSITING_MODE")
        || set("LIBGL_ALWAYS_SOFTWARE"))
}

pub fn run() {
    #[cfg(target_os = "linux")]
    let backdrop_blur = configure_linux_rendering();
    #[cfg(not(target_os = "linux"))]
    let backdrop_blur = true;

    // Read by the web app (utils/desktopEnv.js) before it renders.
    let desktop_env = tauri::plugin::Builder::<tauri::Wry>::new("desktop-env")
        .js_init_script(format!(
            "window.__TEURTASK_DESKTOP__ = {{ backdropBlur: {backdrop_blur} }};"
        ))
        .build();

    tauri::Builder::default()
        .plugin(desktop_env)
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![set_tray_badge, send_native_notification])
        .setup(|app| {
            // ── Window menu ──────────────────────────────────────────────
            let reload = MenuItemBuilder::with_id("reload", "Reload")
                .accelerator("CmdOrCtrl+R")
                .build(app)?;
            let force_reload = MenuItemBuilder::with_id("force_reload", "Force Reload")
                .accelerator("CmdOrCtrl+Shift+R")
                .build(app)?;
            let devtools = MenuItemBuilder::with_id("devtools", "Toggle DevTools")
                .accelerator("F12")
                .build(app)?;
            let zoom_in = MenuItemBuilder::with_id("zoom_in", "Zoom In")
                .accelerator("CmdOrCtrl+=")
                .build(app)?;
            let zoom_out = MenuItemBuilder::with_id("zoom_out", "Zoom Out")
                .accelerator("CmdOrCtrl+-")
                .build(app)?;
            let zoom_reset = MenuItemBuilder::with_id("zoom_reset", "Reset Zoom")
                .accelerator("CmdOrCtrl+0")
                .build(app)?;
            let logout = MenuItemBuilder::with_id("logout", "Logout && Clear Cache")
                .accelerator("CmdOrCtrl+Shift+L")
                .build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit")
                .accelerator("CmdOrCtrl+Q")
                .build(app)?;

            let view_menu = SubmenuBuilder::new(app, "View")
                .item(&reload)
                .item(&force_reload)
                .separator()
                .item(&devtools)
                .separator()
                .item(&zoom_in)
                .item(&zoom_out)
                .item(&zoom_reset)
                .build()?;

            let app_menu = SubmenuBuilder::new(app, "App")
                .item(&logout)
                .separator()
                .item(&quit)
                .build()?;

            let menu = MenuBuilder::new(app)
                .items(&[&view_menu, &app_menu])
                .build()?;

            app.set_menu(menu)?;

            app.on_menu_event(|app, event| {
                let window = app.get_webview_window("main").unwrap();
                match event.id().as_ref() {
                    "reload" => {
                        let _ = window.eval("location.reload()");
                    }
                    "force_reload" => {
                        let _ = window.eval(
                            "caches.keys().then(ks=>Promise.all(ks.map(k=>caches.delete(k)))).finally(()=>location.reload())",
                        );
                    }
                    "devtools" => {
                        if window.is_devtools_open() {
                            window.close_devtools();
                        } else {
                            window.open_devtools();
                        }
                    }
                    "zoom_in" => {
                        let _ = window.eval(
                            "document.documentElement.style.zoom=String(Math.min(2,(parseFloat(document.documentElement.style.zoom||'1')+0.1).toFixed(1)))",
                        );
                    }
                    "zoom_out" => {
                        let _ = window.eval(
                            "document.documentElement.style.zoom=String(Math.max(0.5,(parseFloat(document.documentElement.style.zoom||'1')-0.1).toFixed(1)))",
                        );
                    }
                    "zoom_reset" => {
                        let _ = window.eval("document.documentElement.style.zoom='1'");
                    }
                    "logout" => {
                        let _ = window.eval(
                            "localStorage.clear();sessionStorage.clear();location.reload()",
                        );
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            });

            // ── System tray ──────────────────────────────────────────────
            let tray_show = MenuItemBuilder::with_id("tray_show", "Show Window").build(app)?;
            let tray_quit = MenuItemBuilder::with_id("tray_quit", "Quit").build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&tray_show)
                .separator()
                .item(&tray_quit)
                .build()?;

            let tray_icon = tauri::image::Image::from_bytes(TRAY_ICON)?;

            TrayIconBuilder::with_id("main-tray")
                .icon(tray_icon)
                .menu(&tray_menu)
                .tooltip("TeurTask")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "tray_show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "tray_quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Clear tray badge when the window receives focus
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "linux")]
            enable_media_capture(&window);
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(true) = event {
                    if let Some(tray) = app_handle.tray_by_id("main-tray") {
                        if let Ok(icon) = tauri::image::Image::from_bytes(TRAY_ICON) {
                            let _ = tray.set_icon(Some(icon));
                        }
                    }
                }
            });

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
