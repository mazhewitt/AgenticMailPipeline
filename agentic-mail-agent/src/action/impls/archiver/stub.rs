//! Stub implementation of EmailArchiver for testing and development.

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use super::{ArchiveResult, ArchivingError, EmailArchiver};

/// Stub implementation of EmailArchiver that simulates archiving operations
/// without making actual Gmail API calls.
///
/// This implementation is useful for:
/// - Testing and development
/// - CI/CD pipelines where you don't want to touch real Gmail data
/// - Demonstrating archiving functionality
///
/// # Features
/// - Simulates archiving by tracking message IDs in memory
/// - Configurable to return errors for testing error handling
/// - Thread-safe for concurrent operations
/// - Provides audit logging of all operations
///
/// # Examples
///
/// ```rust
/// use agentic_mail_agent::action::impls::archiver::{EmailArchiver, StubArchiver};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let archiver = StubArchiver::new();
///     
///     // Archive an email
///     let result = archiver.archive_email("msg123").await?;
///     assert!(result.archived);
///     
///     // Check if it's archived
///     let is_archived = archiver.is_archived("msg123").await?;
///     assert!(is_archived);
///     
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct StubArchiver {
    /// Set of message IDs that have been archived
    archived_messages: Arc<Mutex<HashSet<String>>>,
    /// Configuration for error simulation
    error_config: Arc<Mutex<StubArchiverConfig>>,
    /// Audit log of all operations (for testing)
    audit_log: Arc<Mutex<Vec<String>>>,
}

/// Configuration for the stub archiver's behavior.
#[derive(Debug, Clone)]
pub struct StubArchiverConfig {
    /// Should archive operations fail with an error?
    pub archive_should_fail: bool,
    /// Should is_archived operations fail with an error?
    pub is_archived_should_fail: bool,
    /// Error message to return when operations fail
    pub error_message: String,
}

impl Default for StubArchiverConfig {
    fn default() -> Self {
        Self {
            archive_should_fail: false,
            is_archived_should_fail: false,
            error_message: "Simulated error".to_string(),
        }
    }
}

impl StubArchiver {
    /// Create a new stub archiver with default configuration.
    pub fn new() -> Self {
        Self {
            archived_messages: Arc::new(Mutex::new(HashSet::new())),
            error_config: Arc::new(Mutex::new(StubArchiverConfig::default())),
            audit_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a new stub archiver with some messages pre-archived.
    pub fn with_archived_messages(archived_messages: Vec<String>) -> Self {
        let mut message_set = HashSet::new();
        for msg_id in archived_messages {
            message_set.insert(msg_id);
        }

        Self {
            archived_messages: Arc::new(Mutex::new(message_set)),
            error_config: Arc::new(Mutex::new(StubArchiverConfig::default())),
            audit_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Configure the archiver to return errors for testing.
    pub fn with_error_config(self, config: StubArchiverConfig) -> Self {
        *self.error_config.lock().unwrap() = config;
        self
    }

    /// Get the audit log of all operations (for testing).
    pub fn get_audit_log(&self) -> Vec<String> {
        self.audit_log.lock().unwrap().clone()
    }

    /// Clear the audit log.
    pub fn clear_audit_log(&self) {
        self.audit_log.lock().unwrap().clear();
    }

    /// Reset the archiver to its initial state.
    pub fn reset(&self) {
        self.archived_messages.lock().unwrap().clear();
        self.audit_log.lock().unwrap().clear();
        *self.error_config.lock().unwrap() = StubArchiverConfig::default();
    }

    /// Get the set of archived message IDs (for testing).
    pub fn get_archived_messages(&self) -> HashSet<String> {
        self.archived_messages.lock().unwrap().clone()
    }

    /// Manually mark a message as archived (for testing setup).
    pub fn mark_as_archived(&self, message_id: &str) {
        self.archived_messages
            .lock()
            .unwrap()
            .insert(message_id.to_string());
    }

    /// Log an operation to the audit log.
    fn log_operation(&self, operation: String) {
        self.audit_log.lock().unwrap().push(operation);
    }
}

impl Default for StubArchiver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailArchiver for StubArchiver {
    async fn archive_email(&self, message_id: &str) -> Result<ArchiveResult, ArchivingError> {
        // Validate input
        if message_id.is_empty() {
            return Err(ArchivingError::invalid_message_id(
                "Message ID cannot be empty",
            ));
        }

        // Check if we should simulate an error
        let should_fail = self.error_config.lock().unwrap().archive_should_fail;
        if should_fail {
            let error_msg = self.error_config.lock().unwrap().error_message.clone();
            self.log_operation(format!("Archive failed: {message_id} -> {error_msg}"));
            return Err(ArchivingError::gmail_api(error_msg));
        }

        // Check if already archived
        let mut archived_messages = self.archived_messages.lock().unwrap();
        if archived_messages.contains(message_id) {
            let result = ArchiveResult::already_archived(message_id.to_string());
            self.log_operation(format!("Archive skipped: {message_id} (already archived)"));
            return Ok(result);
        }

        // Simulate archiving by adding to the set
        archived_messages.insert(message_id.to_string());

        let result = ArchiveResult::archived(message_id.to_string());
        self.log_operation(format!("Archive success: {message_id}"));

        Ok(result)
    }

    async fn is_archived(&self, message_id: &str) -> Result<bool, ArchivingError> {
        // Validate input
        if message_id.is_empty() {
            return Err(ArchivingError::invalid_message_id(
                "Message ID cannot be empty",
            ));
        }

        // Check if we should simulate an error
        let should_fail = self.error_config.lock().unwrap().is_archived_should_fail;
        if should_fail {
            let error_msg = self.error_config.lock().unwrap().error_message.clone();
            self.log_operation(format!(
                "Is archived check failed: {message_id} -> {error_msg}"
            ));
            return Err(ArchivingError::gmail_api(error_msg));
        }

        let archived_messages = self.archived_messages.lock().unwrap();
        let is_archived = archived_messages.contains(message_id);

        self.log_operation(format!("Is archived check: {message_id} -> {is_archived}"));

        Ok(is_archived)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_archiver_new() {
        let archiver = StubArchiver::new();
        let archived_messages = archiver.get_archived_messages();
        assert!(archived_messages.is_empty());
    }

    #[tokio::test]
    async fn test_stub_archiver_with_archived_messages() {
        let pre_archived = vec!["msg1".to_string(), "msg2".to_string()];
        let archiver = StubArchiver::with_archived_messages(pre_archived.clone());

        let archived_messages = archiver.get_archived_messages();
        assert_eq!(archived_messages.len(), 2);
        assert!(archived_messages.contains("msg1"));
        assert!(archived_messages.contains("msg2"));
    }

    #[tokio::test]
    async fn test_archive_email_success() {
        let archiver = StubArchiver::new();

        let result = archiver.archive_email("msg123").await.unwrap();
        assert!(result.archived);
        assert_eq!(result.message_id, "msg123");
        assert!(result.description.contains("archived successfully"));

        // Check that it's now in the archived set
        let archived_messages = archiver.get_archived_messages();
        assert!(archived_messages.contains("msg123"));
    }

    #[tokio::test]
    async fn test_archive_email_already_archived() {
        let archiver = StubArchiver::with_archived_messages(vec!["msg123".to_string()]);

        let result = archiver.archive_email("msg123").await.unwrap();
        assert!(!result.archived);
        assert_eq!(result.message_id, "msg123");
        assert!(result.description.contains("already archived"));
    }

    #[tokio::test]
    async fn test_archive_email_empty_message_id() {
        let archiver = StubArchiver::new();

        let result = archiver.archive_email("").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArchivingError::InvalidMessageId { .. }
        ));
    }

    #[tokio::test]
    async fn test_archive_email_with_error_config() {
        let config = StubArchiverConfig {
            archive_should_fail: true,
            is_archived_should_fail: false,
            error_message: "Test error".to_string(),
        };

        let archiver = StubArchiver::new().with_error_config(config);

        let result = archiver.archive_email("msg123").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArchivingError::GmailApi { .. }
        ));
    }

    #[tokio::test]
    async fn test_is_archived() {
        let archiver = StubArchiver::with_archived_messages(vec!["msg123".to_string()]);

        let is_archived = archiver.is_archived("msg123").await.unwrap();
        assert!(is_archived);

        let is_archived = archiver.is_archived("msg456").await.unwrap();
        assert!(!is_archived);
    }

    #[tokio::test]
    async fn test_is_archived_empty_message_id() {
        let archiver = StubArchiver::new();

        let result = archiver.is_archived("").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArchivingError::InvalidMessageId { .. }
        ));
    }

    #[tokio::test]
    async fn test_is_archived_with_error_config() {
        let config = StubArchiverConfig {
            archive_should_fail: false,
            is_archived_should_fail: true,
            error_message: "Test error".to_string(),
        };

        let archiver = StubArchiver::new().with_error_config(config);

        let result = archiver.is_archived("msg123").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArchivingError::GmailApi { .. }
        ));
    }

    #[tokio::test]
    async fn test_audit_log() {
        let archiver = StubArchiver::new();

        let _ = archiver.archive_email("msg123").await.unwrap();
        let _ = archiver.is_archived("msg123").await.unwrap();

        let log = archiver.get_audit_log();
        assert_eq!(log.len(), 2);
        assert!(log[0].contains("Archive success"));
        assert!(log[1].contains("Is archived check"));
    }

    #[tokio::test]
    async fn test_reset() {
        let archiver = StubArchiver::with_archived_messages(vec!["msg123".to_string()]);

        let _ = archiver.archive_email("msg456").await.unwrap();
        assert_eq!(archiver.get_archived_messages().len(), 2);
        assert!(!archiver.get_audit_log().is_empty());

        archiver.reset();

        assert!(archiver.get_archived_messages().is_empty());
        assert!(archiver.get_audit_log().is_empty());
    }

    #[tokio::test]
    async fn test_mark_as_archived() {
        let archiver = StubArchiver::new();

        archiver.mark_as_archived("msg123");

        let is_archived = archiver.is_archived("msg123").await.unwrap();
        assert!(is_archived);

        let archived_messages = archiver.get_archived_messages();
        assert!(archived_messages.contains("msg123"));
    }
}
