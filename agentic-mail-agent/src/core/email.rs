//! Email domain types and related functionality.

/// Represents a simplified email message.
///
/// This is the core domain object representing an email in the system.
/// Contains basic metadata including subject and snippet (body preview).
/// Can be extended with additional fields like full body, attachments,
/// timestamps, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    /// Unique identifier for the email (e.g., Gmail message ID)
    pub id: String,
    /// Email subject line (None if not available or fetched)
    pub subject: Option<String>,
    /// Email body preview/snippet (None if not available or fetched)
    pub snippet: Option<String>,
    /// Sender's email address
    pub from: Option<String>,
    /// Recipient email addresses
    pub to: Option<Vec<String>>,
    /// Sent timestamp (ISO 8601 format)
    pub sent: Option<String>,
    /// Full email body content
    pub body: Option<String>,
}

impl Email {
    /// Create a new Email with the given id, subject, and snippet.
    pub fn new(id: String, subject: Option<String>, snippet: Option<String>) -> Self {
        Self {
            id,
            subject,
            snippet,
            from: None,
            to: None,
            sent: None,
            body: None,
        }
    }

    /// Create a new Email with all fields specified.
    pub fn new_full(
        id: String,
        subject: Option<String>,
        snippet: Option<String>,
        from: Option<String>,
        to: Option<Vec<String>>,
        sent: Option<String>,
        body: Option<String>,
    ) -> Self {
        Self {
            id,
            subject,
            snippet,
            from,
            to,
            sent,
            body,
        }
    }

    /// Create a new Email with the given id and subject, but no snippet.
    pub fn with_subject(id: String, subject: String) -> Self {
        Self {
            id,
            subject: Some(subject),
            snippet: None,
            from: None,
            to: None,
            sent: None,
            body: None,
        }
    }

    /// Create an Email with just an ID and no subject or snippet.
    /// Useful when fetching metadata first and details later.
    pub fn with_id(id: String) -> Self {
        Self {
            id,
            subject: None,
            snippet: None,
            from: None,
            to: None,
            sent: None,
            body: None,
        }
    }

    /// Get the subject as a string, returning a default if None.
    pub fn subject_or_default(&self) -> &str {
        self.subject.as_deref().unwrap_or("(No Subject)")
    }

    /// Get the snippet as a string, returning a default if None.
    pub fn snippet_or_default(&self) -> &str {
        self.snippet.as_deref().unwrap_or("(No Preview)")
    }

    /// Get the from field as a string, returning a default if None.
    pub fn from_or_default(&self) -> &str {
        self.from.as_deref().unwrap_or("(Unknown Sender)")
    }

    /// Get the to field as a vector, returning an empty vector if None.
    pub fn to_or_default(&self) -> Vec<String> {
        self.to.as_ref().cloned().unwrap_or_default()
    }

    /// Get the sent timestamp as a string, returning a default if None.
    pub fn sent_or_default(&self) -> &str {
        self.sent.as_deref().unwrap_or("(Unknown Date)")
    }

    /// Get the body as a string, returning a default if None.
    pub fn body_or_default(&self) -> &str {
        self.body.as_deref().unwrap_or("(No Body)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_creation() {
        let email = Email::new(
            "123".to_string(),
            Some("Test Subject".to_string()),
            Some("Test snippet preview".to_string()),
        );
        assert_eq!(email.id, "123");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, Some("Test snippet preview".to_string()));
    }

    #[test]
    fn email_creation_with_full_fields() {
        // This test should fail initially - we haven't added these fields yet
        let from = "sender@example.com".to_string();
        let to = vec!["recipient@example.com".to_string()];
        let sent = "2023-06-30T10:00:00Z".to_string();
        let body = "This is the full email body content.".to_string();

        let email = Email::new_full(
            "123".to_string(),
            Some("Test Subject".to_string()),
            Some("Test snippet".to_string()),
            Some(from.clone()),
            Some(to.clone()),
            Some(sent.clone()),
            Some(body.clone()),
        );

        assert_eq!(email.id, "123");
        assert_eq!(email.from, Some(from));
        assert_eq!(email.to, Some(to));
        assert_eq!(email.sent, Some(sent));
        assert_eq!(email.body, Some(body));
    }

    #[test]
    fn email_getters_for_new_fields() {
        let email = Email::new_full(
            "123".to_string(),
            Some("Subject".to_string()),
            Some("Snippet".to_string()),
            Some("sender@example.com".to_string()),
            Some(vec!["recipient@example.com".to_string()]),
            Some("2023-06-30T10:00:00Z".to_string()),
            Some("Full body content".to_string()),
        );

        assert_eq!(email.from_or_default(), "sender@example.com");
        assert_eq!(
            email.to_or_default(),
            vec!["recipient@example.com".to_string()]
        );
        assert_eq!(email.sent_or_default(), "2023-06-30T10:00:00Z");
        assert_eq!(email.body_or_default(), "Full body content");

        // Test defaults when fields are None
        let empty_email = Email::with_id("456".to_string());
        assert_eq!(empty_email.from_or_default(), "(Unknown Sender)");
        assert_eq!(empty_email.to_or_default(), Vec::<String>::new());
        assert_eq!(empty_email.sent_or_default(), "(Unknown Date)");
        assert_eq!(empty_email.body_or_default(), "(No Body)");
    }

    #[test]
    fn email_with_subject() {
        let email = Email::with_subject("456".to_string(), "Test Subject".to_string());
        assert_eq!(email.id, "456");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, None);
    }

    #[test]
    fn email_with_id() {
        let email = Email::with_id("789".to_string());
        assert_eq!(email.id, "789");
        assert_eq!(email.subject, None);
        assert_eq!(email.snippet, None);
    }

    #[test]
    fn email_equality() {
        let email1 = Email::new(
            "123".to_string(),
            Some("Subject".to_string()),
            Some("Snippet".to_string()),
        );
        let email2 = Email::new(
            "123".to_string(),
            Some("Subject".to_string()),
            Some("Snippet".to_string()),
        );
        let email3 = Email::new(
            "456".to_string(),
            Some("Subject".to_string()),
            Some("Snippet".to_string()),
        );

        assert_eq!(email1, email2);
        assert_ne!(email1, email3);
    }

    #[test]
    fn email_defaults() {
        let email = Email::with_id("123".to_string());
        assert_eq!(email.subject_or_default(), "(No Subject)");
        assert_eq!(email.snippet_or_default(), "(No Preview)");

        let email_with_data = Email::new(
            "456".to_string(),
            Some("Real Subject".to_string()),
            Some("Real snippet".to_string()),
        );
        assert_eq!(email_with_data.subject_or_default(), "Real Subject");
        assert_eq!(email_with_data.snippet_or_default(), "Real snippet");
    }
}
