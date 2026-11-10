//! Per-account privacy policy persistence across daemon restarts.
//!
//! Kept in the account's own state directory on desktop and in `localStorage`
//! on the web, behind a single `mod imp` platform split. Unit tests that build
//! a [`crate::state::StateHub`] directly start from [`PrivacyPolicy::default`]
//! without touching disk; `session_bridge::run` loads the stored policy when an
//! account runtime starts.

use oxidezap_core::{AccountId, PrivacyPolicy};

/// Load the stored privacy policy for `account_id`, falling back to
/// [`PrivacyPolicy::default`] when none has been saved yet.
#[must_use]
pub fn load(account_id: AccountId) -> PrivacyPolicy {
    imp::load(account_id).normalized()
}

/// Persist `policy` for `account_id`.
pub fn save(account_id: AccountId, policy: PrivacyPolicy) {
    imp::save(account_id, policy.normalized());
}

/// Remove any stored privacy policy for `account_id` during account reset or
/// removal.
pub fn forget(account_id: AccountId) {
    imp::forget(account_id);
}

#[cfg(not(target_family = "wasm"))]
mod imp {
    use std::path::PathBuf;

    use oxidezap_core::{AccountId, PrivacyPolicy};

    const FILE_NAME: &str = "privacy.json";

    fn path(account_id: AccountId) -> Option<PathBuf> {
        crate::plugins::account_state_dir(account_id).map(|dir| dir.join(FILE_NAME))
    }

    pub(super) fn load(account_id: AccountId) -> PrivacyPolicy {
        let Some(path) = path(account_id) else {
            return PrivacyPolicy::default();
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return PrivacyPolicy::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub(super) fn save(account_id: AccountId, policy: PrivacyPolicy) {
        let Some(path) = path(account_id) else {
            return;
        };
        let Some(parent) = path.parent() else {
            return;
        };
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!("could not prepare privacy state directory: {e}");
            return;
        }
        let Ok(json) = serde_json::to_string_pretty(&policy) else {
            return;
        };
        if let Err(e) = std::fs::write(&path, json.as_bytes()) {
            log::warn!("could not save privacy policy to {}: {e}", path.display());
        }
    }

    pub(super) fn forget(account_id: AccountId) {
        let Some(path) = path(account_id) else {
            return;
        };
        if let Err(e) = std::fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!("could not remove {}: {e}", path.display());
        }
    }
}

#[cfg(target_family = "wasm")]
mod imp {
    use oxidezap_core::{AccountId, PrivacyPolicy};

    fn storage_key(account_id: AccountId) -> String {
        format!("oxidezap.privacy.{}", account_id.get())
    }

    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub(super) fn load(account_id: AccountId) -> PrivacyPolicy {
        let Some(storage) = local_storage() else {
            return PrivacyPolicy::default();
        };
        let Ok(Some(raw)) = storage.get_item(&storage_key(account_id)) else {
            return PrivacyPolicy::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub(super) fn save(account_id: AccountId, policy: PrivacyPolicy) {
        let Some(storage) = local_storage() else {
            return;
        };
        let Ok(raw) = serde_json::to_string(&policy) else {
            return;
        };
        let _ = storage.set_item(&storage_key(account_id), &raw);
    }

    pub(super) fn forget(account_id: AccountId) {
        if let Some(storage) = local_storage() {
            let _ = storage.remove_item(&storage_key(account_id));
        }
    }
}
