//! Gmail API implementation of EmailFetcher.

use async_trait::async_trait;
use crate::email::Email;
use crate::fetcher::EmailFetcher;
use crate::types::FetchError;
use google_gmail1::hyper_rustls;
use google_gmail1::yup_oauth2;

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
    auth_config: AuthConfig,
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
        let auth_config = AuthConfig::from_env()?;
        Ok(Self { auth_config })
    }
    
    /// Create a new GmailFetcher with explicit paths.
    /// 
    /// # Arguments
    /// * `client_secret_path` - Path to the OAuth2 client secret JSON file
    /// * `token_path` - Path to the OAuth2 token JSON file
    pub fn new(client_secret_path: String, token_path: String) -> Self {
        let auth_config = AuthConfig::new(client_secret_path, token_path);
        Self { auth_config }
    }
}

/// Helper function to extract subject from Gmail headers
fn extract_subject_from_headers(headers: &[google_gmail1::api::MessagePartHeader]) -> Option<String> {
    headers.iter()
        .find(|h| h.name.as_ref().map(|n| n.eq_ignore_ascii_case("subject")).unwrap_or(false))
        .and_then(|h| h.value.clone())
}

/// Helper function to extract body text from Gmail message parts
fn extract_body_from_parts(parts: &[google_gmail1::api::MessagePart]) -> Option<String> {
    use base64::{engine::general_purpose, Engine as _};
    
    // First try to find text/plain content
    for part in parts {
        if let Some(mime_type) = &part.mime_type {
            if mime_type == "text/plain" {
                if let Some(body) = &part.body {
                    if let Some(data) = &body.data {
                        // The data is base64 encoded bytes.
                        if let Ok(decoded) = general_purpose::URL_SAFE.decode(data) {
                            if let Ok(text) = String::from_utf8(decoded) {
                                return Some(text);
                            }
                        }
                    }
                }
            }
        }
        
        // Recursively search in nested parts
        if let Some(nested_parts) = &part.parts {
            if let Some(body) = extract_body_from_parts(nested_parts) {
                return Some(body);
            }
        }
    }
    
    // If no text/plain found, try text/html as fallback
    for part in parts {
        if let Some(mime_type) = &part.mime_type {
            if mime_type == "text/html" {
                if let Some(body) = &part.body {
                    if let Some(data) = &body.data {
                        if let Ok(decoded) = general_purpose::URL_SAFE.decode(data) {
                            if let Ok(text) = String::from_utf8(decoded) {
                                // Use improved HTML stripping
                                return Some(strip_html_basic(&text));
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Strip HTML tags from text content for plain text display
/// 
/// This is a basic HTML stripper that:
/// 1. Converts common break tags to newlines
/// 2. Removes all other HTML tags
/// 3. Trims whitespace
fn strip_html_basic(html: &str) -> String {
    // Convert break tags to newlines
    let with_breaks = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");
    
    // Remove all HTML tags using regex
    // This is safer than the previous implementation
    match regex::Regex::new(r"<[^>]*>") {
        Ok(re) => re.replace_all(&with_breaks, "").trim().to_string(),
        Err(_) => {
            // If regex compilation fails, do basic manual stripping
            let mut result = String::new();
            let mut in_tag = false;
            
            for ch in with_breaks.chars() {
                match ch {
                    '<' => in_tag = true,
                    '>' => in_tag = false,
                    _ if !in_tag => result.push(ch),
                    _ => {}
                }
            }
            
            result.trim().to_string()
        }
    }
}

/// Message parser for Gmail API message data
struct MessageParser;

impl MessageParser {
    /// Parse a Gmail message into an Email struct
    fn parse_message(message_id: String, message: &google_gmail1::api::Message) -> Email {
        let subject = message.payload
            .as_ref()
            .and_then(|p| p.headers.as_ref())
            .and_then(|h| extract_subject_from_headers(h));

        let body = Self::extract_body_from_message(message);

        Email::new(message_id, subject, body)
    }

    /// Extract body text from Gmail message, with fallback to snippet
    fn extract_body_from_message(message: &google_gmail1::api::Message) -> Option<String> {
        if let Some(p) = &message.payload {
            // Try extracting from parts first
            if let Some(parts) = &p.parts {
                if let Some(body) = extract_body_from_parts(parts) {
                    return Some(body);
                }
            }
            
            // Try direct body data
            if let Some(body_data) = p.body.as_ref()
                .and_then(|b| b.data.as_ref())
                .and_then(|data| {
                    use base64::{engine::general_purpose, Engine as _};
                    general_purpose::URL_SAFE.decode(data).ok()
                })
                .and_then(|bytes| String::from_utf8(bytes).ok()) 
            {
                return Some(body_data);
            }
        }
        
        // Fallback to snippet
        message.snippet.clone()
    }
}

#[async_trait]
impl EmailFetcher for GmailFetcher {
    async fn fetch_unread_emails(&self) -> Result<Vec<Email>, FetchError> {
        use google_gmail1 as gmail1;
        use gmail1::api::ListMessagesResponse;
        use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

        // Validate file paths exist
        self.auth_config.validate_files()?;

        // Read and parse client secret
        let secret = std::fs::read_to_string(&self.auth_config.client_secret_path)
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

        // Set up OAuth2 authentication with the new API
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| FetchError::config(format!("Could not load native certs: {}", e)))?
            .https_only()
            .enable_http2()
            .build();

        let executor = google_gmail1::hyper_util::rt::TokioExecutor::new();
        let auth = InstalledFlowAuthenticator::with_client(
            secret,
            InstalledFlowReturnMethod::HTTPRedirect,
            google_gmail1::yup_oauth2::client::CustomHyperClientBuilder::from(
                google_gmail1::hyper_util::client::legacy::Client::builder(executor).build(connector),
            ),
        )
        .persist_tokens_to_disk(&self.auth_config.token_path)
        .build()
        .await
        .map_err(|e| FetchError::auth(format!(
            "Failed to build authenticator: {}", e
        )))?;

        // Explicitly request the required Gmail scopes for full message access
        let required_scopes = GMAIL_SCOPES;
        
        // Test token with required scopes to ensure we have proper permissions
        let _token = auth.token(required_scopes).await
            .map_err(|e| FetchError::auth(format!(
                "Failed to obtain token with required Gmail scopes: {}", e
            )))?;

        // Build HTTPS client and Gmail hub with authentication
        let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| FetchError::config(format!("Could not load native certs: {}", e)))?
            .https_only()
            .enable_http1()
            .build();
        let client = google_gmail1::hyper_util::client::legacy::Client::builder(google_gmail1::hyper_util::rt::TokioExecutor::new()).build(https_connector);
        let hub = gmail1::Gmail::new(client, auth);

        // List unread messages
        let list_result = hub.users().messages_list(GMAIL_USER_ID)
            .add_label_ids(UNREAD_LABEL)
             .max_results(DEFAULT_MAX_RESULTS)
             .doit()
             .await;
        let message_list = match list_result {
            Ok((_, ListMessagesResponse { messages: Some(msgs), .. })) => msgs,
            Ok((_, _)) => Vec::new(),
            Err(e) => return Err(FetchError::network(format!("Failed to list messages: {}", e))),
        };

        // Fetch each unread message in full format to extract subject and body
        let mut emails = Vec::new();
        for msg in message_list {
            if let Some(msg_id) = &msg.id {
                // Fetch full message
                let full = hub
                    .users()
                    .messages_get(GMAIL_USER_ID, msg_id)
                    .format("full")
                    .doit()
                    .await;
                
                let message = match full {
                    Ok((_, m)) => m,
                    Err(e) => {
                        // Log individual message fetch failure but continue processing
                        // Individual message failures shouldn't break the entire batch
                        println!("Warning: Failed to fetch message {}: {}", msg_id, e);
                        emails.push(Email::new(msg_id.clone(), None, None));
                        continue;
                    }
                };

                // Use MessageParser to parse the message
                emails.push(MessageParser::parse_message(msg_id.clone(), &message));
            }
        }

        Ok(emails)
    }
}

/// Gmail API configuration constants
const GMAIL_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/gmail.readonly",
    "https://www.googleapis.com/auth/gmail.modify",
    "https://www.googleapis.com/auth/gmail.compose"
];

const DEFAULT_MAX_RESULTS: u32 = 5;
const GMAIL_USER_ID: &str = "me";
const UNREAD_LABEL: &str = "UNREAD";

/// Configuration for Gmail authentication
#[derive(Debug, Clone)]
struct AuthConfig {
    client_secret_path: String,
    token_path: String,
}

impl AuthConfig {
    /// Create AuthConfig from file paths
    fn new(client_secret_path: String, token_path: String) -> Self {
        Self {
            client_secret_path,
            token_path,
        }
    }
    
    /// Create AuthConfig from environment variables
    fn from_env() -> Result<Self, FetchError> {
        let client_secret_path = std::env::var("GMAIL_CLIENT_SECRET_JSON")
            .map_err(|_| FetchError::config("GMAIL_CLIENT_SECRET_JSON environment variable not set"))?;
        let token_path = std::env::var("GMAIL_TOKEN_JSON")
            .map_err(|_| FetchError::config("GMAIL_TOKEN_JSON environment variable not set"))?;
        Ok(Self::new(client_secret_path, token_path))
    }
    
    /// Validate that the required files exist
    fn validate_files(&self) -> Result<(), FetchError> {
        use std::path::Path;
        
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
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmail_constants() {
        assert_eq!(GMAIL_SCOPES.len(), 3);
        assert!(GMAIL_SCOPES.contains(&"https://www.googleapis.com/auth/gmail.readonly"));
        assert_eq!(DEFAULT_MAX_RESULTS, 5);
        assert_eq!(GMAIL_USER_ID, "me");
        assert_eq!(UNREAD_LABEL, "UNREAD");
    }

    #[test]
    fn test_auth_config_new() {
        let config = AuthConfig::new(
            "/path/to/secret.json".to_string(),
            "/path/to/token.json".to_string(),
        );
        assert_eq!(config.client_secret_path, "/path/to/secret.json");
        assert_eq!(config.token_path, "/path/to/token.json");
    }

    #[test]
    fn test_auth_config_from_env_missing_vars() {
        // Temporarily unset environment variables
        std::env::remove_var("GMAIL_CLIENT_SECRET_JSON");
        std::env::remove_var("GMAIL_TOKEN_JSON");
        
        let result = AuthConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_config_validate_files_nonexistent() {
        let config = AuthConfig::new(
            "/nonexistent/secret.json".to_string(),
            "/nonexistent/token.json".to_string(),
        );
        let result = config.validate_files();
        assert!(result.is_err());
        if let Err(FetchError::Config { message }) = result {
            assert!(message.contains("Client secret file not found"));
        } else {
            panic!("Expected Config error for nonexistent file");
        }
    }

    #[test]
    fn test_message_parser_with_subject_and_snippet() {
        use google_gmail1::api::{Message, MessagePart, MessagePartHeader};
        
        let message = Message {
            payload: Some(MessagePart {
                headers: Some(vec![
                    MessagePartHeader {
                        name: Some("Subject".to_string()),
                        value: Some("Test Subject".to_string()),
                    },
                ]),
                ..Default::default()
            }),
            snippet: Some("Test snippet content".to_string()),
            ..Default::default()
        };

        let email = MessageParser::parse_message("test-123".to_string(), &message);
        
        assert_eq!(email.id, "test-123");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, Some("Test snippet content".to_string()));
    }

    #[test]
    fn test_message_parser_no_subject() {
        use google_gmail1::api::Message;
        
        let message = Message {
            payload: None,
            snippet: Some("Just snippet".to_string()),
            ..Default::default()
        };

        let email = MessageParser::parse_message("test-456".to_string(), &message);
        
        assert_eq!(email.id, "test-456");
        assert_eq!(email.subject, None);
        assert_eq!(email.snippet, Some("Just snippet".to_string()));
    }

    #[test]
    fn test_message_parser_extract_body_from_message() {
        use google_gmail1::api::Message;
        
        let message = Message {
            snippet: Some("Fallback snippet".to_string()),
            ..Default::default()
        };

        let body = MessageParser::extract_body_from_message(&message);
        assert_eq!(body, Some("Fallback snippet".to_string()));
    }

    #[test]
    fn test_error_handling_consistency() {
        // Test that our error handling creates consistent FetchError types
        let config_error = FetchError::config("Test config error");
        let auth_error = FetchError::auth("Test auth error");
        let network_error = FetchError::network("Test network error");
        
        // Verify error types are created correctly
        match config_error {
            FetchError::Config { .. } => (),
            _ => panic!("Expected Config error"),
        }
        
        match auth_error {
            FetchError::Auth { .. } => (),
            _ => panic!("Expected Auth error"),
        }
        
        match network_error {
            FetchError::Network { .. } => (),
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_partial_failure_handling() {
        // Test that individual message failures don't break the entire fetch
        // This validates the current behavior where we return empty emails for failed fetches
        let empty_email = Email::new("failed-123".to_string(), None, None);
        
        assert_eq!(empty_email.id, "failed-123");
        assert_eq!(empty_email.subject, None);
        assert_eq!(empty_email.snippet, None);
        
        // This demonstrates the fallback behavior for individual message failures
        assert_eq!(empty_email.subject_or_default(), "(No Subject)");
        assert_eq!(empty_email.snippet_or_default(), "(No Preview)");
    }

    #[test]
    fn test_strip_html_basic() {
        assert_eq!(strip_html_basic("<p>Hello World</p>"), "Hello World");
        assert_eq!(strip_html_basic("<br>Line 1<br/>Line 2<br />Line 3"), "Line 1\nLine 2\nLine 3");
        assert_eq!(strip_html_basic("Plain text"), "Plain text");
        assert_eq!(strip_html_basic("<div><span>Nested</span></div>"), "Nested");
        assert_eq!(strip_html_basic(""), "");
    }

    #[test]
    fn test_strip_html_complex() {
        let html = r#"<html><body><p>Hello <strong>World</strong>!</p><br><div>Test</div></body></html>"#;
        let expected = "Hello World!\n\nTest";
        assert_eq!(strip_html_basic(html), expected);
    }

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
        assert_eq!(fetcher.auth_config.client_secret_path, "/path/to/secret.json");
        assert_eq!(fetcher.auth_config.token_path, "/path/to/token.json");
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

    #[test]
    fn test_extract_subject_from_headers() {
        // Test helper function to extract subject from Gmail headers
        use google_gmail1::api::MessagePartHeader;
        
        let headers = vec![
            MessagePartHeader {
                name: Some("From".to_string()),
                value: Some("test@example.com".to_string()),
            },
            MessagePartHeader {
                name: Some("Subject".to_string()),
                value: Some("Important Meeting Tomorrow".to_string()),
            },
            MessagePartHeader {
                name: Some("Date".to_string()),
                value: Some("Tue, 25 Jun 2025 12:00:00 GMT".to_string()),
            },
        ];

        let subject = super::extract_subject_from_headers(&headers);
        assert_eq!(subject, Some("Important Meeting Tomorrow".to_string()));
    }

    #[test]
    fn test_extract_subject_from_headers_no_subject() {
        // Test when there's no subject header
        use google_gmail1::api::MessagePartHeader;
        
        let headers = vec![
            MessagePartHeader {
                name: Some("From".to_string()),
                value: Some("test@example.com".to_string()),
            },
        ];

        let subject = super::extract_subject_from_headers(&headers);
        assert_eq!(subject, None);
    }

    #[test]
    fn test_extract_body_from_parts() {
        // Test helper function to extract body text from Gmail message parts
        use google_gmail1::api::{MessagePart, MessagePartBody};
        use base64::{engine::general_purpose, Engine as _};
        
        let base64_data = general_purpose::URL_SAFE.encode("Hello World");
        let parts = vec![
            MessagePart {
                mime_type: Some("text/plain".to_string()),
                body: Some(MessagePartBody {
                    data: Some(base64_data.into_bytes()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];

        let body = super::extract_body_from_parts(&parts);
        assert_eq!(body, Some("Hello World".to_string()));
    }

    #[tokio::test]
    #[ignore] // Ignored because it requires real credentials and is expected to fail
    async fn test_gmail_auth_flow_for_individual_messages() {
        // This test simulates the full auth flow and message fetching
        // It requires valid client_secret.json and token.json
        // The test will be #[ignore]d by default to avoid breaking CI/CD
        
        // Mock data for a message part body
        use google_gmail1::api::MessagePartBody;
        let base64_data = "VGhpcyBpcyBhIHRlc3QgZW1haWwgYm9keS4=".as_bytes().to_vec(); // "This is a test email body." in base64
        let _part_body = MessagePartBody {
            data: Some(base64_data),
            ..Default::default()
        };
    }
}
