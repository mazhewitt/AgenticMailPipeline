//! Gmail API trait abstraction for dependency injection and testing.

use async_trait::async_trait;
use google_gmail1::api::{Label, Message};

/// Result type for Gmail API operations.
pub type GmailApiResult<T> = Result<T, GmailApiError>;

/// Error type for Gmail API operations.
#[derive(Debug, Clone)]
pub enum GmailApiError {
    /// Authentication or permission errors
    Auth { message: String },
    /// Network or transport errors
    Network { message: String },
    /// API quota or rate limiting errors
    RateLimit { message: String },
    /// Invalid request parameters
    InvalidRequest { message: String },
    /// Resource not found
    NotFound { message: String },
    /// Generic API errors
    Api { message: String },
}

impl std::fmt::Display for GmailApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GmailApiError::Auth { message } => write!(f, "Gmail auth error: {message}"),
            GmailApiError::Network { message } => write!(f, "Gmail network error: {message}"),
            GmailApiError::RateLimit { message } => write!(f, "Gmail rate limit error: {message}"),
            GmailApiError::InvalidRequest { message } => {
                write!(f, "Gmail invalid request: {message}")
            }
            GmailApiError::NotFound { message } => write!(f, "Gmail resource not found: {message}"),
            GmailApiError::Api { message } => write!(f, "Gmail API error: {message}"),
        }
    }
}

impl std::error::Error for GmailApiError {}

impl GmailApiError {
    pub fn auth<S: Into<String>>(message: S) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    pub fn rate_limit<S: Into<String>>(message: S) -> Self {
        Self::RateLimit {
            message: message.into(),
        }
    }

    pub fn invalid_request<S: Into<String>>(message: S) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    pub fn not_found<S: Into<String>>(message: S) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub fn api<S: Into<String>>(message: S) -> Self {
        Self::Api {
            message: message.into(),
        }
    }
}

/// Abstract Gmail API operations needed by the labeler.
///
/// This trait defines the minimal set of Gmail API operations required
/// by the GmailLabeler, enabling dependency injection and easier testing.
///
/// The trait focuses on label and message operations:
/// - Label listing, creation, and deletion
/// - Message retrieval and modification
/// - Label-based message queries
#[async_trait]
pub trait GmailApi: Send + Sync {
    /// List all labels in the Gmail account.
    ///
    /// Returns a vector of labels with their names and IDs.
    async fn list_labels(&self) -> GmailApiResult<Vec<Label>>;

    /// Create a new label with the specified properties.
    ///
    /// # Arguments
    /// * `label` - The label to create, with name and visibility settings
    ///
    /// Returns the created label with its assigned ID.
    async fn create_label(&self, label: Label) -> GmailApiResult<Label>;

    /// Delete a label by its ID.
    ///
    /// # Arguments
    /// * `label_id` - The ID of the label to delete
    async fn delete_label(&self, label_id: &str) -> GmailApiResult<()>;

    /// Get a message by its ID.
    ///
    /// # Arguments
    /// * `message_id` - The ID of the message to retrieve
    ///
    /// Returns the message with its metadata and label IDs.
    async fn get_message(&self, message_id: &str) -> GmailApiResult<Message>;

    /// Modify a message's labels.
    ///
    /// # Arguments
    /// * `message_id` - The ID of the message to modify
    /// * `add_label_ids` - Label IDs to add to the message
    /// * `remove_label_ids` - Label IDs to remove from the message
    ///
    /// Returns the modified message.
    async fn modify_message_labels(
        &self,
        message_id: &str,
        add_label_ids: Option<Vec<String>>,
        remove_label_ids: Option<Vec<String>>,
    ) -> GmailApiResult<Message>;

    /// List messages with specific label IDs.
    ///
    /// # Arguments
    /// * `label_ids` - Filter messages that have these label IDs
    /// * `max_results` - Maximum number of messages to return (optional)
    ///
    /// Returns a list of message IDs that match the criteria.
    async fn list_messages_with_labels(
        &self,
        label_ids: &[String],
        max_results: Option<u32>,
    ) -> GmailApiResult<Vec<String>>;
}
