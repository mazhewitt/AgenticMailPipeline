use agentic_mail_agent::fetcher::{EmailFetcher, GmailFetcher, StubFetcher};
use agentic_mail_agent::classifier::{MessageClassifier, StubClassifier};
use agentic_mail_agent::action_router::{ActionRouter, RuleBasedRouter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("Hello, Agentic Gmail Agent!");
    
    // Try to use real Gmail fetcher if environment variables are set,
    // otherwise use stub fetcher for development
    let fetcher: Box<dyn EmailFetcher> = match GmailFetcher::from_env() {
        Ok(gmail_fetcher) => {
            println!("Using Gmail API fetcher");
            Box::new(gmail_fetcher)
        }
        Err(_) => {
            println!("Gmail credentials not found, using stub fetcher with demo data");
            // Create some demo emails to show the complete agentic functionality
            use agentic_mail_agent::email::Email;
            let demo_emails = vec![
                Email::new(
                    "demo-1".to_string(),
                    Some("Welcome to Agentic Mail Agent".to_string()),
                    Some("This is a demo email showing the new subject and snippet functionality. The agent can now extract both the email subject line and a preview of the email content.".to_string())
                ),
                Email::new(
                    "demo-2".to_string(), 
                    Some("URGENT: Meeting Reminder".to_string()),
                    Some("Don't forget about the URGENT team meeting tomorrow at 2 PM. We'll be discussing the new email processing features. Action required!".to_string())
                ),
                Email::new(
                    "demo-3".to_string(),
                    Some("Weekly Newsletter".to_string()),
                    Some("Check out this week's updates from our team. Newsletter content with promotions and news.".to_string())
                ),
                Email::new(
                    "demo-4".to_string(),
                    Some("Suspicious offer - You won $1,000,000!".to_string()),
                    Some("Click here to claim your prize! Limited time offer. Send us your bank details now.".to_string())
                ),
                Email::with_id("demo-5".to_string()), // Email with no subject/snippet
            ];
            Box::new(StubFetcher::with_emails(demo_emails))
        }
    };
    
    match fetcher.fetch_unread_emails().await {
        Ok(emails) => {
            println!("Fetched {} unread emails.", emails.len());
            println!("Starting agentic email processing pipeline...\n");
            
            // Initialize classifier and action router
            let classifier = StubClassifier::deterministic();
            let router = RuleBasedRouter::new();
            
            for (index, email) in emails.iter().enumerate() {
                println!("📧 Processing Email {} of {}:", index + 1, emails.len());
                println!("  ID: {}", email.id);
                if let Some(subject) = &email.subject {
                    println!("  Subject: {}", subject);
                } else {
                    println!("  Subject: (No subject)");
                }
                if let Some(snippet) = &email.snippet {
                    println!("  Preview: {}", snippet);
                } else {
                    println!("  Preview: (No preview)");
                }
                
                // Step 1: Classify the email
                match classifier.classify(email).await {
                    Ok(classification) => {
                        println!("  🏷️  Classification: {} (confidence: {:.2})", 
                                classification.category,
                                classification.score.unwrap_or(0.0));
                        if !classification.llm_response.is_empty() {
                            println!("  🤖 Analysis: {}", classification.llm_response);
                        }
                        
                        // Step 2: Route to actions based on classification
                        match router.route(email, &classification).await {
                            Ok(routing_result) => {
                                println!("  🎯 Actions: {}", routing_result.actions_summary());
                                println!("  💭 Reasoning: {}", routing_result.reasoning);
                                if routing_result.has_high_priority_actions() {
                                    println!("  ⚠️  HIGH PRIORITY ACTIONS DETECTED!");
                                }
                            },
                            Err(e) => {
                                println!("  ❌ Routing error: {}", e);
                            }
                        }
                    },
                    Err(e) => {
                        println!("  ❌ Classification error: {}", e);
                    }
                }
                println!("  ---\n");
            }
            
            println!("✅ Agentic email processing complete!");
        },
        Err(e) => eprintln!("Failed to fetch emails: {}", e),
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentic_mail_agent::email::Email;
    use agentic_mail_agent::fetcher::StubFetcher;
    use agentic_mail_agent::classifier::ClassificationError;

    #[tokio::test]
    async fn test_fetch_and_classify_pipeline() {
        // Setup test emails with different types of content
        let test_emails = vec![
            Email::new(
                "work-email".to_string(),
                Some("Team Meeting".to_string()),
                Some("We have a meeting scheduled for tomorrow to discuss the project.".to_string())
            ),
            Email::new(
                "urgent-email".to_string(),
                Some("URGENT: Action Required".to_string()),
                Some("Please respond ASAP to this urgent matter.".to_string())
            ),
            Email::new(
                "promo-email".to_string(),
                Some("Special Offer".to_string()),
                Some("Unsubscribe from this promotional newsletter at any time.".to_string())
            ),
        ];
        
        // Setup fetcher and classifier
        let fetcher = StubFetcher::with_emails(test_emails);
        let classifier = StubClassifier::deterministic();
        
        // Test the pipeline
        let emails = fetcher.fetch_unread_emails().await
            .expect("Should fetch emails successfully");
        
        assert_eq!(emails.len(), 3);
        
        // Test classification for each email
        let work_classification = classifier.classify(&emails[0]).await
            .expect("Should classify work email");
        assert_eq!(work_classification.category, "work");
        assert_eq!(work_classification.score, Some(0.9));
        
        let urgent_classification = classifier.classify(&emails[1]).await
            .expect("Should classify urgent email");
        assert_eq!(urgent_classification.category, "urgent");
        assert_eq!(urgent_classification.score, Some(0.95));
        
        let promo_classification = classifier.classify(&emails[2]).await
            .expect("Should classify promotional email");
        assert_eq!(promo_classification.category, "promotional");
        assert_eq!(promo_classification.score, Some(0.85));
    }
    
    #[tokio::test]
    async fn test_pipeline_with_empty_emails() {
        // Test pipeline with empty email list
        let fetcher = StubFetcher::new(); // Returns empty list by default
        let _classifier = StubClassifier::deterministic();
        
        let emails = fetcher.fetch_unread_emails().await
            .expect("Should handle empty email list");
        
        assert_eq!(emails.len(), 0);
    }
    
    #[tokio::test]
    async fn test_pipeline_with_classification_error() {
        // Test pipeline when classification fails
        let test_emails = vec![
            Email::new(
                "test-email".to_string(),
                Some("Test Email".to_string()),
                Some("This is a test email.".to_string())
            ),
        ];
        
        let fetcher = StubFetcher::with_emails(test_emails);
        let classifier = StubClassifier::with_fixed_error(
            ClassificationError::llm_service("Test error")
        );
        
        let emails = fetcher.fetch_unread_emails().await
            .expect("Should fetch emails successfully");
        
        assert_eq!(emails.len(), 1);
        
        // Classification should fail with expected error
        let result = classifier.classify(&emails[0]).await;
        assert!(result.is_err());
        
        if let Err(error) = result {
            assert_eq!(error, ClassificationError::llm_service("Test error"));
        }
    }
    
    #[tokio::test]
    async fn test_pipeline_with_missing_email_content() {
        // Test pipeline with emails that have no subject/snippet
        let test_emails = vec![
            Email::new("empty-email".to_string(), None, None),
            Email::new(
                "partial-email".to_string(),
                Some("Subject only".to_string()),
                None
            ),
        ];
        
        let fetcher = StubFetcher::with_emails(test_emails);
        let classifier = StubClassifier::deterministic();
        
        let emails = fetcher.fetch_unread_emails().await
            .expect("Should fetch emails successfully");
        
        assert_eq!(emails.len(), 2);
        
        // Both should get default work classification
        for email in emails {
            let classification = classifier.classify(&email).await
                .expect("Should classify email even without content");
            assert_eq!(classification.category, "work"); // default category
        }
    }
}
