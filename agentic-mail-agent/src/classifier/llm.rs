//! LLM-based classification implementations
//! 
//! This module contains classifiers that use Large Language Models
//! for email classification.

pub use crate::classifier::langchain::{LangChainClassifier, LangChainConfig};
pub use crate::classifier::mock_ollama::{MockOllamaClassifier, RecordedResponse, RecordedResponses};