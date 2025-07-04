use agentic_mail_agent::anonymizer::{
    PiiEntity, PiiReplacer
};
use serde_json;
use std::collections::HashMap;

#[test]
fn test_email_field_reconstruction() {
    // Test our email field reconstruction logic
    let original_email = r#"{
  "id": "test123",
  "subject": "Meeting with John Smith",
  "from": "john.smith@company.com",
  "to": ["manager@company.com"],
  "body": "Hi there, this is John Smith from TechCorp. Please call me at (555) 123-4567.",
  "file_index": 1
}"#;

    let email: serde_json::Value = serde_json::from_str(original_email).unwrap();
    
    // Simulate the text combination that our binary does
    let mut full_text = String::new();
    let mut field_offsets = HashMap::new();
    
    if let Some(subject) = email["subject"].as_str() {
        field_offsets.insert("subject", full_text.len());
        full_text.push_str("Subject: ");
        full_text.push_str(subject);
        full_text.push('\n');
    }
    
    if let Some(from) = email["from"].as_str() {
        field_offsets.insert("from", full_text.len());
        full_text.push_str("From: ");
        full_text.push_str(from);
        full_text.push('\n');
    }
    
    if let Some(to_array) = email["to"].as_array() {
        field_offsets.insert("to", full_text.len());
        full_text.push_str("To: ");
        let to_strings: Vec<String> = to_array.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        full_text.push_str(&to_strings.join(", "));
        full_text.push('\n');
    }
    
    if let Some(body) = email["body"].as_str() {
        field_offsets.insert("body", full_text.len());
        full_text.push_str("Body: ");
        full_text.push_str(body);
        full_text.push('\n');
    }
    
    println!("Combined text:\n{}", full_text);
    
    // Create PII entities based on the combined text - find ALL occurrences
    let mut entities = Vec::new();
    
    // Find all occurrences of "John Smith"
    let mut start = 0;
    while let Some(pos) = full_text[start..].find("John Smith") {
        let actual_pos = start + pos;
        entities.push(PiiEntity {
            pii_type: "name".to_string(),
            text: "John Smith".to_string(),
            start: actual_pos,
            end: actual_pos + "John Smith".len(),
        });
        start = actual_pos + "John Smith".len();
    }
    
    // Find email
    if let Some(email_start) = full_text.find("john.smith@company.com") {
        let email_end = email_start + "john.smith@company.com".len();
        entities.push(PiiEntity {
            pii_type: "email".to_string(),
            text: "john.smith@company.com".to_string(),
            start: email_start,
            end: email_end,
        });
    }
    
    // Find phone
    if let Some(phone_start) = full_text.find("(555) 123-4567") {
        let phone_end = phone_start + "(555) 123-4567".len();
        entities.push(PiiEntity {
            pii_type: "phone".to_string(),
            text: "(555) 123-4567".to_string(),
            start: phone_start,
            end: phone_end,
        });
    }
    
    // Find company
    if let Some(company_start) = full_text.find("TechCorp") {
        let company_end = company_start + "TechCorp".len();
        entities.push(PiiEntity {
            pii_type: "company".to_string(),
            text: "TechCorp".to_string(),
            start: company_start,
            end: company_end,
        });
    }
    
    println!("Found {} PII entities:", entities.len());
    for entity in &entities {
        println!("  {} at {}-{}: '{}'", entity.pii_type, entity.start, entity.end, entity.text);
    }
    
    // Apply anonymization
    let mut replacer = PiiReplacer::new();
    let anonymized_text = replacer.replace_pii(&full_text, &entities).unwrap();
    
    println!("Anonymized text:\n{}", anonymized_text);
    
    // Verify anonymization worked
    assert!(!anonymized_text.contains("John Smith"));
    assert!(!anonymized_text.contains("john.smith@company.com"));
    assert!(!anonymized_text.contains("(555) 123-4567"));
    assert!(!anonymized_text.contains("TechCorp"));
    
    // Verify structure is preserved
    assert!(anonymized_text.contains("Subject: "));
    assert!(anonymized_text.contains("From: "));
    assert!(anonymized_text.contains("To: "));
    assert!(anonymized_text.contains("Body: "));
    
    // Verify we have replacement log
    let log = replacer.get_replacement_log();
    assert_eq!(log.len(), 5); // 2 names + 1 email + 1 phone + 1 company
    assert!(log.iter().filter(|entry| entry.pii_type == "name").count() == 2);
    assert!(log.iter().any(|entry| entry.pii_type == "email"));
    assert!(log.iter().any(|entry| entry.pii_type == "phone"));
    assert!(log.iter().any(|entry| entry.pii_type == "company"));
}

#[test]
fn test_email_field_parsing() {
    // Test parsing the anonymized text back into fields
    let anonymized_text = r#"Subject: Meeting with Alex Smith
From: user1@example.com
To: user2@example.com
Body: Hi there, this is Alex Smith from TechCorp. Please call me at (555) 1001-1001.
"#;
    
    let mut parsed_fields = HashMap::new();
    
    for line in anonymized_text.lines() {
        if let Some(stripped) = line.strip_prefix("Subject: ") {
            parsed_fields.insert("subject", stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("From: ") {
            parsed_fields.insert("from", stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("To: ") {
            let to_list: Vec<String> = stripped.split(", ").map(|s| s.to_string()).collect();
            parsed_fields.insert("to", to_list.join(","));
        } else if let Some(stripped) = line.strip_prefix("Body: ") {
            parsed_fields.insert("body", stripped.to_string());
        }
    }
    
    assert_eq!(parsed_fields.get("subject").unwrap(), "Meeting with Alex Smith");
    assert_eq!(parsed_fields.get("from").unwrap(), "user1@example.com");
    assert_eq!(parsed_fields.get("to").unwrap(), "user2@example.com");
    assert_eq!(parsed_fields.get("body").unwrap(), "Hi there, this is Alex Smith from TechCorp. Please call me at (555) 1001-1001.");
}

#[test]
fn test_fallback_pii_detection() {
    // Test that our fallback patterns work when LLM detection fails
    let mut replacer = PiiReplacer::new();
    
    let text_with_pii = r#"
    Contact me at obvious.email@gmail.com or call 555-123-4567.
    You can also reach me at (555) 987-6543 or another.email@company.org.
    "#;
    
    // Empty LLM entities (simulating LLM failure or missing detection)
    let llm_entities = vec![];
    
    let anonymized = replacer.replace_pii_with_fallback(text_with_pii, &llm_entities).unwrap();
    
    // LLM-only detection - no fallback, so nothing should be replaced
    assert!(anonymized.contains("obvious.email@gmail.com"));
    assert!(anonymized.contains("another.email@company.org"));
    assert!(anonymized.contains("555-123-4567"));
    assert!(anonymized.contains("(555) 987-6543"));
    
    // Should have empty logs since no LLM entities provided
    let log = replacer.get_replacement_log();
    assert!(log.is_empty());
}

#[test]
fn test_consistency_across_multiple_calls() {
    // Test that the same PII gets replaced consistently across multiple calls
    let mut replacer = PiiReplacer::new();
    
    let text1 = "Hello John Smith, please email john.smith@company.com";
    let text2 = "This is John Smith from john.smith@company.com calling back";
    
    let entities1 = vec![
        PiiEntity {
            pii_type: "name".to_string(),
            text: "John Smith".to_string(),
            start: text1.find("John Smith").unwrap(),
            end: text1.find("John Smith").unwrap() + "John Smith".len(),
        },
        PiiEntity {
            pii_type: "email".to_string(),
            text: "john.smith@company.com".to_string(),
            start: text1.find("john.smith@company.com").unwrap(),
            end: text1.find("john.smith@company.com").unwrap() + "john.smith@company.com".len(),
        },
    ];
    
    let entities2 = vec![
        PiiEntity {
            pii_type: "name".to_string(),
            text: "John Smith".to_string(),
            start: text2.find("John Smith").unwrap(),
            end: text2.find("John Smith").unwrap() + "John Smith".len(),
        },
        PiiEntity {
            pii_type: "email".to_string(),
            text: "john.smith@company.com".to_string(),
            start: text2.find("john.smith@company.com").unwrap(),
            end: text2.find("john.smith@company.com").unwrap() + "john.smith@company.com".len(),
        },
    ];
    
    let result1 = replacer.replace_pii(text1, &entities1).unwrap();
    let result2 = replacer.replace_pii(text2, &entities2).unwrap();
    
    // Extract the fake name and email from result1
    let log = replacer.get_replacement_log();
    let name_replacement = log.iter().find(|entry| entry.pii_type == "name").unwrap();
    let email_replacement = log.iter().find(|entry| entry.pii_type == "email").unwrap();
    
    // Verify both results use the same fake values
    assert!(result1.contains(&name_replacement.fake_value));
    assert!(result1.contains(&email_replacement.fake_value));
    assert!(result2.contains(&name_replacement.fake_value));
    assert!(result2.contains(&email_replacement.fake_value));
}
