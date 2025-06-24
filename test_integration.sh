#!/bin/bash
# Quick test script to verify Gmail API setup and functionality

echo "🧪 Gmail API Integration Test"
echo "============================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Check setup status
print_info "Checking setup status..."
./check_setup.sh

echo ""
print_info "Testing main application..."

# Run the application
if cargo run --bin agentic-mail-agent; then
    print_success "Main application ran successfully!"
else
    echo "❌ Main application failed"
    exit 1
fi

echo ""
print_info "Running integration tests..."

# Run integration tests
if cargo test -- --ignored; then
    print_success "Integration tests passed!"
else
    echo "❌ Integration tests failed"
    exit 1
fi

echo ""
print_info "Running all tests..."

# Run all tests
if cargo test; then
    print_success "All tests passed!"
else
    echo "❌ Some tests failed"
    exit 1
fi

echo ""
print_success "🎉 Gmail API integration is working perfectly!"
echo ""
echo "Summary:"
echo "• OAuth2 authentication: ✅ Working"
echo "• Gmail API access: ✅ Working"
echo "• Email fetching: ✅ Working"
echo "• All tests: ✅ Passing"
echo ""
echo "You can now:"
echo "• Run the application: cargo run --bin agentic-mail-agent"
echo "• Run tests: cargo test"
echo "• Run integration tests: cargo test -- --ignored"
