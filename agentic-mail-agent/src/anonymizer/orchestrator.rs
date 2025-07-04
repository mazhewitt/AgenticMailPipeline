//! Orchestrating agent that combines regex and LLM tools for comprehensive PII detection

use crate::anonymizer::{
    config::AnonymizationConfig,
    types::PiiEntity,
    regex_tools::{PhoneDetector, EmailDetector, LocationDetector},
    llm_tools::AddressDetector,
    detection::PiiDetector, // For name detection
};

/// Orchestrating agent that combines multiple PII detection approaches
pub struct PiiOrchestrator {
    // Regex-based tools for structured PII
    phone_detector: PhoneDetector,
    email_detector: EmailDetector,
    location_detector: LocationDetector,
    
    // LLM-based tools for contextual PII
    address_detector: AddressDetector,
    name_detector: PiiDetector,
}

impl PiiOrchestrator {
    /// Create a new PII orchestrator with the given configuration
    pub async fn new(config: AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize regex-based detectors (fast, deterministic)
        let phone_detector = PhoneDetector::new();
        let email_detector = EmailDetector::new();
        let location_detector = LocationDetector::new();
        
        // Initialize LLM-based detectors (intelligent, contextual)
        let address_detector = AddressDetector::new(config.clone()).await?;
        let name_detector = PiiDetector::new(config).await?;
        
        Ok(Self {
            phone_detector,
            email_detector,
            location_detector,
            address_detector,
            name_detector,
        })
    }
    
    /// Orchestrate comprehensive PII detection using multiple specialized tools
    pub async fn detect_all_pii(&self, text: &str) -> Result<Vec<PiiEntity>, Box<dyn std::error::Error>> {
        let mut all_entities = Vec::new();
        
        // Step 1: Use regex tools for structured PII (fast and accurate)
        let phone_entities = self.phone_detector.detect_phone_numbers(text);
        all_entities.extend(phone_entities);
        
        let email_entities = self.email_detector.detect_emails(text);
        all_entities.extend(email_entities);
        
        let location_entities = self.location_detector.detect_locations(text);
        all_entities.extend(location_entities);
        
        #[cfg(debug_assertions)]
        eprintln!("Regex tools found {} entities", all_entities.len());
        
        // Step 2: Use LLM tools for contextual PII (intelligent but slower)
        let address_entities = self.address_detector.detect_addresses(text).await?;
        all_entities.extend(address_entities);
        
        let name_entities = self.name_detector.detect_pii(text).await?;
        all_entities.extend(name_entities);
        
        #[cfg(debug_assertions)]
        eprintln!("All tools found {} total entities", all_entities.len());
        
        // Step 3: Remove overlapping detections (prefer more specific tools)
        let deduplicated_entities = self.deduplicate_entities(all_entities);
        
        #[cfg(debug_assertions)]
        eprintln!("After deduplication: {} entities", deduplicated_entities.len());
        
        Ok(deduplicated_entities)
    }
    
    /// Deduplicate overlapping entities, preferring more specific detections
    fn deduplicate_entities(&self, mut entities: Vec<PiiEntity>) -> Vec<PiiEntity> {
        // Sort by start position
        entities.sort_by(|a, b| a.start.cmp(&b.start));
        
        let mut deduplicated = Vec::new();
        
        for entity in entities {
            // Check if this entity overlaps with any existing entity
            let overlaps_with_existing = deduplicated.iter().any(|existing: &PiiEntity| {
                // Check if ranges overlap
                entity.start < existing.end && existing.start < entity.end
            });
            
            if !overlaps_with_existing {
                deduplicated.push(entity);
            } else {
                // If there's an overlap, replace with the longer/more specific match
                if let Some(existing_idx) = deduplicated.iter().position(|existing: &PiiEntity| {
                    entity.start < existing.end && existing.start < entity.end
                }) {
                    let existing = &deduplicated[existing_idx];
                    
                    // Prefer longer matches, or regex tools over LLM for structured data
                    if entity.text.len() > existing.text.len() 
                        || (entity.pii_type == "phone" && existing.pii_type == "name")
                        || (entity.pii_type == "email" && existing.pii_type == "name")
                        || (entity.pii_type == "location" && existing.pii_type == "name") {
                        deduplicated[existing_idx] = entity;
                    }
                }
            }
        }
        
        deduplicated
    }
    
    /// Get statistics about detection performance
    pub fn get_detection_stats(&self) -> DetectionStats {
        DetectionStats {
            regex_tools_count: 3, // phone, email, and location detectors
            llm_tools_count: 2,   // address and name detectors
            total_tools_count: 5,
        }
    }
}

/// Statistics about the detection tools available
#[derive(Debug)]
pub struct DetectionStats {
    pub regex_tools_count: usize,
    pub llm_tools_count: usize,
    pub total_tools_count: usize,
}