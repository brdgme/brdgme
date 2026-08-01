# age-of-war-2 Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Go game `brdgme-go/age_of_war_1` to a Rust crate `rust/game/age-of-war-2` (Track B conversion), register/deploy it, deprecate age-of-war-1, and update tracking docs.

**Architecture:** Standalone Rust game crate implementing `brdgme_game::Gamer`, following the liars-dice-2 / sushizock-2 template. Domain: 14 castles across 6 clans, dice rolling (7 dice), attack/line/roll commands, no hidden information (PubState carries full state).

**Tech Stack:** Rust, brdgme_game, brdgme_markup, brdgme_color, brdgme_cmd, brdgme_fuzz, rand, serde.

## Global Constraints

- Follow `docs/porting/GAME_PORTING.md` and `docs/porting/RENDER_PARITY.md` exactly.
- Porting correctness rule: preserve Go behaviour verbatim; note (do not silently fix) suspected source defects in the plan progress notes.
- Go tests ported 1:1 with original names snake_cased; the Go suite is thin (2 tests), so add a baseline suite per GAME_PORTING.md step 8.
- Go `GenPlacings` is compact-ordinal; Rust `gen_placings` is standard-competition. The Go suite has no placings test, so baseline placings tests use Rust semantics.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --exclude web --all-targets -- -D warnings` must pass.
- Commit after each task with the trailer: Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

## Key Go source facts (source of truth: brdgme-go/age_of_war_1/)

- Game state: `CurrentPlayer`, `Players`, `Conquered map[int]bool`, `CastleOwners map[int]int`, `CurrentlyAttacking int` (-1 = none), `CompletedLines map[int]bool`, `CurrentRoll []int`. Rust: use `Vec<bool>`/`Vec<Option<usize>>`-style simple shapes; `currently_attacking: Option<usize>`.
- 2-6 players. `New` -> `StartTurn` -> roll 7 dice, public log "{player} rolled  {dice}" (TWO spaces after "rolled", dice joined by two spaces).
- Dice faces: 1 inf, 2 inf, 3 inf (blue), arch (purple), cav (green), dai (red). Roll = uniform 0..5.
- Castles/clans data in castles.go: 14 castles, clans Oda(10)/Tokugawa(8)/Uesugi(8)/Mori(5)/Chosokabe(4)/Shimazu(3) set points; clan colours Yellow/Grey/Purple/Red/Black/Green.
- `Line { infantry, symbols }`; `MinDice = symbols.len() + (infantry+2)/3`; `CanAfford(roll)`: symbols must be a multiset-subset of the roll's non-infantry dice; infantry dice values (sorted descending) consumed until infantry requirement met; `using = symbols.len() + infantry dice used`.
- `CalcLines(stealing)`: castle lines plus an extra `{symbols: [Daimyo]}` line appended when attacking an already-conquered castle.
- Commands: `attack <castle>` (Enum over castle names excluding own conquered castles and conquered clans; can_undo TRUE), `line <n>` (1-based, remaining uncompleted lines of currently attacked castle; can_undo false), `roll` (discard one die, reroll rest; can_undo false). Parser gating: CanAttack = current player and not attacking; CanLine = current player and attacking; CanRoll = current player.
- `Line` action: validates, logs "{player} completed {line render} with {b}{n}{/b} {die/dice}" (Plural of "die"), marks line complete, `CheckEndOfTurn`; if not end of turn, re-roll `len(roll) - using` dice and `CheckEndOfTurn` again.
- `CheckEndOfTurn` (game.go:98-192): when attacking - all lines complete => conquer log "{player} conquered the castle {name}[ from {owner}]", set owner, clan-conquered log if clan complete, NextTurn; else if cumulative min dice of remaining lines exceeds roll count => failed-attack log + NextTurn; else if reqDice == numDice and no remaining line affordable => failed + NextTurn. When not attacking - if no attackable castle (own castles and conquered clans skipped; conquered castles cost MinDice+1) is affordable with current roll count => failed + NextTurn. Failed message: "{player} failed to conquer {castle name|anything}".
- `ClanConquered(clan)`: all castles of clan conquered AND same owner => (true, owner).
- Scores: per conquered clan, owner gets ClanSetPoints; castles in unconquered clans give their Points to their owners. IsFinished when all 14 castles conquered. Placings metrics: `[score, clans_conquered_count]`.
- PubState == PlayerState == full game (no hidden info). PubRender == PlayerRender.
- Render (render.go): "Current roll" bold header, dice joined by three spaces; if attacking, "Currently attacking" section with the castle rendered against current roll; "Castles" section: castles grouped per clan in one row per clan (Table colSpacing 6, rowSpacing 1, centered cells), conquered clans render as text "{Clan} has been conquered by {player} for {b}N{/b} points"; per-castle table: name "(points)" centered, "({owner})" row if conquered, then per line: grey "N.", green bold "X " if affordable-now marker (attacking this castle or not attacking; not own castle; line uncompleted; affordable), line symbols/infantry cells (Table colSpacing 1), "complete" in grey for completed lines of the attacked castle; "Scores" section "{player}: {b}score{/b}" joined by three spaces.
- IMPORTANT render trap (RENDER_PARITY.md): Go `render.Table(cells, rowSpacing, colSpacing)` inserts literal spacer cells; Rust `Node::Table` has no spacing parameter - insert spacer cells by hand matching each Go colSpacing (0, 1, 2, 6 as used) and blank rows for rowSpacing 1.
- Go tests: `TestGame_New` (game_test.go: New(3) no error) and `TestGame_Attack` (attack_test.go: New(2), command current player "attack azu" no error) -> `test_game_new`, `test_game_attack`.

---

### Task 1: Crate core - domain, game logic, commands, tests

**Files:**
- Create: `rust/game/age-of-war-2/Cargo.toml` (copy liars-dice-2's, rename package to `age-of-war-2`, bins `age_of_war_2_{cli,http,repl,fuzz}`)
- Create: `rust/game/age-of-war-2/src/lib.rs` (Game struct, Gamer impl, attack/line/roll actions, check_end_of_turn, scores, clan_conquered, placings)
- Create: `rust/game/age-of-war-2/src/castle.rs` (Clan enum, Castle/Line structs, CASTLES data, dice Die enum with faces/colours/strings, min_dice, calc_lines, can_afford)
- Create: `rust/game/age-of-war-2/src/command.rs` (Command enum + command_parser with attack/line/roll parsers per Go command.go)
- Create: `rust/game/age-of-war-2/src/render.rs` (Renderer for PubState/PlayerState porting render.go, placeholder acceptable ONLY if it still renders all sections - parity is Task 2)
- Create: `rust/game/age-of-war-2/src/bin/age_of_war_2_cli.rs`, `..._http.rs`, `..._repl.rs`, `..._fuzz.rs` (copy liars-dice-2 stubs, rename)
- Create: `rust/game/age-of-war-2/tests/contract.rs` (`assert_gamer_contract::<Game>()`)
- Modify: `rust/Cargo.toml` (add `game/age-of-war-2` to workspace members)
- Create: `rust/game/age-of-war-2/RULES.md` placeholder line only, and `rules()` returning `include_str!("../RULES.md")` (full RULES.md authored in Task 3)

**Interfaces:**
- Produces: `age_of_war_2::Game` implementing `brdgme_game::Gamer`; `pub struct PubState`; `pub struct PlayerState`; castle/dice types in `castle.rs`.

- [ ] Step 1: Read reference crate `rust/game/liars-dice-2` (all of src/, Cargo.toml, tests/) and Go source `brdgme-go/age_of_war_1/` in full.
- [ ] Step 2: Write failing tests first (tests inline in lib.rs `#[cfg(test)]` per template): `test_game_new` and `test_game_attack` (1:1 ports), plus baseline suite: player_counts (2-6, reject 1 and 7), start rolls 7 dice + public roll log, can_attack/can_line/can_roll gating, attack own-castle/conquered-clan errors, attack sets currently_attacking + can_undo true, line completes + rerolls remainder, line can_undo false, roll discards one die + can_undo false, conquer castle (fix state: one line remaining, roll that affords it) transfers ownership + logs + next turn, steal adds daimyo line, clan_conquered detection + clan set scoring, scores/points, failed-attack turn pass, finished when all castles conquered, placings `[score, clans]` standard-competition tie, pub_state equals full public info, command after finished errors, wrong player errors.
- [ ] Step 3: Implement castle.rs, command.rs, lib.rs until `cargo test --package age-of-war-2` passes. Preserve Go behaviour exactly (see Key Go source facts). Use the borrow-order pattern from GAME_PORTING.md for `command()`.
- [ ] Step 4: `cargo test --package age-of-war-2` passes; `cargo build --package age-of-war-2` builds all 4 bins.
- [ ] Step 5: Commit `feat(age-of-war-2): port game core from age_of_war_1`.

### Task 2: Render parity

**Files:**
- Modify: `rust/game/age-of-war-2/src/render.rs`

**Interfaces:**
- Consumes: `PubState`/`PlayerState` from Task 1.

- [ ] Step 1: Port render.go faithfully (sections, spacer cells for colSpacing 0/1/2/6, rowSpacing 1 blank rows, centering, colours, bold).
- [ ] Step 2: Build Go CLI (`go build -o /tmp/render-parity/age_of_war_1_go ./brdgme-go/age_of_war_1/cmd`), Rust CLI, and `render_plain`; compare pub render and every player render at New (2p and 4p) and after representative commands (attack, line, roll, failed attack) using `scripts/render-compare/render.sh`, identical names both sides. Structural comparison only (RNG differs).
- [ ] Step 3: Fix discrepancies until wording/spacing/alignment/ordering match. Record the side-by-side outputs in the task report.
- [ ] Step 4: `cargo test --package age-of-war-2` still green. Commit `feat(age-of-war-2): render parity with Go`.

### Task 3: RULES.md

**Files:**
- Modify: `rust/game/age-of-war-2/RULES.md`

- [ ] Step 1: Read `docs/authoring/RULES_AUTHORING.md` in full and the age-of-war source; write RULES.md per required sections (Overview, Components, Turn Structure with inline commands, Scoring with worked example, Game End, Winning, Reading the Display with a real render captured from the Rust CLI in a ```brdgme block, Commands table, Strategy Tips as header + "tips will be added" note since no rulebook text is available).
- [ ] Step 2: Verify command syntax against command.rs and scoring against lib.rs. Commit `docs(age-of-war-2): add RULES.md`.

### Task 4: Registration, deprecation, fuzz, lint

**Files:**
- Modify: `rust/Dockerfile` (final stage `age-of-war-2` copying `age_of_war_2_http`, mirroring liars-dice-2 stage)
- Modify: `docker-bake.hcl` (add `age-of-war-2` to `tgt` array)
- Modify: `Tiltfile` (add `"age-of-war-2"` to Rust games list)
- Create: `k8s/base/game/age-of-war-2/{deployment.yaml,service.yaml,game-version.yaml,kustomization.yaml}` (mirror liars-dice-2 manifests; GameVersion display name "Age of War")
- Modify: `k8s/base/game/kustomization.yaml` (add dir)
- Modify: `k8s/prod/app/kustomization.yaml` (add `ghcr.io/brdgme/brdgme/age-of-war-2` image override, mirror liars-dice-2 entry)
- Modify: `k8s/base/game/age-of-war-1/game-version.yaml` (add `isDeprecated: true`, mirroring liars-dice-1)

- [ ] Step 1: Make all registration edits, mirroring liars-dice-2 entries exactly.
- [ ] Step 2: Run fuzzer `cargo run --release --package age-of-war-2 --bin age_of_war_2_fuzz` for ~2 minutes; zero panics required.
- [ ] Step 3: `cargo fmt --all -- --check`, `cargo clippy --workspace --exclude web --all-targets -- -D warnings`, `cargo test --package age-of-war-2` all clean. Record test count via `cargo test --package age-of-war-2 -- --list --format terse | rg ': test$' | wc -l`.
- [ ] Step 4: Commit `feat(age-of-war-2): register and deploy, deprecate age-of-war-1`.

### Task 5: Tracking docs

**Files:**
- Modify: `docs/porting/GAME_PORTING_PLAN.md` (add "Done (Track B, 2026-07): age-of-war-2 ported..." entry with test counts, fuzz numbers, port notes, matching prior entries' style)
- Modify: `docs/BACKLOG.md` (move/mark age_of_war port done if listed)

- [ ] Step 1: Write the done entry and any port-specific notes (incl. decisions made without user input, if any).
- [ ] Step 2: Commit `docs: mark age-of-war-2 port done`.

---

## Self-review notes

- All 2 Go tests covered in Task 1; thin-suite baseline required by GAME_PORTING.md step 8 included.
- Placings tie semantics documented in Global Constraints.
- Render spacer-cell trap called out in Key facts and Task 2.
