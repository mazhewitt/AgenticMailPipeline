//! Gmail email fetcher trait and stub implementation.

use crate::types::{Email, FetchError};

/// Trait for fetching unread emails from Gmail.
pub trait EmailFetcher {
    /// Fetch unread emails from Gmail.
    fn fetch_unread_emails(&self) -> Result<Vec<Email>, FetchError>;
}

/// Stub fetcher for development/testing.
pub struct StubFetcher;

impl EmailFetcher for StubFetcher {
    fn fetch_unread_emails(&self) -> Result<Vec<Email>, FetchError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Email, FetchError};

    #[test]
    fn stub_fetcher_returns_empty() {
        let fetcher = StubFetcher;
        let result = fetcher.fetch_unread_emails();
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn fetch_error_equality() {
        assert_eq!(FetchError::Network, FetchError::Network);
        assert_ne!(FetchError::Network, FetchError::Auth);
    }
}
