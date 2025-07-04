//! Unit tests - no external dependencies
//! 
//! These tests can run quickly in any environment without requiring
//! external services like Ollama or Gmail API access.
//! 
//! Run with: cargo test --test unit

mod integration_complete_workflow;
mod integration_full_email_fields;
mod integration_labeling;
mod phone_detector;
mod test_action_execution;
mod test_noise_detection_patterns;
mod unit_pii_architecture;
mod unit_test_data_utils;