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
- ✅ Email fetching from Gmail inbox (read-only, secure)
- ✅ Email classification system with MessageClassifier trait
- ✅ Comprehensive test suite (unit + integration tests)
- ✅ Clean, modular Rust codebase following TDD principles
- ✅ Complete documentation and setup guides
- ✅ Environment variable management
- ✅ Error handling and validation

**Current Functionality:**
- Fetches unread emails from Gmail inbox
- Extracts email subject and snippet (body preview) for each email
- Classifies emails by category (work, personal, promotional, spam, etc.)
- Uses Gmail API with read-only permissions
- Secure OAuth2 authentication flow
- Fallback to stub fetcher when credentials unavailable
- Full integration testing

## Email Classification

The system includes a flexible email classification module that can categorize emails automatically:

**Classification Categories:**
- `work` - Work-related emails, meetings, professional correspondence
- `personal` - Personal emails from family and friends
- `promotional` - Marketing emails and promotional content
- `spam` - Spam and unwanted emails
- `newsletter` - Newsletters and regular updates
- `urgent` - Time-sensitive emails requiring immediate attention

**Current Implementation:**
- **StubClassifier**: Deterministic rule-based classifier for development and testing
- Uses email subject and snippet content for classification
- Returns confidence scores (0.0 to 1.0) for classification results
- Provides detailed classification responses for audit purposes

**Architecture:**
- `MessageClassifier` trait for pluggable classification implementations
- `Classification` struct with category, confidence score, and LLM response
- `ClassificationError` enum for robust error handling
- Async-ready design for future LLM integration

**Ready for Extension:**
- LLM integration (Ollama client)
- Advanced ML-based classification
- Action routing and automation
- Containerization

## Utilities

- `./setup_gmail_auth.sh` - Complete OAuth2 setup automation
- `./check_setup.sh` - Verify current setup status
- `./test_integration.sh` - Comprehensive testing
- `./test_setup.sh` - Demo mode testing

## License
MIT
