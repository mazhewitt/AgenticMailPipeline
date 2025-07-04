//! Hybrid classifier combining rule-based and LLM approaches for optimal accuracy
//! 
//! This classifier uses the strengths of both rule-based classification and LLM inference:
//! - Rule-based for high-confidence patterns (CI failures, receipts, etc.)
//! - LLM for nuanced decisions (Noise vs InterestingInfo, complex content analysis)

use crate::classifier::{Classification, ClassificationError, MessageClassifier};
use crate::classifier::text_preprocessing::prepare_email_for_classification;
use crate::email::Email;
use async_trait::async_trait;
use std::sync::Arc;

/// Hybrid classifier that combines rule-based and LLM classification
/// 
/// Strategy:
/// 1. Apply high-confidence rule-based patterns first
/// 2. Use LLM for ambiguous cases requiring semantic understanding
/// 3. Post-process LLM results with confidence thresholds
pub struct HybridClassifier {
    llm_classifier: Option<Arc<dyn MessageClassifier + Send + Sync>>,
    use_llm: bool,
}

impl HybridClassifier {
    /// Create a new hybrid classifier with LLM support
    pub async fn new_with_llm(llm_classifier: Box<dyn MessageClassifier + Send + Sync>) -> Self {
        Self {
            llm_classifier: Some(Arc::from(llm_classifier)),
            use_llm: true,
        }
    }

    /// Create a new hybrid classifier without LLM (rules-only mode)
    pub fn new_rules_only() -> Self {
        Self {
            llm_classifier: None,
            use_llm: false,
        }
    }

    /// Apply high-confidence rule-based classification first
    fn apply_rule_based_classification(&self, email: &Email) -> Option<Classification> {
        let cleaned_content = prepare_email_for_classification(
            email.subject.as_deref(),
            email.snippet.as_deref(),
            email.body.as_deref(),
            100
        ).to_lowercase();

        let sender_domain = email.from.as_deref()
            .and_then(|from| from.split('@').nth(1))
            .unwrap_or("")
            .to_lowercase();

        // High-confidence ActionRequired patterns
        if cleaned_content.contains("ci") && cleaned_content.contains("failed") {
            return Some(Classification::new(
                "ActionRequired".to_string(),
                Some(0.95),
                "High-confidence rule: CI failure detected".to_string(),
            ));
        }

        if cleaned_content.contains("urgent") || cleaned_content.contains("asap") {
            return Some(Classification::new(
                "ActionRequired".to_string(),
                Some(0.90),
                "High-confidence rule: Urgent content detected".to_string(),
            ));
        }

        if cleaned_content.contains("transfer") && cleaned_content.contains("ticket") {
            return Some(Classification::new(
                "ActionRequired".to_string(),
                Some(0.92),
                "High-confidence rule: Ticket transfer required".to_string(),
            ));
        }

        if cleaned_content.contains("schule") || cleaned_content.contains("school") {
            return Some(Classification::new(
                "ActionRequired".to_string(),
                Some(0.88),
                "High-confidence rule: School communication".to_string(),
            ));
        }

        // High-confidence Reference patterns
        if cleaned_content.contains("receipt") || cleaned_content.contains("invoice") {
            return Some(Classification::new(
                "Reference".to_string(),
                Some(0.92),
                "High-confidence rule: Receipt/invoice detected".to_string(),
            ));
        }

        if cleaned_content.contains("delivered") || cleaned_content.contains("consignment") {
            return Some(Classification::new(
                "Reference".to_string(),
                Some(0.90),
                "High-confidence rule: Delivery confirmation".to_string(),
            ));
        }

        if cleaned_content.contains("welcome") && cleaned_content.contains("plan") {
            return Some(Classification::new(
                "Reference".to_string(),
                Some(0.85),
                "High-confidence rule: Service welcome message".to_string(),
            ));
        }

        // High-confidence Noise patterns
        if sender_domain.contains("facebook") || sender_domain.contains("linkedin") {
            return Some(Classification::new(
                "Noise".to_string(),
                Some(0.88),
                "High-confidence rule: Social media platform".to_string(),
            ));
        }

        if cleaned_content.contains("follow") && cleaned_content.contains("ceo") {
            return Some(Classification::new(
                "Noise".to_string(),
                Some(0.85),
                "High-confidence rule: Social connection suggestion".to_string(),
            ));
        }

        if cleaned_content.contains("notification") && sender_domain.contains("facebook") {
            return Some(Classification::new(
                "Noise".to_string(),
                Some(0.87),
                "High-confidence rule: Facebook notification".to_string(),
            ));
        }

        // High-confidence InterestingInfo patterns
        if cleaned_content.contains("security") && cleaned_content.contains("alert") {
            return Some(Classification::new(
                "InterestingInfo".to_string(),
                Some(0.85),
                "High-confidence rule: Security alert".to_string(),
            ));
        }

        if cleaned_content.contains("economics") || cleaned_content.contains("financial") {
            return Some(Classification::new(
                "InterestingInfo".to_string(),
                Some(0.83),
                "High-confidence rule: Economic/financial content".to_string(),
            ));
        }

        None
    }

    /// Post-process LLM results to fix common over-classification issues
    fn post_process_llm_result(&self, classification: Classification, email: &Email) -> Classification {
        let cleaned_content = prepare_email_for_classification(
            email.subject.as_deref(),
            email.snippet.as_deref(),
            email.body.as_deref(),
            100
        ).to_lowercase();

        // Fix over-classification of ActionRequired
        if classification.category == "ActionRequired" {
            // Login links should be Reference, not ActionRequired
            if cleaned_content.contains("login") && cleaned_content.contains("secure") {
                return Classification::new(
                    "Reference".to_string(),
                    classification.score,
                    "Post-processed: Login link corrected to Reference".to_string(),
                );
            }

            // Personal BBQ conversations should be Reference
            if email.subject.as_deref().unwrap_or("").to_lowercase().contains("tomorrow") &&
               !cleaned_content.contains("meeting") && !cleaned_content.contains("deadline") {
                return Classification::new(
                    "Reference".to_string(),
                    classification.score,
                    "Post-processed: Personal conversation corrected to Reference".to_string(),
                );
            }

            // Product recommendations should be Noise
            if cleaned_content.contains("purchased") || cleaned_content.contains("pair with") {
                return Classification::new(
                    "Noise".to_string(),
                    classification.score,
                    "Post-processed: Product recommendation corrected to Noise".to_string(),
                );
            }
        }

        // Fix over-classification of Spam
        if classification.category == "Spam" {
            // Legitimate login links should not be spam
            if cleaned_content.contains("login") && 
               (cleaned_content.contains("claude") || cleaned_content.contains("anthropic")) {
                return Classification::new(
                    "Reference".to_string(),
                    classification.score,
                    "Post-processed: Legitimate login link corrected from Spam".to_string(),
                );
            }

            // Newsletter content should be Noise, not Spam
            if cleaned_content.contains("newsletter") || cleaned_content.contains("unsubscribe") {
                return Classification::new(
                    "Noise".to_string(),
                    classification.score,
                    "Post-processed: Newsletter corrected from Spam to Noise".to_string(),
                );
            }
        }

        classification
    }
}

#[async_trait]
impl MessageClassifier for HybridClassifier {
    async fn classify(&self, email: &Email) -> Result<Classification, ClassificationError> {
        // Step 1: Try high-confidence rule-based classification
        if let Some(rule_classification) = self.apply_rule_based_classification(email) {
            return Ok(rule_classification);
        }

        // Step 2: Use LLM for ambiguous cases
        if self.use_llm {
            if let Some(llm_classifier) = &self.llm_classifier {
                match llm_classifier.classify(email).await {
                    Ok(llm_classification) => {
                        // Step 3: Post-process LLM result
                        let final_classification = self.post_process_llm_result(llm_classification, email);
                        return Ok(final_classification);
                    }
                    Err(e) => {
                        // Fall back to rules if LLM fails
                        eprintln!("LLM classification failed ({}), falling back to rule-based", e);
                    }
                }
            }
        }

        // Step 3: Fallback rule-based classification with lower confidence
        let cleaned_content = prepare_email_for_classification(
            email.subject.as_deref(),
            email.snippet.as_deref(),
            email.body.as_deref(),
            100
        ).to_lowercase();

        let sender_domain = email.from.as_deref()
            .and_then(|from| from.split('@').nth(1))
            .unwrap_or("")
            .to_lowercase();

        // Fallback patterns with lower confidence
        if cleaned_content.contains("newsletter") && cleaned_content.contains("tech") {
            Ok(Classification::new(
                "InterestingInfo".to_string(),
                Some(0.70),
                "Fallback rule: Tech newsletter content".to_string(),
            ))
        } else if cleaned_content.contains("terms") && cleaned_content.contains("conditions") {
            Ok(Classification::new(
                "Reference".to_string(),
                Some(0.75),
                "Fallback rule: Terms and conditions update".to_string(),
            ))
        } else if cleaned_content.contains("unsubscribe") || sender_domain.contains("marketing") {
            Ok(Classification::new(
                "Noise".to_string(),
                Some(0.70),
                "Fallback rule: Marketing/promotional content".to_string(),
            ))
        } else {
            // Ultimate fallback
            Ok(Classification::new(
                "Reference".to_string(),
                Some(0.60),
                "Ultimate fallback: Default to Reference".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classifier::StubClassifier;

    #[tokio::test]
    async fn test_hybrid_classifier_rule_based() {
        let classifier = HybridClassifier::new_rules_only();
        
        // Test CI failure detection
        let ci_email = Email::new(
            "test1".to_string(),
            Some("[Repo] CI failed: main".to_string()),
            Some("CI workflow run failed with 5 errors".to_string()),
        );
        
        let result = classifier.classify(&ci_email).await.unwrap();
        assert_eq!(result.category, "ActionRequired");
        assert!(result.score.unwrap() > 0.9);

        // Test receipt detection
        let receipt_email = Email::new(
            "test2".to_string(),
            Some("Your receipt from Company".to_string()),
            Some("Thank you for your purchase, here is your receipt".to_string()),
        );
        
        let result = classifier.classify(&receipt_email).await.unwrap();
        assert_eq!(result.category, "Reference");
        assert!(result.score.unwrap() > 0.9);
    }

    #[tokio::test]
    async fn test_hybrid_classifier_with_llm() {
        let stub_llm = Box::new(StubClassifier::deterministic());
        let classifier = HybridClassifier::new_with_llm(stub_llm).await;
        
        // High-confidence rule should override LLM
        let ci_email = Email::new(
            "test1".to_string(),
            Some("[Repo] CI failed: main".to_string()),
            Some("CI workflow run failed with 5 errors".to_string()),
        );
        
        let result = classifier.classify(&ci_email).await.unwrap();
        assert_eq!(result.category, "ActionRequired");
        assert!(result.llm_response.contains("High-confidence rule"));

        // Ambiguous case should use LLM (stub)
        let ambiguous_email = Email::new(
            "test2".to_string(),
            Some("Random content".to_string()),
            Some("Some random email content that doesn't match rules".to_string()),
        );
        
        let result = classifier.classify(&ambiguous_email).await.unwrap();
        // Should use LLM logic (stub classifier would classify this)
        assert!(!result.llm_response.contains("High-confidence rule"));
    }

    #[tokio::test]
    async fn test_post_processing() {
        let classifier = HybridClassifier::new_rules_only();
        
        // Test login link correction
        let login_email = Email::new(
            "test1".to_string(),
            Some("Secure link to log in".to_string()),
            Some("Click here to login securely to your account".to_string()),
        );
        
        // Simulate LLM over-classifying as ActionRequired
        let over_classified = Classification::new(
            "ActionRequired".to_string(),
            Some(0.8),
            "LLM classified as action required".to_string(),
        );
        
        let corrected = classifier.post_process_llm_result(over_classified, &login_email);
        assert_eq!(corrected.category, "Reference");
        assert!(corrected.llm_response.contains("Post-processed"));
    }
}