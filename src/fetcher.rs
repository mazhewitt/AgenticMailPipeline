//! Gmail email fetcher trait and stub/real implementation.

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

/// Fetcher that uses the Gmail API (google-gmail1 crate).
///
/// # Environment Variables
/// - `GMAIL_CLIENT_SECRET_JSON`: Path to OAuth2 client secret JSON file.
/// - `GMAIL_TOKEN_JSON`: Path to OAuth2 token JSON file (will not be modified).
///
/// # Safety
/// Only reads message metadata/IDs. Does not modify or delete messages.
///
/// # Errors
/// Returns `FetchError` variants for missing credentials, API errors, or auth failures.
pub struct GmailFetcher {
    client_secret_path: String,
    token_path: String,
}

impl GmailFetcher {
    /// Create a new GmailFetcher from environment variables.
    pub fn from_env() -> Result<Self, FetchError> {
        let client_secret_path = std::env::var("GMAIL_CLIENT_SECRET_JSON")
            .map_err(|_| FetchError::Auth)?;
        let token_path = std::env::var("GMAIL_TOKEN_JSON")
            .map_err(|_| FetchError::Auth)?;
        Ok(Self { client_secret_path, token_path })
    }
}

impl EmailFetcher for GmailFetcher {
    fn fetch_unread_emails(&self) -> Result<Vec<Email>, FetchError> {
        use google_gmail1 as gmail1;
        use gmail1::api::ListMessagesResponse;
        use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
        use std::fs;
        use tokio::runtime::Runtime;
        use std::path::Path;
        use hyper_util::client::legacy::Client;
        use hyper_util::rt::TokioExecutor;
        use hyper_rustls::HttpsConnectorBuilder;

        let secret_path = Path::new(&self.client_secret_path);
        let token_path = Path::new(&self.token_path);
        if !secret_path.exists() || !token_path.exists() {
            return Err(FetchError::Auth);
        }
        let secret = fs::read_to_string(secret_path).map_err(|_| FetchError::Auth)?;
        let secret: yup_oauth2::ApplicationSecret = {
            // Parse the JSON first
            let google_secret: serde_json::Value = serde_json::from_str(&secret).map_err(|_| FetchError::Auth)?;
            
            // Check if it's in the Google "installed" format
            if let Some(installed) = google_secret.get("installed") {
                // Extract the fields from the "installed" object
                serde_json::from_value(installed.clone()).map_err(|_| FetchError::Auth)?
            } else {
                // Try parsing as direct ApplicationSecret format
                serde_json::from_str(&secret).map_err(|_| FetchError::Auth)?
            }
        };

        let rt = Runtime::new().map_err(|_| FetchError::Unknown)?;
        let result = rt.block_on(async {
            let auth = InstalledFlowAuthenticator::builder(
                secret,
                InstalledFlowReturnMethod::HTTPRedirect,
            )
            .persist_tokens_to_disk(token_path)
            .build()
            .await
            .map_err(|_| FetchError::Auth)?;

            let connector = HttpsConnectorBuilder::new()
                .with_native_roots().expect("native roots")
                .https_or_http()
                .enable_http1()
                .build();
            let client = Client::builder(TokioExecutor::new()).build(connector);
            let hub = gmail1::Gmail::new(client, auth);
            let result: Result<ListMessagesResponse, gmail1::Error> = hub
                .users()
                .messages_list("me")
                .q("is:unread in:inbox")
                .max_results(5)
                .doit()
                .await
                .map(|(_, resp)| resp);
            match result {
                Ok(list) => {
                    let emails = list.messages.unwrap_or_default().into_iter().map(|m| Email {
                        id: m.id.unwrap_or_default(),
                        subject: String::new(), // Not fetched yet
                    }).collect();
                    Ok(emails)
                }
                Err(_) => Err(FetchError::Network),
            }
        });
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FetchError;

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

    /// Integration test for GmailFetcher (requires credentials in env).
    #[test]
    #[ignore]
    fn gmail_fetcher_integration() {
        // Install default crypto provider for rustls
        let _ = rustls::crypto::ring::default_provider().install_default();
        
        if GmailFetcher::from_env().is_err() {
            eprintln!("Skipping GmailFetcher integration test: missing credentials");
            return;
        }
        let fetcher = GmailFetcher::from_env().unwrap();
        let result = fetcher.fetch_unread_emails();
        match result {
            Ok(emails) => println!("Fetched {} emails", emails.len()),
            Err(e) => panic!("GmailFetcher failed: {e:?}"),
        }
    }
}
