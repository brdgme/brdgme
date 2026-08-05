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
# WP diagnostics (the WP ID is interpolated):
#   WP-UNACCOUNTED: <wp>: in-scope work package has no provenance record
#   WP-DUPLICATE:   <wp>: duplicate provenance record
#   WP-NO-PROVENANCE: <wp>: completed work package has neither specification
#                     nor checklist evidence
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
# Usage: check-four-tooth.sh [RECORDS [SCOPE [PROVENANCE]]] - RECORDS defaults
# to signoffs.tsv, SCOPE defaults to none (4.3 gate skipped) and PROVENANCE
# defaults to wp-provenance.tsv. Uses only standard Bash/GNU utilities (grep,
# sed, awk).

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
    while IFS=$'\t' read -r wp p_spec p_chk p_done; do
      [ -n "$wp" ] || continue
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

if [ "$fail" -ne 0 ]; then
  echo "check-four-tooth: FAIL" >&2
  exit 1
fi
if [ -n "${2:-}" ]; then
  echo "check-four-tooth: OK (every record satisfies all four teeth and every in-scope work package has provenance)"
else
  echo "check-four-tooth: OK (every record satisfies all four teeth)"
fi
