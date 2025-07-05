//! Main PII replacement logic

use crate::anonymizer::{
    types::{PiiEntity, ReplacementLogEntry},
    text::find_char_boundary,
};
use super::{FakeDataGenerator, AuditLogger};
use std::collections::HashMap;

/// PII replacement service with fake data generation
pub struct PiiReplacer {
    fake_data_generator: FakeDataGenerator,
    audit_logger: AuditLogger,
    /// Consistent mapping of original values to fake values within a session
    replacement_cache: HashMap<String, String>,
}

impl Default for PiiReplacer {
    fn default() -> Self {
        Self::new()
    }
}

impl PiiReplacer {
    pub fn new() -> Self {
        Self {
            fake_data_generator: FakeDataGenerator::new(),
            audit_logger: AuditLogger::new(),
            replacement_cache: HashMap::new(),
        }
    }
    
    /// Replace PII entities in text with fake data
    pub fn replace_pii(&mut self, text: &str, entities: &[PiiEntity]) -> Result<String, Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        eprintln!("Starting replacement with {} entities", entities.len());
        
        let mut result = text.to_string();
        let mut offset = 0i32; // Track how text length changes due to replacements
        
        // Sort entities by start position to process in order
        let mut sorted_entities = entities.to_vec();
        sorted_entities.sort_by_key(|e| e.start);
        
        for entity in sorted_entities {
            #[cfg(debug_assertions)]
            eprintln!("Processing entity: {entity:?}");
            
            let fake_value = self.generate_fake_value(&entity.pii_type, &entity.text);
            
            // Calculate adjusted positions due to previous replacements
            let adjusted_start = ((entity.start as i32) + offset) as usize;
            let adjusted_end = ((entity.end as i32) + offset) as usize;
            
            // Verify the text matches what we expect
            if adjusted_start < result.len() && adjusted_end <= result.len() {
                // Ensure we're slicing on character boundaries
                let safe_start = find_char_boundary(&result, adjusted_start);
                let safe_end = find_char_boundary(&result, adjusted_end);
                
                if safe_start < safe_end && safe_end <= result.len() {
                    let current_text = &result[safe_start..safe_end];
                    
                    // Check if the text contains what we're looking for (fuzzy match due to character boundary adjustments)
                    if current_text.contains(&entity.text) || entity.text.contains(current_text) || current_text == entity.text {
                        // Replace the text
                        result.replace_range(safe_start..safe_end, &fake_value);
                        
                        // Update offset for next replacements
                        offset += fake_value.len() as i32 - (safe_end - safe_start) as i32;
                        
                        // Log the replacement
                        self.audit_logger.log_replacement(ReplacementLogEntry {
                            pii_type: entity.pii_type.clone(),
                            original_value: entity.text.clone(),
                            fake_value: fake_value.clone(),
                            position: safe_start,
                        });
                    }
                }
            }
        }
        
        Ok(result)
    }
    
     
    /// Generate a fake value for the given PII type, maintaining consistency
    fn generate_fake_value(&mut self, pii_type: &str, original_value: &str) -> String {
        // Check cache for consistency
        if let Some(cached) = self.replacement_cache.get(original_value) {
            return cached.clone();
        }
        
        let fake_value = self.fake_data_generator.generate_for_type(pii_type, original_value, self.replacement_cache.len());
        
        // Cache for consistency
        self.replacement_cache.insert(original_value.to_string(), fake_value.clone());
        
        fake_value
    }
    
    /// Get the replacement log for auditing
    pub fn get_replacement_log(&self) -> &[ReplacementLogEntry] {
        self.audit_logger.get_log()
    }
    
    /// Clear the replacement log (useful when processing multiple emails)
    pub fn clear_replacement_log(&mut self) {
        self.audit_logger.clear();
    }
}