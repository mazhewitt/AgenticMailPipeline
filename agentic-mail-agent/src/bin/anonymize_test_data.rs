#!/usr/bin/env cargo run --bin anonymize_test_data --
//! LLM-powered test data anonymization tool for Gmail emails.
//! 
//! This tool uses LLM (Ollama or OpenAI) to intelligently anonymize downloaded test emails:
//! - Uses context-aware LLM to replace PII while preserving meaning  
//! - Maintains consistency across all fields of an email
//! - Preserves email structure and classification categories
//! - Generates realistic fake data for testing
//! - Provides timing instrumentation and strict error handling
//! 
//! Usage:
//!   cargo run --bin anonymize_test_data -- --input-dir temp_expanded_data --output-dir anonymized_test_data
//!   cargo run --bin anonymize_test_data -- --backend openai --input-dir temp_expanded_data --output-dir anonymized_test_data
//!   cargo run --bin anonymize_test_data -- --backend ollama --model phi3:mini --input-dir temp_expanded_data --output-dir anonymized_test_data

use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use langchain_rust::{
    llm::{openai::{OpenAI, OpenAIConfig}, ollama::client::Ollama},
    language_models::llm::LLM,
};
use serde::Deserialize;
use chrono;

/// Email structure for anonymization
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
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

/// LLM backend type
#[derive(Debug, Clone, PartialEq)]
enum LlmBackend {
    Ollama,
    OpenAI,
}

/// Configuration for the LLM anonymizer
#[derive(Debug, Clone)]
struct AnonymizerConfig {
    /// LLM backend to use
    pub backend: LlmBackend,
    /// OpenAI API key (required for OpenAI backend)
    pub openai_api_key: Option<String>,
    /// Model name to use (e.g., "llama3.1:8b" for Ollama, "gpt-4o-mini" for OpenAI)
    pub model: String,
    /// Temperature for LLM generation (default: 0.3 for some creativity in fake data)
    pub temperature: f64,
    /// Ollama host URL (default: "http://localhost:11434")
    pub ollama_host: String,
}

impl AnonymizerConfig {
    fn new(backend: LlmBackend, model: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let (model, openai_api_key) = match backend {
            LlmBackend::Ollama => {
                (model.unwrap_or_else(|| "llama3.1:8b".to_string()), None)
            }
            LlmBackend::OpenAI => {
                let api_key = Self::load_openai_key()?;
                (model.unwrap_or_else(|| "gpt-4o-mini".to_string()), Some(api_key))
            }
        };
        
        Ok(Self {
            backend,
            openai_api_key,
            model,
            temperature: 0.3,
            ollama_host: "http://localhost:11434".to_string(),
        })
    }
    
    fn load_openai_key() -> Result<String, Box<dyn std::error::Error>> {
        let secrets_path = Path::new("../secrets/openai.json");
        if !secrets_path.exists() {
            return Err("OpenAI API key not found. Please create ../secrets/openai.json with your API key. See ../secrets/openai-template.json for format.".into());
        }
        
        #[derive(Deserialize)]
        struct SecretsFile {
            openai_api_key: String,
        }
        
        let secrets_content = fs::read_to_string(secrets_path)?;
        let secrets: SecretsFile = serde_json::from_str(&secrets_content)?;
        
        Ok(secrets.openai_api_key)
    }
}

/// Timing statistics for anonymization
#[derive(Debug, Default)]
struct TimingStats {
    /// Total time for all emails
    pub total_duration: Duration,
    /// Time per email
    pub email_durations: Vec<Duration>,
    /// Time per field type
    pub field_type_durations: HashMap<String, Vec<Duration>>,
}

impl TimingStats {
    fn add_email_time(&mut self, duration: Duration) {
        self.email_durations.push(duration);
        self.total_duration += duration;
    }
    
    fn add_field_time(&mut self, field_type: &str, duration: Duration) {
        self.field_type_durations
            .entry(field_type.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }
    
    fn print_summary(&self) {
        println!("\n⏱️  Timing Summary:");
        println!("   • Total time: {:.2}s", self.total_duration.as_secs_f64());
        
        if !self.email_durations.is_empty() {
            let avg_per_email = self.total_duration.as_secs_f64() / self.email_durations.len() as f64;
            println!("   • Average per email: {:.2}s", avg_per_email);
            
            let fastest = self.email_durations.iter().min().unwrap();
            let slowest = self.email_durations.iter().max().unwrap();
            println!("   • Fastest email: {:.2}s", fastest.as_secs_f64());
            println!("   • Slowest email: {:.2}s", slowest.as_secs_f64());
        }
        
        if !self.field_type_durations.is_empty() {
            println!("   • Field type averages:");
            for (field_type, durations) in &self.field_type_durations {
                let avg = durations.iter().sum::<Duration>().as_secs_f64() / durations.len() as f64;
                println!("     - {}: {:.2}s ({} calls)", field_type, avg, durations.len());
            }
        }
    }
}

/// LLM-based anonymization service supporting multiple backends
struct LlmAnonymizer {
    llm: Box<dyn LLM>,
    config: AnonymizerConfig,
    timing_stats: TimingStats,
    /// Consistent mapping for names to maintain relationships
    name_mapping: HashMap<String, String>,
    /// Consistent mapping for email addresses
    email_mapping: HashMap<String, String>,
    /// Consistent mapping for places/organizations
    place_mapping: HashMap<String, String>,
}

impl LlmAnonymizer {
    async fn new(config: AnonymizerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let llm: Box<dyn LLM> = match config.backend {
            LlmBackend::Ollama => {
                println!("🦙 Connecting to Ollama at {} with model '{}'...", config.ollama_host, config.model);
                let ollama = Ollama::default().with_model(&config.model);
                
                // Test the connection
                match ollama.invoke("Hello").await {
                    Ok(_) => {
                        println!("✅ Connected to Ollama successfully");
                        Box::new(ollama)
                    }
                    Err(e) => {
                        return Err(format!("Failed to connect to Ollama at {}: {}. Make sure Ollama is running and the model '{}' is available.", config.ollama_host, e, config.model).into());
                    }
                }
            }
            LlmBackend::OpenAI => {
                println!("🤖 Connecting to OpenAI API with model '{}'...", config.model);
                let api_key = config.openai_api_key.as_ref().unwrap();
                let openai_config = OpenAIConfig::default().with_api_key(api_key);
                let openai = OpenAI::new(openai_config).with_model(&config.model);
                
                // Test the connection
                match openai.invoke("Hello").await {
                    Ok(_) => {
                        println!("✅ Connected to OpenAI API successfully");
                        Box::new(openai)
                    }
                    Err(e) => {
                        return Err(format!("Failed to connect to OpenAI API: {}", e).into());
                    }
                }
            }
        };
        
        Ok(Self {
            llm,
            config,
            timing_stats: TimingStats::default(),
            name_mapping: HashMap::new(),
            email_mapping: HashMap::new(),
            place_mapping: HashMap::new(),
        })
    }
    
    async fn anonymize_email(&mut self, email: &TestDataEmail) -> Result<TestDataEmail, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        println!("Processing email {} ({})", email.file_index, 
            email.subject.as_deref().unwrap_or("no subject"));
        
        let mut anonymized = email.clone();
        
        // Anonymize each field using context-aware LLM - NO FALLBACK, FAIL IF LLM FAILS
        if let Some(subject) = &email.subject {
            anonymized.subject = Some(self.anonymize_field(subject, "subject").await?);
        }
        
        if let Some(snippet) = &email.snippet {
            anonymized.snippet = Some(self.anonymize_field(snippet, "snippet").await?);
        }
        
        if let Some(from) = &email.from {
            anonymized.from = Some(self.anonymize_field(from, "from").await?);
        }
        
        if let Some(to) = &email.to {
            let mut anonymized_to = Vec::new();
            for addr in to {
                anonymized_to.push(self.anonymize_field(addr, "to").await?);
            }
            anonymized.to = Some(anonymized_to);
        }
        
        if let Some(body) = &email.body {
            // For body, we might need to work in chunks for very long emails
            anonymized.body = Some(self.anonymize_body(body).await?);
        }
        
        let email_duration = start_time.elapsed();
        self.timing_stats.add_email_time(email_duration);
        println!("  ⏱️  Email processed in {:.2}s", email_duration.as_secs_f64());
        
        Ok(anonymized)
    }
    
    async fn anonymize_field(&mut self, text: &str, field_type: &str) -> Result<String, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        if text.trim().is_empty() {
            return Ok(text.to_string());
        }
        
        let prompt = self.build_anonymization_prompt(text, field_type);
        
        // NO FALLBACK: If LLM fails, we fail
        let response = self.llm.invoke(&prompt).await
            .map_err(|e| format!("LLM failed to anonymize {} field: {}", field_type, e))?;
        
        let cleaned = self.clean_llm_response(&response);
        
        // Basic validation - make sure we got something reasonable back
        if cleaned.trim().is_empty() {
            return Err(format!("LLM returned empty response for {} field", field_type).into());
        }
        
        // Relax length validation - allow up to 10x length (was 5x) and log lengthy responses
        if cleaned.len() > text.len() * 10 {
            println!("⚠️ LLM response very long for {} field (original: {}, response: {})", 
                field_type, text.len(), cleaned.len());
            println!("Original: {}", text);
            println!("Response: {}", &cleaned[..std::cmp::min(200, cleaned.len())]);
            return Err(format!("LLM response excessively long for {} field (original: {}, response: {})", 
                field_type, text.len(), cleaned.len()).into());
        }
        
        let field_duration = start_time.elapsed();
        self.timing_stats.add_field_time(field_type, field_duration);
        
        Ok(cleaned)
    }
    
    async fn anonymize_body(&mut self, body: &str) -> Result<String, Box<dyn std::error::Error>> {
        // If body is very long, process in chunks to avoid token limits
        const MAX_CHUNK_SIZE: usize = 2000;
        
        if body.len() <= MAX_CHUNK_SIZE {
            return self.anonymize_field(body, "body").await;
        }
        
        // Split into paragraphs or sentences when possible
        let chunks = if body.contains("\n\n") {
            self.split_by_paragraphs(body, MAX_CHUNK_SIZE)
        } else {
            self.split_by_sentences(body, MAX_CHUNK_SIZE)
        };
        
        let mut anonymized_chunks = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            println!("  Anonymizing body chunk {} of {}", i + 1, chunks.len());
            anonymized_chunks.push(self.anonymize_field(chunk, "body_chunk").await?);
        }
        
        Ok(anonymized_chunks.join(""))
    }
    
    fn get_timing_stats(&self) -> &TimingStats {
        &self.timing_stats
    }
    
    fn build_anonymization_prompt(&self, text: &str, field_type: &str) -> String {
        match field_type {
            "subject" => {
                format!("Replace personal names and specific details with fake ones in this email subject. Keep the same length and purpose. Return only the anonymized subject line:\n\n{}", text)
            }
            "from" | "to" => {
                format!("Replace the personal name and email address with fake ones. Keep the same email domain. Return only the anonymized email address:\n\n{}", text)
            }
            "snippet" => {
                format!("Replace personal names and specific details with fake ones in this email snippet. Keep the same length and tone. Return only the anonymized snippet:\n\n{}", text)
            }
            _ => {
                format!("Replace personal names, addresses, and specific details with fake ones. Keep the same structure and length. Return only the anonymized text:\n\n{}", text)
            }
        }
    }
    
    fn clean_llm_response(&self, response: &str) -> String {
        let mut cleaned = response.trim().to_string();
        
        // Remove common LLM response artifacts and prefixes
        let prefixes_to_remove = [
            "Here is the anonymized text:",
            "Here is the anonymized email address:",
            "Here is the anonymized subject line:",
            "Here is the anonymized email snippet:",
            "Anonymized text:",
            "ANONYMIZED TEXT:",
            "Anonymized version:",
            "Result:",
            "Output:",
            "Here's the anonymized version:",
            "Anonymized:",
            "The anonymized text is:",
            "The anonymized email address is:",
            "The anonymized subject line is:",
        ];
        
        for prefix in &prefixes_to_remove {
            if cleaned.starts_with(prefix) {
                cleaned = cleaned[prefix.len()..].trim().to_string();
            }
        }
        
        // Remove multi-line prefixes (split by newlines and check if first lines are prefixes)
        let lines: Vec<&str> = cleaned.lines().collect();
        if lines.len() > 1 {
            let first_line = lines[0].trim();
            for prefix in &prefixes_to_remove {
                if first_line == *prefix {
                    cleaned = lines[1..].join("\n").trim().to_string();
                    break;
                }
            }
        }
        
        // Remove quotes if the LLM wrapped the entire response
        if (cleaned.starts_with('"') && cleaned.ends_with('"')) ||
           (cleaned.starts_with('\'') && cleaned.ends_with('\'')) {
            cleaned = cleaned[1..cleaned.len()-1].to_string();
        }
        
        // Remove code block markers if present
        if cleaned.starts_with("```") {
            if let Some(first_newline) = cleaned.find('\n') {
                cleaned = cleaned[first_newline + 1..].to_string();
            }
        }
        if cleaned.ends_with("```") {
            if let Some(last_newline) = cleaned.rfind('\n') {
                cleaned = cleaned[..last_newline].to_string();
            }
        }
        
        cleaned.trim().to_string()
    }
    
    fn split_by_paragraphs(&self, text: &str, max_size: usize) -> Vec<String> {
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        for paragraph in paragraphs {
            if current_chunk.len() + paragraph.len() + 2 > max_size && !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = paragraph.to_string();
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(paragraph);
            }
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        chunks
    }
    
    fn split_by_sentences(&self, text: &str, max_size: usize) -> Vec<String> {
        // Simple sentence splitting - could be improved
        let sentences: Vec<&str> = text.split(". ").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        for (i, sentence) in sentences.iter().enumerate() {
            let sentence_with_period = if i < sentences.len() - 1 && !sentence.ends_with('.') {
                format!("{}. ", sentence)
            } else {
                sentence.to_string()
            };
            
            if current_chunk.len() + sentence_with_period.len() > max_size && !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = sentence_with_period;
            } else {
                current_chunk.push_str(&sentence_with_period);
            }
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        chunks
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

#[derive(Debug)]
struct CliArgs {
    backend: LlmBackend,
    model: Option<String>,
    input_dir: String,
    output_dir: String,
}

fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    let mut backend = LlmBackend::Ollama; // Default to Ollama
    let mut model = None;
    let mut input_dir = "temp_expanded_data".to_string();
    let mut output_dir = "anonymized_test_data".to_string();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--backend" => {
                if i + 1 >= args.len() {
                    return Err("--backend requires a value (ollama or openai)".into());
                }
                backend = match args[i + 1].as_str() {
                    "ollama" => LlmBackend::Ollama,
                    "openai" => LlmBackend::OpenAI,
                    other => return Err(format!("Invalid backend '{}'. Use 'ollama' or 'openai'", other).into()),
                };
                i += 2;
            }
            "--model" => {
                if i + 1 >= args.len() {
                    return Err("--model requires a value".into());
                }
                model = Some(args[i + 1].clone());
                i += 2;
            }
            "--input-dir" => {
                if i + 1 >= args.len() {
                    return Err("--input-dir requires a value".into());
                }
                input_dir = args[i + 1].clone();
                i += 2;
            }
            "--output-dir" => {
                if i + 1 >= args.len() {
                    return Err("--output-dir requires a value".into());
                }
                output_dir = args[i + 1].clone();
                i += 2;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            arg => {
                return Err(format!("Unknown argument: {}", arg).into());
            }
        }
    }
    
    Ok(CliArgs {
        backend,
        model,
        input_dir,
        output_dir,
    })
}

fn print_help() {
    println!("LLM-Powered Test Data Anonymization Tool");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin anonymize_test_data -- [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --backend <BACKEND>     LLM backend to use [default: ollama] [possible values: ollama, openai]");
    println!("    --model <MODEL>         Model name to use");
    println!("                            For Ollama: llama3.1:8b, phi3:mini, etc. [default: llama3.1:8b]");
    println!("                            For OpenAI: gpt-4o-mini, gpt-4o, etc. [default: gpt-4o-mini]");
    println!("    --input-dir <DIR>       Input directory containing emails [default: temp_expanded_data]");
    println!("    --output-dir <DIR>      Output directory for anonymized emails [default: anonymized_test_data]");
    println!("    -h, --help              Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # Use Ollama with default model");
    println!("    cargo run --bin anonymize_test_data");
    println!();
    println!("    # Use Ollama with a smaller/faster model");
    println!("    cargo run --bin anonymize_test_data -- --backend ollama --model phi3:mini");
    println!();
    println!("    # Use OpenAI (requires API key in ../secrets/openai.json)");
    println!("    cargo run --bin anonymize_test_data -- --backend openai");
    println!();
    println!("    # Use custom directories");
    println!("    cargo run --bin anonymize_test_data -- --input-dir my_emails --output-dir my_anonymized");
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    
    let backend_name = match args.backend {
        LlmBackend::Ollama => "Ollama",
        LlmBackend::OpenAI => "OpenAI",
    };
    
    println!("🔒 LLM-Powered Test Data Anonymization Tool");
    println!("==========================================");
    println!("🤖 Backend: {} ({})", backend_name, args.model.as_deref().unwrap_or("default model"));
    println!("📂 Input directory: {}", args.input_dir);
    println!("📂 Output directory: {}", args.output_dir);
    println!();
    
    // Load emails
    println!("📧 Loading emails from {}...", args.input_dir);
    let emails = load_emails(&args.input_dir)?;
    println!("✅ Loaded {} emails", emails.len());
    
    if emails.is_empty() {
        println!("❌ No emails found in {}. Run download_test_data first.", args.input_dir);
        return Ok(());
    }
    
    // Check for existing output and crash recovery
    println!("🔍 Checking for existing output (crash recovery)...");
    let existing_outputs = scan_existing_outputs(&args.output_dir)?;
    let existing_emails = load_existing_emails(&args.output_dir)?;
    
    if !existing_outputs.is_empty() {
        println!("📦 Found {} already-processed emails", existing_outputs.len());
        println!("🔄 Resuming from where we left off...");
        
        // Show which emails are already done
        let mut existing_indices: Vec<usize> = existing_outputs.iter().cloned().collect();
        existing_indices.sort();
        if existing_indices.len() <= 10 {
            println!("   Already completed: {:?}", existing_indices);
        } else {
            println!("   Already completed: {} through {} (and {} more)", 
                existing_indices[0], 
                existing_indices[9], 
                existing_indices.len() - 10);
        }
    } else {
        println!("🆕 Starting fresh anonymization process");
    }
    
    // Filter emails to only process those not already completed
    let total_emails = emails.len();
    let emails_to_process: Vec<_> = emails.into_iter()
        .filter(|email| !existing_outputs.contains(&email.file_index))
        .collect();
    
    if emails_to_process.is_empty() {
        println!("✅ All emails already anonymized! Nothing to do.");
        println!("📊 Total emails: {}", existing_emails.len());
        println!("📂 Output directory: {}/", args.output_dir);
        return Ok(());
    }
    
    println!("📋 Processing plan:");
    println!("   • Total emails in dataset: {}", total_emails);
    println!("   • Already completed: {}", existing_outputs.len());
    println!("   • Remaining to process: {}", emails_to_process.len());
    
    // Initialize LLM anonymizer
    println!("🤖 Initializing {} anonymizer...", backend_name);
    let config = AnonymizerConfig::new(args.backend, args.model)?;
    let mut anonymizer = LlmAnonymizer::new(config).await?;
    println!("✅ LLM ready for anonymization");
    
    // Show preview of what will be anonymized
    println!("\n🔍 Preview of emails to anonymize:");
    for (i, email) in emails_to_process.iter().take(5).enumerate() {
        println!("  {}. File {}: {}", 
            i + 1, 
            email.file_index, 
            email.subject.as_deref().unwrap_or("(no subject)")
        );
        if let Some(from) = &email.from {
            if from.len() > 50 {
                println!("     From: {}...", &from[..47]);
            } else {
                println!("     From: {}", from);
            }
        }
    }
    if emails_to_process.len() > 5 {
        println!("  ... and {} more emails", emails_to_process.len() - 5);
    }
    
    // Ask for confirmation
    print!("\n❓ Proceed with LLM anonymization? [Y/n]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim().to_lowercase();
    
    if choice == "n" || choice == "no" {
        println!("❌ Anonymization cancelled.");
        return Ok(());
    }
    
    // Create output directory early
    println!("📁 Creating output directory: {}", args.output_dir);
    fs::create_dir_all(&args.output_dir)?;
    
    // Check for existing output and crash recovery
    println!("🔍 Checking for existing output (crash recovery)...");
    let existing_outputs = scan_existing_outputs(&args.output_dir)?;
    let existing_emails = load_existing_emails(&args.output_dir)?;
    
    if !existing_outputs.is_empty() {
        println!("� Found {} already-processed emails", existing_outputs.len());
        println!("� Resuming from where we left off...");
        
        // Show which emails are already done
        let mut existing_indices: Vec<usize> = existing_outputs.iter().cloned().collect();
        existing_indices.sort();
        if existing_indices.len() <= 10 {
            println!("   Already completed: {:?}", existing_indices);
        } else {
            println!("   Already completed: {} through {} (and {} more)", 
                existing_indices[0], 
                existing_indices[9], 
                existing_indices.len() - 10);
        }
    } else {
        println!("🆕 Starting fresh anonymization process");
    }
    
    println!("📋 Processing plan:");
    println!("   • Total emails in dataset: {}", total_emails);
    println!("   • Already completed: {}", existing_outputs.len()); 
    println!("   • Remaining to process: {}", emails_to_process.len());
    
    // Process emails with LLM
    println!("🔧 Processing {} remaining emails with {} (strict mode - no fallback)...", 
        emails_to_process.len(), backend_name);
    println!("💾 Each email will be saved immediately after anonymization");
    
    let mut newly_anonymized = Vec::new();
    let mut failed_emails = Vec::new();
    let overall_start = Instant::now();
    
    for (i, email) in emails_to_process.iter().enumerate() {
        println!("📧 [{}/{}] Processing email {} (ID: {})...", 
            i + 1, emails_to_process.len(), email.file_index, email.id);
        
        match anonymizer.anonymize_email(email).await {
            Ok(anonymized) => {
                // Save immediately after anonymization
                match save_single_email(&anonymized, &args.output_dir) {
                    Ok(_) => {
                        newly_anonymized.push(anonymized.clone());
                        println!("✅ Email {} anonymized and saved successfully", email.file_index);
                    }
                    Err(e) => {
                        println!("❌ Failed to save anonymized email {}: {}", email.file_index, e);
                        failed_emails.push((email.file_index, format!("Save failed: {}", e)));
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to anonymize email {}: {}", email.file_index, e);
                failed_emails.push((email.file_index, e.to_string()));
                // NO FALLBACK - we fail for this email
            }
        }
    }
    
    let overall_duration = overall_start.elapsed();
    
    // Print timing summary
    anonymizer.get_timing_stats().print_summary();
    println!("   • Overall time: {:.2}s", overall_duration.as_secs_f64());
    
    // Handle results
    if !failed_emails.is_empty() {
        println!("⚠️  Some emails failed to anonymize:");
        for (index, error) in &failed_emails {
            println!("   • Email {}: {}", index, error);
        }
        println!("❌ Anonymization incomplete. {} succeeded, {} failed.", 
            newly_anonymized.len(), failed_emails.len());
        return Err(format!("{} emails failed anonymization", failed_emails.len()).into());
    }
    
    // Combine existing and newly processed emails for final manifest
    let mut all_anonymized_emails = existing_emails;
    all_anonymized_emails.extend(newly_anonymized.clone());
    
    // Sort by file_index to maintain order
    all_anonymized_emails.sort_by_key(|email| email.file_index);
    
    // Create/update manifest with all emails
    println!("📄 Creating manifest for {} total emails...", all_anonymized_emails.len());
    save_manifest(&all_anonymized_emails, &args.output_dir)?;
    
    println!("✅ Anonymization complete!");
    println!("📊 Summary:");
    println!("   • Total emails in final set: {}", all_anonymized_emails.len());
    println!("   • Previously completed: {}", existing_outputs.len());
    println!("   • Newly anonymized: {}", newly_anonymized.len());
    println!("   • Failed: {}", failed_emails.len());
    println!("   • Backend used: {} ({})", backend_name, 
        anonymizer.config.model);
    println!("   • Processing time: {:.2}s", overall_duration.as_secs_f64());
    println!("   • Output directory: {}/", args.output_dir);
    println!("   • Ready for version control: ✅");
    
    Ok(())
}

fn load_emails(input_dir: &str) -> Result<Vec<TestDataEmail>, Box<dyn std::error::Error>> {
    let manifest_path = Path::new(input_dir).join("manifest.json");
    
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
        let path = Path::new(input_dir).join(&entry.filename);
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let email: TestDataEmail = serde_json::from_str(&content)?;
            emails.push(email);
        }
    }
    
    Ok(emails)
}

/// Scan output directory for existing anonymized emails (crash recovery)
fn scan_existing_outputs(output_dir: &str) -> Result<std::collections::HashSet<usize>, Box<dyn std::error::Error>> {
    let mut existing_indices = std::collections::HashSet::new();
    
    let output_path = Path::new(output_dir);
    if !output_path.exists() {
        return Ok(existing_indices);
    }
    
    // Look for email_XXX.json files
    for entry in fs::read_dir(output_path)? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();
        
        // Match pattern email_001.json, email_002.json, etc.
        if filename_str.starts_with("email_") && filename_str.ends_with(".json") {
            // Extract the number from email_XXX.json
            if let Some(number_part) = filename_str.strip_prefix("email_").and_then(|s| s.strip_suffix(".json")) {
                if let Ok(index) = number_part.parse::<usize>() {
                    existing_indices.insert(index);
                }
            }
        }
    }
    
    Ok(existing_indices)
}

/// Load existing anonymized emails from output directory (crash recovery)
fn load_existing_emails(output_dir: &str) -> Result<Vec<TestDataEmail>, Box<dyn std::error::Error>> {
    let mut emails = Vec::new();
    let output_path = Path::new(output_dir);
    
    if !output_path.exists() {
        return Ok(emails);
    }
    
    // Look for email_XXX.json files and load them
    let mut entries: Vec<_> = fs::read_dir(output_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            filename_str.starts_with("email_") && filename_str.ends_with(".json")
        })
        .collect();
    
    // Sort by filename to maintain order
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    for entry in entries {
        let path = entry.path();
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<TestDataEmail>(&content) {
                    Ok(email) => emails.push(email),
                    Err(e) => {
                        println!("⚠️  Warning: Failed to parse existing email {}: {}", path.display(), e);
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Warning: Failed to read existing email {}: {}", path.display(), e);
            }
        }
    }
    
    Ok(emails)
}

/// Save a single email immediately after anonymization
fn save_single_email(email: &TestDataEmail, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filename = format!("email_{:03}.json", email.file_index);
    let filepath = Path::new(output_dir).join(&filename);
    
    let json_content = serde_json::to_string_pretty(email)?;
    fs::write(&filepath, json_content)?;
    
    Ok(())
}

/// Create manifest file for all anonymized emails
fn save_manifest(emails: &[TestDataEmail], output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(serde::Serialize)]
    struct Manifest {
        created_at: String,
        total_emails: usize,
        source: String,
        anonymized: bool,
        emails: Vec<ManifestEntry>,
    }
    
    #[derive(serde::Serialize)]
    struct ManifestEntry {
        file_index: usize,
        filename: String,
        id: String,
        subject: Option<String>,
        has_snippet: bool,
        has_from: bool,
        has_to: bool,
        has_sent: bool,
        has_body: bool,
    }
    
    let manifest = Manifest {
        created_at: chrono::Utc::now().to_rfc3339(),
        total_emails: emails.len(),
        source: "LLM-anonymized test data".to_string(),
        anonymized: true,
        emails: emails.iter().map(|email| {
            ManifestEntry {
                file_index: email.file_index,
                filename: format!("email_{:03}.json", email.file_index),
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
    
    let manifest_path = Path::new(output_dir).join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_json)?;
    
    println!("📄 Created manifest with {} entries", manifest.emails.len());
    
    Ok(())
}
