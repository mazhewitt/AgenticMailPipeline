//! Entity management, position finding, and deduplication

use crate::anonymizer::{
    types::{PiiEntity, LlmPiiEntity},
    text::find_char_boundary_before,
};

/// Manager for PII entity processing and deduplication
pub struct EntityManager;

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityManager {
    pub fn new() -> Self {
        Self
    }
    
    /// Find all positions of detected PII text in the content
    pub fn find_all_positions(&self, text: &str, llm_entities: Vec<LlmPiiEntity>) -> Vec<PiiEntity> {
        // Safety limit: prevent processing extremely large texts
        if text.len() > 1_000_000 {
            eprintln!("Warning: Text too large ({} chars), truncating to 1MB", text.len());
            let truncated_text = &text[..1_000_000];
            return self.find_all_positions_safe(truncated_text, llm_entities);
        }
        
        // Process with safety limits
        self.find_all_positions_safe(text, llm_entities)
    }
    
    /// Safe version with bounds checking and iteration limits
    fn find_all_positions_safe(&self, text: &str, llm_entities: Vec<LlmPiiEntity>) -> Vec<PiiEntity> {
        let mut entities = Vec::new();
        let max_iterations_per_entity = 1000; // Prevent infinite loops
        
        // First, find all positions for each detected PII text
        for llm_entity in llm_entities {
            let mut start = 0;
            let mut iteration_count = 0;
            
            while start < text.len() && iteration_count < max_iterations_per_entity {
                iteration_count += 1;
                
                // Ensure we're at a character boundary
                let safe_start = find_char_boundary_before(text, start);
                if safe_start >= text.len() {
                    break;
                }
                
                if let Some(pos) = text[safe_start..].find(&llm_entity.text) {
                    let actual_start = safe_start + pos;
                    let actual_end = actual_start + llm_entity.text.len();
                    
                    // Bounds check
                    if actual_end <= text.len() {
                        entities.push(PiiEntity {
                            pii_type: llm_entity.pii_type.clone(),
                            text: llm_entity.text.clone(),
                            start: actual_start,
                            end: actual_end,
                        });
                    }
                    
                    // Move past this occurrence, ensuring we make progress
                    let next_start = actual_start + 1;
                    if next_start <= start {
                        // Safety: ensure we always make progress
                        break;
                    }
                    start = next_start;
                } else {
                    break; // No more occurrences found
                }
            }
            
            if iteration_count >= max_iterations_per_entity {
                eprintln!("Warning: Hit iteration limit for PII text: {}", llm_entity.text);
            }
        }
        
        // Sort entities by start position, then by length (longer first)
        entities.sort_by(|a, b| {
            match a.start.cmp(&b.start) {
                std::cmp::Ordering::Equal => b.text.len().cmp(&a.text.len()), // Longer text first
                other => other,
            }
        });
        
        // Remove overlapping entities, preferring longer and more specific ones
        let mut deduplicated = Vec::new();
        
        for entity in entities {
            let mut should_add = true;
            
            // Check for overlaps with existing entities
            for existing in &deduplicated {
                if self.entities_overlap(&entity, existing) {
                    // Determine which entity to keep
                    if self.should_prefer_existing(existing, &entity) {
                        should_add = false;
                        break;
                    } else {
                        // Remove the existing entity and add this one
                        // We'll handle this by continuing and using a second pass
                    }
                }
            }
            
            if should_add {
                // Remove any existing entities that this one should replace
                deduplicated.retain(|existing| {
                    !self.entities_overlap(&entity, existing) || self.should_prefer_existing(existing, &entity)
                });
                deduplicated.push(entity);
            }
        }
        
        // Sort final entities by start position for consistent processing
        deduplicated.sort_by_key(|e| e.start);
        deduplicated
    }
    
    /// Check if two PII entities overlap in text positions
    fn entities_overlap(&self, a: &PiiEntity, b: &PiiEntity) -> bool {
        // Two entities overlap if one starts before the other ends
        !(a.end <= b.start || b.end <= a.start)
    }
    
    /// Determine which entity to prefer when there's an overlap
    fn should_prefer_existing(&self, existing: &PiiEntity, new: &PiiEntity) -> bool {
        // If one completely contains the other, prefer the longer one
        if existing.start <= new.start && existing.end >= new.end {
            return true; // existing contains new, keep existing
        }
        if new.start <= existing.start && new.end >= existing.end {
            return false; // new contains existing, prefer new
        }
        
        // If they're different lengths, prefer longer
        if existing.text.len() != new.text.len() {
            return existing.text.len() > new.text.len();
        }
        
        // If same length, prefer more specific type
        match (existing.pii_type.as_str(), new.pii_type.as_str()) {
            ("email", "username") => true,  // email is more specific than username
            ("username", "email") => false,
            ("email", "company") => true,   // full email is more specific than just company
            ("company", "email") => false,
            ("name", "title") => true,      // full name is more specific than title
            ("title", "name") => false,
            ("email", "url") => true,       // email is more specific than url
            ("url", "email") => false,
            _ => true, // Keep existing by default
        }
    }
}