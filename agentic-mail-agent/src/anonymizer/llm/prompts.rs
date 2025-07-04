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
{}

JSON array:"#,
            email_text
        )
    }
    
    /// Generate a prompt for extracting general PII from email text
    pub fn extract_pii(email_text: &str) -> String {
        format!(
            r#"You are a PII detection assistant. Extract all personally identifiable information from this email text.

IMPORTANT: Return ONLY a JSON array of objects with "type" and "text" fields. No explanations.

Example: [{{"type": "name", "text": "John Smith"}}, {{"type": "email", "text": "john@example.com"}}]

Email text:
{}

JSON array:"#,
            email_text
        )
    }
}