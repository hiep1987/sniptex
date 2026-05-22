pub mod agents;
mod commands;
pub mod ocr;

use commands::{
    delete_api_key, detect_agents, has_api_key, hello, run_snip, set_api_key, test_agent,
};

#[cfg(desktop)]
use std::sync::Mutex;
#[cfg(desktop)]
use std::time::{Duration, Instant};

#[cfg(desktop)]
use tauri::Emitter;
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

#[cfg(desktop)]
fn snip_shortcut() -> Shortcut {
    // Cmd+Shift+M on macOS, Ctrl+Shift+M on Windows/Linux.
    #[cfg(target_os = "macos")]
    let primary = Modifiers::SUPER;
    #[cfg(not(target_os = "macos"))]
    let primary = Modifiers::CONTROL;

    Shortcut::new(Some(Modifiers::SHIFT | primary), Code::KeyM)
}

// macOS Sequoia + global-hotkey 0.7 emit two Carbon `kEventHotKeyPressed` events
// per physical press. Also guards against accidental key-repeat double-trigger.
#[cfg(desktop)]
const HOTKEY_DEBOUNCE: Duration = Duration::from_millis(150);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_sql::Builder::default().build());

    #[cfg(desktop)]
    let last_press: &'static Mutex<Option<Instant>> = Box::leak(Box::new(Mutex::new(None)));

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([snip_shortcut()])
                .expect("global shortcut registration")
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if shortcut != &snip_shortcut() {
                        return;
                    }

                    let now = Instant::now();
                    let mut guard = match last_press.lock() {
                        Ok(g) => g,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    if let Some(prev) = *guard {
                        if now.duration_since(prev) < HOTKEY_DEBOUNCE {
                            return;
                        }
                    }
                    *guard = Some(now);
                    drop(guard);

                    println!("[sniptex] global hotkey pressed: Cmd/Ctrl+Shift+M");
                    if let Err(err) = app.emit("hotkey-pressed", ()) {
                        eprintln!("[sniptex] failed to emit hotkey-pressed: {err}");
                    }
                })
                .build(),
        );

    builder
        .setup(|_app| Ok(()))
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
