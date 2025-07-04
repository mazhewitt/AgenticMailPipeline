# Mock Ollama Testing System

This document describes the mock Ollama testing system that allows LLM-dependent tests to run without requiring a live Ollama instance.

## Overview

The mock Ollama system works in two phases:

1. **Recording Phase**: Run tests with a real Ollama instance to capture LLM responses
2. **Replay Phase**: Use captured responses in unit tests for fast, deterministic testing

## Architecture

### Core Components

- **`MockOllamaClassifier`**: Main classifier that can record or replay responses
- **`RecordedResponse`**: Structure containing email input and LLM classification output  
- **Recording Files**: JSON files containing captured LLM responses

### Recording Mode
```rust
let real_classifier = Box::new(LangChainClassifier::with_default_config().await?);
let mock = MockOllamaClassifier::new_recording_mode(
    "test_data/recorded_responses/my_test.json",
    real_classifier
);

// Use mock.classify() normally - responses are recorded
mock.save_recordings().await?;
```

### Replay Mode
```rust
let mock = MockOllamaClassifier::new_replay_mode(
    "test_data/recorded_responses/my_test.json"
)?;

// Use mock.classify() - responses are replayed deterministically
```

## Workflow

### 1. Record Real LLM Responses

First, run the recording tests with a live Ollama instance:

```bash
# Ensure Ollama is running
ollama serve

# Record individual examples
cargo test --test ollama record_individual_examples

# Record ground truth dataset responses  
cargo test --test ollama record_classifier_ground_truth_responses

# Record hybrid classifier responses
cargo test --test ollama record_hybrid_classifier_responses
```

This creates JSON files in `test_data/recorded_responses/`:
- `individual_examples.json` - Basic classification examples
- `classifier_ground_truth.json` - Real responses for ground truth dataset
- `hybrid_classifier.json` - Hybrid classifier responses

### 2. Create Unit Tests Using Recorded Responses

Create unit tests that use the recorded responses:

```rust
#[tokio::test]
async fn test_my_classification_feature() {
    let mock = MockOllamaClassifier::new_replay_mode(
        "test_data/recorded_responses/individual_examples.json"
    ).expect("Failed to load recordings");
    
    let email = Email::new_full(
        "urgent001".to_string(),
        Some("URGENT: Server Down".to_string()),
        // ... other fields must match recorded email exactly
    );
    
    let classification = mock.classify(&email).await.unwrap();
    assert_eq!(classification.category, "ActionRequired");
    // Test uses real LLM reasoning but runs deterministically!
}
```

### 3. Migrate Tests from Ollama to Unit Category

Once you have recorded responses and created unit tests:

1. Move the unit test to `tests/unit/`
2. Remove or comment out the original Ollama test
3. Update `tests/unit/mod.rs` to include the new test

## Recorded Response Format

Each recorded response contains:

```json
{
  "email_id": "urgent001",
  "email_subject": "URGENT: Server Down", 
  "email_snippet": "Production server crashed...",
  "email_from": "ops@company.com",
  "classification": {
    "category": "ActionRequired",
    "score": 0.95,
    "llm_response": "LLM Response: The email contains urgent language..."
  },
  "raw_response": "LLM Response: The email contains urgent language...",
  "recorded_at": "2025-07-04T21:25:33.243733+00:00"
}
```

## Benefits

### For Development
- **Fast Tests**: Unit tests run in milliseconds vs. seconds for real LLM calls
- **Deterministic**: Same input always produces same output
- **No Dependencies**: Unit tests work without Ollama installation
- **CI/CD Friendly**: No need for LLM infrastructure in CI pipelines

### For Testing Quality  
- **Real LLM Responses**: Tests use actual LLM reasoning, not simplified stubs
- **Regression Detection**: Changes in classification logic are caught immediately
- **Performance Baseline**: Recorded responses provide accuracy baseline

### For Debugging
- **Detailed Reasoning**: Each response includes LLM's reasoning process
- **Reproducible Issues**: Problems can be replicated exactly
- **Response Analysis**: Easy to analyze LLM decision patterns

## Email Signature Matching

The system uses email signatures to match requests to recorded responses:

```
{email_id}:{subject}:{snippet}:{from}
```

**Important**: The email fields in unit tests must exactly match the recorded email for replay to work.

## Current Test Statistics

- **Unit Tests**: 57 tests (including 9 using mock Ollama)
- **Recorded Responses**: 
  - Individual examples: 4 responses
  - Ground truth dataset: 37 responses  
  - Categories: ActionRequired, InterestingInfo, Reference, Noise, Spam

## Example Usage

See existing unit tests:
- `tests/unit/test_mock_ollama_classifier.rs` - Basic mock functionality
- `tests/unit/test_ground_truth_mock.rs` - Ground truth dataset testing

## Future Enhancements

- **Partial Matching**: Allow fuzzy matching of email content
- **Response Variants**: Record multiple responses for same input
- **Confidence Tracking**: Monitor classification confidence over time
- **Auto-Migration**: Automatically promote Ollama tests to unit tests