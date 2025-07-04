//! Geographic location detection using regex patterns

use regex::Regex;
use crate::anonymizer::types::PiiEntity;

/// Regex-based location detector for Swiss and international locations
pub struct LocationDetector {
    patterns: Vec<Regex>,
    swiss_cities: Vec<&'static str>,
    swiss_postal_codes: Regex,
}

impl Default for LocationDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl LocationDetector {
    /// Create a new location detector with Swiss and international patterns
    pub fn new() -> Self {
        let patterns = vec![
            // Swiss postal code + city pattern: 8700 Küsnacht, 8006 Zürich
            Regex::new(r"\b\d{4}\s+[A-ZÄÖÜa-zäöüß]+(?:\s+[A-ZÄÖÜa-zäöüß]+)*\b").unwrap(),
            // Swiss address pattern: Weinmanngasse 15
            Regex::new(r"\b[A-ZÄÖÜa-zäöüß]+(?:mann|berg|hof|weg|gasse|strasse|str\.)\s+\d+[a-z]?\b").unwrap(),
            // International postal codes: A-1010, D-10115, F-75001
            Regex::new(r"\b[A-Z]-\d{4,5}\b").unwrap(),
            // UK postal codes: SW1A 1AA, M1 1AA
            Regex::new(r"\b[A-Z]{1,2}\d[A-Z\d]?\s*\d[A-Z]{2}\b").unwrap(),
            // US ZIP codes: 12345, 12345-6789
            Regex::new(r"\b\d{5}(?:-\d{4})?\b").unwrap(),
            // German postal codes with city: 10115 Berlin
            Regex::new(r"\b\d{5}\s+[A-ZÄÖÜa-zäöüß]+\b").unwrap(),
        ];
        
        let swiss_cities = vec![
            "Zürich", "Zurich", "Geneva", "Genève", "Basel", "Bern", "Lausanne",
            "Winterthur", "Luzern", "Lucerne", "St. Gallen", "Sankt Gallen",
            "Lugano", "Biel", "Bienne", "Thun", "Köniz", "La Chaux-de-Fonds",
            "Schaffhausen", "Fribourg", "Chur", "Vernier", "Neuchâtel", 
            "Uster", "Sion", "Lancy", "Yverdon-les-Bains", "Emmen", "Zug",
            "Kriens", "Rapperswil-Jona", "Dübendorf", "Dietikon", "Riehen",
            "Montreux", "Frauenfeld", "Küsnacht", "Küssnacht", "Thalwil",
            "Wetzikon", "Baar", "Meyrin", "Carouge", "Wädenswil", "Allschwil"
        ];
        
        let swiss_postal_codes = Regex::new(r"\b[1-9]\d{3}\b").unwrap();
        
        Self { 
            patterns, 
            swiss_cities,
            swiss_postal_codes,
        }
    }
    
    /// Detect geographic locations in the given text
    pub fn detect_locations(&self, text: &str) -> Vec<PiiEntity> {
        let mut entities = Vec::new();
        
        // First, use regex patterns
        for pattern in &self.patterns {
            for mat in pattern.find_iter(text) {
                let location_text = mat.as_str();
                
                if self.is_valid_location(location_text) {
                    let new_entity = PiiEntity {
                        pii_type: "location".to_string(),
                        text: location_text.to_string(),
                        start: mat.start(),
                        end: mat.end(),
                    };
                    
                    // Check for overlaps
                    let overlaps = entities.iter().any(|existing: &PiiEntity| {
                        new_entity.start < existing.end && existing.start < new_entity.end
                    });
                    
                    if !overlaps {
                        entities.push(new_entity);
                    } else {
                        // Prefer longer matches
                        if let Some(idx) = entities.iter().position(|existing: &PiiEntity| {
                            new_entity.start < existing.end && existing.start < new_entity.end
                        }) {
                            if new_entity.text.len() > entities[idx].text.len() {
                                entities[idx] = new_entity;
                            }
                        }
                    }
                }
            }
        }
        
        // Also detect Swiss cities by name
        for city in &self.swiss_cities {
            // Look for city names as standalone words
            let city_pattern = format!(r"\b{}\b", regex::escape(city));
            if let Ok(city_regex) = Regex::new(&city_pattern) {
                for mat in city_regex.find_iter(text) {
                    let city_text = mat.as_str();
                    
                    let new_entity = PiiEntity {
                        pii_type: "location".to_string(),
                        text: city_text.to_string(),
                        start: mat.start(),
                        end: mat.end(),
                    };
                    
                    // Check for overlaps
                    let overlaps = entities.iter().any(|existing: &PiiEntity| {
                        new_entity.start < existing.end && existing.start < new_entity.end
                    });
                    
                    if !overlaps {
                        entities.push(new_entity);
                    }
                }
            }
        }
        
        // Sort by position
        entities.sort_by(|a, b| a.start.cmp(&b.start));
        entities
    }
    
    /// Validate if a matched string is a valid location
    fn is_valid_location(&self, location: &str) -> bool {
        let location = location.trim();
        
        // Skip very short matches
        if location.len() < 3 {
            return false;
        }
        
        // Swiss postal code validation
        if self.swiss_postal_codes.is_match(location) {
            let postal_code: u32 = location.parse().unwrap_or(0);
            // Swiss postal codes are 1000-9999
            return (1000..=9999).contains(&postal_code);
        }
        
        // Validate postal code + city patterns
        if location.chars().next().unwrap_or(' ').is_ascii_digit() {
            let parts: Vec<&str> = location.split_whitespace().collect();
            if parts.len() >= 2 {
                // First part should be valid postal code
                if let Ok(postal_code) = parts[0].parse::<u32>() {
                    // Swiss postal codes
                    if (1000..=9999).contains(&postal_code) {
                        return true;
                    }
                    // German postal codes
                    if (10000..=99999).contains(&postal_code) {
                        return true;
                    }
                }
            }
        }
        
        // Validate street patterns
        if location.contains("mann") || location.contains("berg") || 
           location.contains("hof") || location.contains("weg") ||
           location.contains("gasse") || location.contains("strasse") ||
           location.contains("str.") {
            return true;
        }
        
        // Check if it's a known Swiss city
        for city in &self.swiss_cities {
            if location.eq_ignore_ascii_case(city) {
                return true;
            }
        }
        
        // Validate international patterns
        if location.contains('-') && location.len() <= 8 {
            // Could be A-1010 format
            return true;
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swiss_postal_city() {
        let detector = LocationDetector::new();
        let text = "I live in 8700 Küsnacht, Switzerland.";
        let locations = detector.detect_locations(text);
        
        println!("Detected locations: {:?}", locations);
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|l| l.text.contains("8700") || l.text.contains("Küsnacht")));
    }

    #[test]
    fn test_swiss_street() {
        let detector = LocationDetector::new();
        let text = "Visit me at Weinmanngasse 15 tomorrow.";
        let locations = detector.detect_locations(text);
        
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|l| l.text.contains("Weinmanngasse")));
    }

    #[test]
    fn test_swiss_city_names() {
        let detector = LocationDetector::new();
        let text = "Meeting in Zürich next week.";
        let locations = detector.detect_locations(text);
        
        assert!(!locations.is_empty());
        assert_eq!(locations[0].text, "Zürich");
    }

    #[test]
    fn test_international_patterns() {
        let detector = LocationDetector::new();
        let text = "Shipping to 10115 Berlin, Germany.";
        let locations = detector.detect_locations(text);
        
        assert!(!locations.is_empty());
        assert!(locations.iter().any(|l| l.text.contains("10115")));
    }
}