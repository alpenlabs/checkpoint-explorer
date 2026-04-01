default:
    @just --list

# Format Rust code
fmt:
    cd backend && cargo fmt

# Check Rust formatting
fmt-check:
    cd backend && cargo fmt --check

# Run clippy (warnings as errors)
lint:
    cd backend && cargo clippy --workspace --lib --bins --tests --no-deps \
        -- -D warnings -W clippy::uninlined_format_args

# Run clippy with auto-fix
lint-fix:
    cd backend && cargo clippy --workspace --lib --bins --tests --no-deps --fix --allow-dirty \
        -- -W clippy::uninlined_format_args

# Check Python style
lint-py:
    cd functional-tests && uvx ruff check .

# Build backend
build:
    cd backend && cargo build

# Run functional tests
test:
    cd functional-tests && uv run python entry.py

# Run all checks — use before pushing
check: fmt-check lint lint-py
