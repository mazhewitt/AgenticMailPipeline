//! Address detection using LLM

use crate::anonymizer::{
    config::AnonymizationConfig,
    types::{PiiEntity, LlmPiiEntity},
    llm::{LlmClient, PromptTemplate},
    text::prepare_text_for_llm,
    detection::EntityManager,
};

/// LLM-based address detector
pub struct AddressDetector {
    llm_client: LlmClient,
    entity_manager: EntityManager,
}

impl AddressDetector {
    /// Create a new address detector with the given configuration
    pub async fn new(config: AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let llm_client = LlmClient::new(&config).await?;
        let entity_manager = EntityManager::new();
        
        Ok(Self {
            llm_client,
            entity_manager,
        })
    }
    
    /// Use LLM to detect address entities in the given text
    pub async fn detect_addresses(&self, text: &str) -> Result<Vec<PiiEntity>, Box<dyn std::error::Error>> {
        // Prepare text for LLM consumption (clean HTML and limit words)
        let meaningful_text = prepare_text_for_llm(text, 100); // Use more words for addresses
        
        #[cfg(debug_assertions)]
        eprintln!("Converting HTML to text for address detection. Original length: {}, Meaningful text length: {}", text.len(), meaningful_text.len());
        
        // Generate prompt for address extraction
        let prompt = PromptTemplate::extract_addresses(&meaningful_text);
        
        // Get LLM response
        let response = self.llm_client.invoke(&prompt).await?;
        
        // Parse JSON response to get address strings
        let address_strings = self.parse_address_response(&response)?;
        
        // Convert to LlmPiiEntity format
        let llm_entities: Vec<LlmPiiEntity> = address_strings
            .into_iter()
            .map(|addr| LlmPiiEntity {
                pii_type: "address".to_string(),
                text: addr,
            })
            .collect();
        
        // Find all positions for each detected address in the ORIGINAL text
        let entities = self.entity_manager.find_all_positions(text, llm_entities);
        
        #[cfg(debug_assertions)]
        eprintln!("Found {} address entities: {:?}", entities.len(), entities);
        
        Ok(entities)
    }
    
    /// Parse the LLM response for address extraction
    fn parse_address_response(&self, response: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Try to parse as JSON array of strings
        let response = response.trim();
        
        // Handle common LLM response formats
        let json_start = if let Some(start) = response.find('[') {
            start
        } else {
            return Ok(Vec::new()); // No JSON array found
        };
        
        let json_end = if let Some(end) = response.rfind(']') {
            end + 1
        } else {
            return Ok(Vec::new()); // No complete JSON array
        };
        
        let json_str = &response[json_start..json_end];
        
        match serde_json::from_str::<Vec<String>>(json_str) {
            Ok(addresses) => Ok(addresses),
            Err(_) => {
                // Fallback: try to extract addresses from text
                Ok(Vec::new())
            }
        }
    }
}