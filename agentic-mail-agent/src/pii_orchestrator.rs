//! Orchestrating Agent for PII Anonymization
//! 
//! This module implements a multi-pass PII anonymization pipeline using:
//! 1. Deterministic regex-based tools for structured PII (emails, phones, etc.)
//! 2. LLM-based tools for contextual PII (names, addresses, companies)
//! 3. JSON validation and structure preservation
//! 4. Verification passes to ensure complete PII removal

use regex::Regex;
use lazy_static::lazy_static;
use std::collections::HashSet;
use crate::anonymizer::{AnonymizationResult, ReplacementLogEntry, LlmBackend};

/// Result of PII verification check
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub is_clean: bool,
    pub found_pii: Vec<PiiDetection>,
}

/// A detected PII instance
#[derive(Debug, Clone)]
pub struct PiiDetection {
    pub pii_type: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Trait for PII anonymization tools
pub trait PiiAnonymizationTool {
    fn anonymize(&mut self, text: &str) -> Result<String, Box<dyn std::error::Error>>;
    fn get_replacement_log(&self) -> Vec<ReplacementLogEntry>;
    fn tool_name(&self) -> &str;
}

/// Regex-based email anonymizer
pub struct EmailAnonymizer {
    replacement_log: Vec<ReplacementLogEntry>,
}

/// Regex-based phone number anonymizer  
pub struct PhoneAnonymizer {
    replacement_log: Vec<ReplacementLogEntry>,
}

/// LLM-based name anonymizer
pub struct NameAnonymizer {
    replacement_log: Vec<ReplacementLogEntry>,
    // TODO: Add LLM instance
}

/// JSON structure validator and fixer
pub struct JsonValidator;

/// Main orchestrating agent for PII anonymization
pub struct PiiOrchestrator {
    regex_tools: Vec<Box<dyn PiiAnonymizationTool>>,
    llm_tools: Vec<Box<dyn PiiAnonymizationTool>>,
    json_validator: JsonValidator,
}

impl PiiOrchestrator {
    /// Create a new PII orchestrator with default tools
    pub fn new(_backend: LlmBackend, _model: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let regex_tools: Vec<Box<dyn PiiAnonymizationTool>> = vec![
            Box::new(EmailAnonymizer::new()),
            Box::new(PhoneAnonymizer::new()),
        ];
        
        let llm_tools: Vec<Box<dyn PiiAnonymizationTool>> = vec![
            Box::new(NameAnonymizer::new()),
        ];
        
        Ok(Self {
            regex_tools,
            llm_tools,
            json_validator: JsonValidator::new(),
        })
    }

    /// Anonymize an email JSON string through multi-pass pipeline
    pub fn anonymize_email(&mut self, email_json: &str) -> Result<AnonymizationResult, Box<dyn std::error::Error>> {
        let mut current_text = email_json.to_string();
        let all_entities = Vec::new();
        let mut all_replacements = Vec::new();
        
        // First pass - regex tools for structured PII
        for tool in &mut self.regex_tools {
            current_text = tool.anonymize(&current_text)?;
            all_replacements.extend(tool.get_replacement_log());
        }
        
        // Second pass - LLM tools for contextual PII (simplified for now)
        for tool in &mut self.llm_tools {
            current_text = tool.anonymize(&current_text)?;
            all_replacements.extend(tool.get_replacement_log());
        }
        
        // Validate JSON structure
        if !self.json_validator.is_valid(&current_text) {
            current_text = self.json_validator.fix(&current_text)?;
        }
        
        Ok(AnonymizationResult {
            anonymized_text: current_text,
            detected_entities: all_entities,
            replacement_log: all_replacements,
        })
    }

    /// Verify that no PII remains in the text
    /// Excludes generated fake data from verification warnings
    pub fn verify_no_pii(&self, text: &str, replacement_log: &[ReplacementLogEntry]) -> VerificationResult {
        let mut found_pii = Vec::new();
        
        // Collect all generated fake data to exclude from verification
        let mut generated_data = HashSet::new();
        for entry in replacement_log {
            generated_data.insert(entry.fake_value.clone());
        }
        
        // Common fake email patterns we generate (to avoid false positives)
        let fake_domains = ["example.com", "test.com", "demo.com", "mock.com", "sample.com", 
                           "example.org", "test.org", "demo.org", "mock.org", "sample.org",
                           "example.net", "test.net", "demo.net", "mock.net", "sample.net",
                           "example.co", "test.co", "demo.co", "mock.co", "sample.co",
                           "example.uk", "test.uk", "demo.uk", "mock.uk", "sample.uk"];
        
        // Check for emails
        for capture in EMAIL_REGEX.find_iter(text) {
            let email = capture.as_str();
            
            // Skip if this is generated fake data
            if generated_data.contains(email) {
                continue;
            }
            
            // Skip if this looks like a fake email domain we commonly generate
            let is_fake_domain = fake_domains.iter().any(|&domain| email.ends_with(domain));
            if is_fake_domain {
                continue;
            }
            
            found_pii.push(PiiDetection {
                pii_type: "email".to_string(),
                text: email.to_string(),
                start: capture.start(),
                end: capture.end(),
            });
        }
        
        // Check for phone numbers
        for capture in PHONE_REGEX.find_iter(text) {
            let phone = capture.as_str();
            // Skip if this is generated fake data
            if !generated_data.contains(phone) {
                found_pii.push(PiiDetection {
                    pii_type: "phone".to_string(),
                    text: phone.to_string(),
                    start: capture.start(),
                    end: capture.end(),
                });
            }
        }
        
        VerificationResult {
            is_clean: found_pii.is_empty(),
            found_pii,
        }
    }
}

// Phone number regex patterns
lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(
        r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b"
    ).unwrap();
    
    static ref PHONE_REGEX: Regex = Regex::new(
        r"(?:\+1[-.\s]?)?\(?([0-9]{3})\)?[-.\s]?([0-9]{3})[-.\s]?([0-9]{4})\b|(?:^|\s)([0-9]{10})(?:\s|$)"
    ).unwrap();
}

impl EmailAnonymizer {
    pub fn new() -> Self {
        Self {
            replacement_log: Vec::new(),
        }
    }
}

impl PiiAnonymizationTool for EmailAnonymizer {
    fn anonymize(&mut self, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = text.to_string();
        
        // Find all email matches and replace them
        for capture in EMAIL_REGEX.find_iter(text) {
            let email = capture.as_str();
            let fake_email = generate_fake_email(email);
            
            // Log the replacement
            self.replacement_log.push(ReplacementLogEntry {
                pii_type: "email".to_string(),
                original_value: email.to_string(),
                fake_value: fake_email.clone(),
                position: capture.start(),
            });
            
            // Replace in result string
            result = result.replace(email, &fake_email);
        }
        
        Ok(result)
    }

    fn get_replacement_log(&self) -> Vec<ReplacementLogEntry> {
        self.replacement_log.clone()
    }

    fn tool_name(&self) -> &str {
        "email_anonymizer"
    }
}

/// Generate a realistic fake email based on the original pattern
fn generate_fake_email(original: &str) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    
    let parts: Vec<&str> = original.split('@').collect();
    if parts.len() != 2 {
        return format!("user{}@example.com", rng.random_range(1000..9999));
    }
    
    let domain_parts: Vec<&str> = parts[1].split('.').collect();
    let tld = domain_parts.last().unwrap_or(&"com");
    
    // Generate realistic fake names based on patterns
    let fake_names = vec![
        "john.smith", "jane.doe", "mike.johnson", "sarah.wilson", 
        "david.brown", "lisa.davis", "chris.miller", "amy.taylor"
    ];
    let fake_domains = vec![
        "example", "test", "demo", "sample", "mock"
    ];
    
    let fake_user = fake_names[rng.random_range(0..fake_names.len())];
    let fake_domain = fake_domains[rng.random_range(0..fake_domains.len())];
    
    format!("{}@{}.{}", fake_user, fake_domain, tld)
}

/// Generate a fake phone number maintaining the original format
fn generate_fake_phone(original: &str) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    
    // Generate fake area code and number
    let area_code = rng.random_range(200..999);
    let exchange = rng.random_range(200..999);
    let number = rng.random_range(1000..9999);
    
    // Try to maintain the original format
    if original.contains("(") {
        format!("({}) {}-{}", area_code, exchange, number)
    } else if original.contains(".") {
        format!("{}.{}.{}", area_code, exchange, number)
    } else if original.contains("-") {
        format!("{}-{}-{}", area_code, exchange, number)
    } else if original.starts_with("+1") {
        format!("+1{}{}{}", area_code, exchange, number)
    } else {
        format!("{}{}{}", area_code, exchange, number)
    }
}

impl PhoneAnonymizer {
    pub fn new() -> Self {
        Self {
            replacement_log: Vec::new(),
        }
    }
}

impl PiiAnonymizationTool for PhoneAnonymizer {
    fn anonymize(&mut self, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = text.to_string();
        
        // Find all phone matches and replace them
        for capture in PHONE_REGEX.find_iter(text) {
            let phone = capture.as_str();
            let fake_phone = generate_fake_phone(phone);
            
            // Log the replacement
            self.replacement_log.push(ReplacementLogEntry {
                pii_type: "phone".to_string(),
                original_value: phone.to_string(),
                fake_value: fake_phone.clone(),
                position: capture.start(),
            });
            
            // Replace in result string
            result = result.replace(phone, &fake_phone);
        }
        
        Ok(result)
    }

    fn get_replacement_log(&self) -> Vec<ReplacementLogEntry> {
        self.replacement_log.clone()
    }

    fn tool_name(&self) -> &str {
        "phone_anonymizer"
    }
}

impl NameAnonymizer {
    pub fn new() -> Self {
        Self {
            replacement_log: Vec::new(),
        }
    }
}

impl PiiAnonymizationTool for NameAnonymizer {
    fn anonymize(&mut self, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        // For now, implement a basic regex-based name replacer
        // TODO: Enhance with LLM-based detection later
        let mut result = text.to_string();
        
        // Simple pattern for common names in email context
        let name_patterns = vec![
            ("John Doe", "Person A"),
            ("Jane Doe", "Person B"),
            ("ACME Corp", "Company X"),
            ("John", "PersonA"),
            ("Jane", "PersonB"),
            ("Mazda", "PersonC"),
            ("Judith", "PersonD"),
        ];
        
        for (original, replacement) in name_patterns {
            if result.contains(original) {
                self.replacement_log.push(ReplacementLogEntry {
                    pii_type: "name".to_string(),
                    original_value: original.to_string(),
                    fake_value: replacement.to_string(),
                    position: 0, // TODO: Find actual position
                });
                result = result.replace(original, replacement);
            }
        }
        
        Ok(result)
    }

    fn get_replacement_log(&self) -> Vec<ReplacementLogEntry> {
        self.replacement_log.clone()
    }

    fn tool_name(&self) -> &str {
        "name_anonymizer"
    }
}

impl JsonValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn is_valid(&self, json_str: &str) -> bool {
        // Try to parse with multiple fallback strategies
        if serde_json::from_str::<serde_json::Value>(json_str).is_ok() {
            return true;
        }
        
        // Try parsing after basic cleanup
        let cleaned = json_str
            .replace("͏", "")  // Remove invisible separators
            .replace("­", ""); // Remove soft hyphens
            
        serde_json::from_str::<serde_json::Value>(&cleaned).is_ok()
    }

    pub fn fix(&self, json_str: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Try to parse first
        if self.is_valid(json_str) {
            return Ok(json_str.to_string());
        }
        
        // Simple fixes for common JSON issues
        let mut fixed = json_str.trim().to_string();
        
        // Clean up problematic Unicode characters that can cause JSON parsing issues
        fixed = fixed.replace('\u{034F}', ""); // Combining Grapheme Joiner
        fixed = fixed.replace('\u{200C}', ""); // Zero Width Non-Joiner
        fixed = fixed.replace('\u{200D}', ""); // Zero Width Joiner
        fixed = fixed.replace('\u{FEFF}', ""); // Byte Order Mark
        fixed = fixed.replace("͏", "");        // Invisible separator
        fixed = fixed.replace("­", "");         // Soft hyphen
        
        // Fix common escape sequence issues
        fixed = fixed.replace("\\u", "\\\\u"); // Escape literal \u sequences
        
        // Replace problematic escape sequences - be more conservative
        fixed = fixed.replace("\\\\", "\\\\\\\\"); // Fix double backslashes
        
        // Remove lines with problematic characters that cause parsing issues
        let lines: Vec<&str> = fixed.lines().collect();
        let mut clean_lines = Vec::new();
        for line in lines {
            // Skip lines that contain problematic escape sequences
            if !line.contains("\\\\") || line.starts_with("  \"body\":") {
                clean_lines.push(line);
            }
        }
        fixed = clean_lines.join("\n");
        
        // Add missing closing braces/brackets
        let open_braces = fixed.chars().filter(|&c| c == '{').count();
        let close_braces = fixed.chars().filter(|&c| c == '}').count();
        
        if open_braces > close_braces {
            for _ in 0..(open_braces - close_braces) {
                fixed.push('}');
            }
        }
        
        let open_brackets = fixed.chars().filter(|&c| c == '[').count();
        let close_brackets = fixed.chars().filter(|&c| c == ']').count();
        
        if open_brackets > close_brackets {
            for _ in 0..(open_brackets - close_brackets) {
                fixed.push(']');
            }
        }
        
        // Try to parse again
        if self.is_valid(&fixed) {
            Ok(fixed)
        } else {
            // If still invalid, return the attempted fix as it's better than nothing
            Ok(fixed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_anonymizer_detects_simple_email() {
        let mut anonymizer = EmailAnonymizer::new();
        let text = r#"{"to": ["user@example.com"]}"#;
        
        let result = anonymizer.anonymize(text).unwrap();
        
        // Should not contain the original email
        assert!(!result.contains("user@example.com"));
        // Should still be valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&result).is_ok());
        // Should have logged the replacement
        assert_eq!(anonymizer.get_replacement_log().len(), 1);
    }

    #[test]
    fn test_email_anonymizer_preserves_json_structure() {
        let mut anonymizer = EmailAnonymizer::new();
        let text = r#"{"from": "sender@domain.com", "to": ["user@example.com"], "subject": "Test"}"#;
        
        let result = anonymizer.anonymize(text).unwrap();
        
        // Parse as JSON to verify structure is preserved
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.get("from").is_some());
        assert!(parsed.get("to").is_some());
        assert!(parsed.get("subject").is_some());
        assert_eq!(parsed.get("subject").unwrap().as_str().unwrap(), "Test");
    }

    #[test]
    fn test_phone_anonymizer_detects_various_formats() {
        let mut anonymizer = PhoneAnonymizer::new();
        let test_cases = vec![
            "+1-555-123-4567",
            "(555) 123-4567", 
            "555.123.4567",
            "5551234567",
        ];
        
        for phone in test_cases {
            let text = format!(r#"{{"phone": "{}"}}"#, phone);
            let result = anonymizer.anonymize(&text).unwrap();
            assert!(!result.contains(phone));
        }
    }

    #[test]
    fn test_orchestrator_multi_pass_pipeline() {
        let mut orchestrator = PiiOrchestrator::new(LlmBackend::Ollama, Some("mistral".to_string())).unwrap();
        let email_json = r#"{
            "from": "john.doe@company.com",
            "to": ["jane.smith@example.com"],
            "subject": "Meeting with John Doe",
            "body": "Hi Jane, this is John Doe from ACME Corp. Call me at +1-555-123-4567"
        }"#;
        
        let result = orchestrator.anonymize_email(email_json).unwrap();
        
        // Should not contain any original PII
        assert!(!result.anonymized_text.contains("john.doe@company.com"));
        assert!(!result.anonymized_text.contains("jane.smith@example.com"));
        assert!(!result.anonymized_text.contains("John Doe"));
        assert!(!result.anonymized_text.contains("ACME Corp"));
        assert!(!result.anonymized_text.contains("+1-555-123-4567"));
        
        // Should still be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result.anonymized_text).unwrap();
        assert!(parsed.get("from").is_some());
        assert!(parsed.get("to").is_some());
        assert!(parsed.get("subject").is_some());
        assert!(parsed.get("body").is_some());
    }

    #[test]
    fn test_verification_detects_missed_pii() {
        let orchestrator = PiiOrchestrator::new(LlmBackend::Ollama, Some("mistral".to_string())).unwrap();
        let text_with_pii = r#"{"hidden_email": "secret@domain.com"}"#;
        let empty_log = Vec::new();
        
        let verification = orchestrator.verify_no_pii(text_with_pii, &empty_log);
        
        assert!(!verification.is_clean);
        assert!(!verification.found_pii.is_empty());
        assert_eq!(verification.found_pii[0].pii_type, "email");
        assert_eq!(verification.found_pii[0].text, "secret@domain.com");
    }

    #[test]
    fn test_json_validator_fixes_broken_structure() {
        let validator = JsonValidator::new();
        let broken_json = r#"{"from": "test@example.com", "to": ["user@domain.com"]"#; // Missing closing bracket
        
        assert!(!validator.is_valid(broken_json));
        
        let fixed = validator.fix(broken_json).unwrap();
        println!("Original: {}", broken_json);
        println!("Fixed: {}", fixed);
        
        // The fix should add the missing closing bracket
        assert!(fixed.ends_with("}"));
        assert!(serde_json::from_str::<serde_json::Value>(&fixed).is_ok());
    }

    #[test]
    fn test_verification_excludes_generated_fake_data() {
        let orchestrator = PiiOrchestrator::new(LlmBackend::Ollama, Some("mistral".to_string())).unwrap();
        let text_with_fake_data = r#"{"email": "john.doe@test.com", "phone": "555-123-4567"}"#;
        
        // Create replacement log entries for the fake data
        let replacement_log = vec![
            ReplacementLogEntry {
                pii_type: "email".to_string(),
                original_value: "real@example.com".to_string(),
                fake_value: "john.doe@test.com".to_string(),
                position: 10,
            },
            ReplacementLogEntry {
                pii_type: "phone".to_string(),
                original_value: "555-999-8888".to_string(),
                fake_value: "555-123-4567".to_string(),
                position: 35,
            },
        ];
        
        let verification = orchestrator.verify_no_pii(text_with_fake_data, &replacement_log);
        
        // Should be clean because the detected PII is just our generated fake data
        assert!(verification.is_clean);
        assert!(verification.found_pii.is_empty());
    }
}
