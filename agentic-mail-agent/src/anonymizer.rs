//! PII Detection and Anonymization Module
//! 
//! This module provides intelligent PII detection and replacement capabilities using:
//! 1. LLM-based PII entity detection with structured JSON output
//! 2. Rust-based replacement with fake but realistic data
//! 3. Auditability and consistency across the same email
//! 4. LLM-only detection - no fallback, email fails if LLM fails

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::timeout;
use std::time::Duration;
use langchain_rust::{
    llm::{openai::{OpenAI, OpenAIConfig}, ollama::client::Ollama},
    language_models::llm::LLM,
};

/// Find the nearest character boundary for safe string slicing
fn find_char_boundary_helper(text: &str, position: usize) -> usize {
    if position >= text.len() {
        return text.len();
    }
    
    // Find the nearest character boundary at or before the given position
    let mut pos = position;
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// PII entity detected by the LLM (simplified - no positions)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmPiiEntity {
    /// Type of PII (e.g., "name", "email", "phone", "address", etc.)
    #[serde(alias = "type")]
    pub pii_type: String,
    /// The exact text as it appears in the original content
    pub text: String,
}

/// PII entity with positions calculated in Rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PiiEntity {
    /// Type of PII (e.g., "name", "email", "phone", "address", etc.)
    pub pii_type: String,
    /// The exact text as it appears in the original content
    pub text: String,
    /// Start character index in the original text
    pub start: usize,
    /// End character index in the original text
    pub end: usize,
}

/// Audit log entry for a PII replacement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementLogEntry {
    /// Type of PII that was replaced
    pub pii_type: String,
    /// The original sensitive value
    pub original_value: String,
    /// The fake value used as replacement
    pub fake_value: String,
    /// Position in text where replacement occurred
    pub position: usize,
}

/// Result of email anonymization
#[derive(Debug, Clone)]
pub struct AnonymizationResult {
    /// The anonymized text with all PII replaced
    pub anonymized_text: String,
    /// List of PII entities that were detected
    pub detected_entities: Vec<PiiEntity>,
    /// Audit log of all replacements made
    pub replacement_log: Vec<ReplacementLogEntry>,
}

/// LLM backend configuration
#[derive(Debug, Clone, PartialEq)]
pub enum LlmBackend {
    Ollama,
    OpenAI,
}

/// Configuration for the anonymization pipeline
#[derive(Debug, Clone)]
pub struct AnonymizationConfig {
    /// LLM backend to use for PII detection
    pub backend: LlmBackend,
    /// OpenAI API key (required for OpenAI backend)
    pub openai_api_key: Option<String>,
    /// Model name to use
    pub model: String,
    /// Temperature for LLM generation
    pub temperature: f64,
    /// Ollama host URL
    pub ollama_host: String,
    /// Timeout for LLM requests in seconds
    pub llm_timeout_secs: u64,
}

impl AnonymizationConfig {
    pub fn new(backend: LlmBackend, model: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let (model, openai_api_key) = match backend {
            LlmBackend::Ollama => {
                (model.unwrap_or_else(|| "llama3:8b".to_string()), None)
            }
            LlmBackend::OpenAI => {
                let api_key = Self::load_openai_key()?;
                (model.unwrap_or_else(|| "gpt-4o-mini".to_string()), Some(api_key))
            }
        };
        
        Ok(Self {
            backend,
            openai_api_key,
            model,
            temperature: 0.1, // Low temperature for consistent PII detection
            ollama_host: "http://localhost:11434".to_string(),
            llm_timeout_secs: 60,
        })
    }
    
    fn load_openai_key() -> Result<String, Box<dyn std::error::Error>> {
        use std::path::Path;
        use std::fs;
        
        let secrets_path = Path::new("../secrets/openai.json");
        if !secrets_path.exists() {
            return Err("OpenAI API key not found. Please create ../secrets/openai.json with your API key.".into());
        }
        
        #[derive(Deserialize)]
        struct SecretsFile {
            openai_api_key: String,
        }
        
        let secrets_content = fs::read_to_string(secrets_path)?;
        let secrets: SecretsFile = serde_json::from_str(&secrets_content)?;
        
        Ok(secrets.openai_api_key)
    }
}

/// PII detection service using LLM
pub struct PiiDetector {
    llm: Box<dyn LLM>,
    config: AnonymizationConfig,
}

impl PiiDetector {
    pub async fn new(config: AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let llm: Box<dyn LLM> = match config.backend {
            LlmBackend::Ollama => {
                let ollama = Ollama::default().with_model(&config.model);
                
                // Test the connection
                match ollama.invoke("Hello").await {
                    Ok(_) => Box::new(ollama),
                    Err(e) => {
                        return Err(format!(
                            "Failed to connect to Ollama at {}: {}. Make sure Ollama is running and the model '{}' is available.", 
                            config.ollama_host, e, config.model
                        ).into());
                    }
                }
            }
            LlmBackend::OpenAI => {
                let api_key = config.openai_api_key.as_ref().unwrap();
                let openai_config = OpenAIConfig::default().with_api_key(api_key);
                let openai = OpenAI::new(openai_config).with_model(&config.model);
                
                // Test the connection
                match openai.invoke("Hello").await {
                    Ok(_) => Box::new(openai),
                    Err(e) => {
                        return Err(format!("Failed to connect to OpenAI API: {}", e).into());
                    }
                }
            }
        };
        
        Ok(Self { llm, config })
    }
    
    /// Use LLM to detect PII entities in the given text
    pub async fn detect_pii(&self, text: &str) -> Result<Vec<PiiEntity>, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"SYSTEM: You are a PII extraction robot. You ONLY output JSON arrays. NO explanations, NO markdown, NO conversations.

TASK: Extract ALL personally identifiable information (PII) from email text. Return ONLY valid JSON array.

CRITICAL: You must find ALL instances of PII. Missing any PII will cause the email to fail processing.

FIND ALL of these PII types:
- Full names of people (first/last names, titles)
- Email addresses (ALL email addresses including domains)
- Phone numbers (any format)
- Physical addresses (street, city, state, zip)
- Company/organization names
- Usernames and handles
- Domain names and URLs
- Financial information
- Account numbers and IDs

SPECIFIC EXAMPLES - YOU MUST DETECT ALL OF THESE:

1. Names and Titles:
   - "John Smith", "Mary Johnson", "Dr. Williams", "Ms. Chen", "CEO Sarah Brown"

2. Email Addresses (CRITICAL - find ALL):
   - "john@company.com", "user@gmail.com", "contact@domain.org"
   - "notifications@github.com", "hello@news.smood.ch"
   - In headers: "John <john@test.com>", "support@example.com"

3. Phone Numbers (ALL formats):
   - "+1-555-123-4567", "(555) 123-4567", "555.123.4567", "1234567890"
   - "555-1234", "123 456 7890"

4. Usernames/Handles:
   - "mazhewitt", "user123", "@username", "john_doe"

5. Company/Organization Names:
   - "Google", "Microsoft Corp", "Acme Inc", "GitHub", "Smood"
   - Domain companies: "example.com", "github.com", "smood.ch"

6. Addresses:
   - "123 Main St", "456 Oak Avenue, Springfield", "New York, NY 10001"

7. URLs and Domains:
   - "https://github.com/user/repo", "www.example.com", "smood.ch"

8. Financial Info:
   - "4111-1111-1111-1111", "Account #12345", "$1,000"

EXAMPLE INPUT:
"From: John Smith <john@github.com>
To: mazhewitt@gmail.com
Subject: [user/repo] Build failed
Hi mazhewitt, your build at https://github.com/user/repo failed.
Call support at +1-555-123-4567."

EXAMPLE OUTPUT (MUST find ALL of these):
[
  {{"type": "name", "text": "John Smith"}},
  {{"type": "email", "text": "john@github.com"}},
  {{"type": "email", "text": "mazhewitt@gmail.com"}},
  {{"type": "username", "text": "mazhewitt"}},
  {{"type": "username", "text": "user"}},
  {{"type": "company", "text": "github.com"}},
  {{"type": "url", "text": "https://github.com/user/repo"}},
  {{"type": "phone", "text": "+1-555-123-4567"}}
]

INSTRUCTIONS:
- Extract EXACT text as it appears
- Include partial emails/domains (e.g., "@github.com" if in an email)
- Find usernames even without @ symbol
- Detect company names in domains and URLs
- Be extremely thorough - missing PII fails the email

WARNING: OUTPUT MUST BE VALID JSON ARRAY ONLY. NO TEXT. NO EXPLANATIONS. NO MARKDOWN.
IF YOU OUTPUT ANYTHING OTHER THAN A JSON ARRAY, THE SYSTEM WILL FAIL.

EMAIL TEXT:
{}

JSON:"#,
            text
        );
        
        let timeout_duration = Duration::from_secs(self.config.llm_timeout_secs);
        let response = timeout(timeout_duration, self.llm.invoke(&prompt)).await
            .map_err(|_| "LLM request timed out")?
            .map_err(|e| format!("LLM request failed: {}", e))?;
        
        // Parse JSON response
        let llm_entities = self.parse_pii_response(&response)?;
        
        // Find all positions for each detected PII text in Rust
        let entities = self.find_all_positions(text, llm_entities);
        
        Ok(entities)
    }
    
    /// Find all positions of detected PII text in the content
    fn find_all_positions(&self, text: &str, llm_entities: Vec<LlmPiiEntity>) -> Vec<PiiEntity> {
        // Safety limit: prevent processing extremely large texts
        if text.len() > 1_000_000 {
            eprintln!("Warning: Text too large ({} chars), truncating to 1MB", text.len());
            let truncated_text = &text[..1_000_000];
            return self.find_all_positions_safe(truncated_text, llm_entities);
        }
        
        // Process with safety limits
        self.find_all_positions_safe(text, llm_entities)
    }
    
    /// Safe version with bounds checking and iteration limits
    fn find_all_positions_safe(&self, text: &str, llm_entities: Vec<LlmPiiEntity>) -> Vec<PiiEntity> {
        let mut entities = Vec::new();
        let max_iterations_per_entity = 1000; // Prevent infinite loops
        
        // First, find all positions for each detected PII text
        for llm_entity in llm_entities {
            let mut start = 0;
            let mut iteration_count = 0;
            
            while start < text.len() && iteration_count < max_iterations_per_entity {
                iteration_count += 1;
                
                // Ensure we're at a character boundary
                let safe_start = find_char_boundary_helper(text, start);
                if safe_start >= text.len() {
                    break;
                }
                
                if let Some(pos) = text[safe_start..].find(&llm_entity.text) {
                    let actual_start = safe_start + pos;
                    let actual_end = actual_start + llm_entity.text.len();
                    
                    // Bounds check
                    if actual_end <= text.len() {
                        entities.push(PiiEntity {
                            pii_type: llm_entity.pii_type.clone(),
                            text: llm_entity.text.clone(),
                            start: actual_start,
                            end: actual_end,
                        });
                    }
                    
                    // Move past this occurrence, ensuring we make progress
                    let next_start = actual_start + 1;
                    if next_start <= start {
                        // Safety: ensure we always make progress
                        break;
                    }
                    start = next_start;
                } else {
                    break; // No more occurrences found
                }
            }
            
            if iteration_count >= max_iterations_per_entity {
                eprintln!("Warning: Hit iteration limit for PII text: {}", llm_entity.text);
            }
        }
        
        // Sort entities by start position, then by length (longer first)
        entities.sort_by(|a, b| {
            match a.start.cmp(&b.start) {
                std::cmp::Ordering::Equal => b.text.len().cmp(&a.text.len()), // Longer text first
                other => other,
            }
        });
        
        // Remove overlapping entities, preferring longer and more specific ones
        let mut deduplicated = Vec::new();
        
        for entity in entities {
            let mut should_add = true;
            
            // Check for overlaps with existing entities
            for existing in &deduplicated {
                if self.entities_overlap(&entity, existing) {
                    // Determine which entity to keep
                    if self.should_prefer_existing(existing, &entity) {
                        should_add = false;
                        break;
                    } else {
                        // Remove the existing entity and add this one
                        // We'll handle this by continuing and using a second pass
                    }
                }
            }
            
            if should_add {
                // Remove any existing entities that this one should replace
                deduplicated.retain(|existing| {
                    !self.entities_overlap(&entity, existing) || self.should_prefer_existing(existing, &entity)
                });
                deduplicated.push(entity);
            }
        }
        
        // Sort final entities by start position for consistent processing
        deduplicated.sort_by_key(|e| e.start);
        deduplicated
    }
    
    /// Check if two PII entities overlap in text positions
    fn entities_overlap(&self, a: &PiiEntity, b: &PiiEntity) -> bool {
        // Two entities overlap if one starts before the other ends
        !(a.end <= b.start || b.end <= a.start)
    }
    
    /// Determine which entity to prefer when there's an overlap
    fn should_prefer_existing(&self, existing: &PiiEntity, new: &PiiEntity) -> bool {
        // If one completely contains the other, prefer the longer one
        if existing.start <= new.start && existing.end >= new.end {
            return true; // existing contains new, keep existing
        }
        if new.start <= existing.start && new.end >= existing.end {
            return false; // new contains existing, prefer new
        }
        
        // If they're different lengths, prefer longer
        if existing.text.len() != new.text.len() {
            return existing.text.len() > new.text.len();
        }
        
        // If same length, prefer more specific type
        match (existing.pii_type.as_str(), new.pii_type.as_str()) {
            ("email", "username") => true,  // email is more specific than username
            ("username", "email") => false,
            ("email", "company") => true,   // full email is more specific than just company
            ("company", "email") => false,
            ("name", "title") => true,      // full name is more specific than title
            ("title", "name") => false,
            ("email", "url") => true,       // email is more specific than url
            ("url", "email") => false,
            _ => true, // Keep existing by default
        }
    }
    
    fn parse_pii_response(&self, response: &str) -> Result<Vec<LlmPiiEntity>, Box<dyn std::error::Error>> {
        // Clean the response - sometimes LLMs add markdown or extra text
        let mut cleaned_response = response.trim();
        
        // Remove common LLM prefixes
        let prefixes = ["JSON output:", "Here's the JSON:", "```json", "```", "JSON array:"];
        for prefix in &prefixes {
            if cleaned_response.starts_with(prefix) {
                cleaned_response = cleaned_response[prefix.len()..].trim();
            }
        }
        
        // Remove ending markers
        if cleaned_response.ends_with("```") {
            cleaned_response = cleaned_response.trim_end_matches("```").trim();
        }
        
        // Find the JSON array - look for complete arrays
        let json_arrays = self.extract_json_arrays(cleaned_response);
        
        for json_str in &json_arrays {
            // Clean JSON comments (LLMs sometimes add them)
            let cleaned_json = self.remove_json_comments(json_str);
            
            // First try to parse as LlmPiiEntity, assuming the LLM used the correct field names
            match serde_json::from_str::<Vec<LlmPiiEntity>>(&cleaned_json) {
                Ok(entities) => return Ok(entities),
                Err(_) => {
                    // If that fails, try parsing as raw JSON and converting field names
                    let raw_entities: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&cleaned_json);
                    match raw_entities {
                        Ok(raw_entities) => {
                            let mut entities = Vec::new();
                            for raw_entity in raw_entities {
                                if let Some(entity) = self.parse_single_llm_entity(&raw_entity) {
                                    entities.push(entity);
                                }
                            }
                            if !entities.is_empty() {
                                return Ok(entities);
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
        
        Err(format!("Failed to parse any valid JSON from LLM response. Response was: {}", cleaned_response).into())
    }
    
    fn extract_json_arrays(&self, text: &str) -> Vec<String> {
        let mut arrays = Vec::new();
        let mut current_array = String::new();
        let mut bracket_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut in_array = false;
        
        // Safety limits to prevent memory explosion
        let max_response_length = 100_000; // 100KB limit
        let max_array_length = 50_000; // 50KB per array
        let max_arrays = 10; // Maximum number of arrays to extract
        
        let safe_text = if text.len() > max_response_length {
            eprintln!("Warning: LLM response too large ({} chars), truncating", text.len());
            &text[..max_response_length]
        } else {
            text
        };
        
        for ch in safe_text.chars() {
            // Safety check: prevent arrays from growing too large
            if current_array.len() > max_array_length {
                eprintln!("Warning: JSON array too large, truncating");
                if in_array {
                    // Try to close the array and save what we have
                    current_array.push(']');
                    arrays.push(current_array.clone());
                    in_array = false;
                    current_array.clear();
                    bracket_count = 0;
                }
                break;
            }
            
            // Safety check: prevent too many arrays
            if arrays.len() >= max_arrays {
                eprintln!("Warning: Too many JSON arrays found, stopping extraction");
                break;
            }
            if escape_next {
                if in_array {
                    current_array.push(ch);
                }
                escape_next = false;
                continue;
            }
            
            if ch == '\\' && in_string {
                if in_array {
                    current_array.push(ch);
                }
                escape_next = true;
                continue;
            }
            
            if ch == '"' && !escape_next {
                in_string = !in_string;
                if in_array {
                    current_array.push(ch);
                }
                continue;
            }
            
            if !in_string {
                if ch == '[' {
                    if bracket_count == 0 {
                        in_array = true;
                        current_array.clear();
                    }
                    bracket_count += 1;
                    if in_array {
                        current_array.push(ch);
                    }
                } else if ch == ']' {
                    if in_array {
                        current_array.push(ch);
                    }
                    bracket_count -= 1;
                    if bracket_count == 0 && in_array {
                        arrays.push(current_array.clone());
                        in_array = false;
                        current_array.clear();
                    }
                } else if in_array {
                    current_array.push(ch);
                }
            } else if in_array {
                current_array.push(ch);
            }
        }
        
        arrays
    }
    
    fn parse_single_llm_entity(&self, raw_entity: &serde_json::Value) -> Option<LlmPiiEntity> {
        let pii_type = raw_entity.get("type")
            .or_else(|| raw_entity.get("pii_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let text = raw_entity.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        // Only return valid entities (must have text)
        if !text.is_empty() {
            Some(LlmPiiEntity {
                pii_type,
                text,
            })
        } else {
            None
        }
    }
    
    /// Remove JSON comments that LLMs sometimes add
    fn remove_json_comments(&self, json_str: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut escape_next = false;
        let mut chars = json_str.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if escape_next {
                result.push(ch);
                escape_next = false;
                continue;
            }
            
            if ch == '\\' && in_string {
                result.push(ch);
                escape_next = true;
                continue;
            }
            
            if ch == '"' {
                in_string = !in_string;
                result.push(ch);
                continue;
            }
            
            if !in_string && ch == '/' {
                if let Some(&'/') = chars.peek() {
                    // Skip line comment
                    chars.next(); // consume second '/'
                    while let Some(next_ch) = chars.next() {
                        if next_ch == '\n' || next_ch == '\r' {
                            result.push(next_ch);
                            break;
                        }
                    }
                    continue;
                } else if let Some(&'*') = chars.peek() {
                    // Skip block comment
                    chars.next(); // consume '*'
                    while let Some(next_ch) = chars.next() {
                        if next_ch == '*' {
                            if let Some(&'/') = chars.peek() {
                                chars.next(); // consume '/'
                                break;
                            }
                        }
                    }
                    continue;
                }
            }
            
            result.push(ch);
        }
        
        result
    }
}

/// PII replacement service with fake data generation
pub struct PiiReplacer {
    /// Consistent mapping of original values to fake values within a session
    replacement_cache: HashMap<String, String>,
    /// Audit log of all replacements
    replacement_log: Vec<ReplacementLogEntry>,
}

impl PiiReplacer {
    pub fn new() -> Self {
        Self {
            replacement_cache: HashMap::new(),
            replacement_log: Vec::new(),
        }
    }
    
    /// Replace PII entities in text with fake data
    pub fn replace_pii(&mut self, text: &str, entities: &[PiiEntity]) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = text.to_string();
        let mut offset = 0i32; // Track how text length changes due to replacements
        
        // Sort entities by start position to process in order
        let mut sorted_entities = entities.to_vec();
        sorted_entities.sort_by_key(|e| e.start);
        
        for entity in sorted_entities {
            let fake_value = self.generate_fake_value(&entity.pii_type, &entity.text);
            
            // Calculate adjusted positions due to previous replacements
            let adjusted_start = ((entity.start as i32) + offset) as usize;
            let adjusted_end = ((entity.end as i32) + offset) as usize;
            
            // Verify the text matches what we expect
            if adjusted_start < result.len() && adjusted_end <= result.len() {
                // Ensure we're slicing on character boundaries
                let safe_start = self.find_char_boundary(&result, adjusted_start);
                let safe_end = self.find_char_boundary(&result, adjusted_end);
                
                if safe_start < safe_end && safe_end <= result.len() {
                    let current_text = &result[safe_start..safe_end];
                    
                    // Check if the text contains what we're looking for (fuzzy match due to character boundary adjustments)
                    if current_text.contains(&entity.text) || entity.text.contains(current_text) || current_text == entity.text {
                        // Replace the text
                        result.replace_range(safe_start..safe_end, &fake_value);
                        
                        // Update offset for next replacements
                        offset += fake_value.len() as i32 - (safe_end - safe_start) as i32;
                        
                        // Log the replacement
                        self.replacement_log.push(ReplacementLogEntry {
                            pii_type: entity.pii_type.clone(),
                            original_value: entity.text.clone(),
                            fake_value: fake_value.clone(),
                            position: safe_start,
                        });
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// Replace PII entities in text with fake data (LLM-only, no fallback)
    pub fn replace_pii_with_fallback(&mut self, text: &str, llm_entities: &[PiiEntity]) -> Result<String, Box<dyn std::error::Error>> {
        // No fallback - LLM detection only. If LLM fails, the email should fail anonymization.
        self.replace_pii(text, llm_entities)
    }
    
    /// Generate a fake value for the given PII type, maintaining consistency
    fn generate_fake_value(&mut self, pii_type: &str, original_value: &str) -> String {
        // Check cache for consistency
        if let Some(cached) = self.replacement_cache.get(original_value) {
            return cached.clone();
        }
        
        let fake_value = match pii_type.to_lowercase().as_str() {
            "name" => self.generate_fake_name(),
            "email" => self.generate_fake_email(original_value),
            "phone" => self.generate_fake_phone(),
            "address" => self.generate_fake_address(),
            "company" => self.generate_fake_company(),
            _ => format!("[REDACTED_{}]", pii_type.to_uppercase()),
        };
        
        // Cache for consistency
        self.replacement_cache.insert(original_value.to_string(), fake_value.clone());
        
        fake_value
    }
    
    fn generate_fake_name(&self) -> String {
        let first_names = ["Alex", "Jordan", "Taylor", "Casey", "Morgan", "Riley", "Avery", "Cameron"];
        let last_names = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis"];
        
        let first_idx = (self.replacement_cache.len() * 7) % first_names.len();
        let last_idx = (self.replacement_cache.len() * 13) % last_names.len();
        
        format!("{} {}", first_names[first_idx], last_names[last_idx])
    }
    
    fn generate_fake_email(&self, original: &str) -> String {
        // Preserve the domain structure but anonymize
        if let Some(at_pos) = original.find('@') {
            let domain = &original[at_pos + 1..];
            let fake_username = "user".to_string() + &(self.replacement_cache.len() + 1).to_string();
            
            // If it's a common domain, keep it; otherwise anonymize
            if domain.ends_with(".com") || domain.ends_with(".org") || domain.ends_with(".edu") {
                format!("{}@example.com", fake_username)
            } else {
                format!("{}@{}", fake_username, domain)
            }
        } else {
            format!("user{}@example.com", self.replacement_cache.len() + 1)
        }
    }
    
    fn generate_fake_phone(&self) -> String {
        let area_codes = ["555", "123", "456", "789"];
        let area_idx = (self.replacement_cache.len() * 11) % area_codes.len();
        format!("({}) {}-{}", 
            area_codes[area_idx],
            1000 + (self.replacement_cache.len() % 900),
            1000 + ((self.replacement_cache.len() * 17) % 9000)
        )
    }
    
    fn generate_fake_address(&self) -> String {
        let streets = ["Main Street", "Oak Avenue", "First Street", "Park Avenue", "Elm Street"];
        let cities = ["Springfield", "Riverside", "Franklin", "Georgetown", "Clinton"];
        let states = ["CA", "NY", "TX", "FL", "WA"];
        
        let street_idx = (self.replacement_cache.len() * 19) % streets.len();
        let city_idx = (self.replacement_cache.len() * 23) % cities.len();
        let state_idx = (self.replacement_cache.len() * 29) % states.len();
        let number = 100 + (self.replacement_cache.len() % 900);
        
        format!("{} {}, {}, {} {}", 
            number, streets[street_idx], cities[city_idx], states[state_idx],
            10000 + (self.replacement_cache.len() % 90000)
        )
    }
    
    fn generate_fake_company(&self) -> String {
        let company_names = ["TechCorp", "DataSystems", "InfoTech", "GlobalSoft", "NextGen Solutions"];
        let idx = (self.replacement_cache.len() * 31) % company_names.len();
        company_names[idx].to_string()
    }
    
    /// Get the replacement log for auditing
    pub fn get_replacement_log(&self) -> &[ReplacementLogEntry] {
        &self.replacement_log
    }
    
    /// Clear the replacement log (useful when processing multiple emails)
    pub fn clear_replacement_log(&mut self) {
        self.replacement_log.clear();
    }
    
    /// Find the nearest character boundary for safe string slicing
    fn find_char_boundary(&self, text: &str, position: usize) -> usize {
        if position >= text.len() {
            return text.len();
        }
        
        // If we're already on a character boundary, return as-is
        if text.is_char_boundary(position) {
            return position;
        }
        
        // Search backwards for the nearest character boundary
        for i in (0..=position).rev() {
            if text.is_char_boundary(i) {
                return i;
            }
        }
        
        // Fallback to the start if somehow we can't find a boundary
        0
    }
}

/// Complete anonymization pipeline
pub struct AnonymizationPipeline {
    detector: PiiDetector,
    replacer: PiiReplacer,
}

impl AnonymizationPipeline {
    pub async fn new(config: AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let detector = PiiDetector::new(config).await?;
        let replacer = PiiReplacer::new();
        
        Ok(Self { detector, replacer })
    }
    
    /// Anonymize an email text end-to-end
    pub async fn anonymize_email_text(&mut self, text: &str) -> Result<AnonymizationResult, Box<dyn std::error::Error>> {
        // Clear any previous replacement log to avoid contamination
        self.replacer.clear_replacement_log();
        
        // Step 1: Detect PII using LLM
        let detected_entities = self.detector.detect_pii(text).await?;
        
        // Step 2: Replace PII with fake data (LLM-only, no fallback)
        let anonymized_text = self.replacer.replace_pii_with_fallback(text, &detected_entities)?;
        
        // Step 3: Return comprehensive result
        Ok(AnonymizationResult {
            anonymized_text,
            detected_entities,
            replacement_log: self.replacer.get_replacement_log().to_vec(),
        })
    }
}
