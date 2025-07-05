//! Action execution module for applying labels and archiving emails.
//!
//! This module provides the core functionality for executing actions on emails
//! based on their classification results. It handles:
//! - Applying Gmail labels based on classification categories
//! - Archiving all emails except ActionRequired (removing INBOX label)
//! - Audit logging of all actions taken

use async_trait::async_trait;

use crate::core::email::Email;
use crate::classifier::Classification;
use crate::action::impls::labeler::{EmailLabeler, LabelingError};
use crate::action::impls::archiver::{EmailArchiver, ArchivingError};

/// Result of action execution on an email.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionExecutionResult {
    /// Gmail message ID that was processed
    pub message_id: String,
    /// Actions that were successfully taken
    pub actions_taken: Vec<String>,
    /// Whether the email was archived (removed from inbox)
    pub archived: bool,
    /// Label that was applied
    pub label_applied: String,
    /// Summary description of all actions
    pub summary: String,
}

impl ActionExecutionResult {
    /// Create a new action execution result
    pub fn new(
        message_id: String,
        actions_taken: Vec<String>,
        archived: bool,
        label_applied: String,
    ) -> Self {
        let summary = if archived {
            format!("Applied label '{label_applied}' and archived email")
        } else {
            format!("Applied label '{label_applied}' and kept in inbox (ActionRequired)")
        };
        
        Self {
            message_id,
            actions_taken,
            archived,
            label_applied,
            summary,
        }
    }
}

/// Errors that can occur during action execution.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ActionExecutionError {
    /// Labeling operation failed
    #[error("Labeling failed: {message}")]
    LabelingFailed { message: String },
    
    /// Archive operation failed
    #[error("Archive failed: {message}")]
    ArchiveFailed { message: String },
    
    /// Invalid classification category
    #[error("Invalid category: {category}")]
    InvalidCategory { category: String },
    
    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    /// Unknown error
    #[error("Unknown action execution error: {message}")]
    Unknown { message: String },
}

impl ActionExecutionError {
    /// Create a new labeling failed error
    pub fn labeling_failed(message: impl Into<String>) -> Self {
        Self::LabelingFailed { message: message.into() }
    }
    
    /// Create a new archive failed error
    pub fn archive_failed(message: impl Into<String>) -> Self {
        Self::ArchiveFailed { message: message.into() }
    }
    
    /// Create a new invalid category error
    pub fn invalid_category(category: impl Into<String>) -> Self {
        Self::InvalidCategory { category: category.into() }
    }
    
    /// Create a new config error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    /// Create a new unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown { message: message.into() }
    }
}

// Convert LabelingError to ActionExecutionError
impl From<LabelingError> for ActionExecutionError {
    fn from(error: LabelingError) -> Self {
        Self::labeling_failed(error.to_string())
    }
}

// Convert ArchivingError to ActionExecutionError
impl From<ArchivingError> for ActionExecutionError {
    fn from(error: ArchivingError) -> Self {
        Self::archive_failed(error.to_string())
    }
}

/// Category-to-label mapping for the agentic mail agent.
/// 
/// Maps the 5 classification categories to their corresponding Gmail labels:
/// - ActionRequired → AGENT_ACTIONREQUIRED
/// - InterestingInfo → AGENT_INTERESTINGINFO  
/// - Reference → AGENT_REFERENCE
/// - Noise → AGENT_NOISE
/// - Spam → AGENT_SPAM
pub fn get_label_for_category(category: &str) -> String {
    format!("AGENT_{}", category.to_uppercase())
}

/// Trait for executing actions on emails based on classification results.
/// 
/// This trait provides the core functionality for the agentic mail agent:
/// 1. Apply appropriate Gmail labels based on classification category
/// 2. Archive all emails except ActionRequired (remove from inbox)
/// 3. Provide audit logging of all actions taken
#[async_trait]
pub trait ActionExecutor {
    /// Execute actions on an email based on its classification.
    /// 
    /// This method performs the core action execution logic:
    /// 1. Apply a Gmail label based on the classification category
    /// 2. Archive the email if it's not ActionRequired (remove INBOX label)
    /// 3. Return detailed results for audit purposes
    /// 
    /// # Arguments
    /// 
    /// * `email` - The email to process
    /// * `classification` - The classification result from the classifier
    /// 
    /// # Returns
    /// 
    /// Returns an `ActionExecutionResult` with details of actions taken,
    /// or an `ActionExecutionError` if execution fails.
    async fn execute_actions(
        &self, 
        email: &Email, 
        classification: &Classification
    ) -> Result<ActionExecutionResult, ActionExecutionError>;
}

/// Gmail-based implementation of ActionExecutor.
/// 
/// This implementation uses the Gmail API to apply labels and archive emails.
/// It requires OAuth2 credentials with Gmail modify permissions.
/// 
/// # Gmail API Operations
/// - Uses `modify_message` to apply labels
/// - Archives by removing the INBOX label (standard Gmail archiving)
/// - Creates labels automatically if they don't exist
/// 
/// # Required Scopes
/// - `https://www.googleapis.com/auth/gmail.modify` - For labeling and archiving
pub struct GmailActionExecutor<L: EmailLabeler, A: EmailArchiver> {
    labeler: L,
    archiver: A,
}

impl<L: EmailLabeler, A: EmailArchiver> GmailActionExecutor<L, A> {
    /// Create a new Gmail action executor with the given labeler and archiver.
    pub fn new(labeler: L, archiver: A) -> Self {
        Self { labeler, archiver }
    }
}

#[async_trait]
impl<L: EmailLabeler + Send + Sync, A: EmailArchiver + Send + Sync> ActionExecutor for GmailActionExecutor<L, A> {
    async fn execute_actions(
        &self, 
        email: &Email, 
        classification: &Classification
    ) -> Result<ActionExecutionResult, ActionExecutionError> {
        let mut actions_taken = Vec::new();
        
        // Step 1: Apply label based on classification category
        let label = get_label_for_category(&classification.category);
        
        match self.labeler.apply_label(&email.id, &label).await {
            Ok(labeling_result) => {
                actions_taken.push(format!("Applied label: {}", labeling_result.label));
                if labeling_result.created_new_label {
                    actions_taken.push(format!("Created new label: {}", labeling_result.label));
                }
            }
            Err(e) => {
                return Err(ActionExecutionError::labeling_failed(format!("Failed to apply label '{label}': {e}")));
            }
        }
        
        // Step 2: Archive email if not ActionRequired
        let should_archive = classification.category != "ActionRequired";
        
        if should_archive {
            match self.archiver.archive_email(&email.id).await {
                Ok(archive_result) => {
                    if archive_result.archived {
                        actions_taken.push("Archived email (removed from inbox)".to_string());
                    } else {
                        actions_taken.push("Email was already archived".to_string());
                    }
                }
                Err(e) => {
                    return Err(ActionExecutionError::archive_failed(format!("Failed to archive email '{}': {e}", email.id)));
                }
            }
        } else {
            actions_taken.push("Kept in inbox (ActionRequired category)".to_string());
        }
        
        Ok(ActionExecutionResult::new(
            email.id.clone(),
            actions_taken,
            should_archive,
            label,
        ))
    }
}

/// Stub implementation of ActionExecutor for testing.
/// 
/// This implementation simulates Gmail operations without making actual API calls.
/// It's useful for testing and development when you don't want to touch real Gmail data.
pub struct StubActionExecutor {
    /// Simulated actions taken (for testing verification)
    pub actions_log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl StubActionExecutor {
    /// Create a new stub action executor.
    pub fn new() -> Self {
        Self {
            actions_log: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    
    /// Get all actions that have been logged (for testing).
    pub fn get_actions_log(&self) -> Vec<String> {
        self.actions_log.lock().unwrap().clone()
    }
    
    /// Clear the actions log.
    pub fn clear_log(&self) {
        self.actions_log.lock().unwrap().clear();
    }
}

impl Default for StubActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for StubActionExecutor {
    async fn execute_actions(
        &self, 
        email: &Email, 
        classification: &Classification
    ) -> Result<ActionExecutionResult, ActionExecutionError> {
        let mut actions_taken = Vec::new();
        
        // Step 1: Apply label based on classification category
        let label = get_label_for_category(&classification.category);
        actions_taken.push(format!("Applied label: {label}"));
        
        // Log action for testing verification
        {
            let mut log = self.actions_log.lock().unwrap();
            log.push(format!("Label applied: {} -> {label}", email.id));
        }
        
        // Step 2: Archive email if not ActionRequired
        let should_archive = classification.category != "ActionRequired";
        
        if should_archive {
            actions_taken.push("Archived email (removed from inbox)".to_string());
            
            // Log archive action
            let mut log = self.actions_log.lock().unwrap();
            log.push(format!("Archived: {}", email.id));
        } else {
            actions_taken.push("Kept in inbox (ActionRequired category)".to_string());
            
            // Log inbox retention
            let mut log = self.actions_log.lock().unwrap();
            log.push(format!("Kept in inbox: {}", email.id));
        }
        
        Ok(ActionExecutionResult::new(
            email.id.clone(),
            actions_taken,
            should_archive,
            label,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::impls::labeler::StubLabeler;

    #[test]
    fn test_get_label_for_category() {
        assert_eq!(get_label_for_category("ActionRequired"), "AGENT_ACTIONREQUIRED");
        assert_eq!(get_label_for_category("InterestingInfo"), "AGENT_INTERESTINGINFO");
        assert_eq!(get_label_for_category("Reference"), "AGENT_REFERENCE");
        assert_eq!(get_label_for_category("Noise"), "AGENT_NOISE");
        assert_eq!(get_label_for_category("Spam"), "AGENT_SPAM");
    }

    #[test]
    fn test_action_execution_result_creation() {
        let result = ActionExecutionResult::new(
            "msg123".to_string(),
            vec!["Applied label: AGENT_WORK".to_string()],
            true,
            "AGENT_WORK".to_string(),
        );

        assert_eq!(result.message_id, "msg123");
        assert!(result.archived);
        assert_eq!(result.label_applied, "AGENT_WORK");
        assert!(result.summary.contains("Applied label"));
        assert!(result.summary.contains("archived"));
    }

    #[test]
    fn test_action_execution_error_creation() {
        let error = ActionExecutionError::labeling_failed("Label not found");
        assert!(matches!(error, ActionExecutionError::LabelingFailed { .. }));

        let error = ActionExecutionError::archive_failed("API error");
        assert!(matches!(error, ActionExecutionError::ArchiveFailed { .. }));

        let error = ActionExecutionError::invalid_category("unknown");
        assert!(matches!(error, ActionExecutionError::InvalidCategory { .. }));
    }

    #[tokio::test]
    async fn test_stub_action_executor() {
        let executor = StubActionExecutor::new();
        let email = Email::new("msg123".to_string(), Some("Test".to_string()), None);
        let classification = Classification::with_category("ActionRequired".to_string());

        let result = executor.execute_actions(&email, &classification).await.unwrap();

        assert_eq!(result.message_id, "msg123");
        assert!(!result.archived); // ActionRequired should not be archived
        assert_eq!(result.label_applied, "AGENT_ACTIONREQUIRED");
        assert!(result.actions_taken.iter().any(|a| a.contains("Kept in inbox")));

        // Check actions log
        let log = executor.get_actions_log();
        assert!(log.iter().any(|l| l.contains("Label applied")));
        assert!(log.iter().any(|l| l.contains("Kept in inbox")));
    }

    #[tokio::test]
    async fn test_gmail_action_executor_with_stub_labeler() {
        let labeler = StubLabeler::new();
        let archiver = crate::action::impls::archiver::StubArchiver::new();
        let executor = GmailActionExecutor::new(labeler, archiver);
        
        let email = Email::new("msg456".to_string(), Some("Newsletter".to_string()), None);
        let classification = Classification::with_category("Noise".to_string());

        let result = executor.execute_actions(&email, &classification).await.unwrap();

        assert_eq!(result.message_id, "msg456");
        assert!(result.archived); // Noise should be archived
        assert_eq!(result.label_applied, "AGENT_NOISE");
        assert!(result.actions_taken.iter().any(|a| a.contains("Archived")));
    }

    #[tokio::test]
    async fn test_all_categories_archiving_behavior() {
        let executor = StubActionExecutor::new();
        
        let test_cases = vec![
            ("ActionRequired", false), // Should NOT be archived
            ("InterestingInfo", true),  // Should be archived
            ("Reference", true),        // Should be archived
            ("Noise", true),           // Should be archived
            ("Spam", true),            // Should be archived
        ];

        for (category, should_archive) in test_cases {
            let email = Email::new(format!("msg_{category}"), Some("Test".to_string()), None);
            let classification = Classification::with_category(category.to_string());

            let result = executor.execute_actions(&email, &classification).await.unwrap();

            assert_eq!(result.archived, should_archive, 
                "Category '{}' archiving behavior incorrect", category);
            
            let expected_label = format!("AGENT_{}", category.to_uppercase());
            assert_eq!(result.label_applied, expected_label);
        }
    }
}