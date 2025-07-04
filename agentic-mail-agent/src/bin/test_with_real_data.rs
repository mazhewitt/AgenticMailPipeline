#!/usr/bin/env cargo test --
//! Test demonstrating how to use the downloaded test data for classifier testing.

use agentic_mail_agent::email::Email;
use serde_json;
use std::fs;
use std::path::Path;

/// Test data email structure matching the downloaded format
#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct TestDataEmail {
    id: String,
    subject: Option<String>,
    snippet: Option<String>,
    from: Option<String>,
    to: Option<Vec<String>>,
    sent: Option<String>,
    body: Option<String>,
    downloaded_at: String,
    file_index: usize,
}

impl TestDataEmail {
    /// Convert to the Email type used by the classifier
    #[allow(dead_code)]
    fn to_email(&self) -> Email {
        Email::new_full(
            self.id.clone(),
            self.subject.clone(),
            self.snippet.clone(),
            self.from.clone(),
            self.to.clone(),
            self.sent.clone(),
            self.body.clone(),
        )
    }
}

/// Load a test email from the test_data directory
#[allow(dead_code)]
fn load_test_email(filename: &str) -> Result<TestDataEmail, Box<dyn std::error::Error>> {
    let path = Path::new("test_data").join(filename);
    let content = fs::read_to_string(path)?;
    let test_email: TestDataEmail = serde_json::from_str(&content)?;
    Ok(test_email)
}

/// Load all test emails from manifest
#[allow(dead_code)]
fn load_all_test_emails() -> Result<Vec<TestDataEmail>, Box<dyn std::error::Error>> {
    let manifest_path = Path::new("test_data").join("manifest.json");
    
    if !manifest_path.exists() {
        return Err("Test data not found. Run 'cargo run --bin download_test_data' first.".into());
    }
    
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Manifest {
        emails: Vec<ManifestEntry>,
    }
    
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
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
                println!("⚠️  Could not load test data: {}", e);
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
                assert!(emails.len() <= 50, "Should not exceed the download limit");
                
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
                println!("   • With subjects: {}", with_subject);
                println!("   • With snippets: {}", with_snippet);
            }
            Err(e) => {
                println!("⚠️  Could not load test data: {}", e);
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
            from: Some("sender@example.com".to_string()),
            to: Some(vec!["recipient@example.com".to_string()]),
            sent: Some("2025-06-30T12:00:00Z".to_string()),
            body: Some("This is the full email body content.".to_string()),
            downloaded_at: "2025-06-30T12:00:00Z".to_string(),
            file_index: 1,
        };
        
        let email = test_email.to_email();
        
        assert_eq!(email.id, "test-123");
        assert_eq!(email.subject, Some("Test Subject".to_string()));
        assert_eq!(email.snippet, Some("Test snippet content".to_string()));
        assert_eq!(email.from, Some("sender@example.com".to_string()));
        assert_eq!(email.to, Some(vec!["recipient@example.com".to_string()]));
        assert_eq!(email.sent, Some("2025-06-30T12:00:00Z".to_string()));
        assert_eq!(email.body, Some("This is the full email body content.".to_string()));
        assert_eq!(email.snippet, Some("Test snippet content".to_string()));
        assert_eq!(email.subject_or_default(), "Test Subject");
        assert_eq!(email.snippet_or_default(), "Test snippet content");
    }

    #[tokio::test] 
    async fn test_classifier_with_real_data() {
        // This test demonstrates using real Gmail data with the classifier
        // Note: This requires ollama to be running for the LangChain classifier
        
        match load_all_test_emails() {
            Ok(test_emails) => {
                println!("🧪 Testing classifier with {} real emails", test_emails.len());
                
                // Convert first few emails to Email objects for testing
                let emails: Vec<Email> = test_emails.into_iter()
                    .take(5) // Test with first 5 emails to keep test fast
                    .map(|te| te.to_email())
                    .collect();
                
                println!("📋 Sample emails for classification:");
                for (i, email) in emails.iter().enumerate() {
                    println!("   {}. {} - {}", 
                        i + 1, 
                        email.subject_or_default(),
                        email.snippet_or_default().chars().take(50).collect::<String>()
                    );
                }
                
                // You can uncomment the following lines to test with an actual classifier
                // Note: This requires ollama to be running with a compatible model
                
                /* 
                let classifier = LangChainClassifier::new().await.unwrap();
                
                for email in emails {
                    match classifier.classify(&email).await {
                        Ok(classification) => {
                            println!("✅ Email {} classified as: {:?}", 
                                email.subject_or_default(), classification);
                        }
                        Err(e) => {
                            println!("❌ Failed to classify email {}: {}", 
                                email.subject_or_default(), e);
                        }
                    }
                }
                */
                
                println!("✅ Test data is ready for classifier testing");
            }
            Err(e) => {
                println!("⚠️  Could not load test data: {}", e);
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

fn main() {
    println!("This is a test utility for working with real email data.");
    println!("Run the tests with: cargo test --bin test_with_real_data");
}
