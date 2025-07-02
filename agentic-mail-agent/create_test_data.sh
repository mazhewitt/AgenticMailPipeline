#!/bin/bash

# Script to download 50 emails and create anonymized test data
# This script uses the existing tools to create test data for the repository

set -e

echo "🚀 Creating Anonymized Test Data"
echo "================================="

# Configuration
RAW_DATA_DIR="temp_test_data_raw"
ANONYMIZED_DATA_DIR="test_data/anonymized_emails"

# Check for Gmail authentication
if [ ! -f "../secrets/client-secret.json" ] || [ ! -f "../secrets/token.json" ]; then
    echo "❌ Gmail authentication not found!"
    echo "   Please run the Gmail setup first:"
    echo "   ../setup_gmail_auth.sh"
    exit 1
fi

# Clean up any existing directories
echo "🧹 Cleaning up previous runs..."
rm -rf "$RAW_DATA_DIR"
rm -rf "$ANONYMIZED_DATA_DIR"

# Step 1: Download 50 emails from Gmail
echo ""
echo "📧 Step 1: Downloading 50 emails from Gmail inbox..."
export EMAIL_COUNT=50
export TEST_DATA_DIR="$RAW_DATA_DIR"

cargo run --bin download_test_data

if [ ! -d "$RAW_DATA_DIR" ] || [ -z "$(ls -A $RAW_DATA_DIR 2>/dev/null)" ]; then
    echo "❌ Failed to download emails"
    exit 1
fi

EMAIL_COUNT=$(find "$RAW_DATA_DIR" -name "*.json" | wc -l | xargs)
echo "✅ Downloaded $EMAIL_COUNT emails"

# Step 2: Anonymize the emails using PII pipeline
echo ""
echo "🔒 Step 2: Anonymizing emails using PII pipeline..."

# Check if Ollama is running (required for anonymization)
if ! pgrep -f "ollama" > /dev/null; then
    echo "⚠️  Ollama is not running. Starting Ollama..."
    if command -v ollama > /dev/null; then
        ollama serve &
        OLLAMA_PID=$!
        echo "   Waiting for Ollama to start..."
        sleep 5
    else
        echo "❌ Ollama is not installed. Please install it first:"
        echo "   https://ollama.ai/"
        exit 1
    fi
fi

# Run the anonymization
mkdir -p "$ANONYMIZED_DATA_DIR"
cargo run --bin pii_anonymize -- \
    --input-dir "$RAW_DATA_DIR" \
    --output-dir "$ANONYMIZED_DATA_DIR" \
    --backend ollama

if [ ! -d "$ANONYMIZED_DATA_DIR" ] || [ -z "$(ls -A $ANONYMIZED_DATA_DIR 2>/dev/null)" ]; then
    echo "❌ Failed to anonymize emails"
    exit 1
fi

ANONYMIZED_COUNT=$(find "$ANONYMIZED_DATA_DIR" -name "*.json" | wc -l | xargs)
echo "✅ Anonymized $ANONYMIZED_COUNT emails"

# Step 3: Basic PII spot check
echo ""
echo "🔍 Step 3: Performing basic PII spot check..."

# Simple grep-based check for obvious PII patterns
PII_WARNINGS=0

echo "   Checking for real email patterns..."
REAL_EMAILS=$(grep -r -l "@" "$ANONYMIZED_DATA_DIR" | xargs grep -o '[a-zA-Z0-9._%+-]*@[a-zA-Z0-9.-]*\.[a-zA-Z]{2,}' | grep -v 'example\.com' | grep -v 'user[0-9]*@' | head -5)
if [ ! -z "$REAL_EMAILS" ]; then
    echo "   ⚠️  Potential real emails found:"
    echo "$REAL_EMAILS" | sed 's/^/      /'
    PII_WARNINGS=$((PII_WARNINGS + 1))
fi

echo "   Checking for phone number patterns..."
PHONE_NUMBERS=$(grep -r -E '\([0-9]{3}\) [0-9]{3}-[0-9]{4}|\b[0-9]{3}-[0-9]{3}-[0-9]{4}\b' "$ANONYMIZED_DATA_DIR" | grep -v '555' | head -3)
if [ ! -z "$PHONE_NUMBERS" ]; then
    echo "   ⚠️  Potential real phone numbers found:"
    echo "$PHONE_NUMBERS" | sed 's/^/      /'
    PII_WARNINGS=$((PII_WARNINGS + 1))
fi

echo "   Checking for common real names..."
REAL_NAMES=$(grep -r -iE '\b(John|Jane|Michael|Sarah|David|Emily|Robert|Jessica|William|Ashley)\b' "$ANONYMIZED_DATA_DIR" | head -3)
if [ ! -z "$REAL_NAMES" ]; then
    echo "   ⚠️  Potential real names found:"
    echo "$REAL_NAMES" | sed 's/^/      /'
    PII_WARNINGS=$((PII_WARNINGS + 1))
fi

# Step 4: Clean up raw data
echo ""
echo "🧹 Step 4: Cleaning up raw data..."
rm -rf "$RAW_DATA_DIR"
echo "✅ Raw data cleaned up"

# Step 5: Summary
echo ""
echo "📊 Summary:"
echo "   • Downloaded emails: $EMAIL_COUNT"
echo "   • Anonymized emails: $ANONYMIZED_COUNT"
echo "   • PII warnings: $PII_WARNINGS"
echo "   • Anonymized data location: $ANONYMIZED_DATA_DIR/"
echo ""

if [ $PII_WARNINGS -gt 0 ]; then
    echo "⚠️  WARNING: Potential PII found in anonymized data!"
    echo "   Please manually review the files in $ANONYMIZED_DATA_DIR/ before committing."
    echo "   Consider running the anonymization again or manually editing the files."
else
    echo "✅ PII spot check passed - no obvious PII found."
fi

echo ""
echo "📝 Next steps:"
echo "   1. Review the anonymized emails in $ANONYMIZED_DATA_DIR/"
echo "   2. If satisfied, commit the anonymized data to the repository"
echo "   3. The test data can now be used in CI/CD pipelines"

# Stop Ollama if we started it
if [ ! -z "$OLLAMA_PID" ]; then
    echo "   4. Stopping Ollama..."
    kill $OLLAMA_PID 2>/dev/null || true
fi

echo ""
echo "✅ Test data creation complete!"
