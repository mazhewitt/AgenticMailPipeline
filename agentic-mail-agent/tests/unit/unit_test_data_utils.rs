//! Unit tests for test data creation utilities
//! 
//! These tests verify the PII detection and file handling functions work correctly
//! without requiring actual Gmail access or downloading real emails.

use std::fs;
use agentic_mail_agent::test_data_utils::*;

#[test]
fn test_count_json_files_utility() {
    let test_dir = "temp_test_utils";
    fs::create_dir_all(test_dir).unwrap();
    
    // Create mixed file types
    fs::write(format!("{test_dir}/email1.json"), r#"{"id": "1"}"#).unwrap();
    fs::write(format!("{test_dir}/email2.json"), r#"{"id": "2"}"#).unwrap();
    fs::write(format!("{test_dir}/readme.txt"), "not json").unwrap();
    fs::write(format!("{test_dir}/manifest.json"), r#"{"count": 2}"#).unwrap();
    
    let count = count_json_files(test_dir).unwrap();
    assert_eq!(count, 3); // Should count all 3 JSON files
    
    fs::remove_dir_all(test_dir).unwrap();
}

#[tokio::test]
async fn test_pii_detection_functions() {
    // Create a temporary directory with test email files
    let test_dir = "temp_pii_test";
    fs::create_dir_all(test_dir).unwrap();
    
    // Create a test email with obvious PII that should be flagged
    let test_email_with_pii = r#"{
        "id": "test1",
        "subject": "Meeting with John Smith",
        "from": "real.person@company.com",
        "body": "Hi John, please call me at (555) 123-4567",
        "snippet": "Contact John Smith at john.smith@realcompany.com"
    }"#;
    
    // Create a test email with anonymized data that should pass
    let test_email_anonymized = r#"{
        "id": "test2", 
        "subject": "Meeting with Alex Johnson",
        "from": "user1@example.com",
        "body": "Hi Alex, please call me at (555) 1001-2345",
        "snippet": "Contact user2@example.com for details"
    }"#;
    
    fs::write(format!("{test_dir}/email_with_pii.json"), test_email_with_pii).unwrap();
    fs::write(format!("{test_dir}/email_anonymized.json"), test_email_anonymized).unwrap();
    
    // Test the PII spot check function
    let warnings = spot_check_for_pii(test_dir).await.unwrap();
    
    // Should detect some issues in the first email
    assert!(!warnings.is_empty(), "Should detect PII in test emails");
    
    // Check that warnings mention the problematic file
    let has_pii_file_warning = warnings.iter().any(|w| w.contains("email_with_pii.json"));
    assert!(has_pii_file_warning, "Should flag the email with PII");
    
    println!("PII detection test passed. Warnings: {warnings:?}");
    
    fs::remove_dir_all(test_dir).unwrap();
}

#[test]
fn test_pii_detection_patterns() {
    // Test email detection
    assert!(agentic_mail_agent::test_data_utils::check_for_real_emails("Contact me at real.person@company.com"));
    assert!(!agentic_mail_agent::test_data_utils::check_for_real_emails("Contact user1@example.com"));
    assert!(!agentic_mail_agent::test_data_utils::check_for_real_emails("Contact user123@example.com"));
    
    // Test phone detection 
    assert!(agentic_mail_agent::test_data_utils::check_for_real_phones("Call me at (123) 456-7890"));
    assert!(!agentic_mail_agent::test_data_utils::check_for_real_phones("Call me at (555) 123-4567"));
    
    // Test name detection
    assert!(agentic_mail_agent::test_data_utils::check_for_suspicious_names("Hello John Smith"));
    assert!(agentic_mail_agent::test_data_utils::check_for_suspicious_names("Contact Sarah for details"));
    assert!(!agentic_mail_agent::test_data_utils::check_for_suspicious_names("Contact Alex for details"));
}

#[test]
fn test_text_content_extraction() {
    let email_json = serde_json::json!({
        "id": "test123",
        "subject": "Test Subject",
        "from": "test@example.com",
        "body": "Test body content",
        "snippet": "Test snippet"
    });
    
    let fields = agentic_mail_agent::test_data_utils::get_text_content(&email_json).unwrap();
    
    assert_eq!(fields.len(), 4);
    assert!(fields.iter().any(|(name, _)| name == "subject"));
    assert!(fields.iter().any(|(name, _)| name == "from"));
    assert!(fields.iter().any(|(name, _)| name == "body"));
    assert!(fields.iter().any(|(name, _)| name == "snippet"));
}
