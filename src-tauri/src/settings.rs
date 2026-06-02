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
    pub copy_as_formats: Vec<OutputFormat>,
    pub history_size: HistorySize,
    pub preview_duration_ms: u32,
    pub sound_on_success: bool,
    pub launch_at_login: bool,
    pub theme: ThemeMode,
    pub onboarding_completed: bool,
    pub cloud_mode_enabled: bool,
    #[serde(default)]
    pub local_ocr_enabled: bool,
    #[serde(default = "default_local_ocr_url")]
    pub local_ocr_url: String,
    #[serde(default = "default_enabled")]
    pub local_ocr_formula_enabled: bool,
    #[serde(default = "default_enabled")]
    pub local_ocr_text_enabled: bool,
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
            local_ocr_enabled: false,
            local_ocr_url: default_local_ocr_url(),
            local_ocr_formula_enabled: true,
            local_ocr_text_enabled: true,
        }
    }
}

pub fn default_local_ocr_url() -> String {
    "http://127.0.0.1:8765".to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_hotkey_string() -> String {
    if cfg!(target_os = "macos") {
        "Command+Shift+M".to_string()
    } else {
        "Control+Shift+M".to_string()
    }
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
        apply_patch(&mut guard, patch)?;
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
    pub copy_as_formats: Option<Vec<OutputFormat>>,
    pub history_size: Option<HistorySize>,
    pub preview_duration_ms: Option<u32>,
    pub sound_on_success: Option<bool>,
    pub launch_at_login: Option<bool>,
    pub theme: Option<ThemeMode>,
    pub onboarding_completed: Option<bool>,
    pub cloud_mode_enabled: Option<bool>,
    pub local_ocr_enabled: Option<bool>,
    pub local_ocr_url: Option<String>,
    pub local_ocr_formula_enabled: Option<bool>,
    pub local_ocr_text_enabled: Option<bool>,
}

fn apply_patch(settings: &mut AppSettings, patch: SettingsPatch) -> Result<(), String> {
    if let Some(v) = patch.hotkey {
        settings.hotkey = v;
    }
    if let Some(v) = patch.agent_priority {
        settings.agent_priority = v;
    }
    if let Some(v) = patch.default_format {
        settings.default_format = v;
    }
    if let Some(v) = patch.copy_as_formats {
        settings.copy_as_formats = v;
    }
    if let Some(v) = patch.history_size {
        settings.history_size = v;
    }
    if let Some(v) = patch.preview_duration_ms {
        settings.preview_duration_ms = v;
    }
    if let Some(v) = patch.sound_on_success {
        settings.sound_on_success = v;
    }
    if let Some(v) = patch.launch_at_login {
        settings.launch_at_login = v;
    }
    if let Some(v) = patch.theme {
        settings.theme = v;
    }
    if let Some(v) = patch.onboarding_completed {
        settings.onboarding_completed = v;
    }
    if let Some(v) = patch.cloud_mode_enabled {
        settings.cloud_mode_enabled = v;
    }
    if let Some(v) = patch.local_ocr_enabled {
        settings.local_ocr_enabled = v;
    }
    if let Some(v) = patch.local_ocr_url {
        let trimmed = v.trim();
        validate_local_ocr_url(trimmed)?;
        settings.local_ocr_url = trimmed.to_string();
    }
    if let Some(v) = patch.local_ocr_formula_enabled {
        settings.local_ocr_formula_enabled = v;
    }
    if let Some(v) = patch.local_ocr_text_enabled {
        settings.local_ocr_text_enabled = v;
    }
    Ok(())
}

pub fn validate_local_ocr_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| {
        "local_ocr_url must be a valid http://127.0.0.1 or http://localhost URL".to_string()
    })?;
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    if parsed.scheme() == "http"
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && (host == "127.0.0.1" || host == "localhost")
    {
        return Ok(());
    }
    Err("local_ocr_url must be http://127.0.0.1 or http://localhost without credentials".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ocr_url_accepts_loopback_only() {
        assert!(validate_local_ocr_url("http://127.0.0.1:8765").is_ok());
        assert!(validate_local_ocr_url("http://localhost:8765").is_ok());
        assert!(validate_local_ocr_url("https://127.0.0.1:8765").is_err());
        assert!(validate_local_ocr_url("http://192.168.1.10:8765").is_err());
        assert!(validate_local_ocr_url("http://localhost.evil.test:8765").is_err());
        assert!(validate_local_ocr_url("http://localhost:8765@evil.test").is_err());
        assert!(validate_local_ocr_url("http://127.0.0.1:8765@evil.test").is_err());
    }

    #[test]
    fn settings_patch_rejects_remote_local_ocr_url() {
        let mut settings = AppSettings::default();
        let result = apply_patch(
            &mut settings,
            SettingsPatch {
                local_ocr_url: Some("http://example.com:8765".to_string()),
                ..SettingsPatch::default()
            },
        );
        assert!(result.is_err());
        assert_eq!(settings.local_ocr_url, default_local_ocr_url());
    }
}
