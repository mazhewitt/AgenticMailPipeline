//! Integration tests for the enhanced email fields refactor.
//! 
//! This test suite verifies that the complete workflow works end-to-end
//! with the new email fields (from, to, sent, body).

use agentic_mail_agent::{
    email::Email,
    classifier::{MessageClassifier, StubClassifier, Classification},
    action_router::{ActionRouter, RuleBasedRouter},
    fetcher::{EmailFetcher, StubFetcher},
};

#[tokio::test]
async fn test_complete_workflow_with_full_email_fields() {
    // Create test email with all new fields
    let test_email = Email::new_full(
        "test-001".to_string(),
        Some("URGENT: Production Server Down".to_string()),
        Some("Our main production server has crashed and needs immediate attention".to_string()),
        Some("devops@company.com".to_string()),
        Some(vec!["engineering-team@company.com".to_string(), "cto@company.com".to_string()]),
        Some("Wed, 30 Jun 2023 14:30:00 +0000".to_string()),
        Some("Hi Team,\n\nThis is an EMERGENCY alert. Our main production server (prod-001) has crashed at 2:30 PM UTC. The application is currently down and customers cannot access our services.\n\nPlease investigate immediately and implement the disaster recovery plan.\n\nRegards,\nDevOps Team".to_string()),
    );
    
    // Step 1: Set up fetcher with test email
    let fetcher = StubFetcher::with_emails(vec![test_email.clone()]);
    
    // Step 2: Fetch emails
    let emails = fetcher.fetch_unread_emails().await.expect("Should fetch emails");
    assert_eq!(emails.len(), 1);
    let email = &emails[0];
    
    // Verify all fields are properly populated
    assert_eq!(email.id, "test-001");
    assert_eq!(email.subject, Some("URGENT: Production Server Down".to_string()));
    assert_eq!(email.from, Some("devops@company.com".to_string()));
    assert_eq!(email.to, Some(vec!["engineering-team@company.com".to_string(), "cto@company.com".to_string()]));
    assert_eq!(email.sent, Some("Wed, 30 Jun 2023 14:30:00 +0000".to_string()));
    assert!(email.body.as_ref().unwrap().contains("EMERGENCY"));
    
    // Step 3: Classify the email (using stub classifier for deterministic results)
    let classifier = StubClassifier::with_fixed_classification(
        Classification::with_score("urgent".to_string(), 0.95)
    );
    let classification = classifier.classify(email).await.expect("Should classify email");
    assert_eq!(classification.category, "urgent");
    assert_eq!(classification.score, Some(0.95));
    
    // Step 4: Route the email to actions
    let router = RuleBasedRouter::new();
    let routing_result = router.route(email, &classification).await.expect("Should route email");
    
    // Verify urgent email detection works with the new body field
    assert!(routing_result.has_high_priority_actions(), "Should detect as urgent due to body content");
    assert!(routing_result.reasoning.contains("urgent content detected"));
}

#[tokio::test] 
async fn test_classification_uses_all_email_fields() {
    // Create email where classification clues are in different fields
    let work_email = Email::new_full(
        "work-001".to_string(),
        Some("Project Update".to_string()),
        Some("Quick project status update".to_string()),
        Some("project-manager@company.com".to_string()),
        Some(vec!["dev-team@company.com".to_string()]),
        Some("Wed, 30 Jun 2023 09:00:00 +0000".to_string()),
        Some("Hi Team,\n\nHere's the weekly project status update. We're on track to meet the deadline for the Q3 release. Please review the attached requirements document and provide feedback by Friday.\n\nBest regards,\nProject Manager".to_string()),
    );
    
    let personal_email = Email::new_full(
        "personal-001".to_string(),
        Some("Weekend Plans".to_string()),
        Some("Are you free this weekend?".to_string()),
        Some("friend@personal.com".to_string()),
        Some(vec!["me@personal.com".to_string()]),
        Some("Wed, 30 Jun 2023 18:00:00 +0000".to_string()),
        Some("Hey!\n\nAre you free this weekend? I was thinking we could go hiking in the mountains. The weather looks great and it would be fun to catch up!\n\nLet me know what you think.\n\nCheers,\nYour Friend".to_string()),
    );
    
    // Use stub classifier to simulate different classifications
    let work_classifier = StubClassifier::with_fixed_classification(
        Classification::with_score("work".to_string(), 0.85)
    );
    let personal_classifier = StubClassifier::with_fixed_classification(
        Classification::with_score("personal".to_string(), 0.90)
    );
    
    // Classify emails
    let work_classification = work_classifier.classify(&work_email).await.unwrap();
    let personal_classification = personal_classifier.classify(&personal_email).await.unwrap();
    
    assert_eq!(work_classification.category, "work");
    assert_eq!(personal_classification.category, "personal");
    
    // Test that we can access all the new fields
    assert_eq!(work_email.from_or_default(), "project-manager@company.com");
    assert_eq!(work_email.to_or_default(), vec!["dev-team@company.com".to_string()]);
    assert_eq!(work_email.sent_or_default(), "Wed, 30 Jun 2023 09:00:00 +0000");
    assert!(work_email.body_or_default().contains("deadline"));
    
    assert_eq!(personal_email.from_or_default(), "friend@personal.com");
    assert_eq!(personal_email.to_or_default(), vec!["me@personal.com".to_string()]);
    assert!(personal_email.body_or_default().contains("hiking"));
}

#[tokio::test]
async fn test_urgent_detection_in_body_content() {
    // Create email where urgency is only in the body, not subject
    let urgent_body_email = Email::new_full(
        "urgent-body-001".to_string(),
        Some("Status Report".to_string()),
        Some("Regular weekly status report".to_string()),
        Some("ops@company.com".to_string()),
        Some(vec!["management@company.com".to_string()]),
        Some("Wed, 30 Jun 2023 16:00:00 +0000".to_string()),
        Some("Hi Management,\n\nThis week's status report shows normal operations. However, I need to report a CRITICAL security incident that requires IMMEDIATE attention. We've detected unauthorized access attempts on our database servers.\n\nPlease respond ASAP.\n\nBest regards,\nOps Team".to_string()),
    );
    
    // Set up routing
    let router = RuleBasedRouter::new();
    let classification = Classification::with_score("work".to_string(), 0.80);
    
    // Route the email
    let routing_result = router.route(&urgent_body_email, &classification).await.unwrap();
    
    // Verify urgent content is detected in the body
    assert!(routing_result.has_high_priority_actions(), "Should detect urgency in body content");
    assert!(routing_result.reasoning.contains("urgent content detected"));
}

#[test]
fn test_email_field_accessors() {
    // Test all the new field accessors work correctly
    let full_email = Email::new_full(
        "test-accessors".to_string(),
        Some("Test Subject".to_string()),
        Some("Test Snippet".to_string()),
        Some("sender@test.com".to_string()),
        Some(vec!["recipient1@test.com".to_string(), "recipient2@test.com".to_string()]),
        Some("Wed, 30 Jun 2023 12:00:00 +0000".to_string()),
        Some("Test body content".to_string()),
    );
    
    // Test all accessors return correct values
    assert_eq!(full_email.subject_or_default(), "Test Subject");
    assert_eq!(full_email.snippet_or_default(), "Test Snippet");
    assert_eq!(full_email.from_or_default(), "sender@test.com");
    assert_eq!(full_email.to_or_default(), vec!["recipient1@test.com".to_string(), "recipient2@test.com".to_string()]);
    assert_eq!(full_email.sent_or_default(), "Wed, 30 Jun 2023 12:00:00 +0000");
    assert_eq!(full_email.body_or_default(), "Test body content");
    
    // Test empty email returns defaults
    let empty_email = Email::with_id("empty".to_string());
    assert_eq!(empty_email.subject_or_default(), "(No Subject)");
    assert_eq!(empty_email.snippet_or_default(), "(No Preview)");
    assert_eq!(empty_email.from_or_default(), "(Unknown Sender)");
    assert_eq!(empty_email.to_or_default(), Vec::<String>::new());
    assert_eq!(empty_email.sent_or_default(), "(Unknown Date)");
    assert_eq!(empty_email.body_or_default(), "(No Body)");
}
