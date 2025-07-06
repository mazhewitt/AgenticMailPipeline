use agentic_mail_agent::classifier::{LangChainClassifier, LangChainConfig, MessageClassifier};
use agentic_mail_agent::core::email::Email;
use std::fs;
use std::path::Path;

/// Test data email structure matching the downloaded format
#[derive(serde::Deserialize, Debug, Clone)]
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

/// Load all test emails from manifest
fn load_all_test_emails() -> Result<Vec<TestDataEmail>, Box<dyn std::error::Error>> {
    let manifest_path = Path::new("test_data").join("manifest.json");

    if !manifest_path.exists() {
        return Err("Test data not found. Run 'cargo run --bin download_test_data' first.".into());
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
        let path = Path::new("test_data").join(&entry.filename);
        let content = fs::read_to_string(path)?;
        let test_email: TestDataEmail = serde_json::from_str(&content)?;
        emails.push(test_email);
    }

    Ok(emails)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires running ollama server"]
    async fn test_real_classifier_with_sample_emails() {
        println!("🤖 Testing Real LLM Classifier with Downloaded Gmail Data");
        println!("=======================================================");

        // Try to load test emails
        let test_emails = match load_all_test_emails() {
            Ok(emails) => emails,
            Err(e) => {
                println!("⚠️  Could not load test data: {e}");
                println!("💡 Run 'cargo run --bin download_test_data' to create test data");
                return;
            }
        };

        println!("📧 Loaded {} test emails", test_emails.len());

        // Initialize the LangChain classifier
        let config = LangChainConfig::default();
        let classifier = match LangChainClassifier::new(config).await {
            Ok(classifier) => {
                println!("✅ LangChain classifier initialized successfully");
                classifier
            }
            Err(e) => {
                println!("⚠️  Could not initialize LangChain classifier: {e}");
                println!("💡 Make sure Ollama is running with a compatible model:");
                println!("   ollama serve");
                println!("   ollama pull llama3:8b");
                return;
            }
        };

        // Test with a diverse selection of emails
        let sample_emails: Vec<Email> = test_emails
            .into_iter()
            .take(8) // Test with first 8 emails for reasonable test time
            .map(|te| te.to_email())
            .collect();

        println!("\n🧪 Classifying sample emails with real LLM:");
        println!("=========================================");

        let mut successful_classifications = 0;
        let mut total_classifications = 0;

        for (i, email) in sample_emails.iter().enumerate() {
            total_classifications += 1;
            let subject = email.subject_or_default();
            let from = email.from.as_deref().unwrap_or("Unknown sender");

            println!("\n📬 Email {} of {}:", i + 1, sample_emails.len());
            println!("   Subject: {subject}");
            println!("   From: {from}");
            println!(
                "   Snippet: {}",
                email
                    .snippet_or_default()
                    .chars()
                    .take(80)
                    .collect::<String>()
            );

            match classifier.classify(email).await {
                Ok(classification) => {
                    successful_classifications += 1;
                    println!("   🎯 Classification: {}", classification.category);
                    if let Some(score) = classification.score {
                        println!("   📊 Confidence: {score:.2}");
                    }
                    println!(
                        "   🤖 LLM Response: {}",
                        classification
                            .llm_response
                            .chars()
                            .take(100)
                            .collect::<String>()
                    );

                    // Basic validation - EmailCategory is always valid as an enum
                    assert!(
                        !classification.llm_response.is_empty(),
                        "LLM response should not be empty"
                    );

                    // Check if score is within valid range (if present)
                    if let Some(score) = classification.score {
                        assert!(
                            (0.0..=1.0).contains(&score),
                            "Score should be between 0 and 1"
                        );
                    }
                }
                Err(e) => {
                    println!("   ❌ Classification failed: {e}");
                }
            }
        }

        println!("\n📊 Classification Results Summary:");
        println!("===================================");
        println!("   • Total emails tested: {total_classifications}");
        println!("   • Successful classifications: {successful_classifications}");
        println!(
            "   • Success rate: {:.1}%",
            (successful_classifications as f64 / total_classifications as f64) * 100.0
        );

        // Assert that we achieved reasonable success rate
        let success_rate = successful_classifications as f64 / total_classifications as f64;
        assert!(
            success_rate >= 0.5,
            "Should achieve at least 50% classification success rate"
        );

        if success_rate >= 0.8 {
            println!("✅ Excellent classification performance!");
        } else if success_rate >= 0.6 {
            println!("✅ Good classification performance!");
        } else {
            println!("⚠️  Classification performance could be improved");
        }
    }

    #[tokio::test]
    #[ignore = "requires running ollama server"]
    async fn test_classifier_category_distribution() {
        println!("📊 Testing Classification Category Distribution");
        println!("==============================================");

        // Initialize classifier
        let config = LangChainConfig::default();
        let classifier = match LangChainClassifier::new(config).await {
            Ok(classifier) => classifier,
            Err(e) => {
                println!("⚠️  Could not initialize classifier: {e}");
                return;
            }
        };

        // Use diverse synthetic test emails to ensure variety
        let test_emails = vec![
            Email::new(
                "test1".to_string(),
                Some("[GitHub] CI failed: main branch".to_string()),
                Some("Your CI pipeline has failed. Please check the logs and fix the issues.".to_string()),
            ),
            Email::new(
                "test2".to_string(),
                Some("Weekly Tech Newsletter - AI Updates".to_string()),
                Some("This week in technology: Latest AI developments and industry news.".to_string()),
            ),
            Email::new(
                "test3".to_string(),
                Some("Your Amazon Receipt #12345".to_string()),
                Some("Thank you for your purchase. Your order has been processed successfully.".to_string()),
            ),
            Email::new(
                "test4".to_string(),
                Some("URGENT: Action Required - Security Update".to_string()),
                Some("Please update your password immediately. This is required for security compliance.".to_string()),
            ),
            Email::new(
                "test5".to_string(),
                Some("Special Offer: 50% Off Everything!".to_string()),
                Some("Limited time deal! Get 50% off all items. Don't miss out on this exclusive offer!".to_string()),
            ),
            Email::new(
                "test6".to_string(),
                Some("Subscription Receipt - Netflix".to_string()),
                Some("Your monthly Netflix subscription has been charged to your account.".to_string()),
            ),
            Email::new(
                "test7".to_string(),
                Some("Meeting Reminder: Team Standup".to_string()),
                Some("Reminder: Our weekly team standup is scheduled for tomorrow at 9 AM.".to_string()),
            ),
            Email::new(
                "test8".to_string(),
                Some("Unsubscribe from our newsletter".to_string()),
                Some("We noticed you want to unsubscribe. Click here to manage your preferences.".to_string()),
            ),
        ];

        let mut category_counts = std::collections::HashMap::new();
        let mut classification_details = Vec::new();

        println!("🔍 Analyzing category distribution:");

        for email in &test_emails {
            match classifier.classify(email).await {
                Ok(classification) => {
                    *category_counts.entry(classification.category).or_insert(0) += 1;
                    classification_details.push((
                        email.subject_or_default(),
                        classification.category,
                        classification.score,
                    ));
                }
                Err(e) => {
                    println!(
                        "   ❌ Failed to classify '{}': {}",
                        email.subject_or_default(),
                        e
                    );
                }
            }
        }

        println!("\n📂 Category Distribution:");
        for (category, count) in &category_counts {
            let percentage = (*count as f64 / test_emails.len() as f64) * 100.0;
            println!("   • {category}: {count} emails ({percentage:.1}%)");
        }

        println!("\n📋 Detailed Classifications:");
        for (subject, category, score) in &classification_details {
            let score_str = match score {
                Some(s) => format!(" (confidence: {s:.2})"),
                None => String::new(),
            };
            println!("   • '{subject}' → {category}{score_str}");
        }

        // Assert we have reasonable category diversity with more diverse test data
        assert!(
            category_counts.len() >= 2,
            "Should classify emails into at least 2 different categories. Got: {:?}",
            category_counts
        );

        // Check that no single category dominates too much (unless we have very specialized emails)
        let max_category_percentage = category_counts
            .values()
            .map(|&count| (count as f64 / test_emails.len() as f64) * 100.0)
            .fold(0.0, f64::max);

        if max_category_percentage <= 70.0 {
            println!("✅ Good category distribution - no single category dominates");
        } else {
            println!("⚠️  Single category dominates ({max_category_percentage:.1}%) - may indicate bias or specialized data");
        }
    }

    #[tokio::test]
    #[ignore = "requires running ollama server"]
    async fn test_classifier_consistency() {
        println!("🔄 Testing Classification Consistency");
        println!("====================================");

        // Load test emails
        let test_emails = match load_all_test_emails() {
            Ok(emails) => emails,
            Err(e) => {
                println!("⚠️  Could not load test data: {e}");
                return;
            }
        };

        // Initialize classifier
        let config = LangChainConfig::default();
        let classifier = match LangChainClassifier::new(config).await {
            Ok(classifier) => classifier,
            Err(e) => {
                println!("⚠️  Could not initialize classifier: {e}");
                return;
            }
        };

        // Take first email and classify it multiple times
        if let Some(test_email) = test_emails.first() {
            let email = test_email.to_email();
            let subject = email.subject_or_default();

            println!("🧪 Testing consistency with email: '{subject}'");

            let mut classifications = Vec::new();
            const NUM_RUNS: usize = 3; // Test consistency with 3 runs

            for i in 1..=NUM_RUNS {
                println!("   Run {i}/{NUM_RUNS}: Classifying...");
                match classifier.classify(&email).await {
                    Ok(classification) => {
                        println!("     → Category: {}", classification.category);
                        classifications.push(classification);
                    }
                    Err(e) => {
                        println!("     ❌ Failed: {e}");
                    }
                }
            }

            if classifications.len() >= 2 {
                // Check consistency
                let first_category = &classifications[0].category;
                let consistent = classifications
                    .iter()
                    .all(|c| &c.category == first_category);

                if consistent {
                    println!("✅ Classifications are consistent across runs");
                } else {
                    println!("⚠️  Classifications vary across runs:");
                    for (i, classification) in classifications.iter().enumerate() {
                        println!("     Run {}: {}", i + 1, classification.category);
                    }
                    println!("💡 This is normal for LLM-based classification due to randomness");
                }
            }
        }
    }
}
