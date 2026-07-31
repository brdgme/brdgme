#!/usr/bin/env bash
# Negative test for the delivery-list CI guard (scripts/check-delivery-lists.sh).
#
# Runs the guard against an intentionally broken fixture tree
# (scripts/fixtures/delivery-lists-broken): cargo member bar-1 has a bake
# target and a k8s dir but NO rust/Dockerfile stage. A correct guard must
# detect the mismatch and exit non-zero.
#
# RED until the guard exists: this test fails while check-delivery-lists.sh is
# absent, and again if a present guard exits 0 on the broken fixture. It uses
# only bash and the fixture, so it never fails for an unrelated missing tool
# and never touches the real delivery lists.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$SCRIPT_DIR/check-delivery-lists.sh"
FIXTURE="$SCRIPT_DIR/fixtures/delivery-lists-broken"

if [ ! -f "$GUARD" ]; then
  echo "FAIL: delivery-list guard absent: $GUARD" >&2
  echo "      expected red - the guard is not implemented yet." >&2
  exit 1
fi

# The guard reads its four lists relative to CWD (CI runs it from the repo
# root). Running it with the fixture as CWD isolates it from the real
# rust/Cargo.toml, rust/Dockerfile, docker-bake.hcl and k8s/base/game/.
if (cd "$FIXTURE" && bash "$GUARD") >/dev/null 2>&1; then
  echo "FAIL: guard exited 0 on the broken fixture; it did not detect that" >&2
  echo "      cargo member bar-1 has no rust/Dockerfile stage." >&2
  exit 1
fi

echo "PASS: guard detected the broken delivery fixture."
