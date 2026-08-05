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
# [ESCALATION-RESPONSES]]]]]]]]] - RECORDS defaults to signoffs.tsv, SCOPE
# defaults to none (4.3 gate skipped), PROVENANCE defaults to wp-provenance.tsv,
# ROUTING-SCOPE defaults to none (4.4 gate skipped), ROUTING-RECORDS to
# routing.tsv, ROUTING-DECLARATIONS to routing-declarations.tsv, ROUTING-CLOSURES
# to routing-closures.tsv, ESCALATION-SCOPE defaults to none (4.6 gate skipped)
# and ESCALATION-RESPONSES to escalation-responses.tsv. Uses only standard
# Bash/GNU utilities (grep, sed, awk).

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

if [ "$fail" -ne 0 ]; then
  echo "check-four-tooth: FAIL" >&2
  exit 1
fi
msg="every record satisfies all four teeth"
[ -n "${2:-}" ] && msg="$msg and every in-scope work package has provenance"
[ -n "${4:-}" ] && msg="$msg and every expected routing link is routed, declared, and never closed by its sender"
[ -n "${8:-}" ] && msg="$msg and every declared mandatory trigger has an exact owner-response"
echo "check-four-tooth: OK ($msg)"
