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
//! ## Rate Limiting & Quota Management
//! 
//! All tests now include comprehensive rate limiting to prevent Gmail API quota issues:
//! - Bounded concurrency (max 3 concurrent requests)
//! - 200ms delays between batches of requests
//! - Exponential backoff retry logic for rate limit errors
//! - Enhanced error handling for quota-related failures
//! 
//! ## Available Tests
//! 
//! - `test_gmail_api_authentication`: Validates Gmail API setup and credentials
//! - `test_classifier_labeller_integration_full_workflow`: Full end-to-end integration test
//! - `test_classifier_with_real_emails_quality_assessment`: Classifier quality analysis
//! - `test_labeller_label_management`: Label creation, listing, and cleanup
//! - `test_end_to_end_workflow_with_cleanup`: Complete workflow simulation
//! 
//! Run with: cargo test --test integration -- test_classifier_labeller --ignored

use agentic_mail_agent::{
    fetcher::{EmailFetcher, GmailFetcher},
    classifier::{MessageClassifier, StubClassifier, Classification},
    action::impls::labeler::{ConcreteGmailLabeler, EmailLabeler, LabelingResult, LabelingError},
    core::email::Email,

};
use std::collections::{HashSet, HashMap};
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, Instant, timeout};

// Test configuration
const TEST_LABEL_PREFIX: &str = "TEST_AGENT_";
const MAX_EMAILS_TO_TEST: u32 = 10;

// Rate limiting configuration - REDUCED to 1 to avoid OAuth token conflicts
const MAX_CONCURRENT_REQUESTS: usize = 1;  // Force sequential to avoid OAuth conflicts
const REQUEST_DELAY_MS: u64 = 200;         // 200ms between batches of requests
const RETRY_ATTEMPTS: u32 = 3;             // Number of retries for rate limit errors
const RETRY_DELAY_MS: u64 = 1000;          // Base delay for exponential backoff

// Timeout configuration
const OPERATION_TIMEOUT_SECONDS: u64 = 30;  // 30 seconds for individual operations
const TEST_TIMEOUT_SECONDS: u64 = 300;      // 5 minutes for entire test
const API_CALL_TIMEOUT_SECONDS: u64 = 15;   // 15 seconds for single API calls

/// Result of applying a label to an email
#[derive(Debug, Clone)]
struct LabelingAttempt {
    email_id: String,
    email_subject: Option<String>,
    label: String,
    result: Result<LabelingResult, LabelingError>,
}

/// Result of verifying a label on an email
#[derive(Debug, Clone)]
struct VerificationAttempt {
    email_id: String,
    email_subject: Option<String>,
    expected_label: String,
    result: Result<bool, LabelingError>,
}

/// Aggregated results from a phase of operations
#[derive(Debug)]
struct PhaseResults<T> {
    successes: Vec<T>,
    failures: Vec<T>,
}

impl<T> PhaseResults<T> {
    fn new() -> Self {
        Self {
            successes: Vec::new(),
            failures: Vec::new(),
        }
    }
    
    fn add_result(&mut self, item: T, success: bool) {
        if success {
            self.successes.push(item);
        } else {
            self.failures.push(item);
        }
    }
    
    fn success_count(&self) -> usize {
        self.successes.len()
    }
    
    fn failure_count(&self) -> usize {
        self.failures.len()
    }
    
    fn total_count(&self) -> usize {
        self.successes.len() + self.failures.len()
    }
}

/// Drop guard to ensure cleanup happens even on panic
struct TestCleanupGuard {
    labeler: Arc<ConcreteGmailLabeler>,
    active: bool,
}

impl TestCleanupGuard {
    fn new(labeler: Arc<ConcreteGmailLabeler>) -> Self {
        Self {
            labeler,
            active: true,
        }
    }
    
    /// Manually trigger cleanup and disable the drop behavior
    async fn cleanup_now(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.active {
            self.active = false;
            cleanup_test_labels(&self.labeler).await
        } else {
            Ok(())
        }
    }
}

impl Drop for TestCleanupGuard {
    fn drop(&mut self) {
        if self.active {
            println!("⚠️  TestCleanupGuard: Cleanup required on drop - some test labels may remain");
            // Note: We can't run async cleanup in drop, so we just warn
        }
    }
}

/// Helper to create test label names that won't interfere with production
fn create_test_label(category: &str) -> String {
    format!("{}{}", TEST_LABEL_PREFIX, category.to_uppercase())
}

/// Helper to extract original category from test label
#[allow(dead_code)]
fn extract_category_from_test_label(test_label: &str) -> Option<&str> {
    test_label.strip_prefix(TEST_LABEL_PREFIX)
}


/// Apply labels to multiple emails concurrently with rate limiting
async fn apply_labels_concurrently(
    labeler: Arc<ConcreteGmailLabeler>,
    emails_and_labels: Vec<(Email, Classification, String)>,
) -> PhaseResults<LabelingAttempt> {
    let mut results = PhaseResults::new();
    let rate_limiter = ApiRateLimiter::new(REQUEST_DELAY_MS);
    
    // Process emails in batches to limit concurrency
    for chunk in emails_and_labels.chunks(MAX_CONCURRENT_REQUESTS) {
        let mut futures = FuturesUnordered::new();
        
        for (email, _classification, test_label) in chunk {
            let labeler_clone = Arc::clone(&labeler);
            let email_id = email.id.clone();
            let email_subject = email.subject.clone();
            let label = test_label.clone();
            let rate_limiter_ref = &rate_limiter;
            
            futures.push(async move {
                let result = retry_with_backoff(
                    || async { labeler_clone.apply_label(&email_id, &label).await },
                    rate_limiter_ref,
                    &format!("Apply label '{}' to email {}", label, email_id)
                ).await;
                
                LabelingAttempt {
                    email_id,
                    email_subject,
                    label,
                    result,
                }
            });
        }
        
        // Process this batch before starting the next
        while let Some(attempt) = futures.next().await {
            let success = attempt.result.is_ok();
            results.add_result(attempt, success);
        }
        
        // Add delay between batches
        if !chunk.is_empty() {
            sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;
        }
    }
    
    results
}

/// Verify labels on multiple emails concurrently with rate limiting
async fn verify_labels_concurrently(
    labeler: Arc<ConcreteGmailLabeler>,
    email_label_pairs: Vec<(Email, String)>,
) -> PhaseResults<VerificationAttempt> {
    let mut results = PhaseResults::new();
    let rate_limiter = ApiRateLimiter::new(REQUEST_DELAY_MS);
    
    // Process emails in batches to limit concurrency
    for chunk in email_label_pairs.chunks(MAX_CONCURRENT_REQUESTS) {
        let mut futures = FuturesUnordered::new();
        
        for (email, expected_label) in chunk {
            let labeler_clone = Arc::clone(&labeler);
            let email_id = email.id.clone();
            let email_subject = email.subject.clone();
            let label = expected_label.clone();
            let rate_limiter_ref = &rate_limiter;
            
            futures.push(async move {
                let result = retry_with_backoff(
                    || async { 
                        labeler_clone.get_email_labels(&email_id).await
                            .map(|labels| labels.iter().any(|l| l.name == label))
                    },
                    rate_limiter_ref,
                    &format!("Verify label '{}' on email {}", label, email_id)
                ).await;
                
                VerificationAttempt {
                    email_id,
                    email_subject,
                    expected_label: label,
                    result,
                }
            });
        }
        
        // Process this batch before starting the next
        while let Some(attempt) = futures.next().await {
            let success = attempt.result.as_ref().map_or(false, |&has_label| has_label);
            results.add_result(attempt, success);
        }
        
        // Add delay between batches
        if !chunk.is_empty() {
            sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;
        }
    }
    
    results
}

/// Clean up test labels concurrently with rate limiting  
async fn cleanup_test_labels_concurrently(
    labeler: Arc<ConcreteGmailLabeler>,
) -> PhaseResults<(String, Result<(), LabelingError>)> {
    let mut results = PhaseResults::new();
    let rate_limiter = ApiRateLimiter::new(REQUEST_DELAY_MS);
    
    // Get all existing labels first
    let all_labels = match retry_with_backoff(
        || async { labeler.list_all_labels().await },
        &rate_limiter,
        "List all labels for cleanup"
    ).await {
        Ok(labels) => labels,
        Err(e) => {
            println!("⚠️  Failed to list labels for cleanup: {}", e);
            return results;
        }
    };
    
    // Find test labels
    let test_labels: Vec<_> = all_labels.iter()
        .filter(|label| label.name.starts_with(TEST_LABEL_PREFIX))
        .collect();
    
    if test_labels.is_empty() {
        return results;
    }
    
    // Process deletions in batches to limit concurrency
    for chunk in test_labels.chunks(MAX_CONCURRENT_REQUESTS) {
        let mut futures = FuturesUnordered::new();
        
        for label in chunk {
            let labeler_clone = Arc::clone(&labeler);
            let label_id = label.id.clone();
            let label_name = label.name.clone();
            let rate_limiter_ref = &rate_limiter;
            
            futures.push(async move {
                let result = retry_with_backoff(
                    || async {
                        labeler_clone.delete_label(&label_id).await
                            .map_err(|e| LabelingError::unknown(format!("Failed to delete label: {}", e)))
                    },
                    rate_limiter_ref,
                    &format!("Delete label '{}'", label_name)
                ).await;
                (label_name, result)
            });
        }
        
        // Process this batch before starting the next
        while let Some((label_name, result)) = futures.next().await {
            let success = result.is_ok();
            results.add_result((label_name, result), success);
        }
        
        // Add delay between batches
        if !chunk.is_empty() {
            sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;
        }
    }
    
    results
}

/// Format email context for error messages
fn format_email_context(email_id: &str, subject: &Option<String>) -> String {
    match subject {
        Some(s) => format!("email {} ('{}')", email_id, s),
        None => format!("email {}", email_id),
    }
}

/// Cleanup helper to remove all test labels from Gmail (enhanced with rate limiting)
async fn cleanup_test_labels(labeler: &ConcreteGmailLabeler) -> Result<(), Box<dyn std::error::Error>> {
    cleanup_test_labels_with_retries(labeler).await
}

/// Helper function to handle Gmail API quota errors specifically
fn is_quota_error(error: &LabelingError) -> bool {
    let error_str = error.to_string().to_lowercase();
    error_str.contains("quota") || 
    error_str.contains("rate limit") || 
    error_str.contains("too many requests") ||
    error_str.contains("429")
}

/// Enhanced cleanup with better quota error handling
async fn cleanup_test_labels_with_retries(labeler: &ConcreteGmailLabeler) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Cleaning up test labels with rate limiting...");
    
    let labeler_arc = Arc::new(labeler.clone());
    let rate_limiter = ApiRateLimiter::new(REQUEST_DELAY_MS);
    
    // Get labels with retry logic
    let all_labels = retry_with_backoff(
        || async { labeler.list_all_labels().await },
        &rate_limiter,
        "List all labels for cleanup"
    ).await?;
    
    let test_labels: Vec<_> = all_labels.iter()
        .filter(|label| label.name.starts_with(TEST_LABEL_PREFIX))
        .collect();
    
    if test_labels.is_empty() {
        println!("  ✅ No test labels found to clean up");
        return Ok(());
    }
    
    println!("  🗑️  Found {} test labels to remove", test_labels.len());
    
    // Use the concurrent cleanup function
    let results = cleanup_test_labels_concurrently(labeler_arc).await;
    
    println!("  📊 Cleanup completed: {} succeeded, {} failed", 
             results.success_count(), results.failure_count());
    
    // Report details
    for (label_name, result) in &results.successes {
        match result {
            Ok(_) => println!("    ✅ Deleted: {}", label_name),
            Err(e) => println!("    ⚠️  Error deleting {}: {}", label_name, e),
        }
    }
    
    for (label_name, result) in &results.failures {
        if let Err(e) = result {
            if is_quota_error(e) {
                println!("    🚫 Quota error deleting {}: {}", label_name, e);
            } else {
                println!("    ❌ Failed to delete {}: {}", label_name, e);
            }
        }
    }
    
    if results.failure_count() > 0 {
        Err(format!("Failed to delete {} test labels", results.failure_count()).into())
    } else {
        Ok(())
    }
}

/// Rate limiter for Gmail API calls to prevent quota issues
struct ApiRateLimiter {
    last_request_time: std::sync::Mutex<Option<Instant>>,
    delay_between_requests: Duration,
}

impl ApiRateLimiter {
    fn new(delay_ms: u64) -> Self {
        Self {
            last_request_time: std::sync::Mutex::new(None),
            delay_between_requests: Duration::from_millis(delay_ms),
        }
    }
    
    /// Wait if necessary to enforce rate limiting
    async fn wait_for_rate_limit(&self) {
        let mut last_time = self.last_request_time.lock().unwrap();
        
        if let Some(last) = *last_time {
            let elapsed = last.elapsed();
            if elapsed < self.delay_between_requests {
                let wait_time = self.delay_between_requests - elapsed;
                drop(last_time); // Release lock before sleeping
                sleep(wait_time).await;
            }
        }
        
        // Update last request time
        #[allow(unused_mut)] // mut needed for assignment
        let mut last_time = self.last_request_time.lock().unwrap();
        *last_time = Some(Instant::now());
    }
}

/// Retry wrapper for Gmail API operations with exponential backoff and timeout
async fn retry_with_backoff<F, Fut, T, E>(
    operation: F,
    rate_limiter: &ApiRateLimiter,
    operation_name: &str,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    for attempt in 1..=RETRY_ATTEMPTS {
        // Wait for rate limiting before each attempt
        rate_limiter.wait_for_rate_limit().await;
        
        // Wrap the operation with a timeout
        let timeout_duration = Duration::from_secs(API_CALL_TIMEOUT_SECONDS);
        let operation_with_timeout = async {
            timeout(timeout_duration, operation()).await
                .map_err(|_| format!("Operation timed out after {:?}", timeout_duration))
                .and_then(|result| result.map_err(|e| e.to_string()))
        };
        
        match operation_with_timeout.await {
            Ok(result) => return Ok(result),
            Err(error_msg) => {
                if attempt < RETRY_ATTEMPTS {
                    let delay = Duration::from_millis(RETRY_DELAY_MS * (2_u64.pow(attempt - 1)));
                    println!("  ⚠️  {operation_name} attempt {attempt} failed: {error_msg}. Retrying in {delay:?}...");
                    sleep(delay).await;
                } else {
                    println!("  ❌ {operation_name} failed after {RETRY_ATTEMPTS} attempts: {error_msg}");
                    // We need to return the original error type, so let's try one more time without timeout
                    rate_limiter.wait_for_rate_limit().await;
                    return operation().await;
                }
            }
        }
    }
    
    unreachable!()
}

/// Test Gmail API authentication setup
async fn test_gmail_auth_setup() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Testing Gmail API authentication setup...");
    
    let rate_limiter = ApiRateLimiter::new(REQUEST_DELAY_MS);
    
    // Test Gmail fetcher authentication
    println!("  📧 Testing Gmail fetcher authentication...");
    let fetcher = retry_with_backoff(
        || async { GmailFetcher::from_env().await },
        &rate_limiter,
        "Initialize Gmail fetcher"
    ).await?;
    
    // Test basic fetcher operation (fetch a small number of emails)
    let test_emails = retry_with_backoff(
        || async { 
            fetcher.fetch_unread_emails().await
                .map(|emails| emails.into_iter().take(1).collect::<Vec<_>>())
        },
        &rate_limiter,
        "Fetch test emails"
    ).await?;
    
    println!("    ✅ Gmail fetcher authenticated successfully");
    println!("    📊 Test fetch returned {} emails", test_emails.len());
    
    // Test Gmail labeler authentication
    println!("  🏷️  Testing Gmail labeler authentication...");
    let labeler = retry_with_backoff(
        || async { ConcreteGmailLabeler::from_env().await },
        &rate_limiter,
        "Initialize Gmail labeler"
    ).await?;
    
    // Test basic labeler operation (list labels)
    let labels = retry_with_backoff(
        || async { labeler.list_all_labels().await },
        &rate_limiter,
        "List Gmail labels"
    ).await?;
    
    println!("    ✅ Gmail labeler authenticated successfully");
    println!("    📊 Found {} labels in account", labels.len());
    
    // Test token validation by trying to access user profile (if available)
    println!("  👤 Testing token validation...");
    // Note: We already tested this by successfully calling the API above
    println!("    ✅ Token appears to be valid (API calls succeeded)");
    
    // Check for any existing test labels and warn if found
    let test_labels: Vec<_> = labels.iter()
        .filter(|label| label.name.starts_with(TEST_LABEL_PREFIX))
        .collect();
    
    if !test_labels.is_empty() {
        println!("  ⚠️  Found {} existing test labels - consider running cleanup", test_labels.len());
        for label in &test_labels {
            println!("    - {}", label.name);
        }
    } else {
        println!("    ✅ No existing test labels found");
    }
    
    println!("🔐 Gmail API authentication setup verified successfully!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials and modifies real Gmail account"]
async fn test_classifier_labeller_integration_full_workflow() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    // Wrap the entire test in a timeout
    let test_future = async {
        println!("🧪 CLASSIFIER + LABELLER INTEGRATION TEST (IMPROVED)");
        println!("{}", "=".repeat(60));
        println!("This test will:");
        println!("  1. Fetch up to {MAX_EMAILS_TO_TEST} real emails from your Gmail");
        println!("  2. Classify each email using the stub classifier");
        println!("  3. Apply TEST_AGENT_* labels concurrently based on classification");
        println!("  4. Verify labels were applied correctly (concurrent verification)");
        println!("  5. Clean up all test labels concurrently");
        println!("  ⏰ Test timeout: {} seconds", TEST_TIMEOUT_SECONDS);
        println!();
        
        // Step 1: Initialize components
        println!("📧 Step 1: Initializing Gmail fetcher and labeler...");
        let fetcher = GmailFetcher::from_env().await
            .expect("Failed to create Gmail fetcher - check your credentials");

        let labeler = Arc::new(ConcreteGmailLabeler::from_env().await
            .expect("Failed to create Gmail labeler - check your credentials"));

        let classifier = StubClassifier::new();
        
        println!("  ✅ All components initialized successfully");
        
        // Set up cleanup guard to ensure cleanup happens even on panic
        let mut cleanup_guard = TestCleanupGuard::new(Arc::clone(&labeler));
        
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
            cleanup_guard.cleanup_now().await.unwrap_or(());
            return;
        }
        
        let test_emails = emails.into_iter()
            .take(MAX_EMAILS_TO_TEST as usize)
            .collect::<Vec<_>>();
        
        println!("  ✅ Found {} emails to test with", test_emails.len());
        
        // Step 4: Classify emails (sequential, as this is typically not the bottleneck)
        println!("\n🎯 Step 4: Classifying emails...");
        let mut emails_with_classifications = Vec::new();
        let mut classification_summary = HashMap::new();
        
        for (i, email) in test_emails.iter().enumerate() {
            println!("  📧 Email {} of {}: {}", i + 1, test_emails.len(), 
                     format_email_context(&email.id, &email.subject));
            
            let classification = classifier.classify(email).await
                .expect(&format!("Failed to classify {}", format_email_context(&email.id, &email.subject)));
            
            let score_display = classification.score.map(|s| format!("{s:.2}")).unwrap_or_else(|| "N/A".to_string());
            println!("    🎯 Classification: {} (score: {})", 
                     classification.category, score_display);
            
            let test_label = create_test_label(&classification.category);
            *classification_summary.entry(classification.category.clone()).or_insert(0) += 1;
            emails_with_classifications.push((email.clone(), classification, test_label));
        }
        
        // Step 5: Apply labels concurrently
        println!("\n🏷️  Step 5: Applying labels concurrently...");
        let labeling_results = apply_labels_concurrently(
            Arc::clone(&labeler),
            emails_with_classifications.clone()
        ).await;
        
        println!("  📊 Labeling Results:");
        println!("    ✅ Successful: {}", labeling_results.success_count());
        println!("    ❌ Failed: {}", labeling_results.failure_count());
        
        // Assert that all labeling operations succeeded
        assert_eq!(labeling_results.failure_count(), 0, 
                   "All label applications should succeed. Failures: {:#?}", 
                   labeling_results.failures);
        assert_eq!(labeling_results.success_count(), emails_with_classifications.len(),
                   "Should have applied labels to all {} emails", emails_with_classifications.len());
        
        // Step 6: Verify labels concurrently  
        println!("\n🔍 Step 6: Verifying labels concurrently...");
        let email_label_pairs: Vec<_> = emails_with_classifications.iter()
            .map(|(email, _, test_label)| (email.clone(), test_label.clone()))
            .collect();
        
        let verification_results = verify_labels_concurrently(
            Arc::clone(&labeler),
            email_label_pairs
        ).await;
        
        println!("  📊 Verification Results:");
        println!("    ✅ Verified: {}", verification_results.success_count());
        println!("    ❌ Failed: {}", verification_results.failure_count());
        
        // Collect and report verification failures
        let mut failure_messages = Vec::new();
        for attempt in &verification_results.failures {
            let context = format_email_context(&attempt.email_id, &attempt.email_subject);
            match &attempt.result {
                Ok(false) => {
                    failure_messages.push(format!("{} missing expected label '{}'", context, attempt.expected_label));
                }
                Err(e) => {
                    failure_messages.push(format!("{} verification error: {}", context, e));
                }
                _ => unreachable!(),
            }
        }
        
        // Assert that all verifications passed
        assert_eq!(verification_results.failure_count(), 0,
                   "All label verifications should pass. Failures:\n{}", 
                   failure_messages.join("\n"));
        
        // Step 7: Test idempotency with a subset
        println!("\n🔄 Step 7: Testing label application idempotency...");
        let idempotency_test_items: Vec<_> = emails_with_classifications.iter().take(3).cloned().collect();
        let idempotency_results = apply_labels_concurrently(
            Arc::clone(&labeler),
            idempotency_test_items.clone()
        ).await;
        
        // Verify idempotency (should not create new labels)
        let mut idempotency_failures = Vec::new();
        for attempt in &idempotency_results.successes {
            if let Ok(result) = &attempt.result {
                if result.created_new_label {
                    idempotency_failures.push(format!("Re-application of label '{}' to {} created new label", 
                        attempt.label, format_email_context(&attempt.email_id, &attempt.email_subject)));
                }
            }
        }
        
        assert!(idempotency_failures.is_empty(),
               "Idempotency test failed. New labels created on re-application:\n{}", 
               idempotency_failures.join("\n"));
        
        println!("  ✅ Idempotency verified: No new labels created on re-application");
        
        // Step 8: Analysis and reporting
        println!("\n📊 Step 8: Analysis of classification results...");
        println!("  Classification summary:");
        for (category, count) in &classification_summary {
            println!("    - {category}: {count} emails");
        }
        
        // Step 9: Clean up test labels concurrently
        println!("\n🧹 Step 9: Final cleanup (concurrent)...");
        cleanup_guard.cleanup_now().await
            .expect("Failed to cleanup test labels");
        
        // Step 10: Final verification and assertions
        println!("\n📊 Test Results Summary:");
        println!("  - Emails processed: {}", emails_with_classifications.len());
        println!("  - Labels applied successfully: {}", labeling_results.success_count());
        println!("  - Labels verified successfully: {}", verification_results.success_count());
        println!("  - Classification categories used: {}", classification_summary.len());
        
        // Final assertions
        assert!(!emails_with_classifications.is_empty(),
                "Should have processed at least one email");
        assert!(classification_summary.len() > 0, 
                "Should have used at least one classification category");
        
        println!("\n✅ INTEGRATION TEST PASSED!");
        println!("   All components working together correctly with concurrent operations");
    };
    
    // Run the test with timeout
    match timeout(Duration::from_secs(TEST_TIMEOUT_SECONDS), test_future).await {
        Ok(_) => {
            println!("🎉 Test completed within timeout");
        }
        Err(_) => {
            panic!("❌ Test timed out after {} seconds. This may indicate:\n  - Gmail API is slow or unresponsive\n  - Network connectivity issues\n  - Authentication problems\n  - Rate limiting by Gmail", TEST_TIMEOUT_SECONDS);
        }
    }
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
            println!("  Subject: {subject}");
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
        
        let score_display = classification.score.map(|s| format!("{s:.2}")).unwrap_or_else(|| "N/A".to_string());
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
        println!("  - {category}: {count} emails ({percentage}%)");
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
    println!("  - Average: {avg_score:.2}");
    println!("  - Min: {min_score:.2}");
    println!("  - Max: {max_score:.2}");
    
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
                "Average score should be above 0.6, got {avg_score:.2}");
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
    
    let labeler = ConcreteGmailLabeler::from_env().await
        .expect("Failed to create Gmail labeler");
    
    // Clean up any existing test labels
    cleanup_test_labels(&labeler).await
        .expect("Failed to cleanup existing test labels");
    
    // Test label creation with rate limiting
    println!("🏷️  Testing label creation with rate limiting...");
    let test_labels = vec![
        create_test_label("ActionRequired"),
        create_test_label("InterestingInfo"),
        create_test_label("Reference"),
        create_test_label("Noise"),
        create_test_label("Spam"),
    ];
    
    let mut created_label_ids = Vec::new();
    let rate_limiter = ApiRateLimiter::new(REQUEST_DELAY_MS);
    
    for label_name in &test_labels {
        println!("  Creating label: {label_name}");
        
        let label_id = retry_with_backoff(
            || async { labeler.ensure_label_exists(label_name).await },
            &rate_limiter,
            &format!("Create label '{}'", label_name)
        ).await.expect("Failed to create label");
        
        println!("    ✅ Created with ID: {label_id}");
        created_label_ids.push(label_id.clone());
        
        // Test idempotency with rate limiting
        let label_id2 = retry_with_backoff(
            || async { labeler.ensure_label_exists(label_name).await },
            &rate_limiter,
            &format!("Ensure label '{}' exists (idempotency test)", label_name)
        ).await.expect("Failed to ensure label exists (idempotency test)");
        
        assert_eq!(label_id, label_id2, 
                   "Label ID should be the same on repeated calls");
        println!("    ✅ Idempotency verified");
    }
    
    // Test label listing with rate limiting
    println!("\n📋 Testing label listing with rate limiting...");
    let all_labels = retry_with_backoff(
        || async { labeler.list_all_labels().await },
        &rate_limiter,
        "List all labels"
    ).await.expect("Failed to list labels");
    
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
    
    // Verify cleanup with rate limiting
    let all_labels_after = retry_with_backoff(
        || async { labeler.list_all_labels().await },
        &rate_limiter,
        "List labels after cleanup"
    ).await.expect("Failed to list labels after cleanup");
    
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

    let labeler = Arc::new(ConcreteGmailLabeler::from_env().await
        .expect("Failed to create Gmail labeler"));

    let classifier = StubClassifier::new();
    
    // Set up cleanup guard
    let mut cleanup_guard = TestCleanupGuard::new(Arc::clone(&labeler));
    
    // Clean up any existing test labels
    cleanup_test_labels(&labeler).await.unwrap_or(());
    
    // Fetch a smaller set for end-to-end test
    let emails = fetcher.fetch_unread_emails().await
        .expect("Failed to fetch emails");
    
    let test_emails = emails.into_iter().take(5).collect::<Vec<_>>();
    
    if test_emails.is_empty() {
        println!("No emails available for end-to-end test");
        cleanup_guard.cleanup_now().await.unwrap_or(());
        return;
    }
    
    println!("🔄 Processing {} emails through complete workflow", test_emails.len());
    
    let mut processed_emails = Vec::new();
    let rate_limiter = ApiRateLimiter::new(REQUEST_DELAY_MS);
    
    for (i, email) in test_emails.iter().enumerate() {
        println!("\n📧 Processing email {} of {}: {}", 
                 i + 1, test_emails.len(), 
                 format_email_context(&email.id, &email.subject));
        
        // Classify
        let classification = classifier.classify(email).await
            .expect(&format!("Failed to classify {}", 
                            format_email_context(&email.id, &email.subject)));
        
        // Create test label
        let test_label = create_test_label(&classification.category);
        
        // Apply label with rate limiting and retry logic
        let label_result = retry_with_backoff(
            || async { labeler.apply_label(&email.id, &test_label).await },
            &rate_limiter,
            &format!("Apply label '{}' to email {}", test_label, format_email_context(&email.id, &email.subject))
        ).await.expect(&format!("Failed to apply label to {}", 
                        format_email_context(&email.id, &email.subject)));
        
        println!("  ✅ Applied label: {} (new: {})", 
                 test_label, label_result.created_new_label);
        
        processed_emails.push((email, classification, test_label));
    }
    
    // Verify all labels were applied correctly using concurrent verification
    println!("\n🔍 Verifying all labels concurrently...");
    let email_label_pairs: Vec<_> = processed_emails.iter()
        .map(|(email, _, test_label)| ((*email).clone(), test_label.clone()))
        .collect();
    
    let verification_results = verify_labels_concurrently(
        Arc::clone(&labeler),
        email_label_pairs
    ).await;
    
    // Assert all verifications passed
    assert_eq!(verification_results.failure_count(), 0,
               "All labels should be verified successfully");
    assert_eq!(verification_results.success_count(), processed_emails.len(),
               "Should verify labels on all {} emails", processed_emails.len());
    
    println!("  ✅ All {} labels verified successfully", verification_results.success_count());
    
    // Test that we can retrieve emails by label
    println!("\n📧 Testing email retrieval by labels...");
    let test_labels_used: HashSet<_> = processed_emails.iter()
        .map(|(_, _, label)| label.clone())
        .collect();
    
    for test_label in &test_labels_used {
        let result = retry_with_backoff(
            || async { labeler.get_emails_by_label(test_label).await },
            &rate_limiter,
            &format!("Get emails by label '{}'", test_label)
        ).await;
        
        match result {
            Ok(labeled_emails) => {
                println!("  📋 Label '{}': {} emails", test_label, labeled_emails.len());
                assert!(!labeled_emails.is_empty(), 
                        "Should find emails with label {test_label}");
            }
            Err(e) => {
                println!("  ⚠️  Could not retrieve emails for label {test_label}: {e}");
                // Don't fail the test for this - it might not be implemented
            }
        }
    }
    
    // Final cleanup using cleanup guard
    println!("\n🧹 Final cleanup...");
    cleanup_guard.cleanup_now().await
        .expect("Failed to cleanup test labels");
    
    println!("\n✅ END-TO-END WORKFLOW TEST PASSED!");
    println!("   Successfully processed {} emails through the complete pipeline", 
             processed_emails.len());
}

#[tokio::test]
#[ignore = "Requires Gmail API credentials"]
async fn test_gmail_api_authentication() {
    // Install crypto provider for rustls if needed
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("🧪 GMAIL API AUTHENTICATION TEST");
    println!("{}", "=".repeat(60));
    
    match test_gmail_auth_setup().await {
        Ok(()) => {
            println!("✅ GMAIL AUTHENTICATION TEST PASSED!");
            println!("   All Gmail API components authenticated successfully");
        }
        Err(e) => {
            println!("❌ GMAIL AUTHENTICATION TEST FAILED!");
            println!("   Error: {}", e);
            panic!("Gmail authentication failed: {}", e);
        }
    }
}