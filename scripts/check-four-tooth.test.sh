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
# And three more committed fixtures for the WP provenance gate (4.3), which
# deterministically enumerates every in-scope work package and fails any
# completed WP with neither an approved specification nor at least one
# checklist row:
#   wp-provenance-positive        - completed WPs with spec-only or
#                                   checklist-only evidence, plus open WPs with
#                                   none; the guard must accept it.
#   wp-provenance-nospec          - a completed WP with neither spec nor
#                                   checklist (WP-72 shape: exists only as a
#                                   commit).
#   wp-provenance-omitted         - the same shape, but the WP's provenance row
#                                   is simply omitted from the file. The guard
#                                   must still fail it: the scope enumeration
#                                   is authoritative and never inferred from
#                                   the provenance rows, so omitting a row
#                                   cannot bypass the gate.
#
# Each negative fixture must exit non-zero, emit the exact diagnostic of its
# named tooth (or gate) with the finding (or WP) ID, and emit no other
# diagnostic - so every tooth and the WP gate are enforced independently and no
# fixture can pass for the wrong reason. Uses only bash and the committed
# fixtures; never touches the real tree.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$SCRIPT_DIR/check-four-tooth.sh"
FIXTURES="$SCRIPT_DIR/fixtures"

fail=0

# run <fixture> [guard args...]: run the guard inside the fixture dir.
run() {
  local name="$1"; shift
  (cd "$FIXTURES/$name" && bash "$GUARD" "$@") 2>&1
}

# pass_fixture <name> [guard args...]: the guard must exit 0.
pass_fixture() {
  local name="$1"; shift
  local out
  out="$(run "$name" "$@")"
  if [ $? -ne 0 ]; then
    echo "FAIL: guard rejected the positive fixture $name" >&2
    printf '%s\n' "$out" >&2
    fail=1
  fi
}

# fail_fixture <name> <id> <marker> <exact-diagnostic> [guard args...]: the
# guard must exit non-zero, emit exactly <marker>'s diagnostic for <id>, and
# emit no other tooth's or gate's diagnostic.
fail_fixture() {
  local name="$1" id="$2" marker="$3" diag="$4"; shift 4
  local out other
  out="$(run "$name" "$@")"
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
  for other in CITATION-MISSING CITATION-UNREACHABLE DECOY-TEST UNAMENDED-PREMISE \
      WP-UNACCOUNTED WP-DUPLICATE WP-NO-PROVENANCE; do
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

pass_fixture wp-provenance-positive signoffs.tsv wp-scope.tsv wp-provenance.tsv
fail_fixture wp-provenance-nospec WP-02 WP-NO-PROVENANCE \
  'WP-NO-PROVENANCE: WP-02: completed work package has neither specification nor checklist evidence' \
  signoffs.tsv wp-scope.tsv wp-provenance.tsv
fail_fixture wp-provenance-omitted WP-03 WP-UNACCOUNTED \
  'WP-UNACCOUNTED: WP-03: in-scope work package has no provenance record' \
  signoffs.tsv wp-scope.tsv wp-provenance.tsv

if [ "$fail" -ne 0 ]; then
  echo "check-four-tooth: FAIL" >&2
  exit 1
fi
echo "PASS: four-tooth guard contract fixtures all behave as specified."
