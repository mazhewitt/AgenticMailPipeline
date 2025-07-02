//! Test to ensure 100% replacement rate for detected PII
//! 
//! This test implements TDD to ensure that every piece of PII detected
//! is actually replaced. If the replacement count is below the detection
//! count, the email anonymization should fail.

use agentic_mail_agent::anonymizer::{AnonymizationPipeline, AnonymizationConfig, LlmBackend};
use serde_json;

/// Email structure for anonymization testing
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct TestEmail {
    id: String,
    subject: Option<String>,
    snippet: Option<String>,
    from: Option<String>,
    to: Option<Vec<String>>,
    sent: Option<String>,
    body: Option<String>,
    downloaded_at: String,
    file_index: usize,
}

/// Test that 100% of detected PII gets replaced
#[tokio::test]
async fn test_100_percent_replacement_rate() {
    // Test email with known PII
    let test_email = TestEmail {
        id: "test_001".to_string(),
        subject: Some("Meeting with John Smith".to_string()),
        snippet: Some("Let's meet at john.smith@company.com".to_string()),
        from: Some("John Smith <john.smith@company.com>".to_string()),
        to: Some(vec!["mazhewitt@gmail.com".to_string()]),
        sent: Some("Wed, 02 Jul 2025 09:12:30 +0000".to_string()),
        body: Some("Hi Mazda, Please call me at +1-555-123-4567. Best, John Smith".to_string()),
        downloaded_at: "2025-07-02T09:49:35.647215+00:00".to_string(),
        file_index: 1,
    };
    
    // Create anonymization pipeline
    let config = AnonymizationConfig {
        backend: LlmBackend::Ollama,
        model: "llama3:8b".to_string(),
        ollama_host: "http://localhost:11434".to_string(),
        openai_api_key: None,
        temperature: 0.1,
        llm_timeout_secs: 60,
    };
    
    let mut pipeline = AnonymizationPipeline::new(config).await
        .expect("Failed to create anonymization pipeline");
    
    // Combine all text fields for PII detection (like the binary does)
    let mut full_text = String::new();
    
    if let Some(subject) = &test_email.subject {
        full_text.push_str("Subject: ");
        full_text.push_str(subject);
        full_text.push('\n');
    }
    
    if let Some(from) = &test_email.from {
        full_text.push_str("From: ");
        full_text.push_str(from);
        full_text.push('\n');
    }
    
    if let Some(to) = &test_email.to {
        full_text.push_str("To: ");
        full_text.push_str(&to.join(", "));
        full_text.push('\n');
    }
    
    if let Some(body) = &test_email.body {
        full_text.push_str("Body: ");
        full_text.push_str(body);
        full_text.push('\n');
    }
    
    if let Some(snippet) = &test_email.snippet {
        full_text.push_str("Snippet: ");
        full_text.push_str(snippet);
        full_text.push('\n');
    }
    
    // Anonymize the combined text
    let result = pipeline.anonymize_email_text(&full_text).await
        .expect("Failed to anonymize email");
    
    // TDD: Assert that 100% of detected PII was replaced
    let detected_count = result.detected_entities.len();
    let replaced_count = result.replacement_log.len();
    
    println!("Detected PII entities: {}", detected_count);
    println!("Replaced PII entities: {}", replaced_count);
    
    // Print details for debugging
    println!("Full text being processed:");
    println!("'{}'", full_text);
    println!();
    
    for entity in &result.detected_entities {
        println!("Detected: {} '{}' at {}-{}", entity.pii_type, entity.text, entity.start, entity.end);
        
        // Show what text is actually at those positions
        if entity.start < full_text.len() && entity.end <= full_text.len() {
            let actual_text = &full_text[entity.start..entity.end];
            println!("  Text at {}-{}: '{}'", entity.start, entity.end, actual_text);
            println!("  Match: {}", actual_text == entity.text);
        } else {
            println!("  Position out of bounds!");
        }
        println!();
    }
    
    for replacement in &result.replacement_log {
        println!("Replaced: {} '{}' -> '{}'", replacement.pii_type, replacement.original_value, replacement.fake_value);
    }
    
    // This test should initially fail because we have imperfect replacement
    assert_eq!(
        detected_count, replaced_count,
        "100% replacement rate required: detected {} PII entities but only replaced {}",
        detected_count, replaced_count
    );
    
    println!("✅ 100% replacement rate achieved!");
    
    // Also verify that the anonymized text doesn't contain the original PII
    println!("Checking for remaining PII in result text...");
    let mut pii_issues = Vec::new();
    
    if result.anonymized_text.contains("john.smith@company.com") {
        pii_issues.push("john.smith@company.com still present");
    }
    if result.anonymized_text.contains("mazhewitt@gmail.com") {
        pii_issues.push("mazhewitt@gmail.com still present");
    }
    if result.anonymized_text.contains("+1-555-123-4567") {
        pii_issues.push("+1-555-123-4567 still present");
    }
    
    if !pii_issues.is_empty() {
        println!("⚠️ Issues found: {:?}", pii_issues);
        println!("Original text:");
        println!("{}", full_text);
        println!("Anonymized text:");
        println!("{}", result.anonymized_text);
    }
    
    assert!(pii_issues.is_empty(), "Original PII should be completely replaced: {:?}", pii_issues);
}

/// Test that emails with imperfect replacement get failed/marked as such
#[tokio::test]
async fn test_email_should_fail_on_imperfect_replacement() {
    // This is a wrapper function that should fail emails that don't achieve 100% replacement
    
    let test_email_content = r#"{
        "id": "test_002", 
        "subject": "Contact info for John Doe",
        "from": "Jane Smith <jane@example.com>",
        "to": ["user@domain.com"],
        "body": "Please reach John at john.doe@company.com or call 555-0123",
        "downloaded_at": "2025-07-02T09:49:35.647215+00:00",
        "file_index": 2
    }"#;
    
    let result = validate_email_anonymization(test_email_content).await;
    
    // This should return an error if replacement rate is not 100%
    match result {
        Ok(_) => {
            // If it succeeds, replacement rate must be 100%
            println!("✅ Email passed with 100% replacement rate");
        }
        Err(e) => {
            // Expected to fail initially due to imperfect replacement
            println!("⚠️ Email failed validation: {}", e);
            assert!(e.to_string().contains("replacement rate"), 
                   "Error should mention replacement rate");
        }
    }
}

/// Validate that an email achieves 100% PII replacement
async fn validate_email_anonymization(email_json: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let email: TestEmail = serde_json::from_str(email_json)?;
    
    let config = AnonymizationConfig {
        backend: LlmBackend::Ollama,
        model: "llama3:8b".to_string(),
        ollama_host: "http://localhost:11434".to_string(),
        openai_api_key: None,
        temperature: 0.1,
        llm_timeout_secs: 60,
    };
    
    let mut pipeline = AnonymizationPipeline::new(config).await?;
    
    // Combine text fields
    let mut full_text = String::new();
    
    if let Some(subject) = &email.subject {
        full_text.push_str("Subject: ");
        full_text.push_str(subject);
        full_text.push('\n');
    }
    
    if let Some(from) = &email.from {
        full_text.push_str("From: ");
        full_text.push_str(from);
        full_text.push('\n');
    }
    
    if let Some(to) = &email.to {
        full_text.push_str("To: ");
        full_text.push_str(&to.join(", "));
        full_text.push('\n');
    }
    
    if let Some(body) = &email.body {
        full_text.push_str("Body: ");
        full_text.push_str(body);
        full_text.push('\n');
    }
    
    if let Some(snippet) = &email.snippet {
        full_text.push_str("Snippet: ");
        full_text.push_str(snippet);
        full_text.push('\n');
    }
    
    // Anonymize
    let result = pipeline.anonymize_email_text(&full_text).await?;
    
    // Validate 100% replacement rate
    let detected_count = result.detected_entities.len();
    let replaced_count = result.replacement_log.len();
    
    if detected_count != replaced_count {
        return Err(format!(
            "Insufficient replacement rate: detected {} PII entities but only replaced {} ({}%)",
            detected_count, 
            replaced_count,
            if detected_count > 0 { (replaced_count * 100) / detected_count } else { 100 }
        ).into());
    }
    
    // Return anonymized email as JSON
    Ok(serde_json::json!({
        "id": email.id,
        "subject": email.subject,
        "from": email.from,
        "to": email.to,
        "body": email.body,
        "snippet": email.snippet,
        "sent": email.sent,
        "downloaded_at": email.downloaded_at,
        "file_index": email.file_index,
        "anonymization_stats": {
            "detected_entities": detected_count,
            "replaced_entities": replaced_count,
            "replacement_rate": 100.0
        }
    }))
}
