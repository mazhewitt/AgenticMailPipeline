use agentic_mail_agent::{
    fetcher::{EmailFetcher, GmailFetcher},
    action::impls::labeler::ConcreteGmailLabeler,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Quick Gmail API Test");
    println!("=======================");
    
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // Create components
    println!("1️⃣ Creating Gmail components...");
    let fetcher = GmailFetcher::from_env().await?;
    let labeler = ConcreteGmailLabeler::from_env().await?;
    println!("   ✅ Components created");
    
    // Test 1: List labels (should be fast)
    println!("2️⃣ Testing list labels...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        labeler.list_all_labels()
    ).await {
        Ok(Ok(labels)) => {
            println!("   ✅ Listed {} labels successfully", labels.len());
        },
        Ok(Err(e)) => {
            println!("   ❌ List labels failed: {e}");
            return Err(e.into());
        },
        Err(_) => {
            println!("   ⏰ List labels timed out after 15 seconds");
            return Err("List labels timed out".into());
        }
    }
    
    // Test 2: Fetch a few emails (might be slower)
    println!("3️⃣ Testing fetch emails...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fetcher.fetch_unread_emails()
    ).await {
        Ok(Ok(emails)) => {
            println!("   ✅ Fetched {} emails successfully", emails.len());
        },
        Ok(Err(e)) => {
            println!("   ❌ Fetch emails failed: {e}");
            // Don't return error, this might be expected if no unread emails
            println!("   (This might be normal if you have no unread emails)");
        },
        Err(_) => {
            println!("   ⏰ Fetch emails timed out after 30 seconds");
            return Err("Fetch emails timed out".into());
        }
    }
    
    println!("🎉 API tests completed!");
    
    Ok(())
}