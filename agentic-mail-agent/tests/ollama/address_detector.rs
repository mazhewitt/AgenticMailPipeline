#[cfg(test)]
mod tests {
    use agentic_mail_agent::anonymizer::{AddressDetector, AnonymizationConfig, LlmBackend};

    async fn create_test_detector() -> AddressDetector {
        let config = AnonymizationConfig::new(LlmBackend::Ollama, None).unwrap();
        AddressDetector::new(config)
            .await
            .expect("Failed to create address detector")
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_detect_street_address() {
        let detector = create_test_detector().await;
        let text = "Please send the package to 123 Main Street, Anytown, CA 12345.";
        let entities = detector.detect_addresses(text).await.unwrap();

        assert!(!entities.is_empty());
        assert_eq!(entities[0].pii_type, "address");
        assert!(entities[0].text.contains("123 Main Street"));
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_detect_po_box() {
        let detector = create_test_detector().await;
        let text = "Mail to PO Box 1234, Springfield, IL 62701";
        let entities = detector.detect_addresses(text).await.unwrap();

        assert!(!entities.is_empty());
        assert_eq!(entities[0].pii_type, "address");
        assert!(entities[0].text.contains("PO Box 1234"));
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_detect_multiple_addresses() {
        let detector = create_test_detector().await;
        let text = "Ship from 123 First St, City, ST 12345 to 456 Second Ave, Town, ST 67890";
        let entities = detector.detect_addresses(text).await.unwrap();

        assert!(!entities.is_empty()); // LLM might detect 1 or 2 addresses
        assert_eq!(entities[0].pii_type, "address");
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_no_addresses() {
        let detector = create_test_detector().await;
        let text = "This text has no addresses in it.";
        let entities = detector.detect_addresses(text).await.unwrap();

        assert_eq!(entities.len(), 0);
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_detect_address_in_html() {
        let detector = create_test_detector().await;
        let text = r#"<div>Location: <span>123 Main St, Anytown, CA 12345</span></div>"#;
        let entities = detector.detect_addresses(text).await.unwrap();

        assert!(!entities.is_empty());
        assert_eq!(entities[0].pii_type, "address");
        assert!(entities[0].text.contains("123 Main St"));
    }
}
