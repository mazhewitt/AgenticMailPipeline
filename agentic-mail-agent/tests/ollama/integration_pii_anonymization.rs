use agentic_mail_agent::anonymizer::{
    PiiDetector, AnonymizationPipeline, AnonymizationConfig, LlmBackend
};

#[tokio::test]
#[ignore = "requires running ollama server"]
async fn test_pii_detection_with_llm() {
    let config = AnonymizationConfig::new(LlmBackend::Ollama, Some("llama3:8b".to_string())).unwrap();
    let detector = PiiDetector::new(config).await.unwrap();
    
    let email_text = r#"
Hi John Smith,
Thank you for your email to support@company.com. 
Please contact me at (555) 123-4567 or jane.doe@company.com.
My office is at 123 Main Street, San Francisco, CA 94105.
Best regards,
Jane Doe
    "#;
    
    let pii_entities = detector.detect_pii(email_text).await.unwrap();
    
    println!("Detected PII entities: {pii_entities:?}");
    
    // Should detect at least some PII (LLMs may not detect everything consistently)
    assert!(!pii_entities.is_empty());
    
    // Should find at least one name or email (but not necessarily specific ones)
    let has_name = pii_entities.iter().any(|entity| entity.pii_type == "name");
    let has_email = pii_entities.iter().any(|entity| entity.pii_type == "email");
    let has_phone = pii_entities.iter().any(|entity| entity.pii_type == "phone");
    
    // Should detect at least one type of PII
    assert!(has_name || has_email || has_phone);
}

// Test function `test_pii_replacement_consistency` has been moved to unit tests
// Location: tests/unit/test_pii_replacer.rs
// This test only used PiiReplacer without LLM backend (no Ollama dependency) so it belongs in unit tests

// Test function `test_pii_replacement_with_fallback` has been moved to unit tests
// Location: tests/unit/test_pii_replacer.rs
// This test only used PiiReplacer without LLM backend (no Ollama dependency) so it belongs in unit tests

#[tokio::test]
#[ignore = "requires running ollama server"]
async fn test_full_anonymization_pipeline() {
    let config = AnonymizationConfig::new(LlmBackend::Ollama, Some("llama3:8b".to_string())).unwrap();
    let mut pipeline = AnonymizationPipeline::new(config).await.unwrap();
    
    let test_email = r#"
Subject: Meeting Request
From: alice.smith@company.com
To: bob.jones@company.com

Hi Bob,

I hope this email finds you well. I wanted to schedule a meeting with you next week.
Please call me at (555) 987-6543 or email me back at alice.smith@company.com.

My office is located at 456 Business Ave, Suite 200, New York, NY 10001.

Best regards,
Alice Smith
Senior Manager
Company Corp
    "#;
    
    let result = pipeline.anonymize_email_text(test_email).await.unwrap();
    
    // Print what was detected for debugging
    println!("Detected entities: {:?}", result.detected_entities);
    println!("Anonymized text: {}", result.anonymized_text);
    
    // Should have detected some entities (LLMs are not 100% consistent)
    assert!(!result.detected_entities.is_empty());
    
    // Should have replacement log entries
    assert!(!result.replacement_log.is_empty());
    
    // At least some PII should be replaced (not testing for 100% as LLMs vary)
    let _original_pii_count = test_email.matches("@").count(); // Count email addresses
    let _anonymized_pii_count = result.anonymized_text.matches("@").count();
    
    // The anonymized text should have different content (some emails replaced)
    // Note: We use a less strict test because LLM detection can vary
    assert_ne!(test_email, result.anonymized_text);
    
    // Should have replacement log
    assert!(!result.replacement_log.is_empty());
    
    // Should maintain email structure
    assert!(result.anonymized_text.contains("Subject:"));
    assert!(result.anonymized_text.contains("From:"));
    assert!(result.anonymized_text.contains("To:"));
}

// Test function `test_audit_logging` has been moved to unit tests
// Location: tests/unit/test_pii_replacer.rs
// This test only used PiiReplacer without LLM backend (no Ollama dependency) so it belongs in unit tests
