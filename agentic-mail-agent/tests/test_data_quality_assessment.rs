use agen#[derive(Clone, Debug, serde::Deserialize)]
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
}nt::classifier::{StubClassifier, MessageClassifier};
use agentic_mail_agent::email::Email;
use serde_json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Test data email structure matching the downloaded format
#[derive(serde::Deserialize, Debug, Clone)]
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

/// Analyze sender domains to understand email source diversity
fn analyze_sender_domains(emails: &[TestDataEmail]) -> HashMap<String, Vec<String>> {
    let mut domain_categories = HashMap::new();
    
    for email in emails {
        if let Some(from) = &email.from {
            let domain = extract_domain(from);
            let subject = email.subject.as_deref().unwrap_or("No subject");
            
            domain_categories
                .entry(domain.clone())
                .or_insert_with(Vec::new)
                .push(subject.to_string());
        }
    }
    
    domain_categories
}

/// Extract domain from email address
fn extract_domain(email_addr: &str) -> String {
    if let Some(at_pos) = email_addr.rfind('@') {
        let domain_part = &email_addr[at_pos + 1..];
        // Remove > if present
        domain_part.trim_end_matches('>').to_string()
    } else {
        email_addr.to_string()
    }
}

/// Classify emails into expected categories based on content analysis
fn categorize_emails_by_content(emails: &[TestDataEmail]) -> HashMap<String, Vec<String>> {
    let mut categories = HashMap::new();
    
    for email in emails {
        let subject = email.subject.as_deref().unwrap_or("");
        let from = email.from.as_deref().unwrap_or("");
        let body = email.body.as_deref().unwrap_or("");
        
        let category = classify_email_content(subject, from, body);
        categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(format!("{}: {}", email.id, subject));
    }
    
    categories
}

/// Basic content-based classification
fn classify_email_content(subject: &str, from: &str, body: &str) -> String {
    let subject_lower = subject.to_lowercase();
    let from_lower = from.to_lowercase();
    let _body_lower = body.to_lowercase();
    
    // GitHub notifications
    if from_lower.contains("github.com") || subject_lower.contains("run failed") {
        return "dev_notifications".to_string();
    }
    
    // LinkedIn
    if from_lower.contains("linkedin.com") {
        return "social_professional".to_string();
    }
    
    // Facebook
    if from_lower.contains("facebook") {
        return "social_personal".to_string();
    }
    
    // Security alerts
    if subject_lower.contains("security alert") || from_lower.contains("no-reply@accounts.google.com") {
        return "security".to_string();
    }
    
    // Newsletters
    if from_lower.contains("flipboard") || from_lower.contains("nytimes") || from_lower.contains("uefa") {
        return "newsletter".to_string();
    }
    
    // Entertainment/Media
    if from_lower.contains("spotify") {
        return "entertainment".to_string();
    }
    
    // Business/Transactional
    if subject_lower.contains("rental") || subject_lower.contains("request") {
        return "business".to_string();
    }
    
    // Food/Shopping
    if from_lower.contains("smood") {
        return "shopping".to_string();
    }
    
    "other".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_completeness_assessment() {
        match load_all_test_emails() {
            Ok(emails) => {
                println!("📊 Test Data Completeness Assessment");
                println!("====================================");
                
                let total = emails.len();
                let with_subject = emails.iter().filter(|e| e.subject.is_some()).count();
                let with_snippet = emails.iter().filter(|e| e.snippet.is_some()).count();
                let with_from = emails.iter().filter(|e| e.from.is_some()).count();
                let with_to = emails.iter().filter(|e| e.to.is_some()).count();
                let with_body = emails.iter().filter(|e| e.body.is_some()).count();
                let with_sent = emails.iter().filter(|e| e.sent.is_some()).count();
                
                println!("📈 Field Completeness:");
                println!("   • Total emails: {}", total);
                println!("   • With subject: {} ({:.1}%)", with_subject, (with_subject as f64 / total as f64) * 100.0);
                println!("   • With snippet: {} ({:.1}%)", with_snippet, (with_snippet as f64 / total as f64) * 100.0);
                println!("   • With from: {} ({:.1}%)", with_from, (with_from as f64 / total as f64) * 100.0);
                println!("   • With to: {} ({:.1}%)", with_to, (with_to as f64 / total as f64) * 100.0);
                println!("   • With body: {} ({:.1}%)", with_body, (with_body as f64 / total as f64) * 100.0);
                println!("   • With sent: {} ({:.1}%)", with_sent, (with_sent as f64 / total as f64) * 100.0);
                
                // Assert minimum quality thresholds
                assert!(with_subject >= (total * 90 / 100), "At least 90% of emails should have subjects");
                assert!(with_from >= (total * 95 / 100), "At least 95% of emails should have from addresses");
                assert!(with_snippet >= (total * 80 / 100), "At least 80% of emails should have snippets");
                
                println!("✅ Test data meets completeness thresholds");
            }
            Err(e) => {
                panic!("❌ Could not load test data: {}. Run 'cargo run --bin download_test_data' first.", e);
            }
        }
    }

    #[test]
    fn test_data_diversity_assessment() {
        match load_all_test_emails() {
            Ok(emails) => {
                println!("🎯 Test Data Diversity Assessment");
                println!("=================================");
                
                // Analyze sender domains
                let domain_analysis = analyze_sender_domains(&emails);
                println!("📧 Sender Domain Diversity:");
                for (domain, subjects) in &domain_analysis {
                    println!("   • {}: {} emails", domain, subjects.len());
                }
                
                // Analyze content categories
                let category_analysis = categorize_emails_by_content(&emails);
                println!("\n📂 Content Category Diversity:");
                for (category, emails) in &category_analysis {
                    println!("   • {}: {} emails", category, emails.len());
                    for email in emails.iter().take(2) {
                        println!("     - {}", email);
                    }
                    if emails.len() > 2 {
                        println!("     ... and {} more", emails.len() - 2);
                    }
                }
                
                // Assert diversity requirements
                assert!(domain_analysis.len() >= 5, "Should have emails from at least 5 different domains");
                assert!(category_analysis.len() >= 4, "Should have at least 4 different content categories");
                
                println!("✅ Test data meets diversity requirements");
            }
            Err(e) => {
                panic!("❌ Could not load test data: {}. Run 'cargo run --bin download_test_data' first.", e);
            }
        }
    }

    #[tokio::test]
    async fn test_classifier_compatibility_with_real_data() {
        match load_all_test_emails() {
            Ok(test_emails) => {
                println!("🔬 Classifier Compatibility Test");
                println!("=================================");
                
                // Convert to Email objects
                let emails: Vec<Email> = test_emails.into_iter()
                    .take(10) // Test with first 10 emails
                    .map(|te| te.to_email())
                    .collect();
                
                // Test with StubClassifier to verify data structure compatibility
                let classifier = StubClassifier::new();
                let mut classifications = Vec::new();
                
                println!("🧪 Testing classification compatibility:");
                for email in &emails {
                    match classifier.classify(email).await {
                        Ok(classification) => {
                            println!("   ✅ Email '{}' classified as: {:?}", 
                                email.subject_or_default(), classification.category);
                            classifications.push(classification);
                        }
                        Err(e) => {
                            println!("   ❌ Failed to classify email '{}': {}", 
                                email.subject_or_default(), e);
                        }
                    }
                }
                
                // Assert all emails were classified successfully
                assert_eq!(classifications.len(), emails.len(), 
                    "All emails should be classifiable");
                
                // Verify classifications have expected structure
                for classification in &classifications {
                    assert!(!classification.category.is_empty(), "Category should not be empty");
                    if let Some(score) = classification.score {
                        assert!(score >= 0.0 && score <= 1.0, 
                            "Score should be between 0 and 1");
                    }
                }
                
                println!("✅ All emails are compatible with classifier interface");
            }
            Err(e) => {
                panic!("❌ Could not load test data: {}. Run 'cargo run --bin download_test_data' first.", e);
            }
        }
    }

    #[test]
    fn test_data_quality_metrics() {
        match load_all_test_emails() {
            Ok(emails) => {
                println!("📏 Test Data Quality Metrics");
                println!("============================");
                
                let mut subject_lengths = Vec::new();
                let mut body_lengths = Vec::new();
                let mut snippet_lengths = Vec::new();
                
                for email in &emails {
                    if let Some(subject) = &email.subject {
                        subject_lengths.push(subject.len());
                    }
                    if let Some(body) = &email.body {
                        body_lengths.push(body.len());
                    }
                    if let Some(snippet) = &email.snippet {
                        snippet_lengths.push(snippet.len());
                    }
                }
                
                // Calculate averages
                let avg_subject_len = if !subject_lengths.is_empty() {
                    subject_lengths.iter().sum::<usize>() as f64 / subject_lengths.len() as f64
                } else { 0.0 };
                
                let avg_body_len = if !body_lengths.is_empty() {
                    body_lengths.iter().sum::<usize>() as f64 / body_lengths.len() as f64
                } else { 0.0 };
                
                let avg_snippet_len = if !snippet_lengths.is_empty() {
                    snippet_lengths.iter().sum::<usize>() as f64 / snippet_lengths.len() as f64
                } else { 0.0 };
                
                println!("📊 Content Length Metrics:");
                println!("   • Average subject length: {:.1} characters", avg_subject_len);
                println!("   • Average body length: {:.1} characters", avg_body_len);
                println!("   • Average snippet length: {:.1} characters", avg_snippet_len);
                
                // Quality assertions
                assert!(avg_subject_len > 10.0, "Average subject length should be meaningful");
                assert!(avg_snippet_len > 20.0, "Average snippet length should be meaningful");
                
                // Check for duplicate emails
                let mut unique_ids = HashSet::new();
                let mut duplicate_count = 0;
                
                for email in &emails {
                    if !unique_ids.insert(&email.id) {
                        duplicate_count += 1;
                    }
                }
                
                println!("🔍 Uniqueness Check:");
                println!("   • Unique emails: {}", unique_ids.len());
                println!("   • Duplicates: {}", duplicate_count);
                
                assert_eq!(duplicate_count, 0, "Should have no duplicate emails");
                
                println!("✅ Test data meets quality thresholds");
            }
            Err(e) => {
                panic!("❌ Could not load test data: {}. Run 'cargo run --bin download_test_data' first.", e);
            }
        }
    }

    #[test]
    fn test_data_suitability_for_classification_categories() {
        match load_all_test_emails() {
            Ok(emails) => {
                println!("🎯 Classification Category Suitability Assessment");
                println!("===============================================");
                
                let categories = categorize_emails_by_content(&emails);
                
                // Check we have emails suitable for different classification scenarios
                let required_categories = vec![
                    "dev_notifications",
                    "social_professional", 
                    "newsletter",
                    "security"
                ];
                
                println!("🔍 Required category coverage:");
                for category in &required_categories {
                    if let Some(emails_in_category) = categories.get(*category) {
                        println!("   ✅ {}: {} emails", category, emails_in_category.len());
                        assert!(emails_in_category.len() > 0, "Should have emails in {} category", category);
                    } else {
                        println!("   ❌ {}: 0 emails", category);
                        panic!("Missing emails for required category: {}", category);
                    }
                }
                
                // Print distribution of all categories
                println!("\n📊 Full category distribution:");
                for (category, emails_in_category) in &categories {
                    println!("   • {}: {} emails ({:.1}%)", 
                        category, 
                        emails_in_category.len(),
                        (emails_in_category.len() as f64 / emails.len() as f64) * 100.0
                    );
                }
                
                println!("✅ Test data covers all required classification categories");
            }
            Err(e) => {
                panic!("❌ Could not load test data: {}. Run 'cargo run --bin download_test_data' first.", e);
            }
        }
    }
}
