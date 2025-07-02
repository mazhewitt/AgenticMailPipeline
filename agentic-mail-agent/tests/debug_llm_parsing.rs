//! Debug test to isolate the parsing issue

use agentic_mail_agent::anonymizer::{AnonymizationPipeline, AnonymizationConfig, LlmBackend};

#[tokio::test]
async fn debug_llm_parsing() {
    let config = AnonymizationConfig {
        backend: LlmBackend::Ollama,
        model: "llama3:8b".to_string(),
        ollama_host: "http://localhost:11434".to_string(),
        openai_api_key: None,
        temperature: 0.1,
        llm_timeout_secs: 60,
    };
    
    let mut pipeline = AnonymizationPipeline::new(config).await
        .expect("Failed to create anonymization pipeline");
    
    // Simple text that should be easy to parse
    let simple_text = "Hi John Smith, your email is john@example.com";
    
    let result = pipeline.anonymize_email_text(simple_text).await;
    
    match result {
        Ok(result) => {
            println!("✅ Success! Detected: {}, Replaced: {}", 
                result.detected_entities.len(), 
                result.replacement_log.len());
            
            for entity in &result.detected_entities {
                println!("  Detected: {} '{}' at {}-{}", entity.pii_type, entity.text, entity.start, entity.end);
                
                // Check if the position is correct
                if entity.start < simple_text.len() && entity.end <= simple_text.len() {
                    let actual = &simple_text[entity.start..entity.end];
                    println!("    Actual text at position: '{}'", actual);
                    println!("    Matches: {}", actual == entity.text);
                }
            }
            
            for replacement in &result.replacement_log {
                println!("  Replaced: {} '{}' -> '{}'", 
                    replacement.pii_type, replacement.original_value, replacement.fake_value);
            }
            
            println!("Original: {}", simple_text);
            println!("Result:   {}", result.anonymized_text);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }
}
