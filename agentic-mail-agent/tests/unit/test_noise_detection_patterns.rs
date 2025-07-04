//! Tests for Noise category detection patterns using TDD approach
//! 
//! This test suite validates that the classifier correctly identifies various
//! types of "Noise" emails (marketing, social, promotional content) that should
//! not clutter the inbox but aren't actionable or particularly interesting.

use agentic_mail_agent::classifier::{MessageClassifier, StubClassifier};
use agentic_mail_agent::email::Email;

/// Test marketing domain patterns should be classified as Noise
#[tokio::test]
async fn test_marketing_domains_classified_as_noise() {
    let classifier = StubClassifier::deterministic();
    
    // Test various marketing/promotional domains
    let marketing_emails = vec![
        // Generic marketing domains
        Email::new_full(
            "test1".to_string(),
            Some("Special Offer Inside!".to_string()),
            Some("Limited time deal on our products".to_string()),
            Some("noreply@company.com".to_string()),
            None, None, None,
        ),
        // Mailchimp/SendGrid marketing
        Email::new_full(
            "test2".to_string(),
            Some("Newsletter Update".to_string()),
            Some("Check out our latest products and offers".to_string()),
            Some("marketing@mailchimp.com".to_string()),
            None, None, None,
        ),
        // Email marketing services
        Email::new_full(
            "test3".to_string(),
            Some("Monthly Newsletter".to_string()),
            Some("Subscribe to our newsletter for deals".to_string()),
            Some("updates@sendgrid.net".to_string()),
            None, None, None,
        ),
        // Marketing subdomain
        Email::new_full(
            "test4".to_string(),
            Some("Product Announcement".to_string()),
            Some("New products available now".to_string()),
            Some("info@marketing.company.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in marketing_emails {
        let classification = classifier.classify(&email).await.unwrap();
        assert_eq!(
            classification.category, 
            "Noise",
            "Email from {} with subject '{}' should be classified as Noise",
            email.from.as_deref().unwrap_or("unknown"),
            email.subject.as_deref().unwrap_or("unknown")
        );
    }
}

/// Test promotional phrases should be classified as Noise
#[tokio::test]
async fn test_promotional_phrases_classified_as_noise() {
    let classifier = StubClassifier::deterministic();
    
    let promotional_emails = vec![
        // Limited time offers
        Email::new_full(
            "test1".to_string(),
            Some("Limited Time: 50% Off Everything!".to_string()),
            Some("Don't miss this exclusive limited time offer".to_string()),
            Some("sales@store.com".to_string()),
            None, None, None,
        ),
        // Exclusive deals
        Email::new_full(
            "test2".to_string(),
            Some("Exclusive Deal Just for You".to_string()),
            Some("Special exclusive offer for VIP customers".to_string()),
            Some("vip@retailer.com".to_string()),
            None, None, None,
        ),
        // Discount promotions
        Email::new_full(
            "test3".to_string(),
            Some("Huge Discount on Popular Items".to_string()),
            Some("Get discount on selected items while supplies last".to_string()),
            Some("deals@shop.com".to_string()),
            None, None, None,
        ),
        // Sale announcements
        Email::new_full(
            "test4".to_string(),
            Some("Flash Sale - 24 Hours Only!".to_string()),
            Some("Flash sale ending soon, shop now or miss out".to_string()),
            Some("flash@ecommerce.com".to_string()),
            None, None, None,
        ),
        // Unsubscribe links (common in marketing)
        Email::new_full(
            "test5".to_string(),
            Some("Weekly Deals Newsletter".to_string()),
            Some("Great deals this week. Click here to unsubscribe if not interested".to_string()),
            Some("newsletter@deals.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in promotional_emails {
        let classification = classifier.classify(&email).await.unwrap();
        assert_eq!(
            classification.category,
            "Noise", 
            "Promotional email with subject '{}' should be classified as Noise",
            email.subject.as_deref().unwrap_or("unknown")
        );
    }
}

/// Test social media platform notifications should be classified as Noise
#[tokio::test]
async fn test_social_media_notifications_classified_as_noise() {
    let classifier = StubClassifier::deterministic();
    
    let social_emails = vec![
        // LinkedIn notifications
        Email::new_full(
            "test1".to_string(),
            Some("John Doe is now following you".to_string()),
            Some("John Doe started following you on LinkedIn".to_string()),
            Some("notifications@linkedin.com".to_string()),
            None, None, None,
        ),
        // Facebook notifications
        Email::new_full(
            "test2".to_string(),
            Some("You have 5 new notifications".to_string()),
            Some("Check your Facebook notifications".to_string()),
            Some("notification@facebook.com".to_string()),
            None, None, None,
        ),
        // Twitter/X notifications
        Email::new_full(
            "test3".to_string(),
            Some("New followers and mentions".to_string()),
            Some("You have new activity on X".to_string()),
            Some("notify@twitter.com".to_string()),
            None, None, None,
        ),
        // Instagram notifications
        Email::new_full(
            "test4".to_string(),
            Some("Someone liked your photo".to_string()),
            Some("Activity on your Instagram account".to_string()),
            Some("no-reply@instagram.com".to_string()),
            None, None, None,
        ),
        // Generic social connection suggestions
        Email::new_full(
            "test5".to_string(),
            Some("People you may know".to_string()),
            Some("Connect with people in your network".to_string()),
            Some("suggestions@social.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in social_emails {
        let classification = classifier.classify(&email).await.unwrap();
        assert_eq!(
            classification.category,
            "Noise",
            "Social media email from {} with subject '{}' should be classified as Noise",
            email.from.as_deref().unwrap_or("unknown"),
            email.subject.as_deref().unwrap_or("unknown")
        );
    }
}

/// Test product recommendation emails should be classified as Noise
#[tokio::test]
async fn test_product_recommendations_classified_as_noise() {
    let classifier = StubClassifier::deterministic();
    
    let recommendation_emails = vec![
        // Generic product recommendations
        Email::new_full(
            "test1".to_string(),
            Some("Products you might like".to_string()),
            Some("Based on your browsing history, here are some recommendations".to_string()),
            Some("recommendations@store.com".to_string()),
            None, None, None,
        ),
        // "To pair with" suggestions
        Email::new_full(
            "test2".to_string(),
            Some("To pair with what you purchased".to_string()),
            Some("These items go great with your recent purchase".to_string()),
            Some("suggestions@retailer.com".to_string()),
            None, None, None,
        ),
        // Wishlist reminders
        Email::new_full(
            "test3".to_string(),
            Some("Items in your wishlist are on sale".to_string()),
            Some("Don't miss out on wishlist items now on sale".to_string()),
            Some("wishlist@shop.com".to_string()),
            None, None, None,
        ),
        // Cross-sell attempts
        Email::new_full(
            "test4".to_string(),
            Some("Complete your setup with these accessories".to_string()),
            Some("Enhance your experience with these complementary products".to_string()),
            Some("crosssell@electronics.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in recommendation_emails {
        let classification = classifier.classify(&email).await.unwrap();
        assert_eq!(
            classification.category,
            "Noise",
            "Product recommendation email with subject '{}' should be classified as Noise",
            email.subject.as_deref().unwrap_or("unknown")
        );
    }
}

/// Test generic newsletter content should be classified as Noise (unless tech/interesting)
#[tokio::test]
async fn test_generic_newsletters_classified_as_noise() {
    let classifier = StubClassifier::deterministic();
    
    let newsletter_emails = vec![
        // Sports newsletters
        Email::new_full(
            "test1".to_string(),
            Some("Weekly Sports Roundup".to_string()),
            Some("This week in sports - scores and highlights".to_string()),
            Some("sports@newsletter.com".to_string()),
            None, None, None,
        ),
        // Entertainment newsletters  
        Email::new_full(
            "test2".to_string(),
            Some("Celebrity News Weekly".to_string()),
            Some("Latest celebrity gossip and entertainment news".to_string()),
            Some("celeb@entertainment.com".to_string()),
            None, None, None,
        ),
        // Lifestyle newsletters
        Email::new_full(
            "test3".to_string(),
            Some("Healthy Living Tips".to_string()),
            Some("Weekly tips for a healthier lifestyle".to_string()),
            Some("health@lifestyle.com".to_string()),
            None, None, None,
        ),
        // Generic company updates
        Email::new_full(
            "test4".to_string(),
            Some("Company Newsletter - March 2024".to_string()),
            Some("Monthly updates from our company".to_string()),
            Some("newsletter@company.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in newsletter_emails {
        let classification = classifier.classify(&email).await.unwrap();
        assert_eq!(
            classification.category,
            "Noise",
            "Generic newsletter with subject '{}' should be classified as Noise",
            email.subject.as_deref().unwrap_or("unknown")
        );
    }
}

/// Test location/event spam should be classified as Noise
#[tokio::test]
async fn test_location_event_spam_classified_as_noise() {
    let classifier = StubClassifier::deterministic();
    
    let location_emails = vec![
        // Location-based promotional
        Email::new_full(
            "test1".to_string(),
            Some("📍 Someone was at your location".to_string()),
            Some("Cameron Garcia was at 166 Elm Street, Georgetown, WA 10066".to_string()),
            Some("location@tracker.com".to_string()),
            None, None, None,
        ),
        // Event spam with location
        Email::new_full(
            "test2".to_string(),
            Some("Events near you this weekend".to_string()),
            Some("Find local events and activities in your area".to_string()),
            Some("events@local.com".to_string()),
            None, None, None,
        ),
        // Generic event promotions
        Email::new_full(
            "test3".to_string(),
            Some("Concert tickets available now".to_string()),
            Some("Get tickets for upcoming concerts and shows".to_string()),
            Some("tickets@events.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in location_emails {
        let classification = classifier.classify(&email).await.unwrap();
        assert_eq!(
            classification.category,
            "Noise",
            "Location/event email with subject '{}' should be classified as Noise",
            email.subject.as_deref().unwrap_or("unknown")
        );
    }
}

/// Test that tech newsletters should still be InterestingInfo (not Noise)
#[tokio::test]
async fn test_tech_newsletters_remain_interesting_info() {
    let classifier = StubClassifier::deterministic();
    
    let tech_emails = vec![
        // Tech newsletters should be InterestingInfo
        Email::new_full(
            "test1".to_string(),
            Some("Weekly Tech Newsletter".to_string()),
            Some("Latest tech news and startup updates".to_string()),
            Some("tech@newsletter.com".to_string()),
            None, None, None,
        ),
        // AI/ML newsletters
        Email::new_full(
            "test2".to_string(),
            Some("AI Newsletter Digest".to_string()),
            Some("This week in artificial intelligence and machine learning".to_string()),
            Some("ai@techdigest.com".to_string()),
            None, None, None,
        ),
    ];
    
    for email in tech_emails {
        let classification = classifier.classify(&email).await.unwrap();
        assert_eq!(
            classification.category,
            "InterestingInfo",
            "Tech newsletter with subject '{}' should remain InterestingInfo",
            email.subject.as_deref().unwrap_or("unknown")
        );
    }
}

/// Test that security-related content remains InterestingInfo (not Noise)
#[tokio::test]
async fn test_security_content_remains_interesting_info() {
    let classifier = StubClassifier::deterministic();
    
    let security_email = Email::new_full(
        "test1".to_string(),
        Some("Security Alert: Unusual Activity".to_string()),
        Some("We detected unusual activity on your account".to_string()),
        Some("security@service.com".to_string()),
        None, None, None,
    );
    
    let classification = classifier.classify(&security_email).await.unwrap();
    assert_eq!(
        classification.category,
        "InterestingInfo",
        "Security alerts should remain InterestingInfo, not be classified as Noise"
    );
}