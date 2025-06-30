#!/bin/bash

# Gmail Test Data Workflow Demo
# This script demonstrates the complete workflow for downloading and using Gmail test data

set -e  # Exit on any error

echo "🔧 Gmail Test Data Workflow Demo"
echo "=================================="
echo

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Please run this script from the agentic-mail-agent directory"
    echo "   cd agentic-mail-agent"
    exit 1
fi

# Step 1: Check environment setup
echo "📋 Step 1: Checking environment setup..."
if [ -z "$GMAIL_CLIENT_SECRET_JSON" ] || [ -z "$GMAIL_TOKEN_JSON" ]; then
    echo "⚠️  Gmail environment variables not set"
    echo "🔧 Setting up Gmail environment..."
    
    # Check if the setup script exists
    if [ -f "../set_gmail_env.sh" ]; then
        echo "   Sourcing ../set_gmail_env.sh..."
        source ../set_gmail_env.sh
    else
        echo "❌ Gmail setup script not found. Please run:"
        echo "   source ../set_gmail_env.sh"
        exit 1
    fi
else
    echo "✅ Gmail environment variables are set"
fi

echo "   GMAIL_CLIENT_SECRET_JSON: $GMAIL_CLIENT_SECRET_JSON"
echo "   GMAIL_TOKEN_JSON: $GMAIL_TOKEN_JSON"
echo

# Step 2: Download test data
echo "📋 Step 2: Downloading Gmail test data..."
echo "🔧 Running: cargo run --bin download_test_data"
echo

if cargo run --bin download_test_data; then
    echo "✅ Test data download completed successfully"
else
    echo "❌ Test data download failed"
    exit 1
fi

echo

# Step 3: Verify test data structure
echo "📋 Step 3: Verifying test data structure..."
if [ -d "test_data" ]; then
    echo "✅ Test data directory exists"
    
    # Count files
    EMAIL_COUNT=$(ls test_data/email_*.json 2>/dev/null | wc -l)
    echo "   📁 Found $EMAIL_COUNT email files"
    
    if [ -f "test_data/manifest.json" ]; then
        echo "   📄 Manifest file exists"
        
        # Show manifest summary
        echo "   📊 Manifest summary:"
        if command -v jq >/dev/null 2>&1; then
            jq -r '"   • Created: " + .created_at' test_data/manifest.json
            jq -r '"   • Total emails: " + (.total_emails | tostring)' test_data/manifest.json
        else
            echo "   • Install 'jq' to see manifest details"
        fi
    else
        echo "   ⚠️  Manifest file missing"
    fi
    
    # Show sample file
    if [ -f "test_data/email_001.json" ]; then
        echo "   📄 Sample email file (email_001.json):"
        if command -v jq >/dev/null 2>&1; then
            jq -r '"     ID: " + .id' test_data/email_001.json
            jq -r '"     Subject: " + (.subject // "null")' test_data/email_001.json
            jq -r '"     File Index: " + (.file_index | tostring)' test_data/email_001.json
        else
            echo "     $(head -3 test_data/email_001.json)"
        fi
    fi
else
    echo "❌ Test data directory not found"
    exit 1
fi

echo

# Step 4: Run integration tests
echo "📋 Step 4: Running integration tests with real Gmail data..."
echo "🔧 Running: cargo test --test test_data_integration -- --nocapture"
echo

if cargo test --test test_data_integration -- --nocapture; then
    echo "✅ All integration tests passed"
else
    echo "❌ Some integration tests failed"
    exit 1
fi

echo

# Step 5: Usage examples
echo "📋 Step 5: Usage examples"
echo "========================"
echo
echo "🔬 You can now use this test data for:"
echo
echo "1. 📊 Classifier Testing:"
echo "   cargo test --test test_data_integration -- --nocapture"
echo
echo "2. 🧪 Manual Testing:"
echo "   # Load a single email"
echo '   let email = serde_json::from_str::<TestDataEmail>('
echo '       &fs::read_to_string("test_data/email_001.json")?'
echo '   )?;'
echo
echo "3. 📈 Batch Processing:"
echo "   # Load all emails from manifest"
echo '   let manifest = serde_json::from_str::<Manifest>('
echo '       &fs::read_to_string("test_data/manifest.json")?'
echo '   )?;'
echo
echo "4. 🎯 Classifier Integration:"
echo "   # Convert to Email objects and classify"
echo '   let email = test_email.to_email();'
echo '   let classification = classifier.classify(&email).await?;'
echo

# Step 6: Next steps
echo "📋 Step 6: Next steps"
echo "===================="
echo
echo "🎯 Suggested next steps:"
echo "• Use the test data to test different classification algorithms"
echo "• Analyze the email patterns to improve classification accuracy"
echo "• Create specific test cases for edge cases found in real data"
echo "• Benchmark classifier performance with this dataset"
echo
echo "🔧 Maintenance:"
echo "• Re-run 'cargo run --bin download_test_data' to refresh test data"
echo "• Update TEST_DATA_DIR environment variable to organize multiple datasets"
echo "• Consider gitignoring test_data/ if it contains sensitive information"
echo

echo "✅ Gmail Test Data Workflow Demo completed successfully!"
echo "🎉 Your test data is ready for email classifier development!"
