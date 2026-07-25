# Verification LOG: lib-support (2026-07-24)

Independent verification of `findings/lib-support.md` (unit 2, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

45 findings total in lib-support.md, numbered F1-F45 in file order:

- lib/markup: F1 slice byte/char (crit), F2 parse_u8 unwrap (major),
  F3 silent truncation (major), F4 to_string no round-trip (minor),
  F5 eprintln rgb fallback (minor), F6 duplicated renderer logic (minor),
  F7 PLAYER_COUNT hardcode (minor), F8 word_wrap bytes (minor),
  F9 error discards diagnostics (minor), F10 panic transform/align (nit),
  F11 stale TNode::len doc (nit)
- lib/color: F12 regex/lazy_static dead API (major), F13 mono lossy div
  (minor), F14 three alias tables (minor), F15 palette verbosity (minor),
  F16 themes() doc (nit), F17 sRGB thresholds (nit), F18 hex()/Display
  dup (nit)
- lib/cmd: F19 http.rs unwrap panic (major), F20 REPL undo stale renders
  (minor), F21 bot_cli dead code (minor), F22 REPL EOF spin (minor),
  F23 panic-heavy paths (minor), F24 term_size unmaintained (minor),
  F25 warp vs axum (minor), F26 clippy comparison_to_empty (nit),
  F27 redundant serde default (nit), F28 no content-length limit (nit),
  F29 local requester exit status (nit), F30 first :undo no-op (nit)
- lib/game_client: F31 no crate timeout (major), F32 anyhow in lib
  (minor), F33 retry predicate narrow (minor), F34 serde_yaml deprecated
  (minor), F35 version_name Host header (nit), F36 fetch_game_data
  sequential (nit), F37 timing-sensitive test (nit)
- lib/cost: F38 spurious Clone bound (nit), F39 splendor-2 duplication
  (minor)
- lib/rand_bot: F40 unused chrono (minor), F41 join separator (minor),
  F42 http-server feature pulled (minor), F43 panics on degenerate specs
  (minor), F44 extern crate leftover (nit), F45 mangled comment (nit)

Four serial Workers (model fable):

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | lib/markup | F1-F11 | raw/lib-support-markup.md |
| W2 | lib/color + lib/cost | F12-F18, F38-F39 | raw/lib-support-color-cost.md |
| W3 | lib/cmd | F19-F30 | raw/lib-support-cmd.md |
| W4 | lib/game_client + lib/rand_bot | F31-F37, F40-F45 | raw/lib-support-client-randbot.md |

Lead spot-checks all REJECTED/ADJUSTED verdicts; if a Worker confirms
everything, Lead re-verifies its 1-2 hardest confirmations. Curated
report: `verification/lib-support.md`.

### W1 dispatched — lib/markup (F1-F11)

### W1 returned

10 CONFIRMED, 1 ADJUSTED. Dump: raw/lib-support-markup.md.
- F8 ADJUSTED: byte-measure and leading-space claims correct, but
  space-run collapsing only occurs at line starts/wrap points; mid-line
  runs are preserved. Lead to spot-check.
- All else CONFIRMED, incl. F1 canvas reachability and F10 no-panic rule
  (docs/CODING.md:46).

### W2 dispatched — lib/color + lib/cost (F12-F18, F38-F39)

### W2 returned

8 CONFIRMED, 1 ADJUSTED. Dump: raw/lib-support-color-cost.md.
- F15 ADJUSTED: substance holds but literal line count is ~2,000 not
  ~3,000; savings overstated. Lead to spot-check.
- F12 both sub-claims verified (regex sole dependent, parse API
  test-only); F39 semantic equivalences all verified.

### W3 dispatched — lib/cmd (F19-F30)

### W3 returned

12/12 CONFIRMED. Dump: raw/lib-support-cmd.md.
- Trivial refinement: F21's "three .unwrap()s" in bot_cli::cli is
  actually four (bot_cli.rs:29,30,41,43) — substance unchanged.
- No REJECTED/ADJUSTED; Lead to re-verify hardest confirmation (F19).

### W4 dispatched — lib/game_client + lib/rand_bot (F31-F37, F40-F45)

### W4 returned

13/13 CONFIRMED. Dump: raw/lib-support-client-randbot.md.
- All cross-crate claims verified (operator no-timeout, web 10s, bot
  60s, serde_yaml only shared with bot, chrono zero references).
- No REJECTED/ADJUSTED; Lead to re-verify hardest confirmation (F31).

## Lead spot-checks

- F8 ADJUSTED accepted: read lib/markup/src/wrap.rs in full. Byte
  measure at wrap.rs:16 confirmed. `s.split(' ')` yields empty words for
  mid-line space runs; the `current.len() + 1 + 0 <= width` branch then
  pushes one space per empty word, so mid-line runs ARE preserved
  (except at a wrap boundary); only leading spaces and runs at line
  starts collapse. Original "collapses runs of spaces" is overbroad;
  Worker's correction is right. Severity minor stands.
- F15 ADJUSTED accepted: Worker's dump quantifies 379 `Color {`
  literals x 5 lines = ~1,900 literal lines (not ~3,000), and a
  post-fix file of ~2,300 lines (not ~400) since docs/functions/tests
  dominate the remainder. Substance (const fn rgb, protanopia
  field-by-field clones at palette.rs:2707-2720/2806-2819) verified.
  Severity minor stands.
- F19 re-verified directly (hardest W3 confirmation): read
  lib/cmd/src/http.rs in full. `.unwrap()` on `g.request(&req)` at
  http.rs:54 inside the warp `.map` handler; `impl Reject for
  RequestError {}` at http.rs:17 with no `reject` call anywhere in the
  file. CONFIRMED.
- F31 re-verified directly (hardest W4 confirmation): read
  operator/src/controller.rs:228-231 (`reqwest::Client::new()`, no
  timeout) and lib/game_client/src/lib.rs:47-89 (`send_with_retry`
  never sets a per-request timeout; retry keys on
  `e.is_connect() || e.is_timeout()` at lib.rs:80). CONFIRMED.

## Curation complete (2026-07-24)

43/45 CONFIRMED, 2 ADJUSTED (F8, F15 — both factual refinements,
severities unchanged), 0 REJECTED, 0 UNVERIFIABLE.
Corrected unit tally (unchanged from original): 1 critical / 5 major /
23 minor / 16 nit.
Report: verification/lib-support.md. LOG closed.
