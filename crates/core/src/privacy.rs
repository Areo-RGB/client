//! Outgoing privacy policy and message-preservation settings.
//!
//! One wire-serializable struct shared between the daemon, front ends, and
//! per-account persistence so every surface agrees on which outgoing signals
//! (`read`/`played` receipts, typing/recording indicators, online availability)
//! are suppressed and whether incoming message deletions retain their original
//! payload.

use serde::{Deserialize, Serialize};

/// Maximum safety delay (in seconds) allowed before automatically sending read
/// receipts in an open, focused conversation.
pub const MAX_SAFETY_DELAY_SECS: u8 = 30;

fn default_true() -> bool {
    true
}

/// User-configured privacy and incognito enforcement rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyPolicy {
    /// Suppress automatic outgoing `"read"`, `"read-self"`, and `"played"`
    /// receipts until the user manually confirms, replies (when
    /// [`Self::auto_receipt_on_reply`] is enabled), or the safety delay elapses.
    #[serde(default)]
    pub block_read_receipts: bool,
    /// When [`Self::block_read_receipts`] is active, automatically flush read
    /// receipts for a chat when the user sends a message in it.
    #[serde(default = "default_true")]
    pub auto_receipt_on_reply: bool,
    /// Seconds to wait in an open, focused chat before automatically sending
    /// read receipts (`0` = never auto-send while [`Self::block_read_receipts`]
    /// is enabled; clamped to `0..=30`).
    #[serde(default)]
    pub safety_delay_secs: u8,
    /// Suppress outgoing `Composing` and `Recording` chat presence updates.
    #[serde(default)]
    pub block_typing_updates: bool,
    /// Suppress outgoing `Available` presence updates.
    #[serde(default)]
    pub block_online_updates: bool,
    /// Retain original text and media payloads in the local store when another
    /// participant revokes a message, displaying it with a deleted badge
    /// instead of replacing its body with a bare tombstone.
    #[serde(default)]
    pub preserve_deleted_messages: bool,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            block_read_receipts: false,
            auto_receipt_on_reply: true,
            safety_delay_secs: 0,
            block_typing_updates: false,
            block_online_updates: false,
            preserve_deleted_messages: false,
        }
    }
}

impl PrivacyPolicy {
    /// Clamp [`Self::safety_delay_secs`] into the supported `0..=MAX_SAFETY_DELAY_SECS` range.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.safety_delay_secs > MAX_SAFETY_DELAY_SECS {
            self.safety_delay_secs = MAX_SAFETY_DELAY_SECS;
        }
        self
    }

    /// Return [`Self::safety_delay_secs`] clamped to `0..=MAX_SAFETY_DELAY_SECS`.
    #[must_use]
    pub fn clamped_safety_delay_secs(&self) -> u8 {
        self.safety_delay_secs.min(MAX_SAFETY_DELAY_SECS)
    }

    /// Return a copy with [`Self::safety_delay_secs`] clamped to `0..=MAX_SAFETY_DELAY_SECS`.
    #[must_use]
    pub fn with_safety_delay(mut self, secs: u8) -> Self {
        self.safety_delay_secs = secs.min(MAX_SAFETY_DELAY_SECS);
        self
    }

    /// Whether any incognito protection is currently enabled.
    #[must_use]
    pub fn is_incognito_active(&self) -> bool {
        self.block_read_receipts
            || self.block_typing_updates
            || self.block_online_updates
            || self.preserve_deleted_messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_standard_client_behaviour_with_reply_flush_ready() {
        let policy = PrivacyPolicy::default();
        assert!(!policy.block_read_receipts);
        assert!(policy.auto_receipt_on_reply);
        assert_eq!(policy.safety_delay_secs, 0);
        assert!(!policy.block_typing_updates);
        assert!(!policy.block_online_updates);
        assert!(!policy.preserve_deleted_messages);
        assert!(!policy.is_incognito_active());
    }

    #[test]
    fn safety_delay_clamps_to_maximum() {
        let policy = PrivacyPolicy::default().with_safety_delay(99);
        assert_eq!(policy.safety_delay_secs, MAX_SAFETY_DELAY_SECS);
    }

    #[test]
    fn missing_json_fields_deserialize_with_safe_defaults() {
        let parsed: PrivacyPolicy = serde_json::from_str(r#"{"block_read_receipts":true}"#)
            .expect("partial JSON should deserialize");
        assert!(parsed.block_read_receipts);
        assert!(parsed.auto_receipt_on_reply);
        assert_eq!(parsed.safety_delay_secs, 0);
        assert!(parsed.is_incognito_active());
    }
}
