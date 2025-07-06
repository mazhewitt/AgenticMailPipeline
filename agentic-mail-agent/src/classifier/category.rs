//! Email classification categories as type-safe enums
//!
//! This module provides an enum for email classification categories
//! that can format itself for different contexts (human-friendly vs machine labels).

use std::fmt::{self, Display};

/// Email classification categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EmailCategory {
    /// Requires user action or response
    ActionRequired,
    /// Contains valuable information but no action needed
    InterestingInfo,
    /// Reference material for later
    Reference,
    /// Low priority/noise email
    Noise,
    /// Spam or unwanted email
    Spam,
}

impl EmailCategory {
    /// Get all possible categories
    pub fn all() -> Vec<EmailCategory> {
        vec![
            EmailCategory::ActionRequired,
            EmailCategory::InterestingInfo,
            EmailCategory::Reference,
            EmailCategory::Noise,
            EmailCategory::Spam,
        ]
    }

    /// Get the canonical string representation (for LLM responses)
    pub fn as_str(&self) -> &'static str {
        match self {
            EmailCategory::ActionRequired => "ActionRequired",
            EmailCategory::InterestingInfo => "InterestingInfo",
            EmailCategory::Reference => "Reference",
            EmailCategory::Noise => "Noise",
            EmailCategory::Spam => "Spam",
        }
    }

    /// Get human-friendly label for the category
    pub fn human_label(&self) -> &'static str {
        match self {
            EmailCategory::ActionRequired => "Action Required",
            EmailCategory::InterestingInfo => "Interesting",
            EmailCategory::Reference => "Reference",
            EmailCategory::Noise => "Low Priority",
            EmailCategory::Spam => "Spam",
        }
    }

    /// Get a description of what this category means
    pub fn description(&self) -> &'static str {
        match self {
            EmailCategory::ActionRequired => "Emails that require user action or response",
            EmailCategory::InterestingInfo => {
                "Emails with valuable information but no action needed"
            }
            EmailCategory::Reference => "Reference material to keep for later",
            EmailCategory::Noise => "Low priority emails or noise",
            EmailCategory::Spam => "Spam or unwanted emails",
        }
    }
}

impl Display for EmailCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for EmailCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ActionRequired" => Ok(EmailCategory::ActionRequired),
            "InterestingInfo" => Ok(EmailCategory::InterestingInfo),
            "Reference" => Ok(EmailCategory::Reference),
            "Noise" => Ok(EmailCategory::Noise),
            "Spam" => Ok(EmailCategory::Spam),
            _ => Err(format!(
                "Invalid email category '{}'. Valid categories: {}",
                s,
                EmailCategory::all()
                    .iter()
                    .map(|cat| cat.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_all_categories() {
        let categories = EmailCategory::all();
        assert_eq!(categories.len(), 5);
        assert!(categories.contains(&EmailCategory::ActionRequired));
        assert!(categories.contains(&EmailCategory::InterestingInfo));
        assert!(categories.contains(&EmailCategory::Reference));
        assert!(categories.contains(&EmailCategory::Noise));
        assert!(categories.contains(&EmailCategory::Spam));
    }

    #[test]
    fn test_category_from_string() {
        assert_eq!(
            EmailCategory::from_str("ActionRequired").unwrap(),
            EmailCategory::ActionRequired
        );
        assert_eq!(
            EmailCategory::from_str("InterestingInfo").unwrap(),
            EmailCategory::InterestingInfo
        );
        assert_eq!(
            EmailCategory::from_str("Reference").unwrap(),
            EmailCategory::Reference
        );
        assert_eq!(
            EmailCategory::from_str("Noise").unwrap(),
            EmailCategory::Noise
        );
        assert_eq!(
            EmailCategory::from_str("Spam").unwrap(),
            EmailCategory::Spam
        );
        assert!(EmailCategory::from_str("Invalid").is_err());
    }

    #[test]
    fn test_category_string_conversion() {
        assert_eq!(EmailCategory::ActionRequired.as_str(), "ActionRequired");
        assert_eq!(EmailCategory::InterestingInfo.as_str(), "InterestingInfo");
        assert_eq!(EmailCategory::Reference.as_str(), "Reference");
        assert_eq!(EmailCategory::Noise.as_str(), "Noise");
        assert_eq!(EmailCategory::Spam.as_str(), "Spam");
    }

    #[test]
    fn test_human_labels() {
        assert_eq!(
            EmailCategory::ActionRequired.human_label(),
            "Action Required"
        );
        assert_eq!(EmailCategory::InterestingInfo.human_label(), "Interesting");
        assert_eq!(EmailCategory::Reference.human_label(), "Reference");
        assert_eq!(EmailCategory::Noise.human_label(), "Low Priority");
        assert_eq!(EmailCategory::Spam.human_label(), "Spam");
    }

    #[test]
    fn test_descriptions() {
        assert!(!EmailCategory::ActionRequired.description().is_empty());
        assert!(!EmailCategory::InterestingInfo.description().is_empty());
        assert!(!EmailCategory::Reference.description().is_empty());
        assert!(!EmailCategory::Noise.description().is_empty());
        assert!(!EmailCategory::Spam.description().is_empty());
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(
            format!("{}", EmailCategory::ActionRequired),
            "ActionRequired"
        );
        assert_eq!(
            format!("{}", EmailCategory::InterestingInfo),
            "InterestingInfo"
        );
        assert_eq!(format!("{}", EmailCategory::Reference), "Reference");
        assert_eq!(format!("{}", EmailCategory::Noise), "Noise");
        assert_eq!(format!("{}", EmailCategory::Spam), "Spam");
    }

    #[test]
    fn test_fromstr_trait() {
        use std::str::FromStr;

        assert_eq!(
            EmailCategory::from_str("ActionRequired").unwrap(),
            EmailCategory::ActionRequired
        );
        assert_eq!(
            EmailCategory::from_str("InterestingInfo").unwrap(),
            EmailCategory::InterestingInfo
        );
        assert_eq!(
            EmailCategory::from_str("Reference").unwrap(),
            EmailCategory::Reference
        );
        assert_eq!(
            EmailCategory::from_str("Noise").unwrap(),
            EmailCategory::Noise
        );
        assert_eq!(
            EmailCategory::from_str("Spam").unwrap(),
            EmailCategory::Spam
        );
        assert!(EmailCategory::from_str("Invalid").is_err());
    }

    #[test]
    fn test_serde_serialization() {
        let category = EmailCategory::ActionRequired;
        let serialized = serde_json::to_string(&category).unwrap();
        let deserialized: EmailCategory = serde_json::from_str(&serialized).unwrap();
        assert_eq!(category, deserialized);
    }
}
