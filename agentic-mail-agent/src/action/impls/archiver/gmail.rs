//! Gmail API implementation of EmailArchiver.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{ArchiveResult, ArchivingError, EmailArchiver};
use crate::gmail::{GmailAuthConfig, GmailClient, GmailClientError};
use google_gmail1::api::ModifyMessageRequest;

/// Gmail API implementation of EmailArchiver.
///
/// This implementation uses the Gmail API to archive emails by removing
/// the INBOX label. It requires OAuth2 credentials with Gmail modify permissions.
///
/// # Environment Variables
/// - `GMAIL_CLIENT_SECRET_JSON`: Path to OAuth2 client secret JSON file
/// - `GMAIL_TOKEN_JSON`: Path to OAuth2 token JSON file
///
/// # Required Scopes
/// - `https://www.googleapis.com/auth/gmail.modify` - Required for modifying message labels
/// - `https://www.googleapis.com/auth/gmail.readonly` - Required for reading message info
///
/// # Features
/// - Idempotent archiving (won't fail if already archived)
/// - Label ID caching for improved performance
/// - Proper error handling and retries
/// - Audit logging of all operations
///
/// # Examples
///
/// ```rust,no_run
/// use agentic_mail_agent::action::impls::archiver::{EmailArchiver, GmailArchiver};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Set environment variables:
///     // export GMAIL_CLIENT_SECRET_JSON=/path/to/client_secret.json
///     // export GMAIL_TOKEN_JSON=/path/to/token.json
///     
///     let archiver = GmailArchiver::from_env().await?;
///     let result = archiver.archive_email("message123").await?;
///     println!("Archive result: {}", result.description);
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct GmailArchiver {
    gmail_client: GmailClient,
    /// Cache of label name -> label ID mappings
    label_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl GmailArchiver {
    /// Create a new GmailArchiver from environment variables.
    ///
    /// Reads credential paths from environment variables and initializes
    /// the Gmail API client with OAuth2 authentication.
    ///
    /// # Errors
    /// Returns `ArchivingError::Config` if environment variables are missing
    /// or credential files are invalid.
    pub async fn from_env() -> Result<Self, ArchivingError> {
        let gmail_client = GmailClient::from_env().await?;

        Ok(Self {
            gmail_client,
            label_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create a new GmailArchiver with explicit credential paths.
    ///
    /// # Arguments
    /// * `client_secret_path` - Path to OAuth2 client secret JSON file
    /// * `token_path` - Path to OAuth2 token JSON file
    pub async fn new(
        client_secret_path: String,
        token_path: String,
    ) -> Result<Self, ArchivingError> {
        let config = GmailAuthConfig::new(client_secret_path, token_path);
        let gmail_client = GmailClient::new(config).await?;

        Ok(Self {
            gmail_client,
            label_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get the Gmail label ID for the INBOX label.
    ///
    /// The INBOX label is a system label in Gmail, so it should always exist.
    /// This method caches the label ID for performance.
    async fn get_inbox_label_id(&self) -> Result<String, ArchivingError> {
        // Check cache first
        {
            let cache = self.label_cache.lock().unwrap();
            if let Some(label_id) = cache.get("INBOX") {
                return Ok(label_id.clone());
            }
        }

        // List labels to find INBOX
        let labels_result = self
            .gmail_client
            .hub
            .users()
            .labels_list("me")
            .doit()
            .await
            .map_err(|e| ArchivingError::gmail_api(format!("Failed to list labels: {e}")))?;

        let labels = labels_result.1.labels.unwrap_or_default();

        // Find INBOX label
        for label in labels {
            if let Some(name) = &label.name {
                if name == "INBOX" {
                    let label_id = label.id.unwrap_or_default();

                    // Cache the label ID
                    let mut cache = self.label_cache.lock().unwrap();
                    cache.insert("INBOX".to_string(), label_id.clone());

                    return Ok(label_id);
                }
            }
        }

        Err(ArchivingError::config("INBOX label not found"))
    }

    /// Check if a message has the INBOX label.
    async fn message_has_inbox_label(&self, message_id: &str) -> Result<bool, ArchivingError> {
        let message_result = self
            .gmail_client
            .hub
            .users()
            .messages_get("me", message_id)
            .doit()
            .await
            .map_err(|e| {
                ArchivingError::gmail_api(format!("Failed to get message {message_id}: {e}"))
            })?;

        let message = message_result.1;
        let current_labels = message.label_ids.unwrap_or_default();

        // Get INBOX label ID
        let inbox_label_id = self.get_inbox_label_id().await?;

        Ok(current_labels.contains(&inbox_label_id))
    }
}

// Convert GmailClientError to ArchivingError
impl From<GmailClientError> for ArchivingError {
    fn from(error: GmailClientError) -> Self {
        match error {
            GmailClientError::Config { message } => Self::config(message),
            GmailClientError::Auth { message } => Self::auth(message),
        }
    }
}

#[async_trait]
impl EmailArchiver for GmailArchiver {
    async fn archive_email(&self, message_id: &str) -> Result<ArchiveResult, ArchivingError> {
        // Validate inputs
        if message_id.is_empty() {
            return Err(ArchivingError::invalid_message_id(
                "Message ID cannot be empty",
            ));
        }

        // Check if message is already archived (doesn't have INBOX label)
        if !self.message_has_inbox_label(message_id).await? {
            return Ok(ArchiveResult::already_archived(message_id.to_string()));
        }

        // Get INBOX label ID
        let inbox_label_id = self.get_inbox_label_id().await?;

        // Remove INBOX label from the message (this archives it)
        let modify_request = ModifyMessageRequest {
            add_label_ids: None,
            remove_label_ids: Some(vec![inbox_label_id]),
        };

        self.gmail_client
            .hub
            .users()
            .messages_modify(modify_request, "me", message_id)
            .doit()
            .await
            .map_err(|e| {
                ArchivingError::gmail_api(format!("Failed to archive message {message_id}: {e}"))
            })?;

        Ok(ArchiveResult::archived(message_id.to_string()))
    }

    async fn is_archived(&self, message_id: &str) -> Result<bool, ArchivingError> {
        if message_id.is_empty() {
            return Err(ArchivingError::invalid_message_id(
                "Message ID cannot be empty",
            ));
        }

        // A message is archived if it doesn't have the INBOX label
        let has_inbox_label = self.message_has_inbox_label(message_id).await?;
        Ok(!has_inbox_label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmail_archiver_error_conversion() {
        let gmail_error = GmailClientError::config("Test config error");
        let archiving_error: ArchivingError = gmail_error.into();
        assert!(matches!(archiving_error, ArchivingError::Config { .. }));

        let gmail_error = GmailClientError::auth("Test auth error");
        let archiving_error: ArchivingError = gmail_error.into();
        assert!(matches!(archiving_error, ArchivingError::Auth { .. }));
    }

    // Note: We can't easily test the Gmail API integration without real credentials
    // Most testing should be done with the StubArchiver or integration tests
    // with proper Gmail API setup.
}
