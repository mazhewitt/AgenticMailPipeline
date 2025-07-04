use crate::anonymizer::{AnonymizationConfig, AnonymizationPipeline, LlmBackend};
use tokio::runtime::Runtime;

pub fn anonymize_pii_in_email_body(body: &str) -> String {
    let config = AnonymizationConfig::new(LlmBackend::Ollama, Some("mistral".to_string()))
        .expect("Failed to create anonymization config");

    let rt = Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        match AnonymizationPipeline::new(config).await {
            Ok(mut pipeline) => match pipeline.anonymize_email_text(body).await {
                Ok(result) => result.anonymized_text,
                Err(e) => {
                    eprintln!("Failed to anonymize email text: {}", e);
                    body.to_string()
                }
            },
            Err(e) => {
                eprintln!("Failed to create anonymization pipeline: {}", e);
                body.to_string()
            }
        }
    })
}
