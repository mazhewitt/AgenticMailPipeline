#!/bin/bash
# Gmail API OAuth2 Setup Script
# This script helps you set up OAuth2 authentication for the Gmail API

set -e

echo "🔐 Gmail API OAuth2 Setup"
echo "========================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]] || [[ ! -f "agentic-mail-agent/src/main.rs" ]]; then
    print_error "Please run this script from the project root directory"
    exit 1
fi

# Create secrets directory if it doesn't exist
if [[ ! -d "secrets" ]]; then
    print_info "Creating secrets directory..."
    mkdir -p secrets
fi

# Check for client secret file  
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLIENT_SECRET_FILE="$SCRIPT_DIR/secrets/client-secret.json"
if [[ ! -f "$CLIENT_SECRET_FILE" ]]; then
    print_warning "Client secret file not found at $CLIENT_SECRET_FILE"
    echo ""
    echo "To set up Gmail API access, you need to:"
    echo "1. Go to https://console.developers.google.com/apis/credentials"
    echo "2. Create a new OAuth2 client ID for 'Desktop application'"
    echo "3. Download the client secret JSON file"
    echo "4. Save it as: $CLIENT_SECRET_FILE"
    echo ""
    echo "Minimal required scopes:"
    echo "  - https://www.googleapis.com/auth/gmail.readonly"
    echo ""
    read -p "Have you downloaded and placed the client secret file? (y/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_error "Please set up the client secret file first"
        exit 1
    fi
    
    if [[ ! -f "$CLIENT_SECRET_FILE" ]]; then
        print_error "Client secret file still not found at $CLIENT_SECRET_FILE"
        exit 1
    fi
fi

# Validate client secret file format
if ! jq . "$CLIENT_SECRET_FILE" > /dev/null 2>&1; then
    print_error "Invalid JSON format in client secret file"
    echo ""
    echo "The client secret file should be a JSON file downloaded from Google Cloud Console."
    echo "It should look like the template in: ./secrets/client-secret-template.json"
    echo ""
    echo "Current file content:"
    echo "$(head -3 "$CLIENT_SECRET_FILE")"
    echo ""
    echo "Please download the correct OAuth2 client secret JSON file from:"
    echo "https://console.developers.google.com/apis/credentials"
    exit 1
fi

# Check if it has the expected structure
if ! jq -e '.installed.client_id' "$CLIENT_SECRET_FILE" > /dev/null 2>&1; then
    print_error "Client secret file doesn't have the expected OAuth2 format"
    echo ""
    echo "The file should contain an 'installed' section with client_id, client_secret, etc."
    echo "Please ensure you downloaded the OAuth2 client secret (not an API key) from:"
    echo "https://console.developers.google.com/apis/credentials"
    echo ""
    echo "Select 'Desktop application' when creating the OAuth2 client."
    exit 1
fi

print_success "Client secret file found"

# Set environment variables with absolute paths
export GMAIL_CLIENT_SECRET_JSON="$(realpath "$CLIENT_SECRET_FILE")"
export GMAIL_TOKEN_JSON="$SCRIPT_DIR/secrets/token.json"

print_info "Environment variables set:"
echo "  GMAIL_CLIENT_SECRET_JSON=$GMAIL_CLIENT_SECRET_JSON"
echo "  GMAIL_TOKEN_JSON=$GMAIL_TOKEN_JSON"
echo ""

# Build and run the auth setup utility
print_info "Building auth setup utility..."
if ! cargo build --bin auth_setup; then
    print_error "Failed to build auth setup utility"
    exit 1
fi

print_success "Auth setup utility built successfully"
echo ""

# Run the OAuth2 flow
print_info "Starting OAuth2 flow..."
echo "This will open a browser window for authentication."
echo "Please sign in to your Gmail account and grant the requested permissions."
echo ""
read -p "Press Enter to continue..."

if ! cargo run --bin auth_setup; then
    print_error "OAuth2 setup failed"
    exit 1
fi

echo ""
print_success "OAuth2 setup completed!"
echo ""
print_info "Next steps:"
echo "1. Source the environment script: source ./set_gmail_env.sh"
echo "2. Run the main application: cargo run"
echo ""
print_info "To test the Gmail integration:"
echo "cargo test -- --ignored"
