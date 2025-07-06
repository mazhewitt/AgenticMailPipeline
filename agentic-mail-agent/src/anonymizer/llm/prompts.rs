//! Prompt templates for PII detection

/// Template for generating prompts for name extraction
pub struct PromptTemplate;

impl PromptTemplate {
    /// Generate a prompt for extracting names from email text
    pub fn extract_names(email_text: &str) -> String {
        format!(
            r#"You are a name extraction assistant. Extract all person names from this email text.

IMPORTANT: Return ONLY a JSON array of strings containing the names. No explanations or other text.

Example: ["John Smith", "Mary Johnson"]

Email text:
{email_text}

JSON array:"#
        )
    }

    /// Generate a prompt for extracting addresses from email text
    pub fn extract_addresses(email_text: &str) -> String {
        format!(
            r#"You are an address extraction assistant. Extract all physical addresses from this email text.

IMPORTANT: Return ONLY a JSON array of strings containing the addresses. No explanations or other text.

Include:
- Street addresses (123 Main St, City, ST 12345)
- PO Boxes (PO Box 1234, City, ST 12345)
- Apartment/Suite addresses (456 Oak Ave Apt 2B, City, ST 12345)
- International addresses with postal codes

Example: ["123 Main Street, Anytown, CA 12345", "PO Box 567, Springfield, IL 62701"]

Email text:
{email_text}

JSON array:"#
        )
    }

    /// Generate a prompt for extracting general PII from email text
    pub fn extract_pii(email_text: &str) -> String {
        format!(
            r#"You are a PII detection assistant. Extract all personally identifiable information from this email text.

IMPORTANT: Return ONLY a JSON array of objects with "type" and "text" fields. No explanations.

Example: [{{"type": "name", "text": "John Smith"}}, {{"type": "email", "text": "john@example.com"}}]

Email text:
{email_text}

JSON array:"#
        )
    }
}
