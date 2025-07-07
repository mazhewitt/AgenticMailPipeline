//! Script to delete all agentic labels from Gmail
//!
//! This script will:
//! 1. Find all labels that match agentic patterns (both old and new hierarchical labels)
//! 2. Delete the labels (which automatically removes them from all emails)
//!
//! Usage: cargo run --bin unlabel_agentic_emails

use std::process;

use agentic_mail_agent::config::labels::LabelConfig;
use agentic_mail_agent::gmail::api::GmailApi;
use agentic_mail_agent::gmail::GmailClient;
use google_gmail1::api::Label;

#[tokio::main]
async fn main() {
    // Initialize crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    println!("🔧 Agentic Label Deleter");
    println!("========================");
    println!("⚠️  WARNING: This will DELETE all agentic labels!");
    println!("   Labels will be removed from ALL emails automatically.");
    println!("   Press Ctrl+C to cancel, or Enter to continue...");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read input");
    
    // Initialize Gmail client
    let gmail_client = match GmailClient::from_env().await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("❌ Failed to initialize Gmail client: {}", e);
            eprintln!("   Make sure you have run ./setup_gmail_auth.sh");
            process::exit(1);
        }
    };
    
    // Get all labels from Gmail
    println!("📋 Fetching all Gmail labels...");
    let gmail_labels = match gmail_client.list_labels().await {
        Ok(labels) => labels,
        Err(e) => {
            eprintln!("❌ Failed to fetch Gmail labels: {}", e);
            process::exit(1);
        }
    };
    
    // Find agentic labels (both old and new patterns)
    let agentic_labels = find_agentic_labels(&gmail_labels);
    
    if agentic_labels.is_empty() {
        println!("✅ No agentic labels found. Nothing to do!");
        return;
    }
    
    println!("🔍 Found {} agentic labels:", agentic_labels.len());
    for label in &agentic_labels {
        let name = label.name.as_ref().map(|s| s.as_str()).unwrap_or("Unknown");
        let id = label.id.as_ref().map(|s| s.as_str()).unwrap_or("Unknown");
        println!("   - {} ({})", name, id);
    }
    
    // Delete the labels (this automatically removes them from all emails)
    println!("\n🗑️  Deleting agentic labels...");
    let mut deleted_count = 0;
    
    for label in &agentic_labels {
        if let Some(label_id) = &label.id {
            let label_name = label.name.as_ref().map(|s| s.as_str()).unwrap_or("Unknown");
            match gmail_client.delete_label(label_id).await {
                Ok(_) => {
                    println!("   ✅ Deleted label: {}", label_name);
                    deleted_count += 1;
                }
                Err(e) => {
                    eprintln!("   ❌ Failed to delete label {}: {}", label_name, e);
                }
            }
        }
    }
    
    println!("\n🎉 Label deletion complete!");
    println!("   - {} labels deleted", deleted_count);
    println!("   - All emails with these labels have been automatically unlabeled")
}

/// Find all labels that match agentic patterns (both old and new)
fn find_agentic_labels(gmail_labels: &[Label]) -> Vec<Label> {
    let mut agentic_labels = Vec::new();
    let label_config = LabelConfig::new();
    
    // Get all old-style labels (without Agentic/ prefix)
    let old_labels = vec![
        "Action Required",
        "Interesting",
        "Reference", 
        "Low Priority",
        "Spam",
        "Work",
        "Personal",
        "Promotional",
        "Urgent",
        "Newsletter",
        "Notification",
        "Needs Review",
        // Also check for old AGENT_ prefixed labels
        "AGENT_URGENT",
        "AGENT_PERSONAL",
        "AGENT_PROMOTIONAL",
        "AGENT_SPAM",
        "AGENT_NEEDS_REVIEW",
        "AGENT_WORK",
        "AGENT_INTERESTING",
        "AGENT_REFERENCE",
        "AGENT_NOISE",
        "AGENT_ACTION_REQUIRED",
    ];
    
    // Get all new-style hierarchical labels
    let new_labels = label_config.get_all_production_labels();
    
    // Find matching labels in Gmail
    for label in gmail_labels {
        if let Some(label_name) = &label.name {
            // Check if it's an old-style label
            if old_labels.contains(&label_name.as_str()) {
                agentic_labels.push(label.clone());
            }
            // Check if it's a new-style label  
            else if new_labels.contains(label_name) {
                agentic_labels.push(label.clone());
            }
            // Check if it starts with "Agentic/"
            else if label_name.starts_with("Agentic/") {
                agentic_labels.push(label.clone());
            }
            // Check if it starts with "TEST_" (test labels)
            else if label_name.starts_with("TEST_") {
                agentic_labels.push(label.clone());
            }
        }
    }
    
    agentic_labels
}