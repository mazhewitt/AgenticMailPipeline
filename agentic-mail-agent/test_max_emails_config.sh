#!/bin/bash

# Test script to verify that the default max_emails is now 10 instead of 50

echo "🔧 Testing Default MAX_EMAILS Configuration"
echo "==========================================="

cd /Users/mazdahewitt/projects/MazMailChatBot/agentic-mail-agent/agentic-mail-agent

echo ""
echo "📋 Testing default configuration (no environment variables set)..."

# Make sure MAX_EMAILS is not set
unset MAX_EMAILS

# Run a quick test that shows the configuration
echo "Configuration test:"
cargo test test_processing_config_from_env --verbose -- --nocapture

echo ""
echo "🎯 The default MAX_EMAILS has been changed from 50 to 10 emails."
echo "   This will make testing faster and more manageable."
echo ""
echo "💡 To override this default, you can set:"
echo "   export MAX_EMAILS=20  # or any other number"
echo "   ./target/debug/agentic-mail-agent"
echo ""
echo "🔍 When running the main application, it will now fetch only 10 emails"
echo "   by default instead of 50, making the classification and labeling"
echo "   process much faster for testing and development."
