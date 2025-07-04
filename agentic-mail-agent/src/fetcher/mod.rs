//! Email fetcher trait and implementations.
//! 
//! This module provides an abstraction for fetching emails from various sources,
//! with implementations for Gmail API and a stub for testing.

mod gmail;
mod stub;

pub use gmail::GmailFetcher;
pub use stub::StubFetcher;

use crate::core::email::Email;
use crate::core::types::FetchError;

/// Trait for fetching unread emails from email services.
/// 
/// This trait provides a unified interface for fetching emails from different
/// email providers. Implementations should handle authentication, rate limiting,
/// and error handling appropriately.
/// 
/// # Examples
/// 
/// ```rust
/// use agentic_mail_agent::fetcher::{EmailFetcher, StubFetcher};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let fetcher = StubFetcher::new();
///     let emails = fetcher.fetch_unread_emails().await?;
///     println!("Fetched {} emails", emails.len());
///     Ok(())
/// }
/// ```
#[async_trait::async_trait]
pub trait EmailFetcher {
    /// Fetch unread emails from the email service.
    /// 
    /// This method should return a list of unread emails. The exact number
    /// and order of emails returned may vary depending on the implementation
    /// and the email service's capabilities.
    /// 
    /// # Errors
    /// 
    /// Returns a `FetchError` if:
    /// - Authentication fails (`FetchError::Auth`)
    /// - Network communication fails (`FetchError::Network`) 
    /// - Configuration is invalid (`FetchError::Config`)
    /// - An unexpected error occurs (`FetchError::Unknown`)
    async fn fetch_unread_emails(&self) -> Result<Vec<Email>, FetchError>;

    /// Fetch emails from the inbox (including read and unread).
    /// 
    /// This method fetches a configurable number of recent emails from the inbox,
    /// useful for testing and downloading sample data. The exact number and order
    /// of emails returned may vary depending on the implementation.
    /// 
    /// # Errors
    /// 
    /// Returns a `FetchError` if:
    /// - Authentication fails (`FetchError::Auth`)
    /// - Network communication fails (`FetchError::Network`) 
    /// - Configuration is invalid (`FetchError::Config`)
    /// - An unexpected error occurs (`FetchError::Unknown`)
    async fn fetch_inbox_emails(&self, max_results: u32) -> Result<Vec<Email>, FetchError>;
}
