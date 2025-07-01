#!/usr/bin/env cargo run --bin pii_anonymize --
//! New PII-based anonymization tool using the intelligent PII detection pipeline.
//! 
//! This tool implements the new architecture:
//! 1. Use LLM to detect and list all PII entities with positions
//! 2. Replace PII entities in Rust code with realistic fake data
//! 3. Maintain auditability and consistency
//! 4. Include fallback mechanisms for critical PII types
//! 
//! Usage:
//!   cargo run --bin pii_anonymize -- --input-dir temp_test_data_raw --output-dir temp_anonymized_pii
//!   cargo run --bin pii_anonymize -- --backend openai --input-dir temp_test_data_raw --output-dir temp_anonymized_pii

use agentic_mail_agent::anonymizer::{
    AnonymizationPipeline, AnonymizationConfig, LlmBackend
};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tokio;

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

impl CliConfig {
    fn from_args() -> Result<Self, Box<dyn std::error::Error>> {
        let args: Vec<String> = std::env::args().collect();
        
        let mut input_dir = None;
        let mut output_dir = None;
        let mut backend = LlmBackend::Ollama;
        let mut model = None;
        let mut max_emails = None;
        
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--input-dir" => {
                    if i + 1 < args.len() {
                        input_dir = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        return Err("--input-dir requires a directory path".into());
                    }
                }
                "--output-dir" => {
                    if i + 1 < args.len() {
                        output_dir = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        return Err("--output-dir requires a directory path".into());
                    }
                }
                "--backend" => {
                    if i + 1 < args.len() {
                        backend = match args[i + 1].to_lowercase().as_str() {
                            "ollama" => LlmBackend::Ollama,
                            "openai" => LlmBackend::OpenAI,
                            _ => return Err("--backend must be either 'ollama' or 'openai'".into()),
                        };
                        i += 2;
                    } else {
                        return Err("--backend requires a backend type".into());
                    }
                }
                "--model" => {
                    if i + 1 < args.len() {
                        model = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err("--model requires a model name".into());
                    }
                }
                "--max-emails" => {
                    if i + 1 < args.len() {
                        max_emails = Some(args[i + 1].parse()?);
                        i += 2;
                    } else {
                        return Err("--max-emails requires a number".into());
                    }
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                _ => {
                    return Err(format!("Unknown argument: {}", args[i]).into());
                }
            }
        }
        
        let input_dir = input_dir.ok_or("--input-dir is required")?;
        let output_dir = output_dir.ok_or("--output-dir is required")?;
        
        Ok(Self {
            input_dir,
            output_dir,
            backend,
            model,
            max_emails,
        })
    }
}

fn print_usage() {
    println!("PII-based Email Anonymization Tool");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin pii_anonymize -- [OPTIONS] --input-dir <DIR> --output-dir <DIR>");
    println!("");
    println!("OPTIONS:");
    println!("    --input-dir <DIR>       Directory containing JSON email files to anonymize");
    println!("    --output-dir <DIR>      Directory to write anonymized email files");
    println!("    --backend <BACKEND>     LLM backend: 'ollama' or 'openai' [default: ollama]");
    println!("    --model <MODEL>         Model name (e.g., 'llama3.1:8b' for Ollama, 'gpt-4o-mini' for OpenAI)");
    println!("    --max-emails <NUM>      Maximum number of emails to process [default: all]");
    println!("    --help, -h              Print this help message");
    println!("");
    println!("EXAMPLES:");
    println!("    cargo run --bin pii_anonymize -- --input-dir temp_test_data_raw --output-dir temp_anonymized_pii");
    println!("    cargo run --bin pii_anonymize -- --backend openai --input-dir temp_test_data_raw --output-dir temp_anonymized_pii");
    println!("    cargo run --bin pii_anonymize -- --backend ollama --model phi3:mini --max-emails 5 --input-dir temp_test_data_raw --output-dir temp_anonymized_pii");
}

/// Statistics for the anonymization process
#[derive(Debug, Default)]
struct AnonymizationStats {
    total_emails: usize,
    processed_emails: usize,
    failed_emails: usize,
    total_pii_detected: usize,
    total_pii_replaced: usize,
    total_duration: Duration,
}

impl AnonymizationStats {
    fn add_email_result(&mut self, detected_count: usize, replaced_count: usize, duration: Duration) {
        self.processed_emails += 1;
        self.total_pii_detected += detected_count;
        self.total_pii_replaced += replaced_count;
        self.total_duration += duration;
    }
    
    fn add_failed_email(&mut self) {
        self.failed_emails += 1;
    }
    
    fn print_summary(&self) {
        println!("\n📊 Anonymization Summary:");
        println!("   • Total emails: {}", self.total_emails);
        println!("   • Successfully processed: {}", self.processed_emails);
        println!("   • Failed: {}", self.failed_emails);
        println!("   • Total PII detected: {}", self.total_pii_detected);
        println!("   • Total PII replaced: {}", self.total_pii_replaced);
        println!("   • Total time: {:.2}s", self.total_duration.as_secs_f64());
        
        if self.processed_emails > 0 {
            let avg_per_email = self.total_duration.as_secs_f64() / self.processed_emails as f64;
            let avg_pii_per_email = self.total_pii_detected as f64 / self.processed_emails as f64;
            println!("   • Average per email: {:.2}s", avg_per_email);
            println!("   • Average PII per email: {:.1}", avg_pii_per_email);
        }
    }
}

async fn anonymize_email_file(
    pipeline: &mut AnonymizationPipeline,
    input_path: &Path,
    output_path: &Path,
) -> Result<(usize, usize, Duration), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    // Read the email file
    let content = fs::read_to_string(input_path)?;
    let mut email: TestEmail = serde_json::from_str(&content)?;
    
    // Combine all text fields for PII detection
    let mut full_text = String::new();
    let mut field_offsets = HashMap::new();
    
    if let Some(subject) = &email.subject {
        field_offsets.insert("subject", full_text.len());
        full_text.push_str("Subject: ");
        full_text.push_str(subject);
        full_text.push('\n');
    }
    
    if let Some(from) = &email.from {
        field_offsets.insert("from", full_text.len());
        full_text.push_str("From: ");
        full_text.push_str(from);
        full_text.push('\n');
    }
    
    if let Some(to) = &email.to {
        field_offsets.insert("to", full_text.len());
        full_text.push_str("To: ");
        full_text.push_str(&to.join(", "));
        full_text.push('\n');
    }
    
    if let Some(body) = &email.body {
        field_offsets.insert("body", full_text.len());
        full_text.push_str("Body: ");
        full_text.push_str(body);
        full_text.push('\n');
    }
    
    if let Some(snippet) = &email.snippet {
        field_offsets.insert("snippet", full_text.len());
        full_text.push_str("Snippet: ");
        full_text.push_str(snippet);
        full_text.push('\n');
    }
    
    // Anonymize the combined text
    let result = pipeline.anonymize_email_text(&full_text).await?;
    
    // Parse the anonymized text back into fields
    let mut body_lines = Vec::new();
    let mut processing_body = false;
    
    for line in result.anonymized_text.lines() {
        if let Some(stripped) = line.strip_prefix("Subject: ") {
            email.subject = Some(stripped.to_string());
            processing_body = false;
        } else if let Some(stripped) = line.strip_prefix("From: ") {
            email.from = Some(stripped.to_string());
            processing_body = false;
        } else if let Some(stripped) = line.strip_prefix("To: ") {
            email.to = Some(stripped.split(", ").map(|s| s.to_string()).collect());
            processing_body = false;
        } else if let Some(stripped) = line.strip_prefix("Body: ") {
            body_lines.clear();
            body_lines.push(stripped.to_string());
            processing_body = true;
        } else if let Some(stripped) = line.strip_prefix("Snippet: ") {
            email.snippet = Some(stripped.to_string());
            processing_body = false;
        } else if processing_body {
            // Continue collecting body lines
            body_lines.push(line.to_string());
        }
    }
    
    // Set the body if we collected lines
    if !body_lines.is_empty() {
        email.body = Some(body_lines.join("\n"));
    }
    
    // Write the anonymized email
    let anonymized_content = serde_json::to_string_pretty(&email)?;
    fs::write(output_path, anonymized_content)?;
    
    let duration = start_time.elapsed();
    Ok((result.detected_entities.len(), result.replacement_log.len(), duration))
}

async fn process_directory(
    input_dir: &Path,
    output_dir: &Path,
    pipeline: &mut AnonymizationPipeline,
    max_emails: Option<usize>,
) -> Result<AnonymizationStats, Box<dyn std::error::Error>> {
    // Create output directory
    fs::create_dir_all(output_dir)?;
    
    // Find all JSON files in input directory
    let mut json_files = Vec::new();
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            json_files.push(path);
        }
    }
    
    // Sort files for consistent processing
    json_files.sort();
    
    // Limit the number of files if specified
    if let Some(max) = max_emails {
        json_files.truncate(max);
    }
    
    let mut stats = AnonymizationStats::default();
    stats.total_emails = json_files.len();
    
    println!("🔍 Found {} JSON files to process", json_files.len());
    
    for (i, input_path) in json_files.iter().enumerate() {
        let file_name = input_path.file_name().unwrap();
        let output_path = output_dir.join(file_name);
        
        print!("Processing {}/{}: {} ... ", i + 1, json_files.len(), file_name.to_string_lossy());
        io::stdout().flush().unwrap();
        
        match anonymize_email_file(pipeline, input_path, &output_path).await {
            Ok((detected, replaced, duration)) => {
                println!("✅ {} PII detected, {} replaced ({:.2}s)", detected, replaced, duration.as_secs_f64());
                stats.add_email_result(detected, replaced, duration);
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                stats.add_failed_email();
            }
        }
    }
    
    Ok(stats)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli_config = CliConfig::from_args()?;
    
    println!("🚀 PII-based Email Anonymization Tool");
    println!("   • Input directory: {}", cli_config.input_dir.display());
    println!("   • Output directory: {}", cli_config.output_dir.display());
    println!("   • Backend: {:?}", cli_config.backend);
    if let Some(model) = &cli_config.model {
        println!("   • Model: {}", model);
    }
    if let Some(max) = cli_config.max_emails {
        println!("   • Max emails: {}", max);
    }
    
    // Validate input directory
    if !cli_config.input_dir.exists() {
        return Err(format!("Input directory does not exist: {}", cli_config.input_dir.display()).into());
    }
    
    // Create anonymization configuration
    let anonymization_config = AnonymizationConfig::new(cli_config.backend, cli_config.model)?;
    
    // Initialize the pipeline
    println!("\n🔧 Initializing anonymization pipeline...");
    let mut pipeline = AnonymizationPipeline::new(anonymization_config).await?;
    
    println!("✅ Anonymization pipeline ready!");
    
    // Process all emails
    let stats = process_directory(
        &cli_config.input_dir,
        &cli_config.output_dir,
        &mut pipeline,
        cli_config.max_emails,
    ).await?;
    
    // Print summary
    stats.print_summary();
    
    println!("\n✅ Anonymization complete! Results saved to: {}", cli_config.output_dir.display());
    
    Ok(())
}
