//! Action router for deciding what actions to take on classified emails.
//! 
//! This module provides an abstraction for routing emails to different actions
//! based on their classification results. Actions can include labeling, archiving,
//! forwarding, escalating, or custom processing.

mod rule_based;

pub use rule_based::{RuleBasedRouter, RoutingConfig};

use crate::email::Email;
use crate::classifier::Classification;

/// Represents an action that can be performed on an email.
/// 
/// This enum defines the possible actions that the agentic system can take
/// on an email after classification. Each action contains the necessary
/// parameters to execute the action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailAction {
    /// Apply a label to the email
    Label {
        /// The label to apply (e.g., "work", "urgent", "personal")
        label: String,
    },
    
    /// Archive the email (remove from inbox)
    Archive,
    
    /// Mark the email as important/starred
    MarkImportant,
    
    /// Mark the email as read
    MarkAsRead,
    
    /// Forward the email to another address
    Forward {
        /// Email address to forward to
        to: String,
        /// Optional note to add when forwarding
        note: Option<String>,
    },
    
    /// Escalate the email (e.g., notify human, add to urgent queue)
    Escalate {
        /// Reason for escalation
        reason: String,
        /// Priority level (1-5, where 5 is highest)
        priority: u8,
    },
    
    /// Move email to a specific folder/mailbox
    MoveTo {
        /// Target folder name
        folder: String,
    },
    
    /// No action needed (leave as-is)
    NoAction,
    
    /// Custom action with arbitrary parameters
    Custom {
        /// Action name/type
        action_type: String,
        /// Action parameters as key-value pairs
        parameters: std::collections::HashMap<String, String>,
    },
}

impl EmailAction {
    /// Create a new label action.
    pub fn label(label: impl Into<String>) -> Self {
        Self::Label { label: label.into() }
    }
    
    /// Create a new forward action.
    pub fn forward(to: impl Into<String>, note: Option<String>) -> Self {
        Self::Forward { to: to.into(), note }
    }
    
    /// Create a new escalate action.
    pub fn escalate(reason: impl Into<String>, priority: u8) -> Self {
        Self::Escalate { 
            reason: reason.into(), 
            priority: priority.clamp(1, 5)
        }
    }
    
    /// Create a new move action.
    pub fn move_to(folder: impl Into<String>) -> Self {
        Self::MoveTo { folder: folder.into() }
    }
    
    /// Create a new custom action.
    pub fn custom(action_type: impl Into<String>, parameters: std::collections::HashMap<String, String>) -> Self {
        Self::Custom { 
            action_type: action_type.into(), 
            parameters 
        }
    }
    
    /// Check if this action is a high-priority action that requires immediate attention.
    pub fn is_high_priority(&self) -> bool {
        match self {
            Self::Escalate { priority, .. } => *priority >= 4,
            Self::MarkImportant => true,
            Self::Forward { .. } => true,
            _ => false,
        }
    }
    
    /// Get a human-readable description of this action.
    pub fn description(&self) -> String {
        match self {
            Self::Label { label } => format!("Apply label '{}'", label),
            Self::Archive => "Archive email".to_string(),
            Self::MarkImportant => "Mark as important".to_string(),
            Self::MarkAsRead => "Mark as read".to_string(),
            Self::Forward { to, note } => {
                if let Some(note) = note {
                    format!("Forward to {} with note: {}", to, note)
                } else {
                    format!("Forward to {}", to)
                }
            },
            Self::Escalate { reason, priority } => {
                format!("Escalate (priority {}): {}", priority, reason)
            },
            Self::MoveTo { folder } => format!("Move to folder '{}'", folder),
            Self::NoAction => "No action needed".to_string(),
            Self::Custom { action_type, parameters } => {
                format!("Custom action '{}' with {} parameters", action_type, parameters.len())
            },
        }
    }
}

/// Result of routing an email to actions.
/// 
/// Contains the list of actions to perform and audit information.
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// List of actions to perform on the email
    pub actions: Vec<EmailAction>,
    /// Reasoning for why these actions were chosen
    pub reasoning: String,
    /// Confidence in the routing decision (0.0 to 1.0)
    pub confidence: f32,
}

impl RoutingResult {
    /// Create a new routing result.
    pub fn new(actions: Vec<EmailAction>, reasoning: String, confidence: f32) -> Self {
        Self {
            actions,
            reasoning,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
    
    /// Create a routing result with a single action.
    pub fn single_action(action: EmailAction, reasoning: String) -> Self {
        Self::new(vec![action], reasoning, 1.0)
    }
    
    /// Create a routing result with no actions.
    pub fn no_action(reasoning: String) -> Self {
        Self::new(vec![EmailAction::NoAction], reasoning, 1.0)
    }
    
    /// Check if any of the actions are high priority.
    pub fn has_high_priority_actions(&self) -> bool {
        self.actions.iter().any(|action| action.is_high_priority())
    }
    
    /// Get a summary description of all actions.
    pub fn actions_summary(&self) -> String {
        if self.actions.is_empty() {
            "No actions".to_string()
        } else if self.actions.len() == 1 {
            self.actions[0].description()
        } else {
            let descriptions: Vec<String> = self.actions.iter()
                .map(|action| action.description())
                .collect();
            format!("{} actions: {}", self.actions.len(), descriptions.join(", "))
        }
    }
}

/// Errors that can occur during action routing.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum RoutingError {
    /// Invalid classification input
    #[error("Invalid classification: {message}")]
    InvalidClassification { message: String },
    
    /// Configuration error for the router
    #[error("Router configuration error: {message}")]
    Config { message: String },
    
    /// Action routing logic error
    #[error("Routing logic error: {message}")]
    Logic { message: String },
    
    /// Unknown or unexpected error
    #[error("Unknown routing error: {message}")]
    Unknown { message: String },
}

impl RoutingError {
    /// Create a new invalid classification error.
    pub fn invalid_classification(message: impl Into<String>) -> Self {
        Self::InvalidClassification { message: message.into() }
    }
    
    /// Create a new config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    /// Create a new logic error.
    pub fn logic(message: impl Into<String>) -> Self {
        Self::Logic { message: message.into() }
    }
    
    /// Create a new unknown error.
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown { message: message.into() }
    }
}

/// Trait for routing emails to actions based on their classification.
/// 
/// This trait provides a unified interface for determining what actions
/// to take on an email based on its classification result. Different
/// implementations can provide rule-based routing, LLM-powered routing,
/// or hybrid approaches.
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use agentic_mail_agent::action_router::{ActionRouter, RuleBasedRouter};
/// use agentic_mail_agent::classifier::Classification;
/// use agentic_mail_agent::email::Email;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let router = RuleBasedRouter::new();
///     let email = Email::with_subject("test@example.com".to_string(), "Meeting reminder".to_string());
///     let classification = Classification::with_category("work".to_string());
///     let result = router.route(&email, &classification).await?;
///     println!("Actions: {}", result.actions_summary());
///     Ok(())
/// }
/// ```
#[async_trait::async_trait]
pub trait ActionRouter {
    /// Route an email to appropriate actions based on its classification.
    /// 
    /// This method analyzes the email and its classification to determine
    /// what actions should be taken. The routing logic can be simple
    /// rule-based or sophisticated AI-powered decision making.
    /// 
    /// # Arguments
    /// 
    /// * `email` - The email to route
    /// * `classification` - The classification result for the email
    /// 
    /// # Returns
    /// 
    /// Returns a `RoutingResult` containing the actions to take and reasoning,
    /// or a `RoutingError` if routing fails.
    async fn route(&self, email: &Email, classification: &Classification) -> Result<RoutingResult, RoutingError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_action_creation() {
        let label_action = EmailAction::label("work");
        assert!(matches!(label_action, EmailAction::Label { .. }));
        
        let forward_action = EmailAction::forward("admin@example.com", Some("FYI".to_string()));
        assert!(matches!(forward_action, EmailAction::Forward { .. }));
        
        let escalate_action = EmailAction::escalate("Suspicious content", 5);
        assert!(matches!(escalate_action, EmailAction::Escalate { priority: 5, .. }));
        
        let move_action = EmailAction::move_to("important");
        assert!(matches!(move_action, EmailAction::MoveTo { .. }));
    }
    
    #[test]
    fn test_email_action_priority() {
        assert!(EmailAction::escalate("urgent", 5).is_high_priority());
        assert!(EmailAction::escalate("medium", 4).is_high_priority());
        assert!(!EmailAction::escalate("low", 3).is_high_priority());
        assert!(EmailAction::MarkImportant.is_high_priority());
        assert!(!EmailAction::Archive.is_high_priority());
        assert!(!EmailAction::NoAction.is_high_priority());
    }
    
    #[test]
    fn test_email_action_description() {
        let action = EmailAction::label("spam");
        assert_eq!(action.description(), "Apply label 'spam'");
        
        let action = EmailAction::Archive;
        assert_eq!(action.description(), "Archive email");
        
        let action = EmailAction::forward("admin@example.com", None);
        assert_eq!(action.description(), "Forward to admin@example.com");
    }
    
    #[test]
    fn test_routing_result_creation() {
        let actions = vec![EmailAction::label("work"), EmailAction::MarkAsRead];
        let result = RoutingResult::new(actions, "Work email detected".to_string(), 0.85);
        
        assert_eq!(result.actions.len(), 2);
        assert_eq!(result.reasoning, "Work email detected");
        assert_eq!(result.confidence, 0.85);
        assert!(!result.has_high_priority_actions());
    }
    
    #[test]
    fn test_routing_result_high_priority() {
        let result = RoutingResult::single_action(
            EmailAction::escalate("Security alert", 5),
            "Suspicious activity detected".to_string()
        );
        
        assert!(result.has_high_priority_actions());
    }
    
    #[test]
    fn test_routing_result_summary() {
        let result = RoutingResult::no_action("Email already processed".to_string());
        assert_eq!(result.actions_summary(), "No action needed");
        
        let actions = vec![EmailAction::label("personal"), EmailAction::Archive];
        let result = RoutingResult::new(actions, "Personal email".to_string(), 0.9);
        assert!(result.actions_summary().contains("2 actions"));
    }
    
    #[test]
    fn test_routing_error_creation() {
        let error = RoutingError::invalid_classification("Missing category");
        assert!(matches!(error, RoutingError::InvalidClassification { .. }));
        
        let error = RoutingError::config("Missing router config");
        assert!(matches!(error, RoutingError::Config { .. }));
    }
}
