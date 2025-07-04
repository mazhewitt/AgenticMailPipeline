//! Integration tests - require both Gmail and Ollama
//! 
//! These tests require both Gmail API access and local Ollama server.
//! They test end-to-end workflows combining real Gmail data with LLM processing.
//! 
//! Prerequisites:
//! - Gmail API credentials (see gmail tests)
//! - Local Ollama server (see ollama tests)
//! - Downloaded test data from Gmail API
//! 
//! Run with: cargo test --test integration

mod test_data_integration;
mod test_data_quality_assessment;