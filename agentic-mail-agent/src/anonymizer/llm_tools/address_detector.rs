//! Address detection using LLM

use crate::anonymizer::{
    config::AnonymizationConfig,
    llm::{LlmClient, PromptTemplate},
    text::prepare_text_for_llm,
    types::PiiEntity,
};

/// LLM-based address detector
pub struct AddressDetector {
    llm_client: LlmClient,
}

impl AddressDetector {
    /// Create a new address detector with the given configuration
    pub async fn new(config: AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let llm_client = LlmClient::new(&config).await?;

        Ok(Self { llm_client })
    }

    /// Use LLM to detect address entities in the given text
    pub async fn detect_addresses(
        &self,
        text: &str,
    ) -> Result<Vec<PiiEntity>, Box<dyn std::error::Error>> {
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

        // Convert to LlmPiiEntity format and find flexible matches
        let mut entities = Vec::new();

        for address_string in address_strings {
            if address_string.trim().is_empty() {
                continue; // Skip empty addresses
            }

            // Try to find address parts in the original text using flexible matching
            if let Some(matched_entities) = self.find_address_in_text(text, &address_string) {
                entities.extend(matched_entities);
            }
        }

        #[cfg(debug_assertions)]
        eprintln!("Found {} address entities: {:?}", entities.len(), entities);

        Ok(entities)
    }

    /// Find address components in text using flexible matching
    fn find_address_in_text(&self, text: &str, llm_address: &str) -> Option<Vec<PiiEntity>> {
        // Skip empty or very short addresses
        if llm_address.trim().is_empty() || llm_address.trim().len() < 3 {
            return None;
        }

        // Split the LLM address into meaningful parts
        let address_parts: Vec<&str> = llm_address
            .split(',')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty() && part.len() > 2)
            .collect();

        if address_parts.is_empty() {
            return None;
        }

        let mut entities = Vec::new();

        // Try to find each part of the address in the original text
        for part in &address_parts {
            // Skip country names (common additions by LLM)
            if part.to_lowercase() == "switzerland"
                || part.to_lowercase() == "germany"
                || part.to_lowercase() == "austria"
                || part.to_lowercase() == "france"
            {
                continue;
            }

            // Find this part in the text
            if let Some(start_pos) = text.find(part) {
                entities.push(PiiEntity {
                    pii_type: "address".to_string(),
                    text: part.to_string(),
                    start: start_pos,
                    end: start_pos + part.len(),
                });
            }
        }

        // Also try to find a more complete address match by looking for street + postal patterns
        if let Some(complete_address) = self.find_complete_address_pattern(text, &address_parts) {
            entities.push(complete_address);
        }

        if entities.is_empty() {
            None
        } else {
            Some(entities)
        }
    }

    /// Try to find a complete address pattern in the text
    fn find_complete_address_pattern(
        &self,
        text: &str,
        address_parts: &[&str],
    ) -> Option<PiiEntity> {
        // Look for patterns like "Street Number\nPostal Code City"
        for part in address_parts {
            if part.contains("mann") || part.contains("str") || part.contains("weg") {
                // Street indicators
                if let Some(start) = text.find(part) {
                    // Look ahead to find postal code and city on next lines
                    let remaining_text = &text[start..];
                    let lines: Vec<&str> = remaining_text.split('\n').take(3).collect();

                    if lines.len() >= 2 {
                        let mut end_pos = start + part.len();

                        // Check if next line contains postal code + city
                        for line in &lines[1..] {
                            let trimmed_line = line.trim();
                            if trimmed_line.len() > 4
                                && trimmed_line.chars().take(4).all(|c| c.is_ascii_digit())
                            {
                                // Found postal code line, extend to include it
                                if let Some(line_end) = text[end_pos..].find(trimmed_line) {
                                    end_pos = end_pos + line_end + trimmed_line.len();
                                }
                            }
                        }

                        if end_pos > start + part.len() {
                            let address_text = &text[start..end_pos];
                            return Some(PiiEntity {
                                pii_type: "address".to_string(),
                                text: address_text.to_string(),
                                start,
                                end: end_pos,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// Parse the LLM response for address extraction
    fn parse_address_response(
        &self,
        response: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
