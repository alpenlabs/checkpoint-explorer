#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

echo "Building backend..."
cargo build --manifest-path ../backend/Cargo.toml 2>&1

uv run python entry.py "$@"
