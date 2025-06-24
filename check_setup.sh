#!/bin/bash
# Gmail OAuth2 Setup Help - Shows what you need to do

echo "📋 Gmail API Setup Checklist"
echo "============================"
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Step 1: Get OAuth2 Credentials${NC}"
echo "1. Go to: https://console.developers.google.com/apis/credentials"
echo "2. Create a new project or select existing one"
echo "3. Enable the Gmail API for your project"
echo "4. Click '+ CREATE CREDENTIALS' → 'OAuth client ID'"
echo "5. Choose 'Desktop application' as the application type"
echo "6. Download the JSON file (it will be named like 'client_secret_xxx.json')"
echo ""

echo -e "${BLUE}Step 2: Place the File${NC}"
echo "7. Rename the downloaded file to 'client-secret.json'"
echo "8. Place it in: ./secrets/client-secret.json"
echo ""

echo -e "${BLUE}Step 3: Run Setup${NC}"
echo "9. Run: ./setup_gmail_auth.sh"
echo "10. Follow the browser authentication flow"
echo ""

echo -e "${BLUE}Current Status:${NC}"

# Check if client secret exists
if [[ -f "./secrets/client-secret.json" ]]; then
    echo -e "✅ Client secret file: ${GREEN}Found${NC}"
    
    # Check if it's valid JSON
    if command -v jq >/dev/null 2>&1 && jq . "./secrets/client-secret.json" > /dev/null 2>&1; then
        echo -e "✅ JSON format: ${GREEN}Valid${NC}"
        
        # Check OAuth2 structure
        if jq -e '.installed.client_id' "./secrets/client-secret.json" > /dev/null 2>&1; then
            echo -e "✅ OAuth2 format: ${GREEN}Valid${NC}"
            echo ""
            echo -e "${GREEN}Ready to run: ./setup_gmail_auth.sh${NC}"
        else
            echo -e "❌ OAuth2 format: ${YELLOW}Invalid (missing 'installed' section)${NC}"
            echo "   This looks like an API key, not OAuth2 client secret"
        fi
    else
        echo -e "❌ JSON format: ${YELLOW}Invalid${NC}"
        echo "   File exists but contains invalid JSON"
    fi
else
    echo -e "❌ Client secret file: ${YELLOW}Missing${NC}"
    echo "   Please download from Google Cloud Console"
fi

# Check if token exists
if [[ -f "./secrets/token.json" ]]; then
    echo -e "✅ OAuth token: ${GREEN}Found${NC}"
    echo ""
    echo -e "${GREEN}Authentication complete! You can run: cargo run${NC}"
else
    echo -e "❌ OAuth token: ${YELLOW}Missing${NC}"
    echo "   Will be created during setup"
fi

echo ""
echo -e "${BLUE}Need help?${NC}"
echo "• See detailed guide: ./GMAIL_SETUP.md"
echo "• Test validation: ./test_setup.sh"
echo "• Run setup: ./setup_gmail_auth.sh"
