pub mod agents;
pub mod capture;
mod commands;
#[cfg(desktop)]
mod hotkey;
pub mod ocr;
mod state;
mod tray;

use commands::{
    delete_api_key, detect_agents, has_api_key, hello, hide_window, run_snip, set_api_key,
    show_window, test_agent,
};

#[cfg(desktop)]
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_sql::Builder::default().build());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(hotkey::build_plugin());

    builder
        .on_window_event(|window, event| {
            // Without this, clicking the X on Settings / History /
            // Onboarding destroys the webview. Subsequent
            // `get_webview_window("settings")` calls return None and the
            // tray + main-window buttons become no-ops. Intercept the
            // close request and hide instead so the same handle stays
            // valid for the next show.
            //
            // The `overlay` and `preview` windows already use
            // programmatic hide() and never receive a user-driven close
            // (no decorations), so the intercept is a no-op for them.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if matches!(label, "main" | "settings" | "history" | "onboarding") {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.manage(state::AppState::new());

                tray::init_tray(app.handle())?;
                hotkey::verify_registration(app.handle());

                // (Don't auto-open Preview DevTools — on macOS that
                // force-shows the hidden window. Right-click any
                // visible Preview after a snip to inspect.)
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hello,
            detect_agents,
            test_agent,
            set_api_key,
            has_api_key,
            delete_api_key,
            run_snip,
            show_window,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running SnipTeX");
}
