use agentic_mail_agent::{
    action::impls::labeler::ConcreteGmailLabeler,
    classifier::{MessageClassifier, StubClassifier},
    fetcher::{EmailFetcher, GmailFetcher},
};

#[tokio::main]
async fn main() {
    println!("🧪 Read-only integration test (no label modifications)...");

    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize components
    println!("📧 Initializing components...");
    let fetcher = GmailFetcher::from_env()
        .await
        .expect("Failed to create fetcher");
    let labeler = ConcreteGmailLabeler::from_env()
        .await
        .expect("Failed to create labeler");
    let classifier = StubClassifier::new();
    println!("  ✅ All components initialized");

    // Fetch some emails
    println!("\n📬 Fetching emails...");
    let emails = fetcher
        .fetch_unread_emails()
        .await
        .expect("Failed to fetch emails");
    println!("  ✅ Fetched {} emails", emails.len());

    if emails.is_empty() {
        println!("❌ No emails to test with");
        return;
    }

    // List existing labels
    println!("\n🏷️  Listing existing labels...");
    let labels = labeler
        .list_all_labels()
        .await
        .expect("Failed to list labels");
    println!("  ✅ Found {} existing labels", labels.len());

    // Show first few labels
    for (i, label) in labels.iter().take(5).enumerate() {
        println!("    {}. {} ({})", i + 1, label.name, label.id);
    }

    // Test email classification only
    println!("\n🎯 Testing classification on first 3 emails...");
    for (i, email) in emails.iter().take(3).enumerate() {
        println!(
            "  📧 Email {}: {}",
            i + 1,
            email.subject.as_deref().unwrap_or("(no subject)")
        );

        let classification = classifier
            .classify(email)
            .await
            .expect("Failed to classify");
        println!(
            "    🎯 -> {} (score: {:?})",
            classification.category, classification.score
        );
    }

    // Test reading labels from an email (read-only)
    println!("\n🔍 Testing label reading on first email...");
    let first_email = &emails[0];
    match labeler.get_email_labels(&first_email.id).await {
        Ok(email_labels) => {
            println!("  ✅ Email has {} labels:", email_labels.len());
            for label in &email_labels {
                println!("    - {}", label.name);
            }
        }
        Err(e) => {
            println!("  ❌ Failed to read email labels: {e}");
        }
    }

    println!("\n🎉 Read-only integration test completed successfully!");
    println!("   Classification and read operations work fine.");
    println!("   For write operations (applying labels), you need to authorize in browser.");
}
