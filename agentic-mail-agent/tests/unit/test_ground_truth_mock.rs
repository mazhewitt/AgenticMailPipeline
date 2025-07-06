//! Unit tests using recorded ground truth responses
//!
//! These tests use real LLM responses recorded from the ground truth dataset
//! to test classification accuracy without requiring a live Ollama instance.

use agentic_mail_agent::classifier::{EmailCategory, MessageClassifier, MockOllamaClassifier};
use agentic_mail_agent::core::email::Email;
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
        Err(_) => return None, // Skip problematic JSON files
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

/// Test LLM classification accuracy using recorded responses
#[tokio::test]
async fn test_recorded_llm_classification_accuracy() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/classifier_ground_truth.json",
    )
    .expect("Failed to load recorded ground truth responses");

    let (total_recorded, categories) = mock_classifier.get_stats();
    println!("📊 Loaded {total_recorded} recorded LLM responses with categories: {categories:?}");

    let ground_truth = load_ground_truth_data();
    let mut correct_predictions = 0;
    let mut total_predictions = 0;
    let mut misclassifications = Vec::new();

    for gt_email in &ground_truth.email_ground_truth.test_emails {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            match mock_classifier.classify(&email).await {
                Ok(classification) => {
                    total_predictions += 1;

                    if let Ok(expected_category) = gt_email.category.parse::<EmailCategory>() {
                        if classification.category == expected_category {
                            correct_predictions += 1;
                        } else {
                            misclassifications.push(format!(
                                "Email {}: Expected '{}', Got '{}' (Subject: '{}')",
                                gt_email.id,
                                gt_email.category,
                                classification.category,
                                gt_email.subject
                            ));
                        }
                    }
                }
                Err(_) => {
                    // Email not in recorded responses - this is expected for some emails
                }
            }
        }
    }

    let accuracy = if total_predictions > 0 {
        (correct_predictions as f64 / total_predictions as f64) * 100.0
    } else {
        0.0
    };

    println!("🎯 LLM Classification Results:");
    println!("   Total emails classified: {total_predictions}");
    println!("   Correct predictions: {correct_predictions}");
    println!("   Accuracy: {accuracy:.2}%");

    if !misclassifications.is_empty() && misclassifications.len() <= 10 {
        println!("\n❌ Misclassifications:");
        for misclass in &misclassifications {
            println!("   {misclass}");
        }
    }

    // LLM should have reasonable accuracy (not perfect due to subjectivity)
    assert!(
        total_predictions > 0,
        "Should have classified at least some emails"
    );
    assert!(
        accuracy > 50.0,
        "LLM accuracy should be above 50% (got {accuracy:.2}%)"
    );
}

/// Test specific categories with recorded responses
#[tokio::test]
async fn test_recorded_action_required_classification() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/classifier_ground_truth.json",
    )
    .expect("Failed to load recorded responses");

    let ground_truth = load_ground_truth_data();
    let action_required_emails: Vec<_> = ground_truth
        .email_ground_truth
        .test_emails
        .iter()
        .filter(|e| e.category == "ActionRequired")
        .collect();

    let mut correct = 0;
    let mut total = 0;

    println!("🎯 Testing ActionRequired emails with recorded responses:");

    for gt_email in action_required_emails {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            if let Ok(classification) = mock_classifier.classify(&email).await {
                total += 1;
                println!(
                    "   '{}' -> {} (expected ActionRequired)",
                    gt_email.subject, classification.category
                );
                if classification.category == EmailCategory::ActionRequired {
                    correct += 1;
                }
            }
        }
    }

    let accuracy = if total > 0 {
        (correct as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!("📊 ActionRequired accuracy: {correct}/{total} ({accuracy:.1}%)");

    // Should classify at least some ActionRequired emails correctly
    if total > 0 {
        assert!(
            accuracy >= 50.0,
            "ActionRequired category accuracy ({accuracy:.2}%) should be at least 50%"
        );
    }
}

/// Test that recorded responses contain detailed reasoning
#[tokio::test]
async fn test_recorded_responses_contain_reasoning() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/classifier_ground_truth.json",
    )
    .expect("Failed to load recorded responses");

    let ground_truth = load_ground_truth_data();

    // Test a few emails to ensure they have detailed reasoning
    let mut responses_with_reasoning = 0;
    let mut total_tested = 0;

    for gt_email in ground_truth.email_ground_truth.test_emails.iter().take(5) {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            if let Ok(classification) = mock_classifier.classify(&email).await {
                total_tested += 1;

                println!(
                    "🧠 Reasoning for '{}': {}",
                    gt_email.subject, classification.llm_response
                );

                // Check for substantial reasoning
                if classification.llm_response.len() > 50
                    && classification.llm_response.contains("LLM Response:")
                {
                    responses_with_reasoning += 1;
                }
            }
        }
    }

    if total_tested > 0 {
        let reasoning_rate = (responses_with_reasoning as f64 / total_tested as f64) * 100.0;
        assert!(
            reasoning_rate >= 80.0,
            "Most responses should contain detailed reasoning ({reasoning_rate:.1}% had good reasoning)"
        );
    }
}

/// Test deterministic replay of recorded responses
#[tokio::test]
async fn test_recorded_responses_deterministic() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/classifier_ground_truth.json",
    )
    .expect("Failed to load recorded responses");

    let ground_truth = load_ground_truth_data();

    // Pick first available email
    for gt_email in &ground_truth.email_ground_truth.test_emails {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            // Try to classify multiple times
            if let Ok(classification1) = mock_classifier.classify(&email).await {
                let classification2 = mock_classifier.classify(&email).await.unwrap();
                let classification3 = mock_classifier.classify(&email).await.unwrap();

                // Should be identical
                assert_eq!(classification1.category, classification2.category);
                assert_eq!(classification1.category, classification3.category);
                assert_eq!(classification1.score, classification2.score);
                assert_eq!(classification1.llm_response, classification2.llm_response);

                println!(
                    "✅ Deterministic replay verified for '{}'",
                    gt_email.subject
                );
                break; // Only need to test one
            }
        }
    }
}
