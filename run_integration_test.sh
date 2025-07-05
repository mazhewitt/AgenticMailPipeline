#!/bin/bash
# Quick integration test runner
# Usage: ./run_integration_test.sh [test_name]

set -e

# Default to the main workflow test
TEST_NAME="${1:-test_classifier_labeller_integration_full_workflow}"

echo "🧪 Running integration test: $TEST_NAME"
echo

# Check environment
if [[ -z "$GMAIL_CLIENT_SECRET_JSON" ]] || [[ -z "$GMAIL_TOKEN_JSON" ]]; then
    echo "❌ Gmail environment not set. Run: source ./set_gmail_env.sh"
    exit 1
fi

echo "✅ Environment ready"
echo

# Run the test
cargo test --test integration "$TEST_NAME" -- --ignored

echo
echo "✅ Test completed!"