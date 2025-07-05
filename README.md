# Agentic Mail Agent

**Agentic Mail Agent** is an autonomous, container-ready Rust CLI tool designed to periodically read, classify, and process Gmail inbox messages using local LLMs (served via Ollama). The project is engineered for modularity, security, extensibility, and test-driven development.

---

## 🚩 Goal

- **Automate Gmail Inbox Management:**  
  The system autonomously fetches Gmail messages, classifies them using local LLMs, and triggers configurable actions—enabling secure, ethical, and efficient email processing.
- **Agentic by Design:**  
  The codebase is built for extensible, autonomous operation, enabling future features like search, advanced labeling, and memory modules.

---

## 🛠️ Core Principles & Rules

- **Test-Driven Development (TDD):**  
  All business logic and modules are developed using TDD; unit and integration tests are first-class.
- **Modularity:**  
  Code is organized into small, composable, testable Rust modules with clearly defined traits and interfaces.
- **Privacy & Security:**  
  OAuth2 credentials are handled securely; API usage is safe (read-only unless explicitly expanded); audit trails/logging are maintained.
- **Local-First LLMs:**  
  LLM inference is performed via Ollama-hosted models running on the same hardware (no external AI cloud).
- **Observability:**  
  All agentic actions are logged and auditable. Metrics can be exported or extended.
- **Enterprise Ready:**  
  Design is idiomatic, robust, and suitable for future scale and containerization.
- **AI Collaboration:**  
  The project welcomes development by both humans and AI agents (e.g., GitHub Copilot, ChatGPT), following these rules as coding “guardrails.”

---

## 🏗️ High-Level Architecture

```plaintext
+---------------------------+
|   Container Orchestrator  |
|    (Docker/Helm/K8s)      |
+-----------+---------------+
            |
+-----------v-------------------+
|      Rust Agentic Core        |
|------------------------------|
|  - Agent Orchestrator         |
|  - Fetcher (Gmail API)        |
|  - Classifier (LLM Client)    |
|  - Action Router              |
|  - Logging & Auditing         |
|  - Optional: Indexer/Vector   |
+-----------+-------------------+
            |
+-----------v-----------+
|  Gmail API Adapter    |
+-----------------------+
            |
+-----------v----------+
|   Gmail Inbox (Cloud)|
+----------------------+
            |
+-----------v----------+
|     Ollama LLM API   |
+----------------------+

 Key Modules
	•	Fetcher:
Fetches messages from Gmail inbox (read-only by default), with pluggable strategies and testable traits.
	•	Classifier:
Invokes local LLM via Ollama to classify messages.
	•	Action Router:
Decides what to do with each message (label, archive, escalate, etc.).
	•	Logging/Auditing:
Ensures all actions are tracked for traceability.
	•	Test Suite:
Comprehensive tests for all modules, run in CI.

⸻

📝 Development Rules for Agentic AI (and Humans)
	•	Always write modular, testable Rust.
	•	Each PR/increment must include or update relevant unit and/or integration tests.
	•	Never commit credentials; use environment variables/secrets for config.
	•	Keep Gmail operations read-only unless “write” scope is justified and documented.
	•	Use clear Rustdoc for all public types and functions.
	•	Follow and extend this architecture and these principles as the project grows.

⸻

🔐 Security & Ethics
	•	OAuth2 flow is headless and local.
	•	No email content leaves the local environment (LLMs are run locally).
	•	All logs are privacy-respecting and auditable.
	•	Actions are always safe and reversible (by default).



⸻

This README is both a project guide and a contract for AI and human contributors—future development should respect and extend these rules and architecture.

## Getting Started

### Quick Start

1. **Set up Gmail OAuth2 credentials:**
   ```sh
   ./setup_gmail_auth.sh
   ```

2. **Run the application:**
   ```sh
   source ./set_gmail_env.sh
   cargo run --bin agentic-mail-agent
   ```

3. **Test everything works:**
   ```sh
   ./test_integration.sh
   ```

### Manual Setup

For detailed setup instructions, see [GMAIL_SETUP.md](./GMAIL_SETUP.md).

## Gmail API OAuth2 Setup

**Warning:** The GmailFetcher requires OAuth2 credentials and token files. These must be provided via environment variables and should be kept secure. Only use read-only scopes for safety.

### Quick Setup (Recommended)

Run the automated setup script:

```sh
./setup_gmail_auth.sh
```

This will guide you through the entire OAuth2 setup process, including:
- Checking for required client secret file
- Running the OAuth2 flow to obtain tokens
- Setting up environment variables automatically

### Manual Setup

If you prefer manual setup, see [GMAIL_SETUP.md](./GMAIL_SETUP.md) for detailed instructions.

**Required files:**
- Client secret JSON from Google Cloud Console
- OAuth2 token (generated during setup)
- Minimum scope: `https://www.googleapis.com/auth/gmail.readonly`

To run the integration test:
```sh
cargo test -- --ignored
```

## ✅ Implementation Status

**Completed Features:**
- ✅ Gmail API integration with OAuth2 authentication
- ✅ Automated OAuth2 setup script (`./setup_gmail_auth.sh`)
- ✅ Email fetching from Gmail inbox (secure with read/modify permissions)
- ✅ Email classification system with MessageClassifier trait
- ✅ **Gmail labeling based on classification results**
- ✅ **Automatic label creation and management**
- ✅ **Idempotent labeling operations (safe re-application)**
- ✅ Comprehensive test suite (unit + integration tests)
- ✅ Clean, modular Rust codebase following TDD principles
- ✅ Complete documentation and setup guides
- ✅ Environment variable management
- ✅ Error handling and validation

**Current Functionality:**
- Fetches unread emails from Gmail inbox
- Extracts email subject and snippet (body preview) for each email
- Classifies emails by category (ActionRequired, InterestingInfo, Reference, Noise, Spam)
- **Automatically applies Gmail labels based on classification (AGENT_URGENT, AGENT_PERSONAL, AGENT_PROMOTIONAL, AGENT_SPAM, etc.)**
- **Creates labels if they don't exist**
- **Executes labeling operations idempotently (safe to run multiple times)**
- Uses Gmail API with modify permissions for labeling
- Secure OAuth2 authentication flow
- Fallback to stub implementations when credentials unavailable
- Full integration testing with both stub and Gmail API implementations

## Email Classification

The system includes a sophisticated email classification module that can categorize emails automatically using both AI-powered and rule-based approaches:

**Classification Categories:**
- `ActionRequired` - Something I really need to respond to, schedule, or deal with myself (meeting requests, deadlines, tasks, urgent requests, CI/CD failures)
- `InterestingInfo` - Not actionable, but possibly interesting to me (industry news, tech updates, newsletters with valuable content, security alerts)
- `Reference` - Useful to keep but not urgent or interesting (receipts, confirmations, travel notifications, service updates, terms changes)
- `Noise` - Not useful (generic newsletters, social notifications, low-value promotions, LinkedIn connections, generic marketing)
- `Spam` - Unwanted or truly spammy content (phishing attempts, scams, malicious emails, clearly unwanted solicitations)

**Current Implementation:**
- **LangChain Classifier**: AI-powered classification using local LLM (llama3.1:8b) via Ollama for intelligent, context-aware email categorization
- **StubClassifier**: Deterministic rule-based classifier with extensive pattern matching (200+ lines) for development and testing
- **Hybrid Approach**: Graceful fallback from LLM to rule-based when AI is unavailable
- Uses email subject and snippet content for classification
- Returns confidence scores (0.0 to 1.0) for classification results
- Provides detailed classification responses with explanations for audit purposes

**Architecture:**
- `MessageClassifier` trait for pluggable classification implementations
- `Classification` struct with category, confidence score, and LLM response
- `ClassificationError` enum for robust error handling
- Privacy-preserving: All LLM processing happens locally via Ollama
- Environment variable control: `CLASSIFIER_TYPE` to switch between "langchain" and "stub"

**Action Router Integration:**
- **Category-to-Action Mapping**: 
  - `ActionRequired` → `AGENT_URGENT` label + MarkImportant + Escalate
  - `InterestingInfo` → `AGENT_PERSONAL` label
  - `Reference` → `AGENT_PROMOTIONAL` label + Archive
  - `Noise` → `AGENT_PROMOTIONAL` label + Archive  
  - `Spam` → `AGENT_SPAM` label + Archive
- **Confidence Thresholds**: Low confidence classifications get `AGENT_NEEDS_REVIEW` label
- **Urgency Detection**: Additional urgency detection based on keywords in subject/body

**Ready for Extension:**
- Advanced ML-based classification
- Custom action routing rules
- Containerization and deployment automation

---

## 🎯 Current Status

The project includes **50 fully anonymized test emails** for CI/testing purposes. These emails have been processed through the PII anonymization pipeline to replace all sensitive information with realistic fake data while preserving email structure and content for testing.

**Test Data Features:**
- 50 diverse email examples covering all classification categories
- All PII anonymized: names, emails, phone numbers, addresses
- Safe for CI/CD pipelines and public repositories
- Realistic fake data maintains testing validity
- Comprehensive coverage for classifier training and validation

---

## ✨ Features

### 🤖 AI-Powered Email Classification
- **Local LLM Integration**: Uses langchain-rust with Ollama for intelligent email classification
- **Multiple Classifiers**: Choose between deterministic stub classifier or AI-powered LLM classifier
- **Privacy-First**: All LLM processing happens locally - no cloud AI services
- **Semantic Understanding**: LLM classifier understands context beyond simple keyword matching

### 🔒 PII Anonymization Pipeline
- **Intelligent PII Detection**: Uses local LLMs to detect names, emails, phone numbers, addresses
- **Safe Test Data Generation**: Anonymize emails for CI/testing without privacy concerns
- **Dual-Stage Processing**: LLM detection + Rust-based replacement with realistic fake data
- **Audit Trails**: Complete traceability of all anonymization operations
- **Multiple Backends**: Support for Ollama (local) and OpenAI (cloud) models

### 📧 Email Processing Pipeline
- **Gmail Integration**: Secure OAuth2 authentication with read-only permissions
- **Smart Classification**: Categorizes emails into ActionRequired, InterestingInfo, Reference, Noise, Spam with detailed reasoning
- **Intelligent Actions**: Automatic labeling, archiving, escalation based on classification confidence and urgency detection
- **High-Priority Detection**: Identifies and escalates urgent emails requiring immediate attention with AGENT_URGENT labels

### 🔧 Developer Experience
- **Test-Driven Development**: Comprehensive unit and integration tests
- **Modular Architecture**: Pluggable components with clean trait-based interfaces
- **Stub Implementations**: Fast development with mock data and deterministic behavior
- **Environment Configuration**: Easy switching between classifiers via environment variables

### 🔒 Security & Privacy
- **Local Processing**: Email content never leaves your machine
- **Read-Only Gmail Access**: Safe, auditable Gmail API usage
- **Secure Credential Management**: OAuth2 tokens stored securely
- **Audit Trails**: All actions logged for transparency and compliance

---

## 🚀 Quick Start

### Using Stub Classification (Development)
```bash
# Clone and build
git clone <repository>
cd agentic-mail-agent
cargo build

# Run with stub classifier (fast, deterministic)
cargo run --bin agentic-mail-agent
```

### Using AI Classification (Production)
```bash
# Install and start Ollama
ollama pull llama3:8b
ollama serve

# Run with LLM classifier
CLASSIFIER_TYPE=langchain cargo run --bin agentic-mail-agent
```

For detailed setup instructions, see:
- [Gmail Setup Guide](GMAIL_SETUP.md) - Configure Gmail API access
- [LLM Classification Guide](LLM_CLASSIFICATION_GUIDE.md) - Set up local AI classification
- [PII Anonymization Guide](PII_ANONYMIZATION_GUIDE.md) - Create safe test data with PII anonymization

## Utilities

### Setup & Authentication
- `./setup_gmail_auth.sh` - Complete OAuth2 setup automation
- `./check_setup.sh` - Verify current setup status
- `./set_gmail_env.sh` - Set Gmail environment variables

### Testing & Development
- `./test_integration.sh` - Comprehensive testing
- `./test_setup.sh` - Demo mode testing
- `./demo_pii_pipeline.sh` - Demonstrate PII anonymization pipeline
- `./test_pii_pipeline.sh` - Test PII anonymization functionality

### Data Management
- `cargo run --bin download_test_data` - Download emails for testing
- `cargo run --bin orchestrated_pii_anonymize` - Anonymize emails for safe CI use
- `cargo run --bin check_anonymized_data` - Verify anonymization quality
- `cargo run --bin test_with_real_data` - Test classifier with real data

## License
MIT
