#!/bin/bash
# Test script to demonstrate the auth setup process without real credentials

echo "🧪 Gmail OAuth2 Setup - Demo Mode"
echo "================================="
echo ""

# Create a temporary valid client secret for demo
DEMO_SECRET='{
  "installed": {
    "client_id": "demo-client-id.apps.googleusercontent.com",
    "project_id": "demo-project",
    "auth_uri": "https://accounts.google.com/o/oauth2/auth",
    "token_uri": "https://oauth2.googleapis.com/token",
    "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
    "client_secret": "demo-client-secret",
    "redirect_uris": ["http://localhost"]
  }
}'

# Backup existing file if it exists
if [[ -f "./secrets/client-secret.json" ]]; then
    cp "./secrets/client-secret.json" "./secrets/client-secret.json.backup"
    echo "📄 Backed up existing client secret to client-secret.json.backup"
fi

# Create demo client secret
echo "$DEMO_SECRET" > "./secrets/client-secret.json"
echo "📄 Created demo client secret file"

# Test the validation part of the setup script
echo ""
echo "Testing validation..."

# Source the setup script functions
source ./setup_gmail_auth.sh 2>/dev/null || true

# Test jq validation
if command -v jq >/dev/null 2>&1; then
    echo "✅ jq is available for JSON validation"
    if jq . "./secrets/client-secret.json" > /dev/null 2>&1; then
        echo "✅ Client secret file has valid JSON format"
    else
        echo "❌ Client secret file has invalid JSON format"
    fi
    
    if jq -e '.installed.client_id' "./secrets/client-secret.json" > /dev/null 2>&1; then
        echo "✅ Client secret file has expected OAuth2 structure"
    else
        echo "❌ Client secret file missing OAuth2 structure"
    fi
else
    echo "⚠️  jq not found - JSON validation will be skipped in the real setup"
fi

# Restore backup if it exists
if [[ -f "./secrets/client-secret.json.backup" ]]; then
    mv "./secrets/client-secret.json.backup" "./secrets/client-secret.json"
    echo ""
    echo "📄 Restored original client secret file"
else
    rm "./secrets/client-secret.json"
    echo ""
    echo "📄 Removed demo client secret file"
fi

echo ""
echo "✅ Demo completed successfully!"
echo ""
echo "Next steps to set up real OAuth2:"
echo "1. Get your OAuth2 client secret from Google Cloud Console"
echo "2. Save it as ./secrets/client-secret.json"
echo "3. Run: ./setup_gmail_auth.sh"
