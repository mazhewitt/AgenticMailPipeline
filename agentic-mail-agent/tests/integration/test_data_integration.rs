//! Integration tests for using downloaded Gmail test data with the email classifier.
//! 
//! These tests demonstrate how to load and use the test data created by the
//! `download_test_data` binary for testing email classification functionality.

use agentic_mail_agent::core::email::Email;
use std::fs;
use std::path::Path;

/// Test data email structure matching the downloaded format
#[derive(serde::Deserialize, Debug)]
struct TestDataEmail {
    id: String,
    subject: Option<String>,
    snippet: Option<String>,
    #[allow(dead_code)] // Used for metadata but not directly in tests
    downloaded_at: String,
    file_index: usize,
}

impl TestDataEmail {
    /// Convert to the Email type used by the classifier
    fn to_email(&self) -> Email {
        Email::new(
            self.id.clone(),
            self.subject.clone(),
            self.snippet.clone(),
        )
    }
}

/// Load a test email from the test_data directory
fn load_test_email(filename: &str) -> Result<TestDataEmail, Box<dyn std::error::Error>> {
    let path = Path::new("test_data").join(filename);
    let content = fs::read_to_string(path)?;
    let test_email: TestDataEmail = serde_json::from_str(&content)?;
    Ok(test_email)
}

/// Load all test emails from manifest
fn load_all_test_emails() -> Result<Vec<TestDataEmail>, Box<dyn std::error::Error>> {
    let manifest_path = Path::new("test_data").join("manifest.json");
    
    if !manifest_path.exists() {
        return Err("Test data not found. Run 'cargo run --bin download_test_data' first.".into());
    }
    
    #[derive(serde::Deserialize)]
    struct Manifest {
        emails: Vec<ManifestEntry>,
    }
    
    #[derive(serde::Deserialize)]
    struct ManifestEntry {
        filename: String,
    }
    
    let manifest_content = fs::read_to_string(manifest_path)?;
    let manifest: Manifest = serde_json::from_str(&manifest_content)?;
    
    let mut emails = Vec::new();
    for entry in manifest.emails {
        emails.push(load_test_email(&entry.filename)?);
    }
    
    Ok(emails)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_single_email() {
        // This test verifies we can load individual test email files
        match load_test_email("email_001.json") {
            Ok(test_email) => {
                println!("✅ Successfully loaded test email:");
                println!("   ID: {}", test_email.id);
                println!("   Subject: {:?}", test_email.subject);
                println!("   File Index: {}", test_email.file_index);
                
                // Convert to Email and verify basic properties
                let email = test_email.to_email();
                assert_eq!(email.id, test_email.id);
                assert_eq!(email.subject, test_email.subject);
                assert_eq!(email.snippet, test_email.snippet);
            }
            Err(e) => {
                println!("⚠️  Could not load test data: {e}");
                println!("💡 Run 'cargo run --bin download_test_data' to create test data");
            }
        }
    }

    #[test]
    fn test_load_all_emails() {
        // This test verifies we can load all test emails from the manifest
        match load_all_test_emails() {
            Ok(emails) => {
                println!("✅ Successfully loaded {} test emails", emails.len());
                
                // Verify we have the expected number of emails
                assert!(!emails.is_empty(), "Should have downloaded some emails");
                assert!(emails.len() <= 20, "Should not exceed the download limit");
                
                // Check that all emails have unique IDs
                let mut ids = std::collections::HashSet::new();
                for email in &emails {
                    assert!(ids.insert(email.id.clone()), "Duplicate email ID: {}", email.id);
                }
                
                // Print some statistics
                let with_subject = emails.iter().filter(|e| e.subject.is_some()).count();
                let with_snippet = emails.iter().filter(|e| e.snippet.is_some()).count();
                
                println!("📊 Test Data Statistics:");
                println!("   • Total emails: {}", emails.len());
                println!("   • With subjects: {with_subject}");
                println!("   • With snippets: {with_snippet}");
            }
            Err(e) => {
                println!("⚠️  Could not load test data: {e}");
                println!("💡 Run 'cargo run --bin download_test_data' to create test data");
            }
        }
    }

    #[test]
    fn test_email_conversion() {
        // Test that our test data converts properly to Email objects
        let test_email = TestDataEmail {
            id: "test-123".to_string(),
            subject: Some("Test Subject".to_string()),
            snippet: Some("Test snippet content".to_string()),
            downloaded_at: "2025-06-30T12:00:00Z".to_string(),
            file_index: 1,
        };
        
        let email = test_email.to_email();
        
        assert_eq!(email.id, "test-123");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, Some("Test snippet content".to_string()));
        assert_eq!(email.subject_or_default(), "Test Subject");
        assert_eq!(email.snippet_or_default(), "Test snippet content");
    }

    #[tokio::test] 
    async fn test_prepare_data_for_classifier() {
        // This test demonstrates preparing real Gmail data for classifier testing
        
        match load_all_test_emails() {
            Ok(test_emails) => {
                println!("🧪 Preparing {} real emails for classification", test_emails.len());
                
                // Convert first few emails to Email objects for testing
                let emails: Vec<Email> = test_emails.into_iter()
                    .take(5) // Test with first 5 emails to keep test fast
                    .map(|te| te.to_email())
                    .collect();
                
                println!("📋 Sample emails ready for classification:");
                for (i, email) in emails.iter().enumerate() {
                    let preview = email.snippet_or_default().chars().take(50).collect::<String>();
                    println!("   {}. {} - {}...", 
                        i + 1, 
                        email.subject_or_default(),
                        preview
                    );
                }
                
                // Verify emails are properly formatted
                for email in &emails {
                    assert!(!email.id.is_empty(), "Email ID should not be empty");
                    // Note: subject and snippet can be None, that's valid
                }
                
                println!("✅ Test data is ready for classifier testing");
                println!("💡 You can now use these Email objects with MessageClassifier::classify()");
            }
            Err(e) => {
                println!("⚠️  Could not load test data: {e}");
                println!("💡 Run 'cargo run --bin download_test_data' to create test data");
            }
        }
    }

    #[test]
    fn test_data_directory_structure() {
        // Verify the expected test data directory structure exists
        let test_data_dir = Path::new("test_data");
        
        if test_data_dir.exists() {
            println!("✅ Test data directory found");
            
            // Check for manifest
            let manifest_path = test_data_dir.join("manifest.json");
            assert!(manifest_path.exists(), "manifest.json should exist");
            
            // Check that we have some email files
            let email_files: Vec<_> = fs::read_dir(test_data_dir)
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry.file_name().to_string_lossy().starts_with("email_") &&
                    entry.file_name().to_string_lossy().ends_with(".json")
                })
                .collect();
            
            assert!(!email_files.is_empty(), "Should have some email_*.json files");
            println!("📁 Found {} email files", email_files.len());
        } else {
            println!("⚠️  Test data directory not found");
            println!("💡 Run 'cargo run --bin download_test_data' to create test data");
        }
    }
}
