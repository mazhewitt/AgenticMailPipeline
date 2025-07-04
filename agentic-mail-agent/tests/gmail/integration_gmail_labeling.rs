//! Integration tests for Gmail labeling functionality
//! 
//! These tests require real Gmail credentials and will be ignored by default.
//! To run them, ensure GMAIL_CLIENT_SECRET_JSON and GMAIL_TOKEN_JSON environment variables are set
//! and use: cargo test --test integration_gmail_labeling -- --ignored

use agentic_mail_agent::labeler::{GmailLabeler, EmailLabeler, LabelingError};

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_gmail_labeler_create_and_apply_label() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // This test requires real Gmail credentials
    let labeler = GmailLabeler::from_env().await
        .expect("Gmail labeler should initialize with valid credentials");
    
    // Create a test message ID (this would be a real Gmail message ID in practice)
    let test_message_id = "test-message-123";
    let test_label = "AGENT_TEST_LABEL";
    
    // Apply the label
    let result = labeler.apply_label(test_message_id, test_label).await
        .expect("Labeling should succeed");
    
    assert_eq!(result.message_id, test_message_id);
    assert_eq!(result.label, test_label);
    
    // Test idempotency - applying same label again should not error
    let result2 = labeler.apply_label(test_message_id, test_label).await
        .expect("Second labeling should also succeed");
    
    assert!(!result2.created_new_label); // Should not create new label second time
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_gmail_labeler_ensure_label_exists() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    let labeler = GmailLabeler::from_env().await
        .expect("Gmail labeler should initialize with valid credentials");
    
    let test_label = "AGENT_INTEGRATION_TEST";
    
    // Ensure label exists
    let label_id = labeler.ensure_label_exists(test_label).await
        .expect("Label creation should succeed");
    
    assert!(!label_id.is_empty());
    
    // Calling again should return same label ID
    let label_id2 = labeler.ensure_label_exists(test_label).await
        .expect("Second label creation should also succeed");
    
    assert_eq!(label_id, label_id2);
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_gmail_labeler_invalid_message_id() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    let labeler = GmailLabeler::from_env().await
        .expect("Gmail labeler should initialize with valid credentials");
    
    let invalid_message_id = "invalid-message-id-12345";
    let test_label = "AGENT_TEST";
    
    // This should fail with Gmail API error
    let result = labeler.apply_label(invalid_message_id, test_label).await;
    
    match result {
        Err(LabelingError::GmailApi { .. }) => {
            // Expected - invalid message ID should cause Gmail API error
        }
        Err(other) => {
            panic!("Expected GmailApi error, got: {:?}", other);
        }
        Ok(_) => {
            panic!("Expected error for invalid message ID, but operation succeeded");
        }
    }
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_gmail_labeler_category_mapping() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    let labeler = GmailLabeler::from_env().await
        .expect("Gmail labeler should initialize with valid credentials");
    
    // Test category to label mapping
    assert_eq!(labeler.get_label_for_category("work"), "AGENT_WORK");
    assert_eq!(labeler.get_label_for_category("personal"), "AGENT_PERSONAL");
    assert_eq!(labeler.get_label_for_category("spam"), "AGENT_SPAM");
    assert_eq!(labeler.get_label_for_category("urgent"), "AGENT_URGENT");
    assert_eq!(labeler.get_label_for_category("promotional"), "AGENT_PROMOTIONAL");
    assert_eq!(labeler.get_label_for_category("newsletter"), "AGENT_NEWSLETTER");
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_gmail_labeler_without_credentials() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // Temporarily unset environment variables
    let original_secret = std::env::var("GMAIL_CLIENT_SECRET_JSON");
    let original_token = std::env::var("GMAIL_TOKEN_JSON");
    
    std::env::remove_var("GMAIL_CLIENT_SECRET_JSON");
    std::env::remove_var("GMAIL_TOKEN_JSON");
    
    // Should fail to initialize
    let result = GmailLabeler::from_env().await;
    
    // Restore environment variables
    if let Ok(secret) = original_secret {
        std::env::set_var("GMAIL_CLIENT_SECRET_JSON", secret);
    }
    if let Ok(token) = original_token {
        std::env::set_var("GMAIL_TOKEN_JSON", token);
    }
    
    // Check that it failed as expected
    match result {
        Err(LabelingError::Config { .. }) => {
            // Expected - missing environment variables should cause config error
        }
        Err(other) => {
            panic!("Expected Config error, got: {:?}", other);
        }
        Ok(_) => {
            panic!("Expected error for missing credentials, but initialization succeeded");
        }
    }
}
