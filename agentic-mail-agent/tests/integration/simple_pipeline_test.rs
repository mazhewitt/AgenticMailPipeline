//! Simple end-to-end pipeline test
//!
//! This test demonstrates the complete mail agent workflow:
//! 1. Fetch emails from Gmail
//! 2. Classify them using the stub classifier  
//! 3. Apply labels based on classification
//! 4. Verify labels were applied
//! 5. Clean up test labels
//!
//! Run with: cargo test --test integration -- simple_pipeline --ignored

use agentic_mail_agent::{
    action::impls::labeler::{ConcreteGmailLabeler, EmailLabeler},
    classifier::{MessageClassifier, StubClassifier},
    core::email::Email,
    fetcher::{EmailFetcher, GmailFetcher},
};

const TEST_LABEL_PREFIX: &str = "TEST_SIMPLE_";

#[tokio::test]
#[ignore = "Requires Gmail API credentials and modifies real Gmail account"]
async fn test_simple_end_to_end_pipeline() {
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("🔄 Simple End-to-End Pipeline Test");
    println!("==================================");

    // Step 1: Initialize components
    println!("1️⃣ Initializing components...");
    let fetcher = GmailFetcher::from_env()
        .await
        .expect("Failed to create Gmail fetcher");

    let labeler = ConcreteGmailLabeler::from_env()
        .await
        .expect("Failed to create Gmail labeler");

    let classifier = StubClassifier::new();
    println!("   ✅ All components initialized");

    // Step 2: Fetch a small number of emails
    println!("2️⃣ Fetching emails...");
    let emails = fetcher
        .fetch_unread_emails()
        .await
        .expect("Failed to fetch emails");

    if emails.is_empty() {
        println!("   ℹ️ No unread emails found - sending yourself an email will enable this test");
        return;
    }

    // Use only the first 2 emails to keep it simple
    let test_emails: Vec<Email> = emails.into_iter().take(2).collect();
    println!("   ✅ Processing {} emails", test_emails.len());

    // Step 3: Process each email sequentially
    let mut applied_labels = Vec::new();

    for (i, email) in test_emails.iter().enumerate() {
        println!("\n📧 Processing email {} of {}", i + 1, test_emails.len());
        if let Some(subject) = &email.subject {
            println!("   Subject: {subject}");
        }

        // Classify the email
        let classification = classifier
            .classify(email)
            .await
            .expect("Failed to classify email");

        println!("   🎯 Classification: {}", classification.category);

        // Create test label name
        let test_label = format!(
            "{}{}",
            TEST_LABEL_PREFIX,
            classification.category.as_str().to_uppercase()
        );

        // Apply the label
        let result = labeler
            .apply_label(&email.id, &test_label)
            .await
            .expect("Failed to apply label");

        println!(
            "   ✅ Applied label: {} (new: {})",
            test_label, result.created_new_label
        );
        applied_labels.push(test_label);

        // Small delay between operations
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Step 4: Verify labels were applied
    println!("\n4️⃣ Verifying labels...");
    for (email, expected_label) in test_emails.iter().zip(applied_labels.iter()) {
        let email_labels = labeler
            .get_email_labels(&email.id)
            .await
            .expect("Failed to get email labels");

        let has_label = email_labels
            .iter()
            .any(|label| &label.name == expected_label);
        assert!(
            has_label,
            "Email {} should have label {}",
            email.id, expected_label
        );
        println!("   ✅ Verified label: {expected_label}");
    }

    // Step 5: Clean up test labels
    println!("\n5️⃣ Cleaning up...");
    let all_labels = labeler
        .list_all_labels()
        .await
        .expect("Failed to list labels");

    let test_labels_to_delete: Vec<_> = all_labels
        .iter()
        .filter(|label| label.name.starts_with(TEST_LABEL_PREFIX))
        .collect();

    for label in test_labels_to_delete {
        match labeler.delete_label(&label.id).await {
            Ok(_) => println!("   ✅ Deleted: {}", label.name),
            Err(e) => println!("   ⚠️ Failed to delete {}: {}", label.name, e),
        }

        // Small delay between deletions
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    println!("\n🎉 Pipeline test completed successfully!");
    println!(
        "   Processed {} emails through the complete workflow",
        test_emails.len()
    );
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_gmail_authentication() {
    // Install crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("🔐 Testing Gmail Authentication");

    // Test fetcher authentication
    let _fetcher = GmailFetcher::from_env()
        .await
        .expect("Gmail fetcher authentication failed");
    println!("   ✅ Fetcher authenticated");

    // Test labeler authentication
    let _labeler = ConcreteGmailLabeler::from_env()
        .await
        .expect("Gmail labeler authentication failed");
    println!("   ✅ Labeler authenticated");

    println!("🔐 Authentication test passed!");
}
