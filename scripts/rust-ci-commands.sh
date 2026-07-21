#!/usr/bin/env bash
# Shared Rust verification commands used by both CI and local testing.
# Assumes:
#   - CWD is the rust/ workspace root
#   - DATABASE_URL, NATS_URL, SQLX_OFFLINE, RUST_MIN_STACK are set
#   - Postgres and NATS are reachable
#   - sqlx-cli is installed

set -euo pipefail

echo "==> Running migrations..."
sqlx migrate run --source web/migrations

echo "==> Checking formatting..."
cargo fmt --all -- --check

echo "==> Running clippy (workspace, excluding web)..."
cargo clippy --workspace --exclude web --all-targets -- -D warnings

echo "==> Running clippy (web, ssr feature)..."
cargo clippy -p web --all-targets --features ssr -- -D warnings

echo "==> Checking sqlx prepared queries..."
(cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)

echo "==> Running tests (workspace, excluding web)..."
cargo test --workspace --exclude web

echo "==> Running tests (web, ssr feature)..."
cargo test -p web --features ssr

echo "==> All checks passed."
