# Makefile for agentic-mail-agent

.PHONY: check test clippy audit clean all fmt fix ci-local

# Quick check: tests and clippy with strict settings
check:
	@echo "🧪 Running tests..."
	@cargo test --lib
	@echo "📋 Running clippy with strict settings..."
	@cargo clippy --all-targets --all-features -- -D warnings -D clippy::uninlined_format_args
	@echo "✅ Quick check completed successfully!"

# Full test suite (all test categories)
test:
	@echo "🧪 Running all tests..."
	@cargo test --all
	@echo "✅ All tests completed!"

# Clippy linting with warnings as errors (strict mode like CI)
clippy:
	@echo "📋 Running clippy with strict settings..."
	@cargo clippy --all-targets --all-features -- -D warnings -D clippy::uninlined_format_args
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

# Format code with cargo fmt
fmt:
	@echo "🎨 Formatting code..."
	@cargo fmt --all
	@echo "✅ Code formatting completed!"

# Auto-fix issues where possible
fix:
	@echo "🔧 Auto-fixing issues..."
	@cargo fmt --all
	@cargo fix --all-targets --all-features --allow-dirty
	@echo "✅ Auto-fix completed!"

# Run the exact same checks as CI locally
ci-local:
	@echo "🔍 Running CI checks locally..."
	@cargo fmt --all -- --check
	@cargo clippy --all-targets --all-features -- -D warnings -D clippy::uninlined_format_args
	@cargo test --all
	@echo "✅ All CI checks passed!"

# Show available targets
help:
	@echo "Available targets:"
	@echo "  check     - Quick check (lib tests + strict clippy)"
	@echo "  test      - Run all tests"
	@echo "  clippy    - Run clippy linting with strict settings"
	@echo "  fmt       - Format code with cargo fmt"
	@echo "  fix       - Auto-fix issues where possible"
	@echo "  ci-local  - Run exact same checks as CI"
	@echo "  audit     - Run security audit"
	@echo "  ci        - Full CI pipeline"
	@echo "  clean     - Clean build artifacts"
	@echo "  help      - Show this help"