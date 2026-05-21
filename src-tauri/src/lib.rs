mod commands;

use commands::hello;

#[cfg(desktop)]
use tauri::Emitter;
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

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
        ));

    builder
        .setup(|app| {
            #[cfg(desktop)]
            register_global_shortcut(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![hello])
        .run(tauri::generate_context!())
        .expect("error while running SnipTeX");
}

#[cfg(desktop)]
fn register_global_shortcut<R: tauri::Runtime>(
    app: &mut tauri::App<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Cmd+Shift+M on macOS, Ctrl+Shift+M on Windows/Linux.
    #[cfg(target_os = "macos")]
    let primary = Modifiers::SUPER;
    #[cfg(not(target_os = "macos"))]
    let primary = Modifiers::CONTROL;

    let snip_shortcut = Shortcut::new(Some(Modifiers::SHIFT | primary), Code::KeyM);

    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, shortcut, event| {
                if shortcut == &snip_shortcut && event.state() == ShortcutState::Pressed {
                    println!("[sniptex] global hotkey pressed: Cmd/Ctrl+Shift+M");
                    if let Err(err) = app.emit("hotkey-pressed", ()) {
                        eprintln!("[sniptex] failed to emit hotkey-pressed: {err}");
                    }
                }
            })
            .build(),
    )?;

    app.global_shortcut().register(snip_shortcut)?;
    Ok(())
}
