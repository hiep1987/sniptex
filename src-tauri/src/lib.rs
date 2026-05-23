pub mod agents;
pub mod capture;
mod commands;
#[cfg(desktop)]
mod hotkey;
pub mod ocr;
mod state;
pub mod storage;
mod tray;

use commands::{
    delete_api_key, delete_record, detect_agents, export_record, get_history, has_api_key, hello,
    hide_window, rerun_snip, run_snip, search_history, set_api_key, show_window, test_agent,
};

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
            // Phase 7: open snip-history SQLite + WAL + ensure images/thumbs
            // dirs exist. Hard-failing on init keeps the bug close to the
            // root cause — without storage the rest of the app is useless.
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("app_data_dir resolvable on desktop");
            let history_store = storage::init(&app_data_dir)
                .expect("storage init failed");
            app.manage(history_store);

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
            hide_window,
            get_history,
            search_history,
            delete_record,
            rerun_snip,
            export_record
        ])
        .build(tauri::generate_context!())
        .expect("error while building SnipTeX")
        .run(|app, event| {
            // macOS only: clicking the dock icon while all SnipTeX windows
            // are hidden (either because the user closed them — our
            // close-intercept calls window.hide() — or because run_snip
            // ran `app.hide()` for capture) sends `applicationShouldHandle
            // Reopen:hasVisibleWindows:NO`. Without a handler, Tauri's
            // default does nothing and the user thinks the app is dead.
            // Re-show the main window so the user has somewhere to land.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { has_visible_windows, .. } = event {
                if !has_visible_windows {
                    if let Some(main) = app.get_webview_window("main") {
                        let _ = main.show();
                        let _ = main.unminimize();
                        let _ = main.set_focus();
                    }
                }
            }
            // Suppress unused-variable lint on non-macOS builds.
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (app, event);
            }
        });
}
