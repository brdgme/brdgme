#!/usr/bin/env bash
#
# check-four-tooth.sh - CI guard for the four-tooth sign-off rule (4.1).
#
# Each line of the sign-off records file is one closed finding that must
# satisfy all four teeth before it may be marked closed:
#   tooth 1 - the source citation still exists (the cited file is present and
#             the cited symbol appears in it). (F-109: the fix and its test
#             were deleted together and the row still read closed.)
#   tooth 2 - the citation is reachable: at least one actual invocation of the
#             cited symbol - a call `symbol(`, never a comment or string
#             mention - in a *.rs file other than the cited file. Test modules
#             are excluded from reachability whether or not a named test
#             resolves. (F-147: send_turn_reminder existed and never had a
#             caller.)
#   tooth 3 - a named regression test exists and its body actually invokes the
#             target symbol (a call, not a comment or string mention), so the
#             test genuinely exercises the fix. (F-151/F-161d: tests that
#             name-match their risk without ever calling the function under
#             test.)
#   tooth 4 - a finding whose original premise the closing commit disproved
#             carries an explicit amendment (amended text present and not
#             identical to the original premise). (F-205: dp F12 was never
#             true and the finding text was never amended.)
#
# WP provenance gate (4.3): a work package may not be marked done without
# either an approved specification or at least one checklist row. When a scope
# file is given the guard deterministically enumerates every in-scope work
# package from it and fails any completed WP with neither. Two optional inputs:
#   wp-scope.tsv        one in-scope WP ID per line (e.g. WP-01). The
#                       authoritative enumeration, derived in production from
#                       the commit range / registry headings and never from the
#                       provenance rows - so omitting a completed WP's evidence
#                       row cannot hide it. (default: none, gate skipped)
#   wp-provenance.tsv   tab-separated rows, one per in-scope WP, fields:
#     wp                 WP ID
#     spec               "y" when an approved specification exists (active or
#                        archive), anything else means none
#     checklist          "y" when at least one checklist row exists, anything
#                        else means none
#     completed          "y" when the WP is closed/landed, anything else means
#                        open (parked, deferred, blocked)
#   Every row must be exactly four tab-separated fields, none empty - a
#   truncated row (e.g. a missing completed marker) is malformed and fails the
#   gate rather than silently counting the WP as open.
# WP diagnostics (the WP ID is interpolated):
#   WP-MALFORMED:   <wp>: provenance record must have four tab-separated
#                         non-empty fields (wp spec checklist completed)
#   WP-UNACCOUNTED: <wp>: in-scope work package has no provenance record
#   WP-DUPLICATE:   <wp>: duplicate provenance record
#   WP-NO-PROVENANCE: <wp>: completed work package has neither specification
#                     nor checklist evidence
#
# Deferral-routing gate (4.4): a deferral is a state (`routed-to: WP-NN`), never
# `closed`. A finding deferred from one work package to another is routed, not
# closed; the receiving WP's spec must name every inherited finding. Four
# optional inputs, enabled when a routing scope file is given:
#   routing-scope.tsv       one expected routing link per line, tab-separated
#                           fields (finding sender receiver). The authoritative
#                           enumeration, derived in production from the corpus
#                           routing records and never from the routing records
#                           or declarations below - so omitting a sender's
#                           routing record or a receiver's declaration cannot
#                           hide an expected link. An empty scope is valid only
#                           with no routing records, declarations, or closures;
#                           any routing content under an empty scope is rejected.
#   routing.tsv             tab-separated routing records, one per routed
#                           finding, fields (finding sender receiver state).
#                           state must be exactly `routed-to: <receiver>`; the
#                           4.4 failure mode recorded state as `closed`. Every
#                           record must correspond exactly to an expected scope
#                           link - a rogue record cannot authorize a fabricated
#                           declaration.
#   routing-declarations.tsv  tab-separated receiver declarations, one per
#                           inherited finding, fields (finding receiver): the
#                           receiving WP's spec naming the routed finding.
#   routing-closures.tsv    tab-separated closure attributions, fields (finding
#                           wp): the WP that closed the routed finding. Closure
#                           attribution is explicit here and never inferred from
#                           the generic sign-off records: a closure by the
#                           sender fails; a closure by the receiving WP is
#                           allowed only when the receiver's explicit
#                           declaration exactly matches the routed finding; a
#                           closure by any other WP fails.
# Routing diagnostics (the finding ID is interpolated):
#   ROUTING-MALFORMED:      <finding>: routing scope link must have three
#                           tab-separated non-empty fields (finding sender
#                           receiver) | routing record must have four
#                           tab-separated non-empty fields (finding sender
#                           receiver state) | routing declaration must have two
#                           tab-separated non-empty fields (finding receiver) |
#                           routing closure must have two tab-separated
#                           non-empty fields (finding wp)
#   ROUTING-DUPLICATE:      <finding>: duplicate routing scope link <sender> ->
#                           <receiver> / duplicate routing record / duplicate
#                           routing declaration for receiver <receiver> /
#                           duplicate routing closure attributed to <wp>
#   ROUTING-UNACCOUNTED:    <finding>: expected routing link <sender> ->
#                           <receiver> has no routing record
#   ROUTING-UNEXPECTED-RECORD: <finding>: routing record <sender> -> <receiver>
#                           matches no expected routing link
#   ROUTING-INVALID-STATE:  <finding>: deferral state "<state>" is not valid
#                           (must be "routed-to: <receiver>")
#   ROUTING-CLOSED-BY-SENDER: <finding>: routed finding closed by sender <wp>
#                           (a deferral is a state, never closed)
#   ROUTING-CLOSED-BY-OTHER: <finding>: routed finding closed by <wp>, which is
#                           neither the sender nor the receiving WP
#   ROUTING-CLOSED-WITHOUT-DECLARATION: <finding>: receiver <receiver> closed the
#                           routed finding without declaring it exactly (finding
#                           receiver)
#   ROUTING-UNEXPECTED-CLOSURE: <finding>: closure attributed to <wp> but the
#                           finding has no routed link
#   ROUTING-UNDECLARED:     <finding>: receiver <receiver> has no declaration
#                           of the inherited finding
#   ROUTING-DECLARATION-MISMATCH: <finding>: declaration for <receiver> matches
#                           no routing record
#
# Stop-escalation gate (4.6): a spec's STOP-AND-REPORT trigger firing is an
# escalation to the owner, and the only valid resolutions are an owner-signed
# spec amendment or recorded abandonment of the step - silence, inferred
# approval, and ordinary completion never clear a fired trigger. Two optional
# inputs, enabled when an escalation scope file is given:
#   escalation-scope.tsv     one declared mandatory trigger per line,
#                            tab-separated fields (trigger owner). The
#                            authoritative enumeration, derived in production
#                            from the specs that declare STOP/HALT conditions
#                            and never from the response records - so omitting
#                            a response record cannot hide an unresponded
#                            trigger. (default: none, gate skipped)
#   escalation-responses.tsv tab-separated owner-response records, one per
#                            declared trigger, fields:
#     trigger                the declared mandatory trigger, matching a scope
#                            link exactly
#     owner                  who answered, matching the declared owner exactly
#     response-kind          exactly `amendment` or `abandonment`
#     evidence               where the amendment or abandonment is recorded
#   Every response must correspond exactly to a declared scope link - a rogue
#   response cannot clear a trigger, and an ordinary closure/completion record
#   is never a response. (default: escalation-responses.tsv)
# Escalation diagnostics (the trigger ID is interpolated):
#   ESCALATION-MALFORMED:      <trigger>: escalation scope link must have two
#                              tab-separated non-empty fields (trigger owner) |
#                              escalation response must have four
#                              tab-separated non-empty fields (trigger owner
#                              response-kind evidence)
#   ESCALATION-DUPLICATE:      <trigger>: duplicate escalation scope link /
#                              duplicate escalation response
#   ESCALATION-UNRESPONDED:    <trigger>: declared mandatory trigger has no
#                              owner-response record
#   ESCALATION-UNEXPECTED-RESPONSE: <trigger>: escalation response matches no
#                              declared mandatory trigger
#   ESCALATION-WRONG-OWNER:    <trigger>: escalation response answered by
#                              <owner>, required owner is <required-owner>
#   ESCALATION-INVALID-RESPONSE: <trigger>: escalation response kind "<kind>"
#                              is not valid (must be "amendment" or
#                              "abandonment")
#
# Sibling-sweep gate (4.9a): a one-function fix claims it swept the fixed
# function's file for structurally identical siblings (pattern 2). The claim
# must name a reproducible sibling-search pattern and a recorded current hit
# count, so a later reader can re-run the search and confirm the count still
# holds. Two optional inputs, enabled when a sibling scope file is given:
#   sibling-scope.tsv   one in-scope one-function-fix claim per line,
#                       tab-separated fields (id function). The authoritative
#                       enumeration, derived in production from the commit
#                       range / fix records and never from the sibling records -
#                       so omitting a claim's evidence record cannot hide it.
#                       `function` is the function the one-function fix landed
#                       in; the declared search scope must contain it.
#                       (default: none, gate skipped)
#   sibling.tsv         tab-separated evidence records, one per in-scope claim,
#                       fields:
#     id                 claim ID, matching a scope link exactly
#     pattern            the reproducible sibling-search pattern, as accepted
#                        by `grep -E`
#     scope              the search scope: a file or directory path relative
#                        to CWD; the hit count is the number of matching lines
#                        returned by `grep -E <pattern>` over the
#                        comment/string-stripped content of the scope
#     count              the recorded current hit count (non-negative integer)
#     limit              the approved heuristic limit (non-negative integer)
#                        on structurally identical siblings for this claim
#   Every record must correspond exactly to an in-scope claim - a rogue record
#   cannot authorize a fabricated sweep - and the search must be applicable:
#   the scope must exist, contain the fixed function named in the scope link,
#   and the pattern must match at least the fixed code, so a pattern that
#   matches nothing is a decoy, not evidence of "no siblings". Both the
#   fixed-function presence check and the pattern re-run operate on the
#   comment/string-stripped scope content (the same awk stripper the four teeth
#   and the log-layer gate use), so a comment-only or string-only mention of a
#   claimed function or of a pattern is never evidence of a real sibling search.
#   The re-run must reproduce the recorded count and must not exceed the
#   approved heuristic limit.
# Sibling-sweep heuristic limits: the presence and pattern scans strip comments
# (`//`, `/* */`, including multi-line) and string literals first and scan every
# regular file under the declared scope. The stripper is not a Rust parse: it
# cannot see a function or pattern reference split across lines or one living in
# a file the scope does not enumerate, and a multi-line block comment can join
# neighbouring lines. A pattern whose only matches live in a comment or string
# literal re-runs to zero on the stripped content and is rejected as a decoy
# before the recorded count is compared, so a count that reproduces only over
# comment/string matches can never authorize a fabricated sweep.
# Sibling diagnostics (the claim ID is interpolated):
#   SIBLING-MALFORMED:      <id>: sibling scope link must have two
#                           tab-separated non-empty fields (id function) |
#                           sibling record must have five tab-separated
#                           non-empty fields (id pattern scope count limit) |
#                           count and limit must be non-negative integers
#   SIBLING-DUPLICATE:      <id>: duplicate sibling scope link / duplicate
#                           sibling record
#   SIBLING-MISSING:        <id>: in-scope one-function-fix claim has no
#                           sibling record
#   SIBLING-ROGUE:          <id>: sibling record matches no in-scope claim
#   SIBLING-OMITTED-SCOPE:  <id>: sibling record omits the search scope field
#                           (id pattern scope count limit)
#   SIBLING-STALE-COUNT:    <id>: recorded hit count <count> does not match
#                           the current hit count re-run against the scope
#   SIBLING-DECOY:          <id>: the search is not applicable - the declared
#                           search scope does not exist, the fixed function is
#                           absent from it, or the pattern matches nothing
#   SIBLING-OVER-LIMIT:     <id>: hit count exceeds the approved heuristic
#                           limit
#
# Exhaustive-match gate (4.9b): a fix that makes a `match` exhaustive without a
# wildcard arm (pattern 5 - the `_ => <default>` substitution) must name its
# scope and prove no wildcard match arm remains there, reproducibly. The proof
# is a textual Bash heuristic over the declared scope, documented below; it is
# an auditable approximation, never a parser-grade guarantee. Two optional
# inputs, enabled when an exhaustive scope file is given:
#   exhaustive-scope.tsv  one in-scope exhaustive-match claim per line,
#                         tab-separated fields (id scope). `scope` is the
#                         source file or directory, relative to CWD, that the
#                         claim sweeps. The authoritative enumeration, derived
#                         in production from the commit range / fix records and
#                         never from the evidence records - so omitting a
#                         claim's evidence record cannot hide it.
#                         (default: none, gate skipped)
#   exhaustive-match.tsv  tab-separated evidence records, one per in-scope
#                         claim, fields:
#     id                   claim ID, matching a scope link exactly
#     scope                the search scope, relative to CWD; must equal the
#                          authoritative scope named in the scope link exactly,
#                          so a record scanning a different scope is a nearby
#                          scan, not evidence for this claim
#     wildcards            the recorded count of wildcard match arms returned
#                          by the guard's wildcard-arm heuristic
#     matches              the recorded count of `match` occurrences returned
#                          by the guard's match-presence heuristic (>= 1)
#   The guard re-runs both heuristics against the declared scope. The wildcard
#   count must reproduce at 0 - any remaining wildcard arm is rejected in any
#   variant the heuristic covers. The match count must reproduce the recorded
#   value, and the scope must actually contain at least one `match`: a scope
#   with no match makes a "no wildcard arm remains" claim vacuous, so it is
#   rejected rather than counted as proof. The authoritative scope must be
#   non-empty: an empty scope file is rejected outright, because an
#   exhaustive-match sign-off with no enumerated claims proves nothing. Every
#   record must correspond exactly to an in-scope claim - a rogue record cannot
#   authorize a fabricated sweep.
# Exhaustive-match diagnostics (the claim ID is interpolated):
#   EXMATCH-MALFORMED:      <id>: exhaustive-match scope link must have two
#                           tab-separated non-empty fields (id scope) |
#                           exhaustive-match record must have four
#                           tab-separated non-empty fields (id scope wildcards
#                           matches) | wildcards and matches must be
#                           non-negative integers
#   EXMATCH-DUPLICATE:      <id>: duplicate exhaustive-match scope link /
#                           duplicate exhaustive-match record
#   EXMATCH-MISSING:        <id>: in-scope exhaustive-match claim has no
#                           evidence record
#   EXMATCH-ROGUE:          <id>: exhaustive-match record matches no in-scope
#                           claim
#   EXMATCH-OMITTED-SCOPE:  <id>: exhaustive-match record omits the search
#                           scope field (id scope wildcards matches)
#   EXMATCH-EMPTY-SCOPE:    <scope-file>: the authoritative scope file names no
#                           in-scope exhaustive-match claims (empty scope)
#   EXMATCH-DECOY:          <id>: the scan is not applicable - the record's
#                           scope differs from the authoritative scope, the
#                           declared scope does not exist, or the scope
#                           contains no `match` expression, so the no-wildcard
#                           claim is vacuous
#   EXMATCH-STALE:          <id>: recorded match count <count> does not match
#                           the current re-run, or the recorded wildcard count
#                           <count> does not match the current re-run
#   EXMATCH-WILDCARD-REMAINS: <id>: a wildcard match arm remains in <scope>
#                           (current count <count> > 0)
#
# Exhaustive-match heuristic limits: the wildcard-arm heuristic matches a line
# whose first non-space text is `_` optionally followed by whitespace plus an
# `@` bind or an `if` guard, then optional whitespace and `=>`. It catches the
# plain `_ =>`, space-less `_=>`, `_ @ bind =>`, and `_ if guard =>` variants.
# It is not a Rust parse: it cannot see a `_` nested inside another arm pattern
# (e.g. `(_, x) =>` or `Some((_, y)) =>`), an arm inside a single-line
# `match { ... }`, or a wildcard split across lines, and any line that spells a
# wildcard shape (even in a comment or string) is counted. The match-presence
# heuristic counts word-boundary `match` occurrences line-by-line, so a comment
# or string containing the word `match` also counts. Both scans include every
# file under the declared scope (no *.rs filter), and the guard cannot detect a
# wildcard arm living outside the declared scope - the authoritative scope
# enumeration must be complete for the proof to mean anything.
#
# Dead-code sweep gate (4.9c): a fix that removes or documents dead code must
# sweep the approved universe for remaining `#[allow(dead_code)]` suppressions
# and prove none remains, reproducibly. The proof is a textual Bash heuristic
# over the declared universe, documented below; it is an auditable
# approximation, never a parser-grade guarantee. Two optional inputs, enabled
# when a dead-code scope file is given:
#   deadcode-scope.tsv one in-scope dead-code sweep claim per line,
#                      tab-separated fields (id universe). `universe` is the
#                      source file or directory, relative to CWD, covering the
#                      closure-register commit range the claim sweeps. The
#                      authoritative enumeration, derived in production from the
#                      commit range / closure register and never from the
#                      evidence records - so omitting a claim's evidence record
#                      cannot hide it. (default: none, gate skipped)
#   deadcode.tsv       tab-separated evidence records, one per in-scope claim,
#                      fields:
#     id                   claim ID, matching a scope link exactly
#     universe             the sweep universe, relative to CWD; must equal the
#                          authoritative universe named in the scope link
#                          exactly, so a record scanning a different path is a
#                          nearby scan, not evidence for this claim
#     allowances           the recorded count of dead-code allowance occurrences
#                          (an `allow` of the `dead_code` lint, with optional
#                          bounded same-line whitespace between the lint name
#                          and the opening parenthesis, e.g.
#                          `#[allow(dead_code)]` or `#[allow (dead_code)]`)
#                          returned by the guard's dead-code allowance heuristic
#     variants             the recorded count of relevant dead-code suppression
#                          variants (an `allow` of the `unused` or `warnings`
#                          lint group, both of which cover dead_code, or an
#                          `expect(dead_code)`, `expect(unused)` or
#                          `expect(warnings)` lint expectation; each lint name
#                          may carry the same bounded same-line whitespace
#                          before its opening parenthesis, e.g.
#                          `#[allow (unused)]`) returned by the guard's
#                          suppression-variant heuristic
#     matches              the recorded count of `dead_code` occurrences
#                          returned by the guard's dead-code presence heuristic
#                          (>= 1)
#   The guard re-runs all three heuristics against the declared universe. The
#   allowance count must reproduce at 0 - any remaining `#[allow(dead_code)]` is
#   rejected - and the variant count must reproduce at 0 - any unreported
#   suppression variant is rejected, so an alternate spelling cannot bypass the
#   sweep. The match count must reproduce the recorded value, and the universe
#   must actually contain at least one `dead_code` mention: a universe with no
#   dead-code content makes a "no allowance remains" claim vacuous, so it is
#   rejected rather than counted as proof. The authoritative universe must be
#   non-empty: an empty scope file is rejected outright, because a dead-code
#   sweep sign-off with no enumerated claims proves nothing. Every record must
#   correspond exactly to an in-scope claim - a rogue record cannot authorize a
#   fabricated sweep.
# Dead-code sweep diagnostics (the claim ID is interpolated):
#   DCSWEEP-MALFORMED:      <id>: dead-code scope link must have two
#                           tab-separated non-empty fields (id universe) |
#                           dead-code record must have five tab-separated
#                           non-empty fields (id universe allowances variants
#                           matches) | allowances, variants and matches must be
#                           non-negative integers
#   DCSWEEP-DUPLICATE:      <id>: duplicate dead-code scope link / duplicate
#                           dead-code record
#   DCSWEEP-MISSING:        <id>: in-scope dead-code sweep claim has no evidence
#                           record
#   DCSWEEP-ROGUE:          <id>: dead-code record matches no in-scope claim
#   DCSWEEP-OMITTED-SCOPE:  <id>: dead-code record omits the sweep universe field
#                           (id universe allowances variants matches)
#   DCSWEEP-EMPTY-SCOPE:    <scope-file>: the authoritative scope file names no
#                           in-scope dead-code sweep claims (empty scope)
#   DCSWEEP-DECOY:          <id>: the sweep is not applicable - the record's
#                           universe differs from the authoritative universe,
#                           the declared universe does not exist, or the
#                           universe contains no `dead_code` mention, so the
#                           no-allowance claim is vacuous
#   DCSWEEP-STALE:          <id>: recorded allowance count <count> does not
#                           match the current re-run, or the recorded variant
#                           count <count> does not match the current re-run, or
#                           the recorded match count <count> does not match the
#                           current re-run
#   DCSWEEP-ALLOWANCE-REMAINS: <id>: an `#[allow(dead_code)]` (or bounded
#                           same-line whitespace spelling such as
#                           `#[allow (dead_code)]`) remains in <universe>
#                           (current count <count> > 0)
#   DCSWEEP-VARIANT-REMAINS: <id>: a dead-code suppression variant remains in
#                           <universe> (current count <count> > 0)
#
# Dead-code sweep heuristic limits: the dead-code presence heuristic counts
# word-boundary `dead_code` occurrences line-by-line, so every `allow(dead_code)`
# and `deny(dead_code)` line also counts. The allowance heuristic counts a line
# whose text contains `allow`, optional whitespace, and `(` ... `dead_code`
# inside one `)`-closed group, so it catches `#[allow(dead_code)]`,
# `#[allow (dead_code)]`, the inner `#![allow(dead_code)]`,
# `#[allow(dead_code, ...)]`, and a single-line `cfg_attr(..., allow(dead_code))`.
# The suppression-variant heuristic counts a line whose text contains `allow`,
# optional whitespace, and `(` ... `unused` or `warnings` inside one group (both
# the `unused` and `warnings` lint groups cover dead_code) or `expect`, optional
# whitespace, and `(` ... `dead_code`, `unused` or `warnings` inside one group (a
# lint expectation naming a lint or group that covers dead_code). The whitespace
# between the lint name and the opening `(` is bounded to the line: the scan is
# line-by-line, so a line break between the lint name and `(` is not seen, and
# an attribute split across lines is not seen.
# A line matching both the allowance and the variant heuristic (e.g.
# `#[allow(dead_code, unused)]`) is counted in both counts. Neither scan is a
# Rust parse: an attribute split across lines, a suppression spelled differently,
# or any suppression living outside the declared universe is not seen, and any
# line that spells the shape (even in a comment or string) is counted. Both scans
# include every file under the declared universe (no *.rs filter), and the guard
# cannot detect a suppression outside the declared universe - the authoritative
# universe enumeration must be complete for the proof to mean anything.
#
# Log-layer proof gate (4.9d): a logging-related claim - a hidden-information
# leak closed at the log layer, or a claim of `Log::public` content coverage -
# must prove direct `Log::public` content at the authoritative approved
# log-layer source scope, reproducibly. The proof is a textual Bash heuristic
# over the declared scope, documented below; it is an auditable approximation,
# never a parser-grade or semantic proof of the log content. Two optional
# inputs, enabled when a log-layer scope file is given:
#   loglayer-scope.tsv one in-scope logging-related claim per line,
#                      tab-separated fields (id scope). `scope` is the
#                      authoritative approved log-layer source file or
#                      directory, relative to CWD, where the claim's direct
#                      `Log::public` content lives. The authoritative
#                      enumeration, derived in production from the commit range
#                      / fix records and never from the evidence records - so
#                      omitting a claim's evidence record cannot hide it.
#                      (default: none, gate skipped)
#   loglayer.tsv       tab-separated evidence records, one per in-scope claim,
#                      fields:
#     id                   claim ID, matching a scope link exactly
#     scope                the log-layer scope, relative to CWD; must equal the
#                          authoritative scope named in the scope link exactly,
#                          so a record scanning a wrapper, caller, or
#                          lower/upper-layer file is a nearby scan, not
#                          evidence for this claim
#     pattern              the approved direct `Log::public` pattern, as
#                          accepted by `grep -E`; the recorded count is the
#                          number of matching lines returned by
#                          `grep -E <pattern>` over the comment/string-stripped
#                          content of the scope
#     count                the recorded current count of direct `Log::public`
#                          invocations in the scope (non-negative integer)
#   The guard re-runs two heuristics against the declared scope and requires
#   both to reproduce the recorded count on the same source lines: the
#   direct-invocation heuristic (comment/string-stripped `Log::public(` calls)
#   and the approved pattern applied to that same stripped content. Each
#   heuristic yields a set of `file:line` identities and the two sets must be
#   identical - a pattern whose count merely equals the direct count while its
#   matches land on different source lines is rejected as a decoy, never
#   accepted as evidence. The recorded pattern/count is therefore demonstrably
#   tied to direct `Log::public` invocations - a pattern whose matches come only
#   from comments or strings, or a count that merely coexists with a separate
#   direct call, is rejected as a decoy rather than accepted as evidence. The
#   scope must actually contain
#   at least one direct `Log::public` invocation (a vacuous scope is not
#   proof), the authoritative scope must be non-empty (an empty scope file is
#   rejected outright, because a log-layer sign-off with no enumerated claims
#   proves nothing), and every record must correspond exactly to an in-scope
#   claim - a rogue record cannot authorize a fabricated proof.
# Log-layer diagnostics (the claim ID is interpolated):
#   LOGLAYER-MALFORMED:      <id>: log-layer scope link must have two
#                           tab-separated non-empty fields (id scope) |
#                           log-layer record must have four tab-separated
#                           non-empty fields (id scope pattern count) |
#                           count must be a non-negative integer
#   LOGLAYER-DUPLICATE:      <id>: duplicate log-layer scope link / duplicate
#                           log-layer record
#   LOGLAYER-MISSING:        <id>: in-scope logging-related claim has no
#                           evidence record
#   LOGLAYER-ROGUE:          <id>: log-layer record matches no in-scope claim
#   LOGLAYER-OMITTED-SCOPE:  <id>: log-layer record omits the log-layer scope
#                           field (id scope pattern count)
#   LOGLAYER-EMPTY-SCOPE:    <scope-file>: the authoritative scope file names
#                           no in-scope logging-related claims (empty scope)
#   LOGLAYER-DECOY:          <id>: the proof is not applicable - the record's
#                           scope differs from the authoritative scope, the
#                           declared scope does not exist, the scope contains
#                           no direct `Log::public` invocation, or the recorded
#                           pattern/count is not tied to the direct
#                           comment/string-stripped `Log::public` invocations
#                           (a mixed-layer, coexisting, or different-lines
#                           count), so the coverage claim is vacuous
#   LOGLAYER-STALE:          <id>: recorded count <count> does not match the
#                           current direct-invocation count <count> for pattern
#                           "<pattern>" in <scope>
#
# Log-layer heuristic limits: the direct-invocation heuristic strips comments
# (`//`, `/* */`, including multi-line) and string literals first (the same awk
# stripper the four teeth use) and counts a line whose remaining text directly
# invokes `Log::public` - `Log::public` not as a suffix of a longer identifier,
# followed by optional spaces and `(`. The approved pattern is re-run over the
# same stripped content, so its matches must reproduce the recorded
# direct-invocation count on exactly the same source lines: a pattern whose raw
# matches include comment/string-only lines in addition to real calls, whose
# stripped matches diverge from the direct invocations, or whose count merely
# equals the direct count while the matches land on different lines, is
# rejected as a decoy - a pattern/count that merely coexists with a separate
# direct call is not proof at the direct logging layer. Line identity is
# `file:line`, so the same line number in different *.rs files of a multi-file
# scope does not collide: the two scans must agree file-for-file. A recorded
# count that does not reproduce while
# the pattern's raw matches still coincide with the direct count is plain stale
# evidence (the code changed); a divergence between the raw and direct counts
# is a mixed-layer decoy. Neither scan is a Rust parse: it cannot see a
# reference split across lines, a `Log::public` reached through a wrapper
# function or a helper at a different layer, or a reference living outside the
# declared scope. Both the direct-invocation scan and the stripped-pattern scan
# cover the scope's *.rs files (the log layer is Rust code); the pattern's raw
# scan is used only to tell stale evidence from a mixed-layer decoy, never as
# proof. The guard cannot detect coverage outside the declared scope - the
# authoritative scope enumeration must be complete for the proof to mean
# anything. The proof is that the recorded direct `Log::public` references
# reproduce at the authoritative scope; it is not a semantic proof of the log
# content - a direct `Log::public` reference can still carry a leak, and this
# guard does not and cannot read the logged content.
#
# Every-item checklist gate (4.9e): a broad sign-off claim equivalent to
# "every X" requires an authoritative, reproducible, non-empty item universe
# and exactly one explicit per-item checklist record for every universe item -
# a prose claim alone proves nothing (WP-10 3a shape: "every game crate" was
# applied to three of 28). Two optional inputs, enabled when an every-item
# scope file is given:
#   every-scope.tsv   one in-scope "every X" claim per line, tab-separated
#                     fields (id universe). `universe` is the authoritative,
#                     reproducible item universe: a file, relative to CWD,
#                     whose distinct non-blank lines enumerate the universe
#                     items. The authoritative enumeration, derived in
#                     production from the spec's enumerated list / commit range
#                     and never from the checklist records - so omitting a
#                     claim's checklist records cannot hide an uncovered
#                     universe. Every claim must also be linked to an actual
#                     sign-off record in the RECORDS input, by claim ID: an
#                     "every X" scope claim with no linked sign-off record is
#                     an unlinked claim and is rejected. (default: none, gate
#                     skipped)
#   every.tsv         tab-separated per-item checklist records, one per
#                     universe item, fields:
#     id               claim ID, matching a scope link exactly
#     universe         the item universe, relative to CWD; must equal the
#                      authoritative universe named in the scope link exactly,
#                      so a record pointing at a different list is a nearby
#                      scan, not evidence for this claim
#     count            the recorded current universe item count (the number of
#                      distinct non-blank lines in the universe file); every
#                      accepted record's count must equal the authoritative
#                      current universe count, so a stale count on any one
#                      record - not just the first - fails the claim
#     item             the per-item checklist entry, matching one of the
#                      universe's enumerated items exactly (textual equality)
#   The checklist items must cover the authoritative universe one-to-one: an
#   omitted universe item, a duplicate item, and a rogue extra item are all
#   rejected, an empty universe proves nothing, and every record's count must
#   reproduce. Every record must correspond exactly to an in-scope claim - a
#   rogue record cannot authorize a fabricated checklist.
# Every-item diagnostics (the claim ID is interpolated):
#   EVERY-MALFORMED:      <id>: every-item scope link must have two
#                         tab-separated non-empty fields (id universe) |
#                         every-item record must have four tab-separated
#                         non-empty fields (id universe count item) | count
#                         must be a non-negative integer
#   EVERY-DUPLICATE:      <id>: duplicate every-item scope link / duplicate
#                         every-item checklist record for item <item>
#   EVERY-MISSING:        <id>: in-scope every-item claim has no checklist
#                         records
#   EVERY-ROGUE:          <id>: every-item checklist record matches no
#                         in-scope claim
#   EVERY-UNLINKED:       <id>: every-item scope claim is not linked to a
#                         sign-off record in <records>
#   EVERY-OMITTED-UNIVERSE: <id>: every-item record omits the item universe
#                         field (id universe count item)
#   EVERY-EMPTY-SCOPE:    <scope-file>: every-item scope is empty - no
#                         in-scope every-item claims are enumerated
#   EVERY-EMPTY-UNIVERSE: <universe>: the authoritative universe names no
#                         items (empty universe)
#   EVERY-DECOY:          <id>: record universe <universe> does not match the
#                         authoritative universe <authoritative> | declared
#                         universe not found: <universe>
#   EVERY-STALE-COUNT:    <id>: recorded item count <count> does not match
#                         current count <count> in <universe>
#   EVERY-OMITTED-ITEM:   <id>: universe item <item> has no per-item checklist
#                         record
#   EVERY-ROGUE-EXTRA:    <id>: checklist record names item <item> that is not
#                         in the authoritative universe <universe>
#
# Every-item checklist heuristic limits: the universe enumeration is textual -
# the universe is a file whose distinct non-blank lines are the items, matched
# to checklist records by exact text equality. The guard establishes only
# enumerated input coverage: it proves the checklist records enumerate exactly
# the authoritative universe's items, one per item, that every record's count
# reproduces the current universe count, and that each scope claim is linked to
# a sign-off record by ID. It does not and cannot verify that any claimed fix or
# scope actually covers an item's substance, nor the substance of the linked
# sign-off record - a checklist record asserts enumerated
# input coverage only, never that the underlying per-item work is correct,
# complete, or true, and an item's line text need not correspond to any real
# artifact. The guard cannot detect an item or checklist record living outside
# the authoritative universe - the authoritative universe enumeration must be
# complete for the proof to mean anything.
#
# Records are tab-separated, one per line, fields in order:
#   id                finding ID (e.g. F-109)
#   symbol            cited symbol (teeth 1-3)
#   file              cited source file, relative to CWD (tooth 1)
#   test              named regression test, "-" when the row is not Test? y
#                     (tooth 3)
#   premise-disproved "y" when the closing commit disproved the original
#                     premise, anything else means no (tooth 4)
#   premise           the original finding text / mechanism claim, "-" when
#                     none
#   amendment         the amended finding text, "-" when none (tooth 4)
#
# A test module is a *.rs file under a `tests/` directory, named `tests.rs` or
# `*_test.rs`, or containing `#[cfg(test)]`. An invocation is `symbol` (not as
# a suffix of a longer identifier) followed by optional spaces and `(`, with
# comments (`//`, `/* */`, including multi-line) and string literals stripped
# first - so a comment or a string that merely mentions the symbol is never
# treated as a caller or as test exercise.
#
# Field values must not contain tabs. Every violation is diagnosed with the
# finding ID (or WP ID) and the exact failing tooth or gate, and the run exits
# non-zero. All paths are resolved relative to CWD, so the fixtures invoke the
# script from their own directory (same convention as check-delivery-lists.sh).
# Usage: check-four-tooth.sh [RECORDS [SCOPE [PROVENANCE [ROUTING-SCOPE
# [ROUTING-RECORDS [ROUTING-DECLARATIONS [ROUTING-CLOSURES [ESCALATION-SCOPE
# [ESCALATION-RESPONSES [SIBLING-SCOPE [SIBLING-RECORDS [EXMATCH-SCOPE
# [EXMATCH-RECORDS [DCSWEEP-SCOPE [DCSWEEP-RECORDS [LOGLAYER-SCOPE
# [LOGLAYER-RECORDS [EVERY-SCOPE [EVERY-RECORDS]]]]]]]]]]]]]]]]]]]] - RECORDS
# defaults to signoffs.tsv, SCOPE defaults to none (4.3 gate skipped),
# PROVENANCE defaults to wp-provenance.tsv, ROUTING-SCOPE defaults to none
# (4.4 gate skipped), ROUTING-RECORDS to routing.tsv, ROUTING-DECLARATIONS to
# routing-declarations.tsv, ROUTING-CLOSURES to routing-closures.tsv,
# ESCALATION-SCOPE defaults to none (4.6 gate skipped), ESCALATION-RESPONSES to
# escalation-responses.tsv, SIBLING-SCOPE defaults to none (4.9a gate skipped),
# SIBLING-RECORDS to sibling.tsv, EXMATCH-SCOPE defaults to none (4.9b gate
# skipped), EXMATCH-RECORDS to exhaustive-match.tsv, DCSWEEP-SCOPE defaults to
# none (4.9c gate skipped) and DCSWEEP-RECORDS to deadcode.tsv, LOGLAYER-SCOPE
# defaults to none (4.9d gate skipped) and LOGLAYER-RECORDS to loglayer.tsv,
# EVERY-SCOPE defaults to none (4.9e gate skipped) and EVERY-RECORDS to
# every.tsv.
# Uses only standard Bash/GNU utilities (grep, sed, awk).

set -uo pipefail

RECORDS="${1:-signoffs.tsv}"
fail=0

if [ ! -f "$RECORDS" ]; then
  echo "MISSING-RECORDS: no sign-off records file: $RECORDS" >&2
  exit 1
fi

# awk functions shared by `matches`: strip_line removes string literals and
# comments (in_block/in_str are per-file state so multi-line /* */ works);
# has_call reports an actual invocation of `sym` in the stripped text.
awk_fns='function strip_line(line,    out, i, n, c) {
  out = ""; n = length(line); i = 1
  while (i <= n) {
    c = substr(line, i, 1)
    if (in_block) {
      if (c == "*" && substr(line, i + 1, 1) == "/") { in_block = 0; i += 2; continue }
      i++; continue
    }
    if (in_str) {
      if (c == "\\") { i += 2; continue }
      if (c == "\"") in_str = 0
      i++; continue
    }
    if (c == "\"") { in_str = 1; i++; continue }
    if (c == "/" && substr(line, i + 1, 1) == "/") break
    if (c == "/" && substr(line, i + 1, 1) == "*") { in_block = 1; i += 2; continue }
    out = out c
    i++
  }
  return out
}
function has_call(s,    i, plen, prev, j) {
  i = index(s, sym)
  plen = length(sym)
  while (i > 0) {
    prev = substr(s, i - 1, 1)
    if (i == 1 || prev !~ /[A-Za-z0-9_]/) {
      j = i + plen
      while (j <= length(s) && substr(s, j, 1) == " ") j++
      if (substr(s, j, 1) == "(") return 1
    }
    s = substr(s, i + plen)
    i = index(s, sym)
  }
  return 0
}'

# matches <symbol> <file> [<fn>] -> 0 when <file> actually invokes <symbol>
# (fn omitted), or when the body of `fn <fn>` in <file> actually invokes it.
# The body runs from the `fn <fn>` line to the next top-level `fn`, `pub fn`
# or `async fn` line (or EOF).
matches() {
  awk -v sym="$1" -v fn="${3:--}" "$awk_fns"'
    BEGIN { in_block = 0; in_str = 0; inbody = 0; found = 0 }
    fn != "-" && index($0, "fn " fn) { inbody = 1; next }
    inbody && /^[[:space:]]*(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+/ { inbody = 0 }
    (fn == "-" || inbody) && has_call(strip_line($0)) { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "$2"
}

# find_file_for_fn <fn> -> first *.rs file defining `fn <fn>` ("" when none).
find_file_for_fn() {
  grep -rlw --exclude-dir=.git --include='*.rs' "fn $1" . 2>/dev/null \
    | sed 's#^\./##' | head -1
}

# is_test_module <file> -> 0 when <file> is a test module (tests/ dir,
# tests.rs / *_test.rs basename, or a #[cfg(test)] module).
is_test_module() {
  local f="$1" base
  base="$(basename "$f")"
  case "$f" in
    */tests/*) return 0 ;;
  esac
  case "$base" in
    tests.rs|*_test.rs) return 0 ;;
  esac
  grep -q '#\[cfg(test)\]' "$f" 2>/dev/null
}

# count_direct_log_public <scope> -> the number of direct `Log::public(`
# invocations across the scope's *.rs files after comments and string literals
# are stripped (the same awk stripper the four teeth use). A mention in a
# comment, a string, or as a suffix of a longer identifier is not a direct
# invocation.
count_direct_log_public() {
  local scope="$1" total=0 f
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    total=$((total + $(awk -v sym='Log::public' "$awk_fns"'
      BEGIN { in_block = 0; in_str = 0; n = 0 }
      { if (has_call(strip_line($0))) n++ }
      END { print n }
    ' "$f")))
  done < <(grep -rlw --exclude-dir=.git --include='*.rs' 'Log::public' "$scope" 2>/dev/null)
  echo "$total"
}

# loglayer_direct_lines <scope> -> one `file:line` per source line whose
# comment/string-stripped content directly invokes `Log::public`, across the
# scope's *.rs files. The per-source-line identity (file and line number
# together) is the basis the log-layer gate requires: equal counts on different
# lines must fail, and the same line number in different files must not
# collide.
loglayer_direct_lines() {
  local scope="$1" f
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    awk -v sym='Log::public' -v fname="$f" "$awk_fns"'
      BEGIN { in_block = 0; in_str = 0 }
      { if (has_call(strip_line($0))) print fname ":" NR }
    ' "$f"
  done < <(find "$scope" -type f -name '*.rs' 2>/dev/null)
}

# loglayer_stripped_lines <scope> <pattern> -> one `file:line` per source line
# whose comment/string-stripped content matches <pattern> (as `grep -E` accepts
# it), across the scope's *.rs files. Same per-file stripper and same file
# universe as loglayer_direct_lines, so the two line sets are comparable on a
# per-source-line basis.
loglayer_stripped_lines() {
  local scope="$1" pattern="$2" f
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    awk "$awk_fns"'
      BEGIN { in_block = 0; in_str = 0 }
      { print strip_line($0) }
    ' "$f" \
      | grep -nE -- "$pattern" 2>/dev/null \
      | sed -n "s|^\([0-9][0-9]*\):.*|$f:\1|p"
  done < <(find "$scope" -type f -name '*.rs' 2>/dev/null)
}

# stripped_scope <scope> -> the comment/string-stripped content of every regular
# file under <scope>, one stripped source line per output line (the same awk
# stripper the four teeth and the log-layer gate use). A mention of a claimed
# fixed function or a sibling-search pattern that lives only in a comment or
# string literal is therefore never evidence: the sibling gate re-runs its
# presence and pattern scans over this stripped content.
stripped_scope() {
  local scope="$1" f
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    awk "$awk_fns"'
      BEGIN { in_block = 0; in_str = 0 }
      { print strip_line($0) }
    ' "$f" 2>/dev/null
  done < <(find "$scope" -type f 2>/dev/null)
}

while IFS=$'\t' read -r id symbol file test pdisp premise amendment; do
  [ -n "$id" ] || continue
  cite_missing=0

  # Tooth 1 - the citation still exists.
  if [ -z "$file" ] || [ ! -f "$file" ]; then
    echo "CITATION-MISSING: $id: cited source file not found: ${file:-<none>}" >&2
    fail=1
    cite_missing=1
  elif ! grep -qw "$symbol" "$file"; then
    echo "CITATION-MISSING: $id: cited symbol not found in $file: $symbol" >&2
    fail=1
    cite_missing=1
  fi

  # Tooth 2 - the citation is reachable (only meaningful once it exists).
  if [ "$cite_missing" -eq 0 ]; then
    testfile=""
    if [ -n "$test" ] && [ "$test" != "-" ]; then
      testfile="$(find_file_for_fn "$test")"
    fi
    callers=""
    while IFS= read -r f; do
      [ -n "$f" ] || continue
      [ "$f" = "$file" ] && continue
      [ -n "$testfile" ] && [ "$f" = "$testfile" ] && continue
      is_test_module "$f" && continue
      if matches "$symbol" "$f"; then
        callers="$callers$f"$'\n'
      fi
    done < <(grep -rlw --exclude-dir=.git --include='*.rs' "$symbol" . 2>/dev/null \
      | sed 's#^\./##' | sort -u)
    if [ -z "$(printf '%s\n' "$callers" | grep -v '^$')" ]; then
      echo "CITATION-UNREACHABLE: $id: $symbol has no caller outside its definition and test modules" >&2
      fail=1
    fi
  fi

  # Tooth 3 - the named test actually invokes the target symbol.
  if [ -n "$test" ] && [ "$test" != "-" ]; then
    testfile="$(find_file_for_fn "$test")"
    if [ -z "$testfile" ]; then
      echo "DECOY-TEST: $id: named test not found: $test" >&2
      fail=1
    elif ! matches "$symbol" "$testfile" "$test"; then
      echo "DECOY-TEST: $id: test $test does not invoke $symbol" >&2
      fail=1
    fi
  fi

  # Tooth 4 - a disproved premise must be explicitly amended.
  if [ "$pdisp" = "y" ] \
    && { [ -z "$amendment" ] || [ "$amendment" = "-" ] || [ "$amendment" = "$premise" ]; }; then
    echo "UNAMENDED-PREMISE: $id: disproved premise not amended: ${premise:-<none>}" >&2
    fail=1
  fi
done < "$RECORDS"

# WP provenance gate (4.3): when a scope file is given, enumerate every in-scope
# work package deterministically and fail any completed WP with neither an
# approved specification nor at least one checklist row.
if [ -n "${2:-}" ]; then
  SCOPE="$2"
  PROVENANCE="${3:-wp-provenance.tsv}"
  if [ ! -f "$SCOPE" ]; then
    echo "MISSING-SCOPE: no WP scope file: $SCOPE" >&2
    fail=1
  elif [ ! -f "$PROVENANCE" ]; then
    echo "MISSING-PROVENANCE: no WP provenance file: $PROVENANCE" >&2
    fail=1
  else
    declare -A seen prov_spec prov_chk prov_done
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r wp p_spec p_chk p_done <<< "$line"
      if [ "${#tabs}" -ne 3 ] \
        || [ -z "$wp" ] || [ -z "$p_spec" ] || [ -z "$p_chk" ] || [ -z "$p_done" ]; then
        echo "WP-MALFORMED: ${wp:-<no-wp>}: provenance record must have four tab-separated non-empty fields (wp spec checklist completed)" >&2
        seen[$wp]=1
        fail=1
        continue
      fi
      if [ -n "${seen[$wp]:-}" ]; then
        echo "WP-DUPLICATE: $wp: duplicate provenance record" >&2
        fail=1
        continue
      fi
      seen[$wp]=1
      prov_spec[$wp]="$p_spec"
      prov_chk[$wp]="$p_chk"
      prov_done[$wp]="$p_done"
    done < "$PROVENANCE"
    while IFS= read -r wp; do
      [ -n "$wp" ] || continue
      if [ -z "${seen[$wp]:-}" ]; then
        echo "WP-UNACCOUNTED: $wp: in-scope work package has no provenance record" >&2
        fail=1
        continue
      fi
      if [ "${prov_done[$wp]:-n}" = "y" ] \
        && [ "${prov_spec[$wp]:-n}" != "y" ] \
        && [ "${prov_chk[$wp]:-n}" != "y" ]; then
        echo "WP-NO-PROVENANCE: $wp: completed work package has neither specification nor checklist evidence" >&2
        fail=1
      fi
    done < "$SCOPE"
  fi
fi

# Deferral-routing gate (4.4): a deferral is a state (`routed-to: WP-NN`), never
# `closed`. The routing scope file is the authoritative enumeration of expected
# routing links (finding -> sender -> receiver); the routing records are the
# sender's evidence, the declarations are the receiver's evidence and the
# closures are the routing-specific closure attributions, so omitting any of
# them cannot hide an expected link and no record can authorize a fabricated
# declaration.
if [ -n "${4:-}" ]; then
  RT_SCOPE="$4"
  RT_RECORDS="${5:-routing.tsv}"
  RT_DECLS="${6:-routing-declarations.tsv}"
  RT_CLOSURES="${7:-routing-closures.tsv}"
  if [ ! -f "$RT_SCOPE" ]; then
    echo "ROUTING-MISSING-SCOPE: no routing scope file: $RT_SCOPE" >&2
    fail=1
  elif [ ! -f "$RT_RECORDS" ]; then
    echo "ROUTING-MISSING-RECORDS: no routing records file: $RT_RECORDS" >&2
    fail=1
  elif [ ! -f "$RT_DECLS" ]; then
    echo "ROUTING-MISSING-DECLARATIONS: no routing declarations file: $RT_DECLS" >&2
    fail=1
  elif [ ! -f "$RT_CLOSURES" ]; then
    echo "ROUTING-MISSING-CLOSURES: no routing closures file: $RT_CLOSURES" >&2
    fail=1
  else
    # Authoritative scope: every expected link (finding sender receiver).
    declare -A rt_scope=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r rt_f rt_sender rt_receiver <<< "$line"
      if [ "${#tabs}" -ne 2 ] || [ -z "$rt_f" ] || [ -z "$rt_sender" ] || [ -z "$rt_receiver" ]; then
        echo "ROUTING-MALFORMED: ${rt_f:-<no-finding>}: routing scope link must have three tab-separated non-empty fields (finding sender receiver)" >&2
        fail=1
        continue
      fi
      key="$rt_f|$rt_sender|$rt_receiver"
      if [ -n "${rt_scope[$key]:-}" ]; then
        echo "ROUTING-DUPLICATE: $rt_f: duplicate routing scope link $rt_sender -> $rt_receiver" >&2
        fail=1
        continue
      fi
      rt_scope["$key"]=1
    done < "$RT_SCOPE"

    # Sender evidence: routing records (finding sender receiver state).
    declare -A rt_rec=() rt_fr=() rt_rec_f=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r rt_f rt_sender rt_receiver rt_state <<< "$line"
      if [ "${#tabs}" -ne 3 ] || [ -z "$rt_f" ] || [ -z "$rt_sender" ] || [ -z "$rt_receiver" ] || [ -z "$rt_state" ]; then
        echo "ROUTING-MALFORMED: ${rt_f:-<no-finding>}: routing record must have four tab-separated non-empty fields (finding sender receiver state)" >&2
        fail=1
        continue
      fi
      key="$rt_f|$rt_sender|$rt_receiver"
      if [ -n "${rt_rec[$key]:-}" ]; then
        echo "ROUTING-DUPLICATE: $rt_f: duplicate routing record $rt_sender -> $rt_receiver" >&2
        fail=1
        continue
      fi
      rt_rec["$key"]="$rt_state"
      rt_fr["$rt_f|$rt_receiver"]=1
      rt_rec_f["$rt_f"]=1
    done < "$RT_RECORDS"

    # Every routing record must correspond exactly to an authoritative scope
    # link: a rogue record that matches no expected link fails here and cannot
    # authorize a fabricated declaration.
    for key in "${!rt_rec[@]}"; do
      if [ -z "${rt_scope[$key]:-}" ]; then
        IFS='|' read -r rt_f rt_sender rt_receiver <<< "$key"
        echo "ROUTING-UNEXPECTED-RECORD: $rt_f: routing record $rt_sender -> $rt_receiver matches no expected routing link" >&2
        fail=1
      fi
    done

    # The deferral state must be exactly `routed-to: <receiver>`.
    for key in "${!rt_rec[@]}"; do
      IFS='|' read -r rt_f rt_sender rt_receiver <<< "$key"
      expected="routed-to: $rt_receiver"
      if [ "${rt_rec[$key]}" != "$expected" ]; then
        echo "ROUTING-INVALID-STATE: $rt_f: deferral state \"${rt_rec[$key]}\" is not valid (must be \"$expected\")" >&2
        fail=1
      fi
    done

    # The scope is authoritative: omitting a sender's routing record cannot
    # hide an expected link.
    for key in "${!rt_scope[@]}"; do
      if [ -z "${rt_rec[$key]:-}" ]; then
        IFS='|' read -r rt_f rt_sender rt_receiver <<< "$key"
        echo "ROUTING-UNACCOUNTED: $rt_f: expected routing link $rt_sender -> $rt_receiver has no routing record" >&2
        fail=1
      fi
    done

    # Receiver evidence: each receiving WP's spec names its inherited findings.
    declare -A rt_decl=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r rt_f rt_receiver <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$rt_f" ] || [ -z "$rt_receiver" ]; then
        echo "ROUTING-MALFORMED: ${rt_f:-<no-finding>}: routing declaration must have two tab-separated non-empty fields (finding receiver)" >&2
        fail=1
        continue
      fi
      frkey="$rt_f|$rt_receiver"
      if [ -n "${rt_decl[$frkey]:-}" ]; then
        echo "ROUTING-DUPLICATE: $rt_f: duplicate routing declaration for receiver $rt_receiver" >&2
        fail=1
        continue
      fi
      rt_decl["$frkey"]=1
    done < "$RT_DECLS"

    # Closure attribution is explicit in the routing-specific closure file
    # (finding wp), never inferred from the generic sign-off records. A closure
    # by the sender fails; a closure by the receiving WP is allowed only when
    # the receiver's explicit declaration exactly matches the routed finding; a
    # closure by any other WP fails.
    declare -A rt_closed=() rt_recv_closed=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r rt_f rt_wp <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$rt_f" ] || [ -z "$rt_wp" ]; then
        echo "ROUTING-MALFORMED: ${rt_f:-<no-finding>}: routing closure must have two tab-separated non-empty fields (finding wp)" >&2
        fail=1
        continue
      fi
      ckey="$rt_f|$rt_wp"
      if [ -n "${rt_closed[$ckey]:-}" ]; then
        echo "ROUTING-DUPLICATE: $rt_f: duplicate routing closure attributed to $rt_wp" >&2
        fail=1
        continue
      fi
      rt_closed["$ckey"]=1
      if [ -z "${rt_rec_f[$rt_f]:-}" ]; then
        echo "ROUTING-UNEXPECTED-CLOSURE: $rt_f: closure attributed to $rt_wp but the finding has no routed link" >&2
        fail=1
        continue
      fi
      is_sender=0
      is_receiver=0
      for key in "${!rt_rec[@]}"; do
        IFS='|' read -r rf rs rr <<< "$key"
        [ "$rf" = "$rt_f" ] || continue
        [ "$rt_wp" = "$rs" ] && is_sender=1
        [ "$rt_wp" = "$rr" ] && is_receiver=1
      done
      if [ "$is_sender" -eq 1 ]; then
        echo "ROUTING-CLOSED-BY-SENDER: $rt_f: routed finding closed by sender $rt_wp (a deferral is a state, never closed)" >&2
        fail=1
      elif [ "$is_receiver" -eq 1 ]; then
        rt_recv_closed["$rt_f|$rt_wp"]=1
        if [ -z "${rt_decl[$ckey]:-}" ]; then
          echo "ROUTING-CLOSED-WITHOUT-DECLARATION: $rt_f: receiver $rt_wp closed the routed finding without declaring it exactly (finding receiver)" >&2
          fail=1
        fi
      else
        echo "ROUTING-CLOSED-BY-OTHER: $rt_f: routed finding closed by $rt_wp, which is neither the sender nor the receiving WP" >&2
        fail=1
      fi
    done < "$RT_CLOSURES"

    # Every sender-routed link must be declared by its receiver (omitting a
    # receiver's declaration cannot hide the link), and every declaration must
    # match a routing record.
    for key in "${!rt_scope[@]}"; do
      [ -n "${rt_rec[$key]:-}" ] || continue
      IFS='|' read -r rt_f rt_sender rt_receiver <<< "$key"
      frkey="$rt_f|$rt_receiver"
      if [ -z "${rt_decl[$frkey]:-}" ] && [ -z "${rt_recv_closed[$frkey]:-}" ]; then
        echo "ROUTING-UNDECLARED: $rt_f: receiver $rt_receiver has no declaration of the inherited finding" >&2
        fail=1
      fi
    done
    for key in "${!rt_decl[@]}"; do
      if [ -z "${rt_fr[$key]:-}" ]; then
        IFS='|' read -r rt_f rt_receiver <<< "$key"
        echo "ROUTING-DECLARATION-MISMATCH: $rt_f: declaration for $rt_receiver matches no routing record" >&2
        fail=1
      fi
    done
  fi
fi

# Stop-escalation gate (4.6): a spec's STOP-AND-REPORT trigger firing is an
# escalation to the owner; the only valid resolutions are an owner-signed spec
# amendment or recorded abandonment of the step. When an escalation scope file
# is given, every declared mandatory trigger must carry an exact owner-response
# record: bound to the same trigger, answered by the exact required owner, and
# explicitly classified as `amendment` or `abandonment`. Silence, inferred
# approval, and ordinary closure/completion never clear a trigger.
if [ -n "${8:-}" ]; then
  ES_SCOPE="$8"
  ES_RESPONSES="${9:-escalation-responses.tsv}"
  if [ ! -f "$ES_SCOPE" ]; then
    echo "ESCALATION-MISSING-SCOPE: no escalation scope file: $ES_SCOPE" >&2
    fail=1
  elif [ ! -f "$ES_RESPONSES" ]; then
    echo "ESCALATION-MISSING-RESPONSES: no escalation responses file: $ES_RESPONSES" >&2
    fail=1
  else
    # Authoritative scope: every declared mandatory trigger (trigger owner). A
    # malformed scope link is still recorded so its responses are not cascaded
    # into UNEXPECTED-RESPONSE; the malformed diagnostic already fails the run.
    declare -A es_scope=() es_scope_bad=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r es_trig es_owner <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$es_trig" ] || [ -z "$es_owner" ]; then
        echo "ESCALATION-MALFORMED: ${es_trig:-<no-trigger>}: escalation scope link must have two tab-separated non-empty fields (trigger owner)" >&2
        [ -n "$es_trig" ] && es_scope_bad["$es_trig"]=1
        fail=1
        continue
      fi
      key="$es_trig"
      if [ -n "${es_scope[$key]:-}" ]; then
        echo "ESCALATION-DUPLICATE: $es_trig: duplicate escalation scope link" >&2
        fail=1
        continue
      fi
      es_scope["$key"]="$es_owner"
    done < "$ES_SCOPE"

    # Owner evidence: one exact response record per declared trigger
    # (trigger owner response-kind evidence).
    declare -A es_resp=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r es_trig es_owner es_kind es_evidence <<< "$line"
      if [ "${#tabs}" -ne 3 ] || [ -z "$es_trig" ] || [ -z "$es_owner" ] || [ -z "$es_kind" ] || [ -z "$es_evidence" ]; then
        echo "ESCALATION-MALFORMED: ${es_trig:-<no-trigger>}: escalation response must have four tab-separated non-empty fields (trigger owner response-kind evidence)" >&2
        [ -n "$es_trig" ] && es_resp["$es_trig"]=1
        fail=1
        continue
      fi
      key="$es_trig"
      if [ -n "${es_resp[$key]:-}" ]; then
        echo "ESCALATION-DUPLICATE: $es_trig: duplicate escalation response" >&2
        fail=1
        continue
      fi
      es_resp["$key"]=1
      # A trigger whose scope link was malformed is not cascade-checked here:
      # ESCALATION-MALFORMED already failed the run for that line.
      [ -n "${es_scope_bad[$es_trig]:-}" ] && continue
      # Every response must correspond exactly to a declared scope trigger:
      # a rogue response cannot authorize a fabricated resolution.
      required="${es_scope[$key]:-}"
      if [ -z "$required" ]; then
        echo "ESCALATION-UNEXPECTED-RESPONSE: $es_trig: escalation response matches no declared mandatory trigger" >&2
        fail=1
        continue
      fi
      # The response must be answered by the exact required owner.
      if [ "$es_owner" != "$required" ]; then
        echo "ESCALATION-WRONG-OWNER: $es_trig: escalation response answered by $es_owner, required owner is $required" >&2
        fail=1
        continue
      fi
      # An ordinary closure/completion is never a response: the kind must be
      # exactly `amendment` or `abandonment`.
      if [ "$es_kind" != "amendment" ] && [ "$es_kind" != "abandonment" ]; then
        echo "ESCALATION-INVALID-RESPONSE: $es_trig: escalation response kind \"$es_kind\" is not valid (must be \"amendment\" or \"abandonment\")" >&2
        fail=1
      fi
    done < "$ES_RESPONSES"

    # The scope is authoritative: omitting a response cannot hide an
    # unresponded trigger.
    for es_trig in "${!es_scope[@]}"; do
      if [ -z "${es_resp[$es_trig]:-}" ]; then
        echo "ESCALATION-UNRESPONDED: $es_trig: declared mandatory trigger has no owner-response record" >&2
        fail=1
      fi
    done
  fi
fi

# Sibling-sweep gate (4.9a): a one-function fix claims it swept the fixed
# function's file for structurally identical siblings (pattern 2) and must name
# a reproducible sibling-search pattern and a recorded current hit count. When
# a sibling scope file is given, every in-scope claim must carry an evidence
# record whose pattern re-runs to the recorded count within the approved
# heuristic limit, and the search must be applicable: the declared scope must
# exist, contain the fixed function named in the scope link, and match at
# least one line. Comments and string literals are stripped before both the
# presence check and the pattern re-run, so a comment-only or string-only
# mention of a claimed function or pattern is never evidence of a real sibling
# search, and a pattern that re-runs to zero is rejected as a decoy before any
# recorded count is compared. The scope file is the authoritative enumeration
# and is never inferred from the records, so omitting a record cannot hide a
# claim, and no record can authorize a fabricated sweep.
if [ -n "${10:-}" ]; then
  SIB_SCOPE="${10}"
  SIB_RECORDS="${11:-sibling.tsv}"
  if [ ! -f "$SIB_SCOPE" ]; then
    echo "SIBLING-MISSING-SCOPE: no sibling scope file: $SIB_SCOPE" >&2
    fail=1
  elif [ ! -f "$SIB_RECORDS" ]; then
    echo "SIBLING-MISSING-RECORDS: no sibling records file: $SIB_RECORDS" >&2
    fail=1
  else
    # Authoritative scope: every in-scope one-function-fix claim (id function).
    # A malformed scope link is still recorded so its claim is not cascaded
    # into MISSING; the malformed diagnostic already fails the run.
    declare -A sib_fn=() sib_scope_seen=() sib_rec_seen=()
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r sb_id sb_fn <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$sb_id" ] || [ -z "$sb_fn" ]; then
        echo "SIBLING-MALFORMED: ${sb_id:-<no-id>}: sibling scope link must have two tab-separated non-empty fields (id function)" >&2
        [ -n "$sb_id" ] && sib_scope_seen["$sb_id"]=1
        fail=1
        continue
      fi
      if [ -n "${sib_scope_seen[$sb_id]:-}" ]; then
        echo "SIBLING-DUPLICATE: $sb_id: duplicate sibling scope link" >&2
        fail=1
        continue
      fi
      sib_scope_seen["$sb_id"]=1
      sib_fn["$sb_id"]="$sb_fn"
    done < "$SIB_SCOPE"

    # Claim evidence: one record per in-scope claim (id pattern scope count
    # limit). Every record must correspond exactly to a scope link - a rogue
    # record cannot authorize a fabricated sweep - and the recorded count must
    # be reproducible and within the approved heuristic limit. The record
    # structure is validated with awk, whose -F'\t' split preserves empty
    # fields (unlike `read`, which collapses consecutive tabs), so an empty
    # search-scope field is diagnosed distinctly from a truncated row.
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      nf="$(printf '%s\n' "$line" | awk -F'\t' '{print NF}')"
      empty_field="$(printf '%s\n' "$line" | awk -F'\t' '{ if ($3 == "") print "scope"; else if ($1 == "" || $2 == "" || $4 == "" || $5 == "") print "other" }')"
      if [ "$nf" -ne 5 ]; then
        sb_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
        echo "SIBLING-MALFORMED: ${sb_id:-<no-id>}: sibling record must have five tab-separated non-empty fields (id pattern scope count limit)" >&2
        [ -n "$sb_id" ] && sib_rec_seen["$sb_id"]=1
        fail=1
        continue
      fi
      if [ "$empty_field" = "scope" ]; then
        sb_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
        echo "SIBLING-OMITTED-SCOPE: ${sb_id:-<no-id>}: sibling record omits the search scope field (id pattern scope count limit)" >&2
        [ -n "$sb_id" ] && sib_rec_seen["$sb_id"]=1
        fail=1
        continue
      fi
      if [ "$empty_field" = "other" ]; then
        sb_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
        echo "SIBLING-MALFORMED: ${sb_id:-<no-id>}: sibling record must have five tab-separated non-empty fields (id pattern scope count limit)" >&2
        [ -n "$sb_id" ] && sib_rec_seen["$sb_id"]=1
        fail=1
        continue
      fi
      IFS=$'\t' read -r sb_id sb_pattern sb_scope sb_count sb_limit <<< "$line"
      if [ -n "${sib_rec_seen[$sb_id]:-}" ]; then
        echo "SIBLING-DUPLICATE: $sb_id: duplicate sibling record" >&2
        fail=1
        continue
      fi
      sib_rec_seen["$sb_id"]=1
      if ! [[ "$sb_count" =~ ^[0-9]+$ ]] || ! [[ "$sb_limit" =~ ^[0-9]+$ ]]; then
        echo "SIBLING-MALFORMED: $sb_id: count and limit must be non-negative integers" >&2
        fail=1
        continue
      fi
      # A rogue record that matches no in-scope claim cannot authorize a
      # fabricated sweep.
      if [ -z "${sib_scope_seen[$sb_id]:-}" ]; then
        echo "SIBLING-ROGUE: $sb_id: sibling record matches no in-scope claim" >&2
        fail=1
        continue
      fi
      # The search must be applicable: the declared scope must exist and must
      # contain the fixed function the scope link names. Comments and string
      # literals are stripped first, so a comment-only or string-only mention of
      # the function is never proof that the fix landed in real code.
      if [ ! -e "$sb_scope" ]; then
        echo "SIBLING-DECOY: $sb_id: declared search scope not found: $sb_scope" >&2
        fail=1
        continue
      fi
      if ! grep -qE "fn ${sib_fn[$sb_id]}" <(stripped_scope "$sb_scope") 2>/dev/null; then
        echo "SIBLING-DECOY: $sb_id: fixed function ${sib_fn[$sb_id]} is absent from the declared search scope $sb_scope" >&2
        fail=1
        continue
      fi
      # Re-run the sibling search exactly as recorded and compare the count.
      # Comments and string literals are stripped first, so a pattern whose only
      # matches live in a comment or string is not a real sibling search.
      actual="$(grep -E -- "$sb_pattern" <(stripped_scope "$sb_scope") 2>/dev/null | wc -l | tr -d ' ')"
      # A pattern that matches nothing is a decoy, not evidence of "no
      # siblings": a real sibling search over the fixed code matches at least
      # the fixed function itself. Checked before the count comparison, so a
      # recorded count that reproduces only over comment/string matches is a
      # decoy, never accepted as stale-but-consistent evidence.
      if [ "$actual" -eq 0 ]; then
        echo "SIBLING-DECOY: $sb_id: pattern \"$sb_pattern\" matches nothing in $sb_scope" >&2
        fail=1
        continue
      fi
      if [ "$actual" -ne "$sb_count" ]; then
        echo "SIBLING-STALE-COUNT: $sb_id: recorded hit count $sb_count does not match current hit count $actual for pattern \"$sb_pattern\" in $sb_scope" >&2
        fail=1
        continue
      fi
      # The hit count must stay within the approved heuristic limit.
      if [ "$actual" -gt "$sb_limit" ]; then
        echo "SIBLING-OVER-LIMIT: $sb_id: hit count $actual exceeds the approved heuristic limit $sb_limit" >&2
        fail=1
      fi
    done < "$SIB_RECORDS"

    # The scope is authoritative: omitting a claim's evidence record cannot
    # hide it.
    for sb_id in "${!sib_scope_seen[@]}"; do
      if [ -z "${sib_rec_seen[$sb_id]:-}" ]; then
        echo "SIBLING-MISSING: $sb_id: in-scope one-function-fix claim has no sibling record" >&2
        fail=1
      fi
    done
  fi
fi

# Exhaustive-match gate (4.9b): a fix that makes a `match` exhaustive without a
# wildcard arm (pattern 5 - the `_ => <default>` substitution) must name its
# scope and prove no wildcard match arm remains there, reproducibly. When an
# exhaustive scope file is given, every in-scope claim must carry an evidence
# record whose re-run reproduces both recorded counts, and the proof is
# non-vacuous only when the scope actually contains at least one `match`
# expression. The wildcard scan is a documented textual heuristic, not a Rust
# parse (see the "Exhaustive-match heuristic limits" paragraph in the header).
if [ -n "${12:-}" ]; then
  EXM_SCOPE="${12}"
  EXM_RECORDS="${13:-exhaustive-match.tsv}"
  EXM_MATCH_PATTERN='\bmatch\b'
  EXM_WILDCARD_PATTERN='^[[:space:]]*_([[:space:]]+@[[:space:]]*[A-Za-z_][A-Za-z0-9_]*|[[:space:]]+if[[:space:]].*)?[[:space:]]*=>'
  if [ ! -f "$EXM_SCOPE" ]; then
    echo "EXMATCH-MISSING-SCOPE: no exhaustive-match scope file: $EXM_SCOPE" >&2
    fail=1
  elif [ ! -f "$EXM_RECORDS" ]; then
    echo "EXMATCH-MISSING-RECORDS: no exhaustive-match records file: $EXM_RECORDS" >&2
    fail=1
  else
    # Authoritative scope: every in-scope exhaustive-match claim (id scope).
    # A malformed scope link is still recorded so its claim is not cascaded
    # into MISSING; the malformed diagnostic already fails the run.
    declare -A exm_scope=() exm_scope_seen=() exm_scope_bad=() exm_rec_seen=()
    exm_lines=0
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      exm_lines=$((exm_lines + 1))
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r ex_id ex_scope <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$ex_id" ] || [ -z "$ex_scope" ]; then
        echo "EXMATCH-MALFORMED: ${ex_id:-<no-id>}: exhaustive-match scope link must have two tab-separated non-empty fields (id scope)" >&2
        [ -n "$ex_id" ] && exm_scope_bad["$ex_id"]=1
        fail=1
        continue
      fi
      if [ -n "${exm_scope_seen[$ex_id]:-}" ]; then
        echo "EXMATCH-DUPLICATE: $ex_id: duplicate exhaustive-match scope link" >&2
        fail=1
        continue
      fi
      exm_scope_seen["$ex_id"]=1
      exm_scope["$ex_id"]="$ex_scope"
    done < "$EXM_SCOPE"

    # The authoritative source scope must be non-empty: an exhaustive-match
    # sign-off must enumerate a non-empty source scope, so an empty scope file
    # is rejected outright rather than silently accepted as proof.
    if [ "$exm_lines" -eq 0 ]; then
      echo "EXMATCH-EMPTY-SCOPE: $EXM_SCOPE: exhaustive-match scope is empty - no in-scope exhaustive-match claims are enumerated" >&2
      fail=1
    else
      # Claim evidence: one record per in-scope claim (id scope wildcards
      # matches). Every record must correspond exactly to a scope link - a rogue
      # record cannot authorize a fabricated sweep - and must scan exactly the
      # authoritative scope the link names, so a record pointing at a different
      # scope is a nearby scan, not evidence. The record structure is validated
      # with awk, whose -F'\t' split preserves empty fields (unlike `read`, which
      # collapses consecutive tabs), so an empty search-scope field is diagnosed
      # distinctly from a truncated row.
      while IFS= read -r line; do
        [ -n "$line" ] || continue
        nf="$(printf '%s\n' "$line" | awk -F'\t' '{print NF}')"
        empty_field="$(printf '%s\n' "$line" | awk -F'\t' '{ if ($2 == "") print "scope"; else if ($1 == "" || $3 == "" || $4 == "") print "other" }')"
        if [ "$nf" -ne 4 ]; then
          ex_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "EXMATCH-MALFORMED: ${ex_id:-<no-id>}: exhaustive-match record must have four tab-separated non-empty fields (id scope wildcards matches)" >&2
          [ -n "$ex_id" ] && exm_rec_seen["$ex_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "scope" ]; then
          ex_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "EXMATCH-OMITTED-SCOPE: ${ex_id:-<no-id>}: exhaustive-match record omits the search scope field (id scope wildcards matches)" >&2
          [ -n "$ex_id" ] && exm_rec_seen["$ex_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "other" ]; then
          ex_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "EXMATCH-MALFORMED: ${ex_id:-<no-id>}: exhaustive-match record must have four tab-separated non-empty fields (id scope wildcards matches)" >&2
          [ -n "$ex_id" ] && exm_rec_seen["$ex_id"]=1
          fail=1
          continue
        fi
        IFS=$'\t' read -r ex_id ex_scope ex_wild ex_matches <<< "$line"
        if [ -n "${exm_rec_seen[$ex_id]:-}" ]; then
          echo "EXMATCH-DUPLICATE: $ex_id: duplicate exhaustive-match record" >&2
          fail=1
          continue
        fi
        exm_rec_seen["$ex_id"]=1
        if ! [[ "$ex_wild" =~ ^[0-9]+$ ]] || ! [[ "$ex_matches" =~ ^[0-9]+$ ]]; then
          echo "EXMATCH-MALFORMED: $ex_id: wildcards and matches must be non-negative integers" >&2
          fail=1
          continue
        fi
        # A claim whose scope link was malformed is not cascade-checked here:
        # EXMATCH-MALFORMED already failed the run for that line.
        [ -n "${exm_scope_bad[$ex_id]:-}" ] && continue
        # A rogue record that matches no in-scope claim cannot authorize a
        # fabricated sweep.
        if [ -z "${exm_scope_seen[$ex_id]:-}" ]; then
          echo "EXMATCH-ROGUE: $ex_id: exhaustive-match record matches no in-scope claim" >&2
          fail=1
          continue
        fi
        # The record must scan exactly the authoritative scope: a record that
        # points at a different scope is a nearby scan, not evidence.
        if [ "$ex_scope" != "${exm_scope[$ex_id]}" ]; then
          echo "EXMATCH-DECOY: $ex_id: record scope $ex_scope does not match the authoritative scope ${exm_scope[$ex_id]}" >&2
          fail=1
          continue
        fi
        # The scan must be applicable: the declared scope must exist, and must
        # actually contain at least one `match` expression - otherwise "no
        # wildcard arm remains" is vacuous, not proof.
        if [ ! -e "$ex_scope" ]; then
          echo "EXMATCH-DECOY: $ex_id: declared scope not found: $ex_scope" >&2
          fail=1
          continue
        fi
        if [ "$ex_matches" -eq 0 ]; then
          echo "EXMATCH-DECOY: $ex_id: scope $ex_scope contains no match expression, so the no-wildcard claim is vacuous" >&2
          fail=1
          continue
        fi
        # Re-run the match-presence heuristic exactly as the guard defines it
        # and compare the count: stale evidence (the code has changed) is
        # rejected.
        actual_m="$(grep -rE "$EXM_MATCH_PATTERN" "$ex_scope" 2>/dev/null | wc -l | tr -d ' ')"
        if [ "$actual_m" -ne "$ex_matches" ]; then
          echo "EXMATCH-STALE: $ex_id: recorded match count $ex_matches does not match current count $actual_m in $ex_scope" >&2
          fail=1
          continue
        fi
        # Re-run the wildcard-arm heuristic: the proof of "no wildcard arm
        # remains" is exactly this count reproducing at 0.
        actual_w="$(grep -rE "$EXM_WILDCARD_PATTERN" "$ex_scope" 2>/dev/null | wc -l | tr -d ' ')"
        if [ "$actual_w" -gt 0 ]; then
          echo "EXMATCH-WILDCARD-REMAINS: $ex_id: a wildcard match arm remains in $ex_scope (current count $actual_w > 0)" >&2
          fail=1
          continue
        fi
        if [ "$actual_w" -ne "$ex_wild" ]; then
          echo "EXMATCH-STALE: $ex_id: recorded wildcard count $ex_wild does not match current count $actual_w in $ex_scope" >&2
          fail=1
        fi
      done < "$EXM_RECORDS"

      # The scope is authoritative: omitting a claim's evidence record cannot
      # hide it.
      for ex_id in "${!exm_scope_seen[@]}"; do
        if [ -z "${exm_rec_seen[$ex_id]:-}" ]; then
          echo "EXMATCH-MISSING: $ex_id: in-scope exhaustive-match claim has no evidence record" >&2
          fail=1
        fi
      done
    fi
  fi
fi

# Dead-code sweep gate (4.9c): a fix that removes or documents dead code must
# sweep the approved universe for remaining `#[allow(dead_code)]` suppressions
# and prove none remains, reproducibly. When a dead-code scope file is given,
# every in-scope claim must carry an evidence record whose re-run reproduces all
# three recorded counts, the allowance and suppression-variant counts must be 0,
# and the universe must actually contain at least one `dead_code` mention - so a
# vacuous or nearby scan is never proof. The sweep is a documented textual
# heuristic, not a Rust parse (see the "Dead-code sweep heuristic limits"
# paragraph in the header).
if [ -n "${14:-}" ]; then
  DCSWEEP_SCOPE="${14}"
  DCSWEEP_RECORDS="${15:-deadcode.tsv}"
  DCSWEEP_MATCH_PATTERN='\bdead_code\b'
  DCSWEEP_ALLOWANCE_PATTERN='allow[[:space:]]*\([^)]*dead_code[^)]*\)'
  DCSWEEP_VARIANT_PATTERN='allow[[:space:]]*\([^)]*\b(unused|warnings)\b[^)]*\)|expect[[:space:]]*\([^)]*\b(dead_code|unused|warnings)\b[^)]*\)'
  if [ ! -f "$DCSWEEP_SCOPE" ]; then
    echo "DCSWEEP-MISSING-SCOPE: no dead-code sweep scope file: $DCSWEEP_SCOPE" >&2
    fail=1
  elif [ ! -f "$DCSWEEP_RECORDS" ]; then
    echo "DCSWEEP-MISSING-RECORDS: no dead-code sweep records file: $DCSWEEP_RECORDS" >&2
    fail=1
  else
    # Authoritative scope: every in-scope dead-code sweep claim (id universe).
    # A malformed scope link is still recorded so its claim is not cascaded into
    # MISSING; the malformed diagnostic already fails the run.
    declare -A dc_scope=() dc_scope_seen=() dc_scope_bad=() dc_rec_seen=()
    dc_lines=0
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      dc_lines=$((dc_lines + 1))
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r dc_id dc_universe <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$dc_id" ] || [ -z "$dc_universe" ]; then
        echo "DCSWEEP-MALFORMED: ${dc_id:-<no-id>}: dead-code scope link must have two tab-separated non-empty fields (id universe)" >&2
        [ -n "$dc_id" ] && dc_scope_bad["$dc_id"]=1
        fail=1
        continue
      fi
      if [ -n "${dc_scope_seen[$dc_id]:-}" ]; then
        echo "DCSWEEP-DUPLICATE: $dc_id: duplicate dead-code scope link" >&2
        fail=1
        continue
      fi
      dc_scope_seen["$dc_id"]=1
      dc_scope["$dc_id"]="$dc_universe"
    done < "$DCSWEEP_SCOPE"

    # The authoritative universe must be non-empty: a dead-code sweep sign-off
    # must enumerate a non-empty source scope, so an empty scope file is
    # rejected outright rather than silently accepted as proof.
    if [ "$dc_lines" -eq 0 ]; then
      echo "DCSWEEP-EMPTY-SCOPE: $DCSWEEP_SCOPE: dead-code sweep scope is empty - no in-scope dead-code sweep claims are enumerated" >&2
      fail=1
    else
      # Claim evidence: one record per in-scope claim (id universe allowances
      # variants matches). Every record must correspond exactly to a scope link -
      # a rogue record cannot authorize a fabricated sweep - and must scan
      # exactly the authoritative universe the link names, so a record pointing
      # at a different path is a nearby scan, not evidence. The record structure
      # is validated with awk, whose -F'\t' split preserves empty fields (unlike
      # `read`, which collapses consecutive tabs), so an empty universe field is
      # diagnosed distinctly from a truncated row.
      while IFS= read -r line; do
        [ -n "$line" ] || continue
        nf="$(printf '%s\n' "$line" | awk -F'\t' '{print NF}')"
        empty_field="$(printf '%s\n' "$line" | awk -F'\t' '{ if ($2 == "") print "universe"; else if ($1 == "" || $3 == "" || $4 == "" || $5 == "") print "other" }')"
        if [ "$nf" -ne 5 ]; then
          dc_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "DCSWEEP-MALFORMED: ${dc_id:-<no-id>}: dead-code record must have five tab-separated non-empty fields (id universe allowances variants matches)" >&2
          [ -n "$dc_id" ] && dc_rec_seen["$dc_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "universe" ]; then
          dc_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "DCSWEEP-OMITTED-SCOPE: ${dc_id:-<no-id>}: dead-code record omits the sweep universe field (id universe allowances variants matches)" >&2
          [ -n "$dc_id" ] && dc_rec_seen["$dc_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "other" ]; then
          dc_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "DCSWEEP-MALFORMED: ${dc_id:-<no-id>}: dead-code record must have five tab-separated non-empty fields (id universe allowances variants matches)" >&2
          [ -n "$dc_id" ] && dc_rec_seen["$dc_id"]=1
          fail=1
          continue
        fi
        IFS=$'\t' read -r dc_id dc_universe dc_allow dc_variant dc_matches <<< "$line"
        if [ -n "${dc_rec_seen[$dc_id]:-}" ]; then
          echo "DCSWEEP-DUPLICATE: $dc_id: duplicate dead-code record" >&2
          fail=1
          continue
        fi
        dc_rec_seen["$dc_id"]=1
        if ! [[ "$dc_allow" =~ ^[0-9]+$ ]] || ! [[ "$dc_variant" =~ ^[0-9]+$ ]] || ! [[ "$dc_matches" =~ ^[0-9]+$ ]]; then
          echo "DCSWEEP-MALFORMED: $dc_id: allowances, variants and matches must be non-negative integers" >&2
          fail=1
          continue
        fi
        # A claim whose scope link was malformed is not cascade-checked here:
        # DCSWEEP-MALFORMED already failed the run for that line.
        [ -n "${dc_scope_bad[$dc_id]:-}" ] && continue
        # A rogue record that matches no in-scope claim cannot authorize a
        # fabricated sweep.
        if [ -z "${dc_scope_seen[$dc_id]:-}" ]; then
          echo "DCSWEEP-ROGUE: $dc_id: dead-code record matches no in-scope claim" >&2
          fail=1
          continue
        fi
        # The record must scan exactly the authoritative universe: a record that
        # points at a different path is a nearby scan, not evidence.
        if [ "$dc_universe" != "${dc_scope[$dc_id]}" ]; then
          echo "DCSWEEP-DECOY: $dc_id: record universe $dc_universe does not match the authoritative universe ${dc_scope[$dc_id]}" >&2
          fail=1
          continue
        fi
        # The sweep must be applicable: the declared universe must exist, and
        # must actually contain at least one `dead_code` mention - otherwise "no
        # allowance remains" is vacuous, not proof.
        if [ ! -e "$dc_universe" ]; then
          echo "DCSWEEP-DECOY: $dc_id: declared universe not found: $dc_universe" >&2
          fail=1
          continue
        fi
        if [ "$dc_matches" -eq 0 ]; then
          echo "DCSWEEP-DECOY: $dc_id: universe $dc_universe contains no dead_code mention, so the no-allowance claim is vacuous" >&2
          fail=1
          continue
        fi
        # Re-run the dead-code presence heuristic exactly as the guard defines
        # it and compare the count: stale evidence (the code has changed) is
        # rejected.
        actual_m="$(grep -rE "$DCSWEEP_MATCH_PATTERN" "$dc_universe" 2>/dev/null | wc -l | tr -d ' ')"
        if [ "$actual_m" -ne "$dc_matches" ]; then
          echo "DCSWEEP-STALE: $dc_id: recorded match count $dc_matches does not match current count $actual_m in $dc_universe" >&2
          fail=1
          continue
        fi
        # Re-run the dead-code allowance heuristic: the proof of "no
        # #[allow(dead_code)] remains" is exactly this count reproducing at 0.
        # The allowance pattern also matches bounded same-line whitespace
        # spellings such as `#[allow (dead_code)]`, so the spaced form cannot
        # bypass the sweep.
        actual_a="$(grep -rE "$DCSWEEP_ALLOWANCE_PATTERN" "$dc_universe" 2>/dev/null | wc -l | tr -d ' ')"
        if [ "$actual_a" -gt 0 ]; then
          echo "DCSWEEP-ALLOWANCE-REMAINS: $dc_id: an #[allow(dead_code)] remains in $dc_universe (current count $actual_a > 0)" >&2
          fail=1
          continue
        fi
        if [ "$actual_a" -ne "$dc_allow" ]; then
          echo "DCSWEEP-STALE: $dc_id: recorded allowance count $dc_allow does not match current count $actual_a in $dc_universe" >&2
          fail=1
          continue
        fi
        # Re-run the suppression-variant heuristic: any variant the record fails
        # to report is an unreported suppression, and none may remain.
        actual_v="$(grep -rE "$DCSWEEP_VARIANT_PATTERN" "$dc_universe" 2>/dev/null | wc -l | tr -d ' ')"
        if [ "$actual_v" -gt 0 ]; then
          echo "DCSWEEP-VARIANT-REMAINS: $dc_id: a dead-code suppression variant remains in $dc_universe (current count $actual_v > 0)" >&2
          fail=1
          continue
        fi
        if [ "$actual_v" -ne "$dc_variant" ]; then
          echo "DCSWEEP-STALE: $dc_id: recorded variant count $dc_variant does not match current count $actual_v in $dc_universe" >&2
          fail=1
        fi
      done < "$DCSWEEP_RECORDS"

      # The scope is authoritative: omitting a claim's evidence record cannot
      # hide it.
      for dc_id in "${!dc_scope_seen[@]}"; do
        if [ -z "${dc_rec_seen[$dc_id]:-}" ]; then
          echo "DCSWEEP-MISSING: $dc_id: in-scope dead-code sweep claim has no evidence record" >&2
          fail=1
        fi
      done
    fi
  fi
fi

# Log-layer proof gate (4.9d): a logging-related claim must prove direct
# `Log::public` content coverage at the authoritative approved log-layer source
# scope, reproducibly. When a log-layer scope file is given, every in-scope
# claim must carry an evidence record whose approved `Log::public` pattern
# re-runs to the recorded count within the declared scope, and the scope must
# actually contain at least one direct `Log::public` invocation once comments
# and string literals are stripped - so a comment, string, wrapper, or
# lower/upper-layer mention is never mistaken for direct content proof. The
# proof is a documented textual heuristic, not a Rust parse or a semantic proof
# of the log content (see the "Log-layer heuristic limits" paragraph in the
# header).
if [ -n "${16:-}" ]; then
  LOGL_SCOPE="${16}"
  LOGL_RECORDS="${17:-loglayer.tsv}"
  if [ ! -f "$LOGL_SCOPE" ]; then
    echo "LOGLAYER-MISSING-SCOPE: no log-layer scope file: $LOGL_SCOPE" >&2
    fail=1
  elif [ ! -f "$LOGL_RECORDS" ]; then
    echo "LOGLAYER-MISSING-RECORDS: no log-layer records file: $LOGL_RECORDS" >&2
    fail=1
  else
    # Authoritative scope: every in-scope logging-related claim (id scope). A
    # malformed scope link is still recorded so its claim is not cascaded into
    # MISSING; the malformed diagnostic already fails the run.
    declare -A logl_scope=() logl_scope_seen=() logl_scope_bad=() logl_rec_seen=()
    logl_lines=0
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      logl_lines=$((logl_lines + 1))
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r lo_id lo_scope <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$lo_id" ] || [ -z "$lo_scope" ]; then
        echo "LOGLAYER-MALFORMED: ${lo_id:-<no-id>}: log-layer scope link must have two tab-separated non-empty fields (id scope)" >&2
        [ -n "$lo_id" ] && logl_scope_bad["$lo_id"]=1
        fail=1
        continue
      fi
      if [ -n "${logl_scope_seen[$lo_id]:-}" ]; then
        echo "LOGLAYER-DUPLICATE: $lo_id: duplicate log-layer scope link" >&2
        fail=1
        continue
      fi
      logl_scope_seen["$lo_id"]=1
      logl_scope["$lo_id"]="$lo_scope"
    done < "$LOGL_SCOPE"

    # The authoritative scope must be non-empty: a log-layer sign-off must
    # enumerate a non-empty source scope, so an empty scope file is rejected
    # outright rather than silently accepted as proof.
    if [ "$logl_lines" -eq 0 ]; then
      echo "LOGLAYER-EMPTY-SCOPE: $LOGL_SCOPE: log-layer scope is empty - no in-scope logging-related claims are enumerated" >&2
      fail=1
    else
      # Claim evidence: one record per in-scope claim (id scope pattern count).
      # Every record must correspond exactly to a scope link - a rogue record
      # cannot authorize a fabricated proof - and must scan exactly the
      # authoritative scope the link names, so a record pointing at a wrapper,
      # caller, or lower/upper-layer file is a nearby scan, not evidence. The
      # record structure is validated with awk, whose -F'\t' split preserves
      # empty fields (unlike `read`, which collapses consecutive tabs), so an
      # empty log-layer-scope field is diagnosed distinctly from a truncated
      # row.
      while IFS= read -r line; do
        [ -n "$line" ] || continue
        nf="$(printf '%s\n' "$line" | awk -F'\t' '{print NF}')"
        empty_field="$(printf '%s\n' "$line" | awk -F'\t' '{ if ($2 == "") print "scope"; else if ($1 == "" || $3 == "" || $4 == "") print "other" }')"
        if [ "$nf" -ne 4 ]; then
          lo_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "LOGLAYER-MALFORMED: ${lo_id:-<no-id>}: log-layer record must have four tab-separated non-empty fields (id scope pattern count)" >&2
          [ -n "$lo_id" ] && logl_rec_seen["$lo_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "scope" ]; then
          lo_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "LOGLAYER-OMITTED-SCOPE: ${lo_id:-<no-id>}: log-layer record omits the log-layer scope field (id scope pattern count)" >&2
          [ -n "$lo_id" ] && logl_rec_seen["$lo_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "other" ]; then
          lo_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
          echo "LOGLAYER-MALFORMED: ${lo_id:-<no-id>}: log-layer record must have four tab-separated non-empty fields (id scope pattern count)" >&2
          [ -n "$lo_id" ] && logl_rec_seen["$lo_id"]=1
          fail=1
          continue
        fi
        IFS=$'\t' read -r lo_id lo_scope lo_pattern lo_count <<< "$line"
        if [ -n "${logl_rec_seen[$lo_id]:-}" ]; then
          echo "LOGLAYER-DUPLICATE: $lo_id: duplicate log-layer record" >&2
          fail=1
          continue
        fi
        logl_rec_seen["$lo_id"]=1
        if ! [[ "$lo_count" =~ ^[0-9]+$ ]]; then
          echo "LOGLAYER-MALFORMED: $lo_id: count must be a non-negative integer" >&2
          fail=1
          continue
        fi
        # A claim whose scope link was malformed is not cascade-checked here:
        # LOGLAYER-MALFORMED already failed the run for that line.
        [ -n "${logl_scope_bad[$lo_id]:-}" ] && continue
        # A rogue record that matches no in-scope claim cannot authorize a
        # fabricated proof.
        if [ -z "${logl_scope_seen[$lo_id]:-}" ]; then
          echo "LOGLAYER-ROGUE: $lo_id: log-layer record matches no in-scope claim" >&2
          fail=1
          continue
        fi
        # The record must scan exactly the authoritative log-layer scope: a
        # record that points at a wrapper, caller, or lower/upper-layer file is
        # a nearby scan, not evidence for this claim.
        if [ "$lo_scope" != "${logl_scope[$lo_id]}" ]; then
          echo "LOGLAYER-DECOY: $lo_id: record scope $lo_scope does not match the authoritative scope ${logl_scope[$lo_id]}" >&2
          fail=1
          continue
        fi
        if [ ! -e "$lo_scope" ]; then
          echo "LOGLAYER-DECOY: $lo_id: declared scope not found: $lo_scope" >&2
          fail=1
          continue
        fi
        # The proof must be direct, and the recorded pattern/count must be
        # demonstrably tied to the direct `Log::public` invocations - not merely
        # coexist with one. The direct-invocation heuristic re-runs the count
        # of comment/string-stripped `Log::public(` calls, and the approved
        # pattern is re-run over the same stripped content, so both must
        # reproduce the recorded count on exactly the same source lines: the
        # two scans must agree on the set of `file:line` identities, not just
        # on the count.
        direct="$(count_direct_log_public "$lo_scope")"
        if [ "$direct" -eq 0 ]; then
          # A scope whose references live only in comments, strings, a
          # wrapper, or a lower/upper-layer file is not direct content proof.
          echo "LOGLAYER-DECOY: $lo_id: scope $lo_scope contains no direct Log::public invocation, so the coverage claim is vacuous" >&2
          fail=1
          continue
        fi
        if [ "$direct" -ne "$lo_count" ]; then
          # The recorded count does not reproduce at the direct layer. When
          # the pattern's raw matches also diverge from the direct count, the
          # record is a mixed-layer decoy (the count includes comment/string
          # matches); only when they coincide is the divergence plain stale
          # evidence of code that changed.
          raw="$(grep -rE "$lo_pattern" "$lo_scope" 2>/dev/null | wc -l | tr -d ' ')"
          if [ "$raw" -eq "$direct" ]; then
            echo "LOGLAYER-STALE: $lo_id: recorded hit count $lo_count does not match current hit count $direct for pattern \"$lo_pattern\" in $lo_scope" >&2
          else
            echo "LOGLAYER-DECOY: $lo_id: pattern \"$lo_pattern\" count $lo_count is not tied to the direct Log::public invocations in $lo_scope" >&2
          fi
          fail=1
          continue
        fi
        # The recorded pattern must reproduce the recorded count on exactly
        # the same comment/string-stripped source lines as the direct
        # invocations - an equal count on different lines is a decoy, never
        # proof.
        direct_lines="$(loglayer_direct_lines "$lo_scope" | sort -u)"
        stripped="$(loglayer_stripped_lines "$lo_scope" "$lo_pattern" | sort -u)"
        if [ -n "$stripped" ]; then
          stripped_count="$(printf '%s\n' "$stripped" | wc -l | tr -d ' ')"
        else
          stripped_count=0
        fi
        if [ "$stripped_count" -ne "$lo_count" ]; then
          # The pattern's own stripped matches diverge from the direct
          # invocations, so an arbitrary pattern that happens to match an
          # unrelated comment/string while a separate direct call satisfies the
          # direct count is rejected.
          echo "LOGLAYER-DECOY: $lo_id: pattern \"$lo_pattern\" count $lo_count is not tied to the direct Log::public invocations in $lo_scope" >&2
          fail=1
        elif [ "$stripped" != "$direct_lines" ]; then
          # The counts coincide but the pattern matches different source lines
          # than the direct invocations: the recorded pattern/count is not the
          # direct content proof it claims.
          echo "LOGLAYER-DECOY: $lo_id: pattern \"$lo_pattern\" count $lo_count reproduces on different source lines than the direct Log::public invocations in $lo_scope" >&2
          fail=1
        fi
      done < "$LOGL_RECORDS"

      # The scope is authoritative: omitting a claim's evidence record cannot
      # hide it.
      for lo_id in "${!logl_scope_seen[@]}"; do
        if [ -z "${logl_rec_seen[$lo_id]:-}" ]; then
          echo "LOGLAYER-MISSING: $lo_id: in-scope logging-related claim has no evidence record" >&2
          fail=1
        fi
      done
    fi
  fi
fi

# Every-item checklist gate (4.9e): a broad sign-off claim equivalent to
# "every X" requires an authoritative, reproducible, non-empty item universe
# and exactly one explicit per-item checklist record per universe item. When
# an every-item scope file is given, every in-scope claim must carry per-item
# checklist records whose items cover the authoritative universe one-to-one,
# and the recorded item count must reproduce. The universe enumeration is a
# documented textual heuristic (see the "Every-item checklist heuristic
# limits" paragraph in the header): the universe is a file whose distinct
# non-blank lines are the items, and it proves only enumerated input coverage,
# never that any per-item work is semantically complete.
if [ -n "${18:-}" ]; then
  EV_SCOPE="${18}"
  EV_RECORDS="${19:-every.tsv}"
  if [ ! -f "$EV_SCOPE" ]; then
    echo "EVERY-MISSING-SCOPE: no every-item scope file: $EV_SCOPE" >&2
    fail=1
  elif [ ! -f "$EV_RECORDS" ]; then
    echo "EVERY-MISSING-RECORDS: no every-item checklist records file: $EV_RECORDS" >&2
    fail=1
  else
    # Authoritative scope: every in-scope "every X" claim (id universe). A
    # malformed scope link is still recorded so its claim is not cascaded into
    # MISSING; the malformed diagnostic already fails the run.
    declare -A ev_scope=() ev_scope_seen=() ev_scope_bad=() ev_rec_seen=() \
      ev_item_seen=() ev_failed=() ev_bad=() ev_claim_items=() \
      ev_univ_count=() ev_univ_items=() sign_ids=()
    ev_lines=0
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      ev_lines=$((ev_lines + 1))
      tabs="${line//[^$'\t']/}"
      IFS=$'\t' read -r ev_id ev_universe <<< "$line"
      if [ "${#tabs}" -ne 1 ] || [ -z "$ev_id" ] || [ -z "$ev_universe" ]; then
        echo "EVERY-MALFORMED: ${ev_id:-<no-id>}: every-item scope link must have two tab-separated non-empty fields (id universe)" >&2
        [ -n "$ev_id" ] && ev_scope_bad["$ev_id"]=1
        fail=1
        continue
      fi
      if [ -n "${ev_scope_seen[$ev_id]:-}" ]; then
        echo "EVERY-DUPLICATE: $ev_id: duplicate every-item scope link" >&2
        fail=1
        continue
      fi
      ev_scope_seen["$ev_id"]=1
      ev_scope["$ev_id"]="$ev_universe"
    done < "$EV_SCOPE"

    # The authoritative claim enumeration must be non-empty: an every-item
    # sign-off must enumerate a non-empty source scope, so an empty scope file
    # is rejected outright rather than silently accepted as proof.
    if [ "$ev_lines" -eq 0 ]; then
      echo "EVERY-EMPTY-SCOPE: $EV_SCOPE: every-item scope is empty - no in-scope every-item claims are enumerated" >&2
      fail=1
    else
      # Every in-scope claim must be linked to an actual sign-off record in
      # the RECORDS input, by claim ID: an "every X" scope claim with no
      # linked sign-off record is an unlinked claim, not evidence. An
      # unlinked claim is still recorded so its checklist records are not
      # cascaded into further diagnostics.
      while IFS=$'\t' read -r sign_id sign_rest; do
        [ -n "$sign_id" ] || continue
        sign_ids["$sign_id"]=1
      done < "$RECORDS"
      for ev_id in "${!ev_scope[@]}"; do
        if [ -z "${sign_ids[$ev_id]:-}" ]; then
          echo "EVERY-UNLINKED: $ev_id: every-item scope claim is not linked to a sign-off record in $RECORDS" >&2
          ev_bad["$ev_id"]=1
          fail=1
        fi
      done

      # The authoritative universe is enumerated first, independently of the
      # checklist records: a universe that is missing or names no items proves
      # nothing, so it is rejected outright and its claim's records are not
      # cascaded into further diagnostics.
      for ev_id in "${!ev_scope[@]}"; do
        ev_universe="${ev_scope[$ev_id]}"
        if [ ! -f "$ev_universe" ]; then
          echo "EVERY-DECOY: $ev_id: declared universe not found: $ev_universe" >&2
          ev_bad["$ev_id"]=1
          fail=1
          continue
        fi
        if [ -z "${ev_univ_items[$ev_universe]:-}" ]; then
          ev_univ_items["$ev_universe"]="$(grep -E '[^[:space:]]' "$ev_universe" 2>/dev/null | sort -u)"
          ev_univ_count["$ev_universe"]="$(printf '%s\n' "${ev_univ_items[$ev_universe]}" | grep -c . | tr -d ' ')"
        fi
        if [ "${ev_univ_count[$ev_universe]:-0}" -eq 0 ]; then
          echo "EVERY-EMPTY-UNIVERSE: $ev_universe: the authoritative universe names no items (empty universe)" >&2
          ev_bad["$ev_id"]=1
          fail=1
          continue
        fi
      done

      # Claim evidence: one per-item checklist record per universe item (id
      # universe count item). Every record must correspond exactly to a scope
      # link - a rogue record cannot authorize a fabricated checklist - and
      # must scan exactly the authoritative universe, so a record pointing at
      # a different list is a nearby scan, not evidence. The record structure
      # is validated with awk, whose -F'\t' split preserves empty fields
      # (unlike `read`, which collapses consecutive tabs), so an empty item
      # universe field is diagnosed distinctly from a truncated row.
      while IFS= read -r line; do
        [ -n "$line" ] || continue
        ev_id="$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')"
        # Records of a claim whose authoritative universe is missing or empty
        # are not cascade-checked here: the universe diagnostic already failed
        # the run.
        [ -n "${ev_bad[$ev_id]:-}" ] && continue
        nf="$(printf '%s\n' "$line" | awk -F'\t' '{print NF}')"
        empty_field="$(printf '%s\n' "$line" | awk -F'\t' '{ if ($2 == "") print "universe"; else if ($1 == "" || $3 == "" || $4 == "") print "other" }')"
        if [ "$nf" -ne 4 ]; then
          echo "EVERY-MALFORMED: ${ev_id:-<no-id>}: every-item record must have four tab-separated non-empty fields (id universe count item)" >&2
          [ -n "$ev_id" ] && ev_rec_seen["$ev_id"]=1
          [ -n "$ev_id" ] && ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "universe" ]; then
          echo "EVERY-OMITTED-UNIVERSE: ${ev_id:-<no-id>}: every-item record omits the item universe field (id universe count item)" >&2
          [ -n "$ev_id" ] && ev_rec_seen["$ev_id"]=1
          [ -n "$ev_id" ] && ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        if [ "$empty_field" = "other" ]; then
          echo "EVERY-MALFORMED: ${ev_id:-<no-id>}: every-item record must have four tab-separated non-empty fields (id universe count item)" >&2
          [ -n "$ev_id" ] && ev_rec_seen["$ev_id"]=1
          [ -n "$ev_id" ] && ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        IFS=$'\t' read -r ev_id ev_universe ev_rec_count ev_item <<< "$line"
        rec_key="$ev_id|$ev_item"
        if [ -n "${ev_item_seen[$rec_key]:-}" ]; then
          echo "EVERY-DUPLICATE: $ev_id: duplicate every-item checklist record for item $ev_item" >&2
          ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        ev_item_seen["$rec_key"]=1
        ev_rec_seen["$ev_id"]=1
        if ! [[ "$ev_rec_count" =~ ^[0-9]+$ ]]; then
          echo "EVERY-MALFORMED: $ev_id: count must be a non-negative integer" >&2
          ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        # A claim whose scope link was malformed is not cascade-checked here:
        # EVERY-MALFORMED already failed the run for that line.
        [ -n "${ev_scope_bad[$ev_id]:-}" ] && continue
        # A rogue record that matches no in-scope claim cannot authorize a
        # fabricated checklist.
        if [ -z "${ev_scope_seen[$ev_id]:-}" ]; then
          echo "EVERY-ROGUE: $ev_id: every-item checklist record matches no in-scope claim" >&2
          ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        # The record must scan exactly the authoritative universe: a record
        # that points at a different list is a nearby scan, not evidence.
        if [ "$ev_universe" != "${ev_scope[$ev_id]}" ]; then
          echo "EVERY-DECOY: $ev_id: record universe $ev_universe does not match the authoritative universe ${ev_scope[$ev_id]}" >&2
          ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        # A checklist record naming an item outside the authoritative universe
        # is a rogue extra, never evidence of coverage.
        if ! printf '%s\n' "${ev_univ_items[$ev_universe]:-}" | grep -qxF -- "$ev_item"; then
          echo "EVERY-ROGUE-EXTRA: $ev_id: checklist record names item $ev_item that is not in the authoritative universe $ev_universe" >&2
          ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        # Every accepted checklist record must reproduce the current universe
        # item count: a stale count on any one record, not just the first,
        # fails the claim.
        if [ "$ev_rec_count" != "${ev_univ_count[$ev_universe]:-}" ]; then
          echo "EVERY-STALE-COUNT: $ev_id: recorded item count $ev_rec_count does not match current count ${ev_univ_count[$ev_universe]:-} in $ev_universe" >&2
          ev_failed["$ev_id"]=1
          fail=1
          continue
        fi
        ev_claim_items["$ev_id"]="${ev_claim_items[$ev_id]:-}"$'\n'"$ev_item"
      done < "$EV_RECORDS"

      # The scope is authoritative: omitting a claim's checklist records cannot
      # hide an uncovered universe, and a claim's coverage must reproduce.
      for ev_id in "${!ev_scope_seen[@]}"; do
        # A malformed scope link, a missing/empty universe, or a record-level
        # diagnostic already failed the run for this claim.
        [ -n "${ev_scope_bad[$ev_id]:-}" ] && continue
        [ -n "${ev_bad[$ev_id]:-}" ] && continue
        [ -n "${ev_failed[$ev_id]:-}" ] && continue
        if [ -z "${ev_rec_seen[$ev_id]:-}" ]; then
          echo "EVERY-MISSING: $ev_id: in-scope every-item claim has no checklist records" >&2
          fail=1
          continue
        fi
        # Every universe item must carry exactly one checklist record.
        ev_universe="${ev_scope[$ev_id]}"
        while IFS= read -r item; do
          [ -n "$item" ] || continue
          if ! printf '%s\n' "${ev_claim_items[$ev_id]:-}" | grep -qxF -- "$item"; then
            echo "EVERY-OMITTED-ITEM: $ev_id: universe item $item has no per-item checklist record" >&2
            fail=1
          fi
        done < <(printf '%s\n' "${ev_univ_items[$ev_universe]}")
      done
    fi
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "check-four-tooth: FAIL" >&2
  exit 1
fi
msg="every record satisfies all four teeth"
[ -n "${2:-}" ] && msg="$msg and every in-scope work package has provenance"
[ -n "${4:-}" ] && msg="$msg and every expected routing link is routed, declared, and never closed by its sender"
[ -n "${8:-}" ] && msg="$msg and every declared mandatory trigger has an exact owner-response"
[ -n "${10:-}" ] && msg="$msg and every in-scope one-function-fix claim names a reproducible sibling-search pattern with a current hit count within its approved heuristic limit"
[ -n "${12:-}" ] && msg="$msg and every in-scope exhaustive-match claim proves no wildcard match arm remains in its scope"
[ -n "${14:-}" ] && msg="$msg and every in-scope dead-code sweep claim proves no #[allow(dead_code)] (or spaced spelling such as #[allow (dead_code)]) or suppression variant remains in its universe"
[ -n "${16:-}" ] && msg="$msg and every in-scope logging-related claim proves direct Log::public content coverage at its authoritative log-layer scope"
[ -n "${18:-}" ] && msg="$msg and every in-scope every-item claim enumerates exactly one per-item checklist record per authoritative universe item"
echo "check-four-tooth: OK ($msg)"
