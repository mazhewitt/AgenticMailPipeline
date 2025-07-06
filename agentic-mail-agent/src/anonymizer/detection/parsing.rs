//! LLM response parsing and JSON extraction

use crate::anonymizer::types::LlmPiiEntity;

/// Parser for LLM responses containing PII data
pub struct ResponseParser;

impl Default for ResponseParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse LLM response and extract PII entities
    pub fn parse_pii_response(
        &self,
        response: &str,
    ) -> Result<Vec<LlmPiiEntity>, Box<dyn std::error::Error>> {
        // Clean the response - sometimes LLMs add markdown or extra text
        let mut cleaned_response = response.trim();

        // Remove common LLM prefixes
        let prefixes = [
            "JSON output:",
            "Here's the JSON:",
            "```json",
            "```",
            "JSON array:",
        ];
        for prefix in &prefixes {
            if cleaned_response.starts_with(prefix) {
                cleaned_response = cleaned_response[prefix.len()..].trim();
            }
        }

        // Remove ending markers
        if cleaned_response.ends_with("```") {
            cleaned_response = cleaned_response.trim_end_matches("```").trim();
        }

        // Find the JSON array - look for complete arrays
        let json_arrays = self.extract_json_arrays(cleaned_response);

        for json_str in &json_arrays {
            // Clean JSON comments (LLMs sometimes add them)
            let cleaned_json = self.remove_json_comments(json_str);

            // First, try to parse as a simple array of strings
            if let Ok(names) = serde_json::from_str::<Vec<String>>(&cleaned_json) {
                let mut entities = Vec::new();
                for full_name in names {
                    // Add the full name
                    entities.push(LlmPiiEntity {
                        pii_type: "name".to_string(),
                        text: full_name.clone(),
                    });

                    // Also add individual name parts (first name, last name, hyphenated parts)
                    let name_parts: Vec<&str> = full_name.split_whitespace().collect();
                    for part in name_parts {
                        if part.len() > 1 {
                            // Only consider meaningful name parts
                            entities.push(LlmPiiEntity {
                                pii_type: "name".to_string(),
                                text: part.to_string(),
                            });

                            // Also handle hyphenated names like "Hewitt-Fry"
                            if part.contains('-') {
                                let hyphen_parts: Vec<&str> = part.split('-').collect();
                                for hyphen_part in hyphen_parts {
                                    if hyphen_part.len() > 1 {
                                        entities.push(LlmPiiEntity {
                                            pii_type: "name".to_string(),
                                            text: hyphen_part.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                return Ok(entities);
            }

            // If that fails, try to parse as LlmPiiEntity, assuming the LLM used the correct field names
            match serde_json::from_str::<Vec<LlmPiiEntity>>(&cleaned_json) {
                Ok(entities) => return Ok(entities),
                Err(_) => {
                    // If that fails, try parsing as raw JSON and converting field names
                    let raw_entities: Result<Vec<serde_json::Value>, _> =
                        serde_json::from_str(&cleaned_json);
                    match raw_entities {
                        Ok(raw_entities) => {
                            let mut entities = Vec::new();
                            for raw_entity in raw_entities {
                                if let Some(entity) = self.parse_single_llm_entity(&raw_entity) {
                                    entities.push(entity);
                                }
                            }
                            if !entities.is_empty() {
                                return Ok(entities);
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        Err(format!(
            "Failed to parse any valid JSON from LLM response. Response was: {cleaned_response}"
        )
        .into())
    }

    /// Extract JSON arrays from the LLM response text
    pub fn extract_json_arrays(&self, text: &str) -> Vec<String> {
        const MAX_RESPONSE_LENGTH: usize = 1_048_576; // 1MB
        const MAX_ARRAY_LENGTH: usize = 1_048_576; // 1MB
        const MAX_ARRAYS: usize = 100;

        let mut in_string = false;
        let mut escape_next = false;
        let mut current_array = String::new();
        let mut arrays: Vec<String> = Vec::new();
        let mut bracket_count = 0;
        let mut in_array = false;

        let safe_text = if text.len() > MAX_RESPONSE_LENGTH {
            eprintln!(
                "Warning: LLM response too large ({} chars), truncating",
                text.len()
            );
            &text[..MAX_RESPONSE_LENGTH]
        } else {
            text
        };

        for ch in safe_text.chars() {
            // Safety check: prevent arrays from growing too large
            if current_array.len() > MAX_ARRAY_LENGTH {
                eprintln!("Warning: JSON array too large, truncating");
                if in_array {
                    // Try to close the array and save what we have
                    current_array.push(']');
                    arrays.push(current_array.clone());
                    current_array.clear();
                }
                break;
            }

            // Safety check: prevent too many arrays
            if arrays.len() >= MAX_ARRAYS {
                eprintln!("Warning: Too many JSON arrays found, stopping extraction");
                break;
            }
            if escape_next {
                if in_array {
                    current_array.push(ch);
                }
                escape_next = false;
                continue;
            }

            if ch == '\\' && in_string {
                if in_array {
                    current_array.push(ch);
                }
                escape_next = true;
                continue;
            }

            if ch == '"' && !escape_next {
                in_string = !in_string;
                if in_array {
                    current_array.push(ch);
                }
                continue;
            }

            if !in_string {
                if ch == '[' {
                    if !in_array {
                        in_array = true;
                        current_array.clear();
                    }
                    current_array.push(ch);
                    bracket_count += 1;
                } else if ch == ']' {
                    if in_array {
                        current_array.push(ch);
                        bracket_count -= 1;
                        if bracket_count == 0 {
                            in_array = false;
                            arrays.push(current_array.clone());
                        }
                    }
                } else if in_array {
                    current_array.push(ch);
                }
            } else if in_array {
                current_array.push(ch);
            }
        }
        arrays
    }

    fn parse_single_llm_entity(&self, raw_entity: &serde_json::Value) -> Option<LlmPiiEntity> {
        let pii_type = raw_entity
            .get("type")
            .or_else(|| raw_entity.get("pii_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let text = raw_entity
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Only return valid entities (must have text)
        if !text.is_empty() {
            Some(LlmPiiEntity { pii_type, text })
        } else {
            None
        }
    }

    /// Remove JSON comments that LLMs sometimes add
    fn remove_json_comments(&self, json_str: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut escape_next = false;
        let mut chars = json_str.chars().peekable();

        while let Some(ch) = chars.next() {
            if escape_next {
                result.push(ch);
                escape_next = false;
                continue;
            }

            if ch == '\\' && in_string {
                result.push(ch);
                escape_next = true;
                continue;
            }

            if ch == '"' {
                in_string = !in_string;
                result.push(ch);
                continue;
            }

            if !in_string && ch == '/' {
                if let Some(&'/') = chars.peek() {
                    // Skip line comment
                    chars.next(); // consume second '/'
                    for next_ch in chars.by_ref() {
                        if next_ch == '\n' || next_ch == '\r' {
                            result.push(next_ch);
                            break;
                        }
                    }
                    continue;
                } else if let Some(&'*') = chars.peek() {
                    // Skip block comment
                    chars.next(); // consume '*'
                    while let Some(next_ch) = chars.next() {
                        if next_ch == '*' {
                            if let Some(&'/') = chars.peek() {
                                chars.next(); // consume '/'
                                break;
                            }
                        }
                    }
                    continue;
                }
            }

            result.push(ch);
        }

        result
    }
}
