use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::agents::registry::DEFAULT_FALLBACK_CHAIN;

const STORE_FILENAME: &str = "settings.json";
const STORE_KEY: &str = "app_settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Smart,
    Inline,
    Display,
    Plain,
    Markdown,
    MathMl,
    UnicodePretty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistorySize {
    Fifty,
    OneHundred,
    FiveHundred,
    Unlimited,
}

impl HistorySize {
    pub fn to_limit(self) -> Option<usize> {
        match self {
            HistorySize::Fifty => Some(50),
            HistorySize::OneHundred => Some(100),
            HistorySize::FiveHundred => Some(500),
            HistorySize::Unlimited => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey: String,
    pub agent_priority: Vec<String>,
    pub default_format: OutputFormat,
    #[serde(default = "default_history_copy_format")]
    pub history_copy_format: OutputFormat,
    pub copy_as_formats: Vec<OutputFormat>,
    pub history_size: HistorySize,
    pub preview_duration_ms: u32,
    pub sound_on_success: bool,
    pub launch_at_login: bool,
    pub theme: ThemeMode,
    pub onboarding_completed: bool,
    pub cloud_mode_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey_string(),
            agent_priority: DEFAULT_FALLBACK_CHAIN
                .iter()
                .map(|s| s.to_string())
                .collect(),
            default_format: OutputFormat::Smart,
            history_copy_format: default_history_copy_format(),
            copy_as_formats: vec![
                OutputFormat::Plain,
                OutputFormat::Smart,
                OutputFormat::Inline,
                OutputFormat::Display,
                OutputFormat::Markdown,
            ],
            history_size: HistorySize::OneHundred,
            preview_duration_ms: 3000,
            sound_on_success: true,
            launch_at_login: false,
            theme: ThemeMode::System,
            onboarding_completed: false,
            cloud_mode_enabled: false,
        }
    }
}

fn default_hotkey_string() -> String {
    if cfg!(target_os = "macos") {
        "Command+Shift+M".to_string()
    } else {
        "Control+Shift+M".to_string()
    }
}

fn default_history_copy_format() -> OutputFormat {
    OutputFormat::Smart
}

pub struct SettingsStore {
    pub inner: Mutex<AppSettings>,
    app: AppHandle,
}

impl SettingsStore {
    pub fn load(app: &AppHandle) -> Self {
        let store = app.store(STORE_FILENAME).ok();

        let settings = store
            .as_ref()
            .and_then(|s| s.get(STORE_KEY))
            .and_then(|v| serde_json::from_value::<AppSettings>(v).ok())
            .unwrap_or_default();

        Self {
            inner: Mutex::new(settings),
            app: app.clone(),
        }
    }

    pub fn get(&self) -> AppSettings {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn update(&self, patch: SettingsPatch) -> Result<AppSettings, String> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        apply_patch(&mut guard, patch);
        self.persist(&guard)?;
        Ok(guard.clone())
    }

    pub fn set_full(&self, settings: AppSettings) -> Result<(), String> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        *guard = settings;
        self.persist(&guard)
    }

    fn persist(&self, settings: &AppSettings) -> Result<(), String> {
        let store = self
            .app
            .store(STORE_FILENAME)
            .map_err(|e| format!("open store: {e}"))?;
        let value =
            serde_json::to_value(settings).map_err(|e| format!("serialize settings: {e}"))?;
        store.set(STORE_KEY, value);
        store.save().map_err(|e| format!("save store: {e}"))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SettingsPatch {
    pub hotkey: Option<String>,
    pub agent_priority: Option<Vec<String>>,
    pub default_format: Option<OutputFormat>,
    pub history_copy_format: Option<OutputFormat>,
    pub copy_as_formats: Option<Vec<OutputFormat>>,
    pub history_size: Option<HistorySize>,
    pub preview_duration_ms: Option<u32>,
    pub sound_on_success: Option<bool>,
    pub launch_at_login: Option<bool>,
    pub theme: Option<ThemeMode>,
    pub onboarding_completed: Option<bool>,
    pub cloud_mode_enabled: Option<bool>,
}

fn apply_patch(settings: &mut AppSettings, patch: SettingsPatch) {
    set_if_some(&mut settings.hotkey, patch.hotkey);
    set_if_some(&mut settings.agent_priority, patch.agent_priority);
    set_if_some(&mut settings.default_format, patch.default_format);
    set_if_some(&mut settings.history_copy_format, patch.history_copy_format);
    set_if_some(&mut settings.copy_as_formats, patch.copy_as_formats);
    set_if_some(&mut settings.history_size, patch.history_size);
    set_if_some(&mut settings.preview_duration_ms, patch.preview_duration_ms);
    set_if_some(&mut settings.sound_on_success, patch.sound_on_success);
    set_if_some(&mut settings.launch_at_login, patch.launch_at_login);
    set_if_some(&mut settings.theme, patch.theme);
    set_if_some(&mut settings.onboarding_completed, patch.onboarding_completed);
    set_if_some(&mut settings.cloud_mode_enabled, patch.cloud_mode_enabled);
}

fn set_if_some<T>(target: &mut T, value: Option<T>) {
    if let Some(v) = value {
        *target = v;
    }
}
