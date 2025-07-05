//! LLM client for OpenAI and Ollama integration

use crate::anonymizer::config::AnonymizationConfig;
use crate::anonymizer::types::LlmBackend;
use langchain_rust::{
    llm::{openai::{OpenAI, OpenAIConfig}, ollama::client::Ollama},
    language_models::llm::LLM,
};
use tokio::time::{timeout, Duration};

/// LLM client wrapper for different backends
pub struct LlmClient {
    llm: Box<dyn LLM>,
    timeout_duration: Duration,
}

impl LlmClient {
    /// Create a new LLM client with the given configuration
    pub async fn new(config: &AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
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
                        return Err(format!("Failed to connect to OpenAI API: {e}").into());
                    }
                }
            }
        };
        
        Ok(Self { 
            llm, 
            timeout_duration: Duration::from_secs(config.llm_timeout_secs),
        })
    }
    
    /// Invoke the LLM with a prompt and return the response
    pub async fn invoke(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        eprintln!("Sending prompt to LLM: {prompt}");
        
        let response = timeout(self.timeout_duration, self.llm.invoke(prompt)).await
            .map_err(|_| "LLM request timed out")?
            .map_err(|e| format!("LLM request failed: {e}"))?;
        
        #[cfg(debug_assertions)]
        eprintln!("LLM response: {response}");
        
        Ok(response)
    }
}