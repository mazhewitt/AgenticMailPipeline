//! Test to record real LLM responses for later replay in unit tests
//! 
//! This test runs with a real Ollama instance and records all responses
//! so they can be replayed deterministically in unit tests.

use agentic_mail_agent::classifier::{
    MessageClassifier, LangChainClassifier, MockOllamaClassifier, 
    StubClassifier, HybridClassifier
};
use agentic_mail_agent::core::email::Email;

/// Record responses from classifier ground truth tests
#[tokio::test]
#[ignore = "requires running ollama server"]
async fn record_classifier_ground_truth_responses() {
    // Load ground truth data
    let ground_truth = load_ground_truth_data();
    
    // Try to create real LLM classifier
    let real_classifier: Box<dyn MessageClassifier + Send + Sync> = 
        match LangChainClassifier::with_default_config().await {
            Ok(llm_classifier) => {
                println!("✅ Using real LLM classifier for recording");
                Box::new(llm_classifier)
            }
            Err(e) => {
                println!("⚠️  LLM unavailable ({}), using stub for recording", e);
                Box::new(StubClassifier::deterministic())
            }
        };
    
    // Create mock in recording mode
    let recording_file = "test_data/recorded_responses/classifier_ground_truth.json";
    let mock_classifier = MockOllamaClassifier::new_recording_mode(
        recording_file,
        real_classifier
    );
    
    println!("🎥 Recording responses for {} emails...", ground_truth.email_ground_truth.test_emails.len());
    
    let mut recorded_count = 0;
    for gt_email in &ground_truth.email_ground_truth.test_emails {
        let email_path = format!("test_data/{}", gt_email.file);
        
        if let Some(email) = try_load_test_email(&email_path) {
            match mock_classifier.classify(&email).await {
                Ok(classification) => {
                    recorded_count += 1;
                    println!("📼 Recorded response for '{}': {} -> {}", 
                        gt_email.subject, 
                        classification.category,
                        gt_email.category
                    );
                }
                Err(e) => {
                    eprintln!("❌ Failed to classify email {}: {}", gt_email.id, e);
                }
            }
        }
    }
    
    // Save all recordings
    mock_classifier.save_recordings().await.expect("Failed to save recordings");
    
    println!("✅ Recorded {} responses to {}", recorded_count, recording_file);
    let (total, categories) = mock_classifier.get_stats();
    println!("📊 Statistics: {} total responses, categories: {:?}", total, categories);
}

/// Record responses from hybrid classifier tests
#[tokio::test]
#[ignore = "requires running ollama server"]
async fn record_hybrid_classifier_responses() {
    let ground_truth = load_ground_truth_data();
    
    // Try to create hybrid classifier with real LLM
    let real_classifier: Box<dyn MessageClassifier + Send + Sync> = 
        match LangChainClassifier::with_default_config().await {
            Ok(llm_classifier) => {
                println!("✅ Using Hybrid classifier with real LLM");
                Box::new(HybridClassifier::new_with_llm(Box::new(llm_classifier)).await)
            }
            Err(e) => {
                println!("⚠️  LLM unavailable ({}), using rules-only hybrid", e);
                Box::new(HybridClassifier::new_rules_only())
            }
        };
    
    let recording_file = "test_data/recorded_responses/hybrid_classifier.json";
    let mock_classifier = MockOllamaClassifier::new_recording_mode(
        recording_file,
        real_classifier
    );
    
    // Record a subset of emails for hybrid testing
    let test_emails: Vec<_> = ground_truth.email_ground_truth.test_emails
        .iter()
        .take(10) // Just first 10 for hybrid testing
        .collect();
    
    println!("🎥 Recording hybrid responses for {} emails...", test_emails.len());
    
    for gt_email in test_emails {
        let email_path = format!("test_data/{}", gt_email.file);
        
        if let Some(email) = try_load_test_email(&email_path) {
            match mock_classifier.classify(&email).await {
                Ok(classification) => {
                    println!("📼 Hybrid recorded: '{}' -> {}", 
                        gt_email.subject, 
                        classification.category
                    );
                }
                Err(e) => {
                    eprintln!("❌ Hybrid classification failed for {}: {}", gt_email.id, e);
                }
            }
        }
    }
    
    mock_classifier.save_recordings().await.expect("Failed to save hybrid recordings");
    println!("✅ Saved hybrid recordings to {}", recording_file);
}

/// Record individual email classification examples for unit tests
#[tokio::test]
#[ignore = "requires running ollama server"]
async fn record_individual_examples() {
    // Create real classifier if available
    let real_classifier: Box<dyn MessageClassifier + Send + Sync> = 
        match LangChainClassifier::with_default_config().await {
            Ok(llm_classifier) => {
                println!("✅ Using real LLM for individual examples");
                Box::new(llm_classifier)
            }
            Err(e) => {
                println!("⚠️  LLM unavailable ({}), using stub for examples", e);
                Box::new(StubClassifier::deterministic())
            }
        };
    
    let recording_file = "test_data/recorded_responses/individual_examples.json";
    let mock_classifier = MockOllamaClassifier::new_recording_mode(
        recording_file,
        real_classifier
    );
    
    // Create test emails covering different categories
    let test_emails = vec![
        Email::new_full(
            "urgent001".to_string(),
            Some("URGENT: Server Down".to_string()),
            Some("Production server crashed, need immediate action".to_string()),
            Some("ops@company.com".to_string()),
            None, None, None,
        ),
        Email::new_full(
            "newsletter001".to_string(),
            Some("Weekly Tech Newsletter".to_string()),
            Some("Latest tech trends and AI developments".to_string()),
            Some("tech@newsletter.com".to_string()),
            None, None, None,
        ),
        Email::new_full(
            "spam001".to_string(),
            Some("You've won $1 Million!".to_string()),
            Some("Click here to claim your prize now!".to_string()),
            Some("noreply@scam.com".to_string()),
            None, None, None,
        ),
        Email::new_full(
            "receipt001".to_string(),
            Some("Order Confirmation #12345".to_string()),
            Some("Thank you for your order. Your receipt is attached.".to_string()),
            Some("orders@shop.com".to_string()),
            None, None, None,
        ),
    ];
    
    println!("🎥 Recording individual examples...");
    
    for email in test_emails {
        match mock_classifier.classify(&email).await {
            Ok(classification) => {
                println!("📼 Example recorded: '{}' -> {}", 
                    email.subject.as_deref().unwrap_or("No Subject"), 
                    classification.category
                );
            }
            Err(e) => {
                eprintln!("❌ Failed to classify example {}: {}", email.id, e);
            }
        }
    }
    
    mock_classifier.save_recordings().await.expect("Failed to save individual examples");
    println!("✅ Saved individual examples to {}", recording_file);
}

// Helper functions from classifier ground truth tests
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmailGroundTruth {
    pub file: String,
    pub id: String,
    pub subject: String,
    pub snippet: String,
    pub category: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct GroundTruthData {
    pub email_ground_truth: GroundTruthMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct GroundTruthMetadata {
    pub test_emails: Vec<EmailGroundTruth>,
    pub statistics: HashMap<String, u32>,
}

fn load_ground_truth_data() -> GroundTruthData {
    let ground_truth_json = include_str!("../../test_data/ground_truth.json");
    serde_json::from_str(ground_truth_json).expect("Failed to parse ground truth data")
}

fn try_load_test_email(file_path: &str) -> Option<Email> {
    let email_json = std::fs::read_to_string(file_path).ok()?;
    
    let email_data: serde_json::Value = match serde_json::from_str(&email_json) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Skipping {} due to JSON parsing error: {}", file_path, e);
            return None;
        }
    };
    
    Some(Email::new_full(
        email_data["id"].as_str().unwrap_or("unknown").to_string(),
        email_data["subject"].as_str().map(|s| s.to_string()),
        email_data["snippet"].as_str().map(|s| s.to_string()),
        email_data["from"].as_str().map(|s| s.to_string()),
        email_data["to"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        }),
        email_data["sent"].as_str().map(|s| s.to_string()),
        email_data["body"].as_str().map(|s| s.to_string()),
    ))
}