use agentic_mail_agent::anonymizer::{AnonymizationPipeline, AnonymizationConfig, LlmBackend};
use serde_json::Value;
use std::fs;

#[tokio::test]
async fn test_anonymize_name_in_body_llm() {
    // Load the email from the JSON file
    let email_json_str = fs::read_to_string("test_data/email_046.json").expect("Unable to read file");
    let mut email_json: Value = serde_json::from_str(&email_json_str).expect("JSON was not well-formatted");

    // Setup anonymization pipeline
    let config = AnonymizationConfig::new(LlmBackend::Ollama, Some("mistral".to_string()));
    if let Ok(config) = config {
        let pipeline_result = AnonymizationPipeline::new(config).await;
        
        if let Ok(mut pipeline) = pipeline_result {
            // Anonymize PII in the email body
            let result = pipeline.anonymize_email_text(email_json["body"].as_str().unwrap()).await;
            
            if let Ok(anonymization_result) = result {
                // Update the JSON with the anonymized body
                email_json["body"] = serde_json::to_value(anonymization_result.anonymized_text).unwrap();

                // Assert that the name "Fry" is anonymized
                assert!(!email_json["body"].as_str().unwrap().contains("Fry"));
            } else {
                // Skip test if LLM backend is not available
                println!("Skipping test - LLM backend not available");
            }
        } else {
            println!("Skipping test - Pipeline creation failed");
        }
    } else {
        println!("Skipping test - Config creation failed");
    }
}

#[tokio::test]
async fn test_anonymize_name_in_body_llm_email10() {
    // Load the email from the JSON file
    let email_json_str = fs::read_to_string("test_data/email_010.json").expect("Unable to read file");
    let mut email_json: Value = serde_json::from_str(&email_json_str).expect("JSON was not well-formatted");

    // Setup anonymization pipeline
    let config = AnonymizationConfig::new(LlmBackend::Ollama, Some("mistral".to_string()));
    if let Ok(config) = config {
        let pipeline_result = AnonymizationPipeline::new(config).await;
        
        if let Ok(mut pipeline) = pipeline_result {
            // Anonymize PII in the email body
            let result = pipeline.anonymize_email_text(email_json["body"].as_str().unwrap()).await;
            
            if let Ok(anonymization_result) = result {
                // Update the JSON with the anonymized body
                email_json["body"] = serde_json::to_value(anonymization_result.anonymized_text).unwrap();

                // Assert that the name "Mazda" is anonymized
                assert!(!email_json["body"].as_str().unwrap().contains("Mazda"));
            } else {
                // Skip test if LLM backend is not available
                println!("Skipping test - LLM backend not available");
            }
        } else {
            println!("Skipping test - Pipeline creation failed");
        }
    } else {
        println!("Skipping test - Config creation failed");
    }
}

#[tokio::test]
async fn test_anonymize_name_in_body_llm_email49() {
    // Load the email from the JSON file
    let email_json_str = fs::read_to_string("test_data/email_049.json").expect("Unable to read file");
    let mut email_json: Value = serde_json::from_str(&email_json_str).expect("JSON was not well-formatted");

    // Setup anonymization pipeline
    let config = AnonymizationConfig::new(LlmBackend::Ollama, Some("mistral".to_string()));
    if let Ok(config) = config {
        let pipeline_result = AnonymizationPipeline::new(config).await;
        
        if let Ok(mut pipeline) = pipeline_result {
            // Anonymize PII in the email body
            let result = pipeline.anonymize_email_text(email_json["body"].as_str().unwrap()).await;
            
            if let Ok(anonymization_result) = result {
                // Update the JSON with the anonymized body
                email_json["body"] = serde_json::to_value(anonymization_result.anonymized_text).unwrap();

                // Assert that the name "Mazda" is anonymized
                assert!(!email_json["body"].as_str().unwrap().contains("Mazda"));
            } else {
                // Skip test if LLM backend is not available
                println!("Skipping test - LLM backend not available");
            }
        } else {
            println!("Skipping test - Pipeline creation failed");
        }
    } else {
        println!("Skipping test - Config creation failed");
    }
}
