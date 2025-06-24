# agentic-mail-agent

A modular, async-ready Rust CLI agent for Gmail using google-apis-rs.

## Features
- Async CLI using Tokio
- Ready for Google Gmail API integration
- Modular and testable structure

## Getting Started

```sh
cargo run --release
```

## Gmail API OAuth2 Setup

**Warning:** The GmailFetcher requires OAuth2 credentials and token files. These must be provided via environment variables and should be kept secure. Only use read-only scopes for safety.

1. Go to https://console.developers.google.com/apis/credentials and create an OAuth2 client for "Desktop".
2. Download the client secret JSON and set the path in your environment:
   ```sh
   export GMAIL_CLIENT_SECRET_JSON=/path/to/client_secret.json
   ```
3. Obtain a user token (using the google-apis-rs or yup-oauth2 flow) and set the path:
   ```sh
   export GMAIL_TOKEN_JSON=/path/to/token.json
   ```
4. The agent will only read message metadata/IDs (no modification or deletion).
5. **Scopes:** Use the minimum required, e.g. `https://www.googleapis.com/auth/gmail.readonly`.

To run the integration test:
```sh
cargo test -- --ignored
```

## License
MIT
