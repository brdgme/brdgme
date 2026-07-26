# WP-29: red7-1 cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

## SEQUENCING - READ FIRST (this package lands AFTER WP-30)

`planning/BACKLOG.md:103` lists WP-29 as "READY, but sequence after WP-30". **WP-30** (`planning/work-packages.md:243-247`) is **BLOCKED-ON-DECISION(D-29, D-40)** and owns `e F30` — "Player with zero rule-fulfilling cards is treated as winning (official rules: cannot win)" — whose fix territory is `rust/game/red7-1/src/card.rs` `leader()` and `rust/game/red7-1/src/lib.rs` `leader_with_suit()`. D-29 (`planning/decisions-needed.md:390-400`) reads as follows (the source file hard-wraps; the quote below joins its lines, content is unaltered):

> Context: a player with zero rule-fulfilling cards is treated as winning; official rules say they cannot win. Adopting official needs a defined outcome when ALL players have empty sets.
> Options:
> - A. Official: empty set cannot win; all-empty resolved by elimination order (last player standing by highest card per red7 tie rules).
> - B. Document the deviation.
> Recommendation: A - the current behaviour lets a player win with nothing, which is strategy-breaking, and DATA_DOCS already contradicts the code.

D-40 (`planning/decisions-needed.md:520-531`) is the write-only-stats keep-or-drop decision; it touches acquire-1 and lost-cities-1/-2 only — **nothing in red7-1** (red7's `Status::Finished` emits `stats: vec![]`, lib.rs:408). D-40 therefore cannot affect any text in this spec; it only keeps WP-30 blocked.

**How to sequence this package:**

- **Tasks 1-3 (F33, F35, F34) are code-only and completely D-29-independent.** They touch `lib.rs:16`, `lib.rs:22-24` and a doc comment above `lib.rs:237`. None of them reads, writes or reasons about winning-set contents. **Land them whenever, even while WP-30 is still blocked.**
- **Tasks 4-5 (F31, F32) are the doc rewrites and are gated.** Task 4 opens with a mandatory precondition checkpoint.
- **Precondition for Task 4 (DATA_DOCS.md, F31):** determine whether WP-30 has landed and which D-29 option was chosen. Task 4 supplies **two verbatim variants** of exactly one sentence (the all-empty-winning-sets sentence). Pick variant **B-CURRENT** if WP-30 has not landed or D-29 chose option B; pick variant **A-OFFICIAL** if WP-30 landed with D-29 option A. Everything else in Task 4 is identical either way.
- **Precondition for Task 5 (RULES.md, F32):** none — the replacement text for the Turn and Scoring sections is D-29-independent (verified sentence by sentence in Task 5). **However**, the *untouched* lines `RULES.md:31-32` ("At the end of your turn, if you are not the leader under the current rule, you are eliminated") describe leader-based elimination, and if D-29 option A lands, WP-30 must decide whether to add a "you cannot be the leader with no rule-fulfilling cards" clause there. **That line belongs to WP-30, not to this package. Do not touch it.**
- **Sentences that must be re-checked before committing Task 4 if WP-30 landed first:** (i) the all-empty sentence (variants provided); (ii) the sentence "The leader is the player with the most cards in their winning set" — under D-29 option A a player whose winning set is empty is excluded from leadership entirely, so the A-OFFICIAL variant carries the extra clause. No other sentence in either doc depends on D-29.

---

**Goal:** Make red7-1's two bot-facing docs match the implementation — replace DATA_DOCS.md's fictional second tie-break clause with the tie-break the code actually performs (e F31), and correct RULES.md's Turn section (the play-then-discard combo the code permits and the fact that discarding ends the turn) and Scoring section (only rule-meeting palette cards score; the deck can also end the game) (e F32) — and clear three code nits: replace the dead aliased `PubCard`/`PubSuit` re-export with the sibling-crate `pub use card::*;` convention (e F33), make `end_points` saturate instead of underflowing (e F35), and document `leader_with_suit`'s non-empty-player precondition at the site that would panic (e F34, **comment only**).

**Architecture — how red7-1 decides the leader and the score (read this before touching any doc text):**

`rust/game/red7-1` (package `red7-1`, lib `red7_1`). 2-4 players, Advanced Red7, seeded `GameRng` persisted in `Game` (`lib.rs:38-39`). Modules: `card` (317 lines), `command` (106), `render` (186), `lib.rs` (750, includes the inline `#[cfg(test)] mod tests` at `lib.rs:549-750` — `#[cfg(test)]` on :549, `mod tests {` on :550, closing `}` on :750 — note it is `tests`, **plural**; 17 of the 27 game crates use `mod test` singular and only 10 use `tests`, which is e F9 / WP-65's sweep, not this package's). `rust/game/red7-1/tests/contract.rs` (the crate's only integration test file) holds `assert_gamer_contract::<Game>()` in `fn game_contract()`. `mod card;` is a **private** module (`lib.rs:12`); `Card`/`Suit` reach the public API only through the re-export at `lib.rs:16` and through `PubState`/`PlayerState` fields.

**Derived winner / tie-break rule (from live source — this is what the F31 doc text must encode):**

1. `Game::leader()` (`lib.rs:233-235`) is `leader_with_suit(self.current_rule())`; `current_rule()` (`lib.rs:254-259`) is the top discard's suit, defaulting to `Suit::Red` on an empty pile.
2. `leader_with_suit` (`lib.rs:237-252`) walks players `0..num_players`, **skips eliminated players** (`lib.rs:243-245`), records the surviving indices in `player_map` in ascending order, and for each survivor computes `rule_fn(&self.palettes[p])` — the player's **winning set**, a subset of their palette. `suit_rule` (`card.rs:285-295`) maps: Red -> `highest_card` (1 card, `card.rs:163-170`), Orange -> `cards_of_one_number` (largest equal-rank group, `:172`), Yellow -> `cards_of_one_color` (largest single-suit group, `:197`), Green -> `most_even_cards` (**all** even-rank cards, `:230`), Blue -> `cards_of_different_colors` (highest card of each distinct suit, `:234`), Indigo -> `cards_that_form_a_run` (longest consecutive-rank run, `:250`), Violet -> `most_cards_below_4` (**all** cards of rank < 4, `:278`).
3. `card::leader(&palettes)` (`card.rs:297-317`) picks the winner among those winning sets: start with index 0, then for each later set take it if `p.len() > leader.len()`, or if `p.len() == leader.len()` **and** its max `rank_key()` is strictly greater (`card.rs:311`). `rank_key()` is `(rank, suit.ordinal())` (`card.rs:134-136`) with `ordinal()` Violet=0 … Red=6 (`card.rs:89-99`) — so **rank first, then colour, Red highest and Violet lowest**. An empty set's max falls back to `(0, 0)` (`card.rs:309-310`).
4. **There is no third tie-break.** The deck is 49 distinct cards (`full_deck`, `card.rs:145-153`) and palettes are disjoint, so two *non-empty* winning sets can never tie on their highest card. The strict `>` at `card.rs:311` therefore only falls through when both sets are **empty**, in which case the earliest survivor in `player_map` order — i.e. the **lowest-numbered non-eliminated player** — remains leader. Empty winning sets are possible for Green (no even cards) and Violet (no cards below 4) — and trivially for an empty palette; every other rule returns at least one card for a non-empty palette. **This all-empty case is exactly `e F30` / D-29 and is owned by WP-30.**
5. `leader_with_suit` then returns `(player_map[l_index], palette)` (`lib.rs:251`) — the `player_map` index lookup that `e F34` is about.

**Derived scoring / game-end rule (this is what the F32 doc text must encode):**

- A round ends inside `end_turn` when `remaining_players().len() == 1` (`lib.rs:166-169`).
- `end_round` (`lib.rs:175-216`) calls `self.leader()` and scores **`leader_palette`** — the rule-meeting subset from step 2 above, **not** the whole palette: `points(&leader_palette)` (`lib.rs:177`), `scored_cards[leader_idx].extend(&leader_palette)` (`:178`), and the scored cards are then **removed from the palette** (`:179`). `points` is the sum of ranks (`card.rs:155-157`, `Card::points` = `rank`, `card.rs:126-128`).
- After the round log, if **any** player's total is `>= end_points(num_players)` the game ends (`lib.rs:206-212`). `end_points` = `50 - players*5` -> 40/35/30 for 2/3/4 (`lib.rs:22-24`), matching `RULES.md:52-55`.
- Otherwise `start_round()` runs (`lib.rs:75-104`); it returns every card to the deck and, **if the deck holds fewer than `players * 8` cards, ends the game instead of dealing** (`lib.rs:88-91`). RULES.md today does not mention this second end condition.
- Final placings come from `gen_placings` over total points (`lib.rs:401-409`), so the winner is the highest total, whether the game ended on the target score or on deck exhaustion.

**Derived turn structure (F32 part 1):**

- `can_play` requires `current_player == player && !has_played && !finished` (`lib.rs:283-285`) — **one play per turn**; `play` (`lib.rs:295-314`) moves the card to the palette, sets `has_played = true`, and **does not end the turn**.
- `can_discard` requires only `current_player == player && !finished` (`lib.rs:287-289`) — it **ignores `has_played`**, so **play-then-discard in the same turn is legal**. `discard` (`lib.rs:316-353`) rejects the card unless `leader_with_suit(card.suit).0 == player` (`lib.rs:325-330`), draws one card when `card.rank as usize > self.palettes[player].len()` (`lib.rs:346-348`), then sets `has_played = true` and **calls `end_turn` immediately** (`lib.rs:350-351`) — so nothing can follow a discard, and `done` after a discard is impossible.
- `done` (`lib.rs:355-369`) eliminates the player if `!has_played`, then ends the turn.
- `end_turn` (`lib.rs:154-173`) eliminates the current player if they are not the leader under the current rule.
- `start_turn` (`lib.rs:146-152`) eliminates a player whose hand is empty. **RULES.md does not document this; see "Cross-package / newly discovered" — not fixed here.**

**Tech Stack:** Rust 1.97.0, edition 2024, workspace at `/home/beefsack/Development/brdgme/rust` (channel + rustfmt/clippy pinned by `rust-toolchain.toml`). One crate touched: `red7-1`. Both Markdown files are compiled into the binary via `include_str!` — `RULES.md` at `lib.rs:533`, `DATA_DOCS.md` at `lib.rs:537` — so a doc edit **does** change the crate's build output and is served by `Request::Rules` (`docs/authoring/RULES_AUTHORING.md`, "Storage": the operator reconcile stores the `rules()` string in `game_versions.rules`; the bot reads it from the DB).

**Global Constraints:**

- Run all commands from `/home/beefsack/Development/brdgme/rust`. **Per-crate only:** `cargo test -p red7-1`. NEVER workspace-wide `cargo build`/`check`/`test` (AGENTS.md "Resource constraints": ~30 binaries link, spikes RAM/disk).
- Every task ends with `cargo clippy -p red7-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- All existing tests must keep passing **unmodified**. red7-1 has 18 inline tests in `lib.rs:549-750` plus `game_contract`. Task 2 **adds** a new test rather than extending `test_end_points` (`lib.rs:609-614`) precisely so no existing test is edited.
- **No serialized shape may change.** `Game`, `PubState`, `PlayerState`, `Card` and `Suit` all derive `Serialize`/`Deserialize` and ride the DB; live saved games must keep deserializing. No task here changes a field, a type, a variant or an order.
- **No function signature may change.** Tasks 1-3 keep `end_points(usize) -> u32` and `leader_with_suit(&self, Suit) -> (usize, Vec<Card>)` exactly as they are (rationale in the disposition table).
- Run the full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` before the **final** commit of the package (it provisions the throwaway Postgres/NATS; DB-backed web failures in a bare local run are pre-existing, backlog #40 — not a regression).
- **Line numbers below are LIVE numbers as of the head of this package, and Tasks 1-3 shift them. Locate by NAME, verify by number.** Concretely: Task 1 is a 1-for-1 line replacement (delta 0). Task 2 appends a ~10-line test after `lib.rs:614` (no effect on anything before it) and then replaces `lib.rs:22-24` (3 lines) with an 8-line doc-commented version (**delta +5**), so after Task 2 `leader_with_suit` sits at `lib.rs:242`, not `:237`. Task 3 then inserts 10 `///` lines above it (**delta +10**). Every `lib.rs` citation in Tasks 3-5's verification checklists is a **pre-Task-2** number; add +5 after Task 2 lands and +15 after Task 3 lands, or simply re-find the symbol. Tasks 4-5 touch only `.md` files, so nothing in them depends on `lib.rs` numbering.
- **`docs/authoring/RULES_AUTHORING.md` house rules that bind the Task 5 text:** no verbatim rulebook prose; concise; **code is authoritative over the physical rulebook**; inline command examples right where the action is explained (not only in the Commands table); version-specific. The Task 5 text below obeys all five. red7-1's RULES.md does **not** currently carry all the document's *required sections* (no Reading the Display, no Strategy Tips, no worked scoring example) — bringing it to full authoring compliance is a whole-document rewrite, is **not** what e F32 says, and is routed under "Cross-package / newly discovered".

**Snapshot drift:** **None.** `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/red7-1 /home/beefsack/Development/brdgme/rust/game/red7-1` produces empty output and exits 0 (verified 2026-07-25 against snapshot commit `f8763a5`). All findings' line citations are valid against the live files; live numbers are used throughout.

**No rules-version / manifest bump is needed.** `k8s/base/game/red7-1/game-version.yaml` carries `typeName: Red7`, `weight: 1.0`, `interfaceVersion: 2` and a `blurb` — **no rules version field of any kind**, and no `player_counts` field. The blurb ("A fast-paced card game where the rules change every turn. Play cards to your palette to stay in the lead, or discard to change the rule in your favour. If you aren't winning at the end of your turn, you're out.") is accurate and unaffected by these edits. The rules text reaches the DB because the operator reconcile re-reads `Request::Rules` from the redeployed image (`RULES_AUTHORING.md` "Storage"), not because a version string was bumped. The `e F27`-style "deployed blurb wrong" defect that WP-28 handled (`planning/specs/WP-28-lost-cities-shared-fixes.md:643-670`) **has no red7 analogue** — checked line by line.

## Disposition table (every finding re-derived from live source)

| F# | Claim | Verdict | What this spec does, and why |
|---|---|---|---|
| e F31 | `DATA_DOCS.md:36`'s "then by the highest card overall in the palette" is fictional | **CONFIRMED** | `card::leader` (`card.rs:297-317`) compares only set length then the winning set's max `rank_key`; there is no palette-wide comparison anywhere in the crate. Task 4 replaces the sentence with the real two-step rule plus the (D-29-gated) all-empty behaviour. **Also removes the redundant "(ties broken by highest even card)" parenthetical at `DATA_DOCS.md:31`** — with the general tie-break now stated correctly, a per-suit note implies Green is special-cased when it is not, which is the same "bots get a wrong model" failure F31 reports. |
| e F32 | `RULES.md:21` Turn section undersells the turn; `RULES.md:46-50` scoring line is wrong | **CONFIRMED (verification softened the detail)** | Verification: "Play-then-discard combo undocumented (code permits it, `can_discard` ignores `has_played`); scoring line wrong vs lib.rs:176-179. 'Lists as alternatives' slightly generous — RULES.md is silent, not contradictory." Re-derived: `can_discard` (`lib.rs:287-289`) has no `has_played` check, so play-then-discard is legal; `discard` calls `end_turn` (`lib.rs:351`), so the combo is strictly ordered and nothing follows a discard — a fact the finding did not mention and which the replacement text states. Scoring: `end_round` scores `leader_palette`, the rule-meeting subset (`lib.rs:176-179`). **ADJUSTED (extended) on one point:** the same paragraph says the target score is how the game ends; `start_round` also ends the game when the deck cannot deal (`lib.rs:88-91`), so the replacement text names both end conditions. Extending here rather than filing it separately is justified because the replaced sentence is itself the wrong-end-condition sentence. |
| e F33 | `pub use card::{Card as PubCard, Suit as PubSuit};` is unused and non-conventional | **CONFIRMED** | `rg -n "PubCard|PubSuit" /home/beefsack/Development/brdgme` returns **one** non-`docs/` hit: the definition at `rust/game/red7-1/src/lib.rs:16`. Not API-breaking — see the API-safety note below. Task 1 switches to `pub use card::*;`, matching `game/alhambra-1/src/lib.rs:5`, `game/starship-catan-1/src/lib.rs:5`, `game/seven-wonders-1/src/lib.rs:5`. **The finding's parenthetical "(or drop the re-export)" is REJECTED:** `mod card` is private, so with no re-export `Card` and `Suit` become unnameable outside the crate while still appearing in `pub` fields of `PubState`/`PlayerState` (`lib.rs:51,55,57,71`) — a strictly worse public API than today. |
| e F34 | `leader_with_suit`'s `player_map[l_index]` panics if every player is eliminated | **CONFIRMED as a fact, ADJUSTED to comment-only** | Verification: "`leader()` returns `(0, vec![])` for empty input; all-eliminated case unreachable today." Re-derived and agreed — see the F34 ruling below. Task 3 adds a precondition doc comment and **no code change**. |
| e F35 | `end_points`' `(50 - players * 5) as u32` underflows above 10 players | **CONFIRMED, and a code change IS warranted** | Verification: "pub fn, callers validated 2..=4." Re-derived, plus one reachability path verification did not name — see the F35 ruling below. Task 2 makes the arithmetic saturating with **no signature change**; the finding's "clamp / take a validated type" is narrowed to the clamp, and its "document the precondition" is done as well (both, not either). |
| e F29 | `CardParser` byte-slice non-ASCII panic (`command.rs:31`) | **FENCED to WP-01** | Confirmed covered: `planning/specs/WP-01-char-byte-panic-elimination.md` Task 6 ("red7-1 CardParser - char-boundary card split (e F29)", spec lines 628-745; Task 7's header is at :747) modifies `rust/game/red7-1/src/command.rs` lines 23-42 (live: the `chars.len() < 2` guard at :23-24 through the `None =>` arm's closing brace at :42) and appends a new inline `#[cfg(test)] mod tests` to that file, committing with `cargo test -p red7-1`. WP-01 is finalized (`planning/specs-LOG.md:125-133`: "Unit COMPLETE 2026-07-25. All 7 specs written and Lead-reviewed"). Do not touch `command.rs`. |
| e F30 | Zero-rule-fulfilling player treated as winning | **SKIPPED-BY-DECISION (D-29) — FENCED to WP-30** | `planning/work-packages.md:243-247`. Task 4's variant selection is the only place this package acknowledges it. |

**API-safety of the F33 change (checked, not assumed).** Nothing in the repo can break: (i) no crate depends on `red7-1` as a library — `red7-1` appears in `rust/Cargo.toml:21` only as a workspace **member**, and `rg -n "red7" rust/web/Cargo.toml rust/bot/Cargo.toml` returns nothing (consistent with the verification LOG's note under e F45 that "no in-repo crate consumes a game crate as a library"); (ii) the four `src/bin/` binaries go through `brdgme_cmd`/`brdgme_fuzz` generics over `Game`, never through `PubCard`/`PubSuit`; (iii) the names are not part of any serialized shape — serde uses the **struct/enum names** `Card` and `Suit` (`card.rs:6-15,101-105`), which the alias never changed, so every stored `Game` JSON is byte-identical before and after; (iv) no TypeScript/JSON-schema mirror of these names exists anywhere in the repo (the `rg` above covers the whole tree, `rust/web` included). Direction of change is **widening**: `pub use card::*;` exports a superset (`Card`, `Suit`, `full_deck`, `points`, `sort_by_suit`, `highest_card`, `cards_of_one_number`, `cards_of_one_color`, `most_even_cards`, `cards_of_different_colors`, `cards_that_form_a_run`, `most_cards_below_4`, `suit_rule`, `leader`), so no name that resolves today stops resolving — only `PubCard`/`PubSuit`, which nothing uses, disappear.

**Ruling on e F34 — documented invariant, NOT a code change.** Callers, grepped workspace-wide (`rg -n "\.leader\(\)|\.leader_with_suit\(|end_points\(" /home/beefsack/Development/brdgme/rust`), complete list:

- `leader_with_suit` (`lib.rs:237`): `lib.rs:234` (`Game::leader`), `lib.rs:325` (`discard`'s would-you-be-leader pre-check). **Two direct callers.**
- `Game::leader` (`lib.rs:233`): `lib.rs:100` (`start_round`, right after `self.eliminated = vec![false; l]` at `:86` and after the early `return` at `:88-91` — nobody is eliminated), `lib.rs:156` (`end_turn`, guarded by `!self.eliminated[self.current_player]` at `:155`), `lib.rs:176` (`end_round`, entered only from `lib.rs:166-167` when `remaining_players().len() == 1`), and test-only `lib.rs:592`. **Three runtime callers of `leader`, so four runtime paths into `leader_with_suit`, all with at least one survivor.** (The single `.leader()` hit outside this crate, `game/love-letter-2/src/lib.rs:222`, is an unrelated method on a different type.)
- Structural reinforcement (re-derived, beyond what the finding or verification states): `remaining_players().len() >= 1` is a standing invariant. Elimination happens one player at a time (`eliminate`, `lib.rs:225-231`), and `end_turn` re-checks `== 1` at `:166` immediately after every elimination and diverts to `end_round`, so the count can never reach 0 before `:171`'s `next_player` advance. `next_player` (`:261-271`) returns `from` unchanged when every other player is eliminated, but that branch is unreachable past the `== 1` check. Hence `player_map` is never empty at `:250`.

Making it return `Option` would force `.expect(...)` or an `unwrap_or` fallback at all four runtime paths — relocating the panic, not removing it — and `Game::leader` is `pub`, so the signature change ripples into a public API for a nit whose trigger is unreachable. Decisive factor: **`card::leader`/`leader_with_suit` are exactly the functions WP-30 rewrites** (`work-packages.md:245` lists `game/red7-1/src/{card.rs,lib.rs}`), and D-29 option A explicitly needs "a defined outcome when ALL players have empty sets" — quite possibly an `Option`-shaped return decided there, with the all-eliminated case folded in. Changing the signature here would collide textually and semantically with WP-30 and could pre-empt a decision that is not this package's to make. So: **document the precondition at the panic site, change no code.** If WP-30 later makes the return `Option`, the comment goes with it.

**Ruling on e F35 — code change (saturating), signature unchanged.** Callers of `end_points`, complete: `lib.rs:206` (`end_round`, passes `self.num_players`), `render.rs:135` (passes `pub_state.num_players`), test-only `lib.rs:611-613`. Both runtime callers pass `num_players`, which `Gamer::start` validates to `2..=4` (`lib.rs:377-383`) — so verification's "callers validated 2..=4" is right for freshly-started games. **But `Game` derives `Deserialize` with no validation** (`lib.rs:26`) and its `num_players` is a plain `pub usize` (`lib.rs:28`), so a stored or crafted state with `num_players = 11` reaches `end_points` and panics in any overflow-checked build. That reachability class (trusting a deserialized `Game`) is **WP-09 / D-36** territory and this package does **not** open it — but unlike a `Vec` index, this one is closable in one line with no signature change, no behaviour change for 2..=4, and no decision required, so it is worth doing rather than merely commenting. The fix is arithmetic-only; it does not make an 11-player game legal (`start` still rejects it) and it does not pretend to validate `num_players`.

**Non-Goals (owned elsewhere — do NOT absorb):**

- **e F29** (`CardParser` non-ASCII byte-slice panic) -> **WP-01**, Task 6 (`planning/specs/WP-01-char-byte-panic-elimination.md:628-745`). Do not touch `rust/game/red7-1/src/command.rs` in this package at all. **This includes Task 6's new `#[cfg(test)] mod tests` block at the end of `command.rs`** — do not pre-create it.
- **e F30** + **D-29** / **D-40** -> **WP-30** (`work-packages.md:243-247`, BLOCKED-ON-DECISION). Do not touch `card.rs`, do not change `leader`/`leader_with_suit` behaviour or signature, do not alter `RULES.md:31-32`.
- **Shared / systemic boilerplate-binary items** (`findings/games-batch-e.md:627` onward): **e F45** (binary-only deps declared as library `[dependencies]`) and **e F46** (HTTP binary defaults to port 80) -> **WP-73 game-bins consolidation**, BLOCKED-ON-DECISION(D-20) (`work-packages.md:554-560`; its scope line `:555` lists `e F45, e F46` and its paths line `:556` is "all 27 game crates' Cargo.toml + src/bin/*", which covers red7-1; note e F45's `[dev-dependencies]` recommendation is recorded at `:558-559` as INVALID). red7-1 does exhibit both: `rust/game/red7-1/Cargo.toml` declares `brdgme_cmd`, `brdgme_fuzz` and `tokio = { features = ["full"] }` as library `[dependencies]`, and it has four binaries (`src/bin/red7_1_{cli,fuzz,http,repl}.rs`). **Correction to this package's brief:** these two are *not* WP-08/WP-33 items — WP-08 is the finish/placings epilogue dedup sweep (`work-packages.md:80-86`, and red7-1 is not in its path list at `:82`) and WP-33 is the greed/farkle/ttt/no-thanks/liars-dice cleanup (`work-packages.md:265-269`, paths at `:267`). Neither owns any red7-1 file. Do not touch `rust/game/red7-1/Cargo.toml` or `rust/game/red7-1/src/bin/*`.
- **Deserialized-state trust** (a crafted `Game`/`PubState` with inconsistent `num_players`, short per-player vectors, out-of-range `current_player`) -> **WP-09**, BLOCKED-ON-DECISION(D-36) (`work-packages.md:88-96`; D-36 at `decisions-needed.md:464-476`). Task 2's saturating arithmetic is *not* a validation step and must not grow into one. **Caveat for the Lead:** WP-09's paths line (`work-packages.md:90`) does **not** name `game/red7-1`, so routing red7-1's remaining `num_players` trust there requires the Lead to widen WP-09's path list (see "Cross-package / newly discovered" item 4).
- **Test-module naming** (`mod tests` vs `mod test`, e F9) -> **WP-65 workspace hygiene** (`work-packages.md:501-506`). red7-1's module stays named `tests`.
- **Full `RULES_AUTHORING.md` compliance for red7-1's RULES.md** (missing Reading the Display render block, Strategy Tips, worked scoring example, inline command examples throughout) — see "Cross-package / newly discovered". Task 5 fixes exactly the two sections e F32 names.
- **`render.rs`** — verified clean by the review; untouched here.

**Coordination / landing order:**

1. **WP-01 vs WP-29:** zero textual overlap. WP-01 Task 6 edits `red7-1/src/command.rs` only; this package never opens that file. Either order works; both commit with `cargo test -p red7-1`, so whichever lands second simply runs the other's tests too.
2. **WP-30 before this package's Tasks 4-5** (see SEQUENCING). Tasks 1-3 may land first regardless.
3. **WP-30 vs Task 3:** Task 3 adds a comment immediately above `lib.rs:237` (`leader_with_suit`), a function WP-30 rewrites. If WP-30 lands first, re-read the function and keep the comment truthful (or drop it, if WP-30 already made the return `Option`); if this package lands first, WP-30 will hit the comment as context. Either way it is a comment — no merge hazard beyond text.
4. **WP-65 / WP-73** touch red7-1's `Cargo.toml`, `src/bin/*` and test-module naming; no file in this package overlaps.
5. Landing this package does not require a deploy-manifest change (see the no-bump note above), but the doc edits only reach the bot after the red7-1 image is rebuilt and the operator reconciles.

---

### Task 1: replace the dead `PubCard`/`PubSuit` alias with the sibling convention (e F33, nit)

**Problem (restated):** `rust/game/red7-1/src/lib.rs:16` is `pub use card::{Card as PubCard, Suit as PubSuit};`. `rg -n "PubCard|PubSuit" /home/beefsack/Development/brdgme` finds no consumer anywhere in the workspace — the two aliases are dead public API, and the three sibling crates with a `card` module all use plain `pub use card::*;` (`alhambra-1/src/lib.rs:5`, `starship-catan-1/src/lib.rs:5`, `seven-wonders-1/src/lib.rs:5`).

**Fix (re-derived):** `pub use card::*;`. This is a widening change (full export list in the API-safety note above), keeps `Card`/`Suit` nameable — which is required, because `mod card` is private while `PubState.discard_pile`/`palettes`/`scored_cards` and `PlayerState.hand` are `pub` fields of those types (`lib.rs:51,55,57,71`) — and changes no serialized name.

**Edge cases:**

- `lib.rs:10` already has `use crate::card::{Card, Suit, full_deck, leader, points, sort_by_suit, suit_rule};`. A glob `pub use card::*;` in the same module bringing the same names in again is **not** an error: glob imports have lower precedence than explicit ones, so the explicit `use` on :10 wins and the names still resolve to the identical `card::` items. Leave `lib.rs:10` exactly as it is.
  - **CORRECTION to the earlier draft of this spec (verified live):** none of the three sibling crates is a precedent for this *mixed* shape. `rg -n "use crate::card|use card::" rust/game/{alhambra-1,starship-catan-1,seven-wonders-1}/src/lib.rs` returns exactly one line per crate — the `pub use card::*;` at `:5` — and **no** explicit `use crate::card::{...}`. All three rely on the glob alone. Additionally, `starship-catan-1/src/lib.rs:1` and `seven-wonders-1/src/lib.rs:1` declare `pub mod card;` while `alhambra-1/src/lib.rs:1` declares private `mod card;`, so only alhambra-1 matches red7-1's module visibility. The convention claim (`pub use card::*;` is what siblings write) stands; the "same shape including explicit uses" claim does not.
  - **CONTINGENCY (deterministic, follow it if the build complains).** Because no in-repo crate carries glob + explicit together, the redundancy is unattested here. If `cargo test -p red7-1` or `cargo clippy -p red7-1 --all-targets -- -D warnings` emits **any** diagnostic naming `lib.rs:10` (`unused_imports`, or a redundant/unused-import note): delete `lib.rs:10` entirely. That is safe and produces alhambra-1's exact shape — every name :10 imports (`Card`, `Suit`, `full_deck`, `leader`, `points`, `sort_by_suit`, `suit_rule`) is `pub` in `card.rs` (`card.rs:103`, `:7`, `:145`, `:297`, `:155`, `:159`, `:285`) and is therefore in the crate-root namespace via the new glob, so every use site in `lib.rs` (`leader(&palettes)` at `:250`, `points(...)` at `:177` and `:274`, `suit_rule(suit)` at `:238`, `full_deck()` at `:389`, `sort_by_suit(...)` at `:126`, `:182`, and the `Card`/`Suit` type annotations throughout) keeps resolving. Do **not** delete :10 pre-emptively — only if a diagnostic points at it.
- The inline test module already globs the same items twice over: `lib.rs:551` is `use super::*;` and `lib.rs:552` is `use crate::card::*;`. After Task 1, `use super::*` also re-exports the card items, so :551 and :552 become two overlapping globs. Overlapping globs that resolve to the **same** item are legal and silent in Rust; leave both lines untouched. If a diagnostic names `lib.rs:552`, delete only :552 (the tests use `highest_card`, `cards_of_one_number`, `cards_of_one_color`, `most_even_cards`, `cards_of_different_colors`, `cards_that_form_a_run`, `most_cards_below_4`, `leader` at `:648`-`:717`, all reachable through `super::*` afterwards).
- No name in the glob collides with anything defined in `lib.rs`. Full glob export list from `card.rs`: `Suit` (`:7`), `Card` (`:103`), `full_deck` (`:145`), `points` (`:155`), `sort_by_suit` (`:159`), `highest_card` (`:163`), `cards_of_one_number` (`:172`), `cards_of_one_color` (`:197`), `most_even_cards` (`:230`), `cards_of_different_colors` (`:234`), `cards_that_form_a_run` (`:250`), `most_cards_below_4` (`:278`), `suit_rule` (`:285`), `leader` (`:297`). Crate-root items in `lib.rs` are `MIN_PLAYERS` (`:19`), `MAX_PLAYERS` (`:20`), `end_points` (`:22`), `Game` (`:27`), `PubState` (`:43`), `PlayerState` (`:65`), `Command` (re-exported `:17`) and the three module names. Zero overlap. `card::points`/`card::leader` are free functions while `Game::leader` (`:233`) and `Game::player_points` (`:273`) are inherent methods and `Gamer::points` (`:518`) is a trait method — different namespaces entirely.
- `Suit::ordinal` (`card.rs:89`) is private and stays private — the glob only re-exports `pub` items, so no new inherent method leaks.
- `mod card;` stays **private** (`lib.rs:12`). Do not make it `pub mod`.

**Files:**
- Modify: `rust/game/red7-1/src/lib.rs` (line 16)

**Steps:**

- [ ] Replace `rust/game/red7-1/src/lib.rs:16` in full:

```rust
pub use card::*;
```

  Leave line 17 (`pub use command::Command;`) and line 10's explicit `use` untouched.
- [ ] Verify nothing references the old names: `rg -n "PubCard|PubSuit" /home/beefsack/Development/brdgme/rust` — must print nothing. (Before the edit this command prints exactly one line, `rust/game/red7-1/src/lib.rs:16`; that is the definition you just replaced. Hits under `docs/` are review paperwork and are expected to remain.)
- [ ] Run: `cargo test -p red7-1` — compiles, all 18 inline tests plus `game_contract` PASS. **If the compiler or clippy names `lib.rs:10` or `lib.rs:552`, apply the CONTINGENCY in Edge cases above (delete that one line) and re-run — do not silence it with `#[allow]`.** (No new test: the change has no runtime behaviour. Its correctness property is "the crate and its four binaries still compile and `Card`/`Suit` are still exported", which the build and the existing `test_game_decode` / `pub_state_does_not_leak_hidden_info` serde tests already exercise.)
- [ ] Run: `cargo clippy -p red7-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `git add rust/game/red7-1/src/lib.rs` ; message: `refactor(red7-1): plain card re-export instead of dead PubCard/PubSuit aliases (e F33, WP-29)`

---

### Task 2: make `end_points` saturate instead of underflowing (e F35, nit)

**Problem (restated):** `end_points` (`lib.rs:22-24`) is byte-exactly:

```rust
pub fn end_points(players: usize) -> u32 {
    (50 - players * 5) as u32
}
```

For `players > 10` the subtraction underflows and **panics in every overflow-checked build**. Verified: `rust/Cargo.toml` declares only `[profile.dev]` (`:46-47`, sole key `debug = "line-tables-only"`), `[profile.android-dev]` (`:49`), `[profile.server-dev]` (`:52`), `[profile.wasm-dev]` (`:55-56`) and `[profile.wasm-release]` (`:59-62`) — **no profile sets `overflow-checks` or `debug-assertions`**, so `cargo test`'s implicit `test` profile (inheriting `dev`) keeps overflow checks ON, which is what makes the failing-test step below actually fail. In release it wraps to a colossal target score, which would make the game unwinnable. `Gamer::start` validates `2..=4` (`lib.rs:377-383`), but `Game` derives `Deserialize` with `pub num_players: usize` unvalidated (`lib.rs:26-28`), and both runtime callers (`lib.rs:206` in `end_round`, `render.rs:135` in `common_rows`) feed it straight from `self.num_players` / `pub_state.num_players`.

**Fix (re-derived):** saturating arithmetic, signature and return type unchanged, plus the precondition in a doc comment. Values for the only legal inputs are bit-identical (2 -> 40, 3 -> 35, 4 -> 30). This is deliberately **not** input validation — rejecting a bogus `num_players` is WP-09/D-36's call.

**Edge cases (each is a case in the new test):**

- `players.saturating_mul(5)` must come first. `50usize.saturating_sub(players * 5)` alone still panics for `players > usize::MAX / 5` because the **multiply** overflows before the saturating subtract runs. `end_points(usize::MAX)` is the test case that pins this down; without `saturating_mul` that assertion panics with `attempt to multiply with overflow`.
- `50usize.saturating_sub(...)` yields at most 50, so the `as u32` cast is lossless — no `clippy::cast_possible_truncation` (allow-by-default anyway) and nothing else fires at `-D warnings` with default lint levels.
- Legal inputs are bit-identical: `end_points(2) == 40`, `(3) == 35`, `(4) == 30` before and after, which the untouched `test_end_points` (`lib.rs:609-614`) already asserts.
- `end_points(10)` is `0` both before and after (`50 - 50` does not underflow); it is included in the new test as the exact boundary, and it is the one new case that PASSES before the fix. The first case that fails pre-fix is `end_points(11)`.
- No caller behaviour changes: `end_round` (`lib.rs:206`) and `render.rs:135` both pass a `num_players` that is 2..=4 for any game created through `Gamer::start`.
- Do **not** touch the existing `test_end_points` (`lib.rs:609-614`); add a separate test so no existing test is edited (Global Constraints).
- Return type stays `u32`. Do not switch to `Option<u32>`, do not add a `debug_assert!`, do not add a `players` range check — validation is WP-09/D-36's.

**Files:**
- Modify: `rust/game/red7-1/src/lib.rs` (lines 22-24; new test appended inside `mod tests`)

**Steps:**

- [ ] Write the failing test first. Insert it inside `mod tests` immediately **after** the existing `test_end_points` function (which ends at `lib.rs:614`):

```rust
    #[test]
    fn end_points_saturates_for_impossible_player_counts() {
        // `Gamer::start` only allows 2..=4, but `Game` deserializes
        // `num_players` unvalidated, so `end_points` must not panic.
        assert_eq!(0, end_points(10));
        assert_eq!(0, end_points(11));
        assert_eq!(0, end_points(usize::MAX));
    }
```

- [ ] Run: `cargo test -p red7-1 end_points_saturates_for_impossible_player_counts`. Expected: **FAIL**, with the panic message `attempt to subtract with overflow` raised from `lib.rs:23`, triggered by the `assert_eq!(0, end_points(11))` line (the preceding `end_points(10)` case passes). If instead it PASSES, overflow checks are off for your profile — stop and report, because the test then proves nothing.
- [ ] Replace `lib.rs:22-24` in full. Current byte-exact content:

```rust
pub fn end_points(players: usize) -> u32 {
    (50 - players * 5) as u32
}
```

  New content (note: this grows the file by 5 lines, so every `lib.rs` line number after 24 shifts by +5 for the remainder of Task 2 and for Task 3 — re-locate `test_end_points` and `leader_with_suit` by name, not by number, after this edit):

```rust
/// Points needed to end the game: 40 for 2 players, 35 for 3, 30 for 4.
///
/// Only `MIN_PLAYERS..=MAX_PLAYERS` are meaningful (`Gamer::start` rejects
/// anything else), but `Game` deserializes `num_players` unvalidated, so the
/// arithmetic saturates at 0 rather than underflowing (e F35).
pub fn end_points(players: usize) -> u32 {
    50usize.saturating_sub(players.saturating_mul(5)) as u32
}
```

- [ ] Run: `cargo test -p red7-1 end_points` — both `test_end_points` (40/35/30 unchanged) and the new test PASS.
- [ ] Run: `cargo test -p red7-1` — the whole crate suite PASS.
- [ ] Run: `cargo clippy -p red7-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `git add rust/game/red7-1/src/lib.rs` ; message: `fix(red7-1): saturate end_points instead of underflowing (e F35, WP-29)`

---

### Task 3: document `leader_with_suit`'s non-empty precondition (e F34, nit — COMMENT ONLY)

**Problem (restated):** `leader_with_suit` ends with `(player_map[l_index], palette)` (`lib.rs:251`). If **every** player is eliminated, `player_map` and `palettes` are both empty, `card::leader` returns `(0, vec![])` for an empty slice (`card.rs:298-300`), and `player_map[0]` panics on an empty `Vec`. Unreachable today — all four runtime paths guarantee a survivor (full caller list plus the `remaining_players().len() >= 1` invariant in the F34 ruling above) — but the invariant is implicit.

**Fix (re-derived):** a doc comment stating the precondition, and **nothing else**. Do not change the return type, do not add a fallback, do not add a guard. Reasons, in order: an `Option` return relocates the panic into four `.expect()`s; `Game::leader`/`leader_with_suit` are `pub`, so the change is a public-API break for an unreachable nit; and these are the exact functions **WP-30** rewrites under D-29, which explicitly must define "a defined outcome when ALL players have empty sets" — the all-eliminated case belongs with that decision, not here.

**Edge cases:**
- Comment only: `cargo test -p red7-1` output must be identical to Task 2's run. If any test result changes, something else was edited — revert and redo.
- Do not add the comment to `card::leader` (`card.rs:297`) — `card.rs` is WP-30's file and is out of scope for this package.
- The comment must not assert what the empty-set/all-empty *rule* should be (that is D-29); it states only the caller-side precondition and the consequence of breaking it.

**Files:**
- Modify: `rust/game/red7-1/src/lib.rs` (insert above `leader_with_suit`: line 237 at package head, **line 242 after Task 2's +5 shift**)

**Steps:**

- [ ] Locate the function by name, not by number: `rg -n "pub fn leader_with_suit" rust/game/red7-1/src/lib.rs` — expect `242` if Task 2 has landed, `237` if it has not. Insert immediately above the `pub fn leader_with_suit(&self, suit: Suit) -> (usize, Vec<Card>) {` line, at the same indentation as the `pub fn` (4 spaces — it is inside `impl Game`, which opens at `lib.rs:74`):

```rust
    /// Returns the leading player index under `suit`'s rule and their
    /// rule-fulfilling cards.
    ///
    /// PRECONDITION: at least one player must not be eliminated. With every
    /// player eliminated, `player_map` is empty, `card::leader` returns index
    /// 0 for the empty slice, and the final `player_map[l_index]` would
    /// panic. All four call sites satisfy this: `start_round` (has just reset
    /// `eliminated` to all-false), `end_turn` (guarded by
    /// `!self.eliminated[self.current_player]`), `end_round` (only entered
    /// when exactly one player remains), and `discard` (the current player is
    /// never eliminated while current). (e F34)
    pub fn leader_with_suit(&self, suit: Suit) -> (usize, Vec<Card>) {
```

- [ ] Confirm nothing else changed in the function: `git diff rust/game/red7-1/src/lib.rs` shows only added `///` lines.
- [ ] Run: `cargo test -p red7-1` — full suite PASS, same count as Task 2. (No unit test: there is no reachable input that exercises the documented precondition, and constructing an all-eliminated `Game` to assert a panic would lock in behaviour WP-30 is expected to change.)
- [ ] Run: `cargo clippy -p red7-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `git add rust/game/red7-1/src/lib.rs` ; message: `docs(red7-1): document leader_with_suit non-empty precondition (e F34, WP-29)`

---

### Task 4: correct DATA_DOCS.md's tie-break description (e F31, minor — DOC ONLY, GATED)

> **INVALIDATED - do NOT execute this task as written.** The `e F30` ruling in
> `decisions-ANSWERED.md` holds the documented second tie-break clause ("then by
> the highest card overall in the palette") to be **correct and officially
> supported**, not fictional: the CODE must change to implement it (fall through
> to the full palette's `rank_key` max). The `DATA_DOCS.md` sentence **stays**.
> The corresponding code fix belongs to the `e F30` owner, not to this package.
> Any implementer MUST read the `e F30` row and the `e F30 evidence` note in
> `decisions-ANSWERED.md` before touching `DATA_DOCS.md` or `card.rs`. The text
> below is retained only for traceability; the Lead must respec what, if
> anything, of the Green parenthetical and the per-rule prose survives.

**PRECONDITION CHECKPOINT — do this before editing anything:**

- [ ] Check whether **WP-30** has landed: `git log --oneline -- rust/game/red7-1/src/card.rs` and read `rust/game/red7-1/src/card.rs:297-317` as it stands now.
- [ ] Decide the variant:
  - `card.rs`'s `leader` still selects the first non-eliminated player when all winning sets are empty (i.e. WP-30 has not landed, or D-29 chose **option B**) -> use variant **B-CURRENT**.
  - `leader` now excludes empty winning sets from winning (D-29 **option A**) -> use variant **A-OFFICIAL**, and additionally re-read the new function to confirm the A-OFFICIAL sentence matches the implemented all-empty resolution word for word. If it does not, **stop and report** — the doc must describe the code, and a third behaviour means the Lead has to supply the sentence.
- [ ] Record which variant was chosen in the commit message.

**Problem (restated):** `DATA_DOCS.md:36` claims "Ties within a rule are broken by the highest card in the winning set, then by the highest card overall in the palette." The second clause exists in neither the code nor the official rules — `card::leader` (`card.rs:297-317`) compares set length, then the winning set's maximum `rank_key`, and stops. Bots consuming DATA_DOCS.md get a wrong tie-break model. Separately, `DATA_DOCS.md:31`'s "(ties broken by highest even card)" presents the general tie-break as a Green-only special case.

**Fix (re-derived):** state the two real comparison steps, the colour ordering that `rank_key` implies, why no third step exists, and what happens when nobody has a qualifying card. Facts and their sources are in the Architecture "Derived winner / tie-break rule" section above — every sentence below traces to one of steps 1-5 there.

**Edge cases:**
- `DATA_DOCS.md` is `include_str!`-ed at `lib.rs:537`, so this edit changes the build output. Keep it valid Markdown; the file uses `-` bullets and no trailing blank line beyond one newline.
- House typography: ASCII only, no em dashes, no smart quotes (the existing file complies).
- Do **not** touch the `PubState`/`PlayerState`/`Card` sections (`DATA_DOCS.md:1-24`) — the review found them accurate.
- Do not add per-suit tie-break notes back in.
- **Spelling: `DATA_DOCS.md` uses AMERICAN "color", not "colour"** — `DATA_DOCS.md:23` ("One of Red, Orange, Yellow, Green, Blue, Indigo, Violet."), `:30` ("most cards of one color (suit) wins."), `:32` ("most cards of different colors wins."). The replacement text below therefore says "color"/"colors". This is the **opposite** of `RULES.md`, whose prose is British ("favour" at `RULES.md:5`, "colour" at `:25`) — do not cross-contaminate the two files. (Task 5 uses "colour" for exactly this reason.)
- The file has **36 lines total** and no trailing blank line beyond the final newline (`wc -l rust/game/red7-1/DATA_DOCS.md` -> 36). Line 36 is the last line.

**Files:**
- Modify: `rust/game/red7-1/DATA_DOCS.md` (line 31 and line 36; lines 34-35 are read-only anchors and must not change)

**Steps:**

- [ ] Replace `DATA_DOCS.md:31`, whose current byte-exact content is:

```markdown
- Green: most even cards wins (ties broken by highest even card).
```

  with (dropping the parenthetical; the general rule below covers it):

```markdown
- Green: most even cards wins.
```

- [ ] Replace `DATA_DOCS.md:36` — the file's current last line, whose byte-exact content is:

```markdown
Ties within a rule are broken by the highest card in the winning set, then by the highest card overall in the palette.
```

  with the following block. Lines 34 (`- Violet: most cards below rank 4 wins.`) and 35 (blank) stay as they are, so the new text begins on line 36:

```markdown
Under the active rule, each player still in the round has a "winning set": the
cards in their palette that fulfil the rule (the single highest card for Red,
the largest group of one number for Orange, the largest group of one color for
Yellow, every even card for Green, the highest card of each distinct color for
Blue, the longest run of consecutive numbers for Indigo, every card below rank
4 for Violet).

The leader is the player with the most cards in their winning set. If two
players tie on count, the one whose winning set contains the highest card wins,
comparing rank first and then color, with Red highest and then Orange, Yellow,
Green, Blue, Indigo, Violet. There is no further tie-break: every card in the
deck is unique, so two non-empty winning sets can never tie on their highest
card.
```

- [ ] Append **one** of the following two paragraphs (blank line first). Choose per the precondition checkpoint.

  **Variant B-CURRENT** (WP-30 not landed, or D-29 option B — current implemented behaviour, `card.rs:309-311`):

```markdown
Green and Violet can leave every player with an empty winning set (nobody has
an even card, or nobody has a card below 4). In that case the lowest-numbered
player still in the round is treated as the leader, even though they have no
qualifying cards. This deviates from the official rules, which say a player
with no rule-fulfilling card cannot be winning.
```

  **Variant A-OFFICIAL** (only if WP-30 landed with D-29 option A, and only after confirming the code matches):

```markdown
A player whose winning set is empty is never the leader, so under Green or
Violet a player holding no even card, or no card below 4, cannot be winning.
```

- [ ] Verify the fiction is gone: `rg -n "highest card overall|highest even card" /home/beefsack/Development/brdgme/rust/game/red7-1/DATA_DOCS.md` — must print nothing.

**Verification (doc-only — no unit test; re-read each cited site and tick it off):**

- [ ] "cards in their palette that fulfil the rule" and the seven per-rule descriptions match `suit_rule` and its seven functions: `rust/game/red7-1/src/card.rs:285-295`, `:163-170` (Red), `:172-195` (Orange), `:197-228` (Yellow), `:230-232` (Green), `:234-248` (Blue), `:250-276` (Indigo), `:278-283` (Violet).
- [ ] "the player with the most cards in their winning set" matches the `p.len() > leader_palette.len()` comparison: `rust/game/red7-1/src/card.rs:311`.
- [ ] "comparing rank first and then color" matches `rank_key()` = `(rank, suit.ordinal())`: `rust/game/red7-1/src/card.rs:134-136`.
- [ ] The seven per-rule phrasings are individually true, not just the function names: "single highest card" = `max_by_key(rank_key)` returning one card (`card.rs:163-170`); "largest group of one number" = longest equal-rank run over rank-sorted cards (`card.rs:172-195`); "largest group of one color" = the first suit bucket whose length equals the max (`card.rs:197-228`); "every even card" = unfiltered `rank % 2 == 0` (`card.rs:230-232`); "highest card of each distinct color" = first card per unused suit over descending `rank_key` (`card.rs:234-248`); "longest run of consecutive numbers" = longest descending consecutive-rank chain, duplicate ranks skipped (`card.rs:250-276`); "every card below rank 4" = unfiltered `rank < 4` (`card.rs:278-283`).
- [ ] **Spelling check:** `rg -n "colour" /home/beefsack/Development/brdgme/rust/game/red7-1/DATA_DOCS.md` — must print nothing (this file is American-spelled).
- [ ] "Red highest and then Orange, Yellow, Green, Blue, Indigo, Violet" matches `ordinal()`: `rust/game/red7-1/src/card.rs:89-99` (Violet 0 … Red 6).
- [ ] "every card in the deck is unique" matches `full_deck()`: `rust/game/red7-1/src/card.rs:145-153` (7 suits x ranks 1-7, no duplicates).
- [ ] "players still in the round" (eliminated players excluded) matches the `if self.eliminated[p] { continue; }` skip: `rust/game/red7-1/src/lib.rs:242-248`.
- [ ] The chosen all-empty variant matches the **live** `card.rs` `leader` you read at the checkpoint (`card.rs:297-317` pre-WP-30).
- [ ] `cargo test -p red7-1` still PASSES (the file is `include_str!`-ed, so a broken file would still compile — this only proves the crate builds; the checklist above is the real verification).
- [ ] `cargo clippy -p red7-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `git add rust/game/red7-1/DATA_DOCS.md` ; message: `docs(red7-1): DATA_DOCS tie-break matches the code (e F31, WP-29, variant <B-CURRENT|A-OFFICIAL>)`

---

### Task 5: correct RULES.md's Turn and Scoring sections (e F32, minor — DOC ONLY)

**Problem (restated):** (1) The Turn section (`RULES.md:19-32`) presents Play / Discard / Done as a flat list and never says that **playing and then discarding in the same turn is legal** — the strongest move in the game — which `can_discard` permits by ignoring `has_played` (`lib.rs:287-289`); it also never says that a discard **ends the turn immediately** (`discard` calls `end_turn` at `lib.rs:351`). (2) The Scoring section (`RULES.md:46-50`) says the remaining player "scores their palette cards", but `end_round` scores only `leader_palette`, the rule-meeting subset (`lib.rs:176-179`), and it says the target score is how the game ends, while `start_round` also ends the game when the deck cannot deal a new round (`lib.rs:88-91`).

**Fix (re-derived):** rewrite both sections with the real ordering and the real scoring, keeping the existing house style (`##` sections, numbered turn options, inline command examples per `docs/authoring/RULES_AUTHORING.md`). **This text is D-29-independent** — verified sentence by sentence: no sentence below states who the leader is or how ties resolve; the two references to "leader" are the discard legality check (`lib.rs:325-330`) and the end-of-turn elimination check (`lib.rs:154-164`), both of which hold verbatim under either D-29 option because they delegate to whatever `leader()` returns.

**Edge cases:**
- `RULES.md` is `include_str!`-ed at `lib.rs:533` and reaches the bot via `game_versions.rules`; keep it valid Markdown.
- Leave the existing lines 31-32 ("At the end of your turn, if you are not the leader under the current rule, you are eliminated") **exactly as they are** — they are correct today and any leader-definition clause is WP-30's.
- Leave the Commands section (`RULES.md:13-17`), the Rules-by-colour table (`:34-44`) and the target-score list (`:52-55`) untouched. The targets already match `end_points` (40/35/30).
- Do **not** add the empty-hand elimination rule, the missing Reading-the-Display render block or Strategy Tips here — see "Cross-package / newly discovered".
- ASCII only; the file uses British spelling ("colour", "favour") in prose and "color" inside the colour table — match the surrounding prose, i.e. "colour".

**Files:**
- Modify: `rust/game/red7-1/RULES.md` (lines 21-29 and lines 48-50)

**Steps:**

- [ ] Replace `RULES.md:21-29`, whose byte-exact current content is:

```markdown
On your turn you may:

1. **Play** a card from your hand to your palette (once per turn).
2. **Discard** a card from your hand to change the active rule. The discarded
   card's colour determines the new rule. You must be the leader under the new
   rule after discarding. If the discarded card's number is higher than the
   number of cards in your palette, you draw a card.
3. **Done** - end your turn. If you haven't played or discarded, you are
   eliminated.
```

  (line 21 is `On your turn you may:`, line 22 is blank, line 30 immediately after the block is blank and stays, lines 31-32 stay verbatim) with:

```markdown
On your turn, in this order:

1. **Play** at most one card from your hand to your palette (`play b4`). This
   does not end your turn.
2. **Discard** at most one card from your hand to change the active rule
   (`discard b4`). The discarded card's colour becomes the new rule, and you
   must be the leader under that new rule for the discard to be allowed. If the
   discarded card's number is higher than the number of cards in your palette,
   you draw a card. Discarding ends your turn immediately.
3. **Done** (`done`) - end your turn without discarding. If you have neither
   played nor discarded, you are eliminated.

Playing and then discarding in the same turn is allowed, and is usually the
strongest move: the played card strengthens your palette before the new rule is
judged. Because a discard ends your turn, you cannot play after discarding, and
`done` is only needed when you did not discard.
```

- [ ] Replace `RULES.md:48-50`, whose byte-exact current content is:

```markdown
When all but one player is eliminated in a round, the remaining player (the
leader) scores their palette cards. The first player to reach the target score
wins the game.
```

  (line 47 is blank and stays; line 51 is blank and stays; the `Target scores:` list at `:52-55` stays) with:

```markdown
When all but one player is eliminated in a round, the remaining player scores
the cards in their palette that meet the current rule - not their whole palette.
Each card is worth its number, and the scored cards move out of the palette into
that player's score pile. The game ends as soon as any player reaches the target
score, and also ends if the deck no longer holds enough cards to deal the next
round. The player with the most points then wins; equal totals share the
placing.
```

**Verification (doc-only — no unit test; re-read each cited site and tick it off):**

- [ ] "at most one card ... does not end your turn" matches `can_play`'s `!self.has_played` and `play`'s lack of an `end_turn` call: `rust/game/red7-1/src/lib.rs:283-285` and `:295-314`.
- [ ] "you must be the leader under that new rule" matches the pre-check: `rust/game/red7-1/src/lib.rs:325-330`.
- [ ] "if the discarded card's number is higher than the number of cards in your palette, you draw a card" matches `if card.rank as usize > self.palettes[player].len()`: `rust/game/red7-1/src/lib.rs:346-348`.
- [ ] "Discarding ends your turn immediately" matches the `end_turn` call at the end of `discard`: `rust/game/red7-1/src/lib.rs:350-351`.
- [ ] "Playing and then discarding in the same turn is allowed" matches `can_discard`, which checks only `current_player` and `!finished`: `rust/game/red7-1/src/lib.rs:287-289`.
- [ ] "If you have neither played nor discarded, you are eliminated" matches `done`'s `!self.has_played` branch: `rust/game/red7-1/src/lib.rs:355-369` (note `discard` also sets `has_played`, `:350`).
- [ ] "all but one player is eliminated" matches the round-end trigger `remaining_players().len() == 1`: `rust/game/red7-1/src/lib.rs:166-169`.
- [ ] "the cards in their palette that meet the current rule" matches `end_round` scoring `leader_palette`: `rust/game/red7-1/src/lib.rs:176-178`.
- [ ] "each card is worth its number" matches `points`/`Card::points`: `rust/game/red7-1/src/card.rs:155-157` and `:126-128`.
- [ ] "the scored cards move out of the palette" matches the `retain`: `rust/game/red7-1/src/lib.rs:179`.
- [ ] "ends as soon as any player reaches the target score" matches the `>= ep` loop over all players: `rust/game/red7-1/src/lib.rs:206-212`.
- [ ] "also ends if the deck no longer holds enough cards to deal the next round" matches `if self.deck.len() < l * 8 { self.end_game(...) }`: `rust/game/red7-1/src/lib.rs:88-91`.
- [ ] "The player with the most points then wins" matches placings from `gen_placings` over `player_points`: `rust/game/red7-1/src/lib.rs:401-409` (metrics built at `:403-405`, `gen_placings(&metrics)` at `:407`).
- [ ] "equal totals share the placing" matches `gen_placings`, which groups players by identical metric vectors and assigns every player in a group the same place: `rust/lib/game/src/game.rs:154-179` (grouping at `:155-159`, one `cur_place` per group at `:166-172`). A red7 draw is therefore possible and the doc must not promise a single winner.
- [ ] **Spelling check:** the new prose uses British "colour" (matching `RULES.md:5` "favour" and `:25` "colour") and introduces no "color" outside the existing table at `:36-44`. `rg -n "color" /home/beefsack/Development/brdgme/rust/game/red7-1/RULES.md` must print only table lines 40 and 42.
- [ ] The `play`/`discard`/`done` command spellings used inline match the parser: `rust/game/red7-1/src/command.rs` (`Token`s in `command_parser`) and the Commands section at `RULES.md:13-17`.
- [ ] `cargo test -p red7-1` PASSES; `cargo clippy -p red7-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate once for the package: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` (DB-backed web failures in a bare local run are pre-existing, backlog #40).
- [ ] Commit: `git add rust/game/red7-1/RULES.md` ; message: `docs(red7-1): RULES turn order and rule-meeting scoring (e F32, WP-29)`

---

## Cross-package / newly discovered (evidence + routing — do NOT fix here)

1. **RULES.md does not document the empty-hand elimination.** `start_turn` eliminates the current player "for not having any cards left" when their hand is empty (`rust/game/red7-1/src/lib.rs:146-152`), and that elimination can immediately end the round via `end_turn` (`:150`). No sentence in `RULES.md` mentions it, so a bot playing from the rules alone cannot predict its own elimination. **Not in e F32's scope** (which names the turn-option list and the scoring line). **No existing work package owns red7-1 doc completeness** — WP-30's paths are `card.rs`/`lib.rs` only (`work-packages.md:245`). **Route: Lead to file as a new docs item** (natural home: whichever package next opens red7-1's RULES.md, or a red7 doc-completeness follow-up).
2. **red7-1's RULES.md does not meet `docs/authoring/RULES_AUTHORING.md`'s required-sections list.** The authoritative list is `RULES_AUTHORING.md:13-107`: Overview (`:15`), Cards / Components (`:18`), Turn Structure (`:21`), Scoring (`:29`), Rounds / Game End (`:37`), Winning (`:40`), Reading the Display (`:43`), Commands (`:91`), Strategy Tips (`:100`). red7-1's `RULES.md` (55 lines) has an Overview (`:1-6`), Setup (`:8-11`), Commands (`:13-17`), Turn (`:19-32`), Rules-by-colour (`:34-44`) and Scoring (`:46-55`). **Missing outright: Cards / Components; Rounds / Game End; Winning; Reading the Display (which `RULES_AUTHORING.md:44` calls "critical for the bot"); Strategy Tips.** Also missing within Scoring: the worked example that `RULES_AUTHORING.md:30-36` mandates. Present but thin: inline command examples (Task 5 adds two, per `:23-27`). The crate does ship `BASIC_STRATEGY.md` (25 lines) and `ADVANCED_STRATEGY.md` (33 lines), surfaced through `Gamer::basic_strategy`/`advanced_strategy` (`lib.rs:540-546`), which may be the intended substitute for the Strategy Tips section — needs a Lead ruling, and note `RULES_AUTHORING.md:100-107` says "Always include this section" and restricts its content to the official rulebook or user-supplied tips. This is a whole-document rewrite requiring a live render pulled from a real game state (`RULES_AUTHORING.md:56-64` gives the extraction recipe, which needs a DB and a built binary), which this read-only-derived spec cannot produce. **Route: Lead — new docs work package; no existing package covers it.**
3. **`DATA_DOCS.md`'s `discard_pile` entry says "The suit of the top (last) card determines the current winning rule. If empty, the default rule is Red (highest card)."** — verified correct against `current_rule()` (`lib.rs:254-259`). Recorded only so the next reader does not re-flag it.
4. **`end_points` is not the only place a deserialized `num_players` is trusted** — `render.rs:135`, the `0..self.num_players` loops (`lib.rs:207`, `:403`, `:413`) and the per-player vector indexing all assume `num_players` agrees with the vector lengths. Task 2 closes only the arithmetic panic. **Route: WP-09 deserialized-state trust hardening, BLOCKED-ON-DECISION(D-36)** (`work-packages.md:88-94`). Do not widen Task 2.
