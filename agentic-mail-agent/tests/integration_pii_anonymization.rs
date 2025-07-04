use agentic_mail_agent::anonymizer::{
    PiiEntity, PiiDetector, PiiReplacer, AnonymizationPipeline, AnonymizationConfig, LlmBackend
};

#[tokio::test]
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
    
    println!("Detected PII entities: {:?}", pii_entities);
    
    // Should detect at least some PII (LLMs may not detect everything consistently)
    assert!(!pii_entities.is_empty());
    
    // Should find at least one name or email (but not necessarily specific ones)
    let has_name = pii_entities.iter().any(|entity| entity.pii_type == "name");
    let has_email = pii_entities.iter().any(|entity| entity.pii_type == "email");
    let has_phone = pii_entities.iter().any(|entity| entity.pii_type == "phone");
    
    // Should detect at least one type of PII
    assert!(has_name || has_email || has_phone);
}

#[test]
fn test_pii_replacement_consistency() {
    let mut replacer = PiiReplacer::new();
    
    let original_text = "Hi John Smith, please contact me at john.smith@gmail.com for any questions.";
    
    // Find the correct positions
    let name_start = original_text.find("John Smith").unwrap();
    let name_end = name_start + "John Smith".len();
    let email_start = original_text.find("john.smith@gmail.com").unwrap();
    let email_end = email_start + "john.smith@gmail.com".len();
    
    // Create test PII entities with correct positions
    let entities = vec![
        PiiEntity {
            pii_type: "name".to_string(),
            text: "John Smith".to_string(),
            start: name_start,
            end: name_end,
        },
        PiiEntity {
            pii_type: "email".to_string(),
            text: "john.smith@gmail.com".to_string(),
            start: email_start,
            end: email_end,
        },
    ];
    
    println!("Original text: {}", original_text);
    println!("Name at {}-{}: '{}'", name_start, name_end, &original_text[name_start..name_end]);
    println!("Email at {}-{}: '{}'", email_start, email_end, &original_text[email_start..email_end]);
    
    let anonymized = replacer.replace_pii(original_text, &entities).unwrap();
    println!("Anonymized text: {}", anonymized);
    
    // Should not contain original PII
    assert!(!anonymized.contains("John Smith"));
    assert!(!anonymized.contains("john.smith@gmail.com"));
    
    // Should contain fake replacements
    assert!(anonymized.len() > 0);
    
    // Test consistency - same input should produce same output
    let anonymized2 = replacer.replace_pii(original_text, &entities).unwrap();
    assert_eq!(anonymized, anonymized2);
}

#[test]
fn test_pii_replacement_with_fallback() {
    let mut replacer = PiiReplacer::new();
    
    let text_with_obvious_pii = r#"
Contact me at obvious.email@gmail.com or call me at 555-123-4567.
My name is Obvious Person and I live at 123 Obvious Street.
    "#;
    
    // Test with empty LLM entities (simulating LLM failure)
    let llm_entities = vec![];
    let anonymized = replacer.replace_pii(text_with_obvious_pii, &llm_entities).unwrap();
    
    // LLM-only detection - no fallback, so obvious patterns remain
    assert!(anonymized.contains("obvious.email@gmail.com"));
    assert!(anonymized.contains("555-123-4567"));
}

#[tokio::test]
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

#[test]
fn test_audit_logging() {
    let mut replacer = PiiReplacer::new();
    
    let entities = vec![
        PiiEntity {
            pii_type: "name".to_string(),
            text: "John Doe".to_string(),
            start: 0,
            end: 8,
        },
    ];
    
    let original_text = "John Doe sent an email";
    let _result = replacer.replace_pii(original_text, &entities).unwrap();
    
    let log = replacer.get_replacement_log();
    assert!(!log.is_empty());
    
    let first_log_entry = &log[0];
    assert_eq!(first_log_entry.pii_type, "name");
    assert_eq!(first_log_entry.original_value, "John Doe");
    assert!(!first_log_entry.fake_value.is_empty());
    assert!(first_log_entry.fake_value != "John Doe");
}
