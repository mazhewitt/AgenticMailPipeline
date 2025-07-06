# Development Guide

## Quick Start for Contributors

### Before You Commit

**Always run these commands before committing:**

```bash
make ci-local    # Run exact same checks as CI
```

Or individually:
```bash
make fmt         # Format code
make clippy      # Check with strict clippy settings  
make test        # Run all tests
```

### Code Quality Standards

This project enforces strict code quality standards:

- **Clippy**: All warnings treated as errors (`-D warnings`)
- **Format strings**: Must use modern syntax (`-D clippy::uninlined_format_args`)
- **Formatting**: All code must be formatted with `cargo fmt`
- **Tests**: All tests must pass

### Development Workflow

1. **Make your changes**
2. **Run quality checks**: `make ci-local`
3. **Fix any issues**: `make fix` (auto-fixes what it can)
4. **Commit and push**

### Available Make Targets

- `make check` - Quick check (tests + clippy)
- `make ci-local` - **Run this before every commit!**
- `make clippy` - Clippy with strict settings (matches CI)
- `make fmt` - Format all code
- `make fix` - Auto-fix issues where possible
- `make test` - Run all tests
- `make help` - Show all available targets

### VS Code Setup

The project includes VS Code settings that will:
- Run clippy with strict settings in real-time
- Format code on save
- Show all warnings and errors inline

### Pre-commit Hooks (Optional)

Install pre-commit hooks to automatically run checks:

```bash
# Install pre-commit (if not already installed)
pip install pre-commit

# Install the hooks
pre-commit install
```

### Common Issues and Solutions

#### Format String Warnings
❌ **Wrong**: `println!("Value: {}", variable)`
✅ **Correct**: `println!("Value: {variable}")`

#### Missing Clippy Checks
Always run clippy with the same flags as CI:
```bash
cargo clippy --all-targets --all-features -- -D warnings -D clippy::uninlined_format_args
```

#### CI Failing Locally Passes
Use `make ci-local` to run the exact same checks as CI.

### Debugging CI Issues

If CI fails but local checks pass:
1. Ensure you're running `make ci-local` (not just `cargo test`)
2. Check you have the latest Rust version
3. Verify all files are committed (CI might see different files)

### Getting Help

- Check `make help` for available commands
- Run `make ci-local` to reproduce CI environment
- All quality checks must pass before merging