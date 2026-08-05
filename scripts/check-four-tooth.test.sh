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
# And ten more committed fixtures for the stop-escalation gate (4.6), which
# requires every declared mandatory STOP/HALT trigger to carry an exact
# owner-response record - bound to the same trigger, answered by the exact
# required owner, and explicitly classified as `amendment` or `abandonment`.
# The escalation scope is the authoritative enumeration and is never inferred
# from the response records, so omitting a response cannot hide an unresponded
# trigger:
#   stop-escalation-positive             - two declared triggers each with an
#                                          exact owner-response (one amendment,
#                                          one abandonment); the guard must
#                                          accept it.
#   stop-escalation-missing-response     - a declared trigger with no response
#                                          record (the F-206 shape: the trigger
#                                          fired and nothing answered it). The
#                                          guard must fail it as unresponded.
#   stop-escalation-wrong-owner          - a response for a declared trigger
#                                          answered by someone other than the
#                                          required owner; the guard must fail
#                                          it as a wrong-owner linkage.
#   stop-escalation-unexpected-response  - a rogue response matching no declared
#                                          trigger (an unauthorized response
#                                          that would clear nothing). The guard
#                                          must fail it and no other tooth.
#   stop-escalation-closure-masquerade   - a response whose kind is `closed`
#                                          instead of `amendment`/`abandonment`:
#                                          an ordinary closure masquerading as
#                                          a response. The guard must fail it.
#   stop-escalation-duplicate-response   - the same trigger answered twice; the
#                                          guard must fail it as a duplicate
#                                          rather than silently accept one.
#   stop-escalation-duplicate-scope      - the same declared trigger listed
#                                          twice in the scope; the guard must
#                                          fail it as a duplicate.
#   stop-escalation-empty-scope          - an empty authoritative scope with a
#                                          response record: the guard must
#                                          reject the response as matching no
#                                          declared trigger.
#   stop-escalation-malformed-response   - a truncated response record; the
#                                          guard must fail it with a
#                                          trigger-scoped malformed diagnostic,
#                                          never silently count the trigger as
#                                          answered.
#   stop-escalation-malformed-scope      - a truncated scope link; the guard
#                                          must fail it with the scope malformed
#                                          diagnostic and not cascade the
#                                          trigger's response into an unrelated
#                                          diagnostic.
#
# And ten more committed fixtures for the sibling-sweep gate (4.9a), which
# requires every in-scope one-function-fix claim to name a reproducible
# sibling-search pattern and a recorded current hit count, and requires that
# re-running the pattern against the declared scope reproduces the count within
# the approved heuristic limit. The sibling scope is the authoritative
# enumeration and is never inferred from the records, so omitting a record
# cannot hide a claim:
#   sibling-positive              - one claim (F-1000, fixed function
#                                   update_game_command_success) whose pattern
#                                   `left_at_guard` re-runs to the recorded
#                                   count 2 in src/game_write.rs within limit 2,
#                                   with the fixed function present; the guard
#                                   must accept it.
#   sibling-missing               - the scope enumerates the claim but the
#                                   records file omits its evidence row; the
#                                   guard must fail it as a missing record.
#   sibling-stale-count           - the recorded hit count 1 does not match the
#                                   current hit count 2; the guard must fail it
#                                   as stale, never trusting the recorded count.
#   sibling-omitted-scope         - a record with an empty search-scope field;
#                                   the guard must fail it with the
#                                   scope-omitted diagnostic, never silently
#                                   reusing a neighbouring field as the scope.
#   sibling-malformed             - a truncated record (missing the limit
#                                   field); the guard must fail it as
#                                   malformed.
#   sibling-duplicate             - the same claim recorded twice; the guard
#                                   must fail it as a duplicate rather than
#                                   silently accept the last row.
#   sibling-rogue                 - an extra record for F-9999 matching no
#                                   in-scope claim, paired with a complete
#                                   valid record; the guard must fail the rogue
#                                   record and no other claim.
#   sibling-decoy-pattern         - a decoy pattern that matches nothing with a
#                                   recorded count of 0: a naive count-only
#                                   guard passes it, but the pattern is not a
#                                   real sibling search; the guard must fail it
#                                   as a decoy.
#   sibling-decoy-scope           - a "nearby scan" decoy: the declared scope
#                                   src/other.rs contains matching lines but not
#                                   the fixed function the scope link names;
#                                   the guard must fail it as a decoy.
#   sibling-decoy-comment-fn      - a comment-only fixed-function decoy: the
#                                   claimed function exists only as a comment
#                                   `// fn update_game_command_success()`, never
#                                   in real code, while the recorded pattern
#                                   count reproduces over the comment lines; the
#                                   guard must fail it as a decoy, never accept
#                                   the comment mention as proof the fix landed.
#   sibling-decoy-string-fn       - a string-only fixed-function decoy: the
#                                   claimed function appears only inside a
#                                   string literal `"fn update_game_command_
#                                   success() ..."`, never as real code, while
#                                   the recorded pattern count reproduces over
#                                   real code; the guard must fail it as a
#                                   decoy, never accept the string mention as
#                                   proof the fix landed.
#   sibling-decoy-comment-pattern - a comment-only sibling-pattern decoy: the
#                                   fixed function exists, but the pattern
#                                   `left_at_guard` matches only comment lines,
#                                   so the recorded count reproduces only over
#                                   comments; the guard must fail it as a decoy,
#                                   never accept comment matches as evidence of
#                                   a real sibling search.
#   sibling-decoy-string-pattern  - a string-only sibling-pattern decoy: the
#                                   fixed function exists, but the pattern
#                                   `left_at_guard` matches only string
#                                   literals, so the recorded count reproduces
#                                   only over strings; the guard must fail it as
#                                   a decoy, never accept string matches as
#                                   evidence of a real sibling search.
#   sibling-over-limit            - a correct re-run whose hit count 2 exceeds
#                                   the approved heuristic limit 1; the guard
#                                   must fail it.
#
# Each negative fixture must exit non-zero, emit the exact diagnostic of its
# named tooth (or gate) with the finding (or WP) ID, and emit no other
# diagnostic - so every tooth and the WP, routing, escalation and sibling gates
# are enforced independently and no fixture can pass for the wrong reason. Uses
# only bash and the committed fixtures; never touches the real tree.
#
# And ten more committed fixtures for the exhaustive-match gate (4.9b), which
# requires every in-scope exhaustive-match claim to name its scope and prove no
# wildcard match arm remains there, reproducibly: the guard re-runs its two
# documented textual heuristics (a wildcard-arm scan and a match-presence scan)
# against the declared scope, the recorded wildcard count must reproduce at 0,
# the recorded match count must reproduce, and the scope must actually contain
# at least one `match` expression - so a vacuous or nearby scan is never proof.
# The exhaustive scope is the authoritative enumeration and is never inferred
# from the records, so omitting a record cannot hide a claim:
#   exhaustive-positive             - one claim (F-1100, scope
#                                     src/game_write.rs) with two exhaustive
#                                     `match`es and zero wildcard arms, counts
#                                     recorded as 0 wildcards / 2 matches; the
#                                     guard must accept it.
#   exhaustive-missing              - the scope enumerates the claim but the
#                                     records file omits its evidence row; the
#                                     guard must fail it as a missing record.
#   exhaustive-stale                - the recorded match count 1 does not match
#                                     the current count 2; the guard must fail
#                                     it as stale, never trusting the recorded
#                                     count.
#   exhaustive-omitted-scope        - a record with an empty search-scope field;
#                                     the guard must fail it with the
#                                     scope-omitted diagnostic.
#   exhaustive-empty-scope          - an empty authoritative scope file (and no
#                                     records): the guard must fail it as an
#                                     empty scope, never silently accept a
#                                     sign-off that enumerates no claims.
#   exhaustive-malformed            - a truncated record (missing the matches
#                                     field); the guard must fail it as
#                                     malformed.
#   exhaustive-duplicate            - the same claim recorded twice; the guard
#                                     must fail it as a duplicate rather than
#                                     silently accept the last row.
#   exhaustive-rogue                - an extra record for F-9999 matching no
#                                     in-scope claim, paired with a complete
#                                     valid record; the guard must fail the
#                                     rogue record and no other claim.
#   exhaustive-decoy-nearby         - a "nearby scan" decoy: the record's scope
#                                     src/other.rs differs from the
#                                     authoritative scope src/game_write.rs; the
#                                     guard must fail it as a decoy.
#   exhaustive-decoy-vacuous        - an "empty scope" decoy: the declared scope
#                                     contains no `match` expression, so the
#                                     recorded 0 wildcards proves nothing; the
#                                     guard must fail it as a decoy.
#   exhaustive-wildcard-remains     - a scope whose `match`es still carry
#                                     wildcard arms in every variant the
#                                     heuristic covers (`_ =>`, `_=>`, `_ if
#                                     guard =>`, `_ @ bind =>`); the guard must
#                                     fail it as a remaining wildcard arm.
#
# And twelve more committed fixtures for the dead-code sweep gate (4.9c), which
# requires every in-scope dead-code sweep claim to name the authoritative
# universe (the closure-register commit range it sweeps) and prove no
# `#[allow(dead_code)]` or suppression variant remains there, reproducibly: the
# guard re-runs its three documented textual heuristics (a dead-code presence
# scan, an `allow(dead_code)` scan, and a suppression-variant scan) against the
# declared universe, the allowance and variant counts must reproduce at 0, the
# recorded match count must reproduce, and the universe must actually contain at
# least one `dead_code` mention - so a vacuous or nearby scan is never proof, and
# an unreported variant (e.g. `#[allow(unused)]`, whose lint group covers
# dead_code) cannot bypass the sweep. The dead-code scope is the authoritative
# enumeration and is never inferred from the records, so omitting a record cannot
# hide a claim:
#   deadcode-positive             - one claim (F-1200, universe
#                                   src/game_write.rs) whose universe carries a
#                                   `#[deny(dead_code)]` mention and no
#                                   allowance or suppression variant, counts
#                                   recorded as 0 allowances / 0 variants / 1
#                                   match; the guard must accept it.
#   deadcode-missing              - the scope enumerates the claim but the
#                                   records file omits its evidence row; the
#                                   guard must fail it as a missing record.
#   deadcode-stale                - the recorded match count 1 does not match
#                                   the current count 2; the guard must fail it
#                                   as stale, never trusting the recorded count.
#   deadcode-omitted-scope        - a record with an empty universe field; the
#                                   guard must fail it with the scope-omitted
#                                   diagnostic.
#   deadcode-empty-scope          - an empty authoritative scope file (and no
#                                   records): the guard must fail it as an empty
#                                   scope, never silently accept a sign-off that
#                                   enumerates no claims.
#   deadcode-malformed            - a truncated record (missing the matches
#                                   field); the guard must fail it as
#                                   malformed.
#   deadcode-duplicate            - the same claim recorded twice; the guard
#                                   must fail it as a duplicate rather than
#                                   silently accept the last row.
#   deadcode-rogue                - an extra record for F-9999 matching no
#                                   in-scope claim, paired with a complete
#                                   valid record; the guard must fail the rogue
#                                   record and no other claim.
#   deadcode-decoy-nearby         - a "nearby scan" decoy: the record's universe
#                                   src/other.rs differs from the authoritative
#                                   universe src/game_write.rs; the guard must
#                                   fail it as a decoy.
#   deadcode-decoy-vacuous        - an "empty universe" decoy: the declared
#                                   universe contains no `dead_code` mention, so
#                                   the recorded 0 allowances proves nothing;
#                                   the guard must fail it as a decoy.
#   deadcode-allowance-remains    - a universe whose code still carries
#                                   `#[allow(dead_code)]`; the guard must fail
#                                   it as a remaining allowance.
#   deadcode-variant-remains      - unreported variant coverage: a universe that
#                                   carries `#[allow(unused)]`, `#[allow(warnings)]`,
#                                   `#[expect(dead_code)]`, `#[expect(unused)]`
#                                   and `#[expect(warnings)]` - lint groups and
#                                   expectations that all cover dead_code -
#                                   while the record reports 0 variants; the
#                                   guard must fail it as a remaining
#                                   suppression variant.
#
# And fourteen more committed fixtures for the log-layer proof gate (4.9d), which
# requires every in-scope logging-related claim to prove direct `Log::public`
# content coverage at its authoritative approved log-layer source scope,
# reproducibly: the guard re-runs the record's approved `Log::public` pattern
# against the declared scope and requires the recorded hit count to reproduce,
# and additionally requires the scope to contain at least one direct
# `Log::public` invocation once comments and string literals are stripped - so a
# comment, string, wrapper, or lower/upper-layer mention is never mistaken for
# direct content proof. The log-layer scope is the authoritative enumeration and
# is never inferred from the records, so omitting a record cannot hide a claim:
#   loglayer-positive             - one claim (F-1300, scope src/game_write.rs)
#                                   with two direct `Log::public` invocations,
#                                   pattern `Log::public` re-running to the
#                                   recorded count 2; the guard must accept it.
#   loglayer-missing              - the scope enumerates the claim but the
#                                   records file omits its evidence row; the
#                                   guard must fail it as a missing record.
#   loglayer-stale                - the recorded hit count 1 does not match the
#                                   current count 2; the guard must fail it as
#                                   stale, never trusting the recorded count.
#   loglayer-omitted-scope        - a record with an empty log-layer scope field;
#                                   the guard must fail it with the
#                                   scope-omitted diagnostic.
#   loglayer-empty-scope          - an empty authoritative scope file (and no
#                                   records): the guard must fail it as an empty
#                                   scope, never silently accept a sign-off that
#                                   enumerates no claims.
#   loglayer-malformed            - a truncated record (missing the count field);
#                                   the guard must fail it as malformed.
#   loglayer-duplicate            - the same claim recorded twice; the guard
#                                   must fail it as a duplicate rather than
#                                   silently accept the last row.
#   loglayer-rogue                - an extra record for F-9999 matching no
#                                   in-scope claim, paired with a complete
#                                   valid record; the guard must fail the rogue
#                                   record and no other claim.
#   loglayer-decoy-nearby         - a "nearby wrapper/caller" decoy: the record's
#                                   scope src/other.rs holds a wrapper and its
#                                   caller layer, differing from the
#                                   authoritative scope src/game_write.rs; the
#                                   guard must fail it as a decoy.
#   loglayer-decoy-lower-layer    - a "lower-layer" decoy: the record's scope
#                                   src/game_log.rs is the `Log::public`
#                                   constructor/definition layer, not the
#                                   authoritative game-crate scope; the guard
#                                   must fail it as a decoy.
#   loglayer-decoy-comment        - a "comment" decoy: the only `Log::public`
#                                   mention in the authoritative scope is a
#                                   comment, so the pattern count reproduces
#                                   but there is no direct `Log::public`
#                                   invocation; the guard must fail it as a
#                                   decoy.
#   loglayer-decoy-string         - a "string" decoy: the only `Log::public`
#                                   mention in the authoritative scope is a
#                                   string literal; the guard must fail it as a
#                                   decoy.
#   loglayer-decoy-same-scope     - the same-scope coexistence decoy: an
#                                   arbitrary raw-grep pattern `FORTUNE`
#                                   matching only string mentions in the
#                                   authoritative scope while a separate direct
#                                   `Log::public` call in the same scope
#                                   satisfies a naive direct-count floor. The
#                                   recorded pattern/count is not tied to the
#                                   direct invocations, so the guard must fail
#                                   it as a decoy.
#   loglayer-decoy-equal-count    - the equal-count/different-lines decoy: two
#                                   direct `Log::public` calls and two
#                                   `Log::public;` (non-call) mentions spread
#                                   across two scope files with cross-swapped
#                                   line numbers, so the recorded pattern
#                                   reproduces the direct count 2 while its
#                                   `file:line` identities differ (same line
#                                   numbers, different files). A count-only
#                                   guard passes it; the guard must fail it as
#                                   a decoy.
#
# And fourteen more committed fixtures for the every-item checklist gate
# (4.9e), which requires every in-scope "every X" claim to name an
# authoritative, reproducible, non-empty item universe, to be linked to an
# actual sign-off record in the sign-offs input, and to carry exactly
# one explicit per-item checklist record per universe item: the guard
# re-runs the universe enumeration (the distinct non-blank lines of the
# authoritative universe file), every record's item count must reproduce the
# current universe count, and the checklist items must cover the universe
# one-to-one - so an omitted item, a duplicate item, a rogue extra, an empty
# universe, a stale count on any one record, and an unlinked claim are never
# accepted as "every X" evidence. The every-item scope is the authoritative
# enumeration and is never inferred from the records, so omitting a record
# cannot hide an uncovered claim. Every fixture carries a minimal linked
# four-teeth-valid sign-off record (F-1400 -> symbol record_game in
# fixture-local src/game_write.rs, called from src/other.rs) so the guard's
# linkage requirement is satisfied except where a fixture intentionally
# breaks it:
#   every-positive                  - one claim (F-1400, universe universe.txt
#                                     with three items) whose checklist records
#                                     cover every universe item exactly once
#                                     with a reproducing count of 3, linked to
#                                     the valid sign-off record; the guard must
#                                     accept it.
#   every-missing                   - the scope enumerates the claim but the
#                                     records file omits its checklist rows
#                                     entirely; the guard must fail it as
#                                     missing.
#   every-stale-count               - the per-record count bypass shape: only
#                                     the later record's count (2) is stale
#                                     while the earlier records reproduce 3, so
#                                     a first-count-only check would pass it;
#                                     the guard must fail the stale record as
#                                     stale, never trusting the first record's
#                                     count.
#   every-omitted-item              - a universe item (splendor-1) has no
#                                     per-item checklist record while the
#                                     recorded count 3 still reproduces; the
#                                     guard must fail it as an omitted item.
#   every-omitted-universe          - a record with an empty item-universe
#                                     field; the guard must fail it with the
#                                     universe-omitted diagnostic, never
#                                     silently reusing a neighbouring field.
#   every-empty-universe            - an authoritative universe file that names
#                                     no items (with no records); the guard
#                                     must fail it as an empty universe, never
#                                     accept a vacuous "every X" claim.
#   every-malformed                 - a truncated record (missing the item
#                                     field); the guard must fail it as
#                                     malformed.
#   every-duplicate                 - the same universe item recorded twice;
#                                     the guard must fail it as a duplicate
#                                     rather than silently accept the last
#                                     row.
#   every-rogue                     - an extra record for F-9999 matching no
#                                     in-scope claim, paired with a complete
#                                     valid record; the guard must fail the
#                                     rogue record and no other claim.
#   every-rogue-extra               - a checklist record naming an item
#                                     (splendor-1) that is not in the
#                                     authoritative universe; the guard must
#                                     fail it as a rogue extra, never as
#                                     coverage.
#   every-decoy-nearby              - a "nearby scan" decoy: the record's
#                                     universe other.txt differs from the
#                                     authoritative universe universe.txt; the
#                                     guard must fail it as a decoy.
#   every-decoy-universe-missing    - a "missing universe" decoy: the
#                                     authoritative universe file does not
#                                     exist; the guard must fail it as a
#                                     decoy.
#   every-empty-scope               - an empty authoritative scope file (and no
#                                     records): the guard must fail it as an
#                                     empty scope, never silently accept a
#                                     sign-off that enumerates no claims.
#   every-unlinked                  - an in-scope claim (F-9999) that is not
#                                     linked to any sign-off record in the
#                                     sign-offs input: an "every X" scope claim
#                                     with no linked sign-off record is not
#                                     evidence; the guard must fail it as
#                                     unlinked.

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
      ROUTING-UNEXPECTED-CLOSURE ROUTING-UNDECLARED ROUTING-DECLARATION-MISMATCH \
      ESCALATION-MALFORMED ESCALATION-DUPLICATE ESCALATION-UNRESPONDED \
      ESCALATION-UNEXPECTED-RESPONSE ESCALATION-WRONG-OWNER \
      ESCALATION-INVALID-RESPONSE \
      SIBLING-MISSING SIBLING-STALE-COUNT SIBLING-OMITTED-SCOPE \
      SIBLING-MALFORMED SIBLING-DUPLICATE SIBLING-ROGUE SIBLING-DECOY \
      SIBLING-OVER-LIMIT \
      EXMATCH-MALFORMED EXMATCH-DUPLICATE EXMATCH-MISSING EXMATCH-ROGUE \
      EXMATCH-OMITTED-SCOPE EXMATCH-EMPTY-SCOPE EXMATCH-DECOY EXMATCH-STALE \
      EXMATCH-WILDCARD-REMAINS \
      DCSWEEP-MALFORMED DCSWEEP-DUPLICATE DCSWEEP-MISSING DCSWEEP-ROGUE \
      DCSWEEP-OMITTED-SCOPE DCSWEEP-EMPTY-SCOPE DCSWEEP-DECOY DCSWEEP-STALE \
      DCSWEEP-ALLOWANCE-REMAINS DCSWEEP-VARIANT-REMAINS \
      LOGLAYER-MALFORMED LOGLAYER-DUPLICATE LOGLAYER-MISSING LOGLAYER-ROGUE \
      LOGLAYER-OMITTED-SCOPE LOGLAYER-EMPTY-SCOPE LOGLAYER-DECOY \
      LOGLAYER-STALE \
      EVERY-MALFORMED EVERY-DUPLICATE EVERY-MISSING EVERY-ROGUE \
      EVERY-OMITTED-UNIVERSE EVERY-EMPTY-SCOPE EVERY-EMPTY-UNIVERSE \
      EVERY-DECOY EVERY-STALE-COUNT EVERY-OMITTED-ITEM EVERY-ROGUE-EXTRA \
      EVERY-UNLINKED; do
    [ "$other" = "$marker" ] && continue
    # Match the marker's diagnostic prefix ("MARKER:"), not the bare marker, so
    # a marker that is a prefix of another (EVERY-ROGUE vs EVERY-ROGUE-EXTRA)
    # cannot be falsely reported as an extra diagnostic.
    if grep -qF -- "$other:" <<<"$out"; then
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

pass_fixture stop-escalation-positive \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-missing-response T-02 ESCALATION-UNRESPONDED \
  'ESCALATION-UNRESPONDED: T-02: declared mandatory trigger has no owner-response record' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-wrong-owner T-01 ESCALATION-WRONG-OWNER \
  'ESCALATION-WRONG-OWNER: T-01: escalation response answered by owner-bob, required owner is owner-alex' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-unexpected-response T-99 ESCALATION-UNEXPECTED-RESPONSE \
  'ESCALATION-UNEXPECTED-RESPONSE: T-99: escalation response matches no declared mandatory trigger' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-closure-masquerade T-01 ESCALATION-INVALID-RESPONSE \
  'ESCALATION-INVALID-RESPONSE: T-01: escalation response kind "closed" is not valid (must be "amendment" or "abandonment")' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-duplicate-response T-01 ESCALATION-DUPLICATE \
  'ESCALATION-DUPLICATE: T-01: duplicate escalation response' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-duplicate-scope T-01 ESCALATION-DUPLICATE \
  'ESCALATION-DUPLICATE: T-01: duplicate escalation scope link' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-empty-scope T-01 ESCALATION-UNEXPECTED-RESPONSE \
  'ESCALATION-UNEXPECTED-RESPONSE: T-01: escalation response matches no declared mandatory trigger' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-malformed-response T-01 ESCALATION-MALFORMED \
  'ESCALATION-MALFORMED: T-01: escalation response must have four tab-separated non-empty fields (trigger owner response-kind evidence)' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv
fail_fixture stop-escalation-malformed-scope T-01 ESCALATION-MALFORMED \
  'ESCALATION-MALFORMED: T-01: escalation scope link must have two tab-separated non-empty fields (trigger owner)' \
  signoffs.tsv "" "" "" "" "" "" escalation-scope.tsv escalation-responses.tsv

pass_fixture sibling-positive \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-missing F-1000 SIBLING-MISSING \
  'SIBLING-MISSING: F-1000: in-scope one-function-fix claim has no sibling record' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-stale-count F-1000 SIBLING-STALE-COUNT \
  'SIBLING-STALE-COUNT: F-1000: recorded hit count 1 does not match current hit count 2 for pattern "left_at_guard" in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-omitted-scope F-1000 SIBLING-OMITTED-SCOPE \
  'SIBLING-OMITTED-SCOPE: F-1000: sibling record omits the search scope field (id pattern scope count limit)' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-malformed F-1000 SIBLING-MALFORMED \
  'SIBLING-MALFORMED: F-1000: sibling record must have five tab-separated non-empty fields (id pattern scope count limit)' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-duplicate F-1000 SIBLING-DUPLICATE \
  'SIBLING-DUPLICATE: F-1000: duplicate sibling record' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-rogue F-9999 SIBLING-ROGUE \
  'SIBLING-ROGUE: F-9999: sibling record matches no in-scope claim' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-decoy-pattern F-1000 SIBLING-DECOY \
  'SIBLING-DECOY: F-1000: pattern "right_at_guard" matches nothing in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-decoy-scope F-1000 SIBLING-DECOY \
  'SIBLING-DECOY: F-1000: fixed function update_game_command_success is absent from the declared search scope src/other.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-decoy-comment-fn F-1000 SIBLING-DECOY \
  'SIBLING-DECOY: F-1000: fixed function update_game_command_success is absent from the declared search scope src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-decoy-string-fn F-1000 SIBLING-DECOY \
  'SIBLING-DECOY: F-1000: fixed function update_game_command_success is absent from the declared search scope src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-decoy-comment-pattern F-1000 SIBLING-DECOY \
  'SIBLING-DECOY: F-1000: pattern "left_at_guard" matches nothing in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-decoy-string-pattern F-1000 SIBLING-DECOY \
  'SIBLING-DECOY: F-1000: pattern "left_at_guard" matches nothing in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv
fail_fixture sibling-over-limit F-1000 SIBLING-OVER-LIMIT \
  'SIBLING-OVER-LIMIT: F-1000: hit count 2 exceeds the approved heuristic limit 1' \
  signoffs.tsv "" "" "" "" "" "" "" "" sibling-scope.tsv sibling.tsv "" ""

pass_fixture exhaustive-positive \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-missing F-1100 EXMATCH-MISSING \
  'EXMATCH-MISSING: F-1100: in-scope exhaustive-match claim has no evidence record' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-stale F-1100 EXMATCH-STALE \
  'EXMATCH-STALE: F-1100: recorded match count 1 does not match current count 2 in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-omitted-scope F-1100 EXMATCH-OMITTED-SCOPE \
  'EXMATCH-OMITTED-SCOPE: F-1100: exhaustive-match record omits the search scope field (id scope wildcards matches)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-empty-scope F-1100 EXMATCH-EMPTY-SCOPE \
  'EXMATCH-EMPTY-SCOPE: exhaustive-scope.tsv: exhaustive-match scope is empty - no in-scope exhaustive-match claims are enumerated' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-malformed F-1100 EXMATCH-MALFORMED \
  'EXMATCH-MALFORMED: F-1100: exhaustive-match record must have four tab-separated non-empty fields (id scope wildcards matches)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-duplicate F-1100 EXMATCH-DUPLICATE \
  'EXMATCH-DUPLICATE: F-1100: duplicate exhaustive-match record' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-rogue F-9999 EXMATCH-ROGUE \
  'EXMATCH-ROGUE: F-9999: exhaustive-match record matches no in-scope claim' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-decoy-nearby F-1100 EXMATCH-DECOY \
  'EXMATCH-DECOY: F-1100: record scope src/other.rs does not match the authoritative scope src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-decoy-vacuous F-1100 EXMATCH-DECOY \
  'EXMATCH-DECOY: F-1100: scope src/game_write.rs contains no match expression, so the no-wildcard claim is vacuous' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv
fail_fixture exhaustive-wildcard-remains F-1100 EXMATCH-WILDCARD-REMAINS \
  'EXMATCH-WILDCARD-REMAINS: F-1100: a wildcard match arm remains in src/game_write.rs (current count 5 > 0)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" exhaustive-scope.tsv exhaustive-match.tsv

pass_fixture deadcode-positive \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-missing F-1200 DCSWEEP-MISSING \
  'DCSWEEP-MISSING: F-1200: in-scope dead-code sweep claim has no evidence record' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-stale F-1200 DCSWEEP-STALE \
  'DCSWEEP-STALE: F-1200: recorded match count 1 does not match current count 2 in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-omitted-scope F-1200 DCSWEEP-OMITTED-SCOPE \
  'DCSWEEP-OMITTED-SCOPE: F-1200: dead-code record omits the sweep universe field (id universe allowances variants matches)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-empty-scope F-1200 DCSWEEP-EMPTY-SCOPE \
  'DCSWEEP-EMPTY-SCOPE: deadcode-scope.tsv: dead-code sweep scope is empty - no in-scope dead-code sweep claims are enumerated' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-malformed F-1200 DCSWEEP-MALFORMED \
  'DCSWEEP-MALFORMED: F-1200: dead-code record must have five tab-separated non-empty fields (id universe allowances variants matches)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-duplicate F-1200 DCSWEEP-DUPLICATE \
  'DCSWEEP-DUPLICATE: F-1200: duplicate dead-code record' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-rogue F-9999 DCSWEEP-ROGUE \
  'DCSWEEP-ROGUE: F-9999: dead-code record matches no in-scope claim' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-decoy-nearby F-1200 DCSWEEP-DECOY \
  'DCSWEEP-DECOY: F-1200: record universe src/other.rs does not match the authoritative universe src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-decoy-vacuous F-1200 DCSWEEP-DECOY \
  'DCSWEEP-DECOY: F-1200: universe src/game_write.rs contains no dead_code mention, so the no-allowance claim is vacuous' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-allowance-remains F-1200 DCSWEEP-ALLOWANCE-REMAINS \
  'DCSWEEP-ALLOWANCE-REMAINS: F-1200: an #[allow(dead_code)] remains in src/game_write.rs (current count 1 > 0)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv
fail_fixture deadcode-variant-remains F-1200 DCSWEEP-VARIANT-REMAINS \
  'DCSWEEP-VARIANT-REMAINS: F-1200: a dead-code suppression variant remains in src/game_write.rs (current count 5 > 0)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" deadcode-scope.tsv deadcode.tsv

pass_fixture loglayer-positive \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-missing F-1300 LOGLAYER-MISSING \
  'LOGLAYER-MISSING: F-1300: in-scope logging-related claim has no evidence record' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-stale F-1300 LOGLAYER-STALE \
  'LOGLAYER-STALE: F-1300: recorded hit count 1 does not match current hit count 2 for pattern "Log::public" in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-omitted-scope F-1300 LOGLAYER-OMITTED-SCOPE \
  'LOGLAYER-OMITTED-SCOPE: F-1300: log-layer record omits the log-layer scope field (id scope pattern count)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-empty-scope F-1300 LOGLAYER-EMPTY-SCOPE \
  'LOGLAYER-EMPTY-SCOPE: loglayer-scope.tsv: log-layer scope is empty - no in-scope logging-related claims are enumerated' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-malformed F-1300 LOGLAYER-MALFORMED \
  'LOGLAYER-MALFORMED: F-1300: log-layer record must have four tab-separated non-empty fields (id scope pattern count)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-duplicate F-1300 LOGLAYER-DUPLICATE \
  'LOGLAYER-DUPLICATE: F-1300: duplicate log-layer record' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-rogue F-9999 LOGLAYER-ROGUE \
  'LOGLAYER-ROGUE: F-9999: log-layer record matches no in-scope claim' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-decoy-nearby F-1300 LOGLAYER-DECOY \
  'LOGLAYER-DECOY: F-1300: record scope src/other.rs does not match the authoritative scope src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-decoy-lower-layer F-1300 LOGLAYER-DECOY \
  'LOGLAYER-DECOY: F-1300: record scope src/game_log.rs does not match the authoritative scope src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-decoy-comment F-1300 LOGLAYER-DECOY \
  'LOGLAYER-DECOY: F-1300: scope src/game_write.rs contains no direct Log::public invocation, so the coverage claim is vacuous' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-decoy-string F-1300 LOGLAYER-DECOY \
  'LOGLAYER-DECOY: F-1300: scope src/game_write.rs contains no direct Log::public invocation, so the coverage claim is vacuous' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-decoy-same-scope F-1300 LOGLAYER-DECOY \
  'LOGLAYER-DECOY: F-1300: pattern "FORTUNE" count 2 is not tied to the direct Log::public invocations in src/game_write.rs' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv
fail_fixture loglayer-decoy-equal-count F-1300 LOGLAYER-DECOY \
  'LOGLAYER-DECOY: F-1300: pattern "Log::public;" count 2 reproduces on different source lines than the direct Log::public invocations in src' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" loglayer-scope.tsv loglayer.tsv

pass_fixture every-positive \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-missing F-1400 EVERY-MISSING \
  'EVERY-MISSING: F-1400: in-scope every-item claim has no checklist records' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-stale-count F-1400 EVERY-STALE-COUNT \
  'EVERY-STALE-COUNT: F-1400: recorded item count 2 does not match current count 3 in universe.txt' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-omitted-item F-1400 EVERY-OMITTED-ITEM \
  'EVERY-OMITTED-ITEM: F-1400: universe item splendor-1 has no per-item checklist record' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-omitted-universe F-1400 EVERY-OMITTED-UNIVERSE \
  'EVERY-OMITTED-UNIVERSE: F-1400: every-item record omits the item universe field (id universe count item)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-empty-universe F-1400 EVERY-EMPTY-UNIVERSE \
  'EVERY-EMPTY-UNIVERSE: universe.txt: the authoritative universe names no items (empty universe)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-malformed F-1400 EVERY-MALFORMED \
  'EVERY-MALFORMED: F-1400: every-item record must have four tab-separated non-empty fields (id universe count item)' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-duplicate F-1400 EVERY-DUPLICATE \
  'EVERY-DUPLICATE: F-1400: duplicate every-item checklist record for item alhambra-1' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-rogue F-9999 EVERY-ROGUE \
  'EVERY-ROGUE: F-9999: every-item checklist record matches no in-scope claim' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-rogue-extra F-1400 EVERY-ROGUE-EXTRA \
  'EVERY-ROGUE-EXTRA: F-1400: checklist record names item splendor-1 that is not in the authoritative universe universe.txt' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-decoy-nearby F-1400 EVERY-DECOY \
  'EVERY-DECOY: F-1400: record universe other.txt does not match the authoritative universe universe.txt' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-decoy-universe-missing F-1400 EVERY-DECOY \
  'EVERY-DECOY: F-1400: declared universe not found: missing.txt' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-empty-scope F-1400 EVERY-EMPTY-SCOPE \
  'EVERY-EMPTY-SCOPE: every-scope.tsv: every-item scope is empty - no in-scope every-item claims are enumerated' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv
fail_fixture every-unlinked F-9999 EVERY-UNLINKED \
  'EVERY-UNLINKED: F-9999: every-item scope claim is not linked to a sign-off record in signoffs.tsv' \
  signoffs.tsv "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" "" every-scope.tsv every.tsv

if [ "$fail" -ne 0 ]; then
  echo "check-four-tooth: FAIL" >&2
  exit 1
fi
echo "PASS: four-tooth guard contract fixtures all behave as specified."
