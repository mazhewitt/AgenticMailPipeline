use std::time::Duration;
use tokio::time::timeout;
use agentic_mail_agent::{
    fetcher::GmailFetcher,
    action::impls::labeler::ConcreteGmailLabeler,
};

#[tokio::main]
async fn main() {
    println!("🔐 Quick Gmail API authentication test...");
    
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // Test Gmail fetcher with 10-second timeout
    println!("📧 Testing Gmail fetcher...");
    match timeout(Duration::from_secs(10), GmailFetcher::from_env()).await {
        Ok(Ok(_)) => println!("  ✅ Gmail fetcher: SUCCESS"),
        Ok(Err(e)) => println!("  ❌ Gmail fetcher error: {}", e),
        Err(_) => println!("  ⏰ Gmail fetcher: TIMEOUT (likely waiting for browser)")
    }
    
    // Test Gmail labeler with 10-second timeout
    println!("🏷️  Testing Gmail labeler...");
    match timeout(Duration::from_secs(10), ConcreteGmailLabeler::from_env()).await {
        Ok(Ok(_)) => println!("  ✅ Gmail labeler: SUCCESS"),
        Ok(Err(e)) => println!("  ❌ Gmail labeler error: {}", e),
        Err(_) => println!("  ⏰ Gmail labeler: TIMEOUT (likely waiting for browser)")
    }
    
    println!("\n💡 If you see timeouts, the app is likely waiting for browser authorization.");
    println!("   Check if a browser window opened asking for Gmail permissions.");
}
