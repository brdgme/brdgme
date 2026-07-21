#!/usr/bin/env bash
# Runs the full Rust CI check suite locally: migrations, fmt, clippy, sqlx
# prepare check, and tests. Spins up temporary Postgres and NATS containers
# on non-standard ports so it won't conflict with a running dev environment.
#
# This takes several minutes (compilation + test time). It's optional but
# recommended before pushing - it catches everything CI would catch without
# waiting for GitHub Actions.

set -euo pipefail

PG_NAME="brdgme-test-pg-$$"
NATS_NAME="brdgme-test-nats-$$"

cleanup() {
  docker rm -f "$PG_NAME" 2>/dev/null || true
  docker rm -f "$NATS_NAME" 2>/dev/null || true
}
trap cleanup EXIT

# --- Start services ---
echo "==> Starting Postgres..."
docker run -d --name "$PG_NAME" \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=brdgme \
  -p 15432:5432 postgres:18

echo "==> Starting NATS..."
docker run -d --name "$NATS_NAME" \
  -p 14222:4222 -p 18222:8222 \
  nats:2.11-alpine -js -sd /data -m 8222

# --- Wait for services ---
echo "==> Waiting for Postgres..."
for i in $(seq 1 30); do
  if docker exec "$PG_NAME" pg_isready -U postgres >/dev/null 2>&1; then
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "Postgres failed to start" >&2
    exit 1
  fi
  sleep 1
done

echo "==> Waiting for NATS..."
for i in $(seq 1 30); do
  if curl -sf http://localhost:18222/healthz >/dev/null 2>&1; then
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "NATS failed to start" >&2
    exit 1
  fi
  sleep 1
done

# --- Environment ---
export DATABASE_URL="postgres://postgres:postgres@localhost:15432/brdgme"
export NATS_URL="nats://localhost:14222"
export SQLX_OFFLINE=true
export RUST_MIN_STACK=8388608

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../rust"

exec "$SCRIPT_DIR/rust-ci-commands.sh"
