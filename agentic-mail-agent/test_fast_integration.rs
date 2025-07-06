use agentic_mail_agent::{
    action::impls::labeler::{ConcreteGmailLabeler, EmailLabeler},
    classifier::{EmailCategory, MessageClassifier, StubClassifier},
    config::LabelConfig,
    fetcher::{EmailFetcher, GmailFetcher},
};
use std::time::Duration;
use tokio::time::timeout;

fn create_test_label(category: &EmailCategory) -> String {
    let label_config = LabelConfig::new();
    label_config.get_test_label(category.as_str())
}

#[tokio::main]
async fn main() {
    println!("🧪 Fast integration test (no rate limiting)...");

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

    // Fetch just 1 email for testing
    println!("\n📬 Fetching 1 email for testing...");
    let emails = fetcher
        .fetch_unread_emails()
        .await
        .expect("Failed to fetch emails");

    if emails.is_empty() {
        println!("❌ No emails found for testing");
        return;
    }

    let test_email = &emails[0];
    println!(
        "  ✅ Using email: {} ({:?})",
        test_email.id,
        test_email.subject.as_deref().unwrap_or("(no subject)")
    );

    // Classify the email
    println!("\n🎯 Classifying email...");
    let classification = classifier
        .classify(test_email)
        .await
        .expect("Failed to classify");
    let test_label = create_test_label(&classification.category);
    println!(
        "  ✅ Classification: {} -> {}",
        classification.category, test_label
    );

    // Apply label (no rate limiting)
    println!("\n🏷️  Applying label directly...");
    match timeout(
        Duration::from_secs(10),
        labeler.apply_label(&test_email.id, &test_label),
    )
    .await
    {
        Ok(Ok(result)) => {
            println!(
                "  ✅ Label applied successfully! Created new: {}",
                result.created_new_label
            );
        }
        Ok(Err(e)) => {
            println!("  ❌ Label application error: {e}");
            return;
        }
        Err(_) => {
            println!("  ⏰ Label application timeout (this might be the issue!)");
            return;
        }
    }

    // Verify label was applied
    println!("\n🔍 Verifying label was applied...");
    match timeout(
        Duration::from_secs(10),
        labeler.get_email_labels(&test_email.id),
    )
    .await
    {
        Ok(Ok(labels)) => {
            let has_test_label = labels.iter().any(|l| l.name == test_label);
            if has_test_label {
                println!("  ✅ Label verification successful!");
            } else {
                println!("  ❌ Label not found on email! Available labels:");
                for label in &labels {
                    println!("    - {}", label.name);
                }
            }
        }
        Ok(Err(e)) => {
            println!("  ❌ Label verification error: {e}");
            return;
        }
        Err(_) => {
            println!("  ⏰ Label verification timeout");
            return;
        }
    }

    // Clean up the test label
    println!("\n🧹 Cleaning up test label...");
    let all_labels = labeler
        .list_all_labels()
        .await
        .expect("Failed to list labels");
    let label_config = LabelConfig::new();
    let test_labels: Vec<_> = all_labels
        .iter()
        .filter(|l| label_config.is_test_label(&l.name))
        .collect();

    for label in test_labels {
        match timeout(Duration::from_secs(10), labeler.delete_label(&label.id)).await {
            Ok(Ok(_)) => println!("  ✅ Deleted label: {}", label.name),
            Ok(Err(e)) => println!("  ⚠️  Failed to delete {}: {}", label.name, e),
            Err(_) => println!("  ⏰ Delete timeout for: {}", label.name),
        }
    }

    println!("\n🎉 Fast integration test completed!");
    println!("   This shows the basic flow works without rate limiting delays.");
}
