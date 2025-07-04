//! Fake data generation for different PII types

/// Generator for realistic fake data to replace PII
pub struct FakeDataGenerator;

impl Default for FakeDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}

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
            "location" => self.generate_fake_location(original_value, seed),
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
        // Always use fake domains to prevent any real email leakage
        let clean_original = original.trim_matches(|c| c == '<' || c == '>' || c == '"');
        let _normalized_original = clean_original
            .replace("[at]", "@")
            .replace("(at)", "@")
            .replace("[dot]", ".")
            .replace("(dot)", ".");
        
        let fake_domains = ["example.com", "test.org", "sample.net", "demo.co.uk", "fake.ch"];
        let usernames = ["user", "contact", "admin", "info", "support", "team"];
        
        let domain_idx = (seed * 7) % fake_domains.len();
        let username_idx = (seed * 11) % usernames.len();
        let number = (seed % 999) + 1;
        
        // Preserve HTML/quote formatting if present
        let fake_email = format!("{}{}@{}", usernames[username_idx], number, fake_domains[domain_idx]);
        
        if original.starts_with('<') && original.ends_with('>') {
            format!("<{}>", fake_email)
        } else if original.starts_with('"') && original.ends_with('"') {
            format!("\"{}\"", fake_email)
        } else if original.contains("[at]") {
            fake_email.replace("@", " [at] ").replace(".", " [dot] ")
        } else if original.contains("(at)") {
            fake_email.replace("@", " (at) ").replace(".", " (dot) ")
        } else {
            fake_email
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
    
    fn generate_fake_location(&self, original: &str, seed: usize) -> String {
        // Handle different types of location data
        if original.chars().next().unwrap_or(' ').is_ascii_digit() {
            // Looks like postal code + city
            let fake_postal_codes = ["1000", "2000", "3000", "4000", "5000"];
            let fake_cities = ["Springfield", "Riverside", "Franklin", "Georgetown", "Clinton"];
            
            let postal_idx = (seed * 7) % fake_postal_codes.len();
            let city_idx = (seed * 11) % fake_cities.len();
            
            format!("{} {}", fake_postal_codes[postal_idx], fake_cities[city_idx])
        } else if original.contains("mann") || original.contains("gasse") || 
                  original.contains("strasse") || original.contains("str") {
            // Looks like a street address
            let fake_streets = ["Main Street", "Oak Avenue", "First Street", "Park Avenue", "Elm Street"];
            let street_idx = (seed * 13) % fake_streets.len();
            let number = 100 + (seed % 900);
            
            format!("{} {}", number, fake_streets[street_idx])
        } else {
            // Assume it's a city name
            let fake_cities = ["Springfield", "Riverside", "Franklin", "Georgetown", "Clinton", "Centerville"];
            let city_idx = (seed * 17) % fake_cities.len();
            fake_cities[city_idx].to_string()
        }
    }
    
    fn generate_fake_company(&self, seed: usize) -> String {
        let company_names = ["TechCorp", "DataSystems", "InfoTech", "GlobalSoft", "NextGen Solutions"];
        let idx = (seed * 31) % company_names.len();
        company_names[idx].to_string()
    }
}