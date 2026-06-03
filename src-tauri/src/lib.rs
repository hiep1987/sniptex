pub mod agents;
pub mod capture;
mod commands;
#[cfg(desktop)]
mod hotkey;
pub mod ocr;
pub mod settings;
mod state;
pub mod storage;
mod tray;

use commands::{
    cancel_pdf_ocr, convert_to_tex, delete_api_key, delete_record, detect_agents, export_record,
    get_history, get_settings, has_api_key, hello, hide_window, open_external, rebind_hotkey,
    rerun_snip, run_pdf_ocr, run_snip, search_history, set_api_key, set_launch_at_login,
    show_window, test_agent, test_api_key, update_settings,
};

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("sniptex".into()),
                    }),
                ])
                .build(),
        )
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
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("app_data_dir resolvable on desktop");
            let history_store = storage::init(&app_data_dir).expect("storage init failed");
            app.manage(history_store);

            let settings_store = settings::SettingsStore::load(app.handle());
            let show_onboarding = !settings_store.get().onboarding_completed;
            app.manage(settings_store);

            #[cfg(desktop)]
            {
                app.manage(state::AppState::new());

                tray::init_tray(app.handle())?;
                hotkey::register_saved_shortcut(app.handle());
                hotkey::verify_registration(app.handle());

                if show_onboarding {
                    if let Some(w) = app.get_webview_window("onboarding") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
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
            test_api_key,
            run_snip,
            run_pdf_ocr,
            cancel_pdf_ocr,
            show_window,
            hide_window,
            open_external,
            get_history,
            search_history,
            delete_record,
            rerun_snip,
            export_record,
            get_settings,
            update_settings,
            rebind_hotkey,
            set_launch_at_login,
            convert_to_tex
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
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
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
