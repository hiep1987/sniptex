pub mod agents;
pub mod capture;
mod commands;
#[cfg(desktop)]
mod hotkey;
pub mod ocr;
mod state;
mod tray;

use commands::{
    delete_api_key, detect_agents, has_api_key, hello, run_snip, set_api_key, test_agent,
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
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.manage(state::AppState::new());
                tray::init_tray(app.handle())?;
                hotkey::verify_registration(app.handle());
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
            run_snip
        ])
        .run(tauri::generate_context!())
        .expect("error while running SnipTeX");
}
