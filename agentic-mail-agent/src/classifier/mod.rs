//! Email classification trait and implementations.
//! 
//! This module provides an abstraction for classifying emails using various
//! classification methods, including local LLM integration via Ollama.

mod stub;
mod langchain;
mod text_preprocessing;
mod hybrid;
mod mock_ollama;

pub mod rules;
pub mod llm;

pub use stub::StubClassifier;
pub use langchain::{LangChainClassifier, LangChainConfig};
pub use hybrid::HybridClassifier;
pub use mock_ollama::{MockOllamaClassifier, RecordedResponse, RecordedResponses};
pub use text_preprocessing::{
    clean_html_for_classification,
    clean_text_for_classification,
    prepare_email_for_classification,
    prepare_email_metadata_for_classification,
};

use crate::core::email::Email;

/// Result of email classification.
/// 
/// Contains the classification category, optional confidence score,
/// and the raw LLM response for debugging and audit purposes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Classification {
    /// The classified category (e.g., "work", "personal", "promotional", "spam")
    pub category: String,
    /// Optional confidence score from 0.0 to 1.0, where 1.0 is highest confidence
    pub score: Option<f32>,
    /// Raw response from the LLM for debugging and audit purposes
    pub llm_response: String,
}

impl Classification {
    /// Create a new Classification result.
    pub fn new(category: String, score: Option<f32>, llm_response: String) -> Self {
        Self {
            category,
            score,
            llm_response,
        }
    }
    
    /// Create a Classification with just a category (no score).
    pub fn with_category(category: String) -> Self {
        Self {
            llm_response: format!("Category: {category}"),
            category,
            score: None,
        }
    }
    
    /// Create a Classification with category and score.
    pub fn with_score(category: String, score: f32) -> Self {
        Self {
            llm_response: format!("Category: {category} (score: {score:.2})"),
            category,
            score: Some(score),
        }
    }
}

/// Errors that can occur during email classification.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ClassificationError {
    /// LLM service communication error
    #[error("LLM service error: {message}")]
    LlmService { message: String },
    
    /// Invalid LLM response format
    #[error("Invalid response format: {message}")]
    InvalidResponse { message: String },
    
    /// Configuration error for the classifier
    #[error("Classifier configuration error: {message}")]
    Config { message: String },
    
    /// Network or connectivity error
    #[error("Network error: {message}")]
    Network { message: String },
    
    /// Unknown or unexpected error
    #[error("Unknown classification error: {message}")]
    Unknown { message: String },
}

impl ClassificationError {
    /// Create a new LLM service error with a message
    pub fn llm_service(message: impl Into<String>) -> Self {
        Self::LlmService { message: message.into() }
    }
    
    /// Create a new invalid response error with a message
    pub fn invalid_response(message: impl Into<String>) -> Self {
        Self::InvalidResponse { message: message.into() }
    }
    
    /// Create a new config error with a message
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    /// Create a new network error with a message
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network { message: message.into() }
    }
    
    /// Create a new unknown error with a message
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown { message: message.into() }
    }
}

/// Trait for classifying email messages.
/// 
/// This trait provides a unified interface for classifying emails using different
/// classification methods, from simple rule-based systems to sophisticated LLM-powered
/// classification.
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use agentic_mail_agent::classifier::{MessageClassifier, StubClassifier};
/// use agentic_mail_agent::core::email::Email;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let classifier = StubClassifier::new();
///     let email = Email::with_subject("test@example.com".to_string(), "Meeting reminder".to_string());
///     let classification = classifier.classify(&email).await?;
///     println!("Email classified as: {classification.category}");
///     Ok(())
/// }
/// ```
#[async_trait::async_trait]
pub trait MessageClassifier {
    /// Classify an email and return the classification result.
    /// 
    /// This method analyzes the email content (subject, snippet, etc.) and
    /// returns a classification with category, optional confidence score,
    /// and raw LLM response for audit purposes.
    /// 
    /// # Arguments
    /// 
    /// * `email` - The email to classify
    /// 
    /// # Returns
    /// 
    /// Returns a `Classification` containing the category, optional score,
    /// and raw LLM response, or a `ClassificationError` if classification fails.
    async fn classify(&self, email: &Email) -> Result<Classification, ClassificationError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_new() {
        let classification = Classification::new(
            "work".to_string(),
            Some(0.85),
            "This email appears to be work-related".to_string(),
        );
        
        assert_eq!(classification.category, "work");
        assert_eq!(classification.score, Some(0.85));
        assert_eq!(classification.llm_response, "This email appears to be work-related");
    }
    
    #[test]
    fn classification_with_category() {
        let classification = Classification::with_category("personal".to_string());
        
        assert_eq!(classification.category, "personal");
        assert_eq!(classification.score, None);
        assert_eq!(classification.llm_response, "Category: personal");
    }
    
    #[test]
    fn classification_with_score() {
        let classification = Classification::with_score("spam".to_string(), 0.92);
        
        assert_eq!(classification.category, "spam");
        assert_eq!(classification.score, Some(0.92));
        assert_eq!(classification.llm_response, "Category: spam (score: 0.92)");
    }
    
    #[test]
    fn classification_error_creation() {
        let llm_error = ClassificationError::llm_service("Connection failed");
        assert!(matches!(llm_error, ClassificationError::LlmService { .. }));
        
        let invalid_error = ClassificationError::invalid_response("Malformed JSON");
        assert!(matches!(invalid_error, ClassificationError::InvalidResponse { .. }));
        
        let config_error = ClassificationError::config("Missing API key");
        assert!(matches!(config_error, ClassificationError::Config { .. }));
        
        let network_error = ClassificationError::network("Timeout");
        assert!(matches!(network_error, ClassificationError::Network { .. }));
        
        let unknown_error = ClassificationError::unknown("Unexpected error");
        assert!(matches!(unknown_error, ClassificationError::Unknown { .. }));
    }
}
