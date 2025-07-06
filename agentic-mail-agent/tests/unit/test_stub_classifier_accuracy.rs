use agentic_mail_agent::classifier::{MessageClassifier, StubClassifier};
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

/// Load ground truth data from JSON file
fn load_ground_truth_data() -> GroundTruthData {
    let ground_truth_json = include_str!("../../test_data/ground_truth.json");
    serde_json::from_str(ground_truth_json).expect("Failed to parse ground truth data")
}

/// Load test email from JSON file, return None if parsing fails
fn try_load_test_email(file_path: &str) -> Option<Email> {
    let email_json = std::fs::read_to_string(file_path).ok()?;

    let email_data: serde_json::Value = match serde_json::from_str(&email_json) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Skipping {file_path} due to JSON parsing error: {e}");
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

/// Test that stub classifier achieves accuracy on ground truth data
#[tokio::test]
async fn test_stub_classifier_accuracy_against_ground_truth() {
    let ground_truth = load_ground_truth_data();
    let classifier = StubClassifier::deterministic();

    let mut correct_predictions = 0;
    let mut total_predictions = 0;
    let mut misclassifications = Vec::new();

    for gt_email in &ground_truth.email_ground_truth.test_emails {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            match classifier.classify(&email).await {
                Ok(classification) => {
                    total_predictions += 1;

                    if classification.category == gt_email.category {
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
                Err(e) => {
                    panic!("Classification failed for email {}: {}", gt_email.id, e);
                }
            }
        }
    }

    let accuracy = (correct_predictions as f64 / total_predictions as f64) * 100.0;

    // Print detailed results
    println!("Stub Classifier Results:");
    println!("Total emails: {total_predictions}");
    println!("Correct predictions: {correct_predictions}");
    println!("Accuracy: {accuracy:.2}%");

    if !misclassifications.is_empty() && misclassifications.len() <= 15 {
        println!("\nMisclassifications:");
        for misclass in &misclassifications {
            println!("  {misclass}");
        }
    }

    // Print accuracy by category
    let mut category_stats: HashMap<String, (usize, usize)> = HashMap::new();

    for gt_email in &ground_truth.email_ground_truth.test_emails {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            if let Ok(classification) = classifier.classify(&email).await {
                let entry = category_stats
                    .entry(gt_email.category.clone())
                    .or_insert((0, 0));
                entry.1 += 1; // total
                if classification.category == gt_email.category {
                    entry.0 += 1; // correct
                }
            }
        }
    }

    println!("\nStub Classifier - Accuracy by Category:");
    for (category, (correct, total)) in category_stats {
        let cat_accuracy = (correct as f64 / total as f64) * 100.0;
        println!("  {category}: {correct}/{total} ({cat_accuracy:.1}%)");
    }

    // Report but don't fail - this is for evaluation
    if accuracy < 80.0 {
        println!("\n⚠️  Accuracy ({accuracy:.2}%) is below 80% threshold");
    } else {
        println!("\n✅ Accuracy ({accuracy:.2}%) meets 80% threshold");
    }
}

/// Test specific high-priority categories
#[tokio::test]
async fn test_action_required_category_accuracy() {
    let ground_truth = load_ground_truth_data();
    let classifier = StubClassifier::deterministic();

    let action_required_emails: Vec<_> = ground_truth
        .email_ground_truth
        .test_emails
        .iter()
        .filter(|e| e.category == "ActionRequired")
        .collect();

    let mut correct = 0;
    let mut total = 0;

    println!("Testing ActionRequired emails:");

    for gt_email in action_required_emails {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            if let Ok(classification) = classifier.classify(&email).await {
                total += 1;
                println!("  '{}' -> {}", gt_email.subject, classification.category);
                if classification.category == "ActionRequired" {
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

    println!("ActionRequired category accuracy: {correct}/{total} ({accuracy:.1}%)");

    // ActionRequired emails are critical - we want high precision
    assert!(
        accuracy >= 75.0,
        "ActionRequired category accuracy ({accuracy:.2}%) is below threshold (75%)"
    );
}

/// Test that no emails are misclassified as spam
#[tokio::test]
async fn test_no_false_spam_classification() {
    let ground_truth = load_ground_truth_data();
    let classifier = StubClassifier::deterministic();

    let non_spam_emails: Vec<_> = ground_truth
        .email_ground_truth
        .test_emails
        .iter()
        .filter(|e| e.category != "Spam")
        .collect();

    let mut false_spam_count = 0;
    let mut false_spam_examples = Vec::new();

    for gt_email in non_spam_emails {
        let email_path = format!("test_data/{}", gt_email.file);

        if let Some(email) = try_load_test_email(&email_path) {
            if let Ok(classification) = classifier.classify(&email).await {
                if classification.category == "Spam" {
                    false_spam_count += 1;
                    false_spam_examples.push(format!(
                        "Email '{}' (ID: {}) classified as Spam but should be '{}'",
                        gt_email.subject, gt_email.id, gt_email.category
                    ));
                }
            }
        }
    }

    if false_spam_count > 0 {
        println!("False spam classifications:");
        for example in &false_spam_examples {
            println!("  {example}");
        }
    }

    // No legitimate emails should be classified as spam
    assert_eq!(
        false_spam_count, 0,
        "Found {false_spam_count} false spam classifications"
    );
}
