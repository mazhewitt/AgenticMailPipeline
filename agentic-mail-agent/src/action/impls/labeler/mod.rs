//! Email labeling module for applying Gmail labels based on classification results.
//!
//! This module provides an abstraction for applying labels to emails in Gmail.
//! It supports creating labels if they don't exist and applying them idempotently.

mod gmail;
mod stub;

pub use gmail::{GmailLabeler, ConcreteGmailLabeler, LabelInfo};
pub use stub::StubLabeler;

use async_trait::async_trait;

/// Result of a labeling operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelingResult {
    /// Gmail message ID that was labeled
    pub message_id: String,
    /// Label that was applied
    pub label: String,
    /// Whether a new label was created (true) or existing label was used (false)
    pub created_new_label: bool,
    /// Human-readable description of what happened
    pub description: String,
}

impl LabelingResult {
    /// Create a new labeling result
    pub fn new(
        message_id: String,
        label: String,
        created_new_label: bool,
        description: String,
    ) -> Self {
        Self {
            message_id,
            label,
            created_new_label,
            description,
        }
    }

    /// Create a result for successful labeling with existing label
    pub fn labeled_existing(message_id: String, label: String) -> Self {
        Self::new(
            message_id,
            label.clone(),
            false,
            format!("Applied existing label '{label}'"),
        )
    }

    /// Create a result for successful labeling with new label
    pub fn labeled_new(message_id: String, label: String) -> Self {
        Self::new(
            message_id,
            label.clone(),
            true,
            format!("Created and applied new label '{label}'"),
        )
    }
}

/// Errors that can occur during labeling operations.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LabelingError {
    /// Authentication or authorization error
    #[error("Authentication error: {message}")]
    Auth { message: String },

    /// Network communication error
    #[error("Network error: {message}")]
    Network { message: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Gmail API specific error
    #[error("Gmail API error: {message}")]
    GmailApi { message: String },

    /// Label already exists error (when creating labels)
    #[error("Label already exists: {label}")]
    LabelExists { label: String },

    /// Invalid message ID
    #[error("Invalid message ID: {message_id}")]
    InvalidMessageId { message_id: String },

    /// Unknown error
    #[error("Unknown labeling error: {message}")]
    Unknown { message: String },
}

impl LabelingError {
    /// Create a new authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth { message: message.into() }
    }

    /// Create a new network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network { message: message.into() }
    }

    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }

    /// Create a new Gmail API error
    pub fn gmail_api(message: impl Into<String>) -> Self {
        Self::GmailApi { message: message.into() }
    }

    /// Create a new label exists error
    pub fn label_exists(label: impl Into<String>) -> Self {
        Self::LabelExists { label: label.into() }
    }

    /// Create a new invalid message ID error
    pub fn invalid_message_id(message_id: impl Into<String>) -> Self {
        Self::InvalidMessageId { message_id: message_id.into() }
    }

    /// Create a new unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown { message: message.into() }
    }
}

// Conversion from GmailClientError to LabelingError
impl From<crate::gmail::GmailClientError> for LabelingError {
    fn from(error: crate::gmail::GmailClientError) -> Self {
        match error {
            crate::gmail::GmailClientError::Config { message } => {
                LabelingError::config(message)
            }
            crate::gmail::GmailClientError::Auth { message } => {
                LabelingError::auth(message)
            }
        }
    }
}

/// Trait for applying Gmail labels to emails based on their classification.
/// 
/// This trait provides a unified interface for labeling emails with categories
/// determined by the classifier. Different implementations can provide Gmail API
/// integration or stub implementations for testing.
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use agentic_mail_agent::action::impls::labeler::{EmailLabeler, GmailLabeler};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let labeler = GmailLabeler::from_env().await?;
///     let result = labeler.apply_label("message123", "work").await?;
///     println!("Applied label: {result.description}");
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait EmailLabeler {
    /// Apply a label to an email message.
    /// 
    /// This method applies the specified label to the given message ID.
    /// If the label doesn't exist, it will be created automatically.
    /// The operation is idempotent - applying the same label multiple times
    /// will not cause errors.
    /// 
    /// # Arguments
    /// 
    /// * `message_id` - Gmail message ID to label
    /// * `label` - Label name to apply (e.g., "AGENT_WORK", "AGENT_SPAM")
    /// 
    /// # Returns
    /// 
    /// Returns a `LabelingResult` containing details about the operation,
    /// or a `LabelingError` if labeling fails.
    async fn apply_label(&self, message_id: &str, label: &str) -> Result<LabelingResult, LabelingError>;

    /// Create a label if it doesn't exist.
    /// 
    /// This method ensures a label exists in the user's Gmail account.
    /// If the label already exists, this operation succeeds without error.
    /// 
    /// # Arguments
    /// 
    /// * `label` - Label name to create
    /// 
    /// # Returns
    /// 
    /// Returns the label ID if successful, or a `LabelingError` if creation fails.
    async fn ensure_label_exists(&self, label: &str) -> Result<String, LabelingError>;

    /// Get the Gmail label name for a classification category.
    /// 
    /// This method converts classification categories (e.g., "work", "spam")
    /// into Gmail label names (e.g., "AGENT_WORK", "AGENT_SPAM").
    /// 
    /// # Arguments
    /// 
    /// * `category` - Classification category
    /// 
    /// # Returns
    /// 
    /// Returns the Gmail label name for the category.
    fn get_label_for_category(&self, category: &str) -> String {
        format!("AGENT_{}", category.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_labeling_result_creation() {
        let result = LabelingResult::new(
            "msg123".to_string(),
            "AGENT_WORK".to_string(),
            false,
            "Applied existing label".to_string(),
        );

        assert_eq!(result.message_id, "msg123");
        assert_eq!(result.label, "AGENT_WORK");
        assert!(!result.created_new_label);
        assert_eq!(result.description, "Applied existing label");
    }

    #[test]
    fn test_labeling_result_helpers() {
        let existing = LabelingResult::labeled_existing("msg123".to_string(), "AGENT_WORK".to_string());
        assert!(!existing.created_new_label);
        assert_eq!(existing.label, "AGENT_WORK");

        let new = LabelingResult::labeled_new("msg456".to_string(), "AGENT_PERSONAL".to_string());
        assert!(new.created_new_label);
        assert_eq!(new.label, "AGENT_PERSONAL");
    }

    #[test]
    fn test_labeling_error_creation() {
        let auth_error = LabelingError::auth("Invalid credentials");
        assert!(matches!(auth_error, LabelingError::Auth { .. }));

        let network_error = LabelingError::network("Connection timeout");
        assert!(matches!(network_error, LabelingError::Network { .. }));

        let config_error = LabelingError::config("Missing config");
        assert!(matches!(config_error, LabelingError::Config { .. }));

        let api_error = LabelingError::gmail_api("API quota exceeded");
        assert!(matches!(api_error, LabelingError::GmailApi { .. }));

        let exists_error = LabelingError::label_exists("AGENT_WORK");
        assert!(matches!(exists_error, LabelingError::LabelExists { .. }));

        let invalid_id_error = LabelingError::invalid_message_id("invalid-id");
        assert!(matches!(invalid_id_error, LabelingError::InvalidMessageId { .. }));

        let unknown_error = LabelingError::unknown("Unexpected error");
        assert!(matches!(unknown_error, LabelingError::Unknown { .. }));
    }

    #[test]
    fn test_get_label_for_category() {
        // We'll test this with a concrete implementation in the stub tests
        // Here we just test the default implementation concept
    }
}
