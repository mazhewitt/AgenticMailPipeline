//! Core data structures for PII anonymization

use serde::{Deserialize, Serialize};

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