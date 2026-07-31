#!/usr/bin/env bash
#
# check-delivery-lists.sh - CI guard for delivery-list consistency (F-208).
#
# Asserts that every Rust game crate declared as a workspace member in
# rust/Cargo.toml is actually delivered, i.e. it also appears as:
#   - a distroless stage in rust/Dockerfile,
#   - a game target in docker-bake.hcl,
#   - a deployment dir under k8s/base/game/.
# Every mismatch is diagnosed (named, both directions) and fails the build
# with a non-zero exit. Raw count differences are never reported as a finding
# on their own - only the named set differences are.
#
# CI runs this from the repo root. All paths are relative to CWD so the
# negative test (scripts/check-delivery-lists.test.sh) can point the same
# script at a fixture tree by invoking it with the fixture as CWD.
#
# Uses only standard Bash/GNU utilities (grep -P, sed, sort, comm, ls).

set -uo pipefail

# Intentional absentees: workspace members built but not yet shipped.
# lords-of-vegas-1: WIP, owner-excluded (BACKLOG Out of Scope). Review: 2026-09-01.
ALLOWLIST="lords-of-vegas-1"

# Non-game entries in the docker-bake.hcl tgt matrix (web/migrate/bot/operator
# are delivered but are not Rust game crates, so they are excluded before the
# game-target set is compared against the Cargo game members).
BAKE_NON_GAME='web|migrate|bot|operator'

fail=0

# Print the non-empty lines of a list (one per line), stripping blanks so an
# empty list compares as the empty set rather than a single empty line.
lines() { printf '%s\n' "$1" | grep -v '^$' || true; }

# compare <label> <expected> <actual>: report named set differences both ways.
compare() {
  local label="$1" exp="$2" act="$3" missing extra
  missing="$(comm -23 <(lines "$exp" | sort -u) <(lines "$act" | sort -u))"
  extra="$(comm -13 <(lines "$exp" | sort -u) <(lines "$act" | sort -u))"
  if [ -n "$missing" ] || [ -n "$extra" ]; then
    echo "MISMATCH: $label" >&2
    if [ -n "$missing" ]; then
      echo "  cargo game members with no $label:" >&2
      lines "$missing" | sed 's/^/    /' >&2
    fi
    if [ -n "$extra" ]; then
      echo "  $label with no cargo game member (stale):" >&2
      lines "$extra" | sed 's/^/    /' >&2
    fi
    fail=1
  fi
}

# 1. Rust game workspace members from rust/Cargo.toml ("game/<name>" entries).
cargo_members="$(grep -oP '"game/\K[^"]+' rust/Cargo.toml | sort -u)"

# Expected delivery set: game members minus the allow-list.
expected="$(lines "$cargo_members" | grep -vxFf <(printf '%s\n' "$ALLOWLIST") | sort -u)"

# 2. rust/Dockerfile distroless game stages. Game stages use the distroless
#    base; web/migrate/bot/operator use debian:bookworm-slim, so distroless
#    FROM lines are game-only.
docker_stages="$(grep -oP '^FROM gcr.io/distroless.*AS \K\S+' rust/Dockerfile | sort -u)"

# 3. docker-bake.hcl game targets: the tgt matrix minus the non-game entries.
bake_targets="$(sed -n '/tgt = \[/,/\]/p' docker-bake.hcl \
  | grep -oP '"\K[^"]+(?=")' \
  | grep -vxE "$BAKE_NON_GAME" \
  | sort -u)"

# 4. k8s/base/game deployment dirs, filtered to Cargo-derived names: k8s also
#    holds Go games, which are not Rust workspace members, so intersect the dir
#    names with the Cargo members before comparing.
k8s_dirs="$(ls -d k8s/base/game/*/ 2>/dev/null | xargs -n1 basename | sort -u)"
k8s_rust="$(comm -12 <(lines "$k8s_dirs") <(lines "$cargo_members"))"

compare "rust/Dockerfile stage" "$expected" "$docker_stages"
compare "docker-bake.hcl target" "$expected" "$bake_targets"
compare "k8s/base/game deployment" "$expected" "$k8s_rust"

if [ "$fail" -ne 0 ]; then
  echo "check-delivery-lists: FAIL" >&2
  exit 1
fi
echo "check-delivery-lists: OK (Cargo game members, Dockerfile stages, bake targets and k8s deployments are consistent)"
