//! Main PII detection logic using LLM

use super::{EntityManager, ResponseParser};
use crate::anonymizer::{
    config::AnonymizationConfig,
    llm::{LlmClient, PromptTemplate},
    text::prepare_text_for_llm,
    types::PiiEntity,
};

/// PII detection service using LLM
pub struct PiiDetector {
    llm_client: LlmClient,
    response_parser: ResponseParser,
    entity_manager: EntityManager,
}

impl PiiDetector {
    /// Create a new PII detector with the given configuration
    pub async fn new(config: AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let llm_client = LlmClient::new(&config).await?;
        let response_parser = ResponseParser::new();
        let entity_manager = EntityManager::new();

        Ok(Self {
            llm_client,
            response_parser,
            entity_manager,
        })
    }

    /// Use LLM to detect PII entities in the given text
    pub async fn detect_pii(
        &self,
        text: &str,
    ) -> Result<Vec<PiiEntity>, Box<dyn std::error::Error>> {
        // Prepare text for LLM consumption (clean HTML and limit words)
        let meaningful_text = prepare_text_for_llm(text, 50);

        #[cfg(debug_assertions)]
        eprintln!(
            "Converting HTML to text. Original length: {}, Meaningful text length: {}",
            text.len(),
            meaningful_text.len()
        );

        // Generate prompt for name extraction
        let prompt = PromptTemplate::extract_names(&meaningful_text);

        // Get LLM response
        let response = self.llm_client.invoke(&prompt).await?;

        // Parse JSON response to get LLM entities
        let llm_entities = self.response_parser.parse_pii_response(&response)?;

        // Find all positions for each detected PII text in the ORIGINAL text (not shortened)
        let entities = self.entity_manager.find_all_positions(text, llm_entities);

        #[cfg(debug_assertions)]
        eprintln!("Found {} PII entities: {:?}", entities.len(), entities);

        Ok(entities)
    }
}
