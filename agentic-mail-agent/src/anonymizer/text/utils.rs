//! Text utility functions for safe UTF-8 string operations

/// Find the nearest character boundary for safe string slicing
pub fn find_char_boundary(text: &str, position: usize) -> usize {
    if position >= text.len() {
        return text.len();
    }
    
    // If we're already on a character boundary, return as-is
    if text.is_char_boundary(position) {
        return position;
    }
    
    // Search backwards for the nearest character boundary
    for i in (0..=position).rev() {
        if text.is_char_boundary(i) {
            return i;
        }
    }
    
    // Fallback to the start if somehow we can't find a boundary
    0
}

/// Find the nearest character boundary at or before the given position
pub fn find_char_boundary_before(text: &str, position: usize) -> usize {
    if position >= text.len() {
        return text.len();
    }
    
    // Find the nearest character boundary at or before the given position
    let mut pos = position;
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Check if a position is on a valid character boundary
pub fn is_safe_char_boundary(text: &str, position: usize) -> bool {
    position <= text.len() && text.is_char_boundary(position)
}

/// Safely slice a string ensuring we don't split UTF-8 characters
pub fn safe_slice(text: &str, start: usize, end: usize) -> &str {
    let safe_start = find_char_boundary(text, start);
    let safe_end = find_char_boundary(text, end.min(text.len()));
    
    if safe_start < safe_end {
        &text[safe_start..safe_end]
    } else {
        ""
    }
}