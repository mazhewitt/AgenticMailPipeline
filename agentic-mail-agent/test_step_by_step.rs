use agentic_mail_agent::{
    action::impls::labeler::ConcreteGmailLabeler,
    classifier::StubClassifier,
    fetcher::{EmailFetcher, GmailFetcher},
};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() {
    println!("🧪 Step-by-step Gmail operations test...");

    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Step 1: Initialize components
    println!("\n📧 Step 1: Initialize Gmail fetcher...");
    let fetcher = match timeout(Duration::from_secs(5), GmailFetcher::from_env()).await {
        Ok(Ok(f)) => {
            println!("  ✅ Gmail fetcher initialized");
            f
        }
        Ok(Err(e)) => {
            println!("  ❌ Gmail fetcher error: {e}");
            return;
        }
        Err(_) => {
            println!("  ⏰ Gmail fetcher timeout");
            return;
        }
    };

    println!("🏷️  Step 2: Initialize Gmail labeler...");
    let labeler = match timeout(Duration::from_secs(5), ConcreteGmailLabeler::from_env()).await {
        Ok(Ok(l)) => {
            println!("  ✅ Gmail labeler initialized");
            l
        }
        Ok(Err(e)) => {
            println!("  ❌ Gmail labeler error: {e}");
            return;
        }
        Err(_) => {
            println!("  ⏰ Gmail labeler timeout");
            return;
        }
    };

    // Step 3: Fetch emails (this might be slow)
    println!("\n📬 Step 3: Fetch emails (30s timeout)...");
    match timeout(Duration::from_secs(30), fetcher.fetch_unread_emails()).await {
        Ok(Ok(emails)) => {
            println!("  ✅ Fetched {} emails successfully", emails.len());

            if !emails.is_empty() {
                let first_email = &emails[0];
                println!(
                    "  📧 First email: ID={}, Subject={:?}",
                    first_email.id,
                    first_email.subject.as_deref().unwrap_or("(no subject)")
                );
            }
        }
        Ok(Err(e)) => {
            println!("  ❌ Email fetch error: {e}");
            return;
        }
        Err(_) => {
            println!("  ⏰ Email fetch timeout (this is likely the bottleneck!)");
            return;
        }
    };

    // Step 4: List labels (this might also be slow)
    println!("\n🏷️  Step 4: List labels (15s timeout)...");
    match timeout(Duration::from_secs(15), labeler.list_all_labels()).await {
        Ok(Ok(labels)) => {
            println!("  ✅ Listed {} labels successfully", labels.len());

            // Show first few labels
            for (i, label) in labels.iter().take(3).enumerate() {
                println!("  📋 Label {}: {} ({})", i + 1, label.name, label.id);
            }
        }
        Ok(Err(e)) => {
            println!("  ❌ Label list error: {e}");
            return;
        }
        Err(_) => {
            println!("  ⏰ Label list timeout");
            return;
        }
    };

    // Step 5: Test classification (should be fast)
    println!("\n🎯 Step 5: Test classification...");
    let _classifier = StubClassifier::new();
    println!("  ✅ Classifier created successfully");

    println!("\n🎉 All basic operations completed successfully!");
    println!("   If any step timed out, that's where the bottleneck is.");
}
