//! Types for agentic-mail-agent fetcher module.

/// Represents a simplified email.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    pub id: String,
    pub subject: String,
}

/// Errors that can occur during email fetching.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FetchError {
    /// Network or API error
    #[error("Network error")] 
    Network,
    /// Authentication error
    #[error("Authentication error")]
    Auth,
    /// Unknown error
    #[error("Unknown error")]
    Unknown,
}
