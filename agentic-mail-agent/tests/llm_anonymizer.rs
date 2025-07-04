use agentic_mail_agent::pii::anonymize_pii_in_email_body;
use serde_json::Value;
use std::fs;

#[test]
fn test_anonymize_name_in_body_llm() {
    // Load the email from the JSON file
    let email_json_str = fs::read_to_string("test_data/email_046.json").expect("Unable to read file");
    let mut email_json: Value = serde_json::from_str(&email_json_str).expect("JSON was not well-formatted");

    // Anonymize PII in the email body
    let anonymized_body = anonymize_pii_in_email_body(email_json["body"].as_str().unwrap());

    // Update the JSON with the anonymized body
    email_json["body"] = serde_json::to_value(anonymized_body).unwrap();

    // Assert that the name "Fry" is anonymized
    assert!(!email_json["body"].as_str().unwrap().contains("Fry"));
}

#[test]
fn test_anonymize_name_in_body_llm_email10() {
    // Load the email from the JSON file
    let email_json_str = fs::read_to_string("test_data/email_010.json").expect("Unable to read file");
    let mut email_json: Value = serde_json::from_str(&email_json_str).expect("JSON was not well-formatted");

    // Anonymize PII in the email body
    let anonymized_body = anonymize_pii_in_email_body(email_json["body"].as_str().unwrap());

    // Update the JSON with the anonymized body
    email_json["body"] = serde_json::to_value(anonymized_body).unwrap();

    // Assert that the name "Mazda" is anonymized
    assert!(!email_json["body"].as_str().unwrap().contains("Mazda"));
}

#[test]
fn test_anonymize_name_in_body_llm_email49() {
    // Load the email from the JSON file
    let email_json_str = fs::read_to_string("test_data/email_049.json").expect("Unable to read file");
    let mut email_json: Value = serde_json::from_str(&email_json_str).expect("JSON was not well-formatted");

    // Anonymize PII in the email body
    let anonymized_body = anonymize_pii_in_email_body(email_json["body"].as_str().unwrap());

    // Update the JSON with the anonymized body
    email_json["body"] = serde_json::to_value(anonymized_body).unwrap();

    // Assert that the name "Mazda" is anonymized
    assert!(!email_json["body"].as_str().unwrap().contains("Mazda"));
}
