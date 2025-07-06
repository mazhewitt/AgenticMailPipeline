#!/bin/bash

# Agentic Mail Agent - Inbox Processor
# 
# This script runs the main email processing binary with configurable options.
# 
# Configuration Environment Variables:
# - MAX_EMAILS: Maximum number of emails to process (default: 50)
# - REVIEW_THRESHOLD: Confidence threshold for review (default: 0.7)
# - CLASSIFIER_TYPE: Type of classifier ("stub", "langchain") (default: "stub")  
# - DEMO_MODE: Use demo data instead of Gmail API (set to any value to enable)
# - DRY_RUN: Don't make actual changes (set to any value to enable)

cd "$(dirname "$0")"

echo "🤖 Agentic Gmail Agent - Inbox Processor Script"
echo "==============================================="
echo ""

# Check if binary exists
if [ ! -f "./target/release/agentic-mail-agent" ]; then
    echo "❌ Binary not found. Building..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "❌ Build failed!"
        exit 1
    fi
fi

# Show current configuration
echo "📋 Current Configuration:"
echo "  • MAX_EMAILS: ${MAX_EMAILS:-50}"
echo "  • REVIEW_THRESHOLD: ${REVIEW_THRESHOLD:-0.7}"
echo "  • CLASSIFIER_TYPE: ${CLASSIFIER_TYPE:-stub}"
echo "  • DEMO_MODE: ${DEMO_MODE:-not set}"
echo "  • DRY_RUN: ${DRY_RUN:-not set}"
echo ""

# Run the binary
./target/release/agentic-mail-agent

echo ""
echo "✅ Script completed!"
