#[cfg(test)]
mod tests {
    use agentic_mail_agent::anonymizer::{PiiOrchestrator, AnonymizationConfig, LlmBackend};

    async fn create_test_orchestrator() -> PiiOrchestrator {
        let config = AnonymizationConfig::new(LlmBackend::Ollama, None).unwrap();
        PiiOrchestrator::new(config).await.expect("Failed to create PII orchestrator")
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_orchestrator_detects_phone_numbers() {
        let orchestrator = create_test_orchestrator().await;
        let text = "Please call me at 555-123-4567 for more information.";
        let entities = orchestrator.detect_all_pii(text).await.unwrap();
        
        assert!(!entities.is_empty());
        assert!(entities.iter().any(|e| e.pii_type == "phone"));
        assert!(entities.iter().any(|e| e.text.contains("555-123-4567")));
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_orchestrator_detects_addresses() {
        let orchestrator = create_test_orchestrator().await;
        let text = "Please send the package to 123 Main Street, Anytown, CA 12345.";
        let entities = orchestrator.detect_all_pii(text).await.unwrap();
        
        assert!(!entities.is_empty());
        assert!(entities.iter().any(|e| e.pii_type == "address"));
        assert!(entities.iter().any(|e| e.text.contains("123 Main Street")));
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_orchestrator_detects_names() {
        let orchestrator = create_test_orchestrator().await;
        let text = "Hello John Smith, how are you today?";
        let entities = orchestrator.detect_all_pii(text).await.unwrap();
        
        assert!(!entities.is_empty());
        assert!(entities.iter().any(|e| e.pii_type == "name"));
        assert!(entities.iter().any(|e| e.text.contains("John") || e.text.contains("Smith")));
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_orchestrator_detects_multiple_pii_types() {
        let orchestrator = create_test_orchestrator().await;
        let text = "Hi John Smith, please call me at 555-123-4567 or visit us at 123 Main St, Anytown, CA 12345.";
        let entities = orchestrator.detect_all_pii(text).await.unwrap();
        
        assert!(entities.len() >= 2); // Should detect at least phone and one other type
        
        // Should have different types of PII
        let pii_types: std::collections::HashSet<_> = entities.iter().map(|e| &e.pii_type).collect();
        assert!(pii_types.len() >= 2);
    }

    #[tokio::test] 
    #[ignore] // Requires LLM server
    async fn test_orchestrator_deduplication() {
        let orchestrator = create_test_orchestrator().await;
        // Text where regex and LLM might detect overlapping entities
        let text = "Contact John at 555-123-4567 ext 100";
        let entities = orchestrator.detect_all_pii(text).await.unwrap();
        
        // Should not have overlapping entities
        for (i, entity1) in entities.iter().enumerate() {
            for (j, entity2) in entities.iter().enumerate() {
                if i != j {
                    assert!(
                        entity1.end <= entity2.start || entity2.end <= entity1.start,
                        "Entities overlap: {:?} and {:?}", entity1, entity2
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_orchestrator_stats() {
        let orchestrator = create_test_orchestrator().await;
        let stats = orchestrator.get_detection_stats();
        
        assert_eq!(stats.regex_tools_count, 1); // phone detector
        assert_eq!(stats.llm_tools_count, 2);   // address and name detectors
        assert_eq!(stats.total_tools_count, 3);
    }

    #[tokio::test]
    #[ignore] // Requires LLM server
    async fn test_orchestrator_no_pii() {
        let orchestrator = create_test_orchestrator().await;
        let text = "This is just some regular text with no personal information.";
        let entities = orchestrator.detect_all_pii(text).await.unwrap();
        
        assert_eq!(entities.len(), 0);
    }
}