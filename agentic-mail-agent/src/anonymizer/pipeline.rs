//! End-to-end anonymization pipeline orchestration

use crate::anonymizer::{
    config::AnonymizationConfig, detection::PiiDetector, replacement::PiiReplacer,
    types::AnonymizationResult,
};

/// Complete anonymization pipeline
pub struct AnonymizationPipeline {
    detector: PiiDetector,
    replacer: PiiReplacer,
}

impl AnonymizationPipeline {
    /// Create a new anonymization pipeline with the given configuration
    pub async fn new(config: AnonymizationConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let detector = PiiDetector::new(config).await?;
        let replacer = PiiReplacer::new();

        Ok(Self { detector, replacer })
    }

    /// Anonymize an email text end-to-end
    pub async fn anonymize_email_text(
        &mut self,
        text: &str,
    ) -> Result<AnonymizationResult, Box<dyn std::error::Error>> {
        // Clear any previous replacement log to avoid contamination
        self.replacer.clear_replacement_log();

        // Step 1: Detect PII using LLM
        let detected_entities = self.detector.detect_pii(text).await?;

        // Step 2: Replace PII with fake data (LLM-only, no fallback)
        let anonymized_text = self.replacer.replace_pii(text, &detected_entities)?;

        // Step 3: Return comprehensive result
        Ok(AnonymizationResult {
            anonymized_text,
            detected_entities,
            replacement_log: self.replacer.get_replacement_log().to_vec(),
        })
    }
}
