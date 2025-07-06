# Agentic Mail Agent - Main Binary

This directory contains the main email processing binary for the Agentic Mail Agent. The binary processes emails in your inbox, classifies them, and applies intelligent archiving logic.

## 🚀 Quick Start

### Demo Mode (No Gmail Setup Required)
```bash
# Run with demo data and dry run (safe)
DEMO_MODE=1 DRY_RUN=1 ./target/release/agentic-mail-agent

# Or use the convenience script
DEMO_MODE=1 DRY_RUN=1 ./run_inbox_processor.sh
```

### Production Mode (Requires Gmail Setup)
```bash
# Run with Gmail API (requires credentials in ../secrets/)
./target/release/agentic-mail-agent

# Or with custom configuration
MAX_EMAILS=100 REVIEW_THRESHOLD=0.8 ./target/release/agentic-mail-agent
```

## 📋 Configuration

The binary is configured entirely through environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `MAX_EMAILS` | `50` | Maximum number of emails to process from inbox |
| `REVIEW_THRESHOLD` | `0.7` | Confidence threshold below which emails need review |
| `CLASSIFIER_TYPE` | `stub` | Classifier type: `stub`, `langchain`, or `hybrid` |
| `DEMO_MODE` | (unset) | Use demo data instead of Gmail API |
| `DRY_RUN` | (unset) | Don't make actual changes to Gmail |

## 🎯 Email Processing Logic

The binary implements the following logic for each email:

1. **Classification**: Classify email into categories (ActionRequired, Reference, Noise, Spam, etc.)
2. **Urgency Detection**: Check for urgent keywords in subject/content
3. **Decision Logic**:
   - **Keep in Inbox**: ActionRequired emails, urgent emails, low-confidence classifications
   - **Archive**: Everything else (but retain labels)

### Examples

- **ActionRequired** → Always stays in inbox
- **Urgent keywords** (URGENT, ASAP, etc.) → Always stays in inbox  
- **Low confidence** (< threshold) → Stays in inbox for review
- **High confidence Noise/Spam** → Gets archived with labels

## 🔧 Building

```bash
# Debug build
cargo build

# Release build (recommended)
cargo build --release
```

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Test with demo data
DEMO_MODE=1 DRY_RUN=1 ./target/release/agentic-mail-agent

# Test different classifiers
DEMO_MODE=1 CLASSIFIER_TYPE=langchain ./target/release/agentic-mail-agent
```

## 📊 Output

The binary provides detailed logging and a summary:

```
🤖 Agentic Gmail Agent - Inbox Processor
=========================================
📋 Configuration:
  • Max emails to process: 50
  • Review threshold: 0.70
  • Classifier type: stub
  • Demo mode: true
  • Dry run: true

📧 Processing email 1 of 5: demo-1
  📋 Subject: Welcome to Agentic Mail Agent
  🎯 Classification: Reference (confidence: 0.60)
  📥 INBOX: 📝 - Low confidence (0.60) - needs review

📊 Processing Summary
====================
📧 Total emails processed: 5
📥 Kept in inbox: 3
📦 Archived: 2
🚨 Urgent emails: 1
🔍 Needs review: 2
```

## 🔐 Security

- **Dry Run Mode**: Use `DRY_RUN=1` to test without making changes
- **Demo Mode**: Use `DEMO_MODE=1` to test with sample data
- **Gmail Credentials**: Store in `../secrets/` directory (see Gmail setup guide)

## 📝 Customization

The binary supports different classifier types:

- **`stub`**: Fast, deterministic, good for testing
- **`langchain`**: Uses local Ollama LLM for intelligent classification
- **`hybrid`**: Combines multiple classifiers (future feature)

## 🐛 Troubleshooting

- **Binary not found**: Run `cargo build --release` first
- **Gmail API errors**: Check credentials in `../secrets/`
- **Ollama errors**: Ensure `ollama serve` is running for LangChain classifier
- **Permission errors**: Ensure script is executable (`chmod +x`)

## 📖 See Also

- `../README.md` - Project overview
- `../GMAIL_SETUP.md` - Gmail API setup
- `../LLM_CLASSIFICATION_GUIDE.md` - LLM classifier setup
