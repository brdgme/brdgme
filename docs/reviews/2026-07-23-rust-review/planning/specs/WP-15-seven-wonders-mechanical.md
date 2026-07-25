# WP-15: seven-wonders-1 mechanical fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Fix the three majors in `rust/game/seven-wonders-1` — the Halicarnassus B wonder-stage VP that is never scored (b F1), the reachable permanent soft-lock in the DrawDiscard resolver (b F2), and the 3 coins wrongly paid for the auto-discarded last card of each age (b F3). Also land the verified minors/nits: store the chosen trade deal instead of an index into a recomputed list (b F9 — the original "verified unreachable" analysis was invalidated by verification; this is a reachable wrong-payment/free-build bug), guard out-of-range player indexing (b F10), use `N::Player` in the military-conflict log (b F12), remove the dead `start_hand()` indirection (b F13), close the verified test-coverage gaps (b F14), and split scoring/trade out of the lib.rs grab-bag (b F15). F2 and F3 agree with the crate's own `PORTING_NOTES.md` and the official rules per the work-packages triage — they are mechanical fixes, NOT parity/decision-gated.

**Architecture — how seven-wonders-1 works (read this before editing):**

- One crate, `rust/game/seven-wonders-1` (package name `seven-wonders-1`, confirmed from `Cargo.toml`): `src/lib.rs` (state machine, trading, scoring, resolver queue, `Gamer` impl, inline tests at lib.rs:1206-1565), `src/card.rs` (card database, cities, decks — reviewed clean, only read here), `src/command.rs` (parser, gated on game state), `src/render.rs` (untouched by this package), `tests/contract.rs` (standard contract harness — untouched).
- 3-7 players, simultaneous turns. Each hand: every player with a non-empty hand picks one `Action` (`Build` with `free`/`wonder` flags, or `Discard`); when all are picked and `chosen` (multi-deal builds need a follow-up `deal N`), `check_hand_complete` (lib.rs:243) runs `execute_actions` (lib.rs:265) in player-index order p0..pn, then `end_hand` (lib.rs:182) passes hands (direction alternates by age, `pass_hands` lib.rs:231) or ends the age (`end_round` lib.rs:217 → `military_conflicts` lib.rs:756). When every hand is down to 1 card, `end_hand` auto-discards it (lib.rs:189-201) unless the player has `PlayFinalCard`.
- Resolver queue: building a card with `CardEffect::DrawDiscard` (Halicarnassus wonder stages, card.rs:1252/1269/1277/1285) queues `Resolver::DrawDiscard { player }` in `post_build_hook` (lib.rs:410-412). While `to_resolve` is non-empty, `status()` pins the turn to `to_resolve[0]`'s player (lib.rs:1152-1156) and `command_parser` offers ONLY `take` (command.rs:21-37). `take_from_discard` (lib.rs:908-940) rejects cards the player already owns (lib.rs:921-923) — the b F2 soft-lock when everything in the pile is owned.
- Trading: `can_afford_cost` (lib.rs:448) enumerates neighbor-trade deals via `brdgme_cost::can_afford_perm`; a deal is a `HashMap<i32, i32>` of direction (`DIR_LEFT` = -1 / `DIR_RIGHT` = 1, card.rs:36-38) → coins. Deal enumeration depends on the CURRENT state of both neighbors' built cards, and `can_afford_perm` has an early-return (`rust/lib/cost/src/lib.rs:181-184`) that reshapes the list when a single option covers the remaining cost — which is why re-indexing into a recomputed list at execute time (b F9, lib.rs:418-431) can pay the wrong neighbor or, via `unwrap_or_default()`, build for free after an earlier-indexed player's build changes a neighbor's goods mid-`execute_actions`.
- Scoring: `player_vp` (lib.rs:701-725) sums victory/defeat tokens, coins/3, `science_vp`, and per-card effects; its match has arms for `VP`, `Bonus` (vp>0), `MimicGuild` and drops everything else via `_ => {}` — including `DrawDiscard { vp }` (b F1).
- Serialization: the whole `Game` is serde round-tripped between requests (DB stores game_state JSON). `Game.actions: Vec<Option<Action>>` is non-`None` for any player who has picked but whose hand has not yet executed — i.e. mid-hand saved states carry serialized `Action::Build` values. **No fix in this package may break deserialization of existing saved states.** The only serialized-shape edit in this package is b F9's new `deal_coins` field on `Action::Build`, added with `#[serde(default)]` alongside the retained legacy `deal: Option<usize>` field — compatibility reasoning in re-derivation note 3.
- Tests: inline `#[cfg(test)] mod tests` in `src/lib.rs` (line 1206) with helpers `players()`, `cmd()`, `new_game()` (seed 42, 3 players: MICK=0, STEVE=1, GREG=2), `rhodes_a()`, `db_card(name)`. `tests/contract.rs` is the standard `assert_gamer_contract::<Game>()`. Because the tests module is inside lib.rs, it can call private `Game` methods (`military_conflicts`, `pass_hands`, `score_science`) directly — Task 8 uses this.

**Tech Stack:** Rust 1.97.0 (edition 2024) workspace at `/home/beefsack/Development/brdgme/rust`. Single crate touched: `seven-wonders-1`. `serde_json` is already a dev-dependency (used by `test_pub_state_does_not_leak_hidden_info`).

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p seven-wonders-1`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- ALL existing tests MUST keep passing UNMODIFIED. In particular `test_take_command_already_build` (lib.rs:1476) still passes after the b F2 fix — the prune design was chosen partly so that it does (re-derivation note 2). Any existing-test failure means a fix is wrong — stop and re-check.
- Serialized shapes: only the Task 4 `Action::Build` addition described above. `Game`, `PubState`, `PlayerState`, `Card`, `Resolver` keep their exact serde shapes.
- Line numbers cited below are live-file numbers as of the drift check. Earlier tasks shift later lib.rs numbers — always locate by the quoted symbol/format string, not by count.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script provides the containers).

**Non-Goals:**

- **WP-16 (blocked on D-27/D-28) owns every batch-b rules adjudication: b F4 (same-turn trade of freshly built resources), b F5 (MimicGuild cannot copy Science guilds), b F6 (wonder sacrifice card enters the shared discard pile), b F7 (both sides of one wonder dealt in a game), b F8 (discard pile hidden from players), plus b F19/F20 (alhambra) and b F29 (splendor).** Do NOT snapshot trade goods, extend mimic to Science, stop the wonder-sacrifice discard push, dedupe wonder boards, or expose discard contents in `PubState` here. b F9's fix deliberately preserves b F4's current same-turn-trade semantics: the deal captured at choose time is paid as-is; whether it SHOULD have been recomputed against pre-turn goods is exactly the D-27/D-28 adjudication.
- The six copy-pasted `is_finished()` → placings epilogues in `command()` (lib.rs:1011-1137, b F11) — WP-08 (epilogue dedup). Do not extract a helper here.
- Deserialized-state trust hardening (crafted saved states with out-of-range indices etc.) — WP-09, blocked on D-36. Task 7's guards cover only the two sites b F10 names (framework-passed player index), nothing more.
- `pub_state`/`player_state` redaction review — WP-10 (D-33). Nothing in this package changes what is exposed.
- The pre-existing timing quirk that a Halicarnassus build with an EMPTY pile never queues a resolver even though same-hand discards land afterwards (locked in by `test_take_command_empty`, lib.rs:1462) — no finding covers it; it is a rules-fidelity question adjacent to WP-16. Task 3 does not change it.

**Snapshot drift:** None. `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/seven-wonders-1 /home/beefsack/Development/brdgme/rust/game/seven-wonders-1` is empty (verified 2026-07-25 against snapshot commit f8763a5). All line numbers below are live-file line numbers and match the findings' citations.

**Re-derivation notes (differences from / sharpenings of the findings — read before the tasks):**

1. **b F1 fix shape (Task 1):** the finding's literal recommendation `CardEffect::DrawDiscard { vp } => vp += vp,` self-shadows (`vp` the binding vs `vp` the accumulator) and would not compile as intended. The arm binds a distinct name: `CardEffect::DrawDiscard { vp: stage_vp } => vp += stage_vp,`. Verified against live card data: Halicarnassus B stages 1/2/3 carry `vp: 2/1/0` (card.rs:1269, 1277, 1285) and Halicarnassus A stage 2 carries `vp: 0` (card.rs:1252) — so the fix is worth 3 VP to a Halicarnassus B player and is a provable no-op for Halicarnassus A.
2. **b F2 fix shape (Task 3) — ADJUSTED from the finding's queue-time recommendation:** the finding suggests filtering at queue time (only push the resolver if a takeable card exists) "and re-check at resolve time". Re-derivation from live code shows queue-time filtering alone is BOTH insufficient AND wrong-sided:
   - Insufficient: two players can build DrawDiscard stages in the same hand (two resolvers). The first player's `take` shrinks the pile and can remove the second player's only takeable card — invalidation happens at resolve time, after any queue-time check.
   - Wrong-sided: `post_build_hook` fires mid-`execute_actions`, BEFORE later-indexed players' same-hand discards reach the pile (`execute_discard` lib.rs:364, wonder-sacrifice push lib.rs:325). During `execute_actions` the pile only GROWS, and the resolving player's tableau is fixed once their own action has run — so a queue-time "no takeable card" verdict can be falsified seconds later by a same-hand discard, and queue-time filtering would wrongly cancel those takes (it would also break the existing `test_take_command_already_build`, which relies on exactly that: the pile card MICK owns is joined by STEVE's and GREG's same-hand discards).
   The correct choke points are therefore where the resolver is about to FIRE: (a) after `execute_actions` completes inside `check_hand_complete`, and (b) after each `take` removes a resolver. Task 3 adds a single `prune_resolvers()` used at both points; the queue-time guard `!self.discard.is_empty()` stays as-is. This satisfies `PORTING_NOTES.md`'s claim ("DrawDiscard resolver only fires if there are takeable cards in discard") at the moment of firing, which is the only moment it can be decided. The finding's alternative (a `pass` command) is rejected: it adds parser/state surface for a situation that should simply not stall the game.
3. **b F9 serde compatibility (Task 4) — the store-chosen-deal fix stands (verification overturned the "unreachable" analysis, upgrading nit → minor):** storing the chosen deal requires a new field on `Action::Build`, and `Action` is persisted (mid-hand saved states carry e.g. `{"Build":{"card":2,"free":false,"wonder":false,"deal":0,"chosen":true}}` inside `Game.actions`). Changing `deal`'s type from `Option<usize>` to a map would fail to deserialize those integers. The compatible shape: KEEP `deal: Option<usize>` (legacy, still deserialized), ADD `#[serde(default)] deal_coins: Option<HashMap<i32, i32>>` (serde fills `None` when the key is absent from old JSON; `#[serde(default)]` is supported on struct-variant fields). New states always write `deal: None` + `deal_coins: Some(..)` at choose time; `resolve_deal` prefers `deal_coins` and falls back to the legacy recompute-and-index path (old semantics, including its `unwrap_or_default`) ONLY for pre-upgrade pending actions — a window of at most one in-flight hand per legacy game, after which the legacy path is dead. Old readers do not exist post-deploy (the game binary is replaced atomically), so writing the extra key is safe. `HashMap<i32, i32>` satisfies `Action`'s `PartialEq, Eq` derives.
4. **b F3 log claim (Task 2):** the finding says to "update the log text which currently hides the payment". The live log (lib.rs:196-199) says only "discarded their last card" — it never mentioned coins, so once the payment is removed the existing text is simply accurate. No log change needed.
5. **b F14 gap list (Task 8):** verification corrected the finding — MimicGuild IS tested (`test_card_mimic_guild`, lib.rs:1504-1511) and is excluded from the gap list. The remaining verified gaps landed here: military conflict resolution/token values, pass direction per age, `Bonus`/guild scoring (Haven, Strategists, Builders), multi-deal selection via `deal N`, seed determinism, and a full-game replay. Halicarnassus B VP lands with Task 1.

---

### Task 1: score DrawDiscard wonder-stage VP (b F1, MAJOR)

**Problem (restated):** `player_vp` (`rust/game/seven-wonders-1/src/lib.rs:701-725`) matches `CardEffect::VP`, `CardEffect::Bonus` (vp>0), and `CardEffect::MimicGuild`; everything else hits `_ => {}`. `CardEffect::DrawDiscard { vp }` (card.rs:88-90) is the effect of all Halicarnassus wonder stages, and Halicarnassus B's stages carry `vp: 2`, `vp: 1`, `vp: 0` (card.rs:1269/1277/1285, matching the official 2/1/0 printed values). The payload is silently dropped: a Halicarnassus B player who builds all three stages loses 3 VP every game. Internally inconsistent regardless of official rules — the card data records VP the scorer never reads.

**Fix (re-derived — see re-derivation note 1 for the binding rename):** add an arm to the `player_vp` match.

**Edge cases:** Halicarnassus A stage 2 (`vp: 0`, card.rs:1252) → arm adds 0, no behavior change for A-side players; a B-side player with only stage 3 built (`vp: 0`) → unchanged; the stage cards enter `self.cards[player]` as `CardKind::Wonder` via both the wonder-build path and (in tests) direct placement — `player_vp` iterates `self.cards[player]` uniformly, so both are covered; `mimic_guild_vp` only inspects `CardEffect::Bonus` guilds, unaffected.

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (`player_vp`, insert after the `MimicGuild` arm at lines 717-719)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/seven-wonders-1/src/lib.rs`:

```rust
    #[test]
    fn halicarnassus_b_stage_vp_is_scored() {
        // b F1: DrawDiscard stages carry printed VP (2/1/0 for the B side)
        // that player_vp dropped via the catch-all arm.
        let mut g = new_game();
        g.cards[MICK] = vec![
            db_card("Halicarnassus B Wonder Stage 1"),
            db_card("Halicarnassus B Wonder Stage 2"),
            db_card("Halicarnassus B Wonder Stage 3"),
        ];
        // 3 starting coins = 1 VP; stages = 2 + 1 + 0 = 3 VP.
        assert_eq!(g.player_vp(MICK), 4);
    }

    #[test]
    fn halicarnassus_a_stage_vp_unchanged() {
        // Lock in that the A side (vp: 0 on its DrawDiscard stage) is a no-op.
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Halicarnassus A Wonder Stage 2")];
        assert_eq!(g.player_vp(MICK), 1); // coins only
    }
```

- [ ] Run: `cargo test -p seven-wonders-1 halicarnassus`. Expected: `halicarnassus_b_stage_vp_is_scored` FAILS (returns 1, the coin VP only). `halicarnassus_a_stage_vp_unchanged` passes already.
- [ ] Implement. In `player_vp`, insert after the `CardEffect::MimicGuild` arm (lines 717-719) and before `_ => {}`:

```rust
                CardEffect::DrawDiscard { vp: stage_vp } => vp += stage_vp,
```

- [ ] Run: `cargo test -p seven-wonders-1` — new tests PASS, full suite PASS (in particular `test_card_mimic_guild` still passes: neither neighbor guild there is a DrawDiscard card).
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs` ; message: `fix(seven-wonders-1): score Halicarnassus DrawDiscard stage VP (b F1, WP-15)`

---

### Task 2: end-of-age auto-discard pays no coins (b F3, MAJOR)

**Problem (restated):** in `end_hand`'s `max_hand == 1` branch (`rust/game/seven-wonders-1/src/lib.rs:189-201`), the unplayed last card of the age is auto-discarded AND paid for:

```rust
                if self.hands[p].len() == 1 && !self.has_play_final_card(p) {
                    let card = self.hands[p].pop().unwrap();
                    self.discard.push(card);
                    self.coins[p] += DISCARD_COINS;
```

Official rules: the seventh card of each age is discarded with NO coin gain (only a player-CHOSEN `discard` action pays 3). The bug hands every player up to 9 free coins per game (~3 VP plus trade liquidity). Not listed as a preserved quirk in `PORTING_NOTES.md`; `RULES.md:26` says only "automatically discarded" — silent on coins. The verification note that the inflation is symmetric does not save it: coins are VP and trade fuel, and PlayFinalCard players (Babylon B) are NOT symmetric — they play the card instead and today effectively forgo the free 3 coins others get.

**Fix (re-derived, matches the finding):** delete the `self.coins[p] += DISCARD_COINS;` line (lib.rs:195). Keep the payment in `execute_discard` (lib.rs:365) — that is the player-chosen discard. The log ("discarded their last card") never mentioned coins and needs no change (re-derivation note 4).

**Edge cases:** player with `PlayFinalCard` (Babylon B stage 2) → skipped by the `!self.has_play_final_card(p)` guard, unchanged; a PlayFinalCard player who CHOOSES `discard` for their final card → goes through `execute_discard`, still pays 3 (correct — that is an elective discard); age transition and discard-pile growth unchanged (the card still enters the pile, so Halicarnassus can still retrieve it).

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (`end_hand`, line 195)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn auto_discarded_last_card_pays_no_coins() {
        // b F3: only the player-chosen discard action pays DISCARD_COINS;
        // the end-of-age auto-discard is free per the official rules.
        let mut g = new_game();
        for p in 0..3 {
            g.hands[p].truncate(2);
        }
        cmd(&mut g, MICK, "discard 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert_eq!(g.round, 2, "the age must have ended via the auto-discard");
        assert_eq!(g.discard.len(), 6, "3 chosen + 3 auto-discarded cards");
        // 3 starting coins + 3 for the chosen discard + 0 for the auto-discard.
        assert_eq!(g.coins, vec![6, 6, 6]);
    }
```

- [ ] Run: `cargo test -p seven-wonders-1 auto_discarded`. Expected: FAILS — coins read `[9, 9, 9]`.
- [ ] Implement: in `end_hand`, delete the line `self.coins[p] += DISCARD_COINS;` (lib.rs:195). Touch nothing else in the branch.
- [ ] Run: `cargo test -p seven-wonders-1` — new test PASSES, full suite PASSES (`test_card_play_final_card_with`/`_without` assert rounds only; `test_free_build` never reads coins after the age turnover).
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs` ; message: `fix(seven-wonders-1): end-of-age auto-discard pays no coins (b F3, WP-15)`

---

### Task 3: prune unresolvable DrawDiscard resolvers (b F2, MAJOR)

**Problem (restated):** `post_build_hook` queues `Resolver::DrawDiscard` whenever the pile is merely non-empty (`rust/game/seven-wonders-1/src/lib.rs:410-412`); `take_from_discard` rejects any card the player already owns (lib.rs:921-923); while a resolver is pending, `command_parser` offers ONLY `take` (command.rs:21-37) and `status()` pins the turn to the resolving player (lib.rs:1152-1156). If every card in the pile is owned by the resolving player, NO command can ever succeed — the game is permanently soft-locked. `PORTING_NOTES.md:63-64` claims the resolver "only fires if there are takeable cards in discard (cards the player doesn't already own)" — the code implements no such filter. A second lock path: two same-hand DrawDiscard builds queue two resolvers, and the first player's take can empty the pile (or remove the second player's only takeable card), stranding the second resolver.

**Fix (re-derived — ADJUSTED from the finding's queue-time recommendation, see re-derivation note 2):** leave the queue-time guard alone; add a `prune_resolvers()` that drops front-of-queue resolvers whose player has nothing takeable, invoked at the two points where a resolver is about to fire: after `execute_actions` in `check_hand_complete`, and after each take removes a resolver in `take_from_discard`. During `execute_actions` the pile only grows and the resolving player's tableau is already fixed, so a resolver that survives the post-execution prune is takeable when the player acts; the post-take prune covers multi-resolver invalidation. A front-only `while` loop is sufficient: deeper resolvers are re-checked by the next prune after each take.

**Edge cases:** pile all-owned after the hand's actions → resolver pruned with a public log, hand ends normally (the pre-fix permanent lock); pile contains the resolver's own same-hand discards → still queued and still takeable (unchanged — this is why `test_take_command_already_build` keeps passing: STEVE's and GREG's discards join MICK's owned Palace in the pile); two resolvers, first take empties the pile → second pruned, hand ends; two resolvers, pile still has a card the second player doesn't own → second resolver fires normally; empty pile at build time → resolver never queued (pre-existing behavior, `test_take_command_empty`, out of scope per Non-Goals); `status()` and `command_parser` need no changes — the prune guarantees `to_resolve[0]` is always actionable whenever they read it.

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (new helpers near `take_from_discard`; `check_hand_complete` line 255-259; `take_from_discard` line 932-937; comment on `post_build_hook` line 410)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests:

```rust
    fn giza_a() -> City {
        cities().into_iter().find(|c| c.name == "Giza A").unwrap()
    }

    #[test]
    fn drawdiscard_pruned_when_pile_all_owned() {
        // b F2: with every pile card already owned by the resolver's player,
        // the resolver must be dropped instead of soft-locking the game.
        let mut g = new_game();
        // Giza's initial resource is Stone, so MICK's 3 Ore come only from his
        // own cards and the build resolves as a single, empty trade deal.
        for p in 0..3 {
            g.cities[p] = giza_a();
        }
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry"), db_card("Palace")];
        g.hands[STEVE][0] = db_card("Lumber Yard");
        g.hands[GREG][0] = db_card("Clay Pool");
        g.discard = vec![db_card("Palace")];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "build 1").unwrap();
        // STEVE and GREG BUILD (zero-cost cards) so the pile stays [Palace].
        cmd(&mut g, GREG, "build 1").unwrap();

        assert!(
            g.to_resolve.is_empty(),
            "a resolver with nothing takeable must be pruned"
        );
        assert_eq!(
            g.whose_turn(),
            vec![MICK, STEVE, GREG],
            "the hand must have ended and passed for everyone"
        );
    }

    #[test]
    fn second_resolver_pruned_when_pile_emptied() {
        // b F2 (multi-resolver): the first take empties the pile; the second
        // resolver must be pruned at that moment, not stranded.
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = giza_a();
        }
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry")];
        g.hands[STEVE][0] = db_card("Halicarnassus B Wonder Stage 1");
        g.cards[STEVE] = vec![db_card("Ore Vein"), db_card("Foundry")];
        g.hands[GREG][0] = db_card("Lumber Yard");
        g.discard = vec![db_card("Palace")];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "build 1").unwrap();
        cmd(&mut g, GREG, "build 1").unwrap();

        assert_eq!(g.to_resolve.len(), 2, "both DrawDiscard builds must queue");
        assert_eq!(g.whose_turn(), vec![MICK]);

        cmd(&mut g, MICK, "take 1").unwrap();

        assert!(g.cards[MICK].iter().any(|c| c.name == "Palace"));
        assert!(
            g.to_resolve.is_empty(),
            "the second resolver must be pruned once the pile is empty"
        );
    }
```

- [ ] Run: `cargo test -p seven-wonders-1 pruned`. Expected: BOTH FAIL. First: `to_resolve` holds MICK's resolver and `whose_turn()` is `[MICK]` — the permanent lock. Second: `to_resolve` still holds STEVE's resolver after the take, with an empty pile no `take` can ever satisfy.
- [ ] Implement. In `rust/game/seven-wonders-1/src/lib.rs`:

  1. Add two methods to `impl Game`, directly above `take_from_discard` (lib.rs:908):

```rust
    fn has_takeable_discard(&self, player: usize) -> bool {
        self.discard
            .iter()
            .any(|c| !self.cards[player].iter().any(|o| o.name == c.name))
    }

    fn prune_resolvers(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        while let Some(Resolver::DrawDiscard { player }) = self.to_resolve.first() {
            let player = *player;
            if self.has_takeable_discard(player) {
                break;
            }
            self.to_resolve.remove(0);
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" has no cards they can take from the discard pile"),
            ]));
        }
        logs
    }
```

  (`has_takeable_discard`'s ownership check mirrors `take_from_discard`'s rejection at lib.rs:921 exactly — name equality against the player's built cards.)

  2. In `check_hand_complete`, between `let mut logs = self.execute_actions();` (line 255) and `if self.to_resolve.is_empty() {` (line 257), insert:

```rust
        logs.extend(self.prune_resolvers());
```

  3. In `take_from_discard`, immediately after `self.to_resolve.remove(0);` (line 932) and before `if self.to_resolve.is_empty() {`, insert:

```rust
        logs.extend(self.prune_resolvers());
```

  4. On the `post_build_hook` arm (line 410), add a comment above `CardEffect::DrawDiscard { .. } if !self.discard.is_empty() => {`:

```rust
            // Takeability can change between here and when the resolver fires
            // (same-hand discards grow the pile; an earlier resolver's take
            // shrinks it), so it is enforced by prune_resolvers() at hand
            // completion and after each take, not at queue time (b F2).
```

- [ ] Run: `cargo test -p seven-wonders-1` — both new tests PASS, full suite PASSES UNMODIFIED. Specifically re-check `test_take_command_already_build`: MICK's owned Palace sits in the pile alongside STEVE's and GREG's same-hand discards, so the resolver survives the prune, `whose_turn()` is `[MICK]`, and `take 1` (the Palace) still errors — byte-identical assertions. If it fails, the prune is checking the wrong thing — stop.
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs` ; message: `fix(seven-wonders-1): prune unresolvable DrawDiscard resolvers (b F2, WP-15)`

---

### Task 4: store the chosen trade deal in the action (b F9, minor — upgraded from nit by verification)

**Problem (restated, incorporating the verification invalidation):** when a build needs neighbor trades, `choose_build`/`choose_deal` store only an INDEX (`deal: Option<usize>`) into the deal list returned by `can_afford_cost` (lib.rs:806-820, 833-851, 892-898). At execute time, `resolve_deal` (lib.rs:418-431) RECOMPUTES the list and re-indexes: `deals.get(idx).cloned().unwrap_or_default()`. The original finding called the mismatch "verified unreachable (the deal list is append-only between choice and execution)" — **verification proved that false and upgraded the finding to minor**: `execute_actions` runs players in index order, mutating `cards`/`coins` as it goes, so by the time player p's build executes, an earlier-indexed player's build may have added resource cards to a neighbor's tableau; `can_afford_perm`'s early return (`rust/lib/cost/src/lib.rs:181-184` — `if w.can_afford(c) { return (true, vec![vec![c.clone()]]); }`) fires at every recursion level, so the recomputed list can be REORDERED or SHRUNK. The stored index can then (a) select a different deal — paying the wrong neighbor the wrong amount — or (b) fall out of range, where `unwrap_or_default()` silently produces an empty deal and the player **builds without paying any trade coins**. Reachable in normal play.

**Fix (re-derived, matches the finding's recommendation, with the serde-compat shape from re-derivation note 3):** capture the chosen `HashMap<i32, i32>` at choose time inside the action; execution pays exactly what was chosen. The recompute path survives only as a legacy fallback for pre-upgrade saved states.

**Edge cases:** single-deal builds (`deals.len() <= 1`) → the deal is captured immediately at `choose_build` time (previously index 0 was stored), including the empty-map "all own goods" deal; multi-deal builds → `deal_coins` stays `None` with `chosen: false` until `deal N` captures `deals[deal_idx]`; free builds and wonder-free builds → `deal_coins: None`, and `execute_build` skips `resolve_deal` entirely when `free` (unchanged); legacy saved state mid-hand (`deal: Some(idx)`, no `deal_coins` key) → deserializes with `deal_coins: None`, execution takes the old recompute path once, then the action is cleared; a legacy `deal: Some(idx)` out of range still hits `unwrap_or_default` — accepted for the one-hand legacy window, documented in the field comment; the same-turn-trade semantics question (should the deal have been re-validated against pre-turn goods?) is b F4 / WP-16, deliberately untouched (Non-Goals).

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (`Action` enum lines 28-40; `execute_actions` lines 272-280; `execute_build` signature line 299-306 and calls at 320/347; `resolve_deal` lines 418-431; `choose_build` lines 806-851; `choose_deal` lines 892-898)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests:

```rust
    #[test]
    fn legacy_action_json_still_deserializes() {
        // b F9: mid-hand saved states from before the deal_coins field carry
        // only the legacy index; they must keep deserializing.
        let old = r#"{"Build":{"card":2,"free":false,"wonder":false,"deal":0,"chosen":true}}"#;
        let a: Action = serde_json::from_str(old).unwrap();
        assert_eq!(
            a,
            Action::Build {
                card: 2,
                free: false,
                wonder: false,
                deal: Some(0),
                deal_coins: None,
                chosen: true,
            }
        );
    }

    #[test]
    fn stored_deal_paid_despite_mid_turn_divergence() {
        // b F9: the deal chosen at choose time must be the deal paid at
        // execute time, even when a recompute would find a different (or no)
        // deal list. Pre-fix, the recompute here finds NO deals and
        // unwrap_or_default() builds Haven for free.
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        // Haven costs Wood+Ore+Textile. MICK: Ore (city), Loom (textile),
        // Clay Pit; the Wood must come from STEVE's Tree Farm => one deal,
        // 2 coins to the right.
        g.cards = vec![
            vec![db_card("Clay Pit"), db_card("Loom")],
            vec![db_card("Tree Farm")],
            vec![],
        ];
        g.hands[MICK] = vec![db_card("Haven")];

        cmd(&mut g, MICK, "build 1").unwrap();
        let stored = match &g.actions[MICK] {
            Some(Action::Build {
                deal_coins: Some(m),
                chosen: true,
                ..
            }) => m.clone(),
            other => panic!("deal must be captured at choose time, got {:?}", other),
        };
        assert_eq!(stored.get(&DIR_RIGHT), Some(&2));

        // Sabotage: remove the traded-from neighbor's goods so a recompute
        // at execute time cannot reproduce the deal list.
        g.cards[STEVE].clear();

        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap(); // triggers execution

        assert!(g.cards[MICK].iter().any(|c| c.name == "Haven"));
        assert_eq!(g.coins[MICK], 1, "3 starting coins minus the 2-coin deal");
        assert_eq!(
            g.coins[STEVE],
            8,
            "3 starting + 2 trade payment + 3 discard coins"
        );
    }
```

  (Deal-list determinism for the setup: the only source of Wood is STEVE's Tree Farm; combination deals that also buy Ore from a neighbor city cost 4 coins and are filtered by the `coins - coin_cost >= total_deal_cost` check at lib.rs:507 against MICK's 3 coins — so `deals.len() == 1` and `choose_build` captures it with `chosen: true` immediately.)

- [ ] Run: `cargo test -p seven-wonders-1 legacy_action stored_deal`. Expected: both FAIL TO COMPILE (no `deal_coins` field) — that is the red state for a shape change; after implementing, they must pass.
- [ ] Implement. In `rust/game/seven-wonders-1/src/lib.rs`:

  1. `Action` enum (lines 28-40) — replace the `Build` variant with:

```rust
    Build {
        card: usize,
        free: bool,
        wonder: bool,
        /// Legacy: pre-upgrade saved states stored the chosen deal as an
        /// index into a deal list recomputed at execute time. Read only as a
        /// fallback when `deal_coins` is `None`; new states always write
        /// `None` here. Kept so old mid-hand states keep deserializing.
        deal: Option<usize>,
        /// The chosen trade payment (direction -> coins), captured at choose
        /// time so mid-turn state changes cannot reorder or shrink a
        /// recomputed deal list (b F9).
        #[serde(default)]
        deal_coins: Option<HashMap<i32, i32>>,
        chosen: bool,
    },
```

  2. `resolve_deal` (lines 418-431) — new signature and body:

```rust
    fn resolve_deal(
        &self,
        player: usize,
        cost: &Cost<Good>,
        deal: Option<usize>,
        deal_coins: Option<&HashMap<i32, i32>>,
    ) -> HashMap<i32, i32> {
        if let Some(coins) = deal_coins {
            return coins.clone();
        }
        // Legacy fallback for pre-upgrade pending actions only (b F9).
        match deal {
            Some(idx) => {
                let (_, deals) = self.can_afford_cost(player, cost);
                deals.get(idx).cloned().unwrap_or_default()
            }
            None => HashMap::new(),
        }
    }
```

  3. `execute_build` (lines 299-306) — add the parameter `deal_coins: Option<&HashMap<i32, i32>>` after `deal: Option<usize>`, and pass it through at both `resolve_deal` call sites (lines 320 and 347): `self.resolve_deal(player, &stage_card.cost, deal, deal_coins)` / `self.resolve_deal(player, &card.cost, deal, deal_coins)`.

  4. `execute_actions` (lines 272-280) — extend the destructure and call:

```rust
                    Action::Build {
                        card,
                        free,
                        wonder,
                        deal,
                        deal_coins,
                        ..
                    } => {
                        let (build_logs, built) =
                            self.execute_build(p, *card, *free, *wonder, *deal, deal_coins.as_ref());
```

  5. `choose_build` — wonder branch (lines 808-820): replace the `(deal, chosen)` computation and action construction with:

```rust
            let (deal_coins, chosen) = if deals.len() <= 1 {
                (deals.into_iter().next(), true)
            } else {
                (None, false)
            };

            self.actions[player] = Some(Action::Build {
                card: card_idx,
                free,
                wonder: true,
                deal: None,
                deal_coins,
                chosen,
            });
```

     Free branch (lines 825-831): add `deal_coins: None,` alongside `deal: None,`. Regular branch (lines 838-851): same replacement as the wonder branch but with `free: false, wonder: false`.

  6. `choose_deal` (lines 892-898) — after the existing `deal_idx >= deals.len()` check, store the map instead of the index:

```rust
                self.actions[player] = Some(Action::Build {
                    card,
                    free,
                    wonder,
                    deal: None,
                    deal_coins: Some(deals[deal_idx].clone()),
                    chosen: true,
                });
```

  (`pub_state`'s `actions_chosen` match, `command_parser`'s `Action::Build { chosen: false, .. }` at command.rs:39, and `choose_deal`'s own match all use `..` rest patterns — no changes needed there; the compiler confirms.)

- [ ] Run: `cargo test -p seven-wonders-1` — both new tests PASS, full suite PASSES (`test_take_command*` and `test_free_build` exercise the single-deal and free paths).
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs` ; message: `fix(seven-wonders-1): store chosen trade deal in the action (b F9, WP-15)`

---

### Task 5: military-conflict log uses a player node (b F12, nit)

**Problem (restated):** `military_conflicts` (`rust/game/seven-wonders-1/src/lib.rs:770-777`) interpolates the defeated player's RAW index into the text — `" defeated player {} in military conflict (+{} victory, +1 defeat)"` — instead of the `N::Player` node every other log uses. The rendered name/color is lost, and the raw index is off by one against the 1-based numbering users see elsewhere.

**Fix:** split the text around an `N::Player(right)` node.

**Edge cases:** multiple conflicts in one age → each log restructured identically by the single format site; `tokens` interpolation (1/3/5 by age) unchanged.

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (lines 770-777)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test (tests live inside lib.rs, so the private `military_conflicts` is callable directly):

```rust
    #[test]
    fn military_log_uses_player_node() {
        // b F12: the defeated player must be an N::Player node, not a raw
        // zero-based index in the text.
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Stockade")];
        let logs = g.military_conflicts();
        assert_eq!(logs.len(), 1);
        let rendered = brdgme_markup::to_string(&logs[0].content);
        assert_eq!(
            rendered,
            "{{player 0}} defeated {{player 1}} in military conflict (+1 victory, +1 defeat)"
        );
    }
```

- [ ] Run: `cargo test -p seven-wonders-1 military_log`. Expected: FAILS — rendered text reads `... defeated player 1 in military conflict ...`.
- [ ] Implement: replace the log push at lines 770-777 with:

```rust
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(" defeated "),
                    N::Player(right),
                    N::text(format!(
                        " in military conflict (+{} victory, +1 defeat)",
                        tokens
                    )),
                ]));
```

- [ ] Run: `cargo test -p seven-wonders-1` — full suite PASS.
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs` ; message: `fix(seven-wonders-1): military log names the defeated player (b F12, WP-15)`

---

### Task 6: remove the dead start_hand() indirection (b F13, nit)

**Problem (restated):** `start_hand` (`rust/game/seven-wonders-1/src/lib.rs:177-180`) resets `self.actions` and returns an empty log vec. Both call sites are inside `end_hand` (lines 208-210 and 214), and `end_hand` is only reached from `check_hand_complete` (after `execute_actions`, which already resets `self.actions` at line 295) and from `take_from_discard` (where `execute_actions` has likewise already run). The reset is provably redundant and the fn is pure indirection. (Verification confirmed: sole reset dependency is lib.rs:295.)

**Fix:** delete the fn; simplify both call sites.

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (lines 177-180, 208-210, 213-214)

**Steps:**

- [ ] Delete `fn start_hand(&mut self) -> Vec<Log> { ... }` (lines 177-180).
- [ ] In `end_hand`, replace lines 208-210 (`let sh_logs = self.start_hand(); logs.extend(sh_logs); return logs;`) with `return logs;`.
- [ ] In `end_hand`, replace the tail (lines 213-214) `self.pass_hands(); self.start_hand()` with:

```rust
        self.pass_hands();
        vec![]
```

- [ ] Run: `cargo test -p seven-wonders-1` — full suite PASS (behavioral no-op; `test_free_build` drives six full hand turnovers and is the regression lock).
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs` ; message: `refactor(seven-wonders-1): remove dead start_hand indirection (b F13, WP-15)`

---

### Task 7: guard out-of-range player in player_state / command_parser (b F10, nit)

**Problem (restated):** `player_state` (`rust/game/seven-wonders-1/src/lib.rs:980-986`) indexes `self.hands[player]`, and `command_parser` indexes `self.actions[player]` (command.rs:39) and `self.hands[player]` (command.rs:54, 58) with no bounds check. Sibling crates guard this: `category-5-2` uses `self.hands.get(player).cloned().unwrap_or_default()` in `player_state`, `sushi-go-2` an equivalent `if/else`. Only reachable via a framework-passed out-of-range player (upstream-guarded), so severity nit — but `command_spec` (lib.rs:1177) calls `command_parser` with no prior `assert_player_turn`, so the panic path is one framework bug away.

**Fix:** match the `category-5-2` pattern in `player_state`; early-return `None` for out-of-range players at the top of `command_parser`.

**Edge cases:** in-range players → identical behavior (`get(player).cloned()` is `Some` for all real players); the guard uses `self.players`, which equals `hands.len()` and `actions.len()` at all times (set once in `start_game`, resized per-round to the same count).

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (`player_state`, line 984), `rust/game/seven-wonders-1/src/command.rs` (`command_parser`, after line 19)
- Test: `rust/game/seven-wonders-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn out_of_range_player_is_guarded() {
        // b F10: a framework-passed bad index must degrade gracefully, not
        // panic (sibling-crate pattern: category-5-2, sushi-go-2).
        let g = new_game();
        assert!(g.player_state(99).hand.is_empty());
        assert!(g.command_parser(99).is_none());
        assert!(g.command_spec(99).is_none());
    }
```

- [ ] Run: `cargo test -p seven-wonders-1 out_of_range_player`. Expected: FAILS by PANIC — `index out of bounds` in `player_state`.
- [ ] Implement:
  1. In `player_state` (lib.rs:984), change `hand: self.hands[player].clone(),` to `hand: self.hands.get(player).cloned().unwrap_or_default(),`.
  2. In `command_parser` (`rust/game/seven-wonders-1/src/command.rs`), after the `if self.finished { return None; }` block (lines 17-19), insert:

```rust
        if player >= self.players {
            return None;
        }
```

- [ ] Run: `cargo test -p seven-wonders-1` — new test PASSES, full suite PASS.
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs rust/game/seven-wonders-1/src/command.rs` ; message: `fix(seven-wonders-1): guard out-of-range player in state and parser (b F10, WP-15)`

---

### Task 8: close the verified test-coverage gaps (b F14, minor)

**Problem (restated, per the verification-adjusted gap list — re-derivation note 5):** nothing covers military conflict resolution/token values, pass direction per age, `Bonus`/guild scoring, multi-deal selection via `deal N`, seed determinism, or a full game end-to-end. (MimicGuild is already tested and excluded; Halicarnassus B VP landed in Task 1; the DrawDiscard resolver paths landed in Task 3.)

**Test-design constraints:** private methods (`military_conflicts`, `pass_hands`, `score_science`) are callable because `mod tests` lives in lib.rs. Per `docs/CODING.md`, never assert turn order with `assert_ne!` on a player index — the replay test asserts on `whose_turn()` set contents and final status only. Determinism is asserted by comparing typed values (`Vec<Vec<Card>>`, city names) — NOT serialized JSON, because `Cost` wraps a `HashMap` whose serialization order differs between instances.

**Files:**
- Modify: `rust/game/seven-wonders-1/src/lib.rs` (inline `mod tests` only — no production code changes in this task)

**Steps:**

- [ ] Add the tests below to `mod tests`. Each is documented with the exact arithmetic so a failure localizes instantly.

```rust
    #[test]
    fn military_conflict_awards_tokens_per_age() {
        // b F14: victory tokens are 2*age - 1; each loss is one defeat token.
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Stockade")]; // strength 1 vs 0 vs 0
        g.round = 1;
        g.military_conflicts();
        // Only MICK (1) beats his right neighbor STEVE (0); STEVE vs GREG and
        // GREG vs MICK are not victories for the attacker.
        // NOTE: the live loop battles each player against their RIGHT
        // neighbor only (official rules battle both neighbors). No finding
        // covers that deviation; this test locks CURRENT behavior. If WP-16's
        // adjudication ever extends to it, update this test there.
        assert_eq!(g.victory_tokens, vec![1, 0, 0]);
        assert_eq!(g.defeat_tokens, vec![0, 1, 0]);

        g.round = 3;
        g.military_conflicts();
        assert_eq!(g.victory_tokens, vec![1 + 5, 0, 0]);
        assert_eq!(g.defeat_tokens, vec![0, 2, 0]);
    }

    #[test]
    fn hands_pass_toward_lower_index_in_odd_ages() {
        // b F14: odd ages take the hand from the next-higher index, even ages
        // from the next-lower (pass_hands, lib.rs).
        let mut g = new_game();
        let originals = g.hands.clone();
        g.round = 1;
        g.pass_hands();
        assert_eq!(g.hands[0], originals[1]);
        assert_eq!(g.hands[1], originals[2]);
        assert_eq!(g.hands[2], originals[0]);

        g.round = 2;
        g.pass_hands();
        assert_eq!(g.hands, originals, "one pass each way must round-trip");
    }

    #[test]
    fn haven_scores_own_raw_cards() {
        // Haven: 1 VP per own Raw card (DIR_SELF). Coins zeroed to isolate.
        let mut g = new_game();
        g.coins = vec![0, 0, 0];
        g.cards[MICK] = vec![db_card("Haven"), db_card("Lumber Yard")];
        assert_eq!(g.player_vp(MICK), 1);
    }

    #[test]
    fn strategists_guild_scores_neighbor_defeats() {
        // Strategists Guild: 1 VP per neighbor defeat token (DIR_NEIGHBOURS).
        let mut g = new_game();
        g.coins = vec![0, 0, 0];
        g.cards[STEVE] = vec![db_card("Strategists Guild")];
        g.defeat_tokens = vec![2, 0, 1];
        // Neighbors of STEVE hold 2 + 1 tokens; own tokens are 0.
        assert_eq!(g.player_vp(STEVE), 3);
    }

    #[test]
    fn builders_guild_scores_wonder_stages_all_directions() {
        // Builders Guild: 1 VP per wonder stage self + both neighbors.
        let mut g = new_game();
        g.coins = vec![0, 0, 0];
        g.cards[MICK] = vec![db_card("Rhodes A Wonder Stage 1")];
        g.cards[STEVE] = vec![db_card("Builders Guild")];
        g.cards[GREG] = vec![db_card("Rhodes A Wonder Stage 1")];
        // STEVE: 0 own + 1 (MICK) + 1 (GREG) = 2 VP; the guild card itself
        // is CardKind::Guild, not Wonder.
        assert_eq!(g.player_vp(STEVE), 2);
    }

    #[test]
    fn deal_command_selects_between_multiple_deals() {
        // b F14: with the missing Wood available from BOTH neighbors, the
        // player must be able to pick the second deal and pay that neighbor.
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![
            vec![db_card("Loom")],
            vec![db_card("Tree Farm")],
            vec![db_card("Tree Farm")],
        ];
        g.hands[MICK] = vec![db_card("Haven")];

        let (_, deals) = g.can_afford_cost(MICK, &db_card("Haven").cost);
        assert_eq!(deals.len(), 2, "wood from either neighbor = two deals");
        assert_ne!(deals[0], deals[1]);

        cmd(&mut g, MICK, "build 1").unwrap();
        assert!(matches!(
            g.actions[MICK],
            Some(Action::Build { chosen: false, .. })
        ));
        // MICK still to act: the deal choice is pending.
        assert!(g.whose_turn().contains(&MICK));

        cmd(&mut g, MICK, "deal 2").unwrap();
        let expected = deals[1].clone();
        let coins_before = g.coins.clone();

        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap(); // triggers execution

        let paid: i32 = expected.values().sum();
        assert_eq!(paid, 2, "one wood at the base trade rate");
        assert_eq!(g.coins[MICK], coins_before[MICK] - paid);
        for (&dir, &coins) in &expected {
            let neighbor = if dir == DIR_LEFT { GREG } else { STEVE };
            // +3 is that neighbor's own discard payment.
            assert_eq!(g.coins[neighbor], coins_before[neighbor] + 3 + coins);
        }
    }

    #[test]
    fn same_seed_same_game() {
        // b F14: starts are deterministic per seed. Compare typed values, not
        // JSON — Cost's HashMap serializes in instance-dependent order.
        let (a, _) = Game::start_game(5, 123).unwrap();
        let (b, _) = Game::start_game(5, 123).unwrap();
        assert_eq!(
            a.cities.iter().map(|c| &c.name).collect::<Vec<_>>(),
            b.cities.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        assert_eq!(a.hands, b.hands);
    }

    #[test]
    fn full_game_discard_replay_finishes() {
        // b F14: a deterministic all-discard game must run 3 ages to a
        // finished status with placings for everyone, never stalling.
        let p = players();
        let (mut g, _) = Game::start_game(3, 7).unwrap();
        let mut guard = 0;
        while !g.is_finished() {
            guard += 1;
            assert!(guard < 100, "game did not finish — state machine stalled");
            let turn = g.whose_turn();
            assert!(!turn.is_empty(), "active game with nobody to act");
            for pl in turn {
                g.command(pl, "discard 1", &p).unwrap();
            }
        }
        assert_eq!(g.round, 3);
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings.len(), 3),
            s => panic!("expected finished status, got {:?}", s),
        }
    }
```

  (`deal_command_selects_between_multiple_deals` note: `choose_deal` recomputes the deal list from state identical to the test's own `can_afford_cost` call — nothing mutates between them, and list ORDER is driven by the stable `Vec` ordering of `with` (own, then left, then right options) in `can_afford_cost`, so `deals[1]` is the same deal in both computations. Combination deals buying Ore from a neighbor city cost 4 > 3 coins and are filtered at lib.rs:507, keeping the list at exactly two entries. Requires Task 4 (it reads `deal_coins` indirectly via the payment assertions) — this task is ordered after it.)

- [ ] Run: `cargo test -p seven-wonders-1` — all new tests PASS on the already-fixed code, full suite PASS. If `military_conflict_awards_tokens_per_age` or the guild tests fail, the production code is NOT to be changed in this task — re-derive the expected value against `military_conflicts`/`player_vp`/`bonus_count` and fix the TEST; report if the mismatch implies a real (unfindinged) bug.
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs` ; message: `test(seven-wonders-1): cover conflicts, passing, guilds, deals, determinism (b F14, WP-15)`

---

### Task 9: split scoring and trade out of lib.rs (b F15, nit) + final gate

**Problem (restated):** lib.rs mixes the state machine, trading, scoring, `Gamer` impl, and tests (1,565 lines pre-package; larger now with Task 8). The finding calls the split optional; scoring and trading are the two cohesive clusters worth lifting. This is a pure code move — NO behavioral change; the full test suite is the proof.

**Rust visibility facts the move relies on:** `scoring`/`trade` are child modules of the crate root, so they may call crate-root-private items (`bonus_count`, `BASE_TRADE_COST`, `DISCOUNTED_TRADE_COST`) without visibility changes. The reverse is NOT true: items moved into a child module that are still called from lib.rs or from `mod tests` (a sibling of the new modules) need `pub(crate)`. Inherent-method availability does not depend on the module being `pub` — only the methods' own visibility matters.

**Files:**
- Create: `rust/game/seven-wonders-1/src/scoring.rs`, `rust/game/seven-wonders-1/src/trade.rs`
- Modify: `rust/game/seven-wonders-1/src/lib.rs`

**Steps:**

- [ ] Create `rust/game/seven-wonders-1/src/scoring.rs` containing an `impl Game` block with these methods MOVED VERBATIM from lib.rs (including the Task 1 `DrawDiscard` arm), with only the listed visibility changes:
  - `science_vp` (stays `pub`)
  - `science_permute` (stays private — only called within this module)
  - `score_science` (private → `pub(crate)` — called from `mod tests` in lib.rs)
  - `player_vp` (stays `pub`)
  - `mimic_guild_vp` (stays private — only called by `player_vp` here)

  Header imports:

```rust
use std::collections::HashMap;

use crate::Game;
use crate::card::{CardEffect, CardKind, DIR_LEFT, DIR_NEIGHBOURS, Field, all_fields};
```

  (`bonus_count`, `victory_tokens`, `coins`, etc. remain defined on `Game` in the crate root and are visible here. If the compiler flags a missing import, add it — do not restructure.)

- [ ] Create `rust/game/seven-wonders-1/src/trade.rs` containing an `impl Game` block with these methods MOVED VERBATIM (including the Task 4 `resolve_deal` shape):
  - `can_afford_cost` (stays `pub`)
  - `resolve_deal` (private → `pub(crate)` — called from `execute_build` in lib.rs)
  - `pay_cost` (private → `pub(crate)` — called from `execute_build` in lib.rs)
  - `player_goods_options` (stays private)
  - `trade_cost_per_good` (stays private)

  Header imports:

```rust
use std::collections::HashMap;

use brdgme_cost::{Cost, can_afford_perm};

use crate::card::{CardEffect, DIR_LEFT, DIR_RIGHT, Good};
use crate::{BASE_TRADE_COST, DISCOUNTED_TRADE_COST, Game};
```

- [ ] In `rust/game/seven-wonders-1/src/lib.rs`:
  - Add `mod scoring;` and `mod trade;` after `pub mod render;` (line 3). Private modules are sufficient — nothing names them in a path.
  - Delete the moved method bodies from the `impl Game` block.
  - Remove `can_afford_perm` from the `use brdgme_cost::...` import (now used only in trade.rs); let the compiler flag any other import that became unused (e.g. `Cost` if no remaining lib.rs signature names it — `choose_deal`'s local `cost` binding is inferred) and remove exactly those.
  - `attack_strength` and `bonus_count` STAY in lib.rs: `attack_strength` belongs to the military path (`military_conflicts`), and `bonus_count` is shared by `post_build_hook` (lib.rs) and the scoring module (which can reach it as a root-private item).
- [ ] Run: `cargo build -p seven-wonders-1` — compiles. Any visibility error means a caller was missed: fix by adding `pub(crate)` to the moved method, nothing else.
- [ ] Run: `cargo test -p seven-wonders-1` — FULL suite passes, zero test edits (the move is behavior-neutral; `test_science_vp` exercising `Game::score_science` proves the `pub(crate)` path).
- [ ] `cargo clippy -p seven-wonders-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add rust/game/seven-wonders-1/src/lib.rs rust/game/seven-wonders-1/src/scoring.rs rust/game/seven-wonders-1/src/trade.rs` ; message: `refactor(seven-wonders-1): split scoring and trade modules (b F15, WP-15)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| b F1 Halicarnassus B DrawDiscard VP never scored | major | Add `CardEffect::DrawDiscard { vp } => vp += vp,` arm + test | CONFIRMED — fixed (Task 1) | Live card data carries vp 2/1/0 (B) and 0 (A stage 2); the rec's literal arm self-shadows `vp`, corrected to a distinct binding. Both B (scored) and A (no-op) locked by tests. |
| b F2 DrawDiscard resolver permanent soft-lock | major | Queue-time takeable filter, re-check at resolve time; or a `pass` command | CONFIRMED — fixed, implementation ADJUSTED (Task 3) | Queue-time filtering is insufficient (multi-resolver takes invalidate later) AND wrong-sided (it runs before same-hand discards reach the pile, would cancel legitimate takes and break `test_take_command_already_build`). Fixed with `prune_resolvers()` at hand completion and after each take; queue guard untouched; `pass` command rejected as needless surface. PORTING_NOTES claim now true at fire time. |
| b F3 auto-discarded 7th card pays 3 coins | major | Drop the coin line in the auto-discard path; update the log | CONFIRMED — fixed (Task 2) | Live lib.rs:195 pays `DISCARD_COINS`; official rules pay nothing and PORTING_NOTES lists no such quirk (work-packages: not parity-gated). Log rec moot — the existing text never mentioned coins. `execute_discard`'s payment (chosen discards) kept. |
| b F9 deal re-validated by index into recomputed list | minor (upgraded from nit by verification) | Store the chosen deal map in `Action::Build` at choose time | ANALYSIS OVERTURNED by verification; fix CONFIRMED — implemented with serde-compat shape (Task 4) | The original "verified unreachable / append-only" claim is FALSE: mid-`execute_actions` builds mutate neighbor goods and `can_afford_perm`'s early return (lib/cost:181-184) reorders/shrinks the recomputed list — wrong-neighbor payment or a free build via `unwrap_or_default()` is reachable in normal play. Implemented as new `#[serde(default)] deal_coins` field; legacy `deal` index retained as a one-hand deserialization fallback so no saved state breaks. |
| b F10 unguarded player indexing | nit | Match the sibling-crate defensive pattern | CONFIRMED — fixed (Task 7) | `player_state` adopts category-5-2's `get().cloned().unwrap_or_default()`; `command_parser` early-returns `None` for `player >= self.players` (covers actions and hands indexing; `command_spec` reaches the parser with no upstream turn assert). |
| b F12 military log uses raw player index | nit | Use `N::Player(right)` | CONFIRMED — fixed (Task 5) | Also cures the off-by-one vs 1-based display numbering noted by verification. Exact rendered string locked by test. |
| b F13 start_hand() dead-weight | nit | Inline or remove | CONFIRMED — fixed (Task 6) | Reset re-verified redundant against live code: `execute_actions` (lib.rs:295) resets before every `end_hand` path. Removed. |
| b F14 test coverage gaps | minor | Add scoring/conflict tests + deterministic full-game replay | ADJUSTED per verification — implemented (Task 8) | MimicGuild excluded (already tested at lib.rs:1504); remaining gaps landed: conflicts/token values, pass direction, Haven/Strategists/Builders scoring, `deal N` selection, seed determinism, full replay. Halicarnassus B VP test landed with Task 1; resolver paths with Task 3. |
| b F15 lib.rs grab-bag | nit | Optional split into scoring.rs/trade.rs | CONFIRMED — implemented (Task 9) | Mechanical verbatim move, `pub(crate)` only where lib.rs/tests call in; ordered last so every fix lands against cited line numbers; full suite + rust-test.sh prove the no-op. |

## Cross-package coordination points

- **WP-16 (D-27/D-28)** owns b F4-F8. Two touchpoints created here: (1) Task 4 stores the deal chosen against CURRENT neighbor goods — if WP-16's b F4 resolution mandates hand-start snapshots, the capture point in `choose_build`/`choose_deal` is where the snapshot plugs in; the stored-map mechanism itself is agnostic. (2) Task 3's prune logs "has no cards they can take" — if WP-16's b F8 resolution exposes discard contents, that message needs no change.
- **WP-08** (epilogue dedup) will collapse the six `is_finished()` blocks in `command()` (lib.rs:1011-1137); nothing in this package touches them, so no conflict — but WP-08 rebasing over this package must account for lib.rs line drift from Tasks 1-9.
- **WP-09 (D-36)** deserialized-state trust: Task 4's legacy `deal: Some(idx)` fallback still ends in `unwrap_or_default()` for crafted out-of-range indices — flagged as within WP-09's remit if D-36 lands on hardening; it is unreachable from states this code writes (new states always use `deal_coins`).
- **WP-10 (D-33)** redaction: `PlayerState`/`PubState` shapes are unchanged here; the b F8 discard-visibility question stays with WP-16.
- **Unfindinged deviation flagged during Lead review:** `military_conflicts` battles each player against their RIGHT neighbor only; official 7 Wonders resolves conflicts against BOTH neighbors. No batch-b finding covers this. Task 8's `military_conflict_awards_tokens_per_age` locks the current behavior deliberately; the deviation itself belongs with the WP-16 (D-27/D-28) rules adjudications and should be raised there.
- The `full_game_discard_replay_finishes` test (Task 8) doubles as the state-machine regression lock the review asked for — future packages touching this crate should keep it green rather than replacing it.
