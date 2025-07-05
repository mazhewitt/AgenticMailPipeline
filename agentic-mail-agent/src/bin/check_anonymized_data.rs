#!/usr/bin/env cargo run --bin check_anonymized_data --
//! Quick script to check the anonymized data for PII

use agentic_mail_agent::test_data_utils::*;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let anonymized_dir = "test_data/anonymized_emails";
    
    println!("🔍 Checking anonymized data for PII...");
    
    // Run the PII spot check
    let warnings = spot_check_for_pii(anonymized_dir).await?;
    
    if warnings.is_empty() {
        println!("✅ No obvious PII found in anonymized data!");
    } else {
        println!("⚠️  Found {} potential PII issues:", warnings.len());
        for warning in &warnings {
            println!("   • {warning}");
        }
    }
    
    // Count the files
    let count = count_json_files(anonymized_dir)?;
    println!("\n📊 Statistics:");
    println!("   • Anonymized emails: {count}");
    
    // Sample a few files to show what they look like
    println!("\n📄 Sample content (first 200 chars of subject/from fields):");
    let mut sample_count = 0;
    for entry in fs::read_dir(anonymized_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") && sample_count < 3 {
            let content = fs::read_to_string(&path)?;
            let email: serde_json::Value = serde_json::from_str(&content)?;
            
            let filename = path.file_name().unwrap().to_string_lossy();
            println!("\n   {filename}:");
            
            if let Some(subject) = email.get("subject").and_then(|v| v.as_str()) {
                let preview = if subject.len() > 80 {
                    format!("{}...", &subject[..80])
                } else {
                    subject.to_string()
                };
                println!("     Subject: {preview}");
            }
            
            if let Some(from) = email.get("from").and_then(|v| v.as_str()) {
                let preview = if from.len() > 80 {
                    format!("{}...", &from[..80])
                } else {
                    from.to_string()
                };
                println!("     From: {preview}");
            }
            
            sample_count += 1;
        }
    }
    
    println!("\n✅ PII check complete!");
    
    Ok(())
}
