//! Integration tests for action execution based on email classification.
//!
//! This test suite validates that the agentic mail agent correctly:
//! 1. Applies Gmail labels based on classification categories
//! 2. Archives all emails except ActionRequired
//! 3. Keeps ActionRequired emails in the inbox with proper labels

use agentic_mail_agent::action::executor::{
    get_label_for_category, ActionExecutor, GmailActionExecutor,
};
use agentic_mail_agent::action::impls::archiver::StubArchiver;
use agentic_mail_agent::action::impls::labeler::StubLabeler;
use agentic_mail_agent::classifier::{Classification, EmailCategory};
use agentic_mail_agent::core::email::Email;

/// Test action executor that applies labels and archives based on classification
type TestActionExecutor = GmailActionExecutor<StubLabeler, StubArchiver>;

/// Helper function to create a test action executor
fn create_test_executor() -> TestActionExecutor {
    let labeler = StubLabeler::new();
    let archiver = StubArchiver::new();
    GmailActionExecutor::new(labeler, archiver)
}

/// Test that ActionRequired emails get labeled but stay in inbox
#[tokio::test]
async fn test_action_required_emails_stay_in_inbox() {
    let executor = create_test_executor();

    let email = Email::new(
        "msg123".to_string(),
        Some("[Repo] CI failed: main".to_string()),
        Some("CI workflow run failed with 5 errors".to_string()),
    );

    let classification = Classification::with_category(EmailCategory::ActionRequired);

    let result = executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Should apply Action Required label (hierarchical format)
    assert_eq!(result.label_applied, "Agentic/Action Required");

    // Should NOT archive (keep in inbox)
    assert!(!result.archived);
    assert!(result
        .actions_taken
        .iter()
        .any(|action| action.contains("Kept in inbox")));
    assert!(!result
        .actions_taken
        .iter()
        .any(|action| action.contains("Archived")));
}

/// Test that InterestingInfo emails get labeled and archived
#[tokio::test]
async fn test_interesting_info_emails_get_archived() {
    let executor = create_test_executor();

    let email = Email::new(
        "msg456".to_string(),
        Some("Security Alert: Unusual Activity".to_string()),
        Some("We detected unusual activity on your account".to_string()),
    );

    let classification = Classification::with_category(EmailCategory::InterestingInfo);

    let result = executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Should apply Interesting label
    assert_eq!(result.label_applied, "Agentic/Interesting");

    // Should archive (remove from inbox)
    assert!(result.archived);
    assert!(result
        .actions_taken
        .iter()
        .any(|action| action.contains("Archived")));
    assert!(!result
        .actions_taken
        .iter()
        .any(|action| action.contains("Kept in inbox")));
}

/// Test that Reference emails get labeled and archived
#[tokio::test]
async fn test_reference_emails_get_archived() {
    let executor = create_test_executor();

    let email = Email::new(
        "msg789".to_string(),
        Some("Your receipt from Company".to_string()),
        Some("Thank you for your purchase, here is your receipt".to_string()),
    );

    let classification = Classification::with_category(EmailCategory::Reference);

    let result = executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Should apply Reference label
    assert_eq!(result.label_applied, "Agentic/Reference");

    // Should archive (remove from inbox)
    assert!(result.archived);
    assert!(result
        .actions_taken
        .iter()
        .any(|action| action.contains("Archived")));
}

/// Test that Noise emails get labeled and archived
#[tokio::test]
async fn test_noise_emails_get_archived() {
    let executor = create_test_executor();

    let email = Email::new(
        "msg101".to_string(),
        Some("Follow Casey Johnson - CEO at Google".to_string()),
        Some("See your recommendations on LinkedIn".to_string()),
    );

    let classification = Classification::with_category(EmailCategory::Noise);

    let result = executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Should apply Low Priority label
    assert_eq!(result.label_applied, "Agentic/Low Priority");

    // Should archive (remove from inbox)
    assert!(result.archived);
    assert!(result
        .actions_taken
        .iter()
        .any(|action| action.contains("Archived")));
}

/// Test that Spam emails get labeled and archived
#[tokio::test]
async fn test_spam_emails_get_archived() {
    let executor = create_test_executor();

    let email = Email::new(
        "msg202".to_string(),
        Some("You've won $1,000,000! Click here!".to_string()),
        Some("Claim your prize now by clicking this suspicious link".to_string()),
    );

    let classification = Classification::with_category(EmailCategory::Spam);

    let result = executor
        .execute_actions(&email, &classification)
        .await
        .unwrap();

    // Should apply Spam label
    assert_eq!(result.label_applied, "Agentic/Spam");

    // Should archive (remove from inbox)
    assert!(result.archived);
    assert!(result
        .actions_taken
        .iter()
        .any(|action| action.contains("Archived")));
}

/// Test all 5 categories have correct hierarchical label mapping
#[tokio::test]
async fn test_all_category_labels() {
    // Test all 5 current categories with new hierarchical labels
    assert_eq!(
        get_label_for_category(&EmailCategory::ActionRequired),
        "Agentic/Action Required"
    );
    assert_eq!(
        get_label_for_category(&EmailCategory::InterestingInfo),
        "Agentic/Interesting"
    );
    assert_eq!(
        get_label_for_category(&EmailCategory::Reference),
        "Agentic/Reference"
    );
    assert_eq!(
        get_label_for_category(&EmailCategory::Noise),
        "Agentic/Low Priority"
    );
    assert_eq!(get_label_for_category(&EmailCategory::Spam), "Agentic/Spam");
}

/// Integration test: Process multiple emails with different classifications
#[tokio::test]
async fn test_batch_email_processing() {
    let executor = create_test_executor();

    let test_cases = vec![
        (
            "msg1",
            EmailCategory::ActionRequired,
            "Meeting tomorrow",
            false,
        ), // should not archive
        (
            "msg2",
            EmailCategory::InterestingInfo,
            "Tech newsletter",
            true,
        ), // should archive
        ("msg3", EmailCategory::Reference, "Receipt", true), // should archive
        ("msg4", EmailCategory::Noise, "Social notification", true), // should archive
        ("msg5", EmailCategory::Spam, "Suspicious offer", true), // should archive
    ];

    let mut inbox_count = 0;
    let mut archived_count = 0;

    for (msg_id, category, subject, should_archive) in test_cases {
        let email = Email::new(msg_id.to_string(), Some(subject.to_string()), None);
        let classification = Classification::with_category(category);

        let result = executor
            .execute_actions(&email, &classification)
            .await
            .unwrap();

        // Verify correct label applied
        let expected_label = get_label_for_category(&category);
        assert_eq!(result.label_applied, expected_label);

        // Count inbox vs archived
        if should_archive {
            assert!(result.archived);
            archived_count += 1;
        } else {
            assert!(!result.archived);
            inbox_count += 1;
        }
    }

    // Only ActionRequired should remain in inbox
    assert_eq!(inbox_count, 1);
    assert_eq!(archived_count, 4);
}
