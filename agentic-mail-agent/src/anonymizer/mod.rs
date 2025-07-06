//! PII Detection and Anonymization Module
//!
//! This module provides intelligent PII detection and replacement capabilities using:
//! 1. LLM-based PII entity detection with structured JSON output
//! 2. Rust-based replacement with fake but realistic data
//! 3. Auditability and consistency across the same email
//! 4. LLM-only detection - no fallback, email fails if LLM fails

pub mod config;
pub mod detection;
pub mod llm;
pub mod llm_tools;
pub mod orchestrator;
pub mod pipeline;
pub mod regex_tools;
pub mod replacement;
pub mod text;
pub mod types;

// Re-export main public API
pub use config::AnonymizationConfig;
pub use detection::PiiDetector;
pub use llm_tools::AddressDetector;
pub use orchestrator::{DetectionStats, PiiOrchestrator};
pub use pipeline::AnonymizationPipeline;
pub use regex_tools::PhoneDetector;
pub use replacement::PiiReplacer;
pub use types::{AnonymizationResult, LlmBackend, LlmPiiEntity, PiiEntity, ReplacementLogEntry};
