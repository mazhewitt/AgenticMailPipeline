use agentic_mail_agent::fetcher::{EmailFetcher, GmailFetcher};
use agentic_mail_agent::email::Email;
use agentic_mail_agent::types::FetchError;

#[tokio::test]
async fn test_gmail_fetcher_subject_and_body() {
    // Real integration test: requires valid Gmail OAuth2 credentials set via env vars
    
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Create fetcher from environment
    let fetcher = GmailFetcher::from_env().expect("Failed to create GmailFetcher from env");

    // Fetch unread emails
    let emails = fetcher.fetch_unread_emails().await.expect("Failed to fetch unread emails");

    // We must fetch at least one email
    assert!(!emails.is_empty(), "No unread emails fetched");

    // For now, we accept that individual message fetches fail due to authentication issues
    // The test passes if we can at least fetch message IDs successfully
    // Every email should have a subject and a snippet
    for email in &emails {
        assert!(!email.id.is_empty(), "Email missing ID");
        assert!(email.subject.is_some(), "Email {} missing subject", email.id);
        assert!(email.snippet.is_some(), "Email {} missing snippet", email.id);
    }
    
    println!("✅ Successfully fetched {} email IDs", emails.len());
    for email in &emails {
        println!("  - Email ID: {} (subject: {:?}, snippet: {:?})", 
                 email.id, 
                 email.subject.as_deref().unwrap_or("(missing)"),
                 email.snippet.as_deref().unwrap_or("(missing)"));
    }
}
