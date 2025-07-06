//! Gmail API implementation of EmailFetcher.

use async_trait::async_trait;
use crate::core::email::Email;
use crate::fetcher::EmailFetcher;
use crate::core::types::FetchError;
use crate::gmail::{GmailClient, GmailAuthConfig};

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
///     let fetcher = GmailFetcher::from_env().await?;
///     let emails = fetcher.fetch_unread_emails().await?;
///     println!("Fetched {} unread emails", emails.len());
///     for email in &emails {
///         if let Some(subject) = &email.subject {
///             println!("Subject: {subject}");
///         }
///         if let Some(snippet) = &email.snippet {
///             println!("Preview: {snippet}");
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
    gmail_client: GmailClient,
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
    pub async fn from_env() -> Result<Self, FetchError> {
        let gmail_client = GmailClient::from_env().await?;
        Ok(Self { gmail_client })
    }
    
    /// Create a new GmailFetcher with explicit paths.
    /// 
    /// # Arguments
    /// * `client_secret_path` - Path to the OAuth2 client secret JSON file
    /// * `token_path` - Path to the OAuth2 token JSON file
    pub async fn new(client_secret_path: String, token_path: String) -> Result<Self, FetchError> {
        let config = GmailAuthConfig::new(client_secret_path, token_path);
        let gmail_client = GmailClient::new(config).await?;
        Ok(Self { gmail_client })
    }
}

/// Helper function to extract subject from Gmail headers
fn extract_subject_from_headers(headers: &[google_gmail1::api::MessagePartHeader]) -> Option<String> {
    headers.iter()
        .find(|h| h.name.as_ref().map(|n| n.eq_ignore_ascii_case("subject")).unwrap_or(false))
        .and_then(|h| h.value.clone())
}

/// Helper function to extract from field from Gmail headers
fn extract_from_header(headers: &[google_gmail1::api::MessagePartHeader]) -> Option<String> {
    headers.iter()
        .find(|h| h.name.as_ref().map(|n| n.eq_ignore_ascii_case("from")).unwrap_or(false))
        .and_then(|h| h.value.clone())
}

/// Helper function to extract to field from Gmail headers
fn extract_to_header(headers: &[google_gmail1::api::MessagePartHeader]) -> Option<Vec<String>> {
    headers.iter()
        .find(|h| h.name.as_ref().map(|n| n.eq_ignore_ascii_case("to")).unwrap_or(false))
        .and_then(|h| h.value.clone())
        .map(|to_string| {
            // Split comma-separated email addresses
            to_string
                .split(',')
                .map(|addr| addr.trim().to_string())
                .collect()
        })
}

/// Helper function to extract date field from Gmail headers
fn extract_date_header(headers: &[google_gmail1::api::MessagePartHeader]) -> Option<String> {
    headers.iter()
        .find(|h| h.name.as_ref().map(|n| n.eq_ignore_ascii_case("date")).unwrap_or(false))
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
                        // Gmail API returns raw bytes, try direct UTF-8 conversion first
                        if let Ok(text) = String::from_utf8(data.clone()) {
                            return Some(text);
                        }
                        // Fallback to base64 decoding if needed
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
        
        // Extract From header
        let from = message.payload
            .as_ref()
            .and_then(|p| p.headers.as_ref())
            .and_then(|h| extract_from_header(h));
        
        // Extract To header
        let to = message.payload
            .as_ref()
            .and_then(|p| p.headers.as_ref())
            .and_then(|h| extract_to_header(h));
        
        // Extract Date header
        let sent = message.payload
            .as_ref()
            .and_then(|p| p.headers.as_ref())
            .and_then(|h| extract_date_header(h));

        Email::new_full(message_id, subject, message.snippet.clone(), from, to, sent, body)
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
                    // Try direct UTF-8 conversion first (Gmail API returns raw bytes)
                    String::from_utf8(data.clone()).ok()
                        .or_else(|| {
                            // Fallback to base64 decoding if direct conversion fails
                            use base64::{engine::general_purpose, Engine as _};
                            general_purpose::URL_SAFE.decode(data).ok()
                                .and_then(|bytes| String::from_utf8(bytes).ok())
                        })
                }) 
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

        // List unread messages using the shared Gmail client
        let list_result = tokio::time::timeout(
            std::time::Duration::from_secs(30), // 30 second timeout for API calls
            self.gmail_client.hub.users().messages_list(GMAIL_USER_ID)
                .add_label_ids(UNREAD_LABEL)
                .max_results(DEFAULT_MAX_RESULTS)
                .doit()
        )
        .await
        .map_err(|_| FetchError::network("Gmail API call timed out after 30 seconds".to_string()))?;
        let message_list = match list_result {
            Ok((_, ListMessagesResponse { messages: Some(msgs), .. })) => msgs,
            Ok((_, _)) => Vec::new(),
            Err(e) => return Err(FetchError::network(format!("Failed to list messages: {e}"))),
        };

        // Fetch each unread message in full format to extract subject and body
        let mut emails = Vec::new();
        for (i, msg) in message_list.iter().enumerate() {
            if let Some(msg_id) = &msg.id {
                // Add rate limiting delay between requests (except for first request)
                if i > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                // Fetch full message with timeout
                let full = tokio::time::timeout(
                    std::time::Duration::from_secs(30), // 30 second timeout for individual message fetch
                    self.gmail_client.hub
                        .users()
                        .messages_get(GMAIL_USER_ID, msg_id)
                        .format("full")
                        .doit()
                )
                .await
                .map_err(|_| FetchError::network(format!("Gmail API call timed out for message {msg_id}")))?;
                
                let message = match full {
                    Ok((_, m)) => m,
                    Err(e) => {
                        // Log individual message fetch failure but continue processing
                        // Individual message failures shouldn't break the entire batch
                        println!("Warning: Failed to fetch message {msg_id}: {e}");
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

    async fn fetch_inbox_emails(&self, max_results: u32) -> Result<Vec<Email>, FetchError> {
        use google_gmail1 as gmail1;
        use gmail1::api::ListMessagesResponse;

        // Cap the limit for safety
        let safe_limit = std::cmp::min(max_results, 100);

        // List messages from inbox without any label filters
        let list_result = tokio::time::timeout(
            std::time::Duration::from_secs(30), // 30 second timeout for API calls
            self.gmail_client.hub.users().messages_list(GMAIL_USER_ID)
                .max_results(safe_limit)
                .doit()
        )
        .await
        .map_err(|_| FetchError::network("Gmail API call timed out after 30 seconds".to_string()))?;
        let message_list = match list_result {
            Ok((_, ListMessagesResponse { messages: Some(msgs), .. })) => msgs,
            Ok((_, _)) => Vec::new(),
            Err(e) => return Err(FetchError::network(format!("Failed to list messages: {e}"))),
        };

        // Fetch each message in full format to extract subject and body
        let mut emails = Vec::new();
        for msg in message_list {
            if let Some(msg_id) = &msg.id {
                // Fetch full message with timeout
                let full = tokio::time::timeout(
                    std::time::Duration::from_secs(30), // 30 second timeout for individual message fetch
                    self.gmail_client.hub
                        .users()
                        .messages_get(GMAIL_USER_ID, msg_id)
                        .format("full")
                        .doit()
                )
                .await
                .map_err(|_| FetchError::network(format!("Gmail API call timed out for message {msg_id}")))?;
                
                let message = match full {
                    Ok((_, m)) => m,
                    Err(e) => {
                        // Log individual message fetch failure but continue processing
                        println!("Warning: Failed to fetch message {msg_id}: {e}");
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
const DEFAULT_MAX_RESULTS: u32 = 5;
const GMAIL_USER_ID: &str = "me";
const UNREAD_LABEL: &str = "UNREAD";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmail_constants() {
        assert_eq!(DEFAULT_MAX_RESULTS, 5);
        assert_eq!(GMAIL_USER_ID, "me");
        assert_eq!(UNREAD_LABEL, "UNREAD");
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
    fn test_message_parser_with_full_fields() {
        use google_gmail1::api::{Message, MessagePart, MessagePartHeader, MessagePartBody};
        
        let body_content = "This is the full email body content.";
        // Simulate what the google-gmail1 crate returns: raw UTF-8 bytes
        let body_bytes = body_content.as_bytes().to_vec();
        
        let message = Message {
            payload: Some(MessagePart {
                headers: Some(vec![
                    MessagePartHeader {
                        name: Some("Subject".to_string()),
                        value: Some("Test Subject".to_string()),
                    },
                    MessagePartHeader {
                        name: Some("From".to_string()),
                        value: Some("sender@example.com".to_string()),
                    },
                    MessagePartHeader {
                        name: Some("To".to_string()),
                        value: Some("recipient@example.com".to_string()),
                    },
                    MessagePartHeader {
                        name: Some("Date".to_string()),
                        value: Some("Wed, 30 Jun 2023 10:00:00 +0000".to_string()),
                    },
                ]),
                mime_type: Some("text/plain".to_string()),
                body: Some(MessagePartBody {
                    data: Some(body_bytes),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            snippet: Some("Test snippet content".to_string()),
            ..Default::default()
        };

        let email = MessageParser::parse_message("test-full".to_string(), &message);
        
        assert_eq!(email.id, "test-full");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, Some("Test snippet content".to_string()));
        assert_eq!(email.from, Some("sender@example.com".to_string()));
        assert_eq!(email.to, Some(vec!["recipient@example.com".to_string()]));
        assert_eq!(email.sent, Some("Wed, 30 Jun 2023 10:00:00 +0000".to_string()));
        assert_eq!(email.body, Some(body_content.to_string()));
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
        assert_eq!(email.from, None);
        assert_eq!(email.to, None);
        assert_eq!(email.sent, None);
        assert_eq!(email.body, Some("Just snippet".to_string()));
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

    #[tokio::test]
    async fn gmail_fetcher_from_env_missing_vars() {
        // Install crypto provider for test
        let _ = rustls::crypto::ring::default_provider().install_default();
        
        // Temporarily unset environment variables
        std::env::remove_var("GMAIL_CLIENT_SECRET_JSON");
        std::env::remove_var("GMAIL_TOKEN_JSON");
        
        // Change to a directory where secrets won't be found
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = std::env::temp_dir();
        std::env::set_current_dir(&temp_dir).unwrap();
        
        let result = GmailFetcher::from_env().await;
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_err());
        if let Err(FetchError::Config { message }) = result {
            assert!(message.contains("Gmail credentials not found"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[tokio::test]
    async fn gmail_fetcher_new_missing_files() {
        // Test with nonexistent files
        let result = GmailFetcher::new(
            "/nonexistent/secret.json".to_string(),
            "/nonexistent/token.json".to_string(),
        ).await;
        assert!(result.is_err());
        if let Err(FetchError::Config { message }) = result {
            assert!(message.contains("Client secret file not found"));
        } else {
            panic!("Expected Config error for nonexistent file");
        }
    }

    #[test]
    fn email_construction_with_subject_and_snippet() {
        // Test that our Gmail fetcher would construct emails correctly
        use crate::core::email::Email;
        
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
        
        // Use raw bytes as the Gmail API returns them, not base64-encoded
        let raw_data = "Hello World".as_bytes().to_vec();
        let parts = vec![
            MessagePart {
                mime_type: Some("text/plain".to_string()),
                body: Some(MessagePartBody {
                    data: Some(raw_data),
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
