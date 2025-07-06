//! Unit tests using mock Ollama classifier with recorded responses
//!
//! These tests replay previously recorded LLM responses to test classification
//! logic without requiring a live Ollama instance.

use agentic_mail_agent::classifier::{EmailCategory, MessageClassifier, MockOllamaClassifier};
use agentic_mail_agent::core::email::Email;

/// Test classification using recorded individual examples
#[tokio::test]
async fn test_mock_classification_individual_examples() {
    // Create mock classifier in replay mode
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/individual_examples.json",
    )
    .expect("Failed to load recorded responses");

    let (total, categories) = mock_classifier.get_stats();
    println!("📊 Loaded {total} recorded responses with categories: {categories:?}");

    // Test urgent email classification
    let urgent_email = Email::new_full(
        "urgent001".to_string(),
        Some("URGENT: Server Down".to_string()),
        Some("Production server crashed, need immediate action".to_string()),
        Some("ops@company.com".to_string()),
        None,
        None,
        None,
    );

    let classification = mock_classifier.classify(&urgent_email).await.unwrap();
    assert_eq!(classification.category, EmailCategory::ActionRequired);
    assert!(classification.score.unwrap_or(0.0) > 0.9);
    assert!(classification
        .llm_response
        .to_lowercase()
        .contains("urgent"));

    // Test newsletter classification
    let newsletter_email = Email::new_full(
        "newsletter001".to_string(),
        Some("Weekly Tech Newsletter".to_string()),
        Some("Latest tech trends and AI developments".to_string()),
        Some("tech@newsletter.com".to_string()),
        None,
        None,
        None,
    );

    let classification = mock_classifier.classify(&newsletter_email).await.unwrap();
    assert_eq!(classification.category, EmailCategory::InterestingInfo);
    assert!(
        classification.llm_response.to_lowercase().contains("tech")
            || classification
                .llm_response
                .to_lowercase()
                .contains("developments")
    );

    // Test spam classification
    let spam_email = Email::new_full(
        "spam001".to_string(),
        Some("You've won $1 Million!".to_string()),
        Some("Click here to claim your prize now!".to_string()),
        Some("noreply@scam.com".to_string()),
        None,
        None,
        None,
    );

    let classification = mock_classifier.classify(&spam_email).await.unwrap();
    assert_eq!(classification.category, EmailCategory::Spam);
    assert!(classification.score.unwrap_or(0.0) >= 0.95);

    // Test receipt classification
    let receipt_email = Email::new_full(
        "receipt001".to_string(),
        Some("Order Confirmation #12345".to_string()),
        Some("Thank you for your order. Your receipt is attached.".to_string()),
        Some("orders@shop.com".to_string()),
        None,
        None,
        None,
    );

    let classification = mock_classifier.classify(&receipt_email).await.unwrap();
    assert_eq!(classification.category, EmailCategory::Reference);
    assert!(classification.llm_response.contains("receipt"));
}

/// Test error handling when email signature doesn't match recorded responses
#[tokio::test]
async fn test_mock_classification_unknown_email() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/individual_examples.json",
    )
    .expect("Failed to load recorded responses");

    // Create an email not in the recorded responses
    let unknown_email = Email::new_full(
        "unknown999".to_string(),
        Some("Unknown Subject".to_string()),
        Some("This email was never recorded".to_string()),
        Some("unknown@example.com".to_string()),
        None,
        None,
        None,
    );

    let result = mock_classifier.classify(&unknown_email).await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("No recorded response found"));
}

/// Test that mock classifier provides deterministic results
#[tokio::test]
async fn test_mock_classification_deterministic() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/individual_examples.json",
    )
    .expect("Failed to load recorded responses");

    let email = Email::new_full(
        "urgent001".to_string(),
        Some("URGENT: Server Down".to_string()),
        Some("Production server crashed, need immediate action".to_string()),
        Some("ops@company.com".to_string()),
        None,
        None,
        None,
    );

    // Classify the same email multiple times
    let classification1 = mock_classifier.classify(&email).await.unwrap();
    let classification2 = mock_classifier.classify(&email).await.unwrap();
    let classification3 = mock_classifier.classify(&email).await.unwrap();

    // Results should be identical
    assert_eq!(classification1.category, classification2.category);
    assert_eq!(classification1.category, classification3.category);
    assert_eq!(classification1.score, classification2.score);
    assert_eq!(classification1.score, classification3.score);
    assert_eq!(classification1.llm_response, classification2.llm_response);
    assert_eq!(classification1.llm_response, classification3.llm_response);
}

/// Test classification categories match expected patterns
#[tokio::test]
async fn test_mock_classification_category_patterns() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/individual_examples.json",
    )
    .expect("Failed to load recorded responses");

    let (total, categories) = mock_classifier.get_stats();
    assert_eq!(total, 4); // Should have 4 recorded responses

    // Check that we have the expected categories
    let expected_categories = vec![
        "ActionRequired".to_string(),
        "InterestingInfo".to_string(),
        "Spam".to_string(),
        "Reference".to_string(),
    ];

    for expected_category in expected_categories {
        assert!(
            categories.contains(&expected_category),
            "Missing expected category: {expected_category}. Available: {categories:?}"
        );
    }
}

/// Test that recorded responses contain LLM reasoning
#[tokio::test]
async fn test_mock_classification_contains_reasoning() {
    let mock_classifier = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/individual_examples.json",
    )
    .expect("Failed to load recorded responses");

    let spam_email = Email::new_full(
        "spam001".to_string(),
        Some("You've won $1 Million!".to_string()),
        Some("Click here to claim your prize now!".to_string()),
        Some("noreply@scam.com".to_string()),
        None,
        None,
        None,
    );

    let classification = mock_classifier.classify(&spam_email).await.unwrap();

    // Should contain detailed LLM reasoning
    assert!(classification.llm_response.len() > 50); // Should be substantial
    assert!(classification.llm_response.contains("LLM Response:")); // Should have proper format
    assert!(
        classification.llm_response.to_lowercase().contains("scam")
            || classification
                .llm_response
                .to_lowercase()
                .contains("suspicious")
    ); // Should identify spam characteristics
}
