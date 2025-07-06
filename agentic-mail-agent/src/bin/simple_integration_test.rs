use agentic_mail_agent::{
    fetcher::{EmailFetcher, GmailFetcher},
    action::impls::labeler::{ConcreteGmailLabeler, EmailLabeler},
    classifier::{MessageClassifier, StubClassifier},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Simple Integration Test");
    println!("==========================");
    
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // Step 1: Create components
    println!("1️⃣ Creating components...");
    let fetcher = GmailFetcher::from_env().await?;
    let labeler = ConcreteGmailLabeler::from_env().await?;
    let classifier = StubClassifier::new();
    println!("   ✅ All components created");
    
    // Step 2: Fetch one email
    println!("2️⃣ Fetching emails...");
    let emails = fetcher.fetch_unread_emails().await?;
    if emails.is_empty() {
        println!("   ℹ️  No unread emails - test complete");
        return Ok(());
    }
    
    let test_email = &emails[0];
    println!("   ✅ Got email: {:?}", test_email.subject);
    
    // Step 3: Classify the email
    println!("3️⃣ Classifying email...");
    let classification = classifier.classify(test_email).await?;
    println!("   ✅ Classification: {}", classification.category);
    
    // Step 4: Create a test label name
    let test_label = format!("TEST_SIMPLE_{}", classification.category.to_uppercase());
    println!("   📝 Test label: {test_label}");
    
    // Step 5: Try to apply the label (this is where it might hang)
    println!("4️⃣ Applying label...");
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        labeler.apply_label(&test_email.id, &test_label)
    ).await {
        Ok(Ok(result)) => {
            println!("   ✅ Label applied successfully! Created new: {}", result.created_new_label);
        },
        Ok(Err(e)) => {
            println!("   ❌ Label application failed: {e}");
            return Err(e.into());
        },
        Err(_) => {
            println!("   ⏰ Label application timed out after 30 seconds");
            return Err("Label application timed out".into());
        }
    }
    
    // Step 6: Clean up the test label
    println!("5️⃣ Cleaning up test label...");
    let all_labels = labeler.list_all_labels().await?;
    let test_label_obj = all_labels.iter().find(|l| l.name == test_label);
    
    if let Some(label) = test_label_obj {
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            labeler.delete_label(&label.id)
        ).await {
            Ok(Ok(_)) => {
                println!("   ✅ Test label deleted successfully");
            },
            Ok(Err(e)) => {
                println!("   ❌ Failed to delete test label: {e}");
            },
            Err(_) => {
                println!("   ⏰ Delete label timed out");
            }
        }
    } else {
        println!("   ⚠️  Test label not found in label list");
    }
    
    println!("🎉 Simple integration test completed!");
    
    Ok(())
}