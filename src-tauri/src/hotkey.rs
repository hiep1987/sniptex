use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Wry};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

/// Event emitted on a successful hotkey press. App.tsx listens and
/// invokes `run_snip`.
pub const HOTKEY_PRESSED_EVENT: &str = "hotkey-pressed";

/// Event emitted if registering the default shortcut fails (already
/// claimed by another app). Frontend can show a "change shortcut" dialog.
pub const HOTKEY_CONFLICT_EVENT: &str = "hotkey-conflict";

// macOS Sequoia + global-hotkey 0.7 emit two Carbon `kEventHotKeyPressed`
// events per physical press; this window also absorbs accidental
// key-repeat double-triggers.
const HOTKEY_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Serialize, Clone)]
pub struct HotkeyConflictPayload {
    pub shortcut: String,
    pub reason: String,
}

/// Default snip shortcut: Cmd+Shift+M on macOS, Ctrl+Shift+M elsewhere.
/// Phase 8 will replace this with a user-configurable value pulled from
/// the settings store.
pub fn default_snip_shortcut() -> Shortcut {
    #[cfg(target_os = "macos")]
    let primary = Modifiers::SUPER;
    #[cfg(not(target_os = "macos"))]
    let primary = Modifiers::CONTROL;

    Shortcut::new(Some(Modifiers::SHIFT | primary), Code::KeyM)
}

fn shortcut_display() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Cmd+Shift+M"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl+Shift+M"
    }
}

/// Build the global-shortcut plugin with the default snip shortcut
/// pre-wired to its handler. The handler debounces, then emits
/// `hotkey-pressed` so the frontend can drive the snip flow.
///
/// This returns the plugin so `lib.rs` can register it on the builder
/// before `.setup()` runs.
pub fn build_plugin() -> TauriPlugin<Wry> {
    let last_press: &'static Mutex<Option<Instant>> = Box::leak(Box::new(Mutex::new(None)));
    let default_shortcut = default_snip_shortcut();

    tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts([default_shortcut])
            .expect("global shortcut registration")
        .with_handler(move |app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if shortcut != &default_snip_shortcut() {
                return;
            }

            let now = Instant::now();
            let mut guard = last_press.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(prev) = *guard {
                if now.duration_since(prev) < HOTKEY_DEBOUNCE {
                    return;
                }
            }
            *guard = Some(now);
            drop(guard);

            if let Err(err) = app.emit(HOTKEY_PRESSED_EVENT, ()) {
                eprintln!("[sniptex] failed to emit {HOTKEY_PRESSED_EVENT}: {err}");
            }
        })
        .build()
}

/// Verify the default shortcut is actually registered after the plugin
/// initializes. If a system-wide conflict prevents it, emit a
/// `hotkey-conflict` event so the UI can prompt for a new shortcut.
///
/// Call from the `.setup()` hook after the plugin is mounted.
pub fn verify_registration(app: &AppHandle) {
    let shortcut = default_snip_shortcut();
    let registered = app
        .global_shortcut()
        .is_registered(shortcut);

    if registered {
        return;
    }

    let payload = HotkeyConflictPayload {
        shortcut: shortcut_display().to_string(),
        reason: format!(
            "Could not register {} as a global shortcut. Another app may already own it.",
            shortcut_display()
        ),
    };
    if let Err(err) = app.emit(HOTKEY_CONFLICT_EVENT, payload) {
        eprintln!("[sniptex] failed to emit {HOTKEY_CONFLICT_EVENT}: {err}");
    }
}
