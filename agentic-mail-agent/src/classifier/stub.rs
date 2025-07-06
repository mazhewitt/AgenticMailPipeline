//! Stub implementation of MessageClassifier for testing and development.

use async_trait::async_trait;
use rand::Rng;

use super::text_preprocessing::prepare_email_for_classification;
use super::{Classification, ClassificationError, EmailCategory, MessageClassifier};
use crate::core::email::Email;

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
        let categories = [
            EmailCategory::ActionRequired,
            EmailCategory::InterestingInfo,
            EmailCategory::Reference,
            EmailCategory::Noise,
            EmailCategory::Spam,
        ];
        let mut rng = rand::rng();

        let category = categories[rng.random_range(0..categories.len())];
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
            100, // Limit for deterministic processing
        )
        .to_lowercase();

        // Extract sender domain for additional classification hints
        let sender_domain = email
            .from
            .as_deref()
            .and_then(|from| from.split('@').nth(1))
            .unwrap_or("")
            .to_lowercase();

        let (category, score) = if
        // ActionRequired patterns
        cleaned_content.contains("ci")
            && cleaned_content.contains("failed")
            || cleaned_content.contains("meeting")
                && (cleaned_content.contains("tomorrow") || cleaned_content.contains("reminder"))
            || cleaned_content.contains("urgent")
            || cleaned_content.contains("asap")
            || cleaned_content.contains("action required")
            || cleaned_content.contains("deadline")
            || cleaned_content.contains("due")
            || cleaned_content.contains("transfer") && cleaned_content.contains("ticket")
            || cleaned_content.contains("schule")
            || cleaned_content.contains("school")
        // German school emails
        {
            (EmailCategory::ActionRequired, 0.9)
        } else if
        // InterestingInfo patterns - must come before general newsletter patterns
        // Check original content for newsletter patterns since "newsletter" is filtered out during preprocessing
        (email
            .subject
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("newsletter")
            || email
                .snippet
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains("newsletter")
            || cleaned_content.contains("digest"))
            && (cleaned_content.contains("tech")
                || cleaned_content.contains("ai")
                || cleaned_content.contains("news"))
            || cleaned_content.contains("tech")
                && (email
                    .subject
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains("newsletter")
                    || email
                        .snippet
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains("newsletter"))
            || cleaned_content.contains("security") && cleaned_content.contains("alert")
            || cleaned_content.contains("economics")
            || cleaned_content.contains("financial")
            || cleaned_content.contains("scam") && cleaned_content.contains("protect")
            || cleaned_content.contains("new login")
            || cleaned_content.contains("login") && cleaned_content.contains("device")
            || sender_domain.contains("nytimes")
            || sender_domain.contains("anthropic") && cleaned_content.contains("update")
        {
            (EmailCategory::InterestingInfo, 0.85)
        } else if
        // Reference patterns
        cleaned_content.contains("receipt")
            || cleaned_content.contains("invoice")
            || cleaned_content.contains("confirmation")
            || cleaned_content.contains("delivered")
            || cleaned_content.contains("consignment")
            || cleaned_content.contains("shipping")
            || cleaned_content.contains("terms") && cleaned_content.contains("conditions")
            || cleaned_content.contains("welcome") && cleaned_content.contains("plan")
            || cleaned_content.contains("login") && cleaned_content.contains("secure")
            || cleaned_content.contains("bbq")
            || cleaned_content.contains("tomorrow") && !cleaned_content.contains("meeting")
        // Personal conversations
        {
            (EmailCategory::Reference, 0.8)
        } else if
        // Spam patterns
        cleaned_content.contains("lottery")
            || cleaned_content.contains("won") && cleaned_content.contains("million")
            || cleaned_content.contains("click here") && cleaned_content.contains("claim")
            || cleaned_content.contains("suspicious") && cleaned_content.contains("offer")
        {
            (EmailCategory::Spam, 0.95)
        } else if
        // Noise patterns - Marketing and promotional content

        // Marketing domains and senders
        sender_domain.contains("noreply") || sender_domain.contains("no-reply") ||
            sender_domain.contains("marketing") || sender_domain.contains("mailchimp") || 
            sender_domain.contains("sendgrid") || sender_domain.contains("constantcontact") ||
            sender_domain.contains("mailgun") || sender_domain.contains("campaign") ||

            // Social media platforms and notifications
            sender_domain.contains("facebook") || sender_domain.contains("linkedin") ||
            sender_domain.contains("twitter") || sender_domain.contains("instagram") ||
            sender_domain.contains("social") || sender_domain.contains("notifications") ||

            // E-commerce and retail promotional domains  
            sender_domain.contains("aliexpress") || sender_domain.contains("nespresso") ||
            sender_domain.contains("shopify") || sender_domain.contains("etsy") ||

            // Promotional phrase patterns
            cleaned_content.contains("limited time") || cleaned_content.contains("exclusive") ||
            cleaned_content.contains("deal") || cleaned_content.contains("discount") ||
            cleaned_content.contains("offer") || cleaned_content.contains("sale") ||
            cleaned_content.contains("flash sale") || cleaned_content.contains("special offer") ||
            cleaned_content.contains("promotion") || cleaned_content.contains("promotional") ||

            // Social media engagement patterns
            cleaned_content.contains("follow") && (cleaned_content.contains("linkedin") || cleaned_content.contains("ceo")) ||
            cleaned_content.contains("notification") && (cleaned_content.contains("facebook") || cleaned_content.contains("social")) ||
            cleaned_content.contains("posted") && cleaned_content.contains("update") ||
            cleaned_content.contains("connection") || cleaned_content.contains("add") && cleaned_content.contains("contact") ||
            cleaned_content.contains("people you may know") || cleaned_content.contains("suggested") ||
            cleaned_content.contains("someone liked") || cleaned_content.contains("new followers") ||

            // Product recommendation patterns
            cleaned_content.contains("to pair with") || cleaned_content.contains("you might like") ||
            cleaned_content.contains("recommended for you") || cleaned_content.contains("based on your") ||
            cleaned_content.contains("complete your") || cleaned_content.contains("accessories") ||
            cleaned_content.contains("wishlist") && cleaned_content.contains("sale") ||

            // Newsletter patterns (excluding tech/security/AI newsletters)
            // Check original content for newsletter patterns since "newsletter" is filtered out during preprocessing
            (email.subject.as_deref().unwrap_or("").to_lowercase().contains("newsletter") || 
             email.snippet.as_deref().unwrap_or("").to_lowercase().contains("newsletter") ||
             sender_domain.contains("newsletter")) && 
            !cleaned_content.contains("tech") && !cleaned_content.contains("ai") && !cleaned_content.contains("security") ||
            cleaned_content.contains("weekly digest") && !cleaned_content.contains("tech") && !cleaned_content.contains("ai") ||
            cleaned_content.contains("monthly update") && !cleaned_content.contains("security") && !cleaned_content.contains("tech") ||
            // Generic newsletter-like patterns
            cleaned_content.contains("weekly") && (cleaned_content.contains("roundup") || cleaned_content.contains("digest") || cleaned_content.contains("sports") || cleaned_content.contains("entertainment") || cleaned_content.contains("celebrity") || cleaned_content.contains("tips")) ||
            cleaned_content.contains("monthly") && (cleaned_content.contains("roundup") || cleaned_content.contains("digest") || cleaned_content.contains("updates")) ||
            // Lifestyle and wellness content
            cleaned_content.contains("healthy living") || cleaned_content.contains("lifestyle") && cleaned_content.contains("tips") ||
            cleaned_content.contains("wellness") || cleaned_content.contains("health") && (cleaned_content.contains("tips") || cleaned_content.contains("advice")) ||

            // Generic promotional language
            cleaned_content.contains("marketing") || cleaned_content.contains("subscribe") ||
            cleaned_content.contains("shop now") || cleaned_content.contains("buy now") ||
            cleaned_content.contains("while supplies last") || cleaned_content.contains("hurry") ||
            cleaned_content.contains("don't miss") || cleaned_content.contains("act now") ||

            // Location-based promotional patterns
            cleaned_content.contains("events near you") || cleaned_content.contains("in your area") ||
            cleaned_content.contains("local events") || cleaned_content.contains("near your location") ||
            cleaned_content.contains("at your location") || cleaned_content.contains("someone was at") ||
            // Event and ticket promotions
            cleaned_content.contains("concert tickets") || cleaned_content.contains("tickets available") ||
            cleaned_content.contains("tickets") && (cleaned_content.contains("concerts") || cleaned_content.contains("shows") || cleaned_content.contains("events")) ||
            cleaned_content.contains("get tickets") || cleaned_content.contains("upcoming concerts") ||

            // Check original subject/snippet for unsubscribe since it gets filtered out in preprocessing
            email.subject.as_deref().unwrap_or("").to_lowercase().contains("unsubscribe") ||
            email.snippet.as_deref().unwrap_or("").to_lowercase().contains("unsubscribe")
        {
            (EmailCategory::Noise, 0.85)
        } else {
            (EmailCategory::Reference, 0.6) // Default to Reference as it's the safest default
        };

        Classification::new(
            category,
            Some(score),
            format!(
                "Deterministic classification based on cleaned content and patterns: {}",
                category.as_str()
            ),
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

        assert!(classification.score.is_some());
        assert!(!classification.llm_response.is_empty());
    }

    #[tokio::test]
    async fn stub_classifier_with_fixed_classification() {
        let fixed_classification = Classification::with_score(EmailCategory::ActionRequired, 0.95);
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
        let work_email = Email::with_subject(
            "work@example.com".to_string(),
            "Meeting tomorrow".to_string(),
        );
        let result = classifier.classify(&work_email).await.unwrap();
        assert_eq!(result.category, EmailCategory::ActionRequired); // Meeting tomorrow should be ActionRequired
        assert_eq!(result.score, Some(0.9));

        // Test ActionRequired classification
        let urgent_email = Email::with_subject(
            "urgent@example.com".to_string(),
            "URGENT: Action required".to_string(),
        );
        let result = classifier.classify(&urgent_email).await.unwrap();
        assert_eq!(result.category, EmailCategory::ActionRequired);
        assert_eq!(result.score, Some(0.9));

        // Test Noise classification (promotional content)
        let promo_email = Email::with_subject(
            "promo@example.com".to_string(),
            "Unsubscribe from our newsletter".to_string(),
        );
        let result = classifier.classify(&promo_email).await.unwrap();
        assert_eq!(result.category, EmailCategory::Noise);
        assert_eq!(result.score, Some(0.85));

        // Test Reference classification (receipt)
        let receipt_email = Email::with_subject(
            "receipt@example.com".to_string(),
            "Your receipt from Company".to_string(),
        );
        let result = classifier.classify(&receipt_email).await.unwrap();
        assert_eq!(result.category, EmailCategory::Reference);
        assert_eq!(result.score, Some(0.8));

        // Test InterestingInfo classification (newsletter with tech content)
        let newsletter_email = Email::with_subject(
            "news@example.com".to_string(),
            "Tech newsletter digest".to_string(),
        );
        let result = classifier.classify(&newsletter_email).await.unwrap();
        assert_eq!(result.category, EmailCategory::InterestingInfo);
        assert_eq!(result.score, Some(0.85));

        // Test default classification
        let default_email = Email::with_subject(
            "unknown@example.com".to_string(),
            "Random content".to_string(),
        );
        let result = classifier.classify(&default_email).await.unwrap();
        assert_eq!(result.category, EmailCategory::Reference);
        assert_eq!(result.score, Some(0.6));
    }

    #[tokio::test]
    async fn stub_classifier_handles_empty_content() {
        let classifier = StubClassifier::deterministic();
        let email = Email::new("test@example.com".to_string(), None, None);

        let result = classifier.classify(&email).await;
        assert!(result.is_ok());

        let classification = result.unwrap();
        assert_eq!(classification.category, EmailCategory::Reference); // default category
        assert_eq!(classification.score, Some(0.6));
    }

    #[tokio::test]
    async fn stub_classifier_default() {
        let classifier = StubClassifier::default();
        let email = Email::with_subject("test@example.com".to_string(), "Test email".to_string());

        let result = classifier.classify(&email).await;
        assert!(result.is_ok());

        let _classification = result.unwrap();
    }
}
