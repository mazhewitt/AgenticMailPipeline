//! LangChain-based email classifier using local Ollama LLM.
//! 
//! This module provides an implementation of the MessageClassifier trait
//! that uses the langchain-rust crate with Ollama for local LLM-based
//! email classification.

use crate::classifier::{Classification, ClassificationError, MessageClassifier};
use crate::email::Email;
use async_trait::async_trait;
use langchain_rust::{
    llm::client::Ollama,
    language_models::llm::LLM,
};
use ollama_rs::Ollama as OllamaClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Response format expected from the LLM for email classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LLMClassificationResponse {
    /// The email category (work, personal, promotional, spam, newsletter, urgent)
    category: String,
    /// Confidence score from 0.0 to 1.0
    score: f64,
    /// Explanation for the classification decision
    explanation: String,
}

/// Configuration for the LangChain classifier.
#[derive(Debug, Clone)]
pub struct LangChainConfig {
    /// Ollama base URL (default: http://localhost:11434)
    pub ollama_url: String,
    /// Model name to use for classification (default: llama3:8b)
    pub model: String,
    /// Temperature for LLM generation (default: 0.1 for consistent classification)
    pub temperature: f64,
}

impl Default for LangChainConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.1:8b".to_string(),
            temperature: 0.1,
        }
    }
}

/// LangChain-based email classifier using Ollama.
/// 
/// This classifier uses a local Ollama instance with LangChain patterns
/// to classify emails into categories using an LLM.
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use agentic_mail_agent::classifier::{MessageClassifier, LangChainClassifier, LangChainConfig};
/// use agentic_mail_agent::email::Email;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = LangChainConfig::default();
///     let classifier = LangChainClassifier::new(config).await?;
///     
///     let email = Email::with_subject(
///         "test123".to_string(),
///         "Meeting reminder for tomorrow".to_string()
///     );
///     
///     let classification = classifier.classify(&email).await?;
///     println!("Category: {}", classification.category);
///     Ok(())
/// }
/// ```
pub struct LangChainClassifier {
    /// The configured Ollama LLM instance
    llm: Arc<Ollama>,
    /// Configuration for the classifier
    #[allow(dead_code)]
    config: LangChainConfig,
}

impl LangChainClassifier {
    /// Create a new LangChainClassifier with the given configuration.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Configuration for the classifier including Ollama URL and model
    /// 
    /// # Returns
    /// 
    /// Returns a new LangChainClassifier instance or a ClassificationError
    /// if the Ollama connection cannot be established.
    pub async fn new(config: LangChainConfig) -> Result<Self, ClassificationError> {
        // Create the Ollama client - use default for now
        let ollama_client = Arc::new(OllamaClient::default());
        let ollama = Ollama::new(
            ollama_client,
            config.model.clone(),
            None, // We'll use default generation options for now
        );

        // Test the connection by attempting a simple request
        match ollama.invoke("test").await {
            Ok(_) => {
                println!("✅ Connected to Ollama using model: {}", config.model);
                Ok(Self {
                    llm: Arc::new(ollama),
                    config,
                })
            }
            Err(e) => Err(ClassificationError::network(format!(
                "Failed to connect to Ollama at {}: {}",
                config.ollama_url, e
            ))),
        }
    }

    /// Create a new LangChainClassifier with default configuration.
    pub async fn with_default_config() -> Result<Self, ClassificationError> {
        Self::new(LangChainConfig::default()).await
    }

    /// Build the email classification prompt with the given email content.
    fn build_prompt(&self, email: &Email) -> String {
        let subject = email.subject_or_default();
        let from = email.from_or_default();
        let to = email.to_or_default();
        let sent = email.sent_or_default();
        let body = email.body_or_default();

        format!(
            r#"You are an expert email classifier. Your task is to classify emails into one of these categories:
- work: Business, professional, or work-related emails
- personal: Personal communications from friends, family, or acquaintances
- promotional: Marketing emails, sales, offers, or advertisements
- spam: Unwanted, suspicious, or clearly spam emails
- newsletter: Newsletters, updates, or regular communications from organizations
- urgent: Time-sensitive emails requiring immediate attention

Given the following email:
From: "{}"
To: "{}"
Date: "{}"
Subject: "{}"
Body: "{}"

Please classify this email and respond with a JSON object in this exact format:
{{
  "category": "one_of_the_categories_above",
  "score": 0.95,
  "explanation": "Brief explanation of why this email belongs to this category"
}}

Ensure the score is between 0.0 and 1.0, where 1.0 means completely confident.
Only respond with the JSON object, no additional text."#,
            from,
            to.join(", "),
            sent,
            subject,
            body
        )
    }

    /// Parse the LLM response into a structured classification.
    fn parse_llm_response(&self, response: &str) -> Result<LLMClassificationResponse, ClassificationError> {
        // Try to extract JSON from the response in case there's extra text
        let json_start = response.find('{');
        let json_end = response.rfind('}');

        let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
            &response[start..=end]
        } else {
            response
        };

        serde_json::from_str::<LLMClassificationResponse>(json_str)
            .map_err(|e| ClassificationError::invalid_response(format!(
                "Failed to parse LLM response as JSON: {}. Response was: {}",
                e, response
            )))
    }

    /// Validate that the classification category is one of the expected values.
    fn validate_category(&self, category: &str) -> Result<(), ClassificationError> {
        const VALID_CATEGORIES: &[&str] = &[
            "work", "personal", "promotional", "spam", "newsletter", "urgent"
        ];

        if VALID_CATEGORIES.contains(&category) {
            Ok(())
        } else {
            Err(ClassificationError::invalid_response(format!(
                "Invalid category '{}'. Must be one of: {}",
                category,
                VALID_CATEGORIES.join(", ")
            )))
        }
    }
}

#[async_trait]
impl MessageClassifier for LangChainClassifier {
    async fn classify(&self, email: &Email) -> Result<Classification, ClassificationError> {
        // Build the classification prompt
        let prompt = self.build_prompt(email);

        // Invoke the LLM with the prompt
        let response = self.llm
            .invoke(&prompt)
            .await
            .map_err(|e| ClassificationError::llm_service(format!(
                "LLM invocation failed: {}", e
            )))?;

        // Parse the response
        let parsed_response = self.parse_llm_response(&response)?;

        // Validate the category
        self.validate_category(&parsed_response.category)?;

        // Validate score range
        let score = if parsed_response.score < 0.0 || parsed_response.score > 1.0 {
            return Err(ClassificationError::invalid_response(format!(
                "Score {} is out of valid range [0.0, 1.0]",
                parsed_response.score
            )));
        } else {
            parsed_response.score as f32
        };

        // Create the classification result
        Ok(Classification::new(
            parsed_response.category,
            Some(score),
            format!(
                "LLM Response: {} (Score: {:.2})",
                parsed_response.explanation, score
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt() {
        let config = LangChainConfig::default();
        // Create a classifier without actually connecting to Ollama for testing
        let classifier = LangChainClassifier {
            llm: Arc::new(Ollama::new(
                Arc::new(OllamaClient::default()),
                config.model.clone(),
                None,
            )),
            config,
        };
        let email = Email::new_full(
            "test123".to_string(),
            Some("Meeting reminder".to_string()),
            Some("Don't forget our meeting tomorrow at 2pm".to_string()),
            Some("sender@example.com".to_string()),
            Some(vec!["recipient@example.com".to_string()]),
            Some("Wed, 30 Jun 2023 14:00:00 +0000".to_string()),
            Some("Don't forget our meeting tomorrow at 2pm".to_string()),
        );

        let prompt = classifier.build_prompt(&email);
        
        assert!(prompt.contains("Meeting reminder"));
        assert!(prompt.contains("Don't forget our meeting tomorrow at 2pm"));
        assert!(prompt.contains("sender@example.com"));
        assert!(prompt.contains("recipient@example.com"));
        assert!(prompt.contains("work"));
        assert!(prompt.contains("personal"));
        assert!(prompt.contains("promotional"));
        assert!(prompt.contains("spam"));
        assert!(prompt.contains("newsletter"));
        assert!(prompt.contains("urgent"));
    }

    #[test]
    fn test_build_prompt_with_full_email_fields() {
        // This test should fail initially - we haven't updated build_prompt yet
        let config = LangChainConfig::default();
        let classifier = LangChainClassifier {
            llm: Arc::new(Ollama::new(
                Arc::new(OllamaClient::default()),
                config.model.clone(),
                None,
            )),
            config,
        };
        
        let email = Email::new_full(
            "test123".to_string(),
            Some("Meeting Tomorrow".to_string()),
            Some("Don't forget our 2pm meeting".to_string()),
            Some("boss@company.com".to_string()),
            Some(vec!["employee@company.com".to_string()]),
            Some("Wed, 30 Jun 2023 10:00:00 +0000".to_string()),
            Some("Hi Team,\n\nThis is a reminder about our important meeting tomorrow at 2pm. Please bring your reports.\n\nBest regards,\nThe Boss".to_string()),
        );

        let prompt = classifier.build_prompt(&email);

        // Verify all the new fields are included
        assert!(prompt.contains("boss@company.com"), "Prompt should contain from address");
        assert!(prompt.contains("employee@company.com"), "Prompt should contain to address");
        assert!(prompt.contains("Wed, 30 Jun 2023"), "Prompt should contain sent date");
        assert!(prompt.contains("Meeting Tomorrow"), "Prompt should contain subject");
        assert!(prompt.contains("Hi Team"), "Prompt should contain full body");
        assert!(prompt.contains("Best regards"), "Prompt should contain full body");
    }

    #[test]
    fn test_parse_llm_response_valid() {
        let config = LangChainConfig::default();
        let classifier = LangChainClassifier {
            llm: Arc::new(Ollama::new(
                Arc::new(OllamaClient::default()),
                config.model.clone(),
                None,
            )),
            config,
        };
        let response = r#"{"category": "work", "score": 0.95, "explanation": "This is work-related"}"#;
        
        let parsed = classifier.parse_llm_response(response).unwrap();
        
        assert_eq!(parsed.category, "work");
        assert_eq!(parsed.score, 0.95);
        assert_eq!(parsed.explanation, "This is work-related");
    }

    #[test]
    fn test_parse_llm_response_with_extra_text() {
        let config = LangChainConfig::default();
        let classifier = LangChainClassifier {
            llm: Arc::new(Ollama::new(
                Arc::new(OllamaClient::default()),
                config.model.clone(),
                None,
            )),
            config,
        };
        let response = r#"Here's my analysis: {"category": "spam", "score": 0.88, "explanation": "Suspicious content"} Hope this helps!"#;
        
        let parsed = classifier.parse_llm_response(response).unwrap();
        
        assert_eq!(parsed.category, "spam");
        assert_eq!(parsed.score, 0.88);
        assert_eq!(parsed.explanation, "Suspicious content");
    }

    #[test]
    fn test_parse_llm_response_invalid_json() {
        let config = LangChainConfig::default();
        let classifier = LangChainClassifier {
            llm: Arc::new(Ollama::new(
                Arc::new(OllamaClient::default()),
                config.model.clone(),
                None,
            )),
            config,
        };
        let response = "This is not JSON at all";
        
        let result = classifier.parse_llm_response(response);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClassificationError::InvalidResponse { .. }));
    }

    #[test]
    fn test_validate_category_valid() {
        let config = LangChainConfig::default();
        let classifier = LangChainClassifier {
            llm: Arc::new(Ollama::new(
                Arc::new(OllamaClient::default()),
                config.model.clone(),
                None,
            )),
            config,
        };
        
        assert!(classifier.validate_category("work").is_ok());
        assert!(classifier.validate_category("personal").is_ok());
        assert!(classifier.validate_category("promotional").is_ok());
        assert!(classifier.validate_category("spam").is_ok());
        assert!(classifier.validate_category("newsletter").is_ok());
        assert!(classifier.validate_category("urgent").is_ok());
    }

    #[test]
    fn test_validate_category_invalid() {
        let config = LangChainConfig::default();
        let classifier = LangChainClassifier {
            llm: Arc::new(Ollama::new(
                Arc::new(OllamaClient::default()),
                config.model.clone(),
                None,
            )),
            config,
        };
        
        let result = classifier.validate_category("invalid_category");
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClassificationError::InvalidResponse { .. }));
    }

    #[test]
    fn test_langchain_config_default() {
        let config = LangChainConfig::default();
        
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.model, "llama3:8b");
        assert_eq!(config.temperature, 0.1);
    }
}
