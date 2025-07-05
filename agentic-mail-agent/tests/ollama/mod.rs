//! Ollama tests - require local Ollama server
//! 
//! These tests require a local Ollama instance running with appropriate models.
//! They test LLM-dependent functionality like PII detection and classification.
//! 
//! Many of these tests can be "promoted" to unit tests using the mock Ollama
//! system. Run the recording tests first to capture LLM responses, then create
//! corresponding unit tests that replay the recorded responses.
//! 
//! Prerequisites:
//! - Install Ollama: https://ollama.ai/
//! - Run: ollama serve
//! - Pull required models: ollama pull llama3:8b
//! 
//! Recording workflow:
//! 1. Run: cargo test --test ollama record_individual_examples
//! 2. Run: cargo test --test ollama record_classifier_ground_truth_responses  
//! 3. Create unit tests using MockOllamaClassifier with recorded files
//! 
//! Run with: cargo test --test ollama -- --ignored

mod address_detector;
mod debug_llm_parsing;
mod integration_pii_anonymization;
mod test_100_percent_replacement;
mod test_classifier_ground_truth;
mod test_real_classifier_with_data;
mod test_response_recorder;