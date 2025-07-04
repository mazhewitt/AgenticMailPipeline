//! Fake data generation for different PII types

/// Generator for realistic fake data to replace PII
pub struct FakeDataGenerator;

impl FakeDataGenerator {
    pub fn new() -> Self {
        Self
    }
    
    /// Generate fake data for a specific PII type
    pub fn generate_for_type(&self, pii_type: &str, original_value: &str, seed: usize) -> String {
        match pii_type.to_lowercase().as_str() {
            "name" => self.generate_fake_name(seed),
            "email" => self.generate_fake_email(original_value, seed),
            "phone" => self.generate_fake_phone(seed),
            "address" => self.generate_fake_address(seed),
            "company" => self.generate_fake_company(seed),
            _ => format!("[REDACTED_{}]", pii_type.to_uppercase()),
        }
    }
    
    fn generate_fake_name(&self, seed: usize) -> String {
        let first_names = ["Alex", "Jordan", "Taylor", "Casey", "Morgan", "Riley", "Avery", "Cameron"];
        let last_names = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis"];
        
        let first_idx = (seed * 7) % first_names.len();
        let last_idx = (seed * 13) % last_names.len();
        
        format!("{} {}", first_names[first_idx], last_names[last_idx])
    }
    
    fn generate_fake_email(&self, original: &str, seed: usize) -> String {
        // Preserve the domain structure but anonymize
        if let Some(at_pos) = original.find('@') {
            let domain = &original[at_pos + 1..];
            let fake_username = "user".to_string() + &(seed + 1).to_string();
            
            // If it's a common domain, keep it; otherwise anonymize
            if domain.ends_with(".com") || domain.ends_with(".org") || domain.ends_with(".edu") {
                format!("{}@example.com", fake_username)
            } else {
                format!("{}@{}", fake_username, domain)
            }
        } else {
            format!("user{}@example.com", seed + 1)
        }
    }
    
    fn generate_fake_phone(&self, seed: usize) -> String {
        let area_codes = ["555", "123", "456", "789"];
        let area_idx = (seed * 11) % area_codes.len();
        format!("({}) {}-{}", 
            area_codes[area_idx],
            1000 + (seed % 900),
            1000 + ((seed * 17) % 9000)
        )
    }
    
    fn generate_fake_address(&self, seed: usize) -> String {
        let streets = ["Main Street", "Oak Avenue", "First Street", "Park Avenue", "Elm Street"];
        let cities = ["Springfield", "Riverside", "Franklin", "Georgetown", "Clinton"];
        let states = ["CA", "NY", "TX", "FL", "WA"];
        
        let street_idx = (seed * 19) % streets.len();
        let city_idx = (seed * 23) % cities.len();
        let state_idx = (seed * 29) % states.len();
        let number = 100 + (seed % 900);
        
        format!("{} {}, {}, {} {}", 
            number, streets[street_idx], cities[city_idx], states[state_idx],
            10000 + (seed % 90000)
        )
    }
    
    fn generate_fake_company(&self, seed: usize) -> String {
        let company_names = ["TechCorp", "DataSystems", "InfoTech", "GlobalSoft", "NextGen Solutions"];
        let idx = (seed * 31) % company_names.len();
        company_names[idx].to_string()
    }
}