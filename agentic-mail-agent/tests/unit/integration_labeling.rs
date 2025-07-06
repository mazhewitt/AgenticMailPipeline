//! Integration tests for the labeling functionality

use agentic_mail_agent::action::executor::{ActionExecutor, StubActionExecutor};
use agentic_mail_agent::action::impls::labeler::{EmailLabeler, StubLabeler};
use agentic_mail_agent::classifier::{EmailCategory, MessageClassifier, StubClassifier};
use agentic_mail_agent::core::email::Email;

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
    let classification = classifier
        .classify(&email)
        .await
        .expect("Classification should succeed");

    assert_eq!(classification.category, EmailCategory::ActionRequired); // Meeting = ActionRequired
    assert_eq!(classification.score, Some(0.9));

    // Execute actions (label and archive)
    let action_executor = StubActionExecutor::new();
    let result = action_executor
        .execute_actions(&email, &classification)
        .await
        .expect("Action execution should succeed");

    // Verify action execution results
    assert_eq!(result.message_id, "test-123");
    assert_eq!(result.label_applied, "Action Required");
    assert!(!result.archived); // ActionRequired emails should not be archived
    assert!(result
        .actions_taken
        .iter()
        .any(|action| action.contains("Applied label")));
    assert!(result
        .actions_taken
        .iter()
        .any(|action| action.contains("Kept in inbox")));
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

    // Should be classified as ActionRequired
    assert_eq!(classification.category, EmailCategory::ActionRequired);

    // Execute actions
    let action_executor = StubActionExecutor::new();
    let result = action_executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Verify results
    assert_eq!(result.message_id, "urgent-456");
    assert_eq!(result.label_applied, "Action Required");
    assert!(!result.archived); // ActionRequired should not be archived
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

    // This email should be classified as Reference (fallback for unmatched content)
    assert_eq!(classification.category, EmailCategory::Reference); // Falls back to Reference
    assert_eq!(classification.score, Some(0.6)); // Default score

    // Execute actions
    let action_executor = StubActionExecutor::new();
    let result = action_executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Verify results
    assert_eq!(result.message_id, "spam-789");
    assert_eq!(result.label_applied, "Reference");
    assert!(result.archived); // Reference should be archived
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

    assert_eq!(classification.category, EmailCategory::InterestingInfo); // Newsletter with "news" = InterestingInfo

    // Execute actions
    let action_executor = StubActionExecutor::new();
    let result = action_executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Verify results
    assert_eq!(result.message_id, "newsletter-101");
    assert_eq!(result.label_applied, "Interesting");
    assert!(result.archived); // InterestingInfo should be archived
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
    let result1 = labeler.apply_label(&email.id, "TEST_Work").await.unwrap();
    assert!(result1.created_new_label);
    assert_eq!(result1.label, "TEST_Work");

    // Apply same label again - should be idempotent
    let result2 = labeler.apply_label(&email.id, "TEST_Work").await.unwrap();
    assert!(!result2.created_new_label);
    assert_eq!(result2.label, "TEST_Work");

    // Verify only one instance
    let applied_labels = labeler.get_applied_labels(&email.id);
    assert_eq!(applied_labels.len(), 1);
    assert_eq!(applied_labels[0], "TEST_Work");
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
    labeler.apply_label(&email.id, "Work").await.unwrap();
    labeler.apply_label(&email.id, "Urgent").await.unwrap();

    // Verify both labels are applied
    assert!(labeler.message_has_label(&email.id, "Work"));
    assert!(labeler.message_has_label(&email.id, "Urgent"));

    let applied_labels = labeler.get_applied_labels(&email.id);
    assert_eq!(applied_labels.len(), 2);
    assert!(applied_labels.contains(&"Work".to_string()));
    assert!(applied_labels.contains(&"Urgent".to_string()));
}

#[tokio::test]
async fn test_get_label_for_category() {
    let labeler = StubLabeler::new();

    // Test all category mappings
    assert_eq!(labeler.get_label_for_category("work"), "Work");
    assert_eq!(labeler.get_label_for_category("personal"), "Personal");
    assert_eq!(labeler.get_label_for_category("promotional"), "Promotional");
    assert_eq!(labeler.get_label_for_category("spam"), "Spam");
    assert_eq!(labeler.get_label_for_category("newsletter"), "Newsletter");
    assert_eq!(labeler.get_label_for_category("urgent"), "Urgent");
}
