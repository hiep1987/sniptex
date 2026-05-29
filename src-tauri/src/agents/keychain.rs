//! OS-keychain wrapper for BYOK API keys.
//!
//! Backed by the `keyring` crate: macOS Keychain, Windows Credential
//! Manager, Linux libsecret.
//!
//! On macOS dev builds (unsigned), keyring 3.x cannot read back items
//! across Entry instances due to per-binary keychain ACLs. A file-backed
//! fallback at `{data_dir}/com.sniptex/api-keys.json` ensures keys
//! persist across restarts even when the OS keychain is inaccessible.

use keyring::Entry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

pub const SERVICE: &str = "com.sniptex";
pub const GEMINI_ACCOUNT: &str = "gemini-api-key";
pub const MISTRAL_ACCOUNT: &str = "mistral-api-key";
pub const CLOUD_VISION_ACCOUNT: &str = "cloud-vision-api-key";

const FALLBACK_FILENAME: &str = "api-keys.json";

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("keychain backend unavailable: {0}")]
    Backend(String),
    #[error("key not found")]
    NotFound,
}

impl From<keyring::Error> for KeychainError {
    fn from(e: keyring::Error) -> Self {
        match e {
            keyring::Error::NoEntry => KeychainError::NotFound,
            other => KeychainError::Backend(other.to_string()),
        }
    }
}

fn fallback_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(SERVICE)
        .join(FALLBACK_FILENAME)
}

fn fallback_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| {
        let map = load_fallback_file().unwrap_or_default();
        Mutex::new(map)
    })
}

fn load_fallback_file() -> Option<HashMap<String, String>> {
    let data = std::fs::read_to_string(fallback_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn persist_fallback(map: &HashMap<String, String>) {
    let path = fallback_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(map) {
        let _ = std::fs::write(&path, json);
    }
}

fn entry(account: &str) -> Result<Entry, KeychainError> {
    Entry::new(SERVICE, account).map_err(KeychainError::from)
}

pub fn set(account: &str, secret: &str) -> Result<(), KeychainError> {
    let _ = entry(account).and_then(|e| e.set_password(secret).map_err(KeychainError::from));
    if let Ok(mut guard) = fallback_store().lock() {
        guard.insert(account.to_string(), secret.to_string());
        persist_fallback(&guard);
    }
    Ok(())
}

pub fn get(account: &str) -> Result<String, KeychainError> {
    if let Ok(guard) = fallback_store().lock() {
        if let Some(val) = guard.get(account) {
            return Ok(val.clone());
        }
    }
    Ok(entry(account)?.get_password()?)
}

pub fn has(account: &str) -> bool {
    if let Ok(guard) = fallback_store().lock() {
        if guard.contains_key(account) {
            return true;
        }
    }
    match has_detailed(account) {
        Ok(present) => present,
        Err(e) => {
            log::warn!("[sniptex] keychain probe failed for `{account}`: {e}");
            false
        }
    }
}

pub fn has_detailed(account: &str) -> Result<bool, KeychainError> {
    if let Ok(guard) = fallback_store().lock() {
        if guard.contains_key(account) {
            return Ok(true);
        }
    }
    match entry(account)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(other) => Err(KeychainError::Backend(other.to_string())),
    }
}

pub fn delete(account: &str) -> Result<(), KeychainError> {
    if let Ok(mut guard) = fallback_store().lock() {
        guard.remove(account);
        persist_fallback(&guard);
    }
    let _ = entry(account).and_then(|e| e.delete_credential().map_err(KeychainError::from));
    Ok(())
}

pub fn has_gemini_api_key() -> bool {
    has(GEMINI_ACCOUNT)
}

pub fn get_gemini_api_key() -> Result<String, KeychainError> {
    get(GEMINI_ACCOUNT)
}

pub fn set_gemini_api_key(key: &str) -> Result<(), KeychainError> {
    set(GEMINI_ACCOUNT, key)
}

pub fn has_mistral_api_key() -> bool {
    has(MISTRAL_ACCOUNT)
}

pub fn get_mistral_api_key() -> Result<String, KeychainError> {
    get(MISTRAL_ACCOUNT)
}

pub fn set_mistral_api_key(key: &str) -> Result<(), KeychainError> {
    set(MISTRAL_ACCOUNT, key)
}

pub fn has_cloud_vision_api_key() -> bool {
    has(CLOUD_VISION_ACCOUNT)
}

pub fn get_cloud_vision_api_key() -> Result<String, KeychainError> {
    get(CLOUD_VISION_ACCOUNT)
}

pub fn set_cloud_vision_api_key(key: &str) -> Result<(), KeychainError> {
    set(CLOUD_VISION_ACCOUNT, key)
}
