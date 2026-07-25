# Verification LOG: games-batch-e (2026-07-24)

Independent verification of `findings/games-batch-e.md` (unit 7, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

46 findings total in games-batch-e.md, numbered F1-F46 in document order.
Four serial Workers (model fable per user override), split by crate so each
reads a coherent source set:

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | game/love-letter-2 + shared boilerplate section | F1 command() 8x duplicated wrap-up (major), F2 end_score unreachable! runtime path (minor), F3 end_round hands[p][0] unchecked (minor), F4 assert_target/play_* unvalidated target index (minor), F5 commands accepted after finish (minor), F6 play_baron no-op hand assignments (nit), F7 guard self-target ordering skips validation (nit), F8 discard_card records unheld cards (nit), F9 mod test vs tests (nit), F45 binary-only deps as lib [dependencies] (minor), F46 HTTP binary defaults to port 80 (nit) | raw/games-batch-e-ll.md |
| W2 | game/age-of-war-2 + game/red7-1 | F10 unwrap/expect cluster in runtime paths (minor), F11 completed_lines HashSet nondeterministic serialization (minor), F12 not-your-turn as invalid_input (nit), F13 placings-log tail triplicated (nit), F14 finished game emits duplicate placings logs (nit), F15 clan_conquered duplicated in renderer (nit), F16 "discard one dice" (nit), F29 CardParser byte-slice non-ASCII panic (critical), F30 zero-rule-fulfilling player treated as winning (major), F31 DATA_DOCS tie-break wrong (minor), F32 RULES.md turn/scoring gaps (minor), F33 PubCard/PubSuit alias unused (nit), F34 leader_with_suit index panic all-eliminated (nit), F35 end_points underflow >10 players (nit) | raw/games-batch-e-aow-red7.md |
| W3 | game/lost-cities-2 | F17 Finished stats hardcoded players 0/1 (major), F18 player_state unchecked index (major), F19 draw logs dropped on deck-emptying draw (minor), F20 PlayerState.hand doc says sorted (minor), F21 Stats.investments dead / expeditions write-only (minor), F22 unreachable! on players outside 2..=3 (minor), F23 % MAX_PLAYERS not % players (nit), F24 game-over log regressed vs -1 (nit), F25 discard piles top-card-only (nit), F26 draw-count underflow (nit), F27 k8s blurb says two-player (minor), F28 stale build-release/.rls.toml (nit) | raw/games-batch-e-lc2.md |
| W4 | game/lost-cities-1 | F36 player_state unchecked index (major), F37 draw logs dropped on deck-emptying draw (major), F38 hand doc says sorted (minor), F39 Stats.investments/expeditions dead (minor), F40 expeditions increment condition wrong (minor), F41 HAND_SIZE underflow (nit), F42 literal 2 vs PLAYERS (nit), F43 score() is_none+unwrap (nit), F44 render temp empty Vecs (nit) | raw/games-batch-e-lc1.md |

Original per-finding severity recount (from the blocks, not any summary
line): 1 critical (F29) / 6 major (F1, F17, F18, F30, F36, F37) / 17 minor /
22 nit = 46.

Go sources: love_letter_1 and age_of_war_1 exist in brdgme-go; port-parity
claims for those crates are checked against them. lost-cities-1/-2 and
red7-1 have no Go source — official-rules claims are checked against
in-crate RULES.md/docs where possible; claims resting solely on external
rulebook knowledge are flagged as external-basis, not rejected outright.
Lead spot-checks all REJECTED/ADJUSTED verdicts; if a Worker confirms
everything, Lead re-verifies its 1-2 hardest confirmations. Curated report:
verification/games-batch-e.md.

### W1 dispatched — love-letter-2 + shared (F1-F9, F45-F46)

### W1 returned

9 CONFIRMED, 2 ADJUSTED (F9, F45). Dump: raw/games-batch-e-ll.md.
- F1-F8 verified cleanly incl. Go parity (command.go, char_baron.go,
  char_guard.go, game.go DiscardCard) and PORTING_NOTES.md quirk list
  (Baron/Guard quirks indeed undocumented).
- F9 ADJUSTED (nit stands): premise wrong — 17 game crates use `mod test`
  vs 10 `mod tests`; love-letter-2 follows the game-crate majority. Only
  lib/ is uniformly `mod tests`. Recast as workspace-wide inconsistency.
- F45 ADJUSTED (facts stand, recommendation invalid): Cargo
  `[dev-dependencies]` do NOT apply to `src/bin/` targets — the proposed
  move would break every game binary. Correct fix: optional deps +
  `required-features` on `[[bin]]`, or a separate bin crate. Also no
  in-repo crate consumes a game crate as a library, so the transitively-
  builds impact is currently vacuous; tokio feature trim is the real win.
- F46 confirmed; boilerplate-identical claim spot-checked in age-of-war-2
  and lost-cities-2.
- CODING.md location note: it is `docs/CODING.md` (repo root docs), and
  its no-panic rule (lines 46-49) scopes to server handlers/DB/Leptos —
  citing it against game crates is extension by analogy. Applies to F2
  (and W2/W3 panic findings); noted, does not overturn.

### Lead spot-checks (W1)

- F9 ADJUSTED upheld — Lead grepped the snapshot: 17 game/*/src/lib.rs
  use `mod test {` vs 10 `mod tests {`. The crate matches the local
  majority; nit stands but recast.
- F45 ADJUSTED upheld — Lead read game/love-letter-2/Cargo.toml
  (brdgme_cmd/brdgme_fuzz/tokio-full under [dependencies], brdgme_cmd
  dev-dep with test-support) and concurs on Cargo semantics:
  dev-dependencies apply to tests/examples/benches, not bin targets.
  Severity kept minor (build-time/hygiene cost across 27 crates) with
  corrected recommendation.

### W2 dispatched — age-of-war-2 + red7-1 (F10-F16, F29-F35)

### W2 returned

14 CONFIRMED, 0 ADJUSTED/REJECTED. Dump: raw/games-batch-e-aow-red7.md.
- F29 critical upheld end-to-end: char-count guard (command.rs:23-24) vs
  byte slices (command.rs:31,34-35); reachability traced Request::Play ->
  handle_play (gamer.rs:131) -> command() -> Chain2(Token,
  AfterSpace(CardParser)); Token/AfterSpace ASCII-safe so the non-ASCII
  tail reaches CardParser; no catch_unwind in lib/. Trigger = any current
  player.
- F30 major upheld with the concrete trace (Green rule, p0=[b5], p1=[r7]
  -> palettes [[],[]] -> strict `>` keeps index 0); all three
  consequences confirmed in lib.rs; rules premise flagged external basis;
  deviation undocumented in RULES.md/DATA_DOCS.md.
- F14 Go-parity checked directly: age_of_war_1/game.go:50-64 Command
  never appends placings logs — amplification is new in the Rust port.
- Cosmetic notes only: F32 claim 1 "lists as alternatives" is generous
  (RULES.md silent, not contradictory); F10 carries the CODING.md scope
  note (same as W1's).

### Lead spot-checks (W2)

All-confirm Worker, so Lead re-verified the two hardest directly:
- F29 — read command.rs:18-43: `chars.len() < 2` guard counts chars but
  `Card::parse(&input[..2])` / `consumed` / `remaining` slice bytes;
  `r€` = 4 bytes, guard passes, byte 2 is mid-`€` -> panic. Upheld,
  critical stands.
- F30 — read card.rs:297-317: with all-empty winning sets, `p.len() >
  leader_palette.len()` and `i_max > l_max` (both `(0,0)` via
  `unwrap_or`) are false, so index 0 stays leader with an empty set.
  Upheld, major stands (external rules basis noted).

### W3 dispatched — lost-cities-2 (F17-F28)

### W3 returned

11 CONFIRMED, 1 ADJUSTED (F28 detail, nit stands).
Dump: raw/games-batch-e-lc2.md.
- F28 ADJUSTED: `.rls.toml` is NOT malformed — file is exactly
  `build_lib = true` with no trailing newline; the original's
  "build_lib = truetarget" was a cat-concatenation artifact with
  `.gitignore`'s `target` line. Stale-file substance stands.
- F18 severity sanity check by worker: kept major (not critical) because
  PlayerRender.player comes from the server-side envelope (api.rs:29-32),
  not player command text.
- F23 strengthened: same render.rs clamps correctly at :51-54, making
  the `% MAX_PLAYERS` at :130 a clear outlier.
- F27 location correction: blurb at game-version.yaml line 9, not 1.

### Lead spot-checks (W3)

- F28 ADJUSTED upheld — Lead ran `od -c` on .rls.toml: 16 bytes,
  `build_lib = true`, no newline, no `target` text. Original misquote
  confirmed; nit stands.
- Hardest confirmations re-verified directly: F17 (lib.rs:534
  `stats: vec![self.player_stats(0), self.player_stats(1)]` with
  player_counts at :644 allowing 3) and F18 (lib.rs:570
  `hand: self.hands[player].clone()`; gamer.rs:170-182
  handle_player_render passes `player` unchecked into player_state).
  Both upheld at major.

### W4 dispatched — lost-cities-1 (F36-F44)

### W4 returned

8 CONFIRMED, 1 ADJUSTED (F37 major -> minor).
Dump: raw/games-batch-e-lc1.md.
- F37 ADJUSTED: defect real (lib.rs:434-438 returns `self.end_round()`,
  dropping the local `logs`), but it is byte-for-byte the same defect the
  original rated minor in lost-cities-2 (F19); log-only loss, state
  correct. Aligned both at minor.
- F41 precision: release-mode wrap is immediately clamped by the
  `num > dl` check (lib.rs:403-405) — release drains the deck rather
  than panicking; debug still panics. Nit stands.
- F42 undercounts: bare `2`s also at lib.rs:511-512, 638, 642.
- F38 stronger in -1 than -2: `drawn.sort()` (lib.rs:411) only sorts the
  private log; the hand never gets even per-batch sorting.

### Lead spot-checks (W4)

- F37 ADJUSTED upheld, major -> minor — Lead read lib.rs:398-439: logs
  accumulated then discarded on the `self.deck.is_empty()` branch; state
  transition correct; identical shape to lost-cities-2's F19 which the
  same reviewer rated minor. Internal consistency requires minor.

## Curation complete (2026-07-24)

42/46 CONFIRMED, 4 ADJUSTED (F9 detail; F28 detail; F37 major -> minor;
F45 recommendation invalid), 0 REJECTED, 0 UNVERIFIABLE. All 46 findings
survive. Corrected unit tally: 1 critical / 5 major / 18 minor / 22 nit
(original per-finding fields: 1/6/17/22; the findings file has no summary
tally line to miscount). Report: verification/games-batch-e.md.
LOG closed.
