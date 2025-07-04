//! PII Detection and Anonymization Module
//! 
//! This module provides intelligent PII detection and replacement capabilities using:
//! 1. LLM-based PII entity detection with structured JSON output
//! 2. Rust-based replacement with fake but realistic data
//! 3. Auditability and consistency across the same email
//! 4. LLM-only detection - no fallback, email fails if LLM fails

pub mod types;
pub mod config;
pub mod llm;
pub mod text;
pub mod detection;
pub mod replacement;
pub mod pipeline;

// Re-export main public API
pub use types::{PiiEntity, LlmPiiEntity, ReplacementLogEntry, AnonymizationResult, LlmBackend};
pub use config::AnonymizationConfig;
pub use pipeline::AnonymizationPipeline;
pub use detection::PiiDetector;
pub use replacement::PiiReplacer;