use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::settings::SettingsStore;

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

/// Parse a human-readable shortcut string (e.g. "Command+Shift+M") into
/// the tauri `Shortcut` struct. Returns `Err` if the key code is unknown.
pub fn parse_shortcut(s: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = s.split('+').map(str::trim).collect();
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "command" | "cmd" | "super" | "meta" => mods |= Modifiers::SUPER,
            "control" | "ctrl" | "commandorcontrol" | "cmdorctrl" => {
                #[cfg(target_os = "macos")]
                {
                    mods |= Modifiers::SUPER;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    mods |= Modifiers::CONTROL;
                }
            }
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            key => {
                code = Some(key_str_to_code(key)?);
            }
        }
    }

    let code = code.ok_or_else(|| format!("no key code found in shortcut: {s}"))?;
    Ok(Shortcut::new(Some(mods), code))
}

fn key_str_to_code(key: &str) -> Result<Code, String> {
    let code = match key.to_uppercase().as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" | "DIGIT0" => Code::Digit0,
        "1" | "DIGIT1" => Code::Digit1,
        "2" | "DIGIT2" => Code::Digit2,
        "3" | "DIGIT3" => Code::Digit3,
        "4" | "DIGIT4" => Code::Digit4,
        "5" | "DIGIT5" => Code::Digit5,
        "6" | "DIGIT6" => Code::Digit6,
        "7" | "DIGIT7" => Code::Digit7,
        "8" | "DIGIT8" => Code::Digit8,
        "9" | "DIGIT9" => Code::Digit9,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "SPACE" => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        "ESCAPE" | "ESC" => Code::Escape,
        "TAB" => Code::Tab,
        "BACKSPACE" => Code::Backspace,
        "DELETE" => Code::Delete,
        "MINUS" | "-" => Code::Minus,
        "EQUAL" | "=" => Code::Equal,
        "BRACKETLEFT" | "[" => Code::BracketLeft,
        "BRACKETRIGHT" | "]" => Code::BracketRight,
        "SEMICOLON" | ";" => Code::Semicolon,
        "QUOTE" | "'" => Code::Quote,
        "BACKQUOTE" | "`" => Code::Backquote,
        "COMMA" | "," => Code::Comma,
        "PERIOD" | "." => Code::Period,
        "SLASH" | "/" => Code::Slash,
        other => return Err(format!("unknown key: {other}")),
    };
    Ok(code)
}

/// Build the global-shortcut plugin with the snip shortcut handler.
/// The actual shortcut registered depends on the user's saved settings
/// (loaded during `.setup()` via `register_saved_shortcut`).
pub fn build_plugin() -> TauriPlugin<Wry> {
    let last_press: &'static Mutex<Option<Instant>> = Box::leak(Box::new(Mutex::new(None)));

    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
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

/// Register the user's saved hotkey (or default). Call from `.setup()`
/// after `SettingsStore` is managed.
pub fn register_saved_shortcut(app: &AppHandle) {
    let hotkey_str = app
        .try_state::<SettingsStore>()
        .map(|s| s.get().hotkey)
        .unwrap_or_else(|| default_snip_shortcut_string());

    let shortcut = match parse_shortcut(&hotkey_str) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("bad saved hotkey '{hotkey_str}': {e}, falling back to default");
            default_snip_shortcut()
        }
    };

    match app.global_shortcut().register(shortcut) {
        Ok(()) => {}
        Err(e) => {
            log::warn!("hotkey registration failed: {e}");
            let payload = HotkeyConflictPayload {
                shortcut: hotkey_str,
                reason: format!("Could not register shortcut: {e}"),
            };
            let _ = app.emit(HOTKEY_CONFLICT_EVENT, payload);
        }
    }
}

/// Unregister the current shortcut and register a new one. On failure,
/// attempt to re-register the previous shortcut so the user is never
/// left without a working hotkey.
pub fn rebind(app: &AppHandle, new_shortcut_str: &str) -> Result<(), String> {
    let new_shortcut = parse_shortcut(new_shortcut_str)?;

    let gs = app.global_shortcut();

    // Unregister all existing shortcuts first.
    gs.unregister_all()
        .map_err(|e| format!("unregister_all: {e}"))?;

    match gs.register(new_shortcut) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Rollback: re-register the old shortcut.
            let old_str = app
                .try_state::<SettingsStore>()
                .map(|s| s.get().hotkey)
                .unwrap_or_else(|| default_snip_shortcut_string());
            if let Ok(old) = parse_shortcut(&old_str) {
                let _ = gs.register(old);
            }
            Err(format!("failed to register '{new_shortcut_str}': {e}"))
        }
    }
}

fn default_snip_shortcut_string() -> String {
    if cfg!(target_os = "macos") {
        "Command+Shift+M".to_string()
    } else {
        "Control+Shift+M".to_string()
    }
}

/// Verify the shortcut is registered. Emit conflict event if not.
pub fn verify_registration(app: &AppHandle) {
    let hotkey_str = app
        .try_state::<SettingsStore>()
        .map(|s| s.get().hotkey)
        .unwrap_or_else(|| default_snip_shortcut_string());

    let shortcut = match parse_shortcut(&hotkey_str) {
        Ok(s) => s,
        Err(_) => default_snip_shortcut(),
    };

    if app.global_shortcut().is_registered(shortcut) {
        return;
    }

    let payload = HotkeyConflictPayload {
        shortcut: hotkey_str.clone(),
        reason: format!(
            "Could not register {} as a global shortcut. Another app may already own it.",
            hotkey_str
        ),
    };
    if let Err(err) = app.emit(HOTKEY_CONFLICT_EVENT, payload) {
        eprintln!("[sniptex] failed to emit {HOTKEY_CONFLICT_EVENT}: {err}");
    }
}
