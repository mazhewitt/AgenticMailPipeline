//! Full inbox processor - processes ALL emails in Gmail inbox
//!
//! This script will:
//! 1. Fetch ALL emails currently in the Gmail inbox (read and unread)
//! 2. Process them through the classification and action pipeline
//! 3. Apply appropriate labels and archive/keep decisions
//! 4. Provide detailed progress reporting
//!
//! Usage: cargo run --bin process_full_inbox [--dry-run] [--batch-size N]

use std::process;
use std::time::Instant;

use agentic_mail_agent::action::executor::{ActionExecutor, GmailActionExecutor};
use agentic_mail_agent::action::impls::labeler::GmailLabeler;
use agentic_mail_agent::action::impls::archiver::GmailArchiver;
use agentic_mail_agent::classifier::{HybridClassifier, LangChainClassifier, MessageClassifier};
use agentic_mail_agent::core::email::Email;
use agentic_mail_agent::fetcher::{EmailFetcher, GmailFetcher};

#[tokio::main]
async fn main() {
    // Initialize crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());
    let batch_size = get_batch_size(&args);
    
    println!("🤖 Agentic Gmail Full Inbox Processor");
    println!("====================================");
    println!("📋 Configuration:");
    println!("  • Mode: {}", if dry_run { "DRY RUN (no changes)" } else { "LIVE (will modify emails)" });
    println!("  • Batch size: {} emails", batch_size);
    println!("  • Target: ALL emails in inbox (read and unread)");
    
    if !dry_run {
        println!("\n⚠️  WARNING: This will process ALL emails in your inbox!");
        println!("   - Emails will be labeled with Agentic/* labels");
        println!("   - Some emails will be archived (removed from inbox)");
        println!("   - ActionRequired emails will stay in inbox");
        println!("   Press Ctrl+C to cancel, or Enter to continue...");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read input");
    }
    
    let start_time = Instant::now();
    
    // Initialize Gmail fetcher
    println!("\n🔧 Initializing Gmail connection...");
    let fetcher = match GmailFetcher::from_env().await {
        Ok(fetcher) => fetcher,
        Err(e) => {
            eprintln!("❌ Failed to initialize Gmail fetcher: {}", e);
            eprintln!("   Make sure you have run ./setup_gmail_auth.sh");
            process::exit(1);
        }
    };
    
    // Fetch ALL inbox emails (use large number to get all)
    println!("📬 Fetching ALL emails from inbox...");
    let max_emails = if args.contains(&"--preview".to_string()) { 20 } else { 1000 };
    let emails = match fetcher.fetch_inbox_emails(max_emails).await {
        Ok(emails) => emails,
        Err(e) => {
            eprintln!("❌ Failed to fetch emails: {}", e);
            process::exit(1);
        }
    };
    
    let total_emails = emails.len();
    println!("📊 Found {} emails in inbox", total_emails);
    
    if total_emails == 0 {
        println!("✅ No emails to process. Inbox is already clean!");
        return;
    }
    
    // Initialize classifier
    println!("🤖 Initializing hybrid classifier with LLM support...");
    let llm_classifier = Box::new(LangChainClassifier::with_default_config().await.unwrap());
    let classifier = HybridClassifier::new_with_llm(llm_classifier).await;
    
    // Initialize action executor
    println!("⚡ Initializing action executor...");
    let labeler = match GmailLabeler::from_env().await {
        Ok(labeler) => labeler,
        Err(e) => {
            eprintln!("❌ Failed to initialize Gmail labeler: {}", e);
            process::exit(1);
        }
    };
    
    let archiver = match GmailArchiver::from_env().await {
        Ok(archiver) => archiver,
        Err(e) => {
            eprintln!("❌ Failed to initialize Gmail archiver: {}", e);
            process::exit(1);
        }
    };
    
    let action_executor = GmailActionExecutor::new(labeler, archiver);
    
    // Process emails in batches
    println!("\n🔄 Starting full inbox processing...");
    println!("📈 Progress will be reported every {} emails", batch_size);
    
    let mut processed = 0;
    let mut kept_in_inbox = 0;
    let mut archived = 0;
    let mut urgent_count = 0;
    let mut errors = 0;
    
    let mut classification_stats = std::collections::HashMap::new();
    
    for (i, email) in emails.iter().enumerate() {
        let email_num = i + 1;
        
        // Progress reporting
        if email_num % batch_size == 0 || email_num == total_emails {
            println!("\n📊 Progress: {}/{} emails processed ({:.1}%)", 
                email_num, total_emails, (email_num as f64 / total_emails as f64) * 100.0);
        }
        
        // Process individual email
        match process_single_email(&email, &classifier, &action_executor, dry_run, email_num, total_emails).await {
            Ok(result) => {
                processed += 1;
                
                // Update statistics
                if result.archived {
                    archived += 1;
                } else {
                    kept_in_inbox += 1;
                }
                
                if result.urgent {
                    urgent_count += 1;
                }
                
                // Track classification categories
                *classification_stats.entry(result.category.clone()).or_insert(0) += 1;
            }
            Err(e) => {
                eprintln!("⚠️  Error processing email {}: {}", email_num, e);
                errors += 1;
            }
        }
        
        // Small delay to be respectful to Gmail API
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    
    let elapsed = start_time.elapsed();
    
    // Final summary
    println!("\n🎉 Full Inbox Processing Complete!");
    println!("==================================");
    println!("⏱️  Total time: {:.2} seconds", elapsed.as_secs_f64());
    println!("📧 Total emails processed: {}", processed);
    println!("✅ Successful: {}", processed);
    println!("❌ Errors: {}", errors);
    println!();
    println!("📊 Email Disposition:");
    println!("  📥 Kept in inbox: {}", kept_in_inbox);
    println!("  📦 Archived: {}", archived);
    println!("  🚨 Urgent emails: {}", urgent_count);
    println!();
    println!("📋 Classification Breakdown:");
    for (category, count) in classification_stats.iter() {
        let percentage = (*count as f64 / processed as f64) * 100.0;
        println!("  • {}: {} ({:.1}%)", category, count, percentage);
    }
    
    if !dry_run {
        println!("\n✨ Your inbox has been intelligently organized!");
        println!("   - Check the 'Agentic' label group in Gmail");
        println!("   - ActionRequired emails remain in your inbox");
        println!("   - Other emails have been appropriately archived");
    } else {
        println!("\n💡 DRY RUN complete - no changes were made");
        println!("   Remove --dry-run flag to apply these changes");
    }
}

/// Process a single email through the classification and action pipeline
async fn process_single_email(
    email: &Email,
    classifier: &HybridClassifier,
    action_executor: &impl ActionExecutor,
    dry_run: bool,
    email_num: usize,
    total_emails: usize,
) -> Result<ProcessingResult, Box<dyn std::error::Error>> {
    // Classify the email
    let classification = classifier.classify(email).await?;
    
    // Execute actions (unless dry run)
    let result = if dry_run {
        // In dry run, simulate the actions
        simulate_actions(email, &classification)
    } else {
        let action_result = action_executor.execute_actions(email, &classification).await?;
        ProcessingResult {
            category: classification.category.to_string(),
            archived: action_result.archived,
            urgent: action_result.actions_taken.iter()
                .any(|action| action.contains("URGENT")),
        }
    };
    
    // Log progress for every 10th email or important emails
    if email_num % 10 == 0 || result.urgent {
        let subject = email.subject.as_deref().unwrap_or("(No Subject)");
        let status = if result.archived { "📦 ARCHIVED" } else { "📥 INBOX" };
        let urgent_flag = if result.urgent { "🚨" } else { "" };
        
        println!("  {}/{}: {} {} - {} {}", 
            email_num, total_emails, urgent_flag, status, result.category, 
            truncate_subject(subject, 50));
    }
    
    Ok(result)
}

/// Simulate actions for dry run mode
fn simulate_actions(email: &Email, classification: &agentic_mail_agent::classifier::Classification) -> ProcessingResult {
    use agentic_mail_agent::classifier::EmailCategory;
    
    let archived = match classification.category {
        EmailCategory::ActionRequired => false,
        _ => true,
    };
    
    let urgent = email.subject.as_deref()
        .map(|s| s.to_lowercase().contains("urgent"))
        .unwrap_or(false);
    
    ProcessingResult {
        category: classification.category.to_string(),
        archived,
        urgent,
    }
}

/// Get batch size from command line arguments
fn get_batch_size(args: &[String]) -> usize {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--batch-size" && i + 1 < args.len() {
            return args[i + 1].parse().unwrap_or(25);
        }
    }
    25 // Default batch size
}

/// Truncate subject line for display
fn truncate_subject(subject: &str, max_len: usize) -> String {
    if subject.len() <= max_len {
        subject.to_string()
    } else {
        format!("{}...", &subject[..max_len-3])
    }
}

/// Result of processing a single email
struct ProcessingResult {
    category: String,
    archived: bool,
    urgent: bool,
}