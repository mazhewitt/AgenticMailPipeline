//! Shared types for agentic-mail-agent.

/// Errors that can occur during email fetching or processing.
///
/// This enum provides a structured way to handle different types of errors
/// that can occur when interacting with email services.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum FetchError {
    /// Network or API communication error
    /// 
    /// This occurs when there are issues connecting to the email service,
    /// timeouts, or other network-related problems.
    #[error("Network error: {message}")]
    Network { message: String },
    
    /// Authentication or authorization error
    /// 
    /// This occurs when credentials are missing, invalid, expired,
    /// or when the user lacks permission to access the requested resource.
    #[error("Authentication error: {message}")]
    Auth { message: String },
    
    /// Configuration error
    /// 
    /// This occurs when environment variables are missing or configuration
    /// files are malformed or inaccessible.
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    /// Unknown or unexpected error
    /// 
    /// This is a catch-all for errors that don't fit other categories.
    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl FetchError {
    /// Create a new Network error with a message
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network { message: message.into() }
    }
    
    /// Create a new Auth error with a message
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth { message: message.into() }
    }
    
    /// Create a new Config error with a message
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    /// Create a new Unknown error with a message
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown { message: message.into() }
    }
}
