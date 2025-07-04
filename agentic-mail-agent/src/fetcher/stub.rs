//! Stub implementation of EmailFetcher for testing and development.

use async_trait::async_trait;
use crate::core::email::Email;
use crate::fetcher::EmailFetcher;
use crate::core::types::FetchError;

/// Stub fetcher for development and testing.
/// 
/// This implementation provides a simple stub that can be used during
/// development and testing when you don't want to make actual API calls.
/// It can be configured to return specific emails or errors for testing
/// different scenarios.
/// 
/// # Examples
/// 
/// ```rust
/// use agentic_mail_agent::fetcher::{EmailFetcher, StubFetcher};
/// use agentic_mail_agent::email::Email;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let fetcher = StubFetcher::new();
///     let emails = fetcher.fetch_unread_emails().await?;
///     assert!(emails.is_empty()); // Default stub returns empty list
///     Ok(())
/// }
/// ```
pub struct StubFetcher {
    /// Emails to return when fetch_unread_emails is called
    emails: Vec<Email>,
    /// Error to return instead of emails, if set
    error: Option<FetchError>,
}

impl StubFetcher {
    /// Create a new StubFetcher that returns an empty list of emails.
    pub fn new() -> Self {
        Self {
            emails: Vec::new(),
            error: None,
        }
    }
    
    /// Create a StubFetcher that returns the specified emails.
    pub fn with_emails(emails: Vec<Email>) -> Self {
        Self {
            emails,
            error: None,
        }
    }
    
    /// Create a StubFetcher that returns the specified error.
    pub fn with_error(error: FetchError) -> Self {
        Self {
            emails: Vec::new(),
            error: Some(error),
        }
    }
}

impl Default for StubFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailFetcher for StubFetcher {
    async fn fetch_unread_emails(&self) -> Result<Vec<Email>, FetchError> {
        if let Some(error) = &self.error {
            Err(error.clone())
        } else {
            Ok(self.emails.clone())
        }
    }

    async fn fetch_inbox_emails(&self, max_results: u32) -> Result<Vec<Email>, FetchError> {
        if let Some(error) = &self.error {
            Err(error.clone())
        } else {
            // Return up to max_results emails from our configured emails
            let emails = self.emails.iter()
                .take(max_results as usize)
                .cloned()
                .collect();
            Ok(emails)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::email::Email;

    #[tokio::test]
    async fn stub_fetcher_returns_empty_by_default() {
        let fetcher = StubFetcher::new();
        let result = fetcher.fetch_unread_emails().await;
        assert_eq!(result.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn stub_fetcher_returns_configured_emails() {
        let emails = vec![
            Email::new(
                "1".to_string(), 
                Some("Test Email 1".to_string()),
                Some("This is the first test email".to_string())
            ),
            Email::new(
                "2".to_string(), 
                Some("Test Email 2".to_string()),
                Some("This is the second test email".to_string())
            ),
        ];
        let fetcher = StubFetcher::with_emails(emails.clone());
        let result = fetcher.fetch_unread_emails().await;
        assert_eq!(result.unwrap(), emails);
    }

    #[tokio::test]
    async fn stub_fetcher_returns_configured_error() {
        let error = FetchError::network("Test network error");
        let fetcher = StubFetcher::with_error(error.clone());
        let result = fetcher.fetch_unread_emails().await;
        assert_eq!(result.unwrap_err(), error);
    }

    #[tokio::test]
    async fn stub_fetcher_default() {
        let fetcher = StubFetcher::default();
        let result = fetcher.fetch_unread_emails().await;
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn stub_fetcher_fetch_inbox_emails_empty() {
        let fetcher = StubFetcher::new();
        let result = fetcher.fetch_inbox_emails(10).await;
        assert_eq!(result.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn stub_fetcher_fetch_inbox_emails_with_limit() {
        let emails = vec![
            Email::new(
                "1".to_string(), 
                Some("Test Email 1".to_string()),
                Some("This is the first test email".to_string())
            ),
            Email::new(
                "2".to_string(), 
                Some("Test Email 2".to_string()),
                Some("This is the second test email".to_string())
            ),
            Email::new(
                "3".to_string(), 
                Some("Test Email 3".to_string()),
                Some("This is the third test email".to_string())
            ),
        ];
        let fetcher = StubFetcher::with_emails(emails.clone());
        
        // Test with limit less than available emails
        let result = fetcher.fetch_inbox_emails(2).await;
        assert_eq!(result.unwrap().len(), 2);
        
        // Test with limit greater than available emails
        let result = fetcher.fetch_inbox_emails(5).await;
        assert_eq!(result.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn stub_fetcher_fetch_inbox_emails_returns_error() {
        let error = FetchError::network("Test network error");
        let fetcher = StubFetcher::with_error(error.clone());
        let result = fetcher.fetch_inbox_emails(10).await;
        assert_eq!(result.unwrap_err(), error);
    }
}
