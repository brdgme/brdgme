# games-batch-e — Lead review log

Unit: love-letter-2, age-of-war-2, lost-cities-2, red7-1, lost-cities-1
(snapshot: /home/beefsack/Development/brdgme-review-snapshot, HEAD f8763a5)

## Progress

- [x] Read handover.md, inventory.md (module maps), docs/CODING.md.
- [x] Confirmed snapshot layout: all 5 crates present; brdgme-go has
      love_letter_1 + age_of_war_1 Go sources; NO lost-cities or red7 Go
      sources (workers judge those against official rules, noted per finding).
- [x] Created raw/ dir.

### Worker 1 — love-letter-2 (+ boilerplate-binary deep-dive) — DONE
- Raw file: `raw/games-batch-e-love-letter-2.md` (verified, 199 lines).
- 11 findings: 0 critical / 1 major / 5 minor / 5 nit.
- Headlines: major simplicity — `Gamer::command` (lib.rs:698-860) duplicates
  ~20-line finish/scores wrap-up across all 8 arms. Minor correctness —
  `end_score()` `unreachable!()` (lib.rs:29); unchecked `hands[p][0]` /
  `eliminated[target]` indexing (lib.rs:184, 305, 405, 444, 500); command
  parser has no finished-game guard (command.rs:23). Systemic boilerplate:
  binary-only deps (brdgme_cmd, brdgme_fuzz, tokio "full") declared as library
  `[dependencies]` (Cargo.toml:9-16); http bin binds privileged 0.0.0.0:80
  (love_letter_2_http.rs:9). Go parity verified line-by-line; no rules
  divergence; no serde info leaks.

### Worker 2 — age-of-war-2 — DONE
- Raw file: `raw/games-batch-e-age-of-war-2.md` (verified, 159 lines).
- 7 findings: 0 critical / 0 major / 2 minor / 5 nit.
- Headlines: faithful port, castle/clan/dice tables + turn logic verified
  against Go, no rules divergence. Minor — `completed_lines: HashSet<usize>`
  in serde state serializes nondeterministically (lib.rs:35, use BTreeSet);
  six invariant-guarded unwrap/expect sites (lib.rs:132/219/334,
  command.rs:89, render.rs:48/110) violate no-panic rule though unreachable
  from input. Nits — "not your turn" as unstructured invalid_input instead of
  GameError::NotYourTurn (lib.rs:461-464); placings-log tail triplicated;
  duplicate placings logs in finished games (preserved Go quirk, Rust-amplified);
  `clan_conquered` duplicated lib.rs/render.rs; "discard one dice" typo.
  Binaries byte-identical to boilerplate, zero deviations.

### Worker 3 — lost-cities-2 (+ lc1/lc2 relationship) — DONE
- Raw file: `raw/games-batch-e-lost-cities-2.md` (verified, 121 lines).
- 10 findings: 0 critical / 2 major / 4 minor / 4 nit.
- Relationship conclusion: lost-cities-1 is legacy/superseded —
  `isDeprecated: true` in k8s/base/game/lost-cities-1/game-version.yaml:10;
  both stay deployed per docs/porting/GAME_PORTING.md lifecycle; -2 is a
  faithful generalization adding a documented (non-official) 3-player variant;
  2-player logic/scoring/tests identical where overlapping. Duplication
  justified. (GAME_PORTING.md claims -2 ported from in-repo Go but no
  lost-cities Go exists — rules judged against official rules.)
- Headlines: MAJOR — `status()` hardcodes 2-player stats, finished 3-player
  games lose player 2's stats (lib.rs:534). MAJOR — `player_state()`
  unchecked `self.hands[player]` (lib.rs:570) reachable via crafted
  Request::PlayerRender → panic. Minor — `draw_hand_full` drops draw logs
  when deck empties (lib.rs:441-445); hand docs claim sorted but serialize in
  acquisition order; `Stats.investments` never incremented, `Stats.expeditions`
  write-only; blurb still says "two-player". Nits — unreachable!() arms,
  `% MAX_PLAYERS` perspective bug (render.rs:130), game-over log regression
  vs -1, usize underflow pattern (lib.rs:408), crate-root cruft.
  2-player rules verified against official rules — correct. No info leaks.

### Worker 4 — lost-cities-1 — DONE
- Raw file: `raw/games-batch-e-lost-cities-1.md` (verified, 85 lines).
- 8 findings: 0 critical / 2 major / 3 minor / 3 nit.
- Headlines: MAJOR — `player_state()` unchecked `self.hands[player]`
  (lib.rs:566) panic via crafted Request::PlayerRender (same defect as -2).
  MAJOR — `draw_hand_full` drops draw logs on round end (lib.rs:434-438,
  same as -2). Minor — hand documented "sorted" but never sorted
  (lib.rs:92, DATA_DOCS.md:18); dead Stats.investments/expeditions.
  Nits — HAND_SIZE - hand.len() underflow (lib.rs:401), hardcoded literal 2s
  vs PLAYERS const, guarded unwrap in score() (lib.rs:686), `&vec![]` in
  render.rs:185/196. Rules verified against official rules — correct.
  No info leaks; binaries no deviation.

### Worker 5 — red7-1 — DONE (after one quota-403 failure, retry succeeded)
- First dispatch failed with provider quota error; retry after log
  checkpoint succeeded.
- Raw file: `raw/games-batch-e-red7-1.md` (verified, 165 lines).
- 7 findings: 1 critical / 1 major / 2 minor / 3 nit.
- Headlines: CRITICAL — CardParser char-count vs byte-slice panic
  (`play r€`) at command.rs:31,34-35 (crate-local, distinct from core-parser
  issue). MAJOR — `leader()` (card.rs:297-316) treats first non-eliminated
  player with zero rule-fulfilling cards as leader under Green/Violet;
  official rules say such a player cannot win → wrong survival on `done`,
  illegal discard into such rules (lib.rs:325-330), 0-point round winners.
  Minor — DATA_DOCS.md:36 documents a nonexistent second tie-break;
  RULES.md omits play-then-discard combo and misstates scoring. All 7
  color-rule evaluations + tie-breaks otherwise verified correct vs official
  rulebook. No info leaks; binaries no deviation.

## Curation status — COMPLETE
- All 5 raw files exist and verified (love-letter-2 199, age-of-war-2 159,
  lost-cities-2 121, lost-cities-1 85, red7-1 165 lines).
- Lead spot-checked headline findings against the snapshot before accepting:
  red7-1 command.rs:23-35 byte-slice panic (CONFIRMED verbatim),
  lost-cities-2 lib.rs:534 stats hardcode and lib.rs:570 unchecked index
  (CONFIRMED verbatim), love-letter-2 lib.rs:29 unreachable!() (CONFIRMED).
- Rejections during curation: none — all 43 raw findings carried forward.
  Adjustments: two systemic boilerplate findings moved to a shared trailing
  section (per brief, systemic issues noted once); the lost-cities-2
  blurb finding's location normalized to k8s/...game-version.yaml:1;
  duplicate per-crate mention of lost-cities-2 cruft kept (crate-specific).
- Curated output: `/home/beefsack/Development/brdgme/docs/reviews/2026-07-23-rust-review/findings/games-batch-e.md`
  (34.7 KB, 43 findings: 1 critical / 5 major / 16 minor / 21 nit).
