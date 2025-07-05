use agentic_mail_agent::{
    fetcher::{EmailFetcher, GmailFetcher},
    action::impls::labeler::{ConcreteGmailLabeler, EmailLabeler},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏷️ Minimal Label Test");
    println!("=====================");
    
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // Get components
    let fetcher = GmailFetcher::from_env().await?;
    let labeler = ConcreteGmailLabeler::from_env().await?;
    
    // Get first email
    let emails = fetcher.fetch_unread_emails().await?;
    if emails.is_empty() {
        println!("No unread emails found");
        return Ok(());
    }
    
    let email = &emails[0];
    println!("📧 Found email: {:?}", email.subject);
    
    // Apply simple test label
    let test_label = "TEST_MINIMAL_LABEL";
    println!("🏷️ Applying label: {}", test_label);
    
    match labeler.apply_label(&email.id, test_label).await {
        Ok(result) => {
            println!("✅ SUCCESS! Label applied. Created new: {}", result.created_new_label);
        },
        Err(e) => {
            println!("❌ FAILED: {}", e);
            return Err(e.into());
        }
    }
    
    println!("🎉 Minimal label test completed!");
    println!("Note: You said you'll clean up the label manually");
    
    Ok(())
}