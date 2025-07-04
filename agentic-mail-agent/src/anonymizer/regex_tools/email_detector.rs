//! Email address detection using regex patterns

use regex::Regex;
use crate::anonymizer::types::PiiEntity;

/// Regex-based email address detector
pub struct EmailDetector {
    patterns: Vec<Regex>,
}

impl EmailDetector {
    /// Create a new email detector with comprehensive email patterns
    pub fn new() -> Self {
        let patterns = vec![
            // Standard email pattern (most common)
            Regex::new(r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b").unwrap(),
            // Email in HTML/XML attributes: <email@domain.com>
            Regex::new(r"<[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}>").unwrap(),
            // Email in quotes: "email@domain.com"  
            Regex::new(r#""[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}""#).unwrap(),
            // Email with display name: Name <email@domain.com>
            Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            // Obfuscated emails: email [at] domain [dot] com
            Regex::new(r"\b[a-zA-Z0-9._%-]+\s*\[at\]\s*[a-zA-Z0-9.-]+\s*\[dot\]\s*[a-zA-Z]{2,}\b").unwrap(),
            // Emails with (at) and (dot): email (at) domain (dot) com  
            Regex::new(r"\b[a-zA-Z0-9._%-]+\s*\(at\)\s*[a-zA-Z0-9.-]+\s*\(dot\)\s*[a-zA-Z]{2,}\b").unwrap(),
        ];
        
        Self { patterns }
    }
    
    /// Detect email addresses in the given text
    pub fn detect_emails(&self, text: &str) -> Vec<PiiEntity> {
        let mut entities = Vec::new();
        
        for pattern in &self.patterns {
            for mat in pattern.find_iter(text) {
                let email_text = mat.as_str();
                
                // Validate that it's a reasonable email address
                if self.is_valid_email(email_text) {
                    let new_entity = PiiEntity {
                        pii_type: "email".to_string(),
                        text: email_text.to_string(),
                        start: mat.start(),
                        end: mat.end(),
                    };
                    
                    // Check if this entity overlaps with any existing entity
                    let overlaps = entities.iter().any(|existing: &PiiEntity| {
                        // Check if ranges overlap
                        new_entity.start < existing.end && existing.start < new_entity.end
                    });
                    
                    if !overlaps {
                        entities.push(new_entity);
                    } else {
                        // If there's an overlap, prefer the longer match
                        let existing_idx = entities.iter().position(|existing: &PiiEntity| {
                            new_entity.start < existing.end && existing.start < new_entity.end
                        });
                        
                        if let Some(idx) = existing_idx {
                            let existing = &entities[idx];
                            if new_entity.text.len() > existing.text.len() {
                                entities[idx] = new_entity;
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by position
        entities.sort_by(|a, b| a.start.cmp(&b.start));
        entities
    }
    
    /// Basic validation for email addresses
    fn is_valid_email(&self, email: &str) -> bool {
        // Remove HTML tags and quotes
        let clean_email = email.trim_matches(|c| c == '<' || c == '>' || c == '"');
        
        // Handle obfuscated emails
        let normalized_email = clean_email
            .replace("[at]", "@")
            .replace("(at)", "@")
            .replace("[dot]", ".")
            .replace("(dot)", ".");
        
        // Basic validation
        if !normalized_email.contains('@') {
            return false;
        }
        
        let parts: Vec<&str> = normalized_email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }
        
        let local_part = parts[0];
        let domain_part = parts[1];
        
        // Local part validation
        if local_part.is_empty() || local_part.len() > 64 {
            return false;
        }
        
        // Domain part validation
        if domain_part.is_empty() || domain_part.len() > 253 {
            return false;
        }
        
        // Domain must contain at least one dot
        if !domain_part.contains('.') {
            return false;
        }
        
        // Domain parts should not be empty
        let domain_parts: Vec<&str> = domain_part.split('.').collect();
        if domain_parts.iter().any(|part| part.is_empty()) {
            return false;
        }
        
        // TLD should be at least 2 characters
        if let Some(tld) = domain_parts.last() {
            if tld.len() < 2 {
                return false;
            }
        }
        
        // Check for common fake/test domains that we should still anonymize
        let _fake_domains = [
            "example.com", "example.org", "example.net",
            "test.com", "test.org", "localhost",
            "domain.com", "email.com", "mail.com"
        ];
        
        // Even fake domains should be anonymized for consistency
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_email_detection() {
        let detector = EmailDetector::new();
        let text = "Contact me at john.doe@example.com for more info.";
        let emails = detector.detect_emails(text);
        
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].text, "john.doe@example.com");
        assert_eq!(emails[0].pii_type, "email");
    }

    #[test]
    fn test_multiple_emails() {
        let detector = EmailDetector::new();
        let text = "Send to admin@company.org and backup@test.net";
        let emails = detector.detect_emails(text);
        
        assert_eq!(emails.len(), 2);
        assert_eq!(emails[0].text, "admin@company.org");
        assert_eq!(emails[1].text, "backup@test.net");
    }

    #[test]
    fn test_email_in_html() {
        let detector = EmailDetector::new();
        let text = "Contact <john@example.com> or visit our website.";
        let emails = detector.detect_emails(text);
        
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].text, "<john@example.com>");
    }

    #[test]
    fn test_obfuscated_emails() {
        let detector = EmailDetector::new();
        let text = "Email me at john [at] example [dot] com";
        let emails = detector.detect_emails(text);
        
        assert_eq!(emails.len(), 1);
        assert!(emails[0].text.contains("[at]"));
        assert!(emails[0].text.contains("[dot]"));
    }
}