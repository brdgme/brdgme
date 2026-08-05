#!/usr/bin/env bash
# Contract test for the four-tooth sign-off guard (scripts/check-four-tooth.sh).
#
# Runs the guard against five committed fixtures:
#   four-tooth-positive           - a real closed finding satisfying all four
#                                   teeth; the guard must accept it.
#   four-tooth-missing-citation   - tooth 1: the cited source file does not
#                                   exist (F-109 shape: deleted fix/test).
#   four-tooth-unreachable        - tooth 2: the cited symbol has no caller
#                                   (F-147 shape: send_turn_reminder).
#   four-tooth-decoy-test         - tooth 3: the named test never exercises
#                                   the target symbol (F-151 shape).
#   four-tooth-comment-caller     - tooth 2, adversarial: the only mention of
#                                   the symbol is a comment, never a call, so
#                                   a comment-only caller decoy must not pass.
#   four-tooth-comment-test       - tooth 3, adversarial: the named test body
#                                   only comments about the target, so a
#                                   comment-only test decoy must not pass.
#   four-tooth-unamended-premise  - tooth 4: a disproved premise has no
#                                   explicit amendment (F-205 shape: dp F12).
#
# Each negative fixture must exit non-zero, emit the exact diagnostic of its
# named tooth (with the finding ID), and emit no other tooth's diagnostic - so
# every tooth is enforced independently and no fixture can pass for the wrong
# reason. Uses only bash and the committed fixtures; never touches the real
# tree.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$SCRIPT_DIR/check-four-tooth.sh"
FIXTURES="$SCRIPT_DIR/fixtures"

fail=0

run() {
  (cd "$FIXTURES/$1" && bash "$GUARD") 2>&1
}

# pass_fixture <name>: the guard must exit 0.
pass_fixture() {
  local out
  out="$(run "$1")"
  if [ $? -ne 0 ]; then
    echo "FAIL: guard rejected the positive fixture $1" >&2
    printf '%s\n' "$out" >&2
    fail=1
  fi
}

# fail_fixture <name> <finding-id> <marker> <exact-diagnostic>: the guard must
# exit non-zero, emit exactly <marker>'s diagnostic for <finding-id>, and emit
# no other tooth's diagnostic.
fail_fixture() {
  local name="$1" id="$2" marker="$3" diag="$4" out other
  out="$(run "$name")"
  local rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "FAIL: guard exited 0 on $name (expected $marker failure)" >&2
    fail=1
    return
  fi
  if ! grep -qF -- "$diag" <<<"$out"; then
    echo "FAIL: $name did not emit the exact $marker diagnostic" >&2
    printf '%s\n' "$out" >&2
    echo "  expected: $diag" >&2
    fail=1
  fi
  for other in CITATION-MISSING CITATION-UNREACHABLE DECOY-TEST UNAMENDED-PREMISE; do
    [ "$other" = "$marker" ] && continue
    if grep -qF -- "$other" <<<"$out"; then
      echo "FAIL: $name emitted $other in addition to $marker" >&2
      printf '%s\n' "$out" >&2
      fail=1
    fi
  done
}

pass_fixture four-tooth-positive
fail_fixture four-tooth-missing-citation F-109 CITATION-MISSING \
  'CITATION-MISSING: F-109: cited source file not found: src/ghost.rs'
fail_fixture four-tooth-unreachable F-147 CITATION-UNREACHABLE \
  'CITATION-UNREACHABLE: F-147: send_turn_reminder has no caller outside its definition and test modules'
fail_fixture four-tooth-decoy-test F-151 DECOY-TEST \
  'DECOY-TEST: F-151: test rating_before_aggregates_exclude_nulls does not invoke game_history'
fail_fixture four-tooth-comment-caller F-1001 CITATION-UNREACHABLE \
  'CITATION-UNREACHABLE: F-1001: game_history has no caller outside its definition and test modules'
fail_fixture four-tooth-comment-test F-1002 DECOY-TEST \
  'DECOY-TEST: F-1002: test rating_before_aggregates_exclude_nulls does not invoke game_history'
fail_fixture four-tooth-unamended-premise F-205 UNAMENDED-PREMISE \
  'UNAMENDED-PREMISE: F-205: disproved premise not amended: game_history is excluded from ratings aggregates'

if [ "$fail" -ne 0 ]; then
  echo "check-four-tooth: FAIL" >&2
  exit 1
fi
echo "PASS: four-tooth guard contract fixtures all behave as specified."
