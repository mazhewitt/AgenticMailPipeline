use agentic_mail_agent::classifier::{MessageClassifier, StubClassifier};
use agentic_mail_agent::email::Email;

#[tokio::main]
async fn main() {
    let classifier = StubClassifier::deterministic();
    
    // Test the failing emails one by one
    let test_emails = vec![
        Email::new_full(
            "test1".to_string(),
            Some("Weekly Sports Roundup".to_string()),
            Some("This week in sports - scores and highlights".to_string()),
            Some("sports@newsletter.com".to_string()),
            None, None, None,
        ),
        Email::new_full(
            "test2".to_string(),
            Some("📍 Someone was at your location".to_string()),
            Some("Cameron Garcia was at 166 Elm Street, Georgetown, WA 10066".to_string()),
            Some("location@tracker.com".to_string()),
            None, None, None,
        ),
        Email::new_full(
            "test3".to_string(),
            Some("Weekly Tech Newsletter".to_string()),
            Some("Latest tech news and startup updates".to_string()),
            Some("tech@newsletter.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in test_emails {
        let classification = classifier.classify(&email).await.unwrap();
        println!(
            "Subject: '{}' -> Category: '{}', Reason: '{}'",
            email.subject.as_deref().unwrap_or("unknown"),
            classification.category,
            classification.llm_response
        );
    }
}