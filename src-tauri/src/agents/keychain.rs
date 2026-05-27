//! OS-keychain wrapper for BYOK API keys.
//!
//! Backed by the `keyring` crate: macOS Keychain, Windows Credential
//! Manager, Linux libsecret. Keys are never written to disk by SnipTeX
//! and never logged.
//!
//! On macOS dev builds (unsigned), keyring 3.x cannot read back items
//! across Entry instances due to per-binary keychain ACLs. An in-memory
//! fallback ensures set→get works within the same process session.
//! Production (codesigned) builds have a stable identity so the OS
//! keychain handles persistence across restarts.

use keyring::Entry;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

pub const SERVICE: &str = "com.sniptex";
pub const GEMINI_ACCOUNT: &str = "gemini-api-key";
pub const MISTRAL_ACCOUNT: &str = "mistral-api-key";

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

fn fallback_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn entry(account: &str) -> Result<Entry, KeychainError> {
    Entry::new(SERVICE, account).map_err(KeychainError::from)
}

pub fn set(account: &str, secret: &str) -> Result<(), KeychainError> {
    let _ = entry(account).and_then(|e| e.set_password(secret).map_err(KeychainError::from));
    if let Ok(mut guard) = fallback_store().lock() {
        guard.insert(account.to_string(), secret.to_string());
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
