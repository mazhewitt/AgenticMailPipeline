//! Stub implementation of EmailLabeler for testing and development.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{EmailLabeler, LabelingError, LabelingResult};

/// Stub implementation of EmailLabeler for testing and development.
///
/// This implementation simulates Gmail labeling operations without actually
/// connecting to the Gmail API. It's useful for testing and development
/// when you don't want to make real API calls.
///
/// # Features
/// - Simulates successful labeling operations
/// - Tracks applied labels in memory
/// - Can be configured to simulate errors
/// - Deterministic behavior for testing
///
/// # Examples
///
/// ```rust
/// use agentic_mail_agent::action::impls::labeler::{EmailLabeler, StubLabeler};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let labeler = StubLabeler::new();
///     let result = labeler.apply_label("test-message-123", "work").await?;
///     println!("Applied label: {}", result.label);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct StubLabeler {
    /// Internal state tracking applied labels
    /// Map from message_id -> Vec<label_name>
    applied_labels: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Known labels that exist
    existing_labels: Arc<Mutex<Vec<String>>>,
    /// Error to simulate (if any)
    simulate_error: Option<LabelingError>,
}

impl StubLabeler {
    /// Create a new stub labeler with default behavior.
    pub fn new() -> Self {
        Self {
            applied_labels: Arc::new(Mutex::new(HashMap::new())),
            existing_labels: Arc::new(Mutex::new(Vec::new())),
            simulate_error: None,
        }
    }

    /// Create a stub labeler that simulates the specified error.
    pub fn with_error(error: LabelingError) -> Self {
        Self {
            applied_labels: Arc::new(Mutex::new(HashMap::new())),
            existing_labels: Arc::new(Mutex::new(Vec::new())),
            simulate_error: Some(error),
        }
    }

    /// Create a stub labeler with pre-existing labels.
    pub fn with_existing_labels(labels: Vec<String>) -> Self {
        Self {
            applied_labels: Arc::new(Mutex::new(HashMap::new())),
            existing_labels: Arc::new(Mutex::new(labels)),
            simulate_error: None,
        }
    }

    /// Get all labels applied to a specific message.
    pub fn get_applied_labels(&self, message_id: &str) -> Vec<String> {
        let applied = self.applied_labels.lock().unwrap();
        applied.get(message_id).cloned().unwrap_or_default()
    }

    /// Get all existing labels.
    pub fn get_existing_labels(&self) -> Vec<String> {
        let existing = self.existing_labels.lock().unwrap();
        existing.clone()
    }

    /// Check if a message has a specific label.
    pub fn message_has_label(&self, message_id: &str, label: &str) -> bool {
        let applied = self.applied_labels.lock().unwrap();
        applied
            .get(message_id)
            .map(|labels| labels.contains(&label.to_string()))
            .unwrap_or(false)
    }

    /// Reset all internal state (for testing).
    pub fn reset(&self) {
        let mut applied = self.applied_labels.lock().unwrap();
        let mut existing = self.existing_labels.lock().unwrap();
        applied.clear();
        existing.clear();
    }

    /// Add a label to the internal tracking for a message.
    fn add_label_to_message(&self, message_id: &str, label: &str) {
        let mut applied = self.applied_labels.lock().unwrap();
        applied
            .entry(message_id.to_string())
            .or_default()
            .push(label.to_string());
    }

    /// Check if a label exists in the internal tracking.
    fn label_exists(&self, label: &str) -> bool {
        let existing = self.existing_labels.lock().unwrap();
        existing.contains(&label.to_string())
    }

    /// Add a label to the existing labels list.
    fn add_existing_label(&self, label: &str) {
        let mut existing = self.existing_labels.lock().unwrap();
        if !existing.contains(&label.to_string()) {
            existing.push(label.to_string());
        }
    }
}

impl Default for StubLabeler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailLabeler for StubLabeler {
    async fn apply_label(
        &self,
        message_id: &str,
        label: &str,
    ) -> Result<LabelingResult, LabelingError> {
        // Simulate error if configured
        if let Some(error) = &self.simulate_error {
            return Err(error.clone());
        }

        // Validate inputs
        if message_id.is_empty() {
            return Err(LabelingError::invalid_message_id(
                "Message ID cannot be empty",
            ));
        }

        if label.is_empty() {
            return Err(LabelingError::config("Label name cannot be empty"));
        }

        // Check if label already applied (idempotent)
        if self.message_has_label(message_id, label) {
            return Ok(LabelingResult::labeled_existing(
                message_id.to_string(),
                label.to_string(),
            ));
        }

        // Check if label exists, create if not
        let created_new_label = !self.label_exists(label);
        if created_new_label {
            self.add_existing_label(label);
        }

        // Apply the label
        self.add_label_to_message(message_id, label);

        // Return appropriate result
        if created_new_label {
            Ok(LabelingResult::labeled_new(
                message_id.to_string(),
                label.to_string(),
            ))
        } else {
            Ok(LabelingResult::labeled_existing(
                message_id.to_string(),
                label.to_string(),
            ))
        }
    }

    async fn ensure_label_exists(&self, label: &str) -> Result<String, LabelingError> {
        // Simulate error if configured
        if let Some(error) = &self.simulate_error {
            return Err(error.clone());
        }

        if label.is_empty() {
            return Err(LabelingError::config("Label name cannot be empty"));
        }

        // Add to existing labels if not already present
        self.add_existing_label(label);

        // Return a simulated label ID
        Ok(format!("label_{}", label.to_lowercase()).replace(" ", "_"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_labeler_new() {
        let labeler = StubLabeler::new();
        assert!(labeler.get_applied_labels("test-msg").is_empty());
        assert!(labeler.get_existing_labels().is_empty());
    }

    #[tokio::test]
    async fn test_stub_labeler_apply_label_success() {
        let labeler = StubLabeler::new();
        let result = labeler.apply_label("msg123", "AGENT_WORK").await.unwrap();

        assert_eq!(result.message_id, "msg123");
        assert_eq!(result.label, "AGENT_WORK");
        assert!(result.created_new_label); // First time applying this label
        assert!(result.description.contains("Created and applied new label"));

        // Verify internal state
        assert!(labeler.message_has_label("msg123", "AGENT_WORK"));
        assert!(labeler
            .get_existing_labels()
            .contains(&"AGENT_WORK".to_string()));
    }

    #[tokio::test]
    async fn test_stub_labeler_apply_label_idempotent() {
        let labeler = StubLabeler::new();

        // Apply label first time
        let result1 = labeler.apply_label("msg123", "AGENT_WORK").await.unwrap();
        assert!(result1.created_new_label);

        // Apply same label again - should be idempotent
        let result2 = labeler.apply_label("msg123", "AGENT_WORK").await.unwrap();
        assert!(!result2.created_new_label);
        assert!(result2.description.contains("Applied existing label"));

        // Verify only one instance of the label in applied labels
        let applied = labeler.get_applied_labels("msg123");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], "AGENT_WORK");
    }

    #[tokio::test]
    async fn test_stub_labeler_apply_multiple_labels() {
        let labeler = StubLabeler::new();

        // Apply different labels to the same message
        labeler.apply_label("msg123", "AGENT_WORK").await.unwrap();
        labeler.apply_label("msg123", "AGENT_URGENT").await.unwrap();

        let applied = labeler.get_applied_labels("msg123");
        assert_eq!(applied.len(), 2);
        assert!(applied.contains(&"AGENT_WORK".to_string()));
        assert!(applied.contains(&"AGENT_URGENT".to_string()));
    }

    #[tokio::test]
    async fn test_stub_labeler_with_existing_labels() {
        let existing = vec!["AGENT_WORK".to_string(), "AGENT_PERSONAL".to_string()];
        let labeler = StubLabeler::with_existing_labels(existing);

        assert_eq!(labeler.get_existing_labels().len(), 2);
        assert!(labeler
            .get_existing_labels()
            .contains(&"AGENT_WORK".to_string()));

        // Apply existing label
        let result = labeler.apply_label("msg123", "AGENT_WORK").await.unwrap();
        assert!(!result.created_new_label);
    }

    #[tokio::test]
    async fn test_stub_labeler_with_error() {
        let error = LabelingError::network("Simulated network error");
        let labeler = StubLabeler::with_error(error.clone());

        let result = labeler.apply_label("msg123", "AGENT_WORK").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), error);
    }

    #[tokio::test]
    async fn test_stub_labeler_ensure_label_exists() {
        let labeler = StubLabeler::new();

        let label_id = labeler.ensure_label_exists("AGENT_WORK").await.unwrap();
        assert_eq!(label_id, "label_agent_work");
        assert!(labeler
            .get_existing_labels()
            .contains(&"AGENT_WORK".to_string()));
    }

    #[tokio::test]
    async fn test_stub_labeler_invalid_inputs() {
        let labeler = StubLabeler::new();

        // Empty message ID
        let result = labeler.apply_label("", "AGENT_WORK").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LabelingError::InvalidMessageId { .. }
        ));

        // Empty label
        let result = labeler.apply_label("msg123", "").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LabelingError::Config { .. }));

        // Empty label in ensure_label_exists
        let result = labeler.ensure_label_exists("").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LabelingError::Config { .. }));
    }

    #[tokio::test]
    async fn test_stub_labeler_reset() {
        let labeler = StubLabeler::new();

        // Add some data
        labeler.apply_label("msg123", "AGENT_WORK").await.unwrap();
        assert!(!labeler.get_applied_labels("msg123").is_empty());
        assert!(!labeler.get_existing_labels().is_empty());

        // Reset
        labeler.reset();
        assert!(labeler.get_applied_labels("msg123").is_empty());
        assert!(labeler.get_existing_labels().is_empty());
    }

    #[tokio::test]
    async fn test_stub_labeler_get_label_for_category() {
        let labeler = StubLabeler::new();

        assert_eq!(labeler.get_label_for_category("work"), "Agentic/Work");
        assert_eq!(
            labeler.get_label_for_category("personal"),
            "Agentic/Personal"
        );
        assert_eq!(labeler.get_label_for_category("spam"), "Agentic/Spam");
        assert_eq!(
            labeler.get_label_for_category("promotional"),
            "Agentic/Promotional"
        );
        assert_eq!(
            labeler.get_label_for_category("newsletter"),
            "Agentic/Newsletter"
        );
        assert_eq!(labeler.get_label_for_category("urgent"), "Agentic/Urgent");
    }
}
