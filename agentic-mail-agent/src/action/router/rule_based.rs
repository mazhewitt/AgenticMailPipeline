//! Rule-based action router implementation.
//!
//! This module provides a simple rule-based implementation of the ActionRouter trait
//! that maps email classifications to predefined actions based on configurable rules.

use super::{ActionRouter, EmailAction, RoutingError, RoutingResult};
use crate::classifier::Classification;
use crate::config::LabelConfig;
use crate::core::email::Email;
use std::collections::HashMap;

/// Configuration for rule-based routing.
///
/// Defines the mapping from classification categories to actions,
/// along with additional rules based on confidence thresholds.
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Map from classification category to default actions
    pub category_actions: HashMap<String, Vec<EmailAction>>,
    /// Minimum confidence threshold for applying actions (0.0 to 1.0)
    pub confidence_threshold: f32,
    /// Actions to take for low-confidence classifications
    pub low_confidence_actions: Vec<EmailAction>,
    /// Actions to take for urgent/high-priority emails
    pub urgent_actions: Vec<EmailAction>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        let label_config = LabelConfig::new();
        let mut category_actions = HashMap::new();

        // Default rules for common email categories using human-friendly labels
        category_actions.insert(
            "work".to_string(),
            vec![
                EmailAction::label(label_config.production.work.clone()),
                EmailAction::MarkAsRead,
            ],
        );

        category_actions.insert(
            "personal".to_string(),
            vec![EmailAction::label(label_config.production.personal.clone())],
        );

        category_actions.insert(
            "promotional".to_string(),
            vec![
                EmailAction::label(label_config.production.promotional.clone()),
                EmailAction::Archive,
            ],
        );

        category_actions.insert(
            "spam".to_string(),
            vec![
                EmailAction::label(label_config.production.spam.clone()),
                EmailAction::Archive,
            ],
        );

        category_actions.insert(
            "urgent".to_string(),
            vec![
                EmailAction::label(label_config.production.urgent.clone()),
                EmailAction::MarkImportant,
                EmailAction::escalate("Urgent email detected", 4),
            ],
        );

        category_actions.insert(
            "newsletter".to_string(),
            vec![
                EmailAction::label(label_config.production.newsletter.clone()),
                EmailAction::move_to("newsletters"),
            ],
        );

        category_actions.insert(
            "notification".to_string(),
            vec![
                EmailAction::label(label_config.production.notification.clone()),
                EmailAction::MarkAsRead,
            ],
        );

        Self {
            category_actions,
            confidence_threshold: 0.7,
            low_confidence_actions: vec![EmailAction::label(
                label_config.production.needs_review.clone(),
            )],
            urgent_actions: vec![
                EmailAction::MarkImportant,
                EmailAction::escalate("High priority email", 4),
            ],
        }
    }
}

impl RoutingConfig {
    /// Create a new routing configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the confidence threshold for applying actions.
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Add a category mapping to the configuration.
    pub fn with_category_mapping(mut self, category: String, actions: Vec<EmailAction>) -> Self {
        self.category_actions.insert(category, actions);
        self
    }

    /// Set the actions for low-confidence classifications.
    pub fn with_low_confidence_actions(mut self, actions: Vec<EmailAction>) -> Self {
        self.low_confidence_actions = actions;
        self
    }

    /// Set the actions for urgent emails.
    pub fn with_urgent_actions(mut self, actions: Vec<EmailAction>) -> Self {
        self.urgent_actions = actions;
        self
    }
}

/// Rule-based implementation of ActionRouter.
///
/// Routes emails to actions based on simple rules that map classification
/// categories to predefined actions. Also considers confidence scores and
/// special handling for urgent emails.
pub struct RuleBasedRouter {
    config: RoutingConfig,
}

impl RuleBasedRouter {
    /// Create a new rule-based router with default configuration.
    pub fn new() -> Self {
        Self {
            config: RoutingConfig::default(),
        }
    }

    /// Create a new rule-based router with custom configuration.
    pub fn with_config(config: RoutingConfig) -> Self {
        Self { config }
    }

    /// Check if an email appears to be urgent based on subject and content.
    fn is_urgent_email(&self, email: &Email) -> bool {
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

    /// Get actions for a specific category.
    fn get_category_actions(&self, category: &str) -> Vec<EmailAction> {
        self.config
            .category_actions
            .get(category)
            .cloned()
            .unwrap_or_else(|| vec![EmailAction::label(category.to_string())])
    }
}

impl Default for RuleBasedRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ActionRouter for RuleBasedRouter {
    async fn route(
        &self,
        email: &Email,
        classification: &Classification,
    ) -> Result<RoutingResult, RoutingError> {
        // Check if classification is valid
        if classification.category.is_empty() {
            return Err(RoutingError::invalid_classification("Empty category"));
        }

        let confidence = classification.score.unwrap_or(1.0);
        let is_urgent = self.is_urgent_email(email);

        // Determine actions based on rules
        let mut actions = Vec::new();
        let mut reasoning_parts = Vec::new();

        // Handle urgent emails first
        if is_urgent {
            actions.extend(self.config.urgent_actions.clone());
            reasoning_parts.push("urgent content detected".to_string());
        }

        // Handle low confidence classifications
        if confidence < self.config.confidence_threshold {
            actions.extend(self.config.low_confidence_actions.clone());
            reasoning_parts.push(format!("low confidence ({confidence:.2})"));
        } else {
            // Apply category-based actions for high-confidence classifications
            let category_actions = self.get_category_actions(&classification.category);
            actions.extend(category_actions);
            reasoning_parts.push(format!("category: {}", classification.category));
        }

        // Remove duplicate actions
        actions.dedup();

        // If no actions were determined, default to no action
        if actions.is_empty() {
            actions.push(EmailAction::NoAction);
            reasoning_parts.push("no matching rules".to_string());
        }

        let reasoning = format!(
            "Rule-based routing: {} (confidence: {:.2})",
            reasoning_parts.join(", "),
            confidence
        );

        Ok(RoutingResult::new(actions, reasoning, confidence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classifier::Classification;

    #[test]
    fn test_routing_config_default() {
        let config = RoutingConfig::default();
        assert!(config.category_actions.contains_key("work"));
        assert!(config.category_actions.contains_key("spam"));
        assert_eq!(config.confidence_threshold, 0.7);
    }

    #[test]
    fn test_routing_config_builder() {
        let config = RoutingConfig::new()
            .with_confidence_threshold(0.8)
            .with_category_mapping("test".to_string(), vec![EmailAction::Archive]);

        assert_eq!(config.confidence_threshold, 0.8);
        assert!(config.category_actions.contains_key("test"));
    }

    #[tokio::test]
    async fn test_rule_based_router_work_email() {
        let router = RuleBasedRouter::new();
        let email = Email::with_subject("1".to_string(), "Meeting tomorrow".to_string());
        let classification = Classification::with_score("work".to_string(), 0.9);

        let result = router.route(&email, &classification).await.unwrap();

        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, EmailAction::Label { label } if label == "Work")));
        assert!(result.reasoning.contains("category: work"));
        assert_eq!(result.confidence, 0.9);
    }

    #[tokio::test]
    async fn test_rule_based_router_urgent_email() {
        let router = RuleBasedRouter::new();
        let email = Email::new(
            "1".to_string(),
            Some("URGENT: Action Required".to_string()),
            Some("Please respond immediately".to_string()),
        );
        let classification = Classification::with_score("work".to_string(), 0.8);

        let result = router.route(&email, &classification).await.unwrap();

        assert!(result.has_high_priority_actions());
        assert!(result.reasoning.contains("urgent content detected"));
    }

    #[tokio::test]
    async fn test_rule_based_router_low_confidence() {
        let router = RuleBasedRouter::new();
        let email = Email::with_subject("1".to_string(), "Some email".to_string());
        let classification = Classification::with_score("unknown".to_string(), 0.3);

        let result = router.route(&email, &classification).await.unwrap();

        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, EmailAction::Label { label } if label == "Needs Review")));
        assert!(result.reasoning.contains("low confidence"));
    }

    #[tokio::test]
    async fn test_rule_based_router_spam() {
        let router = RuleBasedRouter::new();
        let email = Email::with_subject("1".to_string(), "You won the lottery!".to_string());
        let classification = Classification::with_score("spam".to_string(), 0.95);

        let result = router.route(&email, &classification).await.unwrap();

        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, EmailAction::Label { label } if label == "Spam")));
        assert!(result.actions.contains(&EmailAction::Archive));
    }

    #[tokio::test]
    async fn test_rule_based_router_invalid_classification() {
        let router = RuleBasedRouter::new();
        let email = Email::with_subject("1".to_string(), "Test".to_string());
        let classification = Classification::with_category("".to_string());

        let result = router.route(&email, &classification).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RoutingError::InvalidClassification { .. }
        ));
    }

    #[test]
    fn test_urgent_email_detection() {
        let router = RuleBasedRouter::new();

        let urgent_email = Email::new(
            "1".to_string(),
            Some("URGENT: Server Down".to_string()),
            Some("The production server is down".to_string()),
        );
        assert!(router.is_urgent_email(&urgent_email));

        let normal_email = Email::with_subject("2".to_string(), "Weekly newsletter".to_string());
        assert!(!router.is_urgent_email(&normal_email));

        // Test urgent detection in body content
        let urgent_in_body = Email::new_full(
            "3".to_string(),
            Some("Status Update".to_string()),
            Some("Regular status update".to_string()),
            Some("sender@example.com".to_string()),
            Some(vec!["recipient@example.com".to_string()]),
            Some("Wed, 30 Jun 2023 10:00:00 +0000".to_string()),
            Some("This is a regular status update. However, there is an EMERGENCY situation that requires immediate attention.".to_string()),
        );
        assert!(router.is_urgent_email(&urgent_in_body));

        // Test urgent detection in from field for VIP senders
        let vip_email = Email::new_full(
            "4".to_string(),
            Some("Regular Meeting".to_string()),
            Some("Regular meeting notes".to_string()),
            Some("ceo@company.com".to_string()),
            Some(vec!["employee@company.com".to_string()]),
            Some("Wed, 30 Jun 2023 10:00:00 +0000".to_string()),
            Some("Please review the quarterly reports.".to_string()),
        );
        // For now, this should not be urgent unless we add VIP detection
        assert!(!router.is_urgent_email(&vip_email));
    }
}
