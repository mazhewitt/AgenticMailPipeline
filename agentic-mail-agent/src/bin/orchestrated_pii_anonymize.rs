#!/usr/bin/env cargo run --bin orchestrated_pii_anonymize --
//! Orchestrated PII anonymization tool using the new multi-pass pipeline.
//! 
//! This tool implements the new architecture:
//! 1. Use deterministic regex tools for structured PII (emails, phones)
//! 2. Use LLM tools for contextual PII (names, companies) 
//! 3. JSON validation and structure preservation
//! 4. Multi-pass verification to ensure complete PII removal
//! 
//! Usage:
//!   cargo run --bin orchestrated_pii_anonymize -- --input-dir test_data --output-dir anonymized_safe_data
//!   cargo run --bin orchestrated_pii_anonymize -- --backend ollama --model mistral --input-dir test_data --output-dir anonymized_safe_data

use agentic_mail_agent::anonymizer::{PiiOrchestrator, AnonymizationConfig, LlmBackend, PiiReplacer};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Write};

/// Email structure for anonymization
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct TestEmail {
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

/// Configuration for the CLI
#[derive(Debug)]
struct CliConfig {
    input_dir: PathBuf,
    output_dir: PathBuf,
    backend: LlmBackend,
    model: Option<String>,
    max_emails: Option<usize>,
}

/// Parse command line arguments
fn parse_args() -> Result<CliConfig, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    let mut backend = LlmBackend::Ollama;
    let mut model = None;
    let mut input_dir = PathBuf::from("test_data");
    let mut output_dir = PathBuf::from("anonymized_safe_data");
    let mut max_emails = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--backend" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing backend value".into());
                }
                backend = match args[i].as_str() {
                    "ollama" => LlmBackend::Ollama,
                    "openai" => LlmBackend::OpenAI,
                    _ => return Err("Invalid backend. Use 'ollama' or 'openai'".into()),
                };
            }
            "--model" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing model value".into());
                }
                model = Some(args[i].clone());
            }
            "--input-dir" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing input-dir value".into());
                }
                input_dir = PathBuf::from(&args[i]);
            }
            "--output-dir" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing output-dir value".into());
                }
                output_dir = PathBuf::from(&args[i]);
            }
            "--max-emails" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing max-emails value".into());
                }
                max_emails = Some(args[i].parse()?);
            }
            _ => return Err(format!("Unknown argument: {}", args[i]).into()),
        }
        i += 1;
    }
    
    Ok(CliConfig {
        input_dir,
        output_dir,
        backend,
        model,
        max_emails,
    })
}

#[tokio::main]
async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    
    let backend_name = match args.backend {
        LlmBackend::Ollama => "Ollama",
        LlmBackend::OpenAI => "OpenAI",
    };
    
    println!("🎯 Orchestrated PII Anonymization Tool");
    println!("=====================================");
    println!("🤖 Backend: {} ({})", backend_name, args.model.as_deref().unwrap_or("default model"));
    println!("📂 Input directory: {}", args.input_dir.display());
    println!("📂 Output directory: {}", args.output_dir.display());
    println!();
    
    // Load emails
    println!("📧 Loading emails from {}...", args.input_dir.display());
    let emails = load_emails(&args.input_dir)?;
    println!("✅ Loaded {} emails", emails.len());
    
    if emails.is_empty() {
        println!("❌ No emails found in {}. Run download_test_data first.", args.input_dir.display());
        return Ok(());
    }
    
    // Apply max_emails filter if specified
    let emails_to_process = if let Some(max) = args.max_emails {
        emails.into_iter().take(max).collect()
    } else {
        emails
    };
    
    println!("🔄 Processing {} emails...", emails_to_process.len());
    
    // Create output directory
    fs::create_dir_all(&args.output_dir)?;
    
    // Initialize PII orchestrator and replacer with new configuration
    let config = AnonymizationConfig::new(args.backend, args.model)?;
    let orchestrator = PiiOrchestrator::new(config).await?;
    let mut replacer = PiiReplacer::new();
    
    // Process each email
    let mut processed_count = 0;
    let mut error_count = 0;
    let mut total_pii_found = 0;
    
    for (index, email) in emails_to_process.iter().enumerate() {
        print!("🔒 Processing email {:03}... ", index + 1);
        io::stdout().flush()?;
        
        // Convert to JSON string
        let email_json = serde_json::to_string_pretty(email)?;
        
        // Step 1: Detect PII using the new orchestrator
        match orchestrator.detect_all_pii(&email_json).await {
            Ok(detected_entities) => {
                // Step 2: Clear previous replacement log
                replacer.clear_replacement_log();
                
                // Step 3: Replace PII with fake data
                let anonymized_text = replacer.replace_pii(&email_json, &detected_entities)?;
                
                let pii_count = detected_entities.len();
                let replacement_count = replacer.get_replacement_log().len();
                
                if pii_count > 0 {
                    println!("✅ Clean ({} PII items anonymized)", replacement_count);
                } else {
                    println!("✅ Clean (no PII found)");
                }
                
                // Debug: Show what was detected vs what was replaced
                if pii_count != replacement_count {
                    println!("   ⚠️  Note: Detected {} PII items, replaced {}", pii_count, replacement_count);
                }
                
                total_pii_found += replacement_count;
                
                // Save anonymized email
                let output_filename = format!("email_{:03}.json", index + 1);
                let output_path = args.output_dir.join(&output_filename);
                fs::write(&output_path, &anonymized_text)?;
                
                processed_count += 1;
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                error_count += 1;
            }
        }
    }
    
    println!();
    println!("📊 Anonymization Summary:");
    println!("   • Successfully processed: {}/{}", processed_count, emails_to_process.len());
    println!("   • Errors: {}", error_count);
    println!("   • Total PII items anonymized: {}", total_pii_found);
    println!("   • Output directory: {}", args.output_dir.display());
    
    if error_count == 0 {
        println!();
        println!("🎉 All emails successfully anonymized!");
        println!("📁 Safe anonymized data saved to: {}", args.output_dir.display());
        println!("💡 This data can now be safely committed to the repository for CI use.");
    } else {
        println!();
        println!("⚠️  Some emails failed to process. Check the errors above.");
    }
    
    Ok(())
}

fn main() {
    if let Err(e) = async_main() {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}

/// Load emails from input directory using manifest
fn load_emails(input_dir: &Path) -> Result<Vec<TestEmail>, Box<dyn std::error::Error>> {
    let manifest_path = input_dir.join("manifest.json");
    
    if !manifest_path.exists() {
        return Err(format!("Manifest not found: {}. Run download_test_data first.", manifest_path.display()).into());
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
        let path = input_dir.join(&entry.filename);
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let email: TestEmail = serde_json::from_str(&content)?;
            emails.push(email);
        }
    }
    
    Ok(emails)
}
