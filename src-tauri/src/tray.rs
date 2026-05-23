use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::{AppState, TrayStatus};

// Tray menu event ids. Frontend listens for the matching tauri events.
const MENU_SHOW_MAIN: &str = "show-main";
const MENU_SNIP_NOW: &str = "snip-now";
const MENU_SHOW_HISTORY: &str = "show-history";
const MENU_OPEN_SETTINGS: &str = "open-settings";
const MENU_ABOUT: &str = "about";
const MENU_QUIT: &str = "quit";

// Emitted by tray menu clicks; consumed by the snip pipeline / main window.
pub const TRAY_SNIP_NOW_EVENT: &str = "tray-snip-now";
pub const TRAY_SHOW_HISTORY_EVENT: &str = "tray-show-history";
pub const TRAY_OPEN_SETTINGS_EVENT: &str = "tray-open-settings";
pub const TRAY_ABOUT_EVENT: &str = "tray-about";

// Mac template PNGs live next to the bundled app icons. Loaded at runtime
// via the resource path so they ship with the bundle on all platforms.
const ICON_IDLE: &[u8] = include_bytes!("../icons/tray/tray-idle.png");
const ICON_CAPTURING: &[u8] = include_bytes!("../icons/tray/tray-capturing.png");
const ICON_PROCESSING: &[u8] = include_bytes!("../icons/tray/tray-processing.png");
const ICON_ERROR: &[u8] = include_bytes!("../icons/tray/tray-error.png");

fn icon_for(status: TrayStatus) -> tauri::Result<Image<'static>> {
    let bytes = match status {
        TrayStatus::Idle => ICON_IDLE,
        TrayStatus::Capturing => ICON_CAPTURING,
        TrayStatus::Processing => ICON_PROCESSING,
        TrayStatus::Error => ICON_ERROR,
    };
    Image::from_bytes(bytes)
}

/// Build the tray icon, menu, and click handlers, and store the handle
/// in `AppState` so `set_status` can swap the icon later.
pub fn init_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_main = MenuItemBuilder::with_id(MENU_SHOW_MAIN, "Show SnipTeX").build(app)?;
    let snip_now = MenuItemBuilder::with_id(MENU_SNIP_NOW, "Snip Now")
        .accelerator("CmdOrCtrl+Shift+M")
        .build(app)?;
    let show_history = MenuItemBuilder::with_id(MENU_SHOW_HISTORY, "Show History").build(app)?;
    let open_settings = MenuItemBuilder::with_id(MENU_OPEN_SETTINGS, "Open Settings…").build(app)?;
    let about = MenuItemBuilder::with_id(MENU_ABOUT, "About SnipTeX").build(app)?;
    let quit = MenuItemBuilder::with_id(MENU_QUIT, "Quit SnipTeX")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show_main)
        .item(&snip_now)
        .item(&separator)
        .item(&show_history)
        .item(&open_settings)
        .item(&separator)
        .item(&about)
        .item(&separator)
        .item(&quit)
        .build()?;

    #[allow(unused_mut)]
    let mut builder = TrayIconBuilder::with_id("sniptex-tray")
        .icon(icon_for(TrayStatus::Idle)?)
        .tooltip(TrayStatus::Idle.tooltip())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW_MAIN => {
                show_window_by_label(app, "main");
            }
            MENU_SNIP_NOW => {
                let _ = app.emit(TRAY_SNIP_NOW_EVENT, ());
            }
            MENU_SHOW_HISTORY => {
                show_window_by_label(app, "history");
                let _ = app.emit(TRAY_SHOW_HISTORY_EVENT, ());
            }
            MENU_OPEN_SETTINGS => {
                show_window_by_label(app, "settings");
                let _ = app.emit(TRAY_OPEN_SETTINGS_EVENT, ());
            }
            MENU_ABOUT => {
                // About lives as a tab inside Settings. Show the window
                // first, then emit so the Settings shell can jump tabs.
                show_window_by_label(app, "settings");
                let _ = app.emit(TRAY_ABOUT_EVENT, ());
            }
            MENU_QUIT => {
                app.exit(0);
            }
            _ => {}
        });

    // On macOS, treating the PNG as a template lets the OS auto-tint for
    // light/dark menu bar. Windows + Linux ignore this and render as-is.
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    let tray = builder.build(app)?;

    if let Some(state) = app.try_state::<AppState>() {
        let mut handle = state
            .tray_handle
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        *handle = Some(tray);
    }

    Ok(())
}

/// Swap the tray icon + tooltip to reflect a new status. Also updates
/// the cached status in `AppState`. Cheap to call from any thread.
pub fn set_status(app: &AppHandle, status: TrayStatus) {
    let Some(state) = app.try_state::<AppState>() else { return };

    {
        let mut cur = state
            .current_status
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if *cur == status {
            return;
        }
        *cur = status;
    }

    let handle_guard = state
        .tray_handle
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let Some(tray) = handle_guard.as_ref() else { return };

    if let Ok(icon) = icon_for(status) {
        let _ = tray.set_icon(Some(icon));
    }
    let _ = tray.set_tooltip(Some(status.tooltip()));
}

/// Show the Error icon for a brief moment, then auto-reset to Idle.
/// Use after a snip failure so the user can glance and see what state
/// the app ended up in.
pub fn flash_error(app: AppHandle) {
    set_status(&app, TrayStatus::Error);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let cur = if let Some(state) = app.try_state::<AppState>() {
            let guard = state
                .current_status
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            *guard
        } else {
            TrayStatus::Idle
        };
        // Only reset if no new status was set in the meantime.
        if cur == TrayStatus::Error {
            set_status(&app, TrayStatus::Idle);
        }
    });
}

fn show_window_by_label(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
