#!/bin/bash
# Integration test runner for classifier and labeller tests
# 
# This script runs comprehensive integration tests that:
# 1. Fetch real emails from your Gmail account
# 2. Classify them using the current classifier
# 3. Apply temporary test labels (TEST_AGENT_*)
# 4. Verify the labels were applied correctly
# 5. Clean up by removing all test labels
#
# Requirements:
# - Gmail API credentials configured (run ./setup_gmail_auth.sh first)
# - Environment variables set (run source ./set_gmail_env.sh)

set -e

echo "🧪 CLASSIFIER + LABELLER INTEGRATION TESTS"
echo "=========================================="
echo

# Check if environment is set up
if [[ -z "$GMAIL_CLIENT_SECRET_JSON" ]] || [[ -z "$GMAIL_TOKEN_JSON" ]]; then
    echo "❌ Gmail environment variables not set"
    echo "💡 Run: source ./set_gmail_env.sh"
    exit 1
fi

if [[ ! -f "$GMAIL_CLIENT_SECRET_JSON" ]] || [[ ! -f "$GMAIL_TOKEN_JSON" ]]; then
    echo "❌ Gmail credential files not found"
    echo "💡 Run: ./setup_gmail_auth.sh"
    exit 1
fi

echo "✅ Gmail credentials found"
echo

# Available test functions
echo "📋 Available integration tests:"
echo "  1. Full workflow test (fetch → classify → label → verify → cleanup)"
echo "  2. Classifier quality assessment with real emails"
echo "  3. Label management test (create, list, delete)"
echo "  4. End-to-end workflow test"
echo

# Choose which tests to run
if [[ "$1" == "all" ]]; then
    TESTS_TO_RUN="test_classifier_labeller_integration_full_workflow test_classifier_with_real_emails_quality_assessment test_labeller_label_management test_end_to_end_workflow_with_cleanup"
elif [[ "$1" == "quick" ]]; then
    TESTS_TO_RUN="test_classifier_labeller_integration_full_workflow"
elif [[ "$1" == "quality" ]]; then
    TESTS_TO_RUN="test_classifier_with_real_emails_quality_assessment"
elif [[ "$1" == "labels" ]]; then
    TESTS_TO_RUN="test_labeller_label_management"
elif [[ "$1" == "workflow" ]]; then
    TESTS_TO_RUN="test_end_to_end_workflow_with_cleanup"
elif [[ -n "$1" ]]; then
    TESTS_TO_RUN="$1"
else
    TESTS_TO_RUN="test_classifier_labeller_integration_full_workflow"
fi

echo "🎯 Running tests: $TESTS_TO_RUN"
echo

# Warning about modifying Gmail account
echo "⚠️  WARNING: These tests will modify your Gmail account:"
echo "   - Temporary TEST_AGENT_* labels will be created"
echo "   - These labels will be applied to your real emails"
echo "   - All test labels will be cleaned up automatically"
echo "   - Your original emails will NOT be modified (only labeled)"
echo

read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

echo

# Run the selected tests
for test_name in $TESTS_TO_RUN; do
    echo "🧪 Running: $test_name"
    echo "   Command: cargo test --test integration $test_name -- --ignored"
    echo
    
    # Run the test with timeout to avoid hanging
    if timeout 300 cargo test --test integration "$test_name" -- --ignored; then
        echo "✅ $test_name PASSED"
    else
        echo "❌ $test_name FAILED"
        echo "💡 Check the output above for details"
        
        # Ask if we should continue with other tests
        if [[ $(echo "$TESTS_TO_RUN" | wc -w) -gt 1 ]]; then
            read -p "Continue with remaining tests? (y/N): " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                exit 1
            fi
        else
            exit 1
        fi
    fi
    echo
done

echo "🎉 All integration tests completed!"
echo
echo "📊 What was tested:"
echo "  - Real Gmail API integration"
echo "  - Email classification accuracy"
echo "  - Label creation and application"
echo "  - Idempotent operations"
echo "  - Cleanup functionality"
echo
echo "✅ Your Gmail account is back to its original state"
echo "   (all TEST_AGENT_* labels have been removed)"