//! PII Detection and Anonymization Module
//! 
//! This module provides intelligent PII detection and replacement capabilities using:
//! 1. LLM-based PII entity detection with structured JSON output
//! 2. Rust-based replacement with fake but realistic data
//! 3. Auditability and consistency across the same email
//! 4. Fallback mechanisms for critical PII types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use tokio::time::timeout;
use std::time::Duration;
use langchain_rust::{
    llm::{openai::{OpenAI, OpenAIConfig}, ollama::client::Ollama},
    language_models::llm::LLM,
};

/// PII entity detected by the LLM
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
                (model.unwrap_or_else(|| "llama3.1:8b".to_string()), None)
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
            r#"You are a PII detection expert. Find all personally identifiable information (PII) in the following email text.

For each PII item found, provide:
- type: The category (name, email, phone, address, company, date, etc.)
- text: The exact text as it appears
- start: Character position where the PII starts (0-indexed)
- end: Character position where the PII ends (0-indexed, exclusive)

Return ONLY a JSON array with this exact format:
[
  {{"type": "name", "text": "John Doe", "start": 15, "end": 23}},
  {{"type": "email", "text": "john@example.com", "start": 45, "end": 61}}
]

Email text to analyze:
{}

JSON array:"#,
            text
        );
        
        let timeout_duration = Duration::from_secs(self.config.llm_timeout_secs);
        let response = timeout(timeout_duration, self.llm.invoke(&prompt)).await
            .map_err(|_| "LLM request timed out")?
            .map_err(|e| format!("LLM request failed: {}", e))?;
        
        // Parse JSON response
        let entities = self.parse_pii_response(&response)?;
        
        Ok(entities)
    }
    
    fn parse_pii_response(&self, response: &str) -> Result<Vec<PiiEntity>, Box<dyn std::error::Error>> {
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
            // First try to parse as is, assuming the LLM used the correct field names
            match serde_json::from_str::<Vec<PiiEntity>>(json_str) {
                Ok(entities) => return Ok(entities),
                Err(_) => {
                    // If that fails, try parsing as raw JSON and converting field names
                    let raw_entities: Result<Vec<serde_json::Value>, _> = serde_json::from_str(json_str);
                    match raw_entities {
                        Ok(raw_entities) => {
                            let mut entities = Vec::new();
                            for raw_entity in raw_entities {
                                if let Some(entity) = self.parse_single_entity(&raw_entity) {
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
        
        for ch in text.chars() {
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
    
    fn parse_single_entity(&self, raw_entity: &serde_json::Value) -> Option<PiiEntity> {
        let pii_type = raw_entity.get("type")
            .or_else(|| raw_entity.get("pii_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let text = raw_entity.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let start = raw_entity.get("start")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
            
        let end = raw_entity.get("end")
            .and_then(|v| v.as_u64())
            .unwrap_or((start + text.len()) as u64) as usize;
        
        // Only return valid entities (must have text)
        if !text.is_empty() {
            Some(PiiEntity {
                pii_type,
                text,
                start,
                end,
            })
        } else {
            None
        }
    }
}

/// PII replacement service with fake data generation
pub struct PiiReplacer {
    /// Consistent mapping of original values to fake values within a session
    replacement_cache: HashMap<String, String>,
    /// Audit log of all replacements
    replacement_log: Vec<ReplacementLogEntry>,
    /// Regex patterns for fallback PII detection
    fallback_patterns: HashMap<String, Regex>,
}

impl PiiReplacer {
    pub fn new() -> Self {
        let mut fallback_patterns = HashMap::new();
        
        // Common PII patterns for fallback
        fallback_patterns.insert(
            "email".to_string(),
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap()
        );
        fallback_patterns.insert(
            "phone".to_string(),
            Regex::new(r"\b(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b").unwrap()
        );
        
        Self {
            replacement_cache: HashMap::new(),
            replacement_log: Vec::new(),
            fallback_patterns,
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
                let current_text = &result[adjusted_start..adjusted_end];
                if current_text == entity.text {
                    // Replace the text
                    result.replace_range(adjusted_start..adjusted_end, &fake_value);
                    
                    // Update offset for next replacements
                    offset += fake_value.len() as i32 - entity.text.len() as i32;
                    
                    // Log the replacement
                    self.replacement_log.push(ReplacementLogEntry {
                        pii_type: entity.pii_type.clone(),
                        original_value: entity.text.clone(),
                        fake_value: fake_value.clone(),
                        position: adjusted_start,
                    });
                }
            }
        }
        
        Ok(result)
    }
    
    /// Replace PII with fallback regex patterns when LLM fails
    pub fn replace_pii_with_fallback(&mut self, text: &str, llm_entities: &[PiiEntity]) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = self.replace_pii(text, llm_entities)?;
        
        // Apply fallback patterns for critical PII types not caught by LLM
        let patterns: Vec<(String, Regex)> = self.fallback_patterns.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
            
        for (pii_type, pattern) in patterns {
            let mut matches = Vec::new();
            for mat in pattern.find_iter(&result) {
                matches.push((mat.start(), mat.end(), mat.as_str().to_string()));
            }
            
            for (start, end, matched_text) in matches.into_iter().rev() {
                // Skip if this was already handled by LLM
                if llm_entities.iter().any(|e| e.text == matched_text) {
                    continue;
                }
                
                let fake_value = self.generate_fake_value(&pii_type, &matched_text);
                result.replace_range(start..end, &fake_value);
                
                self.replacement_log.push(ReplacementLogEntry {
                    pii_type: pii_type.clone(),
                    original_value: matched_text,
                    fake_value: fake_value.clone(),
                    position: start,
                });
            }
        }
        
        Ok(result)
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
        // Step 1: Detect PII using LLM
        let detected_entities = self.detector.detect_pii(text).await?;
        
        // Step 2: Replace PII with fake data (including fallback)
        let anonymized_text = self.replacer.replace_pii_with_fallback(text, &detected_entities)?;
        
        // Step 3: Return comprehensive result
        Ok(AnonymizationResult {
            anonymized_text,
            detected_entities,
            replacement_log: self.replacer.get_replacement_log().to_vec(),
        })
    }
}
