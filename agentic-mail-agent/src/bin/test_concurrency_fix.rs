use agentic_mail_agent::{
    fetcher::{EmailFetcher, GmailFetcher},
    action::impls::labeler::{ConcreteGmailLabeler, EmailLabeler},
    classifier::{MessageClassifier, StubClassifier},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Concurrency Fix Test");
    println!("=======================");
    
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // Test with VERY limited concurrency to avoid token conflicts
    const _MAX_CONCURRENT: usize = 1; // Force sequential execution
    
    // Step 1: Create components
    println!("1️⃣ Creating components...");
    let fetcher = GmailFetcher::from_env().await?;
    let labeler = Arc::new(ConcreteGmailLabeler::from_env().await?);
    let classifier = StubClassifier::new();
    println!("   ✅ All components created");
    
    // Step 2: Fetch emails
    println!("2️⃣ Fetching emails...");
    let emails = fetcher.fetch_unread_emails().await?;
    if emails.is_empty() {
        println!("   ℹ️  No unread emails - test complete");
        return Ok(());
    }
    
    let test_emails = emails.into_iter().take(3).collect::<Vec<_>>();
    println!("   ✅ Got {} emails", test_emails.len());
    
    // Step 3: Process emails SEQUENTIALLY to avoid token conflicts
    println!("3️⃣ Processing emails sequentially...");
    let mut test_labels = Vec::new();
    
    for (i, email) in test_emails.iter().enumerate() {
        println!("   📧 Processing email {} of {}", i + 1, test_emails.len());
        
        // Classify
        let classification = classifier.classify(email).await?;
        let test_label = format!("TEST_CONCURRENT_{}", classification.category.to_uppercase());
        
        // Apply label with timeout
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            labeler.apply_label(&email.id, &test_label)
        ).await {
            Ok(Ok(result)) => {
                println!("     ✅ Applied label: {} (new: {})", test_label, result.created_new_label);
                test_labels.push(test_label);
            },
            Ok(Err(e)) => {
                println!("     ❌ Failed to apply label: {e}");
                return Err(e.into());
            },
            Err(_) => {
                println!("     ⏰ Label application timed out");
                return Err("Label application timed out".into());
            }
        }
        
        // Add delay between operations to be extra careful
        if i < test_emails.len() - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    
    // Step 4: Clean up sequentially
    println!("4️⃣ Cleaning up test labels sequentially...");
    let all_labels = labeler.list_all_labels().await?;
    
    for test_label in &test_labels {
        if let Some(label) = all_labels.iter().find(|l| l.name == *test_label) {
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                labeler.delete_label(&label.id)
            ).await {
                Ok(Ok(_)) => {
                    println!("     ✅ Deleted label: {test_label}");
                },
                Ok(Err(e)) => {
                    println!("     ❌ Failed to delete label {test_label}: {e}");
                },
                Err(_) => {
                    println!("     ⏰ Delete timed out for: {test_label}");
                }
            }
            
            // Delay between deletions
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    
    println!("🎉 Sequential concurrency fix test completed!");
    println!("   If this works, the issue is confirmed to be OAuth token conflicts in concurrent operations");
    
    Ok(())
}