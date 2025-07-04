# Makefile for agentic-mail-agent

.PHONY: check test clippy audit clean all

# Quick check: tests and clippy on library code
check:
	@echo "🧪 Running tests..."
	@cargo test --lib
	@echo "📋 Running clippy on library..."
	@cargo clippy --lib -- -D warnings
	@echo "✅ Quick check completed successfully!"

# Full test suite (all test categories)
test:
	@echo "🧪 Running all tests..."
	@cargo test --all
	@echo "✅ All tests completed!"

# Clippy linting with warnings as errors
clippy:
	@echo "📋 Running clippy..."
	@cargo clippy --lib -- -D warnings
	@echo "✅ Clippy checks passed!"

# Security audit
audit:
	@echo "🔒 Running security audit..."
	@cargo audit
	@echo "✅ Security audit completed!"

# Full CI pipeline: test + clippy + audit
ci: test clippy audit
	@echo "🚀 Full CI pipeline completed successfully!"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Cleanup completed!"

# Default target
all: check

# Show available targets
help:
	@echo "Available targets:"
	@echo "  check  - Quick check (lib tests + clippy)"
	@echo "  test   - Run all tests"
	@echo "  clippy - Run clippy linting"
	@echo "  audit  - Run security audit"
	@echo "  ci     - Full CI pipeline"
	@echo "  clean  - Clean build artifacts"
	@echo "  help   - Show this help"