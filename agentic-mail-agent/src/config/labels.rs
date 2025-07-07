//! Email label configuration and management
//!
//! This module provides a centralized configuration for all email labels used
//! throughout the application, making them easy to maintain and human-friendly.

use std::collections::HashMap;

/// Configuration for email labels
#[derive(Debug, Clone, Default)]
pub struct LabelConfig {
    /// Human-readable label names for production use
    pub production: ProductionLabels,
    /// Test label configuration
    pub test: TestLabels,
}

/// Production email labels with human-friendly names
#[derive(Debug, Clone)]
pub struct ProductionLabels {
    /// Action required - needs immediate attention
    pub action_required: String,
    /// Interesting information - worth reading but not urgent
    pub interesting_info: String,
    /// Reference material - useful to keep
    pub reference: String,
    /// Low-value content - newsletters, notifications
    pub noise: String,
    /// Spam or unwanted content
    pub spam: String,
    /// Work-related emails
    pub work: String,
    /// Personal emails
    pub personal: String,
    /// Promotional content
    pub promotional: String,
    /// Urgent emails requiring quick response
    pub urgent: String,
    /// Newsletter content
    pub newsletter: String,
    /// System notifications
    pub notification: String,
    /// Emails that need manual review
    pub needs_review: String,
}

/// Test label configuration
#[derive(Debug, Clone)]
pub struct TestLabels {
    /// Prefix for all test labels
    pub prefix: String,
}

impl Default for ProductionLabels {
    fn default() -> Self {
        Self {
            action_required: "Agentic/Action Required".to_string(),
            interesting_info: "Agentic/Interesting".to_string(),
            reference: "Agentic/Reference".to_string(),
            noise: "Agentic/Low Priority".to_string(),
            spam: "Agentic/Spam".to_string(),
            work: "Agentic/Work".to_string(),
            personal: "Agentic/Personal".to_string(),
            promotional: "Agentic/Promotional".to_string(),
            urgent: "Agentic/Urgent".to_string(),
            newsletter: "Agentic/Newsletter".to_string(),
            notification: "Agentic/Notification".to_string(),
            needs_review: "Agentic/Needs Review".to_string(),
        }
    }
}

impl Default for TestLabels {
    fn default() -> Self {
        Self {
            prefix: "TEST_".to_string(),
        }
    }
}

impl LabelConfig {
    /// Create a new label configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Get production label by category name
    pub fn get_production_label(&self, category: &str) -> Option<String> {
        match category.to_lowercase().as_str() {
            "actionrequired" | "action_required" => Some(self.production.action_required.clone()),
            "interestinginfo" | "interesting_info" => {
                Some(self.production.interesting_info.clone())
            }
            "reference" => Some(self.production.reference.clone()),
            "noise" => Some(self.production.noise.clone()),
            "spam" => Some(self.production.spam.clone()),
            "work" => Some(self.production.work.clone()),
            "personal" => Some(self.production.personal.clone()),
            "promotional" => Some(self.production.promotional.clone()),
            "urgent" => Some(self.production.urgent.clone()),
            "newsletter" => Some(self.production.newsletter.clone()),
            "notification" => Some(self.production.notification.clone()),
            "needs_review" | "needsreview" => Some(self.production.needs_review.clone()),
            _ => None,
        }
    }

    /// Get test label by category name
    pub fn get_test_label(&self, category: &str) -> String {
        if let Some(production_label) = self.get_production_label(category) {
            format!("{}{}", self.test.prefix, production_label)
        } else {
            format!("{}{}", self.test.prefix, category)
        }
    }

    /// Get all production labels as a vector
    pub fn get_all_production_labels(&self) -> Vec<String> {
        vec![
            self.production.action_required.clone(),
            self.production.interesting_info.clone(),
            self.production.reference.clone(),
            self.production.noise.clone(),
            self.production.spam.clone(),
            self.production.work.clone(),
            self.production.personal.clone(),
            self.production.promotional.clone(),
            self.production.urgent.clone(),
            self.production.newsletter.clone(),
            self.production.notification.clone(),
            self.production.needs_review.clone(),
        ]
    }

    /// Get category to label mapping
    pub fn get_category_mappings(&self) -> HashMap<String, String> {
        let mut mappings = HashMap::new();
        mappings.insert(
            "ActionRequired".to_string(),
            self.production.action_required.clone(),
        );
        mappings.insert(
            "InterestingInfo".to_string(),
            self.production.interesting_info.clone(),
        );
        mappings.insert("Reference".to_string(), self.production.reference.clone());
        mappings.insert("Noise".to_string(), self.production.noise.clone());
        mappings.insert("Spam".to_string(), self.production.spam.clone());
        mappings.insert("Work".to_string(), self.production.work.clone());
        mappings.insert("Personal".to_string(), self.production.personal.clone());
        mappings.insert(
            "Promotional".to_string(),
            self.production.promotional.clone(),
        );
        mappings.insert("Urgent".to_string(), self.production.urgent.clone());
        mappings.insert("Newsletter".to_string(), self.production.newsletter.clone());
        mappings.insert(
            "Notification".to_string(),
            self.production.notification.clone(),
        );
        mappings.insert(
            "NeedsReview".to_string(),
            self.production.needs_review.clone(),
        );
        mappings
    }

    /// Check if a label is a test label
    pub fn is_test_label(&self, label: &str) -> bool {
        label.starts_with(&self.test.prefix)
    }

    /// Get all test labels matching the prefix
    pub fn get_test_labels_with_prefix(&self, prefix: &str) -> Vec<String> {
        let categories = [
            "ActionRequired",
            "InterestingInfo",
            "Reference",
            "Noise",
            "Spam",
        ];
        categories
            .iter()
            .map(|category| format!("{prefix}{category}"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_labels_are_hierarchical() {
        let config = LabelConfig::default();

        // Check that production labels use hierarchical structure
        assert_eq!(config.production.action_required, "Agentic/Action Required");
        assert_eq!(config.production.interesting_info, "Agentic/Interesting");
        assert_eq!(config.production.reference, "Agentic/Reference");
        assert_eq!(config.production.noise, "Agentic/Low Priority");
        assert_eq!(config.production.spam, "Agentic/Spam");

        // Check that they all use the Agentic parent label
        assert!(config.production.action_required.starts_with("Agentic/"));
        assert!(config.production.work.starts_with("Agentic/"));
        assert!(config.production.personal.starts_with("Agentic/"));
    }

    #[test]
    fn test_get_production_label_by_category() {
        let config = LabelConfig::default();

        assert_eq!(
            config.get_production_label("ActionRequired"),
            Some("Agentic/Action Required".to_string())
        );
        assert_eq!(
            config.get_production_label("actionrequired"),
            Some("Agentic/Action Required".to_string())
        );
        assert_eq!(
            config.get_production_label("action_required"),
            Some("Agentic/Action Required".to_string())
        );
        assert_eq!(config.get_production_label("unknown"), None);
    }

    #[test]
    fn test_get_test_label() {
        let config = LabelConfig::default();

        assert_eq!(
            config.get_test_label("ActionRequired"),
            "TEST_Agentic/Action Required"
        );
        assert_eq!(config.get_test_label("unknown"), "TEST_unknown");
    }

    #[test]
    fn test_is_test_label() {
        let config = LabelConfig::default();

        assert!(config.is_test_label("TEST_Agentic/Action Required"));
        assert!(config.is_test_label("TEST_anything"));
        assert!(!config.is_test_label("Agentic/Action Required"));
        assert!(!config.is_test_label("AGENT_WORK"));
    }

    #[test]
    fn test_get_all_production_labels() {
        let config = LabelConfig::default();
        let labels = config.get_all_production_labels();

        assert_eq!(labels.len(), 12);
        assert!(labels.contains(&"Agentic/Action Required".to_string()));
        assert!(labels.contains(&"Agentic/Interesting".to_string()));
        assert!(labels.contains(&"Agentic/Work".to_string()));
    }

    #[test]
    fn test_category_mappings() {
        let config = LabelConfig::default();
        let mappings = config.get_category_mappings();

        assert_eq!(
            mappings.get("ActionRequired"),
            Some(&"Agentic/Action Required".to_string())
        );
        assert_eq!(
            mappings.get("InterestingInfo"),
            Some(&"Agentic/Interesting".to_string())
        );
        assert_eq!(
            mappings.get("Noise"),
            Some(&"Agentic/Low Priority".to_string())
        );
    }
}
