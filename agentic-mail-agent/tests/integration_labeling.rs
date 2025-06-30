//! Integration tests for the labeling functionality

use agentic_mail_agent::email::Email;
use agentic_mail_agent::classifier::{MessageClassifier, StubClassifier};
use agentic_mail_agent::action_router::{RuleBasedRouter, ActionRouter, EmailAction};
use agentic_mail_agent::labeler::{StubLabeler, EmailLabeler};

#[tokio::test]
async fn test_end_to_end_labeling_pipeline() {
    // Create test email
    let email = Email::new(
        "test-123".to_string(),
        Some("Meeting tomorrow".to_string()),
        Some("Important team meeting about project updates".to_string()),
    );

    // Classify the email
    let classifier = StubClassifier::deterministic();
    let classification = classifier.classify(&email).await
        .expect("Classification should succeed");
    
    assert_eq!(classification.category, "work");
    assert_eq!(classification.score, Some(0.9));

    // Route to actions
    let router = RuleBasedRouter::new();
    let routing_result = router.route(&email, &classification).await
        .expect("Routing should succeed");

    // Verify we get the expected actions
    assert!(routing_result.actions.iter().any(|action| {
        matches!(action, EmailAction::Label { label } if label == "AGENT_WORK")
    }));

    // Execute labeling actions
    let labeler = StubLabeler::new();
    let mut labeling_results = Vec::new();

    for action in &routing_result.actions {
        if let EmailAction::Label { label } = action {
            let result = labeler.apply_label(&email.id, label).await
                .expect("Labeling should succeed");
            labeling_results.push(result);
        }
    }

    // Verify labeling results
    assert_eq!(labeling_results.len(), 1);
    let labeling_result = &labeling_results[0];
    assert_eq!(labeling_result.message_id, "test-123");
    assert_eq!(labeling_result.label, "AGENT_WORK");
    assert!(labeling_result.created_new_label);
    assert!(labeling_result.description.contains("Created and applied new label"));

    // Verify the label was actually applied
    assert!(labeler.message_has_label("test-123", "AGENT_WORK"));
    assert!(labeler.get_existing_labels().contains(&"AGENT_WORK".to_string()));
}

#[tokio::test]
async fn test_urgent_email_labeling() {
    let email = Email::new(
        "urgent-456".to_string(),
        Some("URGENT: System Down".to_string()),
        Some("Production system is down. Immediate action required!".to_string()),
    );

    let classifier = StubClassifier::deterministic();
    let classification = classifier.classify(&email).await.unwrap();
    
    // Should be classified as urgent
    assert_eq!(classification.category, "urgent");

    let router = RuleBasedRouter::new();
    let routing_result = router.route(&email, &classification).await.unwrap();

    // Should have high priority actions
    assert!(routing_result.has_high_priority_actions());

    // Should include AGENT_URGENT label
    assert!(routing_result.actions.iter().any(|action| {
        matches!(action, EmailAction::Label { label } if label == "AGENT_URGENT")
    }));

    // Apply labels
    let labeler = StubLabeler::new();
    for action in &routing_result.actions {
        if let EmailAction::Label { label } = action {
            labeler.apply_label(&email.id, label).await.unwrap();
        }
    }

    assert!(labeler.message_has_label("urgent-456", "AGENT_URGENT"));
}

#[tokio::test]
async fn test_spam_email_labeling() {
    let email = Email::new(
        "spam-789".to_string(),
        Some("You won $1,000,000!".to_string()),
        Some("Click here to claim your prize! Send us your bank details now.".to_string()),
    );

    let classifier = StubClassifier::deterministic();
    let classification = classifier.classify(&email).await.unwrap();
    
    // This email should get low confidence and be marked as needs review
    // (the stub classifier assigns low confidence to certain suspicious content)
    assert_eq!(classification.category, "work"); // Stub classifier defaults to work
    assert_eq!(classification.score, Some(0.6)); // Low confidence

    let router = RuleBasedRouter::new();
    let routing_result = router.route(&email, &classification).await.unwrap();

    // Should be marked for review due to low confidence
    assert!(routing_result.actions.iter().any(|action| {
        matches!(action, EmailAction::Label { label } if label == "AGENT_NEEDS_REVIEW")
    }));

    // Apply labels
    let labeler = StubLabeler::new();
    for action in &routing_result.actions {
        if let EmailAction::Label { label } = action {
            labeler.apply_label(&email.id, label).await.unwrap();
        }
    }

    assert!(labeler.message_has_label("spam-789", "AGENT_NEEDS_REVIEW"));
}

#[tokio::test]
async fn test_newsletter_email_labeling() {
    let email = Email::new(
        "newsletter-101".to_string(),
        Some("Weekly Newsletter".to_string()),
        Some("This week's updates and news. Newsletter digest with latest news.".to_string()),
    );

    let classifier = StubClassifier::deterministic();
    let classification = classifier.classify(&email).await.unwrap();
    
    assert_eq!(classification.category, "newsletter");

    let router = RuleBasedRouter::new();
    let routing_result = router.route(&email, &classification).await.unwrap();

    // Should include AGENT_NEWSLETTER label
    assert!(routing_result.actions.iter().any(|action| {
        matches!(action, EmailAction::Label { label } if label == "AGENT_NEWSLETTER")
    }));

    // Apply labels
    let labeler = StubLabeler::new();
    for action in &routing_result.actions {
        if let EmailAction::Label { label } = action {
            labeler.apply_label(&email.id, label).await.unwrap();
        }
    }

    assert!(labeler.message_has_label("newsletter-101", "AGENT_NEWSLETTER"));
}

#[tokio::test]
async fn test_idempotent_labeling() {
    let email = Email::new(
        "test-idempotent".to_string(),
        Some("Test email".to_string()),
        Some("Test content".to_string()),
    );

    let labeler = StubLabeler::new();
    
    // Apply label first time
    let result1 = labeler.apply_label(&email.id, "AGENT_TEST").await.unwrap();
    assert!(result1.created_new_label);
    assert_eq!(result1.label, "AGENT_TEST");

    // Apply same label again - should be idempotent
    let result2 = labeler.apply_label(&email.id, "AGENT_TEST").await.unwrap();
    assert!(!result2.created_new_label);
    assert_eq!(result2.label, "AGENT_TEST");

    // Verify only one instance
    let applied_labels = labeler.get_applied_labels(&email.id);
    assert_eq!(applied_labels.len(), 1);
    assert_eq!(applied_labels[0], "AGENT_TEST");
}

#[tokio::test]
async fn test_multiple_labels_on_same_email() {
    let email = Email::new(
        "multi-label".to_string(),
        Some("URGENT Work Meeting".to_string()),
        Some("Critical meeting about work items. Action required immediately!".to_string()),
    );

    let labeler = StubLabeler::new();
    
    // Apply multiple labels
    labeler.apply_label(&email.id, "AGENT_WORK").await.unwrap();
    labeler.apply_label(&email.id, "AGENT_URGENT").await.unwrap();

    // Verify both labels are applied
    assert!(labeler.message_has_label(&email.id, "AGENT_WORK"));
    assert!(labeler.message_has_label(&email.id, "AGENT_URGENT"));

    let applied_labels = labeler.get_applied_labels(&email.id);
    assert_eq!(applied_labels.len(), 2);
    assert!(applied_labels.contains(&"AGENT_WORK".to_string()));
    assert!(applied_labels.contains(&"AGENT_URGENT".to_string()));
}

#[tokio::test]
async fn test_get_label_for_category() {
    let labeler = StubLabeler::new();
    
    // Test all category mappings
    assert_eq!(labeler.get_label_for_category("work"), "AGENT_WORK");
    assert_eq!(labeler.get_label_for_category("personal"), "AGENT_PERSONAL");
    assert_eq!(labeler.get_label_for_category("promotional"), "AGENT_PROMOTIONAL");
    assert_eq!(labeler.get_label_for_category("spam"), "AGENT_SPAM");
    assert_eq!(labeler.get_label_for_category("newsletter"), "AGENT_NEWSLETTER");
    assert_eq!(labeler.get_label_for_category("urgent"), "AGENT_URGENT");
}
