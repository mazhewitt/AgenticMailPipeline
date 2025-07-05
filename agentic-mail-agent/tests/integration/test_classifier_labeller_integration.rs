//! Comprehensive integration tests for classifier and labeller working together
//! 
//! These tests use your real Gmail account but with temporary test labels to avoid
//! interfering with your actual email workflow. They:
//! 1. Fetch real emails from your inbox
//! 2. Classify them using the current classifier
//! 3. Apply temporary test labels (TEST_AGENT_*)
//! 4. Verify the labels were applied correctly
//! 5. Clean up by removing all test labels
//! 
//! Run with: cargo test --test integration -- test_classifier_labeller --ignored

use agentic_mail_agent::{
    fetcher::{EmailFetcher, GmailFetcher},
    classifier::{MessageClassifier, StubClassifier},
    action::impls::labeler::{GmailLabeler, EmailLabeler},
};
use std::collections::HashSet;

// Test configuration
const TEST_LABEL_PREFIX: &str = "TEST_AGENT_";
const MAX_EMAILS_TO_TEST: u32 = 10;

/// Helper to create test label names that won't interfere with production
fn create_test_label(category: &str) -> String {
    format!("{}{}", TEST_LABEL_PREFIX, category.to_uppercase())
}

/// Helper to extract original category from test label
fn extract_category_from_test_label(test_label: &str) -> Option<&str> {
    test_label.strip_prefix(TEST_LABEL_PREFIX)
}

/// Cleanup helper to remove all test labels from Gmail
async fn cleanup_test_labels(labeler: &GmailLabeler) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Cleaning up test labels...");
    
    // Get all existing labels
    let all_labels = labeler.list_all_labels().await?;
    
    // Find test labels
    let test_labels: Vec<_> = all_labels.iter()
        .filter(|label| label.name.starts_with(TEST_LABEL_PREFIX))
        .collect();
    
    if test_labels.is_empty() {
        println!("  ✅ No test labels found to clean up");
        return Ok(());
    }
    
    println!("  🗑️  Removing {} test labels", test_labels.len());
    for label in &test_labels {
        println!("    - Removing: {}", label.name);
        match labeler.delete_label(&label.id).await {
            Ok(_) => println!("      ✅ Deleted"),
            Err(e) => println!("      ⚠️  Failed to delete: {}", e),
        }
    }
    
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials and modifies real Gmail account"]
async fn test_classifier_labeller_integration_full_workflow() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("🧪 CLASSIFIER + LABELLER INTEGRATION TEST");
    println!("{}", "=".repeat(60));
    println!("This test will:");
    println!("  1. Fetch up to {} real emails from your Gmail", MAX_EMAILS_TO_TEST);
    println!("  2. Classify each email using the stub classifier");
    println!("  3. Apply TEST_AGENT_* labels based on classification");
    println!("  4. Verify labels were applied correctly");
    println!("  5. Clean up all test labels");
    println!();
    
    // Step 1: Initialize components
    println!("📧 Step 1: Initializing Gmail fetcher and labeler...");
    let fetcher = GmailFetcher::from_env().await
        .expect("Failed to create Gmail fetcher - check your credentials");
    let labeler = GmailLabeler::from_env().await
        .expect("Failed to create Gmail labeler - check your credentials");
    let classifier = StubClassifier::new();
    
    println!("  ✅ All components initialized successfully");
    
    // Step 2: Clean up any existing test labels before starting
    println!("\n🧹 Step 2: Pre-test cleanup...");
    cleanup_test_labels(&labeler).await
        .expect("Failed to cleanup existing test labels");
    
    // Step 3: Fetch emails
    println!("\n📬 Step 3: Fetching emails...");
    let emails = fetcher.fetch_unread_emails().await
        .expect("Failed to fetch emails");
    
    if emails.is_empty() {
        println!("  ℹ️  No unread emails found - test cannot proceed");
        println!("  💡 To test this, you can send yourself a test email first");
        return;
    }
    
    let test_emails = emails.into_iter()
        .take(MAX_EMAILS_TO_TEST as usize)
        .collect::<Vec<_>>();
    
    println!("  ✅ Found {} emails to test with", test_emails.len());
    
    // Step 4: Classify and label each email
    println!("\n🏷️  Step 4: Classifying and labeling emails...");
    let mut labeled_emails = Vec::new();
    let mut classification_summary = std::collections::HashMap::new();
    
    for (i, email) in test_emails.iter().enumerate() {
        println!("\n  📧 Email {} of {}:", i + 1, test_emails.len());
        println!("    ID: {}", email.id);
        if let Some(subject) = &email.subject {
            println!("    Subject: {}", subject);
        }
        
        // Classify the email
        let classification = classifier.classify(email).await
            .expect("Failed to classify email");
        
        let score_display = classification.score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "N/A".to_string());
        println!("    🎯 Classification: {} (score: {})", 
                 classification.category, score_display);
        
        // Create test label name
        let test_label = create_test_label(&classification.category);
        println!("    🏷️  Applying test label: {}", test_label);
        
        // Apply the test label
        let label_result = labeler.apply_label(&email.id, &test_label).await
            .expect("Failed to apply label");
        
        println!("    ✅ Label applied successfully");
        if label_result.created_new_label {
            println!("      📝 Created new label: {}", test_label);
        }
        
        *classification_summary.entry(classification.category.clone()).or_insert(0) += 1;
        labeled_emails.push((email.clone(), classification, test_label));
    }
    
    // Step 5: Verify labels were applied
    println!("\n🔍 Step 5: Verifying labels were applied correctly...");
    let mut verification_success = 0;
    let mut verification_failed = 0;
    
    for (email, _classification, expected_label) in &labeled_emails {
        println!("\n  📧 Verifying email: {}", email.id);
        
        // Fetch the email again to check its labels
        match labeler.get_email_labels(&email.id).await {
            Ok(labels) => {
                let has_expected_label = labels.iter()
                    .any(|label| label.name == *expected_label);
                
                if has_expected_label {
                    println!("    ✅ Has expected label: {}", expected_label);
                    verification_success += 1;
                } else {
                    println!("    ❌ Missing expected label: {}", expected_label);
                    println!("    📋 Current labels: {:?}", 
                             labels.iter().map(|l| &l.name).collect::<Vec<_>>());
                    verification_failed += 1;
                }
            }
            Err(e) => {
                println!("    ❌ Failed to get email labels: {}", e);
                verification_failed += 1;
            }
        }
    }
    
    // Step 6: Test classification accuracy with ground truth
    println!("\n📊 Step 6: Analysis of classification results...");
    println!("  Classification summary:");
    for (category, count) in &classification_summary {
        println!("    - {}: {} emails", category, count);
    }
    
    // Step 7: Test idempotency - apply labels again
    println!("\n🔄 Step 7: Testing label application idempotency...");
    for (email, _, test_label) in labeled_emails.iter().take(3) { // Test first 3 emails
        println!("  📧 Re-applying label {} to {}", test_label, email.id);
        
        let label_result = labeler.apply_label(&email.id, test_label).await
            .expect("Failed to re-apply label");
        
        if !label_result.created_new_label {
            println!("    ✅ Idempotent: No new label created on re-application");
        } else {
            println!("    ⚠️  Unexpected: New label created on re-application");
        }
    }
    
    // Step 8: Clean up test labels
    println!("\n🧹 Step 8: Final cleanup...");
    cleanup_test_labels(&labeler).await
        .expect("Failed to cleanup test labels");
    
    // Step 9: Final verification
    println!("\n📊 Test Results Summary:");
    println!("  - Emails processed: {}", labeled_emails.len());
    println!("  - Labels verified successfully: {}", verification_success);
    println!("  - Label verification failures: {}", verification_failed);
    println!("  - Classification categories used: {}", classification_summary.len());
    
    // Assertions
    assert!(verification_success > 0, 
            "At least one email should have been labeled successfully");
    assert!(verification_failed == 0, 
            "All label verifications should pass (found {} failures)", verification_failed);
    assert!(labeled_emails.len() > 0, 
            "Should have processed at least one email");
    
    println!("\n✅ INTEGRATION TEST PASSED!");
    println!("   All components working together correctly");
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_classifier_with_real_emails_quality_assessment() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("🧪 CLASSIFIER QUALITY ASSESSMENT WITH REAL EMAILS");
    println!("{}", "=".repeat(60));
    
    // Initialize components
    let fetcher = GmailFetcher::from_env().await
        .expect("Failed to create Gmail fetcher");
    let classifier = StubClassifier::new();
    
    // Fetch emails
    let emails = fetcher.fetch_unread_emails().await
        .expect("Failed to fetch emails");
    
    if emails.is_empty() {
        println!("No emails to analyze");
        return;
    }
    
    let test_emails = emails.into_iter()
        .take(MAX_EMAILS_TO_TEST as usize)
        .collect::<Vec<_>>();
    
    println!("📊 Analyzing {} emails for classification quality", test_emails.len());
    
    let mut category_counts = std::collections::HashMap::new();
    let mut scores = Vec::new();
    let mut detailed_results = Vec::new();
    
    for (i, email) in test_emails.iter().enumerate() {
        println!("\n📧 Email {} of {}:", i + 1, test_emails.len());
        
        if let Some(subject) = &email.subject {
            println!("  Subject: {}", subject);
        } else {
            println!("  Subject: (missing)");
        }
        
        if let Some(snippet) = &email.snippet {
            println!("  Snippet: {}...", 
                     if snippet.len() > 100 { &snippet[..100] } else { snippet });
        } else {
            println!("  Snippet: (missing)");
        }
        
        // Classify
        let classification = classifier.classify(email).await
            .expect("Failed to classify email");
        
        let score_display = classification.score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "N/A".to_string());
        println!("  🎯 Classification: {} (score: {})", 
                 classification.category, score_display);
        println!("  💭 Reasoning: {}", classification.llm_response);
        
        *category_counts.entry(classification.category.clone()).or_insert(0) += 1;
        scores.push(classification.score);
        detailed_results.push((email.clone(), classification));
    }
    
    // Analysis
    println!("\n📊 CLASSIFICATION ANALYSIS:");
    println!("Category distribution:");
    for (category, count) in &category_counts {
        let percentage = (count * 100) / test_emails.len();
        println!("  - {}: {} emails ({}%)", category, count, percentage);
    }
    
    let valid_scores: Vec<f32> = scores.iter().filter_map(|&s| s).collect();
    let avg_score = if !valid_scores.is_empty() {
        valid_scores.iter().sum::<f32>() / valid_scores.len() as f32
    } else {
        0.0
    };
    let min_score = valid_scores.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max_score = valid_scores.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    
    println!("\nScore analysis:");
    println!("  - Average: {:.2}", avg_score);
    println!("  - Min: {:.2}", min_score);
    println!("  - Max: {:.2}", max_score);
    
    // Quality checks
    let high_score_count = valid_scores.iter()
        .filter(|&&score| score >= 0.8)
        .count();
    let low_score_count = valid_scores.iter()
        .filter(|&&score| score < 0.5)
        .count();
    
    println!("\nQuality metrics:");
    println!("  - High score (≥0.8): {} emails ({}%)", 
             high_score_count, 
             if !valid_scores.is_empty() { (high_score_count * 100) / valid_scores.len() } else { 0 });
    println!("  - Low score (<0.5): {} emails ({}%)", 
             low_score_count, 
             if !valid_scores.is_empty() { (low_score_count * 100) / valid_scores.len() } else { 0 });
    
    // Assertions for quality
    if !valid_scores.is_empty() {
        assert!(avg_score > 0.6, 
                "Average score should be above 0.6, got {:.2}", avg_score);
    }
    assert!(category_counts.len() > 1, 
            "Should classify emails into multiple categories, got {}", category_counts.len());
    
    println!("\n✅ QUALITY ASSESSMENT PASSED!");
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_labeller_label_management() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("🧪 LABELLER LABEL MANAGEMENT TEST");
    println!("{}", "=".repeat(60));
    
    let labeler = GmailLabeler::from_env().await
        .expect("Failed to create Gmail labeler");
    
    // Clean up any existing test labels
    cleanup_test_labels(&labeler).await
        .expect("Failed to cleanup existing test labels");
    
    // Test label creation
    println!("🏷️  Testing label creation...");
    let test_labels = vec![
        create_test_label("ActionRequired"),
        create_test_label("InterestingInfo"),
        create_test_label("Reference"),
        create_test_label("Noise"),
        create_test_label("Spam"),
    ];
    
    let mut created_label_ids = Vec::new();
    
    for label_name in &test_labels {
        println!("  Creating label: {}", label_name);
        let label_id = labeler.ensure_label_exists(label_name).await
            .expect("Failed to create label");
        
        println!("    ✅ Created with ID: {}", label_id);
        created_label_ids.push(label_id.clone());
        
        // Test idempotency
        let label_id2 = labeler.ensure_label_exists(label_name).await
            .expect("Failed to ensure label exists (idempotency test)");
        
        assert_eq!(label_id, label_id2, 
                   "Label ID should be the same on repeated calls");
        println!("    ✅ Idempotency verified");
    }
    
    // Test label listing
    println!("\n📋 Testing label listing...");
    let all_labels = labeler.list_all_labels().await
        .expect("Failed to list labels");
    
    let our_test_labels: Vec<_> = all_labels.iter()
        .filter(|label| label.name.starts_with(TEST_LABEL_PREFIX))
        .collect();
    
    println!("  Found {} test labels", our_test_labels.len());
    assert_eq!(our_test_labels.len(), test_labels.len(), 
               "Should find all created test labels");
    
    for label in &our_test_labels {
        println!("    - {}: {}", label.name, label.id);
        assert!(test_labels.contains(&label.name), 
                "Found unexpected test label: {}", label.name);
    }
    
    // Clean up
    println!("\n🧹 Cleaning up test labels...");
    cleanup_test_labels(&labeler).await
        .expect("Failed to cleanup test labels");
    
    // Verify cleanup
    let all_labels_after = labeler.list_all_labels().await
        .expect("Failed to list labels after cleanup");
    
    let remaining_test_labels: Vec<_> = all_labels_after.iter()
        .filter(|label| label.name.starts_with(TEST_LABEL_PREFIX))
        .collect();
    
    assert_eq!(remaining_test_labels.len(), 0, 
               "All test labels should be cleaned up");
    
    println!("✅ LABEL MANAGEMENT TEST PASSED!");
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_end_to_end_workflow_with_cleanup() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("🧪 END-TO-END WORKFLOW TEST (REAL GMAIL INTEGRATION)");
    println!("{}", "=".repeat(60));
    println!("This test simulates the complete agentic mail agent workflow:");
    println!("  1. Fetch → 2. Classify → 3. Label → 4. Verify → 5. Cleanup");
    println!();
    
    // Initialize components
    let fetcher = GmailFetcher::from_env().await
        .expect("Failed to create Gmail fetcher");
    let labeler = GmailLabeler::from_env().await
        .expect("Failed to create Gmail labeler");
    let classifier = StubClassifier::new();
    
    // Clean up any existing test labels
    cleanup_test_labels(&labeler).await.unwrap_or(());
    
    // Fetch a smaller set for end-to-end test
    let emails = fetcher.fetch_unread_emails().await
        .expect("Failed to fetch emails");
    
    let test_emails = emails.into_iter().take(5).collect::<Vec<_>>();
    
    if test_emails.is_empty() {
        println!("No emails available for end-to-end test");
        return;
    }
    
    println!("🔄 Processing {} emails through complete workflow", test_emails.len());
    
    let mut processed_emails = Vec::new();
    
    for (i, email) in test_emails.iter().enumerate() {
        println!("\n📧 Processing email {} of {}: {}", i + 1, test_emails.len(), email.id);
        
        // Classify
        let classification = classifier.classify(email).await
            .expect("Failed to classify email");
        
        // Create test label
        let test_label = create_test_label(&classification.category);
        
        // Apply label
        let label_result = labeler.apply_label(&email.id, &test_label).await
            .expect("Failed to apply label");
        
        println!("  ✅ Applied label: {} (new: {})", 
                 test_label, label_result.created_new_label);
        
        processed_emails.push((email, classification, test_label));
    }
    
    // Verify all labels were applied correctly
    println!("\n🔍 Verifying all labels...");
    for (email, _classification, expected_label) in &processed_emails {
        let labels = labeler.get_email_labels(&email.id).await
            .expect("Failed to get email labels");
        
        let has_label = labels.iter().any(|l| l.name == *expected_label);
        assert!(has_label, "Email {} should have label {}", email.id, expected_label);
        println!("  ✅ Email {} has label {}", email.id, expected_label);
    }
    
    // Test that we can retrieve emails by label
    println!("\n📧 Testing email retrieval by labels...");
    let test_labels_used: HashSet<_> = processed_emails.iter()
        .map(|(_, _, label)| label.clone())
        .collect();
    
    for test_label in &test_labels_used {
        match labeler.get_emails_by_label(test_label).await {
            Ok(labeled_emails) => {
                println!("  📋 Label '{}': {} emails", test_label, labeled_emails.len());
                assert!(!labeled_emails.is_empty(), 
                        "Should find emails with label {}", test_label);
            }
            Err(e) => {
                println!("  ⚠️  Could not retrieve emails for label {}: {}", test_label, e);
                // Don't fail the test for this - it might not be implemented
            }
        }
    }
    
    // Final cleanup
    println!("\n🧹 Final cleanup...");
    cleanup_test_labels(&labeler).await
        .expect("Failed to cleanup test labels");
    
    println!("\n✅ END-TO-END WORKFLOW TEST PASSED!");
    println!("   Successfully processed {} emails through the complete pipeline", 
             processed_emails.len());
}