//! Unit tests - no external dependencies (57 tests)
//! 
//! These tests can run quickly in any environment without requiring
//! external services like Ollama or Gmail API access. Includes:
//! 
//! - Core functionality tests (action execution, labeling, PII)
//! - Mock Ollama classifier tests using recorded LLM responses
//! - Phone number detection and text processing
//! - Email field validation and workflow integration
//! 
//! Mock LLM tests use pre-recorded responses from real Ollama instances,
//! providing deterministic testing of LLM-dependent features without
//! requiring a live LLM server.
//! 
//! Run with: cargo test --test unit

mod integration_complete_workflow;
mod integration_full_email_fields;
mod integration_labeling;
mod phone_detector;
mod test_action_execution;
mod test_ground_truth_mock;
mod test_mock_ollama_classifier;
mod test_noise_detection_patterns;
mod unit_pii_architecture;
mod unit_test_data_utils;