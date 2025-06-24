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
}

impl Email {
    /// Create a new Email with the given id, subject, and snippet.
    pub fn new(id: String, subject: Option<String>, snippet: Option<String>) -> Self {
        Self { id, subject, snippet }
    }
    
    /// Create a new Email with the given id and subject, but no snippet.
    pub fn with_subject(id: String, subject: String) -> Self {
        Self { 
            id, 
            subject: Some(subject), 
            snippet: None 
        }
    }
    
    /// Create an Email with just an ID and no subject or snippet.
    /// Useful when fetching metadata first and details later.
    pub fn with_id(id: String) -> Self {
        Self {
            id,
            subject: None,
            snippet: None,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_creation() {
        let email = Email::new(
            "123".to_string(), 
            Some("Test Subject".to_string()),
            Some("Test snippet preview".to_string())
        );
        assert_eq!(email.id, "123");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, Some("Test snippet preview".to_string()));
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
            Some("Snippet".to_string())
        );
        let email2 = Email::new(
            "123".to_string(), 
            Some("Subject".to_string()),
            Some("Snippet".to_string())
        );
        let email3 = Email::new(
            "456".to_string(), 
            Some("Subject".to_string()),
            Some("Snippet".to_string())
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
            Some("Real snippet".to_string())
        );
        assert_eq!(email_with_data.subject_or_default(), "Real Subject");
        assert_eq!(email_with_data.snippet_or_default(), "Real snippet");
    }
}
