//! Text preprocessing for email classification
//! 
//! This module provides text cleaning and preprocessing utilities specifically
//! designed for email classification, similar to the PII anonymization pipeline.

use html2text::from_read;
use regex::Regex;
use once_cell::sync::Lazy;

/// Remove email addresses from text (more specific, apply first)
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap()
});

/// Remove URLs from text using regex patterns (apply after email)
static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https?://[^\s]+|www\.[^\s]+").unwrap()
});


/// Remove tracking parameters and encoded URLs
static TRACKING_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"utm_[^&\s]*|[?&][^=]*=[^&\s]*").unwrap()
});

/// Remove HTML entities
static HTML_ENTITY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"&[a-zA-Z0-9#]+;").unwrap()
});

/// Remove excessive punctuation and special characters (keep brackets for placeholders)
static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^\w\s.,!?;:()\-'\[\]]").unwrap()
});

/// Remove multiple consecutive whitespace characters
static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\s+").unwrap()
});

/// Clean HTML email content for classification
/// 
/// This function converts HTML to plain text and removes various artifacts
/// that could interfere with classification accuracy.
pub fn clean_html_for_classification(html: &str) -> String {
    // Convert HTML to plain text with a reasonable line width
    let plain_text = from_read(html.as_bytes(), 100);
    
    // Apply comprehensive cleaning
    clean_text_for_classification(&plain_text)
}

/// Clean plain text for classification
/// 
/// Removes URLs, email addresses, phone numbers, and other noise that
/// doesn't contribute to semantic classification.
pub fn clean_text_for_classification(text: &str) -> String {
    let mut cleaned = text.to_string();
    
    // Remove HTML entities first
    cleaned = HTML_ENTITY_REGEX.replace_all(&cleaned, " ").to_string();
    
    // Remove email addresses first (more specific pattern)
    cleaned = EMAIL_REGEX.replace_all(&cleaned, "[EMAIL]").to_string();
    
    // Remove URLs (they don't help with classification and add noise)
    cleaned = URL_REGEX.replace_all(&cleaned, "[URL]").to_string();
    
    // Remove tracking parameters
    cleaned = TRACKING_REGEX.replace_all(&cleaned, "").to_string();
    
    // Remove control characters but preserve basic whitespace
    cleaned = cleaned
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect::<String>();
    
    // Remove excessive special characters (keep basic punctuation)
    cleaned = SPECIAL_CHAR_REGEX.replace_all(&cleaned, " ").to_string();
    
    // Normalize whitespace
    cleaned = WHITESPACE_REGEX.replace_all(&cleaned, " ").to_string();
    
    // Remove zero-width characters and other Unicode artifacts
    cleaned = cleaned.replace(['\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}', '\u{200f}', '\u{feff}', '\u{ad}'], "");
    
    // Trim and return
    cleaned.trim().to_string()
}

/// Extract meaningful text by limiting word count and removing common email noise
/// 
/// This function is specifically designed for classification where we want
/// to focus on the most semantically important parts of the email.
pub fn extract_meaningful_text_for_classification(text: &str, word_limit: usize) -> String {
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|word| !is_email_noise_word(word))
        .collect();
    
    if words.len() > word_limit {
        words[..word_limit].join(" ")
    } else {
        words.join(" ")
    }
}

/// Check if a word is email noise that doesn't help with classification
fn is_email_noise_word(word: &str) -> bool {
    const NOISE_WORDS: &[&str] = &[
        "unsubscribe", "click", "here", "view", "browser", "privacy", "policy",
        "terms", "conditions", "copyright", "reserved", "rights", "inc", "ltd",
        "llc", "corp", "company", "follow", "twitter", "facebook", "instagram",
        "linkedin", "subscribe", "newsletter", "update", "preferences"
    ];
    
    let lower_word = word.to_lowercase();
    
    // Keep placeholder tokens like [URL], [EMAIL], [PHONE] as they might be useful for classification
    if word.starts_with('[') && word.ends_with(']') {
        return false;
    }
    
    NOISE_WORDS.contains(&lower_word.as_str()) || 
    lower_word.len() < 2 || 
    lower_word.chars().all(|c| !c.is_alphabetic())
}

/// Prepare email content for classification
/// 
/// This is the main entry point that combines HTML cleaning, text cleaning,
/// and meaningful text extraction for optimal classification performance.
pub fn prepare_email_for_classification(
    subject: Option<&str>,
    snippet: Option<&str>, 
    body: Option<&str>,
    word_limit: usize
) -> String {
    let mut combined_text = String::new();
    
    // Add subject (most important for classification)
    if let Some(subj) = subject {
        if !subj.is_empty() {
            // Weight subject more heavily by repeating it
            combined_text.push_str(&format!("{} {} ", subj, subj));
        }
    }
    
    // Add snippet/preview (usually clean and relevant)
    if let Some(snip) = snippet {
        if !snip.is_empty() {
            let cleaned_snippet = clean_text_for_classification(snip);
            combined_text.push_str(&format!("{} ", cleaned_snippet));
        }
    }
    
    // Add body content (may contain HTML and noise)
    if let Some(body_content) = body {
        if !body_content.is_empty() {
            let cleaned_body = if body_content.contains('<') && body_content.contains('>') {
                clean_html_for_classification(body_content)
            } else {
                clean_text_for_classification(body_content)
            };
            combined_text.push_str(&cleaned_body);
        }
    }
    
    // Extract meaningful text with word limit
    extract_meaningful_text_for_classification(&combined_text, word_limit)
}

/// Prepare email metadata for classification
/// 
/// Extracts and cleans sender, recipient information that might be useful
/// for classification without exposing PII.
pub fn prepare_email_metadata_for_classification(
    from: Option<&str>,
    to: Option<&[String]>
) -> String {
    let mut metadata = String::new();
    
    // Extract domain from sender (useful for classification)
    if let Some(sender) = from {
        if let Some(domain) = extract_domain_from_email(sender) {
            metadata.push_str(&format!("from_domain:{} ", domain));
        }
    }
    
    // Extract domains from recipients
    if let Some(recipients) = to {
        for recipient in recipients {
            if let Some(domain) = extract_domain_from_email(recipient) {
                metadata.push_str(&format!("to_domain:{} ", domain));
            }
        }
    }
    
    metadata.trim().to_string()
}

/// Extract domain from email address for classification features
fn extract_domain_from_email(email: &str) -> Option<String> {
    if let Some(at_pos) = email.rfind('@') {
        let domain = &email[at_pos + 1..];
        // Clean up domain (remove brackets, etc.)
        let clean_domain = domain
            .trim_matches(['<', '>', '"', ' '])
            .to_lowercase();
        
        if !clean_domain.is_empty() && clean_domain.contains('.') {
            Some(clean_domain)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_html_for_classification() {
        let html = r#"<html><body><p>Hello <a href="http://example.com">world</a>!</p></body></html>"#;
        let cleaned = clean_html_for_classification(html);
        
        assert!(cleaned.contains("Hello"));
        assert!(cleaned.contains("world"));
        assert!(cleaned.contains("[URL]"));
        assert!(!cleaned.contains("<p>"));
        assert!(!cleaned.contains("href"));
    }

    #[test]
    fn test_clean_text_for_classification() {
        let text = "Visit https://example.com or email test@example.com for more info! Call +1-555-123-4567";
        let cleaned = clean_text_for_classification(text);
        
        assert!(cleaned.contains("[URL]"));
        assert!(cleaned.contains("[EMAIL]"));
        assert!(cleaned.contains("Visit"));
        assert!(cleaned.contains("more info"));
        assert!(cleaned.contains("Call")); // Phone numbers are preserved
        assert!(!cleaned.contains("https://"));
        assert!(!cleaned.contains("test@example.com"));
    }

    #[test]
    fn test_extract_meaningful_text_for_classification() {
        let text = "Important meeting tomorrow click here to unsubscribe view in browser";
        let extracted = extract_meaningful_text_for_classification(text, 10);
        
        assert!(extracted.contains("Important"));
        assert!(extracted.contains("meeting"));
        assert!(extracted.contains("tomorrow"));
        // Noise words should be filtered out
        assert!(!extracted.contains("unsubscribe"));
        assert!(!extracted.contains("click"));
        assert!(!extracted.contains("here"));
    }

    #[test]
    fn test_prepare_email_for_classification() {
        let subject = Some("Meeting Reminder");
        let snippet = Some("Don't forget our meeting tomorrow");
        let body = Some("<p>Hello,<br>This is a reminder about our <a href='http://example.com'>meeting</a></p>");
        
        let prepared = prepare_email_for_classification(subject, snippet, body, 20);
        
        // Subject should be weighted (appears twice)
        assert_eq!(prepared.matches("Meeting Reminder").count(), 2);
        assert!(prepared.contains("Don't forget"));
        assert!(prepared.contains("Hello"));
        assert!(prepared.contains("reminder"));
        assert!(prepared.contains("[URL]"));
        assert!(!prepared.contains("<p>"));
        assert!(!prepared.contains("href"));
    }

    #[test]
    fn test_prepare_email_metadata_for_classification() {
        let from = Some("John Doe <john@company.com>");
        let to_vec = vec!["jane@example.org".to_string(), "team@company.com".to_string()];
        let to = Some(to_vec.as_slice());
        
        let metadata = prepare_email_metadata_for_classification(from, to);
        
        assert!(metadata.contains("from_domain:company.com"));
        assert!(metadata.contains("to_domain:example.org"));
        assert!(metadata.contains("to_domain:company.com"));
        assert!(!metadata.contains("john"));
        assert!(!metadata.contains("jane"));
    }

    #[test]
    fn test_extract_domain_from_email() {
        assert_eq!(extract_domain_from_email("test@example.com"), Some("example.com".to_string()));
        assert_eq!(extract_domain_from_email("John <john@company.org>"), Some("company.org".to_string()));
        assert_eq!(extract_domain_from_email("invalid-email"), None);
        assert_eq!(extract_domain_from_email(""), None);
    }

    #[test]
    fn test_is_email_noise_word() {
        assert!(is_email_noise_word("unsubscribe"));
        assert!(is_email_noise_word("click"));
        assert!(is_email_noise_word("PRIVACY"));
        assert!(is_email_noise_word("123"));
        assert!(is_email_noise_word("a")); // too short
        
        assert!(!is_email_noise_word("[URL]")); // Keep placeholder tokens
        assert!(!is_email_noise_word("[EMAIL]")); // Keep placeholder tokens
        assert!(!is_email_noise_word("important"));
        assert!(!is_email_noise_word("meeting"));
        assert!(!is_email_noise_word("work"));
    }

    #[test]
    fn test_unicode_cleanup() {
        let text = "Hello\u{200b}world\u{feff}test\u{ad}word";
        let cleaned = clean_text_for_classification(text);
        
        assert_eq!(cleaned, "Hello world test word");
    }

    #[test]
    fn test_html_entities() {
        let text = "Hello &amp; goodbye &lt;test&gt; &quot;quoted&quot;";
        let cleaned = clean_text_for_classification(text);
        
        assert!(cleaned.contains("Hello"));
        assert!(cleaned.contains("goodbye"));
        assert!(cleaned.contains("test"));
        assert!(cleaned.contains("quoted"));
        assert!(!cleaned.contains("&amp;"));
        assert!(!cleaned.contains("&lt;"));
        assert!(!cleaned.contains("&gt;"));
        assert!(!cleaned.contains("&quot;"));
    }
}