//! Gmail API implementation of EmailLabeler.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{EmailLabeler, LabelingResult, LabelingError};
use crate::gmail::{GmailClient, GmailAuthConfig};
use google_gmail1::{
    api::{Label, ModifyMessageRequest},
};

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
/// use agentic_mail_agent::labeler::{EmailLabeler, GmailLabeler};
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
pub struct GmailLabeler {
    gmail_client: GmailClient,
    /// Cache of label name -> label ID mappings
    label_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl GmailLabeler {
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
            gmail_client,
            label_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create a new GmailLabeler with explicit credential paths.
    /// 
    /// # Arguments
    /// * `client_secret_path` - Path to OAuth2 client secret JSON file
    /// * `token_path` - Path to OAuth2 token JSON file
    pub async fn new(client_secret_path: String, token_path: String) -> Result<Self, LabelingError> {
        let config = GmailAuthConfig::new(client_secret_path, token_path);
        let gmail_client = GmailClient::new(config).await?;
        
        Ok(Self {
            gmail_client,
            label_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get the Gmail label ID for a label name, creating the label if it doesn't exist.
    async fn get_or_create_label_id(&self, label_name: &str) -> Result<String, LabelingError> {
        // Check cache first
        {
            let cache = self.label_cache.lock().unwrap();
            if let Some(label_id) = cache.get(label_name) {
                return Ok(label_id.clone());
            }
        }

        // List existing labels to find the one we want
        let labels_result = self.gmail_client.hub
            .users()
            .labels_list("me")
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to list labels: {}", e
            )))?;

        let labels = labels_result.1.labels.unwrap_or_default();
        
        // Check if label already exists
        for label in &labels {
            if let Some(name) = &label.name {
                if name == label_name {
                    if let Some(id) = &label.id {
                        // Cache the result
                        let mut cache = self.label_cache.lock().unwrap();
                        cache.insert(label_name.to_string(), id.clone());
                        return Ok(id.clone());
                    }
                }
            }
        }

        // Label doesn't exist, create it
        self.create_label(label_name).await
    }

    /// Create a new Gmail label.
    async fn create_label(&self, label_name: &str) -> Result<String, LabelingError> {
        let new_label = Label {
            name: Some(label_name.to_string()),
            message_list_visibility: Some("show".to_string()),
            label_list_visibility: Some("labelShow".to_string()),
            ..Default::default()
        };

        let result = self.gmail_client.hub
            .users()
            .labels_create(new_label, "me")
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to create label '{}': {}", label_name, e
            )))?;

        let created_label = result.1;
        let label_id = created_label.id.ok_or_else(|| {
            LabelingError::gmail_api(format!("Created label '{}' has no ID", label_name))
        })?;

        // Cache the new label
        let mut cache = self.label_cache.lock().unwrap();
        cache.insert(label_name.to_string(), label_id.clone());

        Ok(label_id)
    }

    /// Check if a message already has a specific label.
    async fn message_has_label(&self, message_id: &str, label_id: &str) -> Result<bool, LabelingError> {
        let message_result = self.gmail_client.hub
            .users()
            .messages_get("me", message_id)
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to get message {}: {}", message_id, e
            )))?;

        let message = message_result.1;
        let current_labels = message.label_ids.unwrap_or_default();
        
        Ok(current_labels.contains(&label_id.to_string()))
    }
}

#[async_trait]
impl EmailLabeler for GmailLabeler {
    async fn apply_label(&self, message_id: &str, label: &str) -> Result<LabelingResult, LabelingError> {
        // Validate inputs
        if message_id.is_empty() {
            return Err(LabelingError::invalid_message_id("Message ID cannot be empty"));
        }

        if label.is_empty() {
            return Err(LabelingError::config("Label name cannot be empty"));
        }

        // Get or create the label
        let label_id = self.get_or_create_label_id(label).await?;
        let created_new_label = !self.label_cache.lock().unwrap().contains_key(label);

        // Check if message already has this label (idempotent operation)
        if self.message_has_label(message_id, &label_id).await? {
            return Ok(LabelingResult::labeled_existing(
                message_id.to_string(),
                label.to_string(),
            ));
        }

        // Apply the label to the message
        let modify_request = ModifyMessageRequest {
            add_label_ids: Some(vec![label_id]),
            remove_label_ids: None,
        };

        self.gmail_client.hub
            .users()
            .messages_modify(modify_request, "me", message_id)
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to apply label '{}' to message {}: {}", label, message_id, e
            )))?;

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

        self.get_or_create_label_id(label).await
    }
}

impl GmailLabeler {
    /// List all labels in the Gmail account
    pub async fn list_all_labels(&self) -> Result<Vec<LabelInfo>, LabelingError> {
        let labels_result = self.gmail_client.hub
            .users()
            .labels_list("me")
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to list labels: {}", e
            )))?;

        let labels = labels_result.1.labels.unwrap_or_default();
        
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
        self.gmail_client.hub
            .users()
            .labels_delete("me", label_id)
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to delete label {}: {}", label_id, e
            )))?;

        // Remove from cache if present
        let mut cache = self.label_cache.lock().unwrap();
        cache.retain(|_, cached_id| cached_id != label_id);

        Ok(())
    }

    /// Get all labels applied to a specific email
    pub async fn get_email_labels(&self, message_id: &str) -> Result<Vec<LabelInfo>, LabelingError> {
        let message_result = self.gmail_client.hub
            .users()
            .messages_get("me", message_id)
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to get message {}: {}", message_id, e
            )))?;

        let message = message_result.1;
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
    pub async fn get_emails_by_label(&self, label_name: &str) -> Result<Vec<String>, LabelingError> {
        // First get the label ID
        let label_id = self.get_or_create_label_id(label_name).await?;

        // Search for messages with this label
        let messages_result = self.gmail_client.hub
            .users()
            .messages_list("me")
            .add_label_ids(&label_id)
            .doit()
            .await
            .map_err(|e| LabelingError::gmail_api(format!(
                "Failed to list messages with label '{}': {}", label_name, e
            )))?;

        let messages = messages_result.1.messages.unwrap_or_default();
        let message_ids = messages
            .into_iter()
            .filter_map(|msg| msg.id)
            .collect();

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

    #[test]
    fn test_gmail_labeler_get_label_for_category() {
        // We can't easily test the Gmail API integration without real credentials
        // so we test the inherited trait behavior instead
        
        // Test the default implementation from the trait
        #[async_trait]
        impl EmailLabeler for TestLabeler {
            async fn apply_label(&self, _message_id: &str, _label: &str) -> Result<LabelingResult, LabelingError> {
                unimplemented!()
            }
            async fn ensure_label_exists(&self, _label: &str) -> Result<String, LabelingError> {
                unimplemented!()
            }
        }

        struct TestLabeler;
        
        let labeler = TestLabeler;
        assert_eq!(labeler.get_label_for_category("work"), "AGENT_WORK");
        assert_eq!(labeler.get_label_for_category("personal"), "AGENT_PERSONAL");
        assert_eq!(labeler.get_label_for_category("spam"), "AGENT_SPAM");
        assert_eq!(labeler.get_label_for_category("promotional"), "AGENT_PROMOTIONAL");
        assert_eq!(labeler.get_label_for_category("newsletter"), "AGENT_NEWSLETTER");
        assert_eq!(labeler.get_label_for_category("urgent"), "AGENT_URGENT");
    }

    // Note: Integration tests for GmailLabeler would require real Gmail credentials
    // and should be in a separate test file with #[ignore] attributes
}
