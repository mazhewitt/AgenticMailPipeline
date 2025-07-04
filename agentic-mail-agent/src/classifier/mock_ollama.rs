//! Mock Ollama classifier that can record and replay LLM responses.
//! 
//! This module provides a way to capture real LLM responses during testing
//! and then replay them deterministically for unit tests.

use crate::classifier::{Classification, ClassificationError, MessageClassifier};
use crate::email::Email;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A single recorded LLM interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedResponse {
    /// Input email content (for matching)
    pub email_id: String,
    pub email_subject: Option<String>,
    pub email_snippet: Option<String>,
    pub email_from: Option<String>,
    /// The classification response from the LLM
    pub classification: Classification,
    /// Raw LLM response text
    pub raw_response: String,
    /// Timestamp when recorded
    pub recorded_at: String,
}

/// Collection of recorded responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedResponses {
    pub responses: Vec<RecordedResponse>,
    pub metadata: RecordingMetadata,
}

/// Metadata about the recording session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub model: String,
    pub recorded_at: String,
    pub total_responses: usize,
}

/// Mock Ollama classifier that can record and replay responses
pub struct MockOllamaClassifier {
    /// Map from email signature to recorded response
    responses: std::sync::Arc<std::sync::Mutex<HashMap<String, RecordedResponse>>>,
    /// Whether to record new responses (requires real LLM)
    recording_mode: bool,
    /// Real classifier for recording mode
    real_classifier: Option<Box<dyn MessageClassifier + Send + Sync>>,
    /// Path to save/load recordings
    recording_file: String,
}

impl MockOllamaClassifier {
    /// Create a new mock classifier in replay mode
    pub fn new_replay_mode(recording_file: &str) -> Result<Self, ClassificationError> {
        let responses = Self::load_recordings(recording_file)?;
        Ok(Self {
            responses: std::sync::Arc::new(std::sync::Mutex::new(responses)),
            recording_mode: false,
            real_classifier: None,
            recording_file: recording_file.to_string(),
        })
    }
    
    /// Create a new mock classifier in recording mode
    pub fn new_recording_mode(
        recording_file: &str, 
        real_classifier: Box<dyn MessageClassifier + Send + Sync>
    ) -> Self {
        Self {
            responses: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            recording_mode: true,
            real_classifier: Some(real_classifier),
            recording_file: recording_file.to_string(),
        }
    }
    
    /// Generate a signature for an email to use as a key
    fn email_signature(email: &Email) -> String {
        format!(
            "{}:{}:{}:{}",
            email.id,
            email.subject.as_deref().unwrap_or(""),
            email.snippet.as_deref().unwrap_or(""),
            email.from.as_deref().unwrap_or("")
        )
    }
    
    /// Load recorded responses from file
    fn load_recordings(file_path: &str) -> Result<HashMap<String, RecordedResponse>, ClassificationError> {
        if !Path::new(file_path).exists() {
            return Err(ClassificationError::config(format!(
                "Recording file {} does not exist. Run in recording mode first.", 
                file_path
            )));
        }
        
        let contents = fs::read_to_string(file_path)
            .map_err(|e| ClassificationError::config(format!(
                "Failed to read recording file {}: {}", file_path, e
            )))?;
            
        let recorded: RecordedResponses = serde_json::from_str(&contents)
            .map_err(|e| ClassificationError::config(format!(
                "Failed to parse recording file {}: {}", file_path, e
            )))?;
            
        let mut responses = HashMap::new();
        for response in recorded.responses {
            let key = format!(
                "{}:{}:{}:{}",
                response.email_id,
                response.email_subject.as_deref().unwrap_or(""),
                response.email_snippet.as_deref().unwrap_or(""),
                response.email_from.as_deref().unwrap_or("")
            );
            responses.insert(key, response);
        }
        
        println!("📼 Loaded {} recorded responses from {}", responses.len(), file_path);
        Ok(responses)
    }
    
    /// Save recorded responses to file
    pub async fn save_recordings(&self) -> Result<(), ClassificationError> {
        if !self.recording_mode {
            return Ok(());
        }
        
        let responses_guard = self.responses.lock().unwrap();
        let responses: Vec<RecordedResponse> = responses_guard.values().cloned().collect();
        drop(responses_guard);
        
        let recorded = RecordedResponses {
            metadata: RecordingMetadata {
                model: "llama3.1:8b".to_string(),
                recorded_at: chrono::Utc::now().to_rfc3339(),
                total_responses: responses.len(),
            },
            responses,
        };
        
        let json = serde_json::to_string_pretty(&recorded)
            .map_err(|e| ClassificationError::unknown(format!(
                "Failed to serialize recordings: {}", e
            )))?;
            
        fs::write(&self.recording_file, json)
            .map_err(|e| ClassificationError::unknown(format!(
                "Failed to write recording file {}: {}", self.recording_file, e
            )))?;
            
        println!("💾 Saved {} recorded responses to {}", recorded.metadata.total_responses, self.recording_file);
        Ok(())
    }
    
    /// Get statistics about loaded recordings
    pub fn get_stats(&self) -> (usize, Vec<String>) {
        let responses_guard = self.responses.lock().unwrap();
        let total = responses_guard.len();
        let categories: Vec<String> = responses_guard
            .values()
            .map(|r| r.classification.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        (total, categories)
    }
}

#[async_trait]
impl MessageClassifier for MockOllamaClassifier {
    async fn classify(&self, email: &Email) -> Result<Classification, ClassificationError> {
        let signature = Self::email_signature(email);
        
        if self.recording_mode {
            // Recording mode: use real classifier and save response
            if let Some(ref real_classifier) = self.real_classifier {
                println!("🎥 Recording response for email: {}", email.id);
                let classification = real_classifier.classify(email).await?;
                
                let recorded = RecordedResponse {
                    email_id: email.id.clone(),
                    email_subject: email.subject.clone(),
                    email_snippet: email.snippet.clone(),
                    email_from: email.from.clone(),
                    classification: classification.clone(),
                    raw_response: classification.llm_response.clone(),
                    recorded_at: chrono::Utc::now().to_rfc3339(),
                };
                
                // Store in memory (will be saved later)
                let mut responses_guard = self.responses.lock().unwrap();
                responses_guard.insert(signature, recorded);
                drop(responses_guard);
                
                Ok(classification)
            } else {
                Err(ClassificationError::config("Recording mode requires a real classifier".to_string()))
            }
        } else {
            // Replay mode: return recorded response
            let responses_guard = self.responses.lock().unwrap();
            if let Some(recorded) = responses_guard.get(&signature) {
                println!("📼 Replaying response for email: {} -> {}", email.id, recorded.classification.category);
                let classification = recorded.classification.clone();
                drop(responses_guard);
                Ok(classification)
            } else {
                let available_signatures: Vec<String> = responses_guard.keys().take(5).cloned().collect();
                drop(responses_guard);
                Err(ClassificationError::unknown(format!(
                    "No recorded response found for email signature: {}. Available signatures: [{}]",
                    signature,
                    available_signatures.join(", ")
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classifier::StubClassifier;
    
    #[tokio::test]
    async fn test_mock_classifier_recording_mode() {
        let real_classifier = Box::new(StubClassifier::deterministic());
        let mock = MockOllamaClassifier::new_recording_mode(
            "/tmp/test_recordings.json", 
            real_classifier
        );
        
        let email = Email::new_full(
            "test123".to_string(),
            Some("Test Subject".to_string()),
            Some("Test snippet".to_string()),
            Some("test@example.com".to_string()),
            None, None, None,
        );
        
        let classification = mock.classify(&email).await.unwrap();
        assert!(!classification.category.is_empty());
        
        // Save recordings
        mock.save_recordings().await.unwrap();
        
        // Verify file was created
        assert!(Path::new("/tmp/test_recordings.json").exists());
        
        // Clean up
        let _ = fs::remove_file("/tmp/test_recordings.json");
    }
    
    #[test]
    fn test_email_signature() {
        let email = Email::new_full(
            "id123".to_string(),
            Some("Subject".to_string()),
            Some("Snippet".to_string()),
            Some("from@example.com".to_string()),
            None, None, None,
        );
        
        let signature = MockOllamaClassifier::email_signature(&email);
        assert_eq!(signature, "id123:Subject:Snippet:from@example.com");
    }
}