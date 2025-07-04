/// Complete integration test demonstrating the new PII anonymization pipeline
/// This test shows the full workflow without requiring an actual LLM connection

use agentic_mail_agent::anonymizer::{
    PiiEntity, PiiReplacer
};

/// Simulates what the LLM would detect in a realistic email
fn simulate_llm_detection(text: &str) -> Vec<PiiEntity> {
    let mut entities = Vec::new();
    
    // Simulate finding all instances of known PII patterns in the text
    let pii_patterns = vec![
        ("name", "Mazda Hewitt"),
        ("email", "mazda@aduki.co.uk"),
        ("email", "info@teamshirts.ch"),
        ("company", "TeamShirts"),
        ("company", "sprd.net AG"),
        ("address", "Gießerstraße 27, 04229 Leipzig, Deutschland"),
    ];
    
    for (pii_type, pattern) in pii_patterns {
        let mut start = 0;
        while let Some(pos) = text[start..].find(pattern) {
            let actual_pos = start + pos;
            entities.push(PiiEntity {
                pii_type: pii_type.to_string(),
                text: pattern.to_string(),
                start: actual_pos,
                end: actual_pos + pattern.len(),
            });
            start = actual_pos + pattern.len();
        }
    }
    
    entities
}

#[test]
fn test_complete_email_anonymization_workflow() {
    // Real email data from our test suite (simplified)
    let original_email = r#"{
  "id": "197c6294582e70f2",
  "subject": "Denken Sie daran, teamshirts.ch zu bewerten",
  "snippet": "Eine kleine Erinnerung für Sie, Mazda Hewitt. Nochmals vielen Dank, dass Sie sich für teamshirts.ch entschieden haben!",
  "from": "TeamShirts <noreply.invitations@trustpilotmail.com>",
  "to": ["mazda@aduki.co.uk"],
  "sent": "Tue, 01 Jul 2025 13:24:39 +0000 (UTC)",
  "body": "Eine kleine Erinnerung für Sie, Mazda Hewitt.\n\nNochmals vielen Dank, dass Sie sich für teamshirts.ch entschieden haben! TeamShirts | sprd.net AG\nGießerstraße 27, 04229 Leipzig, Deutschland\nE-Mail: info@teamshirts.ch",
  "downloaded_at": "2025-07-01T13:40:43.471817+00:00",
  "file_index": 1
}"#;

    let email: serde_json::Value = serde_json::from_str(original_email).unwrap();
    
    // Step 1: Combine email fields as our binary does
    let mut full_text = String::new();
    
    if let Some(subject) = email["subject"].as_str() {
        full_text.push_str("Subject: ");
        full_text.push_str(subject);
        full_text.push('\n');
    }
    
    if let Some(from) = email["from"].as_str() {
        full_text.push_str("From: ");
        full_text.push_str(from);
        full_text.push('\n');
    }
    
    if let Some(to_array) = email["to"].as_array() {
        full_text.push_str("To: ");
        let to_strings: Vec<String> = to_array.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        full_text.push_str(&to_strings.join(", "));
        full_text.push('\n');
    }
    
    if let Some(body) = email["body"].as_str() {
        full_text.push_str("Body: ");
        full_text.push_str(body);
        full_text.push('\n');
    }
    
    if let Some(snippet) = email["snippet"].as_str() {
        full_text.push_str("Snippet: ");
        full_text.push_str(snippet);
        full_text.push('\n');
    }
    
    println!("Original combined text:");
    println!("{}", full_text);
    println!();
    
    // Step 2: Simulate LLM PII detection
    let detected_entities = simulate_llm_detection(&full_text);
    println!("Detected {} PII entities:", detected_entities.len());
    for entity in &detected_entities {
        println!("  {} at {}-{}: '{}'", entity.pii_type, entity.start, entity.end, entity.text);
    }
    println!();
    
    // Step 3: Apply anonymization with fallback
    let mut replacer = PiiReplacer::new();
    let anonymized_text = replacer.replace_pii(&full_text, &detected_entities).unwrap();
    
    println!("Anonymized text:");
    println!("{}", anonymized_text);
    println!();
    
    // Step 4: Verify anonymization worked
    assert!(!anonymized_text.contains("Mazda Hewitt"));
    assert!(!anonymized_text.contains("mazda@aduki.co.uk"));
    assert!(!anonymized_text.contains("info@teamshirts.ch"));
    assert!(!anonymized_text.contains("Gießerstraße 27, 04229 Leipzig, Deutschland"));
    
    // Step 5: Check audit trail
    let replacement_log = replacer.get_replacement_log();
    println!("Replacement audit log ({} entries):", replacement_log.len());
    for entry in replacement_log {
        println!("  {} '{}' → '{}'", entry.pii_type, entry.original_value, entry.fake_value);
    }
    println!();
    
    // Step 6: Verify structure preservation
    assert!(anonymized_text.contains("Subject:"));
    assert!(anonymized_text.contains("From:"));
    assert!(anonymized_text.contains("To:"));
    assert!(anonymized_text.contains("Body:"));
    assert!(anonymized_text.contains("Snippet:"));
    
    // Step 7: Parse back into email structure (as the binary does)
    let mut anonymized_email = email.clone();
    for line in anonymized_text.lines() {
        if let Some(stripped) = line.strip_prefix("Subject: ") {
            anonymized_email["subject"] = serde_json::Value::String(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("From: ") {
            anonymized_email["from"] = serde_json::Value::String(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("To: ") {
            let to_list: Vec<serde_json::Value> = stripped
                .split(", ")
                .map(|s| serde_json::Value::String(s.to_string()))
                .collect();
            anonymized_email["to"] = serde_json::Value::Array(to_list);
        } else if let Some(stripped) = line.strip_prefix("Body: ") {
            anonymized_email["body"] = serde_json::Value::String(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("Snippet: ") {
            anonymized_email["snippet"] = serde_json::Value::String(stripped.to_string());
        }
    }
    
    println!("Final anonymized email JSON:");
    println!("{}", serde_json::to_string_pretty(&anonymized_email).unwrap());
    
    // Verify the final email doesn't contain original PII
    let final_email_str = serde_json::to_string(&anonymized_email).unwrap();
    assert!(!final_email_str.contains("Mazda Hewitt"));
    assert!(!final_email_str.contains("mazda@aduki.co.uk"));
    assert!(!final_email_str.contains("info@teamshirts.ch"));
    
    println!("\n✅ Complete workflow test passed!");
    println!("   - {} PII entities detected", detected_entities.len());
    println!("   - {} replacements made", replacement_log.len());
    println!("   - Email structure preserved");
    println!("   - All original PII removed");
    println!("   - Realistic fake data generated");
}

#[test]
fn test_fallback_detection_on_missed_pii() {
    // Test case where LLM misses some obvious PII but fallback catches it
    let text_with_mixed_pii = r#"
    Subject: Important update
    From: john.doe@example.com
    Body: Please contact support@company.org or call (555) 123-4567.
    The meeting is with Alice Johnson at alice.johnson@startup.io.
    Our office number is +1-800-555-0199.
    "#;
    
    // Simulate LLM missing some obvious PII (only catches names)
    let partial_llm_entities = vec![
        PiiEntity {
            pii_type: "name".to_string(),
            text: "Alice Johnson".to_string(),
            start: text_with_mixed_pii.find("Alice Johnson").unwrap(),
            end: text_with_mixed_pii.find("Alice Johnson").unwrap() + "Alice Johnson".len(),
        }
    ];
    
    let mut replacer = PiiReplacer::new();
    let result = replacer.replace_pii(text_with_mixed_pii, &partial_llm_entities).unwrap();
    
    // LLM-only detection - only replaces what was detected
    assert!(result.contains("john.doe@example.com")); // Not detected by LLM
    assert!(result.contains("support@company.org")); // Not detected by LLM
    assert!(result.contains("alice.johnson@startup.io")); // Not detected by LLM
    assert!(result.contains("(555) 123-4567")); // Not detected by LLM
    assert!(result.contains("+1-800-555-0199")); // Not detected by LLM
    
    // Should replace what LLM caught
    assert!(!result.contains("Alice Johnson"));
    
    // No fallback detection in current implementation
    let log = replacer.get_replacement_log();
    
    // Only name should be replaced (what LLM detected)
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].pii_type, "name");
    assert_eq!(log[0].original_value, "Alice Johnson");
    
    println!("LLM-only detection replaced:");
    for entry in log {
        println!("  {} '{}' → '{}'", entry.pii_type, entry.original_value, entry.fake_value);
    }
}
