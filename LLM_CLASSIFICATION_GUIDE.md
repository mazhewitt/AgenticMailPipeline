# LLM Classification Guide

This guide explains how to use the local LLM-powered email classification feature in the Agentic Mail Agent.

## Overview

The agent supports two classification methods:
- **Stub Classifier**: Fast, deterministic rule-based classification for development and testing
- **LangChain Classifier**: AI-powered classification using local Ollama LLM

## Prerequisites for LLM Classification

### 1. Install Ollama

Download and install Ollama from https://ollama.ai/

### 2. Pull a Compatible Model

```bash
# Install a lightweight model (recommended)
ollama pull llama3.2

# Or use a more powerful model for better accuracy
ollama pull llama3.1:8b
```

### 3. Start Ollama Server

```bash
ollama serve
```

The server will run on `http://localhost:11434` by default.

## Using LLM Classification

### Environment Variable Configuration

Set the classifier type using the `CLASSIFIER_TYPE` environment variable:

```bash
# Use LLM classification
export CLASSIFIER_TYPE=langchain

# Or use stub classification (default)
export CLASSIFIER_TYPE=stub
```

### Running with LLM Classification

```bash
# With LLM classification
CLASSIFIER_TYPE=langchain cargo run --bin agentic-mail-agent

# With stub classification (default)
cargo run --bin agentic-mail-agent
```

## Classification Results Comparison

### Stub Classifier
- **Fast**: Deterministic, rule-based classification
- **Predictable**: Always returns the same results
- **Limited**: Basic keyword matching
- **Good for**: Development, testing, fallback

### LangChain Classifier
- **Intelligent**: Uses local LLM for semantic understanding
- **Accurate**: Better detection of spam, urgency, and categories
- **Context-aware**: Understands email content beyond keywords
- **Privacy-preserving**: All processing happens locally

## Example Output

### Stub Classification
```
🏷️  Classification: work (confidence: 0.60)
🤖 Analysis: Deterministic classification based on content analysis: work
```

### LLM Classification
```
🏷️  Classification: spam (confidence: 0.99)
🤖 Analysis: LLM Response: Highly suspicious language and request for sensitive information (bank details) indicate this is a phishing attempt. (Score: 0.99)
```

## Supported Email Categories

Both classifiers support these categories:
- `work`: Business, professional, or work-related emails
- `personal`: Personal communications from friends, family
- `promotional`: Marketing emails, sales, offers, advertisements
- `spam`: Unwanted, suspicious, or clearly spam emails
- `newsletter`: Newsletters, updates, regular communications
- `urgent`: Time-sensitive emails requiring immediate attention

## Troubleshooting

### LLM Classifier Falls Back to Stub
If you see this message:
```
❌ Failed to initialize LangChain classifier: Failed to connect to Ollama at http://localhost:11434
🔄 Falling back to stub classifier...
```

**Solutions:**
1. Make sure Ollama is running: `ollama serve`
2. Check if the model is available: `ollama list`
3. Pull the required model: `ollama pull llama3.2`
4. Verify Ollama is accessible: `curl http://localhost:11434/api/tags`

### Performance Considerations

**LLM Classification:**
- Slower: Each email requires an LLM inference call
- More accurate: Better understanding of context and nuance
- Resource usage: Requires local compute resources

**Stub Classification:**
- Faster: Immediate rule-based processing
- Consistent: Same results every time
- Low resource: Minimal CPU and memory usage

## Model Recommendations

### For Development
- `llama3.2` (3.2B): Fast, lightweight, good for basic classification
- Good balance of speed and accuracy

### For Production
- `llama3.1:8b` (8B): More accurate, better reasoning
- Higher resource requirements but superior results

### Custom Models
You can use any Ollama-compatible model by updating the default configuration in code or extending the environment variable support.

## Integration with Action Router

The classification results are used by the Action Router to determine appropriate actions:

- **High-confidence spam** → Archive automatically
- **Urgent emails** → Mark important, escalate, notify
- **Work emails** → Apply work label, route to appropriate folder
- **Low-confidence results** → Flag for manual review

## Privacy and Security

✅ **Fully Local**: All LLM processing happens on your machine
✅ **No Data Exfiltration**: Email content never leaves your system
✅ **Offline Capable**: Works without internet connection
✅ **Gmail Read-Only**: Only reads emails, never modifies or sends
