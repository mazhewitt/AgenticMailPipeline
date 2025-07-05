use std::fs;
use agentic_mail_agent::test_data_utils::*;

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct TestEmail {
    id: String,
    subject: Option<String>,
    from: Option<String>,
    to: Option<Vec<String>>,
    body: Option<String>,
    snippet: Option<String>,
}

#[tokio::test]
#[ignore] // Requires Gmail credentials and downloads emails - use 'cargo test -- --ignored' to run
async fn test_create_anonymized_test_data() {
    // This test requires actual Gmail credentials and should be run manually
    // Skip if running in CI environment
    if std::env::var("CI").is_ok() {
        println!("Skipping Gmail integration test in CI environment");
        return;
    }

    let raw_data_dir = "temp_test_data_raw";
    let anonymized_data_dir = "test_data/anonymized_emails";
    
    // Clean up any existing directories from previous runs
    let _ = fs::remove_dir_all(raw_data_dir);
    let _ = fs::remove_dir_all(anonymized_data_dir);
    
    // Test that we can download emails
    let download_result = download_50_emails(raw_data_dir).await;
    assert!(download_result.is_ok(), "Failed to download emails: {:?}", download_result);
    
    // Verify we got 50 emails (or as many as available)
    let downloaded_count = count_json_files(raw_data_dir).unwrap();
    assert!(downloaded_count > 0, "No emails were downloaded");
    assert!(downloaded_count <= 55, "Downloaded more than 55 emails: {}", downloaded_count); // Allow some flexibility for Gmail API
    println!("Downloaded {} emails", downloaded_count);
    
    // Test that we can anonymize the emails
    let anonymize_result = anonymize_test_data(raw_data_dir, anonymized_data_dir).await;
    assert!(anonymize_result.is_ok(), "Failed to anonymize emails: {:?}", anonymize_result);
    
    // Verify anonymized emails exist
    let anonymized_count = count_json_files(anonymized_data_dir).unwrap();
    assert_eq!(anonymized_count, downloaded_count, "Mismatch in email counts");
    
    // Spot check for PII in anonymized data
    let pii_warnings = spot_check_for_pii(anonymized_data_dir).await;
    assert!(pii_warnings.is_ok(), "PII check failed: {:?}", pii_warnings);
    
    let warnings = pii_warnings.unwrap();
    if !warnings.is_empty() {
        println!("⚠️  WARNING: Potential PII found in anonymized data:");
        for warning in &warnings {
            println!("   • {}", warning);
        }
        // Don't fail the test, but warn the user
        println!("⚠️  Please review the anonymized data before committing to the repository!");
    }
    
    // Clean up raw data (anonymized data should remain for commit)
    let _ = fs::remove_dir_all(raw_data_dir);
    
    println!("✅ Test data creation workflow completed successfully");
    println!("   • Raw emails downloaded: {}", downloaded_count);
    println!("   • Anonymized emails created: {}", anonymized_count);
    println!("   • Location: {}/", anonymized_data_dir);
    println!("   • Raw data cleaned up from: {}/", raw_data_dir);
}

#[test]
fn test_count_json_files() {
    // Create a temporary directory for testing
    let test_dir = "temp_test_count";
    fs::create_dir_all(test_dir).unwrap();
    
    // Create some test files
    fs::write(format!("{}/test1.json", test_dir), "{}").unwrap();
    fs::write(format!("{}/test2.json", test_dir), "{}").unwrap();
    fs::write(format!("{}/test3.txt", test_dir), "not json").unwrap();
    
    let count = count_json_files(test_dir).unwrap();
    assert_eq!(count, 2);
    
    // Clean up
    fs::remove_dir_all(test_dir).unwrap();
}
