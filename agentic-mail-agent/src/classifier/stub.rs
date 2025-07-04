//! Stub implementation of MessageClassifier for testing and development.

use async_trait::async_trait;
use rand::Rng;

use super::{Classification, ClassificationError, MessageClassifier};
use super::text_preprocessing::prepare_email_for_classification;
use crate::email::Email;

/// A stub classifier that returns predefined or random classifications.
/// 
/// This classifier is useful for testing and development when you don't want
/// to make actual LLM calls. It can return either a fixed classification
/// or randomly choose from a set of predefined categories.
#[derive(Debug, Clone)]
pub struct StubClassifier {
    /// Optional fixed classification to always return
    fixed_classification: Option<Classification>,
    /// Optional fixed error to always return
    fixed_error: Option<ClassificationError>,
    /// Whether to use random categories when no fixed classification is set
    use_random: bool,
}

impl StubClassifier {
    /// Create a new StubClassifier that returns random classifications.
    pub fn new() -> Self {
        Self {
            fixed_classification: None,
            fixed_error: None,
            use_random: true,
        }
    }
    
    /// Create a StubClassifier that always returns the given classification.
    pub fn with_fixed_classification(classification: Classification) -> Self {
        Self {
            fixed_classification: Some(classification),
            fixed_error: None,
            use_random: false,
        }
    }
    
    /// Create a StubClassifier that always returns the given error.
    pub fn with_fixed_error(error: ClassificationError) -> Self {
        Self {
            fixed_classification: None,
            fixed_error: Some(error),
            use_random: false,
        }
    }
    
    /// Create a StubClassifier that returns deterministic results based on email content.
    /// This is useful for tests that need predictable outcomes.
    pub fn deterministic() -> Self {
        Self {
            fixed_classification: None,
            fixed_error: None,
            use_random: false,
        }
    }
    
    /// Generate a random classification from predefined categories.
    fn random_classification(&self) -> Classification {
        let categories = ["ActionRequired", "InterestingInfo", "Reference", "Noise", "Spam"];
        let mut rng = rand::rng();
        
        let category = categories[rng.random_range(0..categories.len())].to_string();
        let score = rng.random_range(0.5..1.0);
        
        Classification::with_score(category, score)
    }
    
    /// Generate a deterministic classification based on email content.
    fn deterministic_classification(&self, email: &Email) -> Classification {
        // Use text preprocessing to clean the content
        let cleaned_content = prepare_email_for_classification(
            email.subject.as_deref(),
            email.snippet.as_deref(),
            email.body.as_deref(),
            100 // Limit for deterministic processing
        ).to_lowercase();
        
        // Extract sender domain for additional classification hints
        let sender_domain = email.from.as_deref()
            .and_then(|from| from.split('@').nth(1))
            .unwrap_or("")
            .to_lowercase();
        
        let (category, score) = if 
            // ActionRequired patterns
            cleaned_content.contains("ci") && cleaned_content.contains("failed") ||
            cleaned_content.contains("meeting") && (cleaned_content.contains("tomorrow") || cleaned_content.contains("reminder")) ||
            cleaned_content.contains("urgent") || cleaned_content.contains("asap") || cleaned_content.contains("action required") ||
            cleaned_content.contains("deadline") || cleaned_content.contains("due") ||
            cleaned_content.contains("transfer") && cleaned_content.contains("ticket") ||
            cleaned_content.contains("schule") || cleaned_content.contains("school") // German school emails
        {
            ("ActionRequired", 0.9)
        } else if 
            // InterestingInfo patterns
            (cleaned_content.contains("newsletter") || cleaned_content.contains("digest")) && (cleaned_content.contains("tech") || cleaned_content.contains("news")) ||
            cleaned_content.contains("security") && cleaned_content.contains("alert") ||
            cleaned_content.contains("economics") || cleaned_content.contains("financial") ||
            cleaned_content.contains("scam") && cleaned_content.contains("protect") ||
            cleaned_content.contains("new login") || cleaned_content.contains("login") && cleaned_content.contains("device") ||
            sender_domain.contains("nytimes") || sender_domain.contains("anthropic") && cleaned_content.contains("update")
        {
            ("InterestingInfo", 0.85)
        } else if 
            // Reference patterns
            cleaned_content.contains("receipt") || cleaned_content.contains("invoice") ||
            cleaned_content.contains("confirmation") || cleaned_content.contains("delivered") ||
            cleaned_content.contains("consignment") || cleaned_content.contains("shipping") ||
            cleaned_content.contains("terms") && cleaned_content.contains("conditions") ||
            cleaned_content.contains("welcome") && cleaned_content.contains("plan") ||
            cleaned_content.contains("login") && cleaned_content.contains("secure") ||
            cleaned_content.contains("bbq") || cleaned_content.contains("tomorrow") && !cleaned_content.contains("meeting") // Personal conversations
        {
            ("Reference", 0.8)
        } else if 
            // Spam patterns
            cleaned_content.contains("lottery") || cleaned_content.contains("won") && cleaned_content.contains("million") ||
            cleaned_content.contains("click here") && cleaned_content.contains("claim") ||
            cleaned_content.contains("suspicious") && cleaned_content.contains("offer")
        {
            ("Spam", 0.95)
        } else if 
            // Noise patterns
            cleaned_content.contains("follow") && (cleaned_content.contains("linkedin") || cleaned_content.contains("ceo")) ||
            cleaned_content.contains("notification") && (cleaned_content.contains("facebook") || cleaned_content.contains("social")) ||
            cleaned_content.contains("posted") && cleaned_content.contains("update") ||
            cleaned_content.contains("connection") || cleaned_content.contains("add") && cleaned_content.contains("contact") ||
            cleaned_content.contains("promotional") || cleaned_content.contains("newsletter") && !cleaned_content.contains("tech") ||
            cleaned_content.contains("marketing") || cleaned_content.contains("offer") ||
            sender_domain.contains("facebook") || sender_domain.contains("linkedin") ||
            sender_domain.contains("aliexpress") || sender_domain.contains("nespresso") ||
            // Check original subject/snippet for unsubscribe since it gets filtered out
            email.subject.as_deref().unwrap_or("").to_lowercase().contains("unsubscribe") ||
            email.snippet.as_deref().unwrap_or("").to_lowercase().contains("unsubscribe")
        {
            ("Noise", 0.85)
        } else {
            ("Reference", 0.6) // Default to Reference as it's the safest default
        };
        
        Classification::new(
            category.to_string(),
            Some(score),
            format!("Deterministic classification based on cleaned content and patterns: {}", category),
        )
    }
}

impl Default for StubClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageClassifier for StubClassifier {
    async fn classify(&self, email: &Email) -> Result<Classification, ClassificationError> {
        // Return fixed error if configured
        if let Some(error) = &self.fixed_error {
            return Err(error.clone());
        }
        
        // Return fixed classification if configured
        if let Some(classification) = &self.fixed_classification {
            return Ok(classification.clone());
        }
        
        // Return deterministic or random classification
        let classification = if self.use_random {
            self.random_classification()
        } else {
            self.deterministic_classification(email)
        };
        
        Ok(classification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_classifier_new() {
        let classifier = StubClassifier::new();
        let email = Email::with_subject("test@example.com".to_string(), "Test email".to_string());
        
        let result = classifier.classify(&email).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert!(!classification.category.is_empty());
        assert!(classification.score.is_some());
        assert!(!classification.llm_response.is_empty());
    }
    
    #[tokio::test]
    async fn stub_classifier_with_fixed_classification() {
        let fixed_classification = Classification::with_score("work".to_string(), 0.95);
        let classifier = StubClassifier::with_fixed_classification(fixed_classification.clone());
        let email = Email::with_subject("test@example.com".to_string(), "Test email".to_string());
        
        let result = classifier.classify(&email).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification, fixed_classification);
    }
    
    #[tokio::test]
    async fn stub_classifier_with_fixed_error() {
        let fixed_error = ClassificationError::llm_service("Test error");
        let classifier = StubClassifier::with_fixed_error(fixed_error.clone());
        let email = Email::with_subject("test@example.com".to_string(), "Test email".to_string());
        
        let result = classifier.classify(&email).await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert_eq!(error, fixed_error);
    }
    
    #[tokio::test]
    async fn stub_classifier_deterministic() {
        let classifier = StubClassifier::deterministic();
        
        // Test ActionRequired classification  
        let work_email = Email::with_subject("work@example.com".to_string(), "Meeting tomorrow".to_string());
        let result = classifier.classify(&work_email).await.unwrap();
        assert_eq!(result.category, "ActionRequired"); // Meeting tomorrow should be ActionRequired
        assert_eq!(result.score, Some(0.9));
        
        // Test ActionRequired classification
        let urgent_email = Email::with_subject("urgent@example.com".to_string(), "URGENT: Action required".to_string());
        let result = classifier.classify(&urgent_email).await.unwrap();
        assert_eq!(result.category, "ActionRequired");
        assert_eq!(result.score, Some(0.9));
        
        // Test Noise classification (promotional content)
        let promo_email = Email::with_subject("promo@example.com".to_string(), "Unsubscribe from our newsletter".to_string());
        let result = classifier.classify(&promo_email).await.unwrap();
        assert_eq!(result.category, "Noise");
        assert_eq!(result.score, Some(0.85));
        
        // Test Reference classification (receipt)
        let receipt_email = Email::with_subject("receipt@example.com".to_string(), "Your receipt from Company".to_string());
        let result = classifier.classify(&receipt_email).await.unwrap();
        assert_eq!(result.category, "Reference");
        assert_eq!(result.score, Some(0.8));
        
        // Test InterestingInfo classification (newsletter with tech content)
        let newsletter_email = Email::with_subject("news@example.com".to_string(), "Tech newsletter digest".to_string());
        let result = classifier.classify(&newsletter_email).await.unwrap();
        assert_eq!(result.category, "InterestingInfo");
        assert_eq!(result.score, Some(0.85));
        
        // Test default classification
        let default_email = Email::with_subject("unknown@example.com".to_string(), "Random content".to_string());
        let result = classifier.classify(&default_email).await.unwrap();
        assert_eq!(result.category, "Reference");
        assert_eq!(result.score, Some(0.6));
    }
    
    #[tokio::test]
    async fn stub_classifier_handles_empty_content() {
        let classifier = StubClassifier::deterministic();
        let email = Email::new("test@example.com".to_string(), None, None);
        
        let result = classifier.classify(&email).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification.category, "Reference"); // default category
        assert_eq!(classification.score, Some(0.6));
    }
    
    #[tokio::test]
    async fn stub_classifier_default() {
        let classifier = StubClassifier::default();
        let email = Email::with_subject("test@example.com".to_string(), "Test email".to_string());
        
        let result = classifier.classify(&email).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert!(!classification.category.is_empty());
    }
}
