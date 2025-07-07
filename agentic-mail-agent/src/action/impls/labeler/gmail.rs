//! Gmail API implementation of EmailLabeler.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{EmailLabeler, LabelingError, LabelingResult};
#[cfg(test)]
use crate::gmail::api::GmailApiResult;
use crate::gmail::api::{GmailApi, GmailApiError};
use crate::gmail::{GmailAuthConfig, GmailClient};
use google_gmail1::api::Label;

/// Convert GmailApiError to LabelingError
fn gmail_api_error_to_labeling_error(error: GmailApiError) -> LabelingError {
    match error {
        GmailApiError::Auth { message } => {
            LabelingError::gmail_api(format!("Authentication error: {message}"))
        }
        GmailApiError::Network { message } => {
            LabelingError::gmail_api(format!("Network error: {message}"))
        }
        GmailApiError::RateLimit { message } => {
            LabelingError::gmail_api(format!("Rate limit error: {message}"))
        }
        GmailApiError::InvalidRequest { message } => {
            LabelingError::gmail_api(format!("Invalid request: {message}"))
        }
        GmailApiError::NotFound { message } => {
            LabelingError::gmail_api(format!("Not found: {message}"))
        }
        GmailApiError::Api { message } => LabelingError::gmail_api(message),
    }
}

/// Gmail API implementation of EmailLabeler.
///
/// This implementation uses the Gmail API to apply labels to emails.
/// It requires OAuth2 credentials with Gmail modify permissions.
///
/// # Environment Variables
/// - `GMAIL_CLIENT_SECRET_JSON`: Path to OAuth2 client secret JSON file
/// - `GMAIL_TOKEN_JSON`: Path to OAuth2 token JSON file
///
/// # Required Scopes
/// - `https://www.googleapis.com/auth/gmail.modify` - Required for labeling operations
/// - `https://www.googleapis.com/auth/gmail.readonly` - Required for reading message info
///
/// # Features
/// - Automatic label creation if labels don't exist
/// - Idempotent labeling (won't re-apply existing labels)
/// - Label caching for improved performance
/// - Proper error handling and retries
///
/// # Examples
///
/// ```rust,no_run
/// use agentic_mail_agent::action::impls::labeler::{EmailLabeler, GmailLabeler};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Set environment variables:
///     // export GMAIL_CLIENT_SECRET_JSON=/path/to/client_secret.json
///     // export GMAIL_TOKEN_JSON=/path/to/token.json
///     
///     let labeler = GmailLabeler::from_env().await?;
///     let result = labeler.apply_label("message123", "work").await?;
///     println!("Applied label: {}", result.description);
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct GmailLabeler<T: GmailApi + Clone> {
    gmail_api: T,
    /// Cache of label name -> label ID mappings
    label_cache: Arc<Mutex<HashMap<String, String>>>,
}

/// Type alias for the concrete GmailLabeler using GmailClient
pub type ConcreteGmailLabeler = GmailLabeler<GmailClient>;

impl ConcreteGmailLabeler {
    /// Create a new GmailLabeler from environment variables.
    ///
    /// Reads credential paths from environment variables and initializes
    /// the Gmail API client with OAuth2 authentication.
    ///
    /// # Errors
    /// Returns `LabelingError::Config` if environment variables are missing
    /// or credential files are invalid.
    pub async fn from_env() -> Result<Self, LabelingError> {
        let gmail_client = GmailClient::from_env().await?;

        Ok(Self {
            gmail_api: gmail_client,
            label_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create a new GmailLabeler with explicit credential paths.
    ///
    /// # Arguments
    /// * `client_secret_path` - Path to OAuth2 client secret JSON file
    /// * `token_path` - Path to OAuth2 token JSON file
    pub async fn new(
        client_secret_path: String,
        token_path: String,
    ) -> Result<Self, LabelingError> {
        let config = GmailAuthConfig::new(client_secret_path, token_path);
        let gmail_client = GmailClient::new(config).await?;

        Ok(Self {
            gmail_api: gmail_client,
            label_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

impl<T: GmailApi + Clone> GmailLabeler<T> {
    /// Create a new GmailLabeler with a custom GmailApi implementation.
    ///
    /// This constructor is primarily used for testing with mock implementations.
    ///
    /// # Arguments
    /// * `gmail_api` - Implementation of the GmailApi trait
    pub fn new_with_api(gmail_api: T) -> Self {
        Self {
            gmail_api,
            label_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the Gmail label ID for a label name, creating the label if it doesn't exist.
    /// Returns (label_id, was_created)
    async fn get_or_create_label_id(
        &self,
        label_name: &str,
    ) -> Result<(String, bool), LabelingError> {
        // Check cache first
        {
            let cache = self.label_cache.lock().unwrap();
            if let Some(label_id) = cache.get(label_name) {
                return Ok((label_id.clone(), false));
            }
        }

        // List existing labels to find the one we want
        let labels = self
            .gmail_api
            .list_labels()
            .await
            .map_err(gmail_api_error_to_labeling_error)?;

        // Check if label already exists
        for label in &labels {
            if let Some(name) = &label.name {
                if name == label_name {
                    if let Some(id) = &label.id {
                        // Cache the result
                        let mut cache = self.label_cache.lock().unwrap();
                        cache.insert(label_name.to_string(), id.clone());
                        return Ok((id.clone(), false));
                    }
                }
            }
        }

        // Label doesn't exist, create it
        let label_id = self.create_label(label_name).await?;
        Ok((label_id, true))
    }

    /// Create a new Gmail label.
    async fn create_label(&self, label_name: &str) -> Result<String, LabelingError> {
        let new_label = Label {
            name: Some(label_name.to_string()),
            message_list_visibility: Some("show".to_string()),
            label_list_visibility: Some("labelShow".to_string()),
            ..Default::default()
        };

        let created_label = self
            .gmail_api
            .create_label(new_label)
            .await
            .map_err(gmail_api_error_to_labeling_error)?;
        let label_id = created_label.id.ok_or_else(|| {
            LabelingError::gmail_api(format!("Created label '{label_name}' has no ID"))
        })?;

        // Cache the new label
        let mut cache = self.label_cache.lock().unwrap();
        cache.insert(label_name.to_string(), label_id.clone());

        Ok(label_id)
    }

    /// Check if a message already has a specific label.
    async fn message_has_label(
        &self,
        message_id: &str,
        label_id: &str,
    ) -> Result<bool, LabelingError> {
        let message = self
            .gmail_api
            .get_message(message_id)
            .await
            .map_err(gmail_api_error_to_labeling_error)?;
        let current_labels = message.label_ids.unwrap_or_default();

        Ok(current_labels.contains(&label_id.to_string()))
    }
}

#[async_trait]
impl<T: GmailApi + Clone> EmailLabeler for GmailLabeler<T> {
    async fn apply_label(
        &self,
        message_id: &str,
        label: &str,
    ) -> Result<LabelingResult, LabelingError> {
        // Validate inputs
        if message_id.is_empty() {
            return Err(LabelingError::invalid_message_id(
                "Message ID cannot be empty",
            ));
        }

        if label.is_empty() {
            return Err(LabelingError::config("Label name cannot be empty"));
        }

        // Get or create the label
        let (label_id, created_new_label) = self.get_or_create_label_id(label).await?;

        // Check if message already has this label (idempotent operation)
        if self.message_has_label(message_id, &label_id).await? {
            return Ok(LabelingResult::new(
                message_id.to_string(),
                label.to_string(),
                false,
                format!("Label '{label}' already applied to message"),
            ));
        }

        // Apply the label to the message
        self.gmail_api
            .modify_message_labels(message_id, Some(vec![label_id]), None)
            .await
            .map_err(gmail_api_error_to_labeling_error)?;

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
        if label.is_empty() {
            return Err(LabelingError::config("Label name cannot be empty"));
        }

        let (label_id, _) = self.get_or_create_label_id(label).await?;
        Ok(label_id)
    }
}

impl<T: GmailApi + Clone> GmailLabeler<T> {
    /// List all labels in the Gmail account
    pub async fn list_all_labels(&self) -> Result<Vec<LabelInfo>, LabelingError> {
        let labels = self
            .gmail_api
            .list_labels()
            .await
            .map_err(gmail_api_error_to_labeling_error)?;

        let label_infos = labels
            .into_iter()
            .filter_map(|label| {
                let name = label.name?;
                let id = label.id?;
                Some(LabelInfo { name, id })
            })
            .collect();

        Ok(label_infos)
    }

    /// Delete a label by its ID
    pub async fn delete_label(&self, label_id: &str) -> Result<(), LabelingError> {
        self.gmail_api
            .delete_label(label_id)
            .await
            .map_err(gmail_api_error_to_labeling_error)?;

        // Remove from cache if present
        let mut cache = self.label_cache.lock().unwrap();
        cache.retain(|_, cached_id| cached_id != label_id);

        Ok(())
    }

    /// Get all labels applied to a specific email
    pub async fn get_email_labels(
        &self,
        message_id: &str,
    ) -> Result<Vec<LabelInfo>, LabelingError> {
        let message = self
            .gmail_api
            .get_message(message_id)
            .await
            .map_err(gmail_api_error_to_labeling_error)?;
        let label_ids = message.label_ids.unwrap_or_default();

        // Get all labels to map IDs to names
        let all_labels = self.list_all_labels().await?;
        let id_to_label: std::collections::HashMap<String, String> = all_labels
            .iter()
            .map(|label| (label.id.clone(), label.name.clone()))
            .collect();

        let email_labels = label_ids
            .into_iter()
            .filter_map(|id| {
                id_to_label.get(&id).map(|name| LabelInfo {
                    id: id.clone(),
                    name: name.clone(),
                })
            })
            .collect();

        Ok(email_labels)
    }

    /// Get all emails with a specific label
    pub async fn get_emails_by_label(
        &self,
        label_name: &str,
    ) -> Result<Vec<String>, LabelingError> {
        // First get the label ID
        let (label_id, _) = self.get_or_create_label_id(label_name).await?;

        // Search for messages with this label
        let message_ids = self
            .gmail_api
            .list_messages_with_labels(&[label_id], None)
            .await
            .map_err(gmail_api_error_to_labeling_error)?;

        Ok(message_ids)
    }
}

/// Information about a Gmail label
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelInfo {
    pub name: String,
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use google_gmail1::api::{Label, Message};
    use std::collections::HashMap;
    use tokio::sync::Mutex as AsyncMutex;

    /// Mock implementation of GmailApi for testing
    #[derive(Debug, Clone)]
    pub struct MockGmailApi {
        /// Stored labels by ID
        labels: Arc<AsyncMutex<HashMap<String, Label>>>,
        /// Stored messages by ID
        messages: Arc<AsyncMutex<HashMap<String, Message>>>,
        /// Next ID to assign to new labels
        next_label_id: Arc<AsyncMutex<u32>>,
        /// Track API call counts for verification
        call_counts: Arc<AsyncMutex<HashMap<String, u32>>>,
    }

    impl MockGmailApi {
        pub fn new() -> Self {
            Self {
                labels: Arc::new(AsyncMutex::new(HashMap::new())),
                messages: Arc::new(AsyncMutex::new(HashMap::new())),
                next_label_id: Arc::new(AsyncMutex::new(1)),
                call_counts: Arc::new(AsyncMutex::new(HashMap::new())),
            }
        }

        /// Add a pre-existing label to the mock
        pub async fn add_label(&self, name: &str, id: &str) {
            let label = Label {
                id: Some(id.to_string()),
                name: Some(name.to_string()),
                message_list_visibility: Some("show".to_string()),
                label_list_visibility: Some("labelShow".to_string()),
                ..Default::default()
            };
            self.labels.lock().await.insert(id.to_string(), label);
        }

        /// Add a pre-existing message to the mock
        pub async fn add_message(&self, id: &str, label_ids: Vec<String>) {
            let message = Message {
                id: Some(id.to_string()),
                label_ids: Some(label_ids),
                ..Default::default()
            };
            self.messages.lock().await.insert(id.to_string(), message);
        }

        /// Get the number of times a specific API method was called
        pub async fn get_call_count(&self, method: &str) -> u32 {
            self.call_counts
                .lock()
                .await
                .get(method)
                .copied()
                .unwrap_or(0)
        }

        /// Increment call count for a method
        async fn increment_call_count(&self, method: &str) {
            let mut counts = self.call_counts.lock().await;
            *counts.entry(method.to_string()).or_insert(0) += 1;
        }
    }

    #[async_trait]
    impl GmailApi for MockGmailApi {
        async fn list_labels(&self) -> GmailApiResult<Vec<Label>> {
            self.increment_call_count("list_labels").await;
            let labels = self.labels.lock().await;
            Ok(labels.values().cloned().collect())
        }

        async fn create_label(&self, mut label: Label) -> GmailApiResult<Label> {
            self.increment_call_count("create_label").await;

            let mut next_id = self.next_label_id.lock().await;
            let id = format!("label_{}", *next_id);
            *next_id += 1;

            label.id = Some(id.clone());

            self.labels.lock().await.insert(id, label.clone());
            Ok(label)
        }

        async fn delete_label(&self, label_id: &str) -> GmailApiResult<()> {
            self.increment_call_count("delete_label").await;

            let mut labels = self.labels.lock().await;
            if labels.remove(label_id).is_some() {
                Ok(())
            } else {
                Err(GmailApiError::not_found(format!(
                    "Label {label_id} not found"
                )))
            }
        }

        async fn get_message(&self, message_id: &str) -> GmailApiResult<Message> {
            self.increment_call_count("get_message").await;

            let messages = self.messages.lock().await;
            messages
                .get(message_id)
                .cloned()
                .ok_or_else(|| GmailApiError::not_found(format!("Message {message_id} not found")))
        }

        async fn modify_message_labels(
            &self,
            message_id: &str,
            add_label_ids: Option<Vec<String>>,
            remove_label_ids: Option<Vec<String>>,
        ) -> GmailApiResult<Message> {
            self.increment_call_count("modify_message_labels").await;

            let mut messages = self.messages.lock().await;
            let message = messages.get_mut(message_id).ok_or_else(|| {
                GmailApiError::not_found(format!("Message {message_id} not found"))
            })?;

            let mut current_labels = message.label_ids.clone().unwrap_or_default();

            // Add labels
            if let Some(add_labels) = add_label_ids {
                for label_id in add_labels {
                    if !current_labels.contains(&label_id) {
                        current_labels.push(label_id);
                    }
                }
            }

            // Remove labels
            if let Some(remove_labels) = remove_label_ids {
                current_labels.retain(|id| !remove_labels.contains(id));
            }

            message.label_ids = Some(current_labels);
            Ok(message.clone())
        }

        async fn list_messages_with_labels(
            &self,
            label_ids: &[String],
            _max_results: Option<u32>,
        ) -> GmailApiResult<Vec<String>> {
            self.increment_call_count("list_messages_with_labels").await;

            let messages = self.messages.lock().await;
            let matching_messages: Vec<String> = messages
                .values()
                .filter(|message| {
                    if let Some(msg_labels) = &message.label_ids {
                        label_ids
                            .iter()
                            .any(|search_label| msg_labels.contains(search_label))
                    } else {
                        false
                    }
                })
                .filter_map(|message| message.id.clone())
                .collect();

            Ok(matching_messages)
        }
    }

    #[tokio::test]
    async fn test_gmail_labeler_label_creation_and_caching() {
        let mock_api = MockGmailApi::new();
        let labeler = GmailLabeler::new_with_api(mock_api);

        // First call should create the label
        let label_id = labeler.ensure_label_exists("test-label").await.unwrap();
        assert_eq!(label_id, "label_1");

        // Second call should use cache, not call API again
        let label_id2 = labeler.ensure_label_exists("test-label").await.unwrap();
        assert_eq!(label_id, label_id2);

        // Verify API was called only once to create the label
        let api_calls = labeler.gmail_api.get_call_count("create_label").await;
        assert_eq!(api_calls, 1);

        // List labels should have been called once to check if label exists
        let list_calls = labeler.gmail_api.get_call_count("list_labels").await;
        assert_eq!(list_calls, 1);
    }

    #[tokio::test]
    async fn test_gmail_labeler_apply_label_idempotent() {
        let mock_api = MockGmailApi::new();

        // Add a message without any labels
        mock_api.add_message("msg123", vec![]).await;

        let labeler = GmailLabeler::new_with_api(mock_api);

        // First application should create label and apply it
        let result1 = labeler.apply_label("msg123", "work").await.unwrap();
        assert!(result1.description.contains("new"));

        // Second application should be idempotent (no change)
        let result2 = labeler.apply_label("msg123", "work").await.unwrap();
        assert!(result2.description.contains("already"));

        // Verify the message now has the label
        let labels = labeler.get_email_labels("msg123").await.unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "work");
    }

    #[tokio::test]
    async fn test_gmail_labeler_existing_label_reuse() {
        let mock_api = MockGmailApi::new();

        // Pre-add a label to the mock API
        mock_api.add_label("existing-label", "existing_123").await;
        mock_api.add_message("msg456", vec![]).await;

        let labeler = GmailLabeler::new_with_api(mock_api);

        // Apply the existing label
        let result = labeler
            .apply_label("msg456", "existing-label")
            .await
            .unwrap();
        assert!(result.description.contains("existing"));

        // Verify no new label was created (should reuse existing)
        let create_calls = labeler.gmail_api.get_call_count("create_label").await;
        assert_eq!(create_calls, 0);
    }

    #[tokio::test]
    async fn test_gmail_labeler_error_handling() {
        let mock_api = MockGmailApi::new();
        let labeler = GmailLabeler::new_with_api(mock_api);

        // Try to apply label to non-existent message
        let result = labeler.apply_label("nonexistent", "test").await;
        assert!(result.is_err());

        // Try with empty message ID
        let result = labeler.apply_label("", "test").await;
        assert!(result.is_err());

        // Try with empty label name
        let result = labeler.apply_label("msg123", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gmail_labeler_list_emails_by_label() {
        let mock_api = MockGmailApi::new();

        // Add some test data
        mock_api.add_label("work", "work_123").await;
        mock_api
            .add_message("msg1", vec!["work_123".to_string()])
            .await;
        mock_api
            .add_message("msg2", vec!["work_123".to_string()])
            .await;
        mock_api
            .add_message("msg3", vec!["personal_456".to_string()])
            .await;

        let labeler = GmailLabeler::new_with_api(mock_api);

        // Get emails with work label
        let work_emails = labeler.get_emails_by_label("work").await.unwrap();
        assert_eq!(work_emails.len(), 2);
        assert!(work_emails.contains(&"msg1".to_string()));
        assert!(work_emails.contains(&"msg2".to_string()));
    }

    #[test]
    fn test_gmail_labeler_get_label_for_category() {
        // Test the inherited trait behavior for category mapping

        #[async_trait]
        impl EmailLabeler for TestLabeler {
            async fn apply_label(
                &self,
                _message_id: &str,
                _label: &str,
            ) -> Result<LabelingResult, LabelingError> {
                unimplemented!()
            }
            async fn ensure_label_exists(&self, _label: &str) -> Result<String, LabelingError> {
                unimplemented!()
            }
        }

        struct TestLabeler;

        let labeler = TestLabeler;
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
