//! API key resolution with a resilient fallback chain:
//! 1. OS keyring (service `linux-ai`, account = provider name)
//! 2. environment variable (`api_key_env`, or the provider default)
//! 3. plaintext `api_key` in the config file
//!
//! Keyring access can fail on headless systems without a secret service; in
//! that case we silently fall back to env/config rather than erroring.

use crate::config::ProviderConfig;
use crate::error::{Error, Result};

const KEYRING_SERVICE: &str = "linux-ai";

fn keyring_get(provider: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider).ok()?;
    match entry.get_password() {
        Ok(pw) if !pw.is_empty() => Some(pw),
        _ => None,
    }
}

/// Resolve the API key for a provider, or `None` if unset everywhere.
pub fn resolve_api_key(provider_name: &str, cfg: &ProviderConfig) -> Option<String> {
    if let Some(key) = keyring_get(provider_name) {
        return Some(key);
    }
    if let Some(var) = &cfg.api_key_env {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    if let Some(key) = &cfg.api_key {
        if !key.is_empty() {
            return Some(key.clone());
        }
    }
    None
}

/// Like [`resolve_api_key`] but returns a typed error when missing.
pub fn require_api_key(provider_name: &str, cfg: &ProviderConfig) -> Result<String> {
    resolve_api_key(provider_name, cfg)
        .ok_or_else(|| Error::MissingApiKey(provider_name.to_string()))
}

/// Store an API key in the OS keyring.
pub fn store_api_key(provider_name: &str, key: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider_name)
        .map_err(|e| Error::other(format!("keyring unavailable: {e}")))?;
    entry
        .set_password(key)
        .map_err(|e| Error::other(format!("failed to store key: {e}")))?;
    Ok(())
}

/// Remove a stored API key from the OS keyring (no error if absent).
pub fn delete_api_key(provider_name: &str) -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, provider_name) {
        let _ = entry.delete_credential();
    }
    Ok(())
}
