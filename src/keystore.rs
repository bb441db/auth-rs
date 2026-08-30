use std::sync::{Arc, OnceLock};

use keyring_core::CredentialStore;

use crate::error::{AuthError, Result};

#[cfg(target_os = "linux")]
fn build_store() -> keyring_core::Result<Arc<CredentialStore>> {
    linux_keyutils_keyring_store::Store::new().map(|store| store as Arc<CredentialStore>)
}

#[cfg(target_os = "macos")]
fn build_store() -> keyring_core::Result<Arc<CredentialStore>> {
    apple_native_keyring_store::Store::new().map(|store| store as Arc<CredentialStore>)
}

#[cfg(target_os = "windows")]
fn build_store() -> keyring_core::Result<Arc<CredentialStore>> {
    windows_native_keyring_store::Store::new().map(|store| store as Arc<CredentialStore>)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn build_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Err(keyring_core::Error::NoStorageAccess(
        "No credential store backend is available for this platform".into(),
    ))
}

static INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

pub fn ensure_initialized() -> Result<()> {
    match INIT.get_or_init(|| {
        build_store()
            .map(keyring_core::set_default_store)
            .map_err(|e| e.to_string())
    }) {
        Ok(()) => Ok(()),
        Err(e) => Err(AuthError::KeyringError(e.clone())),
    }
}
