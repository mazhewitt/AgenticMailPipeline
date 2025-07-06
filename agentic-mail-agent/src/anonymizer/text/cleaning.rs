//! Text cleaning and preprocessing for LLM consumption

use html2text::from_read;

/// Clean HTML text to make it more readable for the LLM
pub fn clean_html_for_llm(html: &str) -> String {
    // Convert HTML to plain text
    let plain_text = from_read(html.as_bytes(), 80);

    // Clean up the text to make it more readable for the LLM
    plain_text
        // Remove excessive whitespace and control characters
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect::<String>()
        // Normalize whitespace
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        // Remove common email artifacts
        .replace("[\u{200b}\u{200c}\u{200d}\u{feff}]", "") // Zero-width characters
        .replace("\u{200c}\u{200b}\u{200d}\u{200e}\u{200f}\u{feff}", "") // More zero-width characters
}

/// Extract meaningful text by taking the first N words
pub fn extract_meaningful_text(text: &str, word_limit: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() > word_limit {
        words[..word_limit].join(" ")
    } else {
        text.to_string()
    }
}

/// Process text for LLM consumption - combines HTML cleaning and text extraction
pub fn prepare_text_for_llm(text: &str, word_limit: usize) -> String {
    let cleaned = clean_html_for_llm(text);
    extract_meaningful_text(&cleaned, word_limit)
}
