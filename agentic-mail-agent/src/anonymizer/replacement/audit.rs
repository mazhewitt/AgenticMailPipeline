//! Audit logging for PII replacements

use crate::anonymizer::types::ReplacementLogEntry;

/// Logger for tracking PII replacement operations
pub struct AuditLogger {
    replacement_log: Vec<ReplacementLogEntry>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            replacement_log: Vec::new(),
        }
    }
    
    /// Log a PII replacement operation
    pub fn log_replacement(&mut self, entry: ReplacementLogEntry) {
        self.replacement_log.push(entry);
    }
    
    /// Get the complete replacement log
    pub fn get_log(&self) -> &[ReplacementLogEntry] {
        &self.replacement_log
    }
    
    /// Clear the replacement log
    pub fn clear(&mut self) {
        self.replacement_log.clear();
    }
    
    /// Get count of replacements by PII type
    pub fn get_replacement_count_by_type(&self, pii_type: &str) -> usize {
        self.replacement_log.iter()
            .filter(|entry| entry.pii_type == pii_type)
            .count()
    }
    
    /// Get all replacements of a specific type
    pub fn get_replacements_by_type(&self, pii_type: &str) -> Vec<&ReplacementLogEntry> {
        self.replacement_log.iter()
            .filter(|entry| entry.pii_type == pii_type)
            .collect()
    }
}