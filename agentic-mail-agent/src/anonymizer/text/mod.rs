//! Text processing utilities for PII anonymization

pub mod cleaning;
pub mod utils;

pub use cleaning::{clean_html_for_llm, extract_meaningful_text, prepare_text_for_llm};
pub use utils::{find_char_boundary, find_char_boundary_before, is_safe_char_boundary, safe_slice};
