use agentic_mail_agent::action::{ActionExecutor, StubActionExecutor};
use agentic_mail_agent::classifier::{
    EmailCategory, LangChainClassifier, MessageClassifier, StubClassifier,
};
use agentic_mail_agent::core::email::Email;
use agentic_mail_agent::fetcher::{EmailFetcher, GmailFetcher, StubFetcher};
use std::collections::HashMap;

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
static ENV_TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Configuration for the main email processing binary
#[derive(Debug, Clone)]
struct ProcessingConfig {
    /// Maximum number of emails to process from inbox (configurable depth)
    max_emails: u32,
    /// Confidence threshold below which emails need review
    review_threshold: f32,
    /// Whether to use demo mode (stub data)
    demo_mode: bool,
    /// Type of classifier to use ("stub", "langchain", "hybrid")
    classifier_type: String,
    /// Whether to process in dry-run mode (no actual changes)
    dry_run: bool,
}

impl ProcessingConfig {
    /// Load configuration from environment variables and defaults
    fn from_env() -> Self {
        Self {
            max_emails: std::env::var("MAX_EMAILS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
            review_threshold: std::env::var("REVIEW_THRESHOLD")
                .unwrap_or_else(|_| "0.7".to_string())
                .parse()
                .unwrap_or(0.7),
            demo_mode: std::env::var("DEMO_MODE").is_ok(),
            classifier_type: std::env::var("CLASSIFIER_TYPE")
                .unwrap_or_else(|_| "stub".to_string()),
            dry_run: std::env::var("DRY_RUN").is_ok(),
        }
    }
}

/// Statistics for email processing
#[derive(Debug, Default)]
struct ProcessingStats {
    total_processed: usize,
    kept_in_inbox: usize,
    archived: usize,
    classification_counts: HashMap<String, usize>,
    urgent_count: usize,
    needs_review_count: usize,
    errors: usize,
}

impl ProcessingStats {
    fn print_summary(&self) {
        println!("\n📊 Processing Summary");
        println!("====================");
        println!("📧 Total emails processed: {}", self.total_processed);
        println!("�� Kept in inbox: {}", self.kept_in_inbox);
        println!("📦 Archived: {}", self.archived);
        println!("🚨 Urgent emails: {}", self.urgent_count);
        println!("🔍 Needs review: {}", self.needs_review_count);
        if self.errors > 0 {
            println!("❌ Errors encountered: {}", self.errors);
        }

        println!("\n📋 Classification Breakdown:");
        for (category, count) in &self.classification_counts {
            println!("  • {category}: {count}");
        }
    }
}

/// Check if an email is urgent based on content analysis
fn is_urgent_email(email: &Email) -> bool {
    let urgent_keywords = [
        "urgent",
        "asap",
        "emergency",
        "critical",
        "immediate",
        "deadline",
        "time sensitive",
        "action required",
        "important",
        "breaking",
        "urgent:",
        "emergency:",
        "critical:",
        "asap:",
    ];

    let text_to_check = format!(
        "{} {} {}",
        email.subject_or_default().to_lowercase(),
        email.snippet_or_default().to_lowercase(),
        email.body_or_default().to_lowercase()
    );

    urgent_keywords
        .iter()
        .any(|&keyword| text_to_check.contains(keyword))
}

/// Determine if an email should stay in inbox based on classification and heuristics
fn should_stay_in_inbox(
    email: &Email,
    classification: &agentic_mail_agent::classifier::Classification,
    config: &ProcessingConfig,
) -> (bool, String) {
    // Always keep ActionRequired emails in inbox
    if classification.category == EmailCategory::ActionRequired {
        return (true, "Action required".to_string());
    }

    // Keep urgent emails in inbox regardless of classification
    if is_urgent_email(email) {
        return (true, "Urgent content detected".to_string());
    }

    // Keep low-confidence classifications for review
    if let Some(score) = classification.score {
        if score < config.review_threshold {
            return (true, format!("Low confidence ({score:.2}) - needs review"));
        }
    }

    // Archive everything else
    (false, "Archived based on classification".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Load configuration
    let config = ProcessingConfig::from_env();

    println!("🤖 Agentic Gmail Agent - Inbox Processor");
    println!("=========================================");
    println!("📋 Configuration:");
    println!("  • Max emails to process: {}", config.max_emails);
    println!("  • Review threshold: {:.2}", config.review_threshold);
    println!("  • Classifier type: {}", config.classifier_type);
    println!("  • Demo mode: {}", config.demo_mode);
    println!("  • Dry run: {}", config.dry_run);
    println!();

    // Initialize fetcher
    let fetcher: Box<dyn EmailFetcher> = if config.demo_mode
        || GmailFetcher::from_env().await.is_err()
    {
        println!("📧 Using demo data (Gmail credentials not found or DEMO_MODE set)");
        let demo_emails = vec![
            Email::new(
                "demo-1".to_string(),
                Some("Welcome to Agentic Mail Agent".to_string()),
                Some("This is a demo email showing the complete processing pipeline.".to_string())
            ),
            Email::new(
                "demo-2".to_string(), 
                Some("URGENT: Meeting Reminder".to_string()),
                Some("Don't forget about the URGENT team meeting tomorrow at 2 PM. Action required!".to_string())
            ),
            Email::new(
                "demo-3".to_string(),
                Some("Weekly Newsletter".to_string()),
                Some("Check out this week's updates from our team. Newsletter content with promotions.".to_string())
            ),
            Email::new(
                "demo-4".to_string(),
                Some("Suspicious offer - You won $1,000,000!".to_string()),
                Some("Click here to claim your prize! Limited time offer. Send us your bank details now.".to_string())
            ),
            Email::new(
                "demo-5".to_string(),
                Some("Low confidence example".to_string()),
                Some("This email will likely get a low classification confidence score.".to_string())
            ),
        ];
        Box::new(StubFetcher::with_emails(demo_emails))
    } else {
        println!("📧 Using Gmail API fetcher");
        Box::new(GmailFetcher::from_env().await?)
    };

    // Fetch emails from inbox with configurable depth
    let emails = if config.demo_mode {
        fetcher.fetch_unread_emails().await?
    } else {
        fetcher.fetch_inbox_emails(config.max_emails).await?
    };

    if emails.is_empty() {
        println!("📭 No emails found to process.");
        return Ok(());
    }

    println!("📬 Fetched {} emails from inbox", emails.len());

    // Initialize classifier
    let classifier: Box<dyn MessageClassifier> = match config.classifier_type.as_str() {
        "langchain" | "llm" => {
            println!("🤖 Initializing LangChain LLM classifier...");
            match LangChainClassifier::with_default_config().await {
                Ok(llm_classifier) => {
                    println!("✅ LangChain classifier initialized successfully");
                    Box::new(llm_classifier)
                }
                Err(e) => {
                    eprintln!("❌ Failed to initialize LangChain classifier: {e}");
                    eprintln!("🔄 Falling back to stub classifier...");
                    Box::new(StubClassifier::deterministic())
                }
            }
        }
        "stub" => {
            println!("🎯 Using deterministic stub classifier");
            Box::new(StubClassifier::deterministic())
        }
        _ => {
            println!("🎯 Unknown classifier type, using deterministic stub classifier");
            Box::new(StubClassifier::deterministic())
        }
    };

    // Initialize action executor
    let action_executor: Box<dyn ActionExecutor> = if config.demo_mode {
        println!("🎯 Using stub action executor (demo mode)");
        Box::new(StubActionExecutor::new())
    } else {
        println!("🎯 Using stub action executor");
        Box::new(StubActionExecutor::new())
    };

    if config.dry_run {
        println!("🏃 DRY RUN MODE - No actual changes will be made");
    }

    println!("\n🔄 Starting email processing pipeline...\n");

    // Process emails and collect statistics
    let mut stats = ProcessingStats::default();

    for (index, email) in emails.iter().enumerate() {
        println!(
            "📧 Processing email {} of {}: {}",
            index + 1,
            emails.len(),
            email.id
        );

        if let Some(subject) = &email.subject {
            println!("  📋 Subject: {subject}");
        }
        if let Some(snippet) = &email.snippet {
            println!(
                "  👁️  Preview: {}",
                snippet.chars().take(80).collect::<String>()
            );
        }

        // Classify the email
        match classifier.classify(email).await {
            Ok(classification) => {
                stats.total_processed += 1;
                *stats
                    .classification_counts
                    .entry(classification.category.to_string())
                    .or_insert(0) += 1;

                let score_display = classification
                    .score
                    .map(|s| format!("{s:.2}"))
                    .unwrap_or_else(|| "N/A".to_string());

                println!(
                    "  🎯 Classification: {} (confidence: {})",
                    classification.category, score_display
                );

                if !classification.llm_response.is_empty() {
                    println!("  🤖 Analysis: {}", classification.llm_response);
                }

                // Check if this email is urgent
                let is_urgent = is_urgent_email(email);
                if is_urgent {
                    stats.urgent_count += 1;
                    println!("  �� URGENT: Email marked as urgent");
                }

                // Determine if this email should stay in inbox
                let (stay_in_inbox, reason) = should_stay_in_inbox(email, &classification, &config);

                if stay_in_inbox {
                    stats.kept_in_inbox += 1;
                    if classification
                        .score
                        .is_some_and(|s| s < config.review_threshold)
                    {
                        stats.needs_review_count += 1;
                    }
                    println!(
                        "  📥 INBOX: {} - {}",
                        if is_urgent { "🚨 URGENT" } else { "📝" },
                        reason
                    );
                } else {
                    stats.archived += 1;
                    println!("  📦 ARCHIVE: {reason}");
                }

                // Execute actions if not in dry-run mode
                if !config.dry_run {
                    match action_executor
                        .execute_actions(email, &classification)
                        .await
                    {
                        Ok(result) => {
                            println!("  ✅ Actions completed:");
                            println!("    • Label: {}", result.label_applied);
                            for action in &result.actions_taken {
                                println!("    • {action}");
                            }
                        }
                        Err(e) => {
                            stats.errors += 1;
                            println!("  ❌ Action execution failed: {e}");
                        }
                    }
                } else {
                    println!(
                        "  🏃 Dry run: Would apply label and {}",
                        if stay_in_inbox {
                            "keep in inbox"
                        } else {
                            "archive"
                        }
                    );
                }
            }
            Err(e) => {
                stats.errors += 1;
                println!("  ❌ Classification failed: {e}");
            }
        }

        println!(); // Add spacing between emails
    }

    // Print final summary
    stats.print_summary();

    if config.dry_run {
        println!("\n🏃 This was a dry run - no actual changes were made to your Gmail account.");
    }

    if stats.needs_review_count > 0 {
        println!(
            "\n🔍 {} emails were kept in inbox for manual review due to low confidence scores.",
            stats.needs_review_count
        );
        println!("   Consider reviewing these emails manually or adjusting the review threshold.");
    }

    if stats.urgent_count > 0 {
        println!(
            "\n🚨 {} urgent emails were detected and kept in inbox.",
            stats.urgent_count
        );
    }

    println!("\n✅ Email processing completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentic_mail_agent::classifier::Classification;
    use agentic_mail_agent::core::email::Email;

    #[test]
    fn test_processing_config_from_env() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap();

        // Save original values
        let orig_max = std::env::var("MAX_EMAILS").ok();
        let orig_threshold = std::env::var("REVIEW_THRESHOLD").ok();
        let orig_demo = std::env::var("DEMO_MODE").ok();
        let orig_classifier = std::env::var("CLASSIFIER_TYPE").ok();
        let orig_dry_run = std::env::var("DRY_RUN").ok();

        // Remove all variables to test defaults
        std::env::remove_var("MAX_EMAILS");
        std::env::remove_var("REVIEW_THRESHOLD");
        std::env::remove_var("DEMO_MODE");
        std::env::remove_var("CLASSIFIER_TYPE");
        std::env::remove_var("DRY_RUN");

        let config = ProcessingConfig::from_env();
        assert_eq!(config.max_emails, 50);
        assert_eq!(config.review_threshold, 0.7);
        assert!(!config.demo_mode);
        assert_eq!(config.classifier_type, "stub");
        assert!(!config.dry_run);

        // Restore original values
        match orig_max {
            Some(val) => std::env::set_var("MAX_EMAILS", val),
            None => std::env::remove_var("MAX_EMAILS"),
        }
        match orig_threshold {
            Some(val) => std::env::set_var("REVIEW_THRESHOLD", val),
            None => std::env::remove_var("REVIEW_THRESHOLD"),
        }
        match orig_demo {
            Some(val) => std::env::set_var("DEMO_MODE", val),
            None => std::env::remove_var("DEMO_MODE"),
        }
        match orig_classifier {
            Some(val) => std::env::set_var("CLASSIFIER_TYPE", val),
            None => std::env::remove_var("CLASSIFIER_TYPE"),
        }
        match orig_dry_run {
            Some(val) => std::env::set_var("DRY_RUN", val),
            None => std::env::remove_var("DRY_RUN"),
        }
    }

    #[test]
    fn test_processing_config_custom_values() {
        let _lock = ENV_TEST_MUTEX.lock().unwrap();

        // Save original values
        let orig_max = std::env::var("MAX_EMAILS").ok();
        let orig_threshold = std::env::var("REVIEW_THRESHOLD").ok();
        let orig_demo = std::env::var("DEMO_MODE").ok();
        let orig_classifier = std::env::var("CLASSIFIER_TYPE").ok();
        let orig_dry_run = std::env::var("DRY_RUN").ok();

        // Set test values
        std::env::set_var("MAX_EMAILS", "100");
        std::env::set_var("REVIEW_THRESHOLD", "0.8");
        std::env::set_var("DEMO_MODE", "1");
        std::env::set_var("CLASSIFIER_TYPE", "langchain");
        std::env::set_var("DRY_RUN", "1");

        let config = ProcessingConfig::from_env();
        assert_eq!(config.max_emails, 100);
        assert_eq!(config.review_threshold, 0.8);
        assert!(config.demo_mode);
        assert_eq!(config.classifier_type, "langchain");
        assert!(config.dry_run);

        // Restore original values
        match orig_max {
            Some(val) => std::env::set_var("MAX_EMAILS", val),
            None => std::env::remove_var("MAX_EMAILS"),
        }
        match orig_threshold {
            Some(val) => std::env::set_var("REVIEW_THRESHOLD", val),
            None => std::env::remove_var("REVIEW_THRESHOLD"),
        }
        match orig_demo {
            Some(val) => std::env::set_var("DEMO_MODE", val),
            None => std::env::remove_var("DEMO_MODE"),
        }
        match orig_classifier {
            Some(val) => std::env::set_var("CLASSIFIER_TYPE", val),
            None => std::env::remove_var("CLASSIFIER_TYPE"),
        }
        match orig_dry_run {
            Some(val) => std::env::set_var("DRY_RUN", val),
            None => std::env::remove_var("DRY_RUN"),
        }
    }

    #[test]
    fn test_is_urgent_email() {
        let urgent_email = Email::new(
            "test1".to_string(),
            Some("URGENT: System Down".to_string()),
            Some("Production system is down. Immediate action required!".to_string()),
        );
        assert!(is_urgent_email(&urgent_email));

        let normal_email = Email::new(
            "test2".to_string(),
            Some("Weekly Newsletter".to_string()),
            Some("Here's your weekly newsletter with updates".to_string()),
        );
        assert!(!is_urgent_email(&normal_email));

        let emergency_email = Email::new(
            "test3".to_string(),
            Some("Emergency: Server Issue".to_string()),
            Some("Emergency situation detected".to_string()),
        );
        assert!(is_urgent_email(&emergency_email));
    }

    #[test]
    fn test_should_stay_in_inbox() {
        let config = ProcessingConfig {
            max_emails: 50,
            review_threshold: 0.7,
            demo_mode: false,
            classifier_type: "stub".to_string(),
            dry_run: false,
        };

        // Test ActionRequired always stays
        let email = Email::new("test1".to_string(), Some("Test".to_string()), None);
        let classification = Classification::with_category(EmailCategory::ActionRequired);
        let (stay, reason) = should_stay_in_inbox(&email, &classification, &config);
        assert!(stay);
        assert!(reason.contains("Action required"));

        // Test urgent email stays regardless of classification
        let urgent_email = Email::new(
            "test2".to_string(),
            Some("URGENT: Important".to_string()),
            None,
        );
        let noise_classification = Classification::with_category(EmailCategory::Noise);
        let (stay, reason) = should_stay_in_inbox(&urgent_email, &noise_classification, &config);
        assert!(stay);
        assert!(reason.contains("Urgent"));

        // Test low confidence stays for review
        let normal_email = Email::new("test3".to_string(), Some("Normal".to_string()), None);
        let low_conf_classification = Classification::new(
            EmailCategory::Reference,
            Some(0.5), // Below 0.7 threshold
            "Low confidence".to_string(),
        );
        let (stay, reason) = should_stay_in_inbox(&normal_email, &low_conf_classification, &config);
        assert!(stay);
        assert!(reason.contains("Low confidence"));

        // Test high confidence non-urgent gets archived
        let high_conf_classification = Classification::new(
            EmailCategory::Noise,
            Some(0.9), // Above threshold
            "High confidence".to_string(),
        );
        let (stay, reason) =
            should_stay_in_inbox(&normal_email, &high_conf_classification, &config);
        assert!(!stay);
        assert!(reason.contains("Archived"));
    }

    #[test]
    fn test_processing_stats() {
        let mut stats = ProcessingStats::default();

        // Test defaults
        assert_eq!(stats.total_processed, 0);
        assert_eq!(stats.kept_in_inbox, 0);
        assert_eq!(stats.archived, 0);
        assert_eq!(stats.urgent_count, 0);
        assert_eq!(stats.needs_review_count, 0);
        assert_eq!(stats.errors, 0);

        // Test incrementing
        stats.total_processed += 1;
        stats.kept_in_inbox += 1;
        *stats
            .classification_counts
            .entry(EmailCategory::ActionRequired.to_string())
            .or_insert(0) += 1;

        assert_eq!(stats.total_processed, 1);
        assert_eq!(stats.kept_in_inbox, 1);
        assert_eq!(
            stats
                .classification_counts
                .get(&EmailCategory::ActionRequired.to_string()),
            Some(&1)
        );
    }
}
