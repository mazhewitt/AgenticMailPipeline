//! Test data creation utilities
//! 
//! This module provides functions to download emails from Gmail and anonymize them
//! for use as test data that can be committed to the repository.

use std::process::Command;
use std::fs;
use std::path::Path;

/// Download 50 emails from Gmail inbox using the existing download_test_data binary
pub async fn download_50_emails(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("📧 Downloading 50 emails from Gmail inbox...");
    
    // Create output directory
    fs::create_dir_all(output_dir)?;
    
    // Set environment variables for the download
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
       .arg("--bin")
       .arg("download_test_data")
       .env("EMAIL_COUNT", "50")
       .env("TEST_DATA_DIR", output_dir);

    // Execute the download command
    let output = cmd.output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Download failed. stdout: {}, stderr: {}", stdout, stderr).into());
    }
    
    // Verify we have some emails
    let count = count_json_files(output_dir)?;
    if count == 0 {
        return Err("No emails were downloaded".into());
    }
    
    println!("✅ Downloaded {} emails to {}/", count, output_dir);
    Ok(())
}

/// Anonymize test data using the existing pii_anonymize binary
pub async fn anonymize_test_data(input_dir: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔒 Anonymizing emails using PII pipeline...");
    
    // Create output directory
    fs::create_dir_all(output_dir)?;
    
    // Build the pii_anonymize command
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
       .arg("--bin")
       .arg("pii_anonymize")
       .arg("--")
       .arg("--input-dir")
       .arg(input_dir)
       .arg("--output-dir")
       .arg(output_dir)
       .arg("--backend")
       .arg("ollama");  // Use Ollama as default backend
    
    // Execute the anonymization command
    let output = cmd.output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Anonymization failed. stdout: {}, stderr: {}", stdout, stderr).into());
    }
    
    // Verify we have anonymized emails
    let count = count_json_files(output_dir)?;
    if count == 0 {
        return Err("No anonymized emails were created".into());
    }
    
    println!("✅ Anonymized {} emails to {}/", count, output_dir);
    Ok(())
}

/// Spot check anonymized emails for any remaining PII
pub async fn spot_check_for_pii(data_dir: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    println!("🔍 Performing PII spot check on anonymized emails...");
    
    let mut warnings = Vec::new();
    let mut checked_count = 0;
    let max_to_check = 10; // Check up to 10 emails as a spot check
    
    // Read a sample of email files
    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if checked_count >= max_to_check {
                break;
            }
            
            let file_warnings = check_file_for_pii(&path).await?;
            warnings.extend(file_warnings);
            checked_count += 1;
        }
    }
    
    if warnings.is_empty() {
        println!("✅ PII spot check passed - no obvious PII found in {} checked files", checked_count);
    } else {
        println!("⚠️  PII spot check found {} potential issues:", warnings.len());
        for warning in &warnings {
            println!("   • {}", warning);
        }
    }
    
    Ok(warnings)
}

/// Check a single email file for potential PII
async fn check_file_for_pii(file_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let email: serde_json::Value = serde_json::from_str(&content)?;
    
    let mut warnings = Vec::new();
    let filename = file_path.file_name().unwrap().to_string_lossy();
    
    // Check for common PII patterns that should have been anonymized
    if let Some(text_fields) = get_text_content(&email) {
        for (field, text) in text_fields {
            // Check for email patterns that aren't the safe anonymized ones
            if check_for_real_emails(&text) {
                warnings.push(format!("{}: Potential real email found in {}", filename, field));
            }
            
            // Check for phone number patterns
            if check_for_real_phones(&text) {
                warnings.push(format!("{}: Potential real phone number found in {}", filename, field));
            }
            
            // Check for common real name patterns (this is heuristic)
            if check_for_suspicious_names(&text) {
                warnings.push(format!("{}: Potentially real names found in {}", filename, field));
            }
        }
    }
    
    Ok(warnings)
}

/// Extract text content from email JSON for PII checking
pub fn get_text_content(email: &serde_json::Value) -> Option<Vec<(String, String)>> {
    let mut fields = Vec::new();
    
    if let Some(subject) = email.get("subject").and_then(|v| v.as_str()) {
        fields.push(("subject".to_string(), subject.to_string()));
    }
    
    if let Some(from) = email.get("from").and_then(|v| v.as_str()) {
        fields.push(("from".to_string(), from.to_string()));
    }
    
    if let Some(body) = email.get("body").and_then(|v| v.as_str()) {
        fields.push(("body".to_string(), body.to_string()));
    }
    
    if let Some(snippet) = email.get("snippet").and_then(|v| v.as_str()) {
        fields.push(("snippet".to_string(), snippet.to_string()));
    }
    
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// Check for real email addresses (not the anonymized ones)
pub fn check_for_real_emails(text: &str) -> bool {
    use regex::Regex;
    
    let email_regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    
    for email_match in email_regex.find_iter(text) {
        let email = email_match.as_str();
        
        // Skip common anonymized patterns
        if email.contains("example.com") || 
           email.contains("user") && email.matches('@').count() == 1 ||
           email.starts_with("user") && email.chars().nth(4).map_or(false, |c| c.is_ascii_digit()) {
            continue;
        }
        
        // This looks like it might be a real email
        return true;
    }
    
    false
}

/// Check for real phone numbers (not the anonymized ones)
pub fn check_for_real_phones(text: &str) -> bool {
    use regex::Regex;
    
    let phone_regex = Regex::new(r"\b(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b").unwrap();
    
    for phone_match in phone_regex.find_iter(text) {
        let phone = phone_match.as_str();
        
        // Skip common fake phone patterns (555 area code, etc.)
        if phone.contains("555") {
            continue;
        }
        
        // This might be a real phone number
        return true;
    }
    
    false
}

/// Check for potentially real names (this is heuristic and may have false positives)
pub fn check_for_suspicious_names(text: &str) -> bool {
    // This is a simple heuristic check
    // Look for common real first names that shouldn't appear in anonymized data
    let common_real_names = [
        "John", "Jane", "Michael", "Sarah", "David", "Emily", "Robert", "Jessica",
        "William", "Ashley", "Richard", "Amanda", "Joseph", "Stephanie", "Thomas", "Michelle"
    ];
    
    for name in &common_real_names {
        if text.contains(name) {
            // Make sure it's not part of a larger word
            let words: Vec<&str> = text.split_whitespace().collect();
            for word in words {
                let clean_word = word.trim_matches(|c: char| !c.is_alphabetic());
                if clean_word.eq_ignore_ascii_case(name) {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Count JSON files in a directory
pub fn count_json_files(dir: &str) -> Result<usize, std::io::Error> {
    let mut count = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            count += 1;
        }
    }
    Ok(count)
}
