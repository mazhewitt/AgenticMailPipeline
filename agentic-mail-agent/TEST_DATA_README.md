# Test Data Downloader

This binary downloads emails from your Gmail inbox to create test data files that can be used for testing the email classifier.

## Setup

Before using this tool, make sure you have set up Gmail API authentication:

1. Set the required environment variables:
   ```bash
   export GMAIL_CLIENT_SECRET_JSON=/path/to/client_secret.json
   export GMAIL_TOKEN_JSON=/path/to/token.json
   ```

2. If you haven't set up authentication yet, run:
   ```bash
   ./setup_gmail_auth.sh
   ```

## Usage

To download emails from your inbox (default: 20 emails):

```bash
cargo run --bin download_test_data
```

To download a specific number of emails:

```bash
EMAIL_COUNT=50 cargo run --bin download_test_data
```

## Configuration

You can customize the behavior using environment variables:

- `EMAIL_COUNT`: Number of emails to download (default: 20)
- `TEST_DATA_DIR`: Directory to save test data files (default: `test_data`)
- `GMAIL_CLIENT_SECRET_JSON`: Path to OAuth2 client secret JSON file
- `GMAIL_TOKEN_JSON`: Path to OAuth2 token JSON file

Example with custom settings:
```bash
EMAIL_COUNT=50 TEST_DATA_DIR=my_test_emails cargo run --bin download_test_data
```

## Current Test Data

The repository includes 50 pre-downloaded and anonymized emails in the `test_data/` directory. These emails have been processed through the PII anonymization pipeline to remove all sensitive information while preserving structure for testing.

## Output

The tool creates:

1. **Individual email files**: `email_001.json`, `email_002.json`, etc.
   - Each file contains email metadata (ID, subject, snippet)
   - Includes download timestamp and file index
   - Suitable for loading as test data in unit tests

2. **Manifest file**: `manifest.json`
   - Summary of all downloaded emails
   - Creation timestamp and total count
   - Index of all email files with basic metadata

## Example Output Structure

```
test_data/
├── manifest.json
├── email_001.json
├── email_002.json
├── email_003.json
└── ...
```

### Email File Format

Each email file contains:
```json
{
  "id": "gmail-message-id-123",
  "subject": "Email Subject Line",
  "snippet": "Preview of email content...",
  "downloaded_at": "2025-06-30T12:00:00Z",
  "file_index": 1
}
```

### Manifest File Format

The manifest provides an overview:
```json
{
  "created_at": "2025-06-30T12:00:00Z",
  "total_emails": 20,
  "emails": [
    {
      "file_index": 1,
      "filename": "email_001.json",
      "id": "gmail-message-id-123",
      "subject": "Email Subject Line",
      "has_snippet": true
    }
  ]
}
```

## Using Test Data in Tests

You can load this test data in your unit tests:

```rust
use std::fs;
use serde_json;

#[derive(serde::Deserialize)]
struct TestDataEmail {
    id: String,
    subject: Option<String>,
    snippet: Option<String>,
    downloaded_at: String,
    file_index: usize,
}

#[test]
fn test_with_real_email_data() {
    let test_data: TestDataEmail = serde_json::from_str(
        &fs::read_to_string("test_data/email_001.json").unwrap()
    ).unwrap();
    
    // Use test_data.subject, test_data.snippet for testing
    // your classifier or other email processing logic
}
```

## Security and Privacy

- This tool only downloads email metadata (subject lines and snippets)
- Full email bodies are not downloaded to protect privacy
- Test data files may contain sensitive information - handle appropriately
- Consider gitignoring the test data directory if it contains real emails

## Troubleshooting

**Error: "Failed to initialize Gmail fetcher"**
- Ensure environment variables are set correctly
- Check that credential files exist and are readable
- Run `./setup_gmail_auth.sh` to set up authentication

**Error: "No emails found in inbox"**
- Your inbox might be empty
- Check Gmail API quotas and permissions
- Verify your OAuth2 scopes include Gmail read access

**Error: "Failed to fetch emails"**
- Check your internet connection
- Verify Gmail API is enabled in Google Cloud Console
- Check for API quota limits
