//! Configuration management for PII anonymization

use crate::anonymizer::types::LlmBackend;
use serde::Deserialize;
use std::fs;
use std::path::Path;

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
    pub fn new(
        backend: LlmBackend,
        model: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (model, openai_api_key) = match backend {
            LlmBackend::Ollama => (model.unwrap_or_else(|| "llama3:8b".to_string()), None),
            LlmBackend::OpenAI => {
                let api_key = Self::load_openai_key()?;
                (
                    model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
                    Some(api_key),
                )
            }
        };

        Ok(Self {
            backend,
            openai_api_key,
            model,
            temperature: 0.1, // Low temperature for consistent PII detection
            ollama_host: "http://localhost:11434".to_string(),
            llm_timeout_secs: 120,
        })
    }

    fn load_openai_key() -> Result<String, Box<dyn std::error::Error>> {
        let secrets_path = Path::new("../secrets/openai.json");
        if !secrets_path.exists() {
            return Err(
                "OpenAI API key not found. Please create ../secrets/openai.json with your API key."
                    .into(),
            );
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
