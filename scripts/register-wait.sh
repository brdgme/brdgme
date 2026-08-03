#!/usr/bin/env bash
#
# register-wait.sh - one-shot bulk game registration for the root Compose lane
# (backlog #50 unit-04b). Runs inside the register image (debian:bookworm-slim,
# which has bash/sed but no curl/jq): waits for every selected game's probe URL
# to accept a TCP connection (bounded, mirroring the k8s tcpSocket readiness
# probes), then executes `register set` against the static set input. The
# distroless game images have no shell, so Compose cannot healthcheck them -
# this script is the bounded game-readiness gate for registration.
#
# The probe URLs are read from the set input itself
# (compose/register-set.json, bind-mounted at /register-set.json), so this
# script holds no game list of its own.

set -euo pipefail

SET_FILE="${REGISTER_SET_FILE:-/register-set.json}"
BOUND="${REGISTER_READY_BOUND:-120}"

field() {
    sed -n "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" "$SET_FILE"
}

urls="$(field probeUrl)"
if [ -z "$urls" ]; then
    echo "register-wait: no probe URLs found in $SET_FILE" >&2
    exit 1
fi

while IFS= read -r url; do
    [ -z "$url" ] && continue
    host="${url#http://}"
    port="${host##*:}"
    host="${host%:*}"
    secs=0
    while ! (exec 3<>"/dev/tcp/$host/$port") 2>/dev/null; do
        if (( secs >= BOUND )); then
            echo "register-wait: FAIL $url not ready within ${BOUND}s" >&2
            exit 1
        fi
        sleep 1
        secs=$((secs + 1))
    done
    echo "register-wait: $url ready (${secs}s)"
done <<< "$urls"

echo "register-wait: all games ready; running register set"
exec /register set "$SET_FILE"
