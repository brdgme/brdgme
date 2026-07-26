# Work packages - brdgme Rust review remediation

Groups all surviving findings into work packages sized for roughly one
implementation session each. Finding IDs use unit prefixes: lg = lib-game,
ls = lib-support, a-f = games-batch-a..f, ws = web-server, wd = web-domain,
wfe = web-frontend-email, bo = bot-operator-tools, dp = dependencies.

IMPORTANT: units 10-13 findings docs (web-domain, web-frontend-email,
bot-operator-tools, dependencies) do not number their findings. The F-numbers
used here were assigned by this triage in document order and are recorded,
with one row per finding, in `planning/raw/` (w5-webdomain-email.md ID audit
section, w6-botops-deps.md). Those raw files are the ID-to-finding mapping.

IMPORTANT: per REVIEW.md section 5, fix recommendations in the findings files
are a starting direction only - several were proven wrong or harmful. Every
recommendation must be re-validated at spec/fix time. Known-unsound
recommendations are flagged in the relevant package notes below and
enumerated in the raw files' grouping notes.

Status legend: READY = mechanical, spec can be written now.
BLOCKED-ON-DECISION(D-nn) = gated on an item in `decisions-needed.md`.
**As of 2026-07-26 this status is EXTINCT - all decisions are answered. Any
remaining `BLOCKED-ON-DECISION` text in this file is historical narrative, not
a live status.**
BLOCKED-ON-USER-RULES-REVIEW = **stronger than BLOCKED-ON-DECISION.** Gated on
the user personally reviewing the game rules; does NOT clear when a decision is
answered, only on the user's per-game sign-off. Added 2026-07-25 for the parked
parity packages. An implementing agent must not pick these up, must not change
gameplay, and must not "correct" a `RULES.md` under them.

**2026-07-25 decision session:** the 10 critical-path gating groups (D-1,
D-2, D-3+D-4, D-5, D-6+D-13, D-8, D-12+D-14, D-33+D-35, D-36, D-37) are
ANSWERED - see the ANSWERED summary table at the top of
`decisions-needed.md`. Twelve packages flipped to READY: WP-02, WP-09,
WP-10, WP-35, WP-38, WP-40, WP-42, WP-45, WP-47, WP-49, WP-56, WP-57. Two
had one of two blockers cleared: WP-46 (now D-11 only), WP-11 (now D-30
only). Each flipped package's note below carries the binding answer;
read the ANSWERED block in `decisions-needed.md` before speccing.
WP-42 carries a confirmation caveat (D-13 label ambiguity).

**2026-07-25 refinement session (four refinements):**
1. **WP-56 narrowed** - only `emails add`/`confirm`/`active`|`use`/`remove` leave
   the email interface. Username (`name`), `theme`, `colors` and notification
   prefs are KEPT. Token + SPF/DKIM work unchanged. Cold start resolved (web UI
   opt-in reveal of the tokenised address; never an email footer).
2. **WP-42 unblocked** - D-13's label ambiguity is resolved (option B shape:
   authenticate the upgrade, filter server-side, `sub`/`unsub` for public-game
   pages only). The confirmation caveat in the line above is superseded; see
   WP-42's entry and the D-13 block for the verified `/ws` findings and design.
   WP-47 gains a note: WP-42 must reuse its `is_game_visible_to_user`.
   **Partly superseded 2026-07-26 by D-44** - the `sub`/`unsub` half is
   ELIMINATED and the authenticate-the-upgrade half is SUPERSEDED by WP-84
   (SSE). The server-side-filtering half stands. See WP-42's and WP-84's
   entries.
3. **Parity PARKED** - D-35 and D-26..D-32, D-34 are
   PARKED-PENDING-USER-RULES-REVIEW. Packages **WP-11, WP-12, WP-16, WP-20,
   WP-26, WP-30** are now BLOCKED-ON-USER-RULES-REVIEW. **WP-10 stays READY**
   despite its D-35 mention (redaction, not parity). WP-15, WP-19, WP-22, WP-23,
   WP-25, WP-29 are unaffected. Five egregious-fix candidates (a F1, b F4, b F7,
   e F30's seat-order half, d F37) are flagged to the user in the Group D banner
   of `decisions-needed.md` - flagged only, NOT unparked and NOT specced.
4. **Landing order** - WP-41 before WP-40, and the WP-56/WP-59/WP-54 interactions
   are consolidated in `planning/landing-order.md`. Read it before speccing or
   implementing any of WP-40, WP-41, WP-54, WP-56, WP-59.

**2026-07-26 decision session: ALL 34 remaining decisions are ANSWERED.**
Read **`planning/decisions-ANSWERED.md`** first - it is the resolved record
and it WINS over any older text in this file, in `decisions-needed.md`, in a
spec, or in a checklist. `open-decisions-for-user.md` is now a stub.

- **`BLOCKED-ON-DECISION` is now EXTINCT.** Every package carrying that label
  is cleared: **WP-04** (D-38), **WP-05** (D-39), **WP-17** (D-25, the partial
  block), **WP-46** (D-11), **WP-48** (D-7, **scope SHRANK** - see its entry),
  **WP-50** (D-9), **WP-55** (D-16, **scope GREW** - see its entry), **WP-58**
  (D-10, **scope GREW**), **WP-64** (D-19), **WP-66** (D-17), **WP-67**
  (D-18), **WP-69** (D-23), **WP-70** (D-21), **WP-71** (D-22), **WP-72**
  (D-24), **WP-73** (D-20). WP-20 and WP-30 lose their D-40 blocker but stay
  `BLOCKED-ON-USER-RULES-REVIEW`. WP-62's `bo F25` rider is answered.
- **New package: WP-81** - the D-40 stats deletions, split out of WP-20/WP-30
  so they land ahead of the rules review.
- **The parity park STAYS** (D-35). Review per game, prioritising acquire-1,
  seven-wonders-1/splendor-2, modern-art-2, red7-1. `WP-11, WP-12, WP-16,
  WP-20, WP-26, WP-30` remain `BLOCKED-ON-USER-RULES-REVIEW`.
- **The five egregious candidates are ruled on individually** and are the only
  movement out of the park: **`a F1` FIX NOW** (WP-12), **`b F7` FIX NOW**
  (WP-16), **`e F30` FIX NOW** (WP-30 - its condition was verified satisfied),
  **`b F4` PARKED** with the user's binding correction, **`d F37` REJECTED**
  as not a bug. Details in each package's entry and in the Group D banner of
  `decisions-needed.md`.
- **Five standing constraints** now bind implementers beyond the row that
  produced them: dependency work upgrades everything to latest FIRST (D-17,
  WP-64..WP-73); macro surfaces stay small and obvious, pause and discuss if
  one gets complex (D-20); WP-04 keeps the parser straightforward and obvious
  (D-38); no Sentry functionality may be lost to the feature trim (D-18);
  `lib/cost` gains suitable automated tests as part of the port (D-25).

## Core libraries

### WP-01 char/byte panic elimination - READY
- Scope (7): lg F1, lg F2, lg F3, lg F4, lg F16, ls F1, e F29
- Paths: rust/lib/game/src/command/parser/mod.rs, rust/lib/markup/src/transform.rs, game/red7-1/src/command.rs
- Severity: 5c/1M/0m/1n
- One defect class: char counts used as byte indices. Fix pattern shared
  across all sites; add a workspace non-ASCII input test convention (NBSP,
  multi-byte names) - the core libs currently have zero non-ASCII coverage.
- Deps: none. Highest-priority code package.

### WP-02 markup robustness and dedup - READY (D-37 answered: option A)
- Scope (10): ls F2-F11
- Paths: rust/lib/markup/src/{parser,lib,wrap,html,html_class,transform,semantic,ast,error}.rs
- Severity: 2M/6m/2n
- Parser panics (F2/F10), silent truncation + no round-trip (F3/F4 - gated
  on the literal-{ escape decision), helper dedup (F6/F7), word_wrap (F8),
  diagnostics (F9). F7 links to lib/color (export a player-count const).

### WP-03 lib-game parser mechanical fixes - READY
- Scope (11): lg F5, lg F6, lg F8, lg F9, lg F10, lg F11, lg F12, lg F15, lg F18, lg F20, c F31
- Paths: rust/lib/game/src/command/{parser/mod.rs,suggest.rs,doc.rs}, rust/lib/game/Cargo.toml
- Severity: 2M/7m/2n
- Enum match priority, Many zero-progress guards, suggest max-cap (fixing
  lg F9 discharges c F31 sushizock suggest overrun), doc rendering, unused
  combine dep line.

### WP-04 lib-game parser design items - READY (D-38 answered 2026-07-26: all four sub-items as recommended)
- Scope (5): lg F7, lg F13, lg F14, lg F17, lg F19
- Paths: rust/lib/game/src/command/{parser/mod.rs,suggest.rs}
- Severity: 1M/2m/2n
- OneOf offset propagation vs delete ranking; typed-vs-spec expected()
  divergence; case-folding convention; spec depth guard.
- **D-38 ANSWERED:** (i) **implement** OneOf offset propagation; (ii) align
  spec `expected()` impls to typed behaviour and **extend the parity tests to
  cover `expected()`**; (iii) adopt **UniCase** in `suggest`; (iv) **skip** the
  deserialized-spec depth guard (specs cross no trust boundary today).
- **STANDING CONSTRAINT on this package as a whole, not just those four items:
  keep the parser as straightforward and obvious as possible.** It is complex
  but critical to the app and must stay reliable and maintainable. Prefer the
  plainer implementation at every choice point - especially in the
  offset-propagation plumbing, which is the item most likely to tempt an
  elegant abstraction.

### WP-05 lib color - READY (D-39 answered 2026-07-26: option A, delete the dead parse API)
- Scope (7): ls F12-F18
- Paths: rust/lib/color/src/{lib.rs,palette.rs}, rust/lib/color/Cargo.toml
- Severity: 1M/3m/3n
- Delete-vs-keep of the dead parse API (F12) resolves F14 for free and
  drops regex/lazy_static. F13/F17/F18 math+dedup, F15 const-fn palette.
- **D-39 ANSWERED: option A - delete `from_hex`/`from_str`.** Drops `regex`
  and `lazy_static` workspace-wide and resolves the three-way alias-table
  divergence by deletion. Git can resurrect it if ever needed.

### WP-06 lib cmd tools and http - READY
- Scope (12): ls F19, ls F20, ls F21, ls F22, ls F23, ls F26, ls F27, ls F28, ls F29, ls F30, ls F44, ls F45
- Paths: rust/lib/cmd/src/{http.rs,repl.rs,bot_cli.rs,cli.rs,requester/gamer.rs,requester/local.rs,api.rs}, rust/lib/rand_bot/src/main.rs
- Severity: 1M/4m/7n
- F19 (prod warp handler unwrap) is the urgent item; rest is dev-tool
  robustness and dead-code removal (bot_cli).

### WP-07 game_client and rand_bot - READY
- Scope (11): ls F31, ls F32, ls F33, ls F35, ls F36, ls F37, ls F40, ls F41, ls F42, ls F43, dp F10
- Paths: rust/lib/game_client/src/lib.rs, rust/lib/rand_bot/{src/lib.rs,Cargo.toml}
- Severity: 1M/7m/3n
- Request timeout (F31 - operator hang risk), error types, chrono removal
  (ls F40 = dp F10, same finding), rand_bot panics, dep trims.

## Cross-cutting game packages

### WP-08 finish/placings epilogue dedup sweep - READY
- Scope (12): a F6, b F11, b F22, b F33, c F6, d F21, d F33, e F1, e F13, e F14, f F7, f F35
- Paths: game/{roll-through-the-ages-2,seven-wonders-1,alhambra-1,splendor-2,texas-holdem-2,jaipur-2,sushi-go-2,love-letter-2,age-of-war-2,zombie-dice-2,greed-2}/src/lib.rs
- Severity: 1M/1m/10n
- Same copy-pasted epilogue in ~11 crates; pick one refactor shape (shared
  helper in lib/game vs identical per-crate extract) at spec time. Also
  fixes e F14's duplicate-log amplification.

### WP-09 deserialized-state trust hardening - READY (D-36 answered: option A) - **SPLIT at spec time into WP-09a + WP-09b**
- **SPLIT (2026-07-25, T2-B4 Lead).** This package is now two specs:
  - **WP-09a deserialized-state boundary** (`specs/WP-09a-deserialized-state-boundary.md`)
    - the requester side: the `lib/cmd/src/requester/gamer.rs` bounds check plus a new
    defaulted `Gamer::validate` hook in `rust/lib/game/src/game.rs`. **Lands first.**
  - **WP-09b game-crate state-trust sweep** (`specs/WP-09b-game-crate-state-trust-sweep.md`)
    - the 18-item per-crate sweep filling WP-09a's hook. **Strictly depends on WP-09a.**
- Scope (19): d F5, d F38, e F2, e F3, e F4, e F10, e F18, e F22, e F36, f F4, f F9, f F19, f F26, f F29, f F34, f F44, f F46, f F53, f F55
- Paths: lib/cmd/src/requester/gamer.rs, **rust/lib/game/src/game.rs** (WP-09a's new `Gamer::validate` hook) + ~15 game crates (lords-of-vegas-1, modern-art-2, love-letter-2, age-of-war-2, lost-cities-1/-2, zombie-dice-2, battleship-2, for-sale-2, category-5-2, greed-2, farkle-2, tic-tac-toe-2, no-thanks-2, liars-dice-2, red7-1, **rust/game/acquire-1**, **rust/game/sushizock-2**)
- Severity: 2M/12m/5n
- ADDED at spec time (unit 4d Lead, from the WP-29 spec's cross-package item 4):
  `rust/game/red7-1` also trusts a deserialized `num_players` beyond the one
  arithmetic panic WP-29 Task 2 closes - `render.rs:135` and the
  `0..self.num_players` loops at `lib.rs:207`, `:403`, `:413` assume
  `num_players` agrees with the per-player vector lengths. No finding ID (found
  during spec writing, not in the review); do not widen WP-29 Task 2 for it.
- The systemic "deserialized Game/PubState trusted verbatim" class. D-36
  decides requester-boundary validation vs per-crate defensive sweep vs
  accept. The two request-reachable player_state panics (e F18/F36) are
  covered by one bounds check at gamer.rs regardless. May split into a
  requester package + per-crate sweep at spec time.

### WP-10 pub_state hidden-info redaction - READY (D-33 answered: option A) - **STAYS READY: the 2026-07-25 D-35 park does NOT apply here.** This package is hidden-information leakage, not rules parity; its D-35 mention is incidental. Do not re-block it.
- Scope (2): f F1, f F13
- Paths: game/zombie-dice-2/src/lib.rs, game/for-sale-2/src/lib.rs
- Severity: 2M
- Both batch-f majors; redaction shape decided once for all game crates.

### WP-11 batch-f port-parity adjudication - BLOCKED-ON-USER-RULES-REVIEW (D-30 + D-35 PARKED 2026-07-25 - do not pick up)
- Scope (8): f F2, f F14, f F15, f F21, f F33, f F43, f F50, f F54
- Paths: game/{zombie-dice-2,for-sale-2,greed-2,farkle-2,no-thanks-2,liars-dice-2}/src + RULES.md files, lib/game/src/game.rs (F21 gen_placings)
- Severity: 6m/2n
- Zero code until the parity policy ruling; then per-crate one-liners plus
  RULES.md updates.
- **PARKED-PENDING-USER-RULES-REVIEW.** All eight items are annotated Go
  quirk/port-parity and none produces invalid or asymmetric state, so NONE is an
  egregious-fix candidate. Also do not "correct" any RULES.md here - the docs are
  themselves under user review and some content was AI-generated.

## Game crates

### WP-12 roll-through-the-ages-2 - BLOCKED-ON-USER-RULES-REVIEW **except `a F1`, which is FIX NOW** (D-34 PARKED; a F1 released 2026-07-26)
- Scope (9): a F1, a F2, a F3, a F4, a F5, a F7, a F8, a F9, a F10
- Paths: game/roll-through-the-ages-2/{src/lib.rs,src/command.rs,src/player_board.rs,RULES.md}
- Severity: 1M/3m/5n
- F1 (roll-phase re-match) needs the fidelity call; RULES.md doc fixes
  (F2-F5) are mechanical riders. F1's fix must adjudicate the crate's own
  `next`-path test which locks the current behaviour.
- **PARKED-PENDING-USER-RULES-REVIEW.** a F1 is flagged to the user as an
  **egregious-fix candidate** (the previous player's `roll()` decrements the NEXT
  player's `remaining_rolls` - cross-player state corruption no edition
  specifies), but it is NOT unparked and NOT specced: wait for the user. F7/F9 and
  the quirk-preservation policy are squarely parked.
- **RESOLVED 2026-07-26: `a F1` is FIX NOW, outside the park, as recommended.**
  It is released from the park and needs a spec. **The other eight items
  (a F2-F5, F7, F8, F9, F10) STAY PARKED** - do not touch RULES.md here.
  The a F1 fix must adjudicate the crate's own
  `test_game_keep_skulls_all_disaster_leadership`, which asserts the opposite
  outcome for the `next`-command path.

### WP-13 starship-catan-1 - READY
- Scope (10): a F11-F20
- Paths: game/starship-catan-1/src/{lib.rs,command.rs,render.rs,card.rs}
- Severity: 5M/2m/3n
- Three legal-play economy bugs, i32 overflow from input, Sensor peek
  render gap, cleanups. F20 (BTreeSet shape) is a serialized-state change;
  handle with a compatible migration or skip.

### WP-14 alhambra-1 core fixes - READY
- Scope (10): b F16, b F17, b F18, b F21, b F23, b F24, b F25, b F26, b F27, b F28
- Paths: game/alhambra-1/src/{lib.rs,card.rs,render.rs,command.rs}
- Severity: 1c/2M/1m/6n
- Critical duplicate-card mint (F16), place-index divergence (F17), wall
  walk undercount + HashMap nondeterminism (F18); F21's missing-test list
  enumerates exactly these paths - land tests with the fixes.

### WP-15 seven-wonders-1 mechanical fixes - READY
- Scope (9): b F1, b F2, b F3, b F9, b F10, b F12, b F13, b F14, b F15
- Paths: game/seven-wonders-1/src/{lib.rs,command.rs,card.rs}
- Severity: 3M/2m/4n
- Halicarnassus VP, DrawDiscard soft-lock, 7th-card coin bug (F2/F3 agree
  with PORTING_NOTES + official rules - not parity-gated). F9: original
  "unreachable" analysis was invalidated; the store-chosen-deal fix stands.

### WP-16 batch-b rules adjudication - BLOCKED-ON-USER-RULES-REVIEW **except `b F7`, which is FIX NOW** (D-27, D-28 PARKED; b F7 released and b F4 re-parked 2026-07-26)
- Scope (8): b F4, b F5, b F6, b F7, b F8, b F19, b F20, b F29
- Paths: game/seven-wonders-1/src, game/alhambra-1/src, game/splendor-2/src
- Severity: 8m
- seven-wonders deviations (F4-F7), discard-pile visibility (F8), alhambra
  Dirk/deck-size (F19/F20), splendor tie-break (F29, test-locked).
- **PARKED-PENDING-USER-RULES-REVIEW.** Flagged to the user as
  **egregious-fix candidates** (not unparked, not specced): **b F4** (same-turn
  trade of freshly built goods is asymmetric by player index - unfair by seat, and
  no edition of 7 Wonders is anything but simultaneous-symmetric) and **b F7**
  (both A and B sides of one wonder can be dealt - 14 boards where every printing
  has 7). F5/F6/F8, alhambra and splendor F29 are genuine edition/adjudication
  questions and stay parked.
- **RESOLVED 2026-07-26 - the two candidates split:**
  - **`b F7` is FIX NOW, outside the park.** Ensure **only one of each
    physical board can be in play**. Released from the park; needs a spec.
  - **`b F4` is REMOVED from the egregious list and PARKED** under the rules
    review. **The user's correction is binding: 7 Wonders resources are NOT
    depleted by trade** - they are printed on cards and both players use them,
    so there is no competition for a resource and the "asymmetric advantage by
    seat" framing above was **WRONG**. **Residual narrower question, recorded
    so it is not lost, parked for the user's review and NOT scheduled:**
    because players resolve in seat order against live state, player p+1 can
    trade for a resource card player p **built on that same turn**, which p
    could not have done in reverse - a **simultaneity** question, not a
    scarcity one.
  - Everything else in this package (b F5, F6, F8, F19, F20, F29) **stays
    parked.**
- Not affected by the park: **WP-15** (b F1/F2/F3, including the reachable
  permanent DrawDiscard soft-lock) is READY and was never parity-gated.

### WP-17 splendor-2 + lib/cost consolidation - READY (D-25 answered 2026-07-26: option A; the partial block is CLEARED)
- Scope (8): b F30, b F31, b F32, b F34, b F35, ls F38, ls F39, dp F27
- Paths: game/splendor-2/src/{lib.rs,command.rs,cost.rs}, rust/lib/cost/src/lib.rs
- Severity: 4m/4n
- **D-25 gates only 3 of the 8: `b F31`, `ls F39`, `dp F27`** - the lib/cost
  keep-or-fold consolidation, which is one change seen from three findings
  units and must be implemented together. The other 5 (`b F30`, `b F32`,
  `b F34`, `b F35`, `ls F38`) are splendor-2 / lib/cost hardening and cleanups
  that are **implementable now**. An earlier package-level
  `BLOCKED-ON-DECISION(D-25)` label overstated the gate.
- Checklist: `planning/checklists/T3-B3-splendor-libcost-holdem.md` (authoritative
  row-by-row split of implementable vs D-25-blocked).
- **D-25 ANSWERED 2026-07-26: option A - port splendor-2 onto `lib/cost`.**
  Add generic `get`/`set`; keep splendor's gold-joker `can_afford` as a
  crate-local extension. All 8 findings are now implementable and the
  `PARTIALLY-BLOCKED` label is retired.
- **CONSTRAINT: the shared `lib/cost` must have a suitable amount of automated
  testing as part of the port.** It gains a second consumer and so stops being
  incidentally covered by seven-wonders-1's tests; give it its own.

### WP-18 texas-holdem-2 cleanup - READY
- Scope (4): c F1, c F3, c F4, c F5
- Paths: game/texas-holdem-2/src/{command.rs,lib.rs,poker.rs,card.rs}
- Severity: 1m/3n
- (c F2 player cap sits in WP-20.)

### WP-19 acquire-1 fixes - READY
- Scope (11): c F7, c F8, c F9, c F10, c F11, c F16, c F17, c F18, c F19, c F20, c F21
- Paths: game/acquire-1/src/{lib.rs,command.rs,render.rs,stats.rs,board.rs}, game/acquire-1/Cargo.toml
- Severity: 2M/4m/5n
- 6-player offering, dummy die never rolls 6, panic hardening, stats
  copy-paste. (Edition decisions and stats keep-or-drop sit in WP-20.)
- **`c F11` is SUPERSEDED by WP-81** (added 2026-07-26 by the Batch 6 spec
  Lead). WP-19 Task 5 edits one line inside `src/stats.rs::to_brdgme_stats`
  and adds a `stats.rs` test; **WP-81 deletes that file entirely** (zero
  callers, verified). Land WP-81 first and DROP Task 5. `c F11` stays listed
  here as superseded, not reassigned, so the one-package-per-finding invariant
  and the 570 sum are unaffected. See `landing-order.md` section 9.

### WP-20 batch-c rules and edition adjudication - BLOCKED-ON-USER-RULES-REVIEW (D-30, D-31 PARKED 2026-07-25 - do not pick up). **D-40 block CLEARED; `c F12` moved to WP-81.**
- Scope (4, was 5): c F2, c F13, c F14, c F15. **`c F12` left this package
  2026-07-26** - D-40 answered option B and the acquire stats deletion is now
  **WP-81**, which lands ahead of the rules review. Coverage bookkeeping:
  c F12 still appears in exactly one package (WP-81), so the 570 sum and the
  one-package-per-finding invariant are unaffected; WP-20's row in the
  coverage table drops 5 -> 4 and WP-81 gains 3.
- Paths: game/texas-holdem-2/src/lib.rs, game/acquire-1/src/{lib.rs,board.rs,stats.rs}
- Severity: 5m
- holdem player cap; acquire edition trio (F13/F14/F15 compound - decide
  together); acquire stats wire-or-delete.
- **PARKED-PENDING-USER-RULES-REVIEW** for the rules half. The acquire trio is the
  clearest edition question in the whole review (the findings name later-Hasbro vs
  classic 3M/AH explicitly, and bag-exhaustion behaviour "differs by edition"), so
  **no egregious-fix candidate here**. The D-40 acquire-stats item (c F12,
  write-only `to_brdgme_stats`) is NOT a rules question and could be split out into
  its own package if the user wants it moving before the rules review - flagged,
  not done.
- **DONE 2026-07-26: the split-out happened.** D-40 was answered option B
  (delete the dead machinery) and c F12 now lives in **WP-81**. What remains
  here (c F2, c F13, c F14, c F15) is purely the parked rules/edition half.

### WP-21 cathedral-2 + sushizock-2 - READY
- Scope (12): c F22, c F23, c F24, c F25, c F26, c F27, c F28, c F29, c F30, c F32, c F33, c F34
- Paths: game/cathedral-2/src/{command.rs,lib.rs,loc.rs,piece.rs}, game/cathedral-2/Cargo.toml, game/sushizock-2/src/{lib.rs,command.rs}
- Severity: 2M/3m/7n
- Box::leak traffic-driven memory leak; i32::MIN overflow; missing
  placings log. F23 is comment-only - verification refuted the premise; do
  NOT change the flood-fill.
- **NOTE (2026-07-25, T2-B4 Lead): Task 10 must carry WP-09a's new sushizock-2
  `Player{}` `target` bounds guard forward through its refactor.** WP-09a adds
  the guard; WP-21 Task 10 rewrites the surrounding code and must not drop it.

### WP-22 lords-of-vegas-1 - READY
- Scope (10): d F1, d F2, d F3, d F4, d F6, d F7, d F8, d F9, d F10, d F11
- Paths: game/lords-of-vegas-1/{src/lib.rs,src/board.rs,src/render.rs,src/tile.rs,src/casino.rs,Cargo.toml,RULES.md}
- Severity: 3M/2m/5n
- unimplemented!() arms, RNG nondeterminism (HashMap order), render
  underflow reachable in ordinary 5-6p play (verification upgraded F4).

### WP-23 jaipur-2 - READY
- Scope (6): d F14, d F17, d F18, d F19, d F20, d F22
- Paths: game/jaipur-2/{src/lib.rs,src/command.rs,src/render.rs,RULES.md}
- Severity: 1M/2m/3n
- 6/7-card sale bonus token (F14). NOTE: rejected sibling F13's fix
  (Camel => 11) would create 14 camels - keep camel count at 8/3 split.

### WP-24 sushi-go-2 - READY
- Scope (7): d F26, d F27, d F28, d F29, d F30, d F31, d F32
- Paths: game/sushi-go-2/{src/lib.rs,src/render.rs,RULES.md}
- Severity: 2m/5n
- F26 is doc-only (pudding tiebreak is correct per official rules).

### WP-25 modern-art-2 liveness and cleanup - READY
- Scope (9): d F34, d F35, d F39, d F40, d F41, d F42, d F44, d F45, d F46
- Paths: game/modern-art-2/{src/lib.rs,src/render.rs,RULES.md}
- Severity: 1c/1M/3m/4n
- F34 (critical infinite busy-loop) + F35 (round-4 soft-lock) share one
  missing invariant (empty-hand handling at round boundaries); fix
  together with round-4 regression tests. Does not wait on the D items in
  WP-26.

### WP-26 batch-d rules adjudication - BLOCKED-ON-USER-RULES-REVIEW (D-26, D-30, D-32 PARKED 2026-07-25 - do not pick up; d F37 is an egregious-fix candidate awaiting the user's word)
- Scope (9): d F12, d F15, d F16, d F23, d F24, d F25, d F36, d F37, d F43
- Paths: game/lords-of-vegas-1, game/jaipur-2, game/sushi-go-2, game/modern-art-2 (src + RULES.md)
- Severity: 1M/5m/3n
- Modern Art payout/ranking/tie-break; jaipur starter/tie-break/camel
  visibility; sushi-go pass direction + 2p pudding; LoV player counts.
- **PARKED-PENDING-USER-RULES-REVIEW.** Flagged to the user as an
  **egregious-fix candidate** (not unparked, not specced): **d F37** - `end_round`
  initialises `highest_count = -1`, so artists with ZERO cards on the table are
  still awarded 2nd ($20) / 3rd ($10) whenever fewer than three artists had
  paintings played, and the bogus values enter `value_board` and inflate every
  later round. Honest caveat: `modern_art.go:389-403` is identical, so it IS a
  Go-parity item and a strict parity framing can claim it - which is precisely why
  it needs the user's word.
- **RESOLVED 2026-07-26: `d F37` is REJECTED - NOT A BUG. Do not "fix" it
  later.** The user: **this is the accepted way to play** - if only one artist
  has cards, 2nd and 3rd go to the artists **in order from the top**.
  `suits()` already returns the canonical top-to-bottom order (Lite Metal top,
  Krypto bottom) and `end_round` scans `suits()` in declared order, so the
  current behaviour is correct. **No follow-up, no ordering caveat.** The
  Go-parity caveat above is moot: the behaviour is intended, not inherited by
  accident. Everything else in this package stays parked.
- Not affected by the park: **WP-25** (READY) owns d F34 (critical infinite
  busy-loop) and d F35 (round-4 soft-lock) and its own note already says it "does
  not wait on the D items in WP-26". The modern-art liveness fixes ship regardless.

### WP-27 love-letter-2 + age-of-war-2 - READY
- Scope (8): e F5, e F6, e F7, e F8, e F11, e F12, e F15, e F16
- Paths: game/love-letter-2/src/{lib.rs,command.rs}, game/age-of-war-2/src/{lib.rs,command.rs,render.rs}
- Severity: 2m/6n
- Post-finish command acceptance; HashSet serialization nondeterminism.

### WP-28 lost-cities-1/-2 shared fixes - READY
- Scope (13): e F17, e F19, e F20, e F23, e F24, e F26, e F27, e F37, e F38, e F41, e F42, e F43, e F44
- Paths: game/lost-cities-2/src, game/lost-cities-1/src, k8s/base/game/lost-cities-2/game-version.yaml
- Severity: 1M/5m/7n
- 3p stats hardcoded to players 0/1 (F17); most defects shared verbatim
  between the two crates - fix both in one package to avoid drift.

### WP-29 red7-1 cleanup - READY
- Scope (5): e F31, e F32, e F33, e F34, e F35
- Paths: game/red7-1/{src/lib.rs,DATA_DOCS.md,RULES.md}
- Severity: 2m/3n
- Doc rewrites (F31/F32) partially downstream of D-29's outcome; write
  them after WP-30 resolves if D-29 changes behaviour.

### WP-30 batch-e rules and stats adjudication - BLOCKED-ON-USER-RULES-REVIEW (D-29 PARKED) **except `e F30`'s seat-order half, which is FIX NOW**. **D-40 block CLEARED; `e F39`/`e F40` moved to WP-81.**
- Scope (3, was 5): e F21, e F25, e F30. **`e F39` and `e F40` left this
  package 2026-07-26** for **WP-81** (D-40 answered option B). Coverage
  bookkeeping: both still appear in exactly one package, so the 570 sum and
  the one-package-per-finding invariant hold; WP-30's coverage row drops
  5 -> 3.
- Paths: game/red7-1/src/{card.rs,lib.rs}, game/lost-cities-1/src/lib.rs, game/lost-cities-2/src/lib.rs
- Severity: 1M/3m/1n
- red7 empty-winning-set; lost-cities stats keep-or-drop and discard-pile
  visibility.
- **PARKED-PENDING-USER-RULES-REVIEW** for e F30's rules half ("can a player with
  an empty winning set win at all"). **The other half is flagged to the user as an
  egregious-fix candidate** (not unparked, not specced): when ALL palettes have an
  empty winning set, counts tie at 0 and `rank_key` maxes tie at `(0,0)`, so the
  strict `>` at `card.rs:311` leaves the **first non-eliminated player** as leader
  and the `discard` pre-check lets the **lowest-index** player discard into a rule
  nobody satisfies. Tie-breaking by seat order is in no edition.
- The lost-cities D-40 items are NOT rules questions and are only blocked on D-40.
- **RESOLVED 2026-07-26: `e F30`'s seat-order half is FIX NOW.** The user's
  ruling was conditional - fix it only if the correct behaviour is officially
  described or universally accepted, and park it if resolving it would need a
  subjective judgement on our part. **The condition was verified SATISFIED
  (read-only, 2026-07-26):** `rust/game/red7-1/DATA_DOCS.md` states the second
  tie-break verbatim - "Ties within a rule are broken by the highest card in
  the winning set, **then by the highest card overall in the palette**" - and
  official Red7 rules agree (card value = number then colour, exactly what
  `Card::rank_key` already encodes). The code simply never implements it.
  **Cause:** `leader()` in `card.rs` only ever receives the **already-filtered
  winning sets** (`lib.rs` pushes `rule_fn(&self.palettes[p])`), so the full
  palette is unreachable from it; all-empty means every `len()` is 0 and every
  max is `(0,0)`, the strict `>` never fires, and seat 0 wins. **The fix is to
  fall through to the FULL palette's `rank_key()` max**, which requires
  plumbing the unfiltered palette into `leader()`. **No rule is being
  invented.**
- **`e F30`'s D-29 half ("can an empty winning set win at all") STAYS PARKED**,
  as do `e F21` and `e F25`. WP-29's doc rewrites (e F31/F32) remain partially
  downstream of D-29 and should still wait on it.

### WP-31 zombie-dice-2 + battleship-2 - READY
- Scope (7): f F3, f F5, f F6, f F8, f F10, f F11, f F12
- Paths: game/zombie-dice-2/src/{lib.rs,render.rs}, game/battleship-2/src/{lib.rs,command.rs}
- Severity: 2m/5n
- f F6: transition-only guard would miss mid-rolloff membership changes;
  f F11: command.rs:68 needs .to_vec() with the return-type change.

### WP-32 for-sale-2 + category-5-2 - READY
- Scope (12): f F16, f F17, f F18, f F20, f F22, f F23, f F24, f F25, f F27, f F28, f F30, f F31
- Paths: game/for-sale-2/{src/lib.rs,src/render.rs,RULES.md}, game/category-5-2/{src/lib.rs,src/render.rs,RULES.md}
- Severity: 5m/7n
- f F18: #[serde(default)] Phase migration is UNSOUND (defaults to Buying,
  corrupts live Selling games) - use Option<Phase> with fallback. f F28:
  do NOT negate points() - ELO uses place; label-only fix.

### WP-33 small-crate cleanup (greed/farkle/ttt/no-thanks/liars-dice) - READY
- Scope (17): f F32, f F36, f F37, f F38, f F39, f F40, f F41, f F42, f F45, f F47, f F48, f F49, f F51, f F52, f F56, f F57, f F58
- Paths: game/{greed-2,farkle-2,tic-tac-toe-2,no-thanks-2,liars-dice-2}/src + RULES.md
- Severity: 4m/13n
- f F48: casing fix must update the exact-render test at lib.rs:589-600.

## web-server

### WP-34 auth races and session mechanical - READY
- Scope (9): ws F1, ws F3, ws F5, ws F6, ws F10, ws F12, ws F13, ws F14, ws F15
- Paths: web/src/auth/{server.rs,session.rs}
- Severity: 1M/4m/4n
- F1: atomic UPDATE, compare with > not >= (original rec off-by-one).
  F6: only the windowed counter is sound (reset-on-rotation under-counts).

### WP-35 auth edge semantics and fail-open - READY (D-12 + D-14 answered: A, MODIFIED - no session expiry; email change requires re-verification)
- Scope (6): ws F2, ws F4, ws F7, ws F8, ws F11, ws F16
- Paths: web/src/auth/server.rs, web/src/crypto.rs, web/src/main.rs, web/src/db.rs
- Severity: 2M/4m
- Email squatting, enumeration, Turnstile/encryption-key fail-open, token
  expiry. F4's uniform-reject alternative would lock out existing verified
  users - inferior option.

### WP-36 crypto and deploy hardening - READY
- Scope (4): ws F17, ws F52, ws F54, ws F55
- Paths: web/src/crypto.rs, web/src/auth/session.rs, web/src/main.rs, web/src/websocket.rs, k8s/base/web/deployment.yaml, web/.env.template
- Severity: 1M/2m/1n
- F52: prefer default-secure-in-code with explicit dev opt-out (setting
  SECURE_COOKIE in k8s base would force Secure on the dev overlay).

### WP-37 admin.rs pass - READY
- Scope (14): ws F18, ws F19, ws F20, ws F21, ws F22, ws F23, ws F24, ws F25, ws F26, ws F28, ws F29, ws F31, ws F32, ws F33
- Paths: web/src/admin.rs
- Severity: 11m/3n
- Do the F28 admin-gate dedup first; the rest gets cleaner. NOTE: rejected
  ws F30 - bot_providers has NO updated_at column; do not add it to
  update_bot_provider.

### WP-38 bot-turn wedge recovery - READY (D-5 answered: C-lite, MODIFIED - bots stay by NAME, dangling names no-op)
- Scope (6): ws F27, wd F1, wd F2, wd F3, wd F5, bo F2
- Paths: web/src/game/mod.rs, web/src/admin.rs, web/src/nats.rs, bot/src/main.rs
- Severity: 5M/1m
- The "game permanently wedged" family: UserError ack, retry exhaustion,
  publish-after-commit loss, bot-by-name rename deadlock, ack-heartbeat.
  One recovery design (D-5) covers all of it.

### WP-39 bot consumer supervision mechanical - READY
- Scope (10): ws F53, ws F56, ws F57, ws F58, wd F4, wd F9, bo F1, bo F3, bo F5, bo F8
- Paths: web/src/main.rs, web/src/game/mod.rs, web/src/nats.rs, bot/src/main.rs
- Severity: 3M/4m/3n
- Consumer restart supervision, reachable unreachable!() in the retry
  loop, graceful shutdown, max_deliver stranding visibility. Independent
  of the D-5 recovery design; do not wait on WP-38.

### WP-40 undo/concede TOCTOU + ratings integrity - READY (D-3 + D-4 answered: option A, no rating rewind)
- Scope (8): wd F14, wd F15, wd F16, wfe F19, wfe F20, wfe F22, ws F34, ws F38
- Paths: web/src/db.rs, web/src/game/server_fns.rs, web/src/email/commands.rs
- Severity: 1c/6M/1m
- One shared root cause: db::undo_game/db::concede_game lack the move
  path's guards. Fix once in db.rs; extract concede_core/undo_core so the
  email path stops duplicating server fns (wfe F22). D-3 settles
  undo-vs-ratings semantics first. NOTE: ws F34's "recompute on next
  finish" alone double-counts ratings - rewind via stored deltas.

### WP-41 db.rs quality pass - READY
- Scope (16): ws F35, ws F36, ws F37, ws F39, ws F40, ws F41, ws F42, ws F43, ws F44, ws F45, ws F46, ws F47, ws F48, ws F49, ws F50, ws F51
- Paths: web/src/db.rs
- Severity: 1M/7m/8n
- NOTE: F36 sweep must EXCLUDE lines :1357/:1363 (game_proposals has no
  trigger); F48's silent-Ok breaks the Err assertion at db.rs:3317 -
  change the test with it; F35 list minus choose_colors/ELO (tested).

### WP-42 realtime visibility predicates and per-connection filter cache - READY, **RESCOPED 2026-07-26 by D-44** (spec: `specs/WP-42-websocket-auth-and-filtering.md` - filename kept so cross-references resolve; it is no longer a websocket auth package)
- Scope (4): ws F59 (predicate half only), ws F60, ws F61, ws F62
- Paths: web/src/db.rs (post-WP-82: `web/src/db/`), plus one new reusable
  visibility-cache module. **No longer touches `web/src/websocket.rs`,
  `web/src/router.rs` or `web/src/websocket_client.rs`** - those are WP-84's.
- Severity: 2m/2n
- The READY-PENDING-CONFIRMATION flag is cleared: the user confirmed the intent
  is to gate the feed, not accept the firehose.
- **D-44 pivot (2026-07-26): the transport half of this package is SUPERSEDED by
  WP-84 (SSE migration).** What remains is transport-independent:
  - **SUPERSEDED - do not build:** the `ws_handler` pre-upgrade auth dance
    (`Session` + `State<PgPool>` extractors resolved before `ws.on_upgrade`).
    It exists only because the 101 upgrade hijacks the connection. On SSE it
    collapses to ordinary extractors - see `specs/WP-84-sse-migration.md` §3c.
    Building it here means building it twice.
  - **SURVIVES - now the whole package:** add
    `is_proposal_visible_to_user(pool, proposal_id, viewer_id)` to `db.rs`
    (one `EXISTS` over `game_proposal_players`); consume WP-47's
    `is_game_visible_to_viewer` for game frames (do not fork it); build the
    bounded per-connection TTL cache (~256 entries, 30s TTL, positive and
    negative cached alike, **fail closed** on `sqlx` error and do not cache the
    error). Accepted staleness <=30s either direction. WP-84 §3d wires all of
    it into `/events`.
  - **Task B (`sub`/`unsub`) is ELIMINATED - never build it.** There is no
    client->server channel under SSE; subscription scope lives in the URL. The
    side-channel alternative was considered and rejected in
    `sse-topology-decision.md` §4.
- **`/ws` stays unfiltered until WP-84's client switch.** The filter cannot be
  wired into `handle_socket` without the superseded auth dance, so WP-42 makes
  no `websocket.rs` edit. Accepted risk: skinny UUID payloads, existence and
  timing only, status quo rather than a new regression. Flag it if WP-84 slips.
- Still fixes a load bug, not just privacy: `trigger.last_update` is bumped on
  EVERY frame and keys `active_games` and `public_index`, so today every
  site-wide event forces a server-fn refetch on every connected client. The fix
  now lands with WP-84's per-frame filter.
- Filtering approach (D-13, unchanged and carried into WP-84): wildcard
  subscribe + per-connection filter, NOT per-user NATS fan-out subjects (which
  would require all publish sites to learn the recipient set). The `user.>`
  stay-empty assertions in `websocket.rs` must keep holding.
- **Tests:** unit-test the new predicate at the `db` level and the cache as a
  pure module; write NO new tests against `tests/websocket_hygiene.rs`, which
  WP-84 §7 deletes. End-to-end filtering assertions are deferred to WP-84 §8's
  SSE tests (renumbered from §9 when the single-stream fallback was deleted).
  Rationale in the spec §5.
- **Landing order: WP-82 -> WP-47 -> WP-42 -> WP-84.** See `landing-order.md`
  section 10.
- F60-F62 are mechanical and independent of all of the above; re-check them
  against WP-84's deletion list before doing them, as some may become moot.

### WP-43 web cargo deps - READY
- Scope (5): ws F63, ws F64, ws F65, ws F66, ws F67
- Paths: web/Cargo.toml, web/src/bin/import_game.rs
- Severity: 2m/3n
- F66: naive optional futures-util breaks non-ssr cargo test; needs
  dev-dep or required-features.

## web-domain and web-frontend-email

### WP-44 proposals integrity and email_token leak - READY
- Scope (11): wd F26, wd F29, wd F30, wd F31, wd F35, wd F36, wd F40, wd F41, wd F42, wd F43, wd F44
- Paths: web/src/proposals.rs
- Severity: 1M/5m/5n
- wd F26 (email_token serialized to all proposal viewers) is the takeover
  enabler composing with wfe's forgeable From - land it immediately,
  independent of the D-1 redesign. Owner-decline wedge, ownership
  transfer, authz dedup.

### WP-45 bot-slot validation choke point - READY (D-8 answered: option C, reconciled with D-5 - validate on write, tolerate on read)
- Scope (2): wd F27, wfe F18
- Paths: web/src/proposals.rs, web/src/game/server_fns.rs, web/src/db.rs, web/src/email/commands.rs
- Severity: 2M
- Four entry points, one shared validation. Consequence amplified by the
  wedge-recovery gap (WP-38).

### WP-46 sweep delivery semantics - READY (D-11 answered 2026-07-26: option A; D-2 answered: option A, at-least-once, do not mark `sent` on skip paths)
- Scope (12): wd F28, wd F38, wd F39, wfe F11, wfe F30, wfe F31, wfe F32, wfe F33, wfe F34, wfe F35, wfe F37, wfe F40
- Paths: web/src/email/sweep.rs, web/src/proposals.rs, web/src/email/outbound.rs
- Severity: 3M/8m/1n
- Mark-before-do in every sweep, SKIP LOCKED no-op under autocommit,
  auto-decline keyed on wrong timestamp, reminder-pref flag choice.
- **D-11 ANSWERED: option A.** `reminder_emails_enabled` **alone** governs
  reminder emails; `turn_emails_enabled` governs turn notifications only. The
  reminder sweep must **not** consult `turn_emails_enabled`. User's rationale
  (the design intent to preserve): some users play mainly by web and do not
  want turn emails, but reminders are still useful if they have **missed or
  forgotten** a game.

### WP-47 game_visibility gates - READY (D-6 + D-13 answered: option A, anonymize private users in stats)
- Scope (2): wd F17, wd F45
- Paths: web/src/game/server_fns.rs, web/src/stats/{mod.rs,queries.rs}, web/src/db.rs
- Severity: 2M
- Wire is_game_visible_to_user into game details + stats once D-6 fixes
  the scope; one shared predicate for both.
- **WP-42 and WP-84 consume this predicate too** (D-13 refinement 2026-07-25,
  updated for the D-44 SSE pivot 2026-07-26): the realtime per-frame filter
  calls WP-47's `is_game_visible_to_viewer` dispatcher - the `Option<Uuid>`
  thin wrapper this package adds over `is_game_visible_to_user` - not a
  reimplementation. Both take a `PgPool` and a `Uuid`, no leptos context, no
  `get_current_user`. **WP-47 must land before WP-42**, which must land before
  WP-84; see `landing-order.md` section 10.

### WP-48 export/import - READY (D-7 answered 2026-07-26: OVERRULED) - **SCOPE SHRANK**
- Scope (5): wd F7, wd F10, wd F11, wd F12, wd F13
- Paths: web/src/game/{export.rs,import.rs}
- Severity: 1m/4n
- Privacy posture (D-7) gates F7; the import nits are mechanical riders.
- **SCOPE SHRANK 2026-07-26.** D-7 was **OVERRULED**: do **not** build a
  redacted user-facing export at all. **The only export path is the full
  bundle, admin-only.**
  - **OUT of scope:** the `--redact-private` flag; the user-facing export
    path. Both were in the superseded option-A recommendation. Do not build
    them.
  - **IN scope for wd F7:** make the export **admin-only**. That is the whole
    of the privacy work now.
  - Bug reporting is by **game ID**. The user **explicitly accepts the risk**
    that game state may change after a report is filed and render it useless -
    that is not a defect to design around.
  - wd F10-F13 (import nits) are unchanged mechanical riders.

### WP-49 rules and game-info pages - READY (D-6 answered: rules pages stay public)
- Scope (8): wd F67, wd F68, wd F69, wd F70, wd F71, wd F76, wd F79, wd F80
- Paths: web/src/{rules.rs,game_info/mod.rs,game_info/queries.rs,db.rs}
- Severity: 1M/4m/3n
- F67 (version picked by ORDER BY name) is mechanical and can land ahead
  of the public-content posture call that gates F68/F80.

### WP-50 email canonicalization - READY (D-9 answered 2026-07-26: option B)
- Scope (4): ws F9, wd F37, wd F60, wd F72
- Paths: web/src/auth/server.rs, web/src/proposals.rs, web/src/new_game.rs, web/src/settings.rs, web/src/db.rs
- Severity: 4m
- One policy applied at all four boundaries.
- **D-9 ANSWERED: option B.** Trim + lowercase at all four input boundaries,
  **plus** the one-off migration lowercasing stored rows, **plus** the
  lower-index (or citext) unique constraint. Boundary-only would leave
  existing mixed-case rows permanently unmatchable. Surface the case-collision
  risk once, deliberately, during the migration.

### WP-51 invite-mailer and notify dedup - READY
- Scope (10): wd F8, wd F32, wd F33, wd F34, wfe F36, wfe F38, wfe F39, wfe F41, wfe F42, wfe F43
- Paths: web/src/proposals.rs, web/src/email/{sweep.rs,notify.rs}, web/src/game/{mod.rs,server_fns.rs}
- Severity: 7m/3n
- send_reminder's ~90-line duplication of notify::send_one (already
  drifted), notify gating bypass, before=None re-notification, N+1 sends.

### WP-52 stats and query performance pass - READY
- Scope (13): wd F21, wd F46, wd F47, wd F48, wd F49, wd F50, wd F51, wd F52, wd F53, wd F55, wd F62, wd F74, wd F75
- Paths: web/src/stats/{mod.rs,queries.rs}, web/src/{friends.rs,index.rs,game_info/mod.rs,players.rs,db.rs}
- Severity: 9m/4n
- N+1s, unbounded public queries, page-offset clamp, eligibility-predicate
  dedup.

### WP-53 domain misc server fns - READY
- Scope (14): wd F6, wd F18, wd F19, wd F20, wd F22, wd F23, wd F24, wd F25, wd F54, wd F56, wd F61, wd F65, wd F77, wd F78
- Paths: web/src/game/{mod.rs,server_fns.rs}, web/src/{friends.rs,players.rs,settings.rs,models/game.rs}, web/src/db.rs, web/src/stats/viz.rs
- Severity: 5m/9n
- is_eliminated wipe, HTTP call inside FOR UPDATE tx, cross friend-request
  race, authz hygiene nits.

### WP-54 frontend UX error handling - READY
- Scope (17): wd F57, wd F58, wd F59, wd F63, wd F64, wd F66, wd F73, wfe F52, wfe F54, wfe F55, wfe F56, wfe F57, wfe F58, wfe F59, wfe F60, wfe F61, wfe F62
- Paths: web/src/{friends.rs,new_game.rs,settings.rs,app.rs}, web/src/components/{game.rs,layout.rs,opponent_slot.rs,mod.rs}
- Severity: 1M/10m/6n
- Fire-and-forget ServerAction values never observed (shared error-slot
  pattern already exists in GameCommandInput/UsernameSection); SPA latch
  resets; a11y nits.

### WP-55 Turnstile SPA rendering - READY (D-16 answered 2026-07-26: option B, OVERRULING the recommendation) - **SCOPE GREW**
- Scope (1): wfe F53
- Paths: web/src/app.rs, web/src/components/layout.rs, web/src/settings.rs, web/src/admin.rs
- Severity: 1M
- **D-16 OVERRULED to option B.** Do **NOT** call Turnstile's `render()` from
  a login-component effect (the superseded option A). Instead make `/login` a
  **normal, unrouted link that forces a full page load**, so Turnstile's
  automatic rendering just works. User's reasons: complexity concern, and the
  login page should load very fast.
- **Mechanism VERIFIED 2026-07-26 (read-only) against the version in the
  tree** - `leptos = "0.8.20"`, `leptos_router = "0.8.14"` (lock-resolved):
  - `rel="external"` **IS honoured**. `leptos_router-0.8.14`'s
    `src/location/mod.rs` reads the DOM `rel` attribute, splits on space/tab,
    and returns early for an `external` token (or a `download` attribute).
  - **A plain `<a>` is NOT sufficient on its own.** Interception is a
    **window-level** click listener (`src/location/history.rs`) walking
    `composed_path()` for any `HtmlAnchorElement`; it does not distinguish
    `<A>` from a literal `<a>`. `rel="external"` is required either way.
  - `<A>` has **no `rel` prop**. Use `attr:rel="external"` on `<A>`
    (attribute spreading on `<A>` is already proven in this codebase - see the
    `attr:class` on the `/login` link in `app.rs`), or a plain
    `<a href="/login" rel="external">`. The plain anchor is simplest; `<A>`'s
    only extra behaviour is `aria-current` marking, irrelevant here.
  - Both current `/login` links are `<A>` and are client-side routed today:
    the `index-cta` "Start a game" link in `app.rs` and the "Login" nav link
    in `components/layout.rs`.
- **SCOPE GREW - a gap `rel` cannot cover.** Three navigations to `/login` go
  through `use_navigate`, which never touches an anchor and is therefore never
  subject to the `rel` check: **`components/layout.rs`** (post-logout),
  **`settings.rs`** (anonymous redirect) and **`admin.rs`** (anonymous
  redirect). WP-55 must convert these to hard navigations (a location
  assignment) as well, or Turnstile will still fail to render for users
  arriving at `/login` by those paths.

### WP-56 email From-auth redesign - READY (D-1 answered: option B - s- token + SPF/DKIM + remove account-security commands)
- Scope (3): wfe F1, wfe F5, wfe F17
- Paths: web/src/email/{inbound.rs,commands.rs}
- Severity: 2c/1M
- Both remaining criticals (account takeover). Highest-priority decision.

### WP-57 inbound webhook delivery semantics - READY (D-2 answered: option A, at-least-once)
- Scope (3): wfe F2, wfe F10, wfe F16
- Paths: web/src/email/inbound.rs
- Severity: 1M/1m/1n
- Marker-after-success + 5xx-for-retry + enqueue vs inline (svix 15s).

### WP-58 unsubscribe RFC 8058 - READY (D-10 answered 2026-07-26: option A **plus an addition**) - **SCOPE GREW**
- Scope (2): wfe F3, wfe F25
- Paths: web/src/email/{inbound.rs,render.rs,commands.rs}
- Severity: 1M/1m
- **D-10 ANSWERED: option A - build the HTTPS one-click unsubscribe endpoint**
  (tokenised, no auth redirect). Driver is Gmail/Yahoo bulk-sender
  deliverability for a turn-notification product.
- **SCOPE GREW - the mail must also carry TWO VISIBLE LINKS:**
  1. A **type-specific** unsubscribe link matching the email type actually
     received - e.g. "Unsubscribe from game reminders" on a reminder mail.
  2. A **"Manage my subscriptions"** link to the user settings page.
  The `List-Unsubscribe` / `List-Unsubscribe-Post` headers **still point at
  the one-click endpoint**; the visible links are **additional, not a
  replacement** for the headers.
- Also fix the help text that advertises subscribe/unsubscribe verbs the
  standalone dispatch currently rejects.

### WP-59 inbound processing quality - READY
- Scope (16): wfe F4, wfe F6, wfe F7, wfe F8, wfe F9, wfe F12, wfe F13, wfe F14, wfe F15, wfe F21, wfe F23, wfe F24, wfe F26, wfe F27, wfe F28, wfe F29
- Paths: web/src/email/{inbound.rs,commands.rs,notify.rs,render.rs}, web/src/db.rs
- Severity: 2M/7m/7n
- Display-name address parsing (F4), internal errors emailed verbatim
  (F21), reply parsing, plumbing dedup. F29 follows D-15's outcome
  (recommended: document the verb reservation).
- **D-15 ANSWERED 2026-07-26 - Task 14 is UNGATED but its CONTENT CHANGES.**
  The ruling is a redesign, not the "A-plus" reserved-list option:
  **do NOT hardcode a reserved-verb list.** On game-scoped messages, **try the
  game command parser FIRST; platform commands are the FALLBACK** when the
  game parser fails. **One carve-out:** a **small hard-reserved set of
  escape-hatch verbs** (`help` and equivalents) **always wins**, even on the
  game path, so a game with a greedy parser cannot swallow the only command
  that unsticks a user. Keep that set small and obvious.
- Consequence: **do not execute Task 14 as specced.** Rewrite the COMMANDS.md
  section to describe **parser-first dispatch plus the escape-hatch set**,
  not a "Reserved verbs on the email path" reservation. This also fixes the
  live defect where acquire-1 and starship-catan-1 players cannot issue `end`
  by email, because the game parser is now consulted before
  `"end" => run_end(ctx)` in `web/src/email/commands.rs`. `wfe F29` follows
  this outcome.
- **NOTE (2026-07-25, T2-B3 Lead): WP-57 widens `fetch_inbound_text`'s return
  shape.** WP-59 lands BEFORE WP-57, so WP-57 absorbs the change - but any
  WP-59 work assuming the current return type must be re-checked when WP-57
  lands on top.

### WP-60 outbound tokens, metrics, render - READY
- Scope (9): wfe F44, wfe F45, wfe F46, wfe F47, wfe F48, wfe F49, wfe F50, wfe F51, wfe F63
- Paths: web/src/email/{outbound.rs,render.rs,sweep.rs}, web/src/{theme.rs,app.rs}
- Severity: 5m/4n
- ensure_email_token races (one atomic UPDATE..RETURNING rewrite covers
  F44+F45), metric-before-send, silent mrml fallback.

## Bot, operator, tools

### WP-61 bot service quality - READY
- Scope (12): bo F4, bo F6, bo F7, bo F9, bo F10, bo F11, bo F12, bo F13, bo F14, bo F15, bo F16, dp F7
- Paths: bot/src/{main.rs,config.rs,crypto.rs,prompt.rs,routing.rs}, bot/Cargo.toml, bot/user_prompt.md
- Severity: 8m/4n
- bo F12 (aes-gcm generate_nonce) also resolves dp F7 (getrandom drift) -
  one change.

### WP-62 operator - READY
- Scope (8): bo F18, bo F19, bo F20, bo F21, bo F22, bo F23, bo F24, bo F25
- Paths: operator/src/{controller.rs,crd.rs}, operator/Cargo.toml
- Severity: 1M/4m/3n
- Finalizer merge-patch race (use a typed status patch / server-side
  apply); bo F25's k8s-openapi pin needs the deployed cluster version -
  confirm with Michael at spec time.
- **`bo F25` ANSWERED 2026-07-26.** The deployed cluster runs **Kubernetes
  server v1.36.0** (client v1.36.2, kustomize v5.8.1). **Pin `k8s-openapi` to
  the `v1_36` feature.** **The implementer must confirm `k8s-openapi` actually
  ships a `v1_36` feature flag at fix time**; if it does not, use the highest
  available flag at or below v1.36 and **record the choice** in the WP-62
  spec. The rider no longer blocks anything.

### WP-63 fuzz tool - READY
- Scope (7): bo F26, bo F27, bo F28, bo F29, bo F30, bo F31, dp F20
- Paths: tools/fuzz/{src/lib.rs,Cargo.toml}
- Severity: 1M/2m/4n
- Hang-forever channel bug plus riders; dp F20 = bo F28 (num_cpus).

## Dependencies and build

> **STANDING PROCESS CHANGE (2026-07-26, from the D-17 answer) - binding on
> EVERY package in this section, not just WP-66.**
>
> Michael's strategy is to stay **as close to latest dependencies as possible
> so they never go stale.** So for this and **any similar dependency problem**
> the **first** step is: **"upgrade all dependencies to latest and see where
> we stand."** The problem may simply resolve. Only if it does **not** should
> the recorded workaround (vendoring, pinning, feature-juggling) be taken.
> Record what the upgrade changed.

### WP-64 workspace-deps migration - READY (D-19 answered 2026-07-26: option A, all three tables)
- Scope (3): dp F1, dp F2, dp F3
- Paths: Cargo.toml + all 40 crate manifests
- Severity: 1M/2m
- The umbrella: do early so every later version unification is a one-line
  root edit. [workspace.dependencies] + [workspace.package] + [workspace.lints]
  in one migration PR.
- **D-19 ANSWERED: option A** - all three tables in one migration, early.
  Also resolves the `dp F9` version-pin row in the T3-B8 checklist. Sequence
  per the section banner above: upgrade everything to latest first, then
  migrate.

### WP-65 workspace hygiene - READY
- Scope (9): dp F4, dp F5, dp F9, dp F17, dp F21, dp F22, dp F23, e F9, e F28
- Paths: Cargo.toml, web/Cargo.toml, lib/cmd/Cargo.toml, lib/color/Cargo.toml, game/lords-of-vegas-1/Cargo.toml, game/lost-cities-2/{build-release,.rls.toml} (+ acquire-1, lords-of-vegas-1 equivalents), CI config
- Severity: 3m/6n
- Best after WP-64. lazy_static->LazyLock, stale template files, deps CI
  job, test-module naming convention sweep (e F9).

### WP-66 sqlx unification - READY (D-17 answered 2026-07-26: upgrade first, vendor only if needed)
- Scope (3): dp F6, dp F8, dp F19
- Paths: web/Cargo.toml, bot/Cargo.toml, operator/Cargo.toml, Cargo.lock
- Severity: 1M/2m
- Also collapses part of the rand/lock duplicate clusters; re-audit lock
  after (F8/F19 are monitor items).
- **D-17 ANSWERED, with an explicit FIRST STEP before either option:**
  (1) **upgrade all dependencies to latest and see where we stand** - the sqlx
  0.8/0.9 split may simply resolve; (2) only if it does **not**, vendor the
  `tower-sessions-sqlx-store` (option B) and move everything to 0.9. This is
  the standing process change recorded in the section banner above, not a
  one-off for this package.

### WP-67 sentry feature trim - READY (D-18 answered 2026-07-26: yes, with a hard no-functionality-loss constraint)
- Scope (1): dp F12
- Paths: bot/Cargo.toml, web/Cargo.toml, lib/cmd/Cargo.toml, lib/game_client/Cargo.toml
- Severity: 1M
- Verify actix/ureq drop via cargo tree; preserve the deliberate
  native-tls transport. Do before re-auditing the lock (makes svix the
  sole http-0.2 holdout).
- **D-18 ANSWERED: yes**, trim to explicit features (backtrace, contexts,
  panic, tracing/tower as used, native-tls transport), verified with
  `cargo tree`.
- **STANDING CONSTRAINT: it is CRITICAL that no Sentry functionality is
  lost.** The trim must be verified to **preserve current behaviour**, not
  merely to shrink the dependency tree. Enumerate the sentry features actually
  in use before removing any, and confirm the trimmed build still reports what
  it reports today. This is an acceptance criterion, not a nicety.

### WP-68 term_size replacement - READY
- Scope (2): dp F13, ls F24
- Paths: lib/cmd/Cargo.toml + call sites, deny.toml
- Severity: 1M/1m
- Drop-in terminal_size swap (RUSTSEC-2020-0163); clears one deny ignore.

### WP-69 deny.toml hardening - READY (D-23 answered 2026-07-26: yes, in exactly that order)
- Scope (3): dp F18, dp F24, dp F25
- Paths: deny.toml
- Severity: 3m
- Land LAST among dep packages: warn->deny only after WP-66/67/68 shrink
  the duplicate set, so the skip-list starts minimal.
- **D-23 ANSWERED: yes, in exactly that order.** Clear the 4 stale advisory
  ignores **now**; flip `multiple-versions` to deny only **after** WP-66/67/68
  land, with the residual duplicates enumerated in skip/skip-tree.

### WP-70 serde_yaml migration - READY (D-21 answered 2026-07-26: option A, `serde_yaml_ng`)
- Scope (3): dp F14, bo F17, ls F34
- Paths: lib/game_client/Cargo.toml, bot/Cargo.toml, bot/src/prompt.rs
- Severity: 3m
- Both consumers must move together.
- **D-21 ANSWERED: option A - `serde_yaml_ng`.** Drop-in API, maintained. Not
  JSON: that would change a file format ops and users may depend on.

### WP-71 warp -> axum consolidation - READY (D-22 answered 2026-07-26: port now)
- Scope (2): dp F16, ls F25
- Paths: lib/cmd/{Cargo.toml,src/http.rs}
- Severity: 2m
- Small surface but the HTTP layer of all 28 game binaries; also the
  natural moment for ls F19/F28 (WP-06) if sequenced together.
- **D-22 ANSWERED: port now**, in the same window as WP-06's `http.rs` fixes,
  so the surface is touched once.

### WP-72 combine posture - READY (D-24 answered 2026-07-26: option A, accept as recorded risk)
- Scope (1): dp F15
- Paths: lib/game/Cargo.toml, lib/markup/Cargo.toml
- Severity: 1m
- **D-24 ANSWERED: option A.** Accept `combine` 4.6 as a recorded risk, note
  it in `deny.toml`, and migrate markup off it only when the parser is next
  rewritten. WP-02 already changes markup enough for one release, and combine
  carries no advisory today.

### WP-73 game-bins consolidation - READY (D-20 answered 2026-07-26: option B, generic bin crate - NOT the macro; D-41/D-42/D-43 answered 2026-07-26)
- Scope (4): dp F11, dp F26, e F45, e F46
- Paths: all 27 game crates' Cargo.toml + src/bin/*, the new
  `rust/lib/game_bin`, `docs/porting/GAME_PORTING.md`
- Severity: 3m/1n
- e F45's [dev-dependencies] recommendation is INVALID (dev-deps do not
  apply to src/bin targets). Macro vs generic bin crate is the D-20 call;
  determines where tokio/fuzz deps live. Sequence after WP-64.
- **D-20 ANSWERED: option B - one generic bin crate parameterised over the
  `Gamer` trait, with thin per-game wrapper bin crates. EXPLICITLY NOT option
  A (the macro).** Michael approved B **partly because it avoids macros** - do
  not "simplify" it back into one.
- **STANDING CONSTRAINT on macros, wider than this package:** keep any macro
  surface **small and obvious**, and **pause and discuss** if a macro starts
  getting really complex. Maintenance and cognitive cost are the concern.
- **Concrete name, VERIFIED against the repo's layout (read-only,
  2026-07-26): `rust/lib/game_bin`, `[package] name = "brdgme_game_bin"`.**
  The convention is snake_case dirs under `lib/`/`tools/` with package names
  `brdgme_<snake_dir>` - consistent across all ten (`lib/cmd` ->
  `brdgme_cmd`, `lib/game_client` -> `brdgme_game_client`, `tools/fuzz` ->
  `brdgme_fuzz`). Hyphens are the **game-crate** convention
  (`game/red7-1` -> `red7-1`); `brdgme-operator` is the lone hyphenated
  outlier and is not under `lib/`. **Do NOT use `game-bin` /
  `brdgme-game-bin`.**
- **Structural note, CORRECTED by the spec (read-only verification):** the
  bins are **auto-discovered** from `src/bin/<snake>_{cli,fuzz,http,repl}.rs`
  inside each game crate - there are **no `[[bin]]` stanzas** in any game
  manifest. The spec deliberately does **not** create per-game bin crates: the
  wrappers stay as `src/bin/` files, so `[workspace] members` gains exactly one
  entry (`lib/game_bin`) and no Dockerfile/bake/Tilt/k8s change is needed.
  Bin **filenames are frozen** - `_http` is copied by flat filename by
  `rust/Dockerfile`, and `_cli` is what `tools/fuzz`/`tools/repl` are pointed at
  by path.
- **D-41 ANSWERED, then D-43 SUPERSEDED its fuzz half: delete the per-game
  `_repl` bins ONLY.** Dependency sweep
  (`planning/raw/wp73-fuzz-repl-dependency-sweep.md`) came back CLEAN - nothing
  in the repo invokes them. Scope grows by **27 file deletions**
  (`rust/game/*/src/bin/<snake>_repl.rs`), the `brdgme_fuzz` dep drops from all
  27 game manifests (it moves up to `game_bin`, not unused), and
  `docs/porting/GAME_PORTING.md` swaps the repl invocation for the generic tool
  while KEEPING its fuzz-bin references.
- **D-43 ANSWERED: the 27 `_fuzz` bins and `fuzz_gamer` SURVIVE.** Fuzzing is
  selected on raw throughput (`planning/fuzz-throughput-evaluation.md`,
  Lead-ACCEPTED): out-of-process costs a process spawn per move plus a second
  JSON layer. `rust/tools/fuzz/src/lib.rs` is **not touched** by WP-73.
- **Consequence: `brdgme_game_bin` has THREE entry points, not four** -
  `cli_main`, `http_main` and `fuzz_main` (`repl_main` stays dropped).
  `brdgme_fuzz` **IS** a dep of `game_bin`, which `fuzz_main` calls through; the
  `_cli`/`_http` bins therefore transitively link its subtree.
- **D-42 ANSWERED: `lords-of-vegas-1` gets the same treatment**, but stays
  undeployed - **do not** add it to Dockerfile/docker-bake.hcl/Tiltfile/k8s.
  Only the bin consolidation applies.

## Documentation packages filed at spec time

These two packages carry NO review finding IDs. They were discovered while
writing `specs/WP-29-red7-cleanup.md` (its "Cross-package / newly discovered"
items 1 and 2) and filed by the unit 4d Lead. They are therefore excluded from
the finding-count coverage table below by construction - see the note under it.

### WP-74 red7-1 empty-hand-elimination rules documentation - READY
- Scope (0 findings): new defect, WP-29 spec cross-package item 1
  (`specs/WP-29-red7-cleanup.md:464`)
- Paths: rust/game/red7-1/RULES.md
- Severity: 1m (bot-facing rules incompleteness)
- `start_turn` eliminates the current player "for not having any cards left"
  when their hand is empty (`rust/game/red7-1/src/lib.rs:146-152`), and that
  elimination can immediately end the round via `end_turn` (`:150`). No sentence
  in `RULES.md` mentions it, so a bot playing from the rules alone cannot
  predict its own elimination. Explicitly NOT in e F32's scope (which names the
  turn-option list and the scoring line), so WP-29 Task 5 must not absorb it.
- Sequence AFTER WP-29 Task 5 and AFTER WP-30 (both rewrite RULES.md sections;
  WP-30 is still parked on D-29 - its D-40 half moved to WP-81 on 2026-07-26 -
  and D-29's outcome may change how elimination is described). Small enough to
  fold into whichever of those lands last if the implementer prefers.
- **2026-07-26 caveat:** WP-30's `e F30` seat-order fix is now released from
  the park, but the D-29 question it sits next to is not, so this doc package
  should still wait on D-29 rather than on all of WP-30.

### WP-75 red7-1 RULES.md RULES_AUTHORING.md compliance - READY
- Scope (0 findings): new defect, WP-29 spec cross-package item 2
  (`specs/WP-29-red7-cleanup.md:465`)
- Paths: rust/game/red7-1/RULES.md (+ read `docs/authoring/RULES_AUTHORING.md`,
  `rust/game/red7-1/{BASIC_STRATEGY.md,ADVANCED_STRATEGY.md}`)
- Severity: 1m (bot-facing rules incompleteness)
- Against `RULES_AUTHORING.md:13-107`'s required-sections list, red7-1's
  55-line `RULES.md` is missing outright: Cards / Components; Rounds / Game End;
  Winning; Reading the Display (which `RULES_AUTHORING.md:44` calls "critical
  for the bot"); Strategy Tips. Scoring is missing the worked example mandated
  by `RULES_AUTHORING.md:30-36`.
- **N-4 ANSWERED 2026-07-26 - the ruling below is GIVEN; this is no longer a
  blocker.** The shipped `BASIC_STRATEGY.md` / `ADVANCED_STRATEGY.md`,
  surfaced via `Gamer::basic_strategy` / `advanced_strategy`, **DO satisfy**
  `RULES_AUTHORING.md`'s mandatory "Strategy Tips" section. Amend
  `RULES_AUTHORING.md` to say so.
  **The amendment must state the RATIONALE, not merely grant the permission:**
  the two files are deliberately separate **so bot difficulty can be tiered** -
  every bot gets BASIC to stop it making game-throwing moves, and only hard
  bots also get ADVANCED. **They must NOT be folded into `RULES.md`.**
  Duplicating them into every RULES.md would guarantee drift and would break
  the tiering.
- (Historical - now answered by N-4.) REQUIRES A LEAD/USER RULING before a spec can be written: whether the shipped
  `BASIC_STRATEGY.md` / `ADVANCED_STRATEGY.md` (surfaced via
  `Gamer::basic_strategy`/`advanced_strategy`, `lib.rs:540-546`) satisfy the
  Strategy Tips requirement, given `RULES_AUTHORING.md:100-107` says "Always
  include this section".
- REQUIRES A LIVE ARTEFACT: the Reading the Display section needs a real render
  pulled from a live game state (extraction recipe at
  `RULES_AUTHORING.md:56-64`), which needs a DB and a built binary. A read-only
  spec cannot produce it, so the spec must instruct the implementer to capture
  it. Marked READY because no decision item gates it, but it is NOT
  spec-writable from source reading alone.
- Sequence AFTER WP-29 Task 5, WP-30 and WP-74 - it rewrites the whole document
  and would otherwise churn their diffs.
- Consider auditing the other 26 game crates' RULES.md against the same
  checklist; red7-1 is unlikely to be the only offender. Out of scope here.

## Coverage check

Per-package counts:

| WP | n | WP | n | WP | n | WP | n |
|----|---|----|---|----|---|----|---|
| 01 | 7 | 20 | 5 | 39 | 10 | 58 | 2 |
| 02 | 10 | 21 | 12 | 40 | 8 | 59 | 16 |
| 03 | 11 | 22 | 10 | 41 | 16 | 60 | 9 |
| 04 | 5 | 23 | 6 | 42 | 4 | 61 | 12 |
| 05 | 7 | 24 | 7 | 43 | 5 | 62 | 8 |
| 06 | 12 | 25 | 9 | 44 | 11 | 63 | 7 |
| 07 | 11 | 26 | 9 | 45 | 2 | 64 | 3 |
| 08 | 12 | 27 | 8 | 46 | 12 | 65 | 9 |
| 09 | 19 | 28 | 13 | 47 | 2 | 66 | 3 |
| 10 | 2 | 29 | 5 | 48 | 5 | 67 | 1 |
| 11 | 8 | 30 | 5 | 49 | 8 | 68 | 2 |
| 12 | 9 | 31 | 7 | 50 | 4 | 69 | 3 |
| 13 | 10 | 32 | 12 | 51 | 10 | 70 | 3 |
| 14 | 10 | 33 | 17 | 52 | 13 | 71 | 2 |
| 15 | 9 | 34 | 9 | 53 | 14 | 72 | 1 |
| 16 | 8 | 35 | 6 | 54 | 17 | 73 | 4 |
| 17 | 8 | 36 | 4 | 55 | 1 | | |
| 18 | 4 | 37 | 14 | 56 | 3 | | |
| 19 | 11 | 38 | 6 | 57 | 3 | | |

Sum = 570. Every finding appears in exactly one package (per-unit checks:
lib-game 20, lib-support 45, batch-a 20, batch-b 35, batch-c 34, batch-d 45,
batch-e 46, batch-f 58, web-server 66, web-domain 80, web-frontend-email 63,
bot-operator-tools 31, dependencies 27).

Reconciliation vs REVIEW.md's 563: the four lead-verified units' own
severity-tally sections undercount the finding headings actually present in
their bodies (web-domain 80 vs 78 stated, web-frontend-email 63 vs 60,
bot-operator-tools 31 vs 30, dependencies 27 vs 26; +7 total, verified by
the W5 ID audit and W6 heading counts in planning/raw/). This triage covers
all 570 real findings; corrected grand tally 10c/78M/257m/225n. The two
rejected findings (games-batch-d F13, web-server F30) are excluded as in
REVIEW.md.

WP-74 and WP-75 are absent from the table above BY DESIGN: they were filed at
spec time from defects discovered while writing specs, carry zero review finding
IDs, and so contribute 0 to the 570 sum. The sum, the per-unit checks and the
"every finding appears in exactly one package" invariant are all unaffected.
Any future spec-time package must be recorded the same way.

Package totals: 75 packages - 41 READY, 34 BLOCKED-ON-DECISION. (73 review
packages - 39 READY, 34 BLOCKED - plus the 2 spec-time docs packages WP-74 and
WP-75, both READY.)

**Updated after the 2026-07-25 decision session:** 75 packages - **52 READY
(51 clear + WP-42 READY-PENDING-CONFIRMATION), 23 BLOCKED-ON-DECISION.**
Flipped to READY: WP-02, WP-09, WP-10, WP-35, WP-38, WP-40, WP-45, WP-47,
WP-49, WP-56, WP-57 (11) plus WP-42 pending the D-13 label confirmation.
Still blocked but with one blocker cleared: WP-46 (D-11 only), WP-11 (D-30
only).

**Updated after the 2026-07-26 decision session - `BLOCKED-ON-DECISION` is
now EXTINCT.** Counting the review packages plus WP-74..WP-81:

- **16 packages flipped to READY:** WP-04 (D-38), WP-05 (D-39), WP-17 (D-25 -
  the `PARTIALLY-BLOCKED` label retired), WP-46 (D-11), WP-48 (D-7), WP-50
  (D-9), WP-55 (D-16), WP-58 (D-10), WP-64 (D-19), WP-66 (D-17), WP-67
  (D-18), WP-69 (D-23), WP-70 (D-21), WP-71 (D-22), WP-72 (D-24), WP-73
  (D-20).
- **1 new package:** WP-81 (D-40 stats deletions, split out of WP-20/WP-30).
- **3 scope changes:** WP-48 **SHRANK** (no redacted export, no user-facing
  path - admin-only full bundle); WP-55 **GREW** (three `use_navigate`
  redirects to `/login` must also become hard navigations); WP-58 **GREW**
  (two visible links in the mail, additional to the headers). WP-59 Task 14
  is ungated but must be **rewritten** for D-15's parser-first design.
- **WP-62's `bo F25` rider is answered** (cluster is k8s v1.36.0; pin
  `k8s-openapi` `v1_36`, confirming the flag exists at fix time).
- **6 packages remain `BLOCKED-ON-USER-RULES-REVIEW`** - WP-11, WP-12, WP-16,
  WP-20, WP-26, WP-30 - **with three carve-outs released from the park**:
  `a F1` (in WP-12), `b F7` (in WP-16) and `e F30`'s seat-order half (in
  WP-30) are all **FIX NOW** and need specs. `b F4` was re-parked with the
  user's correction; `d F37` was rejected outright as not a bug.
- WP-20's and WP-30's D-40 blockers are cleared; only their rules halves are
  parked.
- **N-item blessings** (they gate no package, but the shipped shapes are now
  confirmed): **N-1** - WP-38's 15-minute stuck-bot sweep threshold and 60s
  `AckKind::Progress` cadence accepted as defaults. **N-2** - WP-10's
  `PubState::cup_counts: Vec<(Colour, usize)>` shape accepted. **N-3** -
  WP-62's newest-non-deprecated-version-wins semantics for
  `game_types.player_counts` accepted. **N-4** - the separate
  `BASIC_STRATEGY.md`/`ADVANCED_STRATEGY.md` satisfy `RULES_AUTHORING.md`'s
  Strategy Tips requirement, which unblocks **WP-75**.

**Package totals recomputed 2026-07-26** (after WP-83 was added and WP-84
filed; the counts above are kept as history and are superseded by this block).
Counted by enumerating every `### WP-` heading in this file and reading the
status suffix off the heading itself:

- **84 headings = 77 READY + 6 parked (`BLOCKED-ON-USER-RULES-REVIEW`) + 1
  `SUPERSEDED`.**
- The 6 parked: WP-11, WP-12, WP-16, WP-20, WP-26, WP-30 (three of which carry
  carve-outs now executed by WP-83).
- The 1 superseded: WP-78 (superseded by WP-82).
- **`BLOCKED-ON-DECISION` remains EXTINCT** - zero headings carry it.
- Actionable-now count is therefore **77**; 83 packages exist as live work
  items once the superseded WP-78 is discounted.
- Findings coverage is unchanged by this recount: WP-83 and WP-84 both add 0
  to the **570** sum, so the per-package table above and the
  one-package-per-finding invariant stand.

## Unowned / newly discovered

Five items with no owning package. WP-76 and WP-77 carry no review finding IDs
(spec-time discoveries, like WP-74/WP-75, so 0 toward the 570 sum). WP-78/79/80
re-file findings already counted in WP-41 / WP-53 / WP-09's scope, so they add 0
as well; the one-package-per-finding invariant is unaffected.

### WP-76 notify_game_emails wiring gap for email-originated moves - READY
- Scope (0 findings): new defect, WP-51 spec cross-package item 1
  (`specs/WP-51-invite-mailer-notify-dedup.md:43`)
- Paths: web/src/email/{commands.rs,inbound.rs}, web/src/email/notify.rs
- Severity: 1M (functional gap: other players get no turn email)
- WP-51 explicitly refuses to absorb it; must NOT fold into WP-59 or WP-40.
  Five-line change once WP-51 Task 1 returns the pre-command snapshot.

### WP-77 get_available_bots default bot_name - READY
- Scope (0 findings): new defect, WP-54 spec cross-package item 9
  (`specs/WP-54-frontend-ux-error-handling.md:2007`)
- Paths: web/src/game/server_fns.rs, web/src/components/opponent_slot.rs
- Severity: 1m
- Nothing guarantees the hard-coded default is in the returned list; the default
  should come from the same source as the list. Nearest owner WP-53 declined.

### WP-78 db.rs module split - SUPERSEDED BY WP-82 (2026-07-26)
This entry is retained only so existing cross-references resolve
(`landing-order.md` 6.4 cites `WP-50 -> WP-78`). **Do not work from it.** The
item is now **WP-82**, owned and specced - see
`specs/WP-82-db-module-split.md`.

Two things in the old entry are now WRONG:
- It says DEFERRED. Michael escalated it on 2026-07-26; the file "is becoming
  problematic due to its size and complexity".
- **Its ordering is INVERTED.** It said the split lands *after* WP-35, WP-40,
  WP-42, WP-45, WP-47, WP-49, WP-50, WP-52, WP-53, WP-59. The split now lands
  **before** all of them, as a hard predecessor, so those packages rebase onto
  the new module layout instead of the split rebasing onto ten moved-file
  edits.

### WP-79 hoist game-service HTTP call out of the FOR UPDATE transaction - READY
- Scope (0 new findings): wd F18, counted in WP-53's scope
- Paths: web/src/game/server_fns.rs
- Severity: 1m
- `create_game_from_service` (:611) takes `&mut tx` and does the reqwest call
  plus all new-game inserts in one body, so the fix splits it into fetch and
  insert halves and touches four callers (`restart_core`, the proposal path,
  `new_game`, the email command path). COLLIDES with WP-40 and WP-45 in
  `restart_core`; sequence after both.

### WP-80 tic-tac-toe-2 unbounded players - READY
- Scope (0 new findings): f F46, counted in WP-09's scope
- Paths: game/tic-tac-toe-2/src/lib.rs, lib/cmd/src/requester/gamer.rs
- Severity: 1m
- Crafted `players` count drives unbounded allocation/iteration. In no Tier 3
  package; WP-09a/WP-09b own the requester-trust pattern - fold in there.

## Split out by the 2026-07-26 decision session

### WP-81 dead per-game stats machinery removal - READY (D-40 answered 2026-07-26: option B)
- Scope (3): **c F12** (moved out of WP-20), **e F39** and **e F40** (moved
  out of WP-30). These are re-homed findings, not new ones - each still
  appears in exactly one package, so the 570 sum and the
  one-package-per-finding invariant are unaffected. WP-20 drops 5 -> 4 and
  WP-30 drops 5 -> 3 in the coverage table.
- Paths: game/acquire-1/src/{lib.rs,stats.rs}, game/lost-cities-1/src/lib.rs,
  game/lost-cities-2/src/lib.rs
- Severity: 3m
- **Why it exists:** D-40 answered **option B - delete the dead machinery**.
  acquire-1's `to_brdgme_stats` has zero callers; lost-cities-1/-2 `Stats`
  fields are never read and one increment counts the wrong thing. These are
  **stats questions, not rules questions**, so they were split out of the two
  parked parity packages to land **ahead of the rules review**.
- **Not blocked by the parity park.** WP-20 and WP-30 remain
  `BLOCKED-ON-USER-RULES-REVIEW` for their rules halves; this package is not.
- **For the record:** Michael wants to revisit **"game specific stats" in
  future from a CLEAN SLATE**. That is exactly why deletion is right now -
  there is no platform consumption path to wire into, and a future feature
  should not inherit this shape. Do **not** substitute option A (wiring them
  up) on the grounds that the data "might be useful".
- Delete only the dead machinery. Do not change any gameplay or scoring, and
  do not touch a `RULES.md` - the park still covers those crates' rules.

## Escalated by the user 2026-07-26

### WP-82 db.rs module split - READY (spec: `specs/WP-82-db-module-split.md`)
- Scope (0 new findings): **ws F42**, counted in WP-41's scope. Supersedes the
  unowned **WP-78** entry; adds 0 to the 570 sum and the
  one-package-per-finding invariant is unaffected.
- Paths: web/src/db.rs -> web/src/db/{mod.rs, plus 12 domain modules}
- Severity: 1m
- **Escalated by Michael on 2026-07-26** from WP-78's DEFERRED status: the file
  "is becoming problematic due to its size and complexity".
- **Measured 2026-07-26 against the live post-WP-41 tree** (WP-41 landed,
  +1397/-125, so the review's numbers are stale): **8149 lines**, of which the
  single `#[cfg(all(test, feature = "ssr"))] mod tests` is **4838 (59%)**.
  Production is **~3311 lines** across **107 top-level fns**. Full evidence
  base: `raw/db-split-inventory.md`.
- **Axis: by domain**, with a small pure/row-mapper `db/common.rs`. Chosen on
  measured coupling: of 28 intra-file call edges the domain axis crosses **6**
  module pairs (all acyclic, all downward); an operation-kind axis (read /
  write / tx) would cross **~19**.
- **Pure refactor. Move code, do not change it.** Any bug spotted during the
  move is **reported, not fixed** - it probably belongs to another package.
- **Callers do not change.** 293 external `db::` references across 22 files,
  but only **5** `use` lines tree-wide; everything else is fully-qualified
  `crate::db::foo(...)`. `pub use` re-exports from `db/mod.rs` keep 100% of
  them compiling.
- **No `cargo sqlx prepare` needed.** `rust/web/.sqlx` entries key on the query
  text hash (`db_name`/`describe`/`hash`/`query`); they carry no file path, so
  moving `sqlx::query!` macros between files does not invalidate the cache.
- **HARD PREDECESSOR for the remaining web cluster:** WP-35, WP-40, WP-45,
  WP-47, WP-49, WP-50, WP-52, WP-53, WP-59 (all list `web/src/db.rs`), plus
  WP-42 (adds `is_proposal_visible_to_user` to `db.rs` and consumes WP-47's
  predicate) and, transitively, WP-84. See `landing-order.md` section 7.1 and
  section 10.
- Structural problems noticed while inventorying (oversized fns, inverted
  dependencies, missing transaction/repository boundary) are **out of scope**
  and parked in `architecture-observations.md` for the deferred architectural
  review. Do not widen into them.

## Released from the rules park 2026-07-26 (D-35)

### WP-83 parity fixes released from the rules park - READY (spec: `specs/WP-83-parity-fixes-released.md`)
- Scope (0 new findings): **`a F1`** (roll-through-the-ages-2 stale phase
  re-match), **`b F7`** (seven-wonders both sides of one wonder board),
  **`e F30`'s seat-order half** (red7 empty winning set wins on seat order).
- **Coverage bookkeeping - these findings do NOT move.** This is a *released
  carve-out*, not a re-assignment: `a F1` stays counted in **WP-12** (scope 9),
  `b F7` in **WP-16** (scope 8) and `e F30` in **WP-30** (scope 3), exactly as
  those entries still list them. Unlike WP-81 (which moved `c F12`/`e F39`/
  `e F40` out of WP-20/WP-30 and adjusted their coverage rows), **no coverage
  row changes here**: WP-83 adds 0 to the 570 sum and the
  one-package-per-finding invariant is unaffected. The three parked entries
  remain the owners of record; WP-83 is the execution vehicle for the released
  halves.
- Paths: game/roll-through-the-ages-2/src/lib.rs,
  game/seven-wonders-1/src/{card.rs,lib.rs}, game/red7-1/src/{card.rs,lib.rs}
- Severity: not tallied separately - carried in WP-12's, WP-16's and WP-30's
  severity lines.
- **Why it exists:** D-35 released these three from the
  `BLOCKED-ON-USER-RULES-REVIEW` park (WP-11, WP-12, WP-16, WP-20, WP-26,
  WP-30) as **FIX NOW**. Do not re-park them. Everything else in those packages
  stays parked - in particular **`b F4` was re-parked** under a binding user
  correction and **`d F37` was REJECTED as not a bug**; neither is in scope.
- Three surgical fixes in three crates, landable as three commits. **Do not
  touch any `RULES.md`** in these crates and do not widen into the parked rules
  questions they carry. The only public API change is `card::leader`'s
  signature in red7-1.

## Added by the 2026-07-26 SSE pivot (D-44)

### WP-84 migrate `/ws` to Server-Sent Events - READY (spec: `specs/WP-84-sse-migration.md`)
- Scope (0 new findings): supersedes the **transport half** of `ws F59`, which
  stays counted in WP-42's scope. Adds 0 to the 570 sum; the
  one-package-per-finding invariant is unaffected.
- Paths: new `web/src/events.rs`; web/src/router.rs, web/src/websocket.rs
  (deletions), web/src/websocket_client.rs (-> `events_client.rs` in the
  deletion commit), web/Cargo.toml, web/tests/websocket_hygiene.rs (deleted),
  web/tests/ssr_pages.rs, infra/cloudflare.tf
- Severity: n/a (transport migration, not a review finding)
- **Decisions:** D-44 (COMMIT to SSE, migrate NOW ahead of WP-42's WebSocket
  hardening), D-45 (no `Last-Event-ID`, no `id:` - NATS Core has no replay),
  D-46 (topology - see `sse-topology-decision.md`, RESOLVED), D-47 (Cloudflare:
  rate-limit connection establishment only, never stream duration; server
  heartbeat; verify the real config, do not assume), D-48 (browser leg measured
  `HTTP/2 200` -> **two streams**; never three; dev stays h1), D-49 (future SSE
  uses are hypothetical - build no topic machinery, no multiplexing layer, no
  channel registry; name `event:` fields meaningfully from day one), D-50
  (`/events/public` takes a repeatable `topic` param, N games from day one).
- **Settled two-stream design (no blocker):** `GET /events` - private,
  identity-scoped, opened once per SPA session, never swapped on navigation;
  plus `GET /events/public?topic=game:<a>&topic=game:<b>` - unauthenticated,
  swapped on navigation, same key repeated (no `[]` suffix). Parses into a
  collection from day one, accepts N `game:` topics up to a small cap, and
  **rejects any other topic kind, any malformed topic and zero topics with a
  400** rather than silently ignoring them. Per-frame filter is set membership
  over the requested ids plus `db::is_game_publicly_visible`. Extractor is
  `Query<Vec<(String, String)>>` filtered for key `topic` - verified against
  axum 0.8.9 source; the `HashMap` and struct-with-`Vec` forms collapse or 400.
  No new dependency.
- **Why now:** the 101 upgrade hijack is the only reason WP-42 needed a
  hand-rolled pre-upgrade auth path, a `TaskTracker` and a real-listener test
  harness. SSE is an ordinary `GET`, so all three dissolve. Hardening the
  WebSocket and then deleting it is wasted effort. Michael's forward-looking
  motivation is on the record in D-44 (Topcoat plans SSE and not WebSockets;
  Leptos bus-factor risk) - do not re-argue this as "axum supports both".
- **Consumes, does not author:** WP-47's `is_game_visible_to_viewer` and WP-42's
  `is_proposal_visible_to_user` + per-connection TTL cache. `/events/public`
  needs no auth, no per-user predicate and no cache at all - public game ids are
  already public.
- **Rollout is side-by-side, NOT a cutover** (spec §5, three commits): add
  `/events` alongside `/ws`; switch the client; delete `/ws`. `/pkg/` assets are
  edge-cached `immutable`, so a browser on an old wasm bundle keeps opening
  `/ws` after deploy.
- **Explicitly UNKNOWN and must be proven, not assumed:** whether hyper's
  graceful shutdown lets the `ws_tasks: TaskTracker` be deleted (spec §3g
  requires a real-listener proof first), and Cloudflare's edge idle behaviour
  for `text/event-stream` (spec §6).
- Deletes WP-42 Task B outright - **do not build `sub`/`unsub`.**
- **Landing order: WP-82 -> WP-47 -> WP-42 (predicate work ONLY -
  `is_proposal_visible_to_user` + the TTL cache; its pre-upgrade auth dance is
  superseded and must NOT be written) -> WP-84.** See `landing-order.md`
  section 10.
- **Package-count note:** this entry added one package to the totals recorded
  earlier in this file. Those totals were **recomputed 2026-07-26** in the
  Coverage-check section, which now includes WP-83 and WP-84.
