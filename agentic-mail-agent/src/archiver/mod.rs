//! Email archiving functionality for the agentic mail agent.
//!
//! This module provides traits and implementations for archiving emails
//! by removing them from the inbox (removing the INBOX label in Gmail).
//!
//! # Implementations
//! - `GmailArchiver` - Production Gmail API implementation
//! - `StubArchiver` - Test/mock implementation

use async_trait::async_trait;

pub mod gmail;
pub mod stub;

pub use gmail::GmailArchiver;
pub use stub::StubArchiver;

/// Result of an archiving operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveResult {
    /// Gmail message ID that was archived
    pub message_id: String,
    /// Whether the email was successfully archived
    pub archived: bool,
    /// Human-readable description of the result
    pub description: String,
}

impl ArchiveResult {
    /// Create a new successful archive result.
    pub fn archived(message_id: String) -> Self {
        Self {
            message_id: message_id.clone(),
            archived: true,
            description: format!("Email {} archived successfully", message_id),
        }
    }

    /// Create a result for an email that was already archived.
    pub fn already_archived(message_id: String) -> Self {
        Self {
            message_id: message_id.clone(),
            archived: false,
            description: format!("Email {} was already archived", message_id),
        }
    }

    /// Create a result for an email that was not archived (e.g., ActionRequired).
    pub fn not_archived(message_id: String, reason: String) -> Self {
        Self {
            message_id: message_id.clone(),
            archived: false,
            description: format!("Email {} not archived: {}", message_id, reason),
        }
    }
}

/// Errors that can occur during archiving operations.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ArchivingError {
    /// Gmail API error
    #[error("Gmail API error: {message}")]
    GmailApi { message: String },
    
    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    /// Invalid message ID
    #[error("Invalid message ID: {message}")]
    InvalidMessageId { message: String },
    
    /// Network or communication error
    #[error("Network error: {message}")]
    Network { message: String },
    
    /// Authentication error
    #[error("Authentication error: {message}")]
    Auth { message: String },
    
    /// Unknown error
    #[error("Unknown archiving error: {message}")]
    Unknown { message: String },
}

impl ArchivingError {
    /// Create a new Gmail API error.
    pub fn gmail_api(message: impl Into<String>) -> Self {
        Self::GmailApi { message: message.into() }
    }
    
    /// Create a new configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    /// Create a new invalid message ID error.
    pub fn invalid_message_id(message: impl Into<String>) -> Self {
        Self::InvalidMessageId { message: message.into() }
    }
    
    /// Create a new network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network { message: message.into() }
    }
    
    /// Create a new authentication error.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth { message: message.into() }
    }
    
    /// Create a new unknown error.
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown { message: message.into() }
    }
}

/// Trait for archiving emails by removing them from the inbox.
/// 
/// In Gmail, archiving means removing the INBOX label from a message.
/// The message remains accessible in "All Mail" but is no longer in the inbox.
/// 
/// # Implementation Notes
/// 
/// Implementations should:
/// - Be idempotent (archiving an already archived email should succeed)
/// - Handle rate limiting and network errors gracefully
/// - Provide clear error messages for troubleshooting
/// - Support batch operations where possible for efficiency
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use agentic_mail_agent::archiver::{EmailArchiver, StubArchiver};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let archiver = StubArchiver::new();
///     let result = archiver.archive_email("message123").await?;
///     println!("Archive result: {}", result.description);
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait EmailArchiver {
    /// Archive an email by removing it from the inbox.
    /// 
    /// In Gmail, this removes the INBOX label from the message.
    /// The message remains accessible in "All Mail" but is no longer in the inbox.
    /// 
    /// # Arguments
    /// 
    /// * `message_id` - The Gmail message ID to archive
    /// 
    /// # Returns
    /// 
    /// Returns an `ArchiveResult` with details of the operation,
    /// or an `ArchivingError` if the operation fails.
    /// 
    /// # Examples
    /// 
    /// ```rust,no_run
    /// use agentic_mail_agent::archiver::{EmailArchiver, StubArchiver};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let archiver = StubArchiver::new();
    ///     let result = archiver.archive_email("msg123").await?;
    ///     assert!(result.archived);
    ///     Ok(())
    /// }
    /// ```
    async fn archive_email(&self, message_id: &str) -> Result<ArchiveResult, ArchivingError>;
    
    /// Check if an email is currently archived (not in inbox).
    /// 
    /// # Arguments
    /// 
    /// * `message_id` - The Gmail message ID to check
    /// 
    /// # Returns
    /// 
    /// Returns `true` if the email is archived (not in inbox), `false` if it's in the inbox.
    async fn is_archived(&self, message_id: &str) -> Result<bool, ArchivingError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_result_creation() {
        let result = ArchiveResult::archived("msg123".to_string());
        assert_eq!(result.message_id, "msg123");
        assert!(result.archived);
        assert!(result.description.contains("archived successfully"));

        let result = ArchiveResult::already_archived("msg456".to_string());
        assert_eq!(result.message_id, "msg456");
        assert!(!result.archived);
        assert!(result.description.contains("already archived"));

        let result = ArchiveResult::not_archived("msg789".to_string(), "ActionRequired".to_string());
        assert_eq!(result.message_id, "msg789");
        assert!(!result.archived);
        assert!(result.description.contains("not archived"));
    }

    #[test]
    fn test_archiving_error_creation() {
        let error = ArchivingError::gmail_api("API quota exceeded");
        assert!(matches!(error, ArchivingError::GmailApi { .. }));

        let error = ArchivingError::config("Missing credentials");
        assert!(matches!(error, ArchivingError::Config { .. }));

        let error = ArchivingError::invalid_message_id("Empty message ID");
        assert!(matches!(error, ArchivingError::InvalidMessageId { .. }));

        let error = ArchivingError::network("Connection timeout");
        assert!(matches!(error, ArchivingError::Network { .. }));

        let error = ArchivingError::auth("Invalid token");
        assert!(matches!(error, ArchivingError::Auth { .. }));

        let error = ArchivingError::unknown("Unexpected error");
        assert!(matches!(error, ArchivingError::Unknown { .. }));
    }
}