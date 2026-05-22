//! OS-keychain wrapper for BYOK API keys.
//!
//! Backed by the `keyring` crate: macOS Keychain, Windows Credential
//! Manager, Linux libsecret. Keys are never written to disk by SnipTeX
//! and never logged.

use keyring::Entry;
use thiserror::Error;

pub const SERVICE: &str = "com.sniptex";
pub const GEMINI_ACCOUNT: &str = "gemini-api-key";

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

fn entry(account: &str) -> Result<Entry, KeychainError> {
    Entry::new(SERVICE, account).map_err(KeychainError::from)
}

pub fn set(account: &str, secret: &str) -> Result<(), KeychainError> {
    entry(account)?.set_password(secret)?;
    Ok(())
}

pub fn get(account: &str) -> Result<String, KeychainError> {
    Ok(entry(account)?.get_password()?)
}

/// True iff a value is stored. Backend failures (keychain locked,
/// libsecret unavailable, etc.) are treated as "unknown → false" but
/// also logged so an outage doesn't silently masquerade as "no key
/// configured" and trigger a re-onboarding loop. Callers that need to
/// distinguish those cases should use `has_detailed`.
pub fn has(account: &str) -> bool {
    match has_detailed(account) {
        Ok(present) => present,
        Err(e) => {
            log::warn!("[sniptex] keychain probe failed for `{account}`: {e}");
            false
        }
    }
}

/// Variant of `has` that surfaces backend errors instead of swallowing
/// them. `Ok(true)` = value present, `Ok(false)` = explicit NotFound,
/// `Err(_)` = backend fault (do NOT treat as "not present").
pub fn has_detailed(account: &str) -> Result<bool, KeychainError> {
    match entry(account)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(other) => Err(KeychainError::Backend(other.to_string())),
    }
}

pub fn delete(account: &str) -> Result<(), KeychainError> {
    entry(account)?.delete_credential()?;
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
