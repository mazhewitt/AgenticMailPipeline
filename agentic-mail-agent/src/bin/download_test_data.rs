#!/usr/bin/env cargo run --bin download_test_data --
//! Test data downloader for Gmail emails.
//! 
//! This binary downloads the first 20 emails from your Gmail inbox
//! and saves them as JSON files that can be used as test data for
//! the email classifier.
//! 
//! Usage:
//!   cargo run --bin download_test_data
//! 
//! Environment Variables:
//!   GMAIL_CLIENT_SECRET_JSON - Path to OAuth2 client secret JSON file
//!   GMAIL_TOKEN_JSON - Path to OAuth2 token JSON file
//!   TEST_DATA_DIR - Directory to save test data files (default: test_data)

use agentic_mail_agent::fetcher::{EmailFetcher, GmailFetcher};
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("🔧 Gmail Test Data Downloader");
    println!("================================");
    
    // Get output directory from environment or use default
    let test_data_dir = std::env::var("TEST_DATA_DIR").unwrap_or_else(|_| "test_data".to_string());
    
    // Create test data directory if it doesn't exist
    if !Path::new(&test_data_dir).exists() {
        println!("📁 Creating test data directory: {}", test_data_dir);
        fs::create_dir_all(&test_data_dir)?;
    }
    
    // Initialize Gmail fetcher
    println!("🔑 Initializing Gmail API connection...");
    let fetcher = match GmailFetcher::from_env().await {
        Ok(fetcher) => {
            println!("✅ Successfully connected to Gmail API");
            fetcher
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize Gmail fetcher: {}", e);
            eprintln!();
            eprintln!("💡 Make sure you have set the following environment variables:");
            eprintln!("   GMAIL_CLIENT_SECRET_JSON=/path/to/client_secret.json");
            eprintln!("   GMAIL_TOKEN_JSON=/path/to/token.json");
            eprintln!();
            eprintln!("🔧 Run the Gmail setup script if you haven't already:");
            eprintln!("   ./setup_gmail_auth.sh");
            std::process::exit(1);
        }
    };
    
    // Get number of emails to download from environment or use default
    let email_count: u32 = std::env::var("EMAIL_COUNT")
        .unwrap_or_else(|_| "20".to_string())
        .parse()
        .unwrap_or(20);
    
    // Fetch emails from inbox
    println!("📧 Fetching emails from Gmail inbox (limit: {})...", email_count);
    let emails = match fetcher.fetch_inbox_emails(email_count).await {
        Ok(emails) => {
            println!("✅ Successfully fetched {} emails", emails.len());
            emails
        }
        Err(e) => {
            eprintln!("❌ Failed to fetch emails: {}", e);
            std::process::exit(1);
        }
    };
    
    if emails.is_empty() {
        println!("⚠️  No emails found in inbox");
        return Ok(());
    }
    
    // Save each email as a separate JSON file
    println!("💾 Saving emails as test data files...");
    for (index, email) in emails.iter().enumerate() {
        let filename = format!("email_{:03}.json", index + 1);
        let filepath = Path::new(&test_data_dir).join(&filename);
        
        // Create a serializable version of the email with additional metadata
        let test_email = TestDataEmail {
            id: email.id.clone(),
            subject: email.subject.clone(),
            snippet: email.snippet.clone(),
            from: email.from.clone(),
            to: email.to.clone(),
            sent: email.sent.clone(),
            body: email.body.clone(),
            downloaded_at: chrono::Utc::now().to_rfc3339(),
            file_index: index + 1,
        };
        
        // Save to JSON file
        let json_content = serde_json::to_string_pretty(&test_email)?;
        fs::write(&filepath, json_content)?;
        
        println!("  📄 Saved: {} (ID: {})", filename, email.id);
        if let Some(subject) = &email.subject {
            println!("     Subject: {}", subject);
        }
        if let Some(from) = &email.from {
            println!("     From: {}", from);
        }
        if let Some(to) = &email.to {
            println!("     To: {}", to.join(", "));
        }
        if let Some(sent) = &email.sent {
            println!("     Date: {}", sent);
        }
        if let Some(snippet) = &email.snippet {
            let preview = if snippet.chars().count() > 100 {
                let truncated: String = snippet.chars().take(100).collect();
                format!("{}...", truncated)
            } else {
                snippet.clone()
            };
            println!("     Preview: {}", preview);
        }
        if let Some(body) = &email.body {
            let body_preview = if body.chars().count() > 150 {
                let truncated: String = body.chars().take(150).collect();
                format!("{}...", truncated)
            } else {
                body.clone()
            };
            println!("     Body: {}", body_preview);
        }
        println!();
    }
    
    // Create a manifest file with summary information
    let manifest = TestDataManifest {
        created_at: chrono::Utc::now().to_rfc3339(),
        total_emails: emails.len(),
        emails: emails.iter().enumerate().map(|(index, email)| {
            EmailSummary {
                file_index: index + 1,
                filename: format!("email_{:03}.json", index + 1),
                id: email.id.clone(),
                subject: email.subject.clone(),
                has_snippet: email.snippet.is_some(),
                has_from: email.from.is_some(),
                has_to: email.to.is_some(),
                has_sent: email.sent.is_some(),
                has_body: email.body.is_some(),
            }
        }).collect(),
    };
    
    let manifest_path = Path::new(&test_data_dir).join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_json)?;
    
    println!("✅ Test data download complete!");
    println!("📊 Summary:");
    println!("   • Downloaded: {} emails", emails.len());
    println!("   • Saved to: {}/", test_data_dir);
    println!("   • Manifest: {}/manifest.json", test_data_dir);
    println!();
    println!("🔬 You can now use these files for testing the email classifier:");
    println!("   cargo test -- --test-threads=1");
    
    Ok(())
}

/// Extended email structure for test data with metadata
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct TestDataEmail {
    /// Original email ID from Gmail
    pub id: String,
    /// Email subject line
    pub subject: Option<String>,
    /// Email snippet/preview
    pub snippet: Option<String>,
    /// Sender's email address
    pub from: Option<String>,
    /// Recipient email addresses
    pub to: Option<Vec<String>>,
    /// Sent timestamp (ISO 8601 format)
    pub sent: Option<String>,
    /// Full email body content
    pub body: Option<String>,
    /// Timestamp when this test data was downloaded
    pub downloaded_at: String,
    /// Index in the downloaded batch (1-based)
    pub file_index: usize,
}

impl TestDataEmail {
    /// Convert to the Email type used by the classifier
    #[allow(dead_code)]
    fn to_email(&self) -> agentic_mail_agent::core::email::Email {
        agentic_mail_agent::core::email::Email::new_full(
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

/// Summary information about a test email file
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct EmailSummary {
    /// Index in the downloaded batch (1-based)
    pub file_index: usize,
    /// Filename of the test data file
    pub filename: String,
    /// Original email ID from Gmail
    pub id: String,
    /// Email subject line
    pub subject: Option<String>,
    /// Whether the email has snippet content
    pub has_snippet: bool,
    /// Whether the email has from address
    pub has_from: bool,
    /// Whether the email has to addresses
    pub has_to: bool,
    /// Whether the email has sent timestamp
    pub has_sent: bool,
    /// Whether the email has body content
    pub has_body: bool,
}

/// Manifest file containing metadata about all downloaded test emails
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct TestDataManifest {
    /// Timestamp when the test data was created
    pub created_at: String,
    /// Total number of emails downloaded
    pub total_emails: usize,
    /// Summary information for each email
    pub emails: Vec<EmailSummary>,
}
