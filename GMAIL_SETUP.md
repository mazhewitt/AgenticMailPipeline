# Gmail OAuth2 Setup Guide

This guide helps you set up OAuth2 authentication for Gmail API access.

## Quick Setup

Run the automated setup script:

```bash
./setup_gmail_auth.sh
```

This script will:
1. Check for the required client secret file
2. Guide you through obtaining OAuth2 credentials if needed
3. Run the OAuth2 flow to obtain and save your token
4. Create environment variable scripts for easy usage

## Manual Setup

If you prefer to set up manually:

### 1. Get OAuth2 Credentials

1. Go to [Google Cloud Console](https://console.developers.google.com/apis/credentials)
2. Create a new project or select an existing one
3. Enable the Gmail API
4. Create OAuth2 credentials for **"Desktop application"** (not web application or API key)
5. Download the client secret JSON file
6. Save it as `./secrets/client-secret.json`

**Important:** Make sure you download the OAuth2 client secret JSON (not an API key). The file should contain a structure like:

```json
{
  "installed": {
    "client_id": "your-client-id.apps.googleusercontent.com",
    "project_id": "your-project-id",
    "auth_uri": "https://accounts.google.com/o/oauth2/auth",
    "token_uri": "https://oauth2.googleapis.com/token",
    "client_secret": "your-client-secret",
    "redirect_uris": ["http://localhost"]
  }
}
```

See `./secrets/client-secret-template.json` for a complete template.

### 2. Run OAuth2 Setup

```bash
# Set environment variables
export GMAIL_CLIENT_SECRET_JSON="./secrets/client-secret.json"
export GMAIL_TOKEN_JSON="./secrets/token.json"

# Run the OAuth2 setup script (automated)
./setup_gmail_auth.sh

# Or set up manually if needed
```

### 3. Set Environment Variables

After successful setup, source the generated environment script:

```bash
source ./set_gmail_env.sh
```

## Required Scopes

The application uses these OAuth2 scopes:
- `https://www.googleapis.com/auth/gmail.readonly` - Read-only access to Gmail

## Security Notes

- The token file contains sensitive authentication data - keep it secure
- The application only requests read-only access to Gmail
- All authentication happens locally - no data is sent to external services
- Tokens are stored locally and can be revoked from your Google Account settings

## Testing

Test the setup with:

```bash
# Run the main application
cargo run

# Run integration tests (requires valid credentials)
cargo test -- --ignored
```

## Troubleshooting

### "Client secret file not found"
- Ensure you've downloaded the client secret JSON from Google Cloud Console
- Check the file path matches the environment variable

### "OAuth2 setup failed"
- Check your internet connection
- Ensure the Gmail API is enabled in your Google Cloud project
- Verify the client secret file is valid JSON

### "Failed to obtain valid token"
- Try running the setup again
- Check if you completed the browser authentication flow
- Ensure you granted the requested permissions
