use agentic_mail_agent::{action::impls::labeler::ConcreteGmailLabeler, fetcher::GmailFetcher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Quick OAuth Authentication Test");
    println!("==================================");

    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Test 1: Try to create Gmail fetcher (read-only operations)
    println!("1️⃣ Testing Gmail Fetcher (readonly)...");
    match tokio::time::timeout(std::time::Duration::from_secs(10), GmailFetcher::from_env()).await {
        Ok(Ok(_fetcher)) => {
            println!("   ✅ Gmail Fetcher created successfully");
        }
        Ok(Err(e)) => {
            println!("   ❌ Gmail Fetcher failed: {e}");
            return Err(e.into());
        }
        Err(_) => {
            println!("   ⏰ Gmail Fetcher timed out - OAuth2 flow may be waiting for browser");
            return Err("Fetcher creation timed out".into());
        }
    }

    // Test 2: Try to create Gmail labeler (modify operations)
    println!("2️⃣ Testing Gmail Labeler (modify)...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ConcreteGmailLabeler::from_env(),
    )
    .await
    {
        Ok(Ok(_labeler)) => {
            println!("   ✅ Gmail Labeler created successfully");
        }
        Ok(Err(e)) => {
            println!("   ❌ Gmail Labeler failed: {e}");
            return Err(e.into());
        }
        Err(_) => {
            println!("   ⏰ Gmail Labeler timed out - OAuth2 flow may be waiting for browser");
            return Err("Labeler creation timed out".into());
        }
    }

    println!("🎉 Both components created successfully!");
    println!("   This means OAuth2 tokens are valid and have correct scopes");

    Ok(())
}
