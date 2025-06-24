//! Stub implementation of MessageClassifier for testing and development.

use async_trait::async_trait;
use rand::Rng;

use super::{Classification, ClassificationError, MessageClassifier};
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
        let categories = ["work", "personal", "promotional", "spam", "newsletter", "urgent"];
        let mut rng = rand::thread_rng();
        
        let category = categories[rng.gen_range(0..categories.len())].to_string();
        let score = rng.gen_range(0.5..1.0);
        
        Classification::with_score(category, score)
    }
    
    /// Generate a deterministic classification based on email content.
    fn deterministic_classification(&self, email: &Email) -> Classification {
        // Simple rules based on email content for predictable testing
        let subject = email.subject.as_deref().unwrap_or("");
        let snippet = email.snippet.as_deref().unwrap_or("");
        let content = format!("{} {}", subject, snippet).to_lowercase();
        
        let (category, score) = if content.contains("meeting") || content.contains("calendar") {
            ("work", 0.9)
        } else if content.contains("urgent") || content.contains("asap") {
            ("urgent", 0.95)
        } else if content.contains("unsubscribe") || content.contains("promotional") {
            ("promotional", 0.85)
        } else if content.contains("spam") || content.contains("lottery") {
            ("spam", 0.98)
        } else if content.contains("newsletter") || content.contains("digest") {
            ("newsletter", 0.8)
        } else if content.contains("personal") || content.contains("family") {
            ("personal", 0.75)
        } else {
            ("work", 0.6) // default category
        };
        
        Classification::new(
            category.to_string(),
            Some(score),
            format!("Deterministic classification based on content analysis: {}", category),
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
        
        // Test work classification
        let work_email = Email::with_subject("work@example.com".to_string(), "Meeting tomorrow".to_string());
        let result = classifier.classify(&work_email).await.unwrap();
        assert_eq!(result.category, "work");
        assert_eq!(result.score, Some(0.9));
        
        // Test urgent classification
        let urgent_email = Email::with_subject("urgent@example.com".to_string(), "URGENT: Action required".to_string());
        let result = classifier.classify(&urgent_email).await.unwrap();
        assert_eq!(result.category, "urgent");
        assert_eq!(result.score, Some(0.95));
        
        // Test promotional classification
        let promo_email = Email::with_subject("promo@example.com".to_string(), "Unsubscribe from our newsletter".to_string());
        let result = classifier.classify(&promo_email).await.unwrap();
        assert_eq!(result.category, "promotional");
        assert_eq!(result.score, Some(0.85));
        
        // Test spam classification
        let spam_email = Email::with_subject("spam@example.com".to_string(), "You won the lottery!".to_string());
        let result = classifier.classify(&spam_email).await.unwrap();
        assert_eq!(result.category, "spam");
        assert_eq!(result.score, Some(0.98));
        
        // Test newsletter classification
        let newsletter_email = Email::with_subject("news@example.com".to_string(), "Weekly newsletter digest".to_string());
        let result = classifier.classify(&newsletter_email).await.unwrap();
        assert_eq!(result.category, "newsletter");
        assert_eq!(result.score, Some(0.8));
        
        // Test personal classification
        let personal_email = Email::with_subject("family@example.com".to_string(), "Personal family update".to_string());
        let result = classifier.classify(&personal_email).await.unwrap();
        assert_eq!(result.category, "personal");
        assert_eq!(result.score, Some(0.75));
        
        // Test default classification
        let default_email = Email::with_subject("unknown@example.com".to_string(), "Random content".to_string());
        let result = classifier.classify(&default_email).await.unwrap();
        assert_eq!(result.category, "work");
        assert_eq!(result.score, Some(0.6));
    }
    
    #[tokio::test]
    async fn stub_classifier_handles_empty_content() {
        let classifier = StubClassifier::deterministic();
        let email = Email::new("test@example.com".to_string(), None, None);
        
        let result = classifier.classify(&email).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification.category, "work"); // default category
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
