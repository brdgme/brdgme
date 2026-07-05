#!/usr/bin/env bash
# E2E smoke suite entry point. Boots Postgres schema, the game service, and
# the release web binary, then runs the Playwright suite against them.
#
# Assumes Postgres is already running (devenv/Tilt locally, a service
# container in CI) - this script does not start it.
#
# This script never runs `playwright install`: locally, devenv's
# PLAYWRIGHT_BROWSERS_PATH points at a Nix-provided Chromium build matching
# @playwright/test's pinned version (see package.json - keep both in sync);
# CI has no PLAYWRIGHT_BROWSERS_PATH set and installs its own browser
# (see .github/workflows/ci.yml).
set -euo pipefail

END2END_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="$(dirname "$END2END_DIR")"
RUST_DIR="$(dirname "$WEB_DIR")"

export E2E_DATABASE_URL="${E2E_DATABASE_URL:-postgres://brdgme_user:brdgme_password@localhost:5432/brdgme_e2e}"
export NATS_URL="${NATS_URL:-nats://localhost:4222}"

GAME_ADDR="127.0.0.1:8100"
WEB_ADDR="127.0.0.1:3010"

echo "==> Resetting e2e database"
DATABASE_URL="$E2E_DATABASE_URL" sqlx database drop -y
DATABASE_URL="$E2E_DATABASE_URL" sqlx database create
DATABASE_URL="$E2E_DATABASE_URL" sqlx migrate run --source "$WEB_DIR/migrations"

echo "==> Seeding e2e database"
psql "$E2E_DATABASE_URL" -v ON_ERROR_STOP=1 -f "$END2END_DIR/seed.sql"

if [ "${E2E_SKIP_BUILD:-0}" != "1" ]; then
    echo "==> Building web (release)"
    (cd "$WEB_DIR" && cargo leptos build --release)
fi
echo "==> Building game service (release)"
(cd "$RUST_DIR" && cargo build --release -p lost-cities-2 --bin lost_cities_2_http)

GAME_PID=""
WEB_PID=""
cleanup() {
    if [ -n "$GAME_PID" ]; then
        kill "$GAME_PID" 2>/dev/null || true
    fi
    if [ -n "$WEB_PID" ]; then
        kill "$WEB_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

echo "==> Starting game service on $GAME_ADDR"
ADDR="$GAME_ADDR" "$RUST_DIR/target/release/lost_cities_2_http" &
GAME_PID=$!

echo "==> Starting web on $WEB_ADDR"
(
    cd "$WEB_DIR"
    unset RESEND_API_KEY
    unset BOT_SERVICE_URL
    export DATABASE_URL="$E2E_DATABASE_URL"
    export LEPTOS_OUTPUT_NAME="web"
    export LEPTOS_SITE_ADDR="$WEB_ADDR"
    export LEPTOS_SITE_ROOT="$RUST_DIR/target/site"
    export LEPTOS_ENV="PROD"
    exec "$RUST_DIR/target/release/web"
) &
WEB_PID=$!

wait_for_port() {
    local host="$1" port="$2" name="$3"
    for _ in $(seq 1 60); do
        if (exec 3<>"/dev/tcp/$host/$port") 2>/dev/null; then
            exec 3<&- 3>&-
            echo "==> $name is up"
            return 0
        fi
        sleep 1
    done
    echo "==> $name did not become ready in time" >&2
    exit 1
}

wait_for_port 127.0.0.1 8100 "game service"
wait_for_port 127.0.0.1 3010 "web"

echo "==> Running Playwright"
(cd "$END2END_DIR" && npx playwright test)
