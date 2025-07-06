//! Phone number detection using regex patterns

use crate::anonymizer::types::PiiEntity;
use regex::Regex;

/// Regex-based phone number detector
pub struct PhoneDetector {
    patterns: Vec<Regex>,
}

impl Default for PhoneDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PhoneDetector {
    /// Create a new phone detector with US and international phone number patterns
    pub fn new() -> Self {
        let patterns = vec![
            // Phone with extension: 555-123-4567 ext 1234 (check this first to avoid partial matches)
            Regex::new(r"\d{3}-\d{3}-\d{4}\s+ext\s+\d{1,4}").unwrap(),
            // International: +41 79 706 7378 (Swiss), +44 20 1234 5678 (UK), etc.
            Regex::new(r"\+\d{1,4}\s\d{1,4}\s\d{1,4}\s\d{1,4}").unwrap(),
            // International compact: +41797067378
            Regex::new(r"\+\d{8,15}").unwrap(),
            // +1-555-123-4567 (US with country code)
            Regex::new(r"\+1-\d{3}-\d{3}-\d{4}").unwrap(),
            // (555) 123-4567 (US)
            Regex::new(r"\(\d{3}\)\s?\d{3}-\d{4}").unwrap(),
            // 555-123-4567 (US)
            Regex::new(r"\d{3}-\d{3}-\d{4}").unwrap(),
            // 555.123.4567 (US)
            Regex::new(r"\d{3}\.\d{3}\.\d{4}").unwrap(),
            // 5551234567 (10 digits, US)
            Regex::new(r"\b\d{10}\b").unwrap(),
        ];

        Self { patterns }
    }

    /// Detect phone numbers in the given text
    pub fn detect_phone_numbers(&self, text: &str) -> Vec<PiiEntity> {
        let mut entities = Vec::new();

        for pattern in &self.patterns {
            for mat in pattern.find_iter(text) {
                let phone_text = mat.as_str();

                // Validate that it's a reasonable phone number
                if self.is_valid_phone_number(phone_text) {
                    let new_entity = PiiEntity {
                        pii_type: "phone".to_string(),
                        text: phone_text.to_string(),
                        start: mat.start(),
                        end: mat.end(),
                    };

                    // Check if this entity overlaps with any existing entity
                    let overlaps = entities.iter().any(|existing: &PiiEntity| {
                        // Check if ranges overlap
                        new_entity.start < existing.end && existing.start < new_entity.end
                    });

                    if !overlaps {
                        entities.push(new_entity);
                    } else {
                        // If there's an overlap, prefer the longer match
                        let existing_idx = entities.iter().position(|existing: &PiiEntity| {
                            new_entity.start < existing.end && existing.start < new_entity.end
                        });

                        if let Some(idx) = existing_idx {
                            let existing = &entities[idx];
                            if new_entity.text.len() > existing.text.len() {
                                entities[idx] = new_entity;
                            }
                        }
                    }
                }
            }
        }

        // Sort by position
        entities.sort_by(|a, b| a.start.cmp(&b.start));
        entities
    }

    /// Basic validation for phone numbers
    fn is_valid_phone_number(&self, phone: &str) -> bool {
        // Handle phone with extension separately
        if phone.contains("ext") {
            // Extract the phone part (before "ext")
            let phone_part = phone.split("ext").next().unwrap_or(phone).trim();
            return self.is_valid_base_phone_number(phone_part);
        }

        self.is_valid_base_phone_number(phone)
    }

    /// Validate the base phone number (without extension)
    fn is_valid_base_phone_number(&self, phone: &str) -> bool {
        // Extract just the digits
        let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
        let digit_count = digits.len();

        // International phone numbers (with + prefix)
        if phone.starts_with('+') {
            // International numbers should have 7-15 digits total
            if (7..=15).contains(&digit_count) {
                return true;
            }
        }

        // US phone numbers should have 10 digits (or 11 with country code)
        if digit_count == 10 || digit_count == 11 {
            // If 11 digits, should start with 1 (US country code)
            if digit_count == 11 && !digits.starts_with('1') {
                return false;
            }

            // Area code shouldn't start with 0 or 1 for US numbers
            let area_code_start = if digit_count == 11 { 1 } else { 0 };
            if let Some(area_code_first_digit) = digits.chars().nth(area_code_start) {
                if area_code_first_digit == '0' || area_code_first_digit == '1' {
                    return false;
                }
            }

            return true;
        }

        false
    }
}
