# Consolidation LOG (2026-07-25)

Lead session: consolidate the 13 curated unit findings files (+ 9
verification reports) into `2026-07-23-rust-review/REVIEW.md`. No code
changes; unit findings/verification files untouched.

## Plan

Serial Workers (model fable, per user override), each extracting per-unit
notes into `raw/consolidation-units-*.md`:

- W1: units 1-3 (lib-game, lib-support, games-batch-a) + verifications
  -> consolidation-units-1-3.md
- W2: units 4-6 (games-batch-b/c/d) + verifications
  -> consolidation-units-4-6.md
- W3: units 7-9 (games-batch-e/f, web-server) + verifications
  -> consolidation-units-7-9.md
- W4: units 10-11 (web-domain, web-frontend-email; no verification reports)
  -> consolidation-units-10-11.md
- W5: units 12-13 (bot-operator-tools, dependencies; no verification
  reports) -> consolidation-units-12-13.md

Per unit each worker extracts: corrected tallies (verification-corrected
for 1-9, curated for 10-13; rejected findings excluded and listed by ID);
verification verdict counts + notable rejections/adjustments +
invalid-recommendation catches; all criticals + notable majors (ID/title);
2-3 sentence unit characterization; theme-evidence bullets; discussion/
design-decision candidates.

Lead then assembles REVIEW.md directly from the five note files and updates
PROGRESS.md to final state.

## Dispatch/return entries

### W1 dispatched (units 1-3)

### W1 returned
- consolidation-units-1-3.md written. Tallies: U1 3c/4M/8m/5n (20/20
  CONFIRMED), U2 1c/5M/23m/16n (43 CONF, 2 ADJ), U3 0c/6M/6m/8n (19 CONF,
  1 ADJ). Zero rejections across 85 findings; original tallies matched.

### W2 dispatched (units 4-6)

### W2 returned
- consolidation-units-4-6.md written. U4 1c/5M/13m/16n (F9 upgraded), U5
  0c/4M/14m/16n (F23 downgraded), U6 1c/6M/16m/22n with F13 REJECTED (its
  recommendation would have introduced a bug). Notable: U6 F4 upgraded to
  major, F36 downgraded (documented in RULES.md), F26 premise refuted.
  Modern-art Go-inherited cluster (F34/F35/F36/F37) flagged for the
  port-parity design decision.

### W3 dispatched (units 7-9)

### W3 returned
- consolidation-units-7-9.md written. U7 1c/5M/18m/22n (F37 downgraded),
  U8 0c/2M/22m/34n (unchanged), U9 0c/8M/36m/22n (F1 crit->major, F27
  minor->major, F18 major->minor, F58 minor->nit, F30 REJECTED). U9 has a
  dense invalid-recommendation cluster (F30 SQL error; caveats on F1, F4,
  F6, F34, F48, F52, F66). Themes: deserialized-state trust panics (U7-8),
  Go-parity batch (7 items in U8), fail-open/silent-failure in U9.

### W4 dispatched (units 10-11)

### W4 returned
- consolidation-units-10-11.md written. U10 1c/12M/35m/30n (78), U11
  2c/12M/28m/18n (60). U11 criticals compose into account takeover
  (spoofable From + email-management commands), and further with U10
  email_token leak (forgeable invite responses). Undo/concede TOCTOU is
  one db.rs root cause across both units; unvalidated bot slots recur at
  4 entry points. Discussion candidates confirmed + extras (unsubscribe
  RFC 8058, game_visibility scope, reminder prefs, bot-wedge recovery).

### W5 dispatched (units 12-13)

### W5 returned
- consolidation-units-12-13.md written. U12 0c/4M/15m/11n (30), U13
  0c/4M/17m/5n (26). Notables: fuzzer permanent hang (live Sender); 4 of
  7 deny.toml advisory ignores reference crates absent from the lock;
  getrandom/serde_yaml/num_cpus findings overlap across units.

## Consolidation complete (2026-07-25)

REVIEW.md assembled at
`2026-07-23-rust-review/REVIEW.md` from the five worker note files. No code
changes; unit findings/verification files untouched.

Grand totals (verification-corrected units 1-9, curated units 10-13; 2
rejected findings excluded): 10 critical / 77 major / 251 minor / 225 nit
= 563 findings.

Verification rollup (units 1-9, 371 findings): 337 CONFIRMED, 31 ADJUSTED,
2 REJECTED, 1 UNVERIFIABLE. 4+ invalid-recommendation catches recorded
(games-batch-d F13, games-batch-e F45, games-batch-f F18, web-server F30 +
cluster) - REVIEW.md flags that all recommendations must be re-validated
at fix time.

Discussion/design-decision candidate list: 34 items (16 platform/infra, 9
dependencies/build, 9 game port-parity), plus smaller foldable questions.

PROGRESS.md updated to final state.
