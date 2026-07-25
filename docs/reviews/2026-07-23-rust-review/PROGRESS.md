# Rust code review — progress log (2026-07-23)

Status: **COMPLETE (2026-07-25)**. All 13 units reviewed; units 1-9
independently verified; final consolidated report at
`REVIEW.md`. This file is now a historical progress record - the
authoritative output is REVIEW.md (executive summary, corrected tallies,
cross-cutting themes, per-unit index, verification summary, and the
34-item design-decision list). Read `handover.md` for snapshot/criteria/
format.

## Snapshot under review

- Worktree: `/home/beefsack/Development/brdgme-review-snapshot`
- HEAD SHA: `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`
- All findings are written in the MAIN repo under this directory; curated
  per-unit files in `findings/`, unfiltered worker dumps in `findings/raw/`.

## Durability protocol (must be in every future Lead brief)

1. Every Worker writes its COMPLETE findings (including candidates the Lead
   may reject) to `findings/raw/<unit>-<topic>.md` as its final act, and
   appends incrementally as it finds things — never one final dump.
2. Each Lead maintains a continuously-updated `findings/raw/<unit>-LOG.md`
   (dispatches, returns, verify/reject decisions) so a quota cutoff loses at
   most the in-flight worker.
3. Leads curate/verify from raw files into `findings/<unit>.md`.

## Completed units (final)

Tallies are verification-corrected for units 1-9 and curated for units
10-13. Two rejected findings excluded (games-batch-d F13, web-server F30).

| # | Unit | Findings file | Findings (crit/maj/min/nit) |
|---|------|---------------|------------------------------|
| 1 | lib-game | `findings/lib-game.md` | 20 (3/4/8/5) |
| 2 | lib-support | `findings/lib-support.md` | 45 (1/5/23/16) |
| 3 | games-batch-a | `findings/games-batch-a.md` | 20 (0/6/6/8) |
| 4 | games-batch-b | `findings/games-batch-b.md` | 35 (1/5/13/16) |
| 5 | games-batch-c | `findings/games-batch-c.md` | 34 (0/4/14/16) |
| 6 | games-batch-d | `findings/games-batch-d.md` | 45 (1/6/16/22) |
| 7 | games-batch-e | `findings/games-batch-e.md` | 46 (1/5/18/22) |
| 8 | games-batch-f | `findings/games-batch-f.md` | 58 (0/2/22/34) |
| 9 | web-server | `findings/web-server.md` | 66 (0/8/36/22) |
| 10 | web-domain | `findings/web-domain.md` | 78 (1/12/35/30) |
| 11 | web-frontend-email | `findings/web-frontend-email.md` | 60 (2/12/28/18) |
| 12 | bot-operator-tools | `findings/bot-operator-tools.md` | 30 (0/4/15/11) |
| 13 | dependencies | `findings/dependencies.md` | 26 (0/4/17/5) |

Grand total: **563 findings (10 critical, 77 major, 251 minor, 225 nit)**.

Verification of units 1-9 (371 findings): 337 CONFIRMED, 31 ADJUSTED, 2
REJECTED, 1 UNVERIFIABLE. Verification reports under
`findings/verification/`. See REVIEW.md sections 2 and 5 for the full
rollup and the invalid-recommendation catches.

## Headline findings so far (details in the unit files)

- **lib/game parser: 3 critical panics on non-ASCII input** (char counts used
  as byte indices in `Space`/`Token`/`Enum` parse; `Enum` reachable via
  non-ASCII player names). Plus: `Enum` multi-byte values can never match;
  full-match priority is declaration-order dependent; `Many` loops lack a
  zero-progress guard (hang); OneOf "furthest error" machinery is dead code;
  suggest `Many` ignores `max`; typed-vs-spec impl drift; `doc_int`/`doc_many`
  render wrong help; `combine` dep unused.
- **lib/markup: critical byte/char `slice()` panic** in `{{canvas}}`
  (transform.rs:274); `.unwrap()` on digit overflow in markup parser; silent
  truncation because callers discard `rest`.
- **lib/cmd:** warp handler `unwrap()` panics on invalid `game` string in
  every game service (http.rs:54).
- **lib/game_client:** no timeout despite documented timeout-retry; hung game
  pod can block an operator reconcile worker forever.
- **lib/color:** regex+lazy_static exist only for dead API `Color::from_hex`;
  4k LOC otherwise justified (palette data, strong tests).
- **lib/cost vs splendor-2:** consolidate, but lib/cost needs `get`/`set`
  first; splendor keeps gold-joker `can_afford`.
- **alhambra-1: critical card-duplication exploit in `take()`** (lib.rs:570);
  place-index desync corrupts grid; longest-wall undercount.
- **starship-catan-1:** cannon priced off booster count; `can_lose_module`
  `||` vs `&&` lets players skip pirate fights; astro can go negative via
  TradeAndBuild; `i32::MAX` amounts overflow; Sensor peek never rendered.
- **roll-through-the-ages-2:** `roll()` phase-cascade re-match bug; four
  RULES.md↔code contradictions.
- **seven-wonders-1:** DrawDiscard resolver soft-lock; Halicarnassus VP
  dropped; end-of-age auto-discard wrongly pays 3 coins.
- **modern-art-2: critical infinite busy-loop in `settle_auction`**
  (lib.rs:452) → hang/OOM; round-4 soft-lock; payout deviates from official
  rules (Go-inherited — needs port-parity decision).
- **cathedral-2:** `Box::leak` per parser construction (~4–8 KB per request in
  the HTTP service).
- **sushizock-2:** `steal` with `i32::MIN` overflows → debug/fuzz panic.
- **acquire-1:** `player_counts()` excludes 6 despite MAX_PLAYERS=6; 2-player
  dummy shareholder never rolls 6.
- **lords-of-vegas-1:** 5× `unimplemented!()` one wiring line from a player
  panic; nondeterministic HashSet/HashMap iteration feeds seeded-RNG rerolls.
- **jaipur-2:** deck 8 camels/52 cards vs official 11/55; missing 6–7-card
  bonus token; round loser doesn't start next round.
- **red7-1: critical char/byte slice panic in CardParser** (command.rs:31);
  `leader()` wrong on Green/Violet with no fulfilling cards.
- **lost-cities-1/-2:** unchecked `hands[player]` index panics on crafted
  PlayerRender; -2 `status()` hardcodes stats to players 0/1 (3-player games
  lose stats); final-draw logs dropped. -1 is deprecated-but-deployed;
  duplication deliberate.
- **Cross-cutting:** 108 near-identical boilerplate game binaries; binary-only
  deps declared as library deps in every game crate; http bins bind 0.0.0.0:80;
  several Go-inherited rules deviations need a project-wide "port parity vs
  official rules" decision (modern-art-2 payout, splendor-2 tie-break, etc.).

(The headline list above is the units 1-7 snapshot; the full, corrected
headline set for all 13 units is in REVIEW.md section 1.)

## Consolidation (final unit)

All 13 units complete and consolidated into `REVIEW.md` on 2026-07-25.
REVIEW.md contains: executive summary and verdict against the five charter
values, the per-unit + grand-total tallies (section 2), cross-cutting
themes (section 3), the per-unit index with links to findings and
verification reports (section 4), the verification summary and
invalid-recommendation catches (section 5), and a 34-item "requires
discussion / design decision" list for the follow-on backlog/spec effort
(section 6). Consolidation working notes and log:
`findings/raw/consolidation-*.md`.
