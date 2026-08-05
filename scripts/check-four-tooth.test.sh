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
#   wp-provenance-malformed       - a truncated provenance row (empty checklist
#                                   and completed fields). The guard must fail
#                                   it with a WP-scoped malformed-record
#                                   diagnostic, never silently count the WP as
#                                   open.
#
# And twelve more committed fixtures for the deferral-routing gate (4.4), which
# requires every expected routing link to carry sender evidence (a routing
# record with the deferral state exactly `routed-to: WP-NN`) and receiver
# evidence (a declaration naming the inherited finding). Every routing record
# must correspond exactly to an authoritative scope link, and closure
# attribution is explicit in a routing-specific input (routing-closures.tsv),
# never inferred from generic sign-off rows:
#   routing-positive             - a closed finding (four-teeth-valid), routed
#                                   findings with valid state, matching
#                                   declarations, full sender evidence, and a
#                                   receiver closure backed by an exact
#                                   declaration (the allowed closure path); the
#                                   guard must accept it.
#   routing-invalid-state        - a routing record whose state is `closed`
#                                   instead of `routed-to: WP-NN` (the 4.4
#                                   historical failure mode). The guard must
#                                   fail it with a routing-scoped
#                                   invalid-state diagnostic.
#   routing-sender-closure       - a routed finding closed by its sender in the
#                                   routing-specific closure input (F-55/F-57/
#                                   F-60 shape). The guard must fail it and no
#                                   other tooth.
#   routing-missing-declaration  - a routed finding the receiving WP never
#                                   declares (the receiver never picked it up).
#                                   The guard must fail it.
#   routing-mismatched-declaration - a declaration that names a finding for a
#                                   receiver no routing record sent it to; the
#                                   declaration must match a routing record.
#   routing-omitted-linkage      - an expected routing link with no routing
#                                   record: the routing scope is authoritative,
#                                   so omitting the sender's evidence cannot
#                                   bypass the gate.
#   routing-unexpected-record    - a rogue extra routing record (F-9999) that
#                                   matches no expected scope link, paired with
#                                   a fabricated declaration the rogue record
#                                   would authorize; the guard must fail it and
#                                   no other tooth.
#   routing-empty-scope          - an empty authoritative scope with routing
#                                   content: the guard must reject rather than
#                                   silently claim a validated route set.
#   routing-duplicate-declaration - the receiving WP declares the routed finding
#                                   twice; the guard must fail it as a
#                                   duplicate rather than silently accept the
#                                   last row.
#   routing-receiver-close-missing-declaration - the receiving WP closes the
#                                   routed finding but never declares it; the
#                                   guard must fail it.
#   routing-receiver-close-mismatched-declaration - the receiving WP closes the
#                                   routed finding but the only declaration
#                                   names a different routed finding, so none
#                                   exactly matches; the guard must fail it.
#   routing-closed-by-other      - a routed finding closed by a WP that is
#                                   neither its sender nor its receiver; the
#                                   guard must fail it.
#
# Each negative fixture must exit non-zero, emit the exact diagnostic of its
# named tooth (or gate) with the finding (or WP) ID, and emit no other
# diagnostic - so every tooth and the WP and routing gates are enforced
# independently and no fixture can pass for the wrong reason. Uses only bash and
# the committed fixtures; never touches the real tree.

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
      WP-MALFORMED WP-UNACCOUNTED WP-DUPLICATE WP-NO-PROVENANCE \
      ROUTING-MALFORMED ROUTING-UNACCOUNTED ROUTING-UNEXPECTED-RECORD \
      ROUTING-DUPLICATE ROUTING-INVALID-STATE ROUTING-CLOSED-BY-SENDER \
      ROUTING-CLOSED-BY-OTHER ROUTING-CLOSED-WITHOUT-DECLARATION \
      ROUTING-UNEXPECTED-CLOSURE ROUTING-UNDECLARED ROUTING-DECLARATION-MISMATCH; do
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
fail_fixture wp-provenance-malformed WP-02 WP-MALFORMED \
  'WP-MALFORMED: WP-02: provenance record must have four tab-separated non-empty fields (wp spec checklist completed)' \
  signoffs.tsv wp-scope.tsv wp-provenance.tsv

pass_fixture routing-positive \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-invalid-state F-1003 ROUTING-INVALID-STATE \
  'ROUTING-INVALID-STATE: F-1003: deferral state "closed" is not valid (must be "routed-to: WP-03")' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-sender-closure F-1003 ROUTING-CLOSED-BY-SENDER \
  'ROUTING-CLOSED-BY-SENDER: F-1003: routed finding closed by sender WP-01 (a deferral is a state, never closed)' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-missing-declaration F-1003 ROUTING-UNDECLARED \
  'ROUTING-UNDECLARED: F-1003: receiver WP-03 has no declaration of the inherited finding' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-mismatched-declaration F-1004 ROUTING-DECLARATION-MISMATCH \
  'ROUTING-DECLARATION-MISMATCH: F-1004: declaration for WP-04 matches no routing record' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-omitted-linkage F-1005 ROUTING-UNACCOUNTED \
  'ROUTING-UNACCOUNTED: F-1005: expected routing link WP-02 -> WP-04 has no routing record' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-unexpected-record F-9999 ROUTING-UNEXPECTED-RECORD \
  'ROUTING-UNEXPECTED-RECORD: F-9999: routing record WP-01 -> WP-05 matches no expected routing link' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-empty-scope F-1003 ROUTING-UNEXPECTED-RECORD \
  'ROUTING-UNEXPECTED-RECORD: F-1003: routing record WP-01 -> WP-03 matches no expected routing link' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-duplicate-declaration F-1003 ROUTING-DUPLICATE \
  'ROUTING-DUPLICATE: F-1003: duplicate routing declaration for receiver WP-03' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-receiver-close-missing-declaration F-1003 ROUTING-CLOSED-WITHOUT-DECLARATION \
  'ROUTING-CLOSED-WITHOUT-DECLARATION: F-1003: receiver WP-03 closed the routed finding without declaring it exactly (finding receiver)' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-receiver-close-mismatched-declaration F-1003 ROUTING-CLOSED-WITHOUT-DECLARATION \
  'ROUTING-CLOSED-WITHOUT-DECLARATION: F-1003: receiver WP-03 closed the routed finding without declaring it exactly (finding receiver)' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv
fail_fixture routing-closed-by-other F-1003 ROUTING-CLOSED-BY-OTHER \
  'ROUTING-CLOSED-BY-OTHER: F-1003: routed finding closed by WP-09, which is neither the sender nor the receiving WP' \
  signoffs.tsv "" "" routing-scope.tsv routing.tsv routing-declarations.tsv routing-closures.tsv

if [ "$fail" -ne 0 ]; then
  echo "check-four-tooth: FAIL" >&2
  exit 1
fi
echo "PASS: four-tooth guard contract fixtures all behave as specified."
