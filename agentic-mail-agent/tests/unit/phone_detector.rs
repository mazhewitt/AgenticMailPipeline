#[cfg(test)]
mod tests {
    use agentic_mail_agent::anonymizer::PhoneDetector;

    #[test]
    fn test_detect_us_phone_with_dashes() {
        let detector = PhoneDetector::new();
        let text = "Please call me at 555-123-4567 for more information.";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].pii_type, "phone");
        assert_eq!(entities[0].text, "555-123-4567");
        assert_eq!(entities[0].start, 18);
        assert_eq!(entities[0].end, 30);
    }

    #[test]
    fn test_detect_us_phone_with_dots() {
        let detector = PhoneDetector::new();
        let text = "Contact: 555.123.4567";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].pii_type, "phone");
        assert_eq!(entities[0].text, "555.123.4567");
        assert_eq!(entities[0].start, 9);
        assert_eq!(entities[0].end, 21);
    }

    #[test]
    fn test_detect_us_phone_with_parentheses() {
        let detector = PhoneDetector::new();
        let text = "My number is (555) 123-4567";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].pii_type, "phone");
        assert_eq!(entities[0].text, "(555) 123-4567");
        assert_eq!(entities[0].start, 13);
        assert_eq!(entities[0].end, 27);
    }

    #[test]
    fn test_detect_us_phone_digits_only() {
        let detector = PhoneDetector::new();
        let text = "Phone: 5551234567";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].pii_type, "phone");
        assert_eq!(entities[0].text, "5551234567");
        assert_eq!(entities[0].start, 7);
        assert_eq!(entities[0].end, 17);
    }

    #[test]
    fn test_detect_us_phone_with_country_code() {
        let detector = PhoneDetector::new();
        let text = "International: +1-555-123-4567";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].pii_type, "phone");
        assert_eq!(entities[0].text, "+1-555-123-4567");
        assert_eq!(entities[0].start, 15);
        assert_eq!(entities[0].end, 30);
    }

    #[test]
    fn test_detect_multiple_phones() {
        let detector = PhoneDetector::new();
        let text = "Call 555-123-4567 or (555) 987-6543 for support.";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].text, "555-123-4567");
        assert_eq!(entities[1].text, "(555) 987-6543");
    }

    #[test]
    fn test_no_phone_numbers() {
        let detector = PhoneDetector::new();
        let text = "This text has no phone numbers in it.";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_ignore_invalid_phone_numbers() {
        let detector = PhoneDetector::new();
        let text = "Invalid numbers: 123-45-6789 (SSN), 12345 (too short), 123-456-78901 (too long)";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_detect_phone_in_html() {
        let detector = PhoneDetector::new();
        let text = r#"<div>Contact us at <a href="tel:555-123-4567">555-123-4567</a></div>"#;
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 2); // Both in href and display text
        assert!(entities.iter().any(|e| e.text == "555-123-4567"));
    }

    #[test]
    fn test_detect_phone_with_extension() {
        let detector = PhoneDetector::new();
        let text = "Office: 555-123-4567 ext 1234";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].text, "555-123-4567 ext 1234");
    }

    #[test]
    fn test_detect_swiss_phone_number() {
        let detector = PhoneDetector::new();
        let text = "Swiss number: +41 79 706 7378";
        let entities = detector.detect_phone_numbers(text);
        
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].pii_type, "phone");
        assert_eq!(entities[0].text, "+41 79 706 7378");
    }
}