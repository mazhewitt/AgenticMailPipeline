//! Ollama tests - require local Ollama server
//! 
//! These tests require a local Ollama instance running with appropriate models.
//! They test LLM-dependent functionality like PII detection and classification.
//! 
//! Prerequisites:
//! - Install Ollama: https://ollama.ai/
//! - Run: ollama serve
//! - Pull required models: ollama pull llama3:8b
//! 
//! Run with: cargo test --test ollama

mod address_detector;
mod debug_llm_parsing;
mod integration_pii_anonymization;
mod test_100_percent_replacement;
mod test_classifier_ground_truth;
mod test_real_classifier_with_data;