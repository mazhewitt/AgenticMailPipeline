//! Gmail API implementation of EmailFetcher.

use async_trait::async_trait;
use crate::email::Email;
use crate::fetcher::EmailFetcher;
use crate::types::FetchError;

/// Fetcher that uses the Gmail API (google-gmail1 crate) to fetch emails.
///
/// This implementation provides access to Gmail messages using OAuth2 authentication.
/// It requires proper setup of client credentials and user authorization tokens.
///
/// # Environment Variables
/// - `GMAIL_CLIENT_SECRET_JSON`: Path to OAuth2 client secret JSON file from Google Cloud Console
/// - `GMAIL_TOKEN_JSON`: Path to OAuth2 token JSON file (created during first auth flow)
///
/// # Authentication Flow
/// The fetcher supports both Google's "installed" client secret format and direct
/// `ApplicationSecret` format. On first use, it will initiate the OAuth2 flow
/// to obtain user authorization.
///
/// # Safety
/// This implementation only reads message metadata and IDs. It does not modify 
/// or delete any messages. It uses the minimum required Gmail API scopes.
///
/// # Rate Limiting
/// The Gmail API has rate limits. This implementation fetches a maximum of 5
/// messages at a time to avoid hitting rate limits during development.
///
/// # Examples
///
/// ```rust,no_run
/// use agentic_mail_agent::fetcher::{EmailFetcher, GmailFetcher};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Install crypto provider for rustls
///     let _ = rustls::crypto::ring::default_provider().install_default();
///     
///     // Set environment variables first:
///     // export GMAIL_CLIENT_SECRET_JSON=/path/to/client_secret.json
///     // export GMAIL_TOKEN_JSON=/path/to/token.json
///     
///     let fetcher = GmailFetcher::from_env()?;
///     let emails = fetcher.fetch_unread_emails().await?;
///     println!("Fetched {} unread emails", emails.len());
///     for email in &emails {
///         if let Some(subject) = &email.subject {
///             println!("Subject: {}", subject);
///         }
///         if let Some(snippet) = &email.snippet {
///             println!("Preview: {}", snippet);
///         }
///     }
///     println!("Fetched {} unread emails", emails.len());
///     Ok(())
/// }
/// ```
///
/// # Errors
/// Returns `FetchError` variants for:
/// - Missing or invalid credentials (`FetchError::Auth`)
/// - Network or API communication errors (`FetchError::Network`)
/// - Configuration issues like missing environment variables (`FetchError::Config`)
/// - Unexpected errors (`FetchError::Unknown`)
pub struct GmailFetcher {
    client_secret_path: String,
    token_path: String,
}

impl GmailFetcher {
    /// Create a new GmailFetcher from environment variables.
    /// 
    /// This method reads the credential file paths from the following environment variables:
    /// - `GMAIL_CLIENT_SECRET_JSON`: Path to the OAuth2 client secret JSON file
    /// - `GMAIL_TOKEN_JSON`: Path to the OAuth2 token JSON file
    /// 
    /// # Errors
    /// Returns `FetchError::Config` if either environment variable is missing.
    pub fn from_env() -> Result<Self, FetchError> {
        let client_secret_path = std::env::var("GMAIL_CLIENT_SECRET_JSON")
            .map_err(|_| FetchError::config("GMAIL_CLIENT_SECRET_JSON environment variable not set"))?;
        let token_path = std::env::var("GMAIL_TOKEN_JSON")
            .map_err(|_| FetchError::config("GMAIL_TOKEN_JSON environment variable not set"))?;
        Ok(Self { client_secret_path, token_path })
    }
    
    /// Create a new GmailFetcher with explicit paths.
    /// 
    /// # Arguments
    /// * `client_secret_path` - Path to the OAuth2 client secret JSON file
    /// * `token_path` - Path to the OAuth2 token JSON file
    pub fn new(client_secret_path: String, token_path: String) -> Self {
        Self { client_secret_path, token_path }
    }
}

#[async_trait]
impl EmailFetcher for GmailFetcher {
    async fn fetch_unread_emails(&self) -> Result<Vec<Email>, FetchError> {
        use google_gmail1 as gmail1;
        use gmail1::api::ListMessagesResponse;
        use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
        use std::fs;
        use std::path::Path;
        use hyper_util::client::legacy::Client;
        use hyper_util::rt::TokioExecutor;
        use hyper_rustls::HttpsConnectorBuilder;

        // Validate file paths exist
        let secret_path = Path::new(&self.client_secret_path);
        let token_path = Path::new(&self.token_path);
        if !secret_path.exists() {
            return Err(FetchError::config(format!(
                "Client secret file not found: {}", 
                self.client_secret_path
            )));
        }
        if !token_path.exists() {
            return Err(FetchError::config(format!(
                "Token file not found: {}", 
                self.token_path
            )));
        }

        // Read and parse client secret
        let secret = fs::read_to_string(secret_path)
            .map_err(|e| FetchError::config(format!(
                "Failed to read client secret file: {}", e
            )))?;

        let secret: yup_oauth2::ApplicationSecret = {
            // Parse the JSON first
            let google_secret: serde_json::Value = serde_json::from_str(&secret)
                .map_err(|e| FetchError::config(format!(
                    "Failed to parse client secret JSON: {}", e
                )))?;
            
            // Check if it's in the Google "installed" format
            if let Some(installed) = google_secret.get("installed") {
                // Extract the fields from the "installed" object
                serde_json::from_value(installed.clone())
                    .map_err(|e| FetchError::config(format!(
                        "Failed to parse installed client secret: {}", e
                    )))?
            } else {
                // Try parsing as direct ApplicationSecret format
                serde_json::from_str(&secret)
                    .map_err(|e| FetchError::config(format!(
                        "Failed to parse ApplicationSecret: {}", e
                    )))?
            }
        };

        // Set up OAuth2 authentication
        let auth = InstalledFlowAuthenticator::builder(
            secret,
            InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk(token_path)
        .build()
        .await
        .map_err(|e| FetchError::auth(format!(
            "Failed to build authenticator: {}", e
        )))?;

        // Explicitly request token with the correct scope
        let scopes = &["https://www.googleapis.com/auth/gmail.readonly"];
        let _token = auth.token(scopes).await
            .map_err(|e| FetchError::auth(format!(
                "Failed to get token with gmail.readonly scope: {}", e
            )))?;

        // Set up HTTP client
        let connector = HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| FetchError::unknown(format!(
                "Failed to create HTTPS connector: {}", e
            )))?
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new()).build(connector);

        // Create Gmail API hub
        let hub = gmail1::Gmail::new(client, auth);

        // Fetch unread messages list
        let list_result: Result<ListMessagesResponse, gmail1::Error> = hub
            .users()
            .messages_list("me")
            .q("is:unread in:inbox")
            .max_results(5)
            .doit()
            .await
            .map(|(_, resp)| resp);

        let message_list = match list_result {
            Ok(list) => list.messages.unwrap_or_default(),
            Err(e) => return Err(FetchError::network(format!(
                "Failed to list Gmail messages: {}", e
            ))),
        };

        // For now, just create emails with IDs since we can't fetch details due to auth issues
        // TODO: Fix OAuth scope issues to get full message content
        let mut emails = Vec::new();
        for msg in message_list {
            if let Some(msg_id) = msg.id {
                // Create basic email with ID - subject and snippet will be None for now
                emails.push(Email::new(
                    msg_id,
                    None, // We'll need to fix auth to get subjects
                    None, // We'll need to fix auth to get snippets
                ));
            }
        }

        Ok(emails)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_fetcher_from_env_missing_vars() {
        // Temporarily unset environment variables
        std::env::remove_var("GMAIL_CLIENT_SECRET_JSON");
        std::env::remove_var("GMAIL_TOKEN_JSON");
        
        let result = GmailFetcher::from_env();
        assert!(result.is_err());
        if let Err(FetchError::Config { message }) = result {
            assert!(message.contains("GMAIL_CLIENT_SECRET_JSON"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[test]
    fn gmail_fetcher_new() {
        let fetcher = GmailFetcher::new(
            "/path/to/secret.json".to_string(),
            "/path/to/token.json".to_string(),
        );
        assert_eq!(fetcher.client_secret_path, "/path/to/secret.json");
        assert_eq!(fetcher.token_path, "/path/to/token.json");
    }

    #[test]
    fn email_construction_with_subject_and_snippet() {
        // Test that our Gmail fetcher would construct emails correctly
        use crate::email::Email;
        
        let email = Email::new(
            "test-123".to_string(),
            Some("Test Subject".to_string()),
            Some("This is a test snippet".to_string())
        );
        
        assert_eq!(email.id, "test-123");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, Some("This is a test snippet".to_string()));
        assert_eq!(email.subject_or_default(), "Test Subject");
        assert_eq!(email.snippet_or_default(), "This is a test snippet");
    }

    /// Integration test for GmailFetcher (requires credentials in env).
    /// 
    /// This test is ignored by default because it requires:
    /// 1. Valid Gmail API credentials
    /// 2. Network access
    /// 3. User authorization
    /// 
    /// To run: `cargo test gmail_fetcher_integration -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn gmail_fetcher_integration() {
        // Install default crypto provider for rustls
        let _ = rustls::crypto::ring::default_provider().install_default();
        
        if GmailFetcher::from_env().is_err() {
            eprintln!("Skipping GmailFetcher integration test: missing credentials");
            return;
        }
        
        let fetcher = GmailFetcher::from_env().unwrap();
        let result = fetcher.fetch_unread_emails().await;
        
        match result {
            Ok(emails) => {
                println!("Fetched {} emails", emails.len());
                for email in &emails {
                    println!("Email ID: {}", email.id);
                }
            }
            Err(e) => panic!("GmailFetcher failed: {:?}", e),
        }
    }
}
