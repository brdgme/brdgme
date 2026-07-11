# Render Parity Audit (Rust ports vs Go) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a repeatable procedure to compare plain-text renders from
Rust `_cli` binaries against the Go implementations, make the porting docs
require it, and audit every ported game for wording/spacing/alignment
regressions (trigger: category-5-2 lost all table column spacing).

**Architecture:** Both implementations speak the same one-shot JSON protocol
on stdin/stdout (`brdgme_cmd` in Rust, `brdgme-go/cmd` in Go). Renders are
returned as markup strings; a small markup-to-plain helper renders them to
text for comparison. One orchestrator (Fable, read-only + planning files)
delegates all work to Sonnet 5 subagents, one per game, each under a 150k
token context budget.

**Tech Stack:** Rust (`brdgme_markup`, `brdgme_cmd`), Go, `jq`, bash.

## Global Constraints

- Orchestrator uses read-only tools plus planning-file writes only.
- All subagents run on Sonnet 5 (`model: sonnet`), context budget 150k tokens.
- Audit agents REPORT discrepancies; they do not fix game code.
- Task 1 builds all binaries once; audit agents must not run builds
  (avoids cargo target-dir lock contention between parallel agents).
- Findings aggregate into
  `docs/superpowers/plans/2026-07-11-render-parity-audit-findings.md`.

## Known protocol facts (verified by orchestrator)

- Go: each game has `brdgme-go/<game>_1/cmd/main.go`; build with
  `go build -o <out> ./brdgme-go/<game>_1/cmd`. Reads ONE JSON request from
  stdin, writes one JSON response. `requestNew` has NO seed field.
- Rust: each game has a `<name>_N_cli` bin target; build with
  `cargo build --package <name>-N --bin <name>_N_cli` in `rust/`. Same
  one-shot request/response. `Request::New` accepts optional `seed`.
- Shared request shapes: `{"New":{"players":N}}`,
  `{"PubRender":{"game":"<state string>"}}`,
  `{"PlayerRender":{"player":P,"game":"<state string>"}}`,
  `{"Play":{"player":P,"command":"...","names":[...],"game":"..."}}`.
- Response `render` fields are markup strings (`{{...}}` tags, and in Rust
  possibly unexpanded table/align structures) - they must be rendered to
  plain text via `brdgme_markup` (see `rust/lib/markup/src/plain.rs`,
  `transform.rs`) before spacing comparison. Go renders tables to final
  strings inside the game (`brdgme-go/render`), so confirming what each
  side's markup looks like pre/post transform is part of Task 1.
- RNGs differ across languages, so identical states cannot be seeded.
  Comparison is structural: same player count, compare layout, column
  spacing, alignment, headers, wording - not random values.

---

### Task 1: Comparison harness + verified procedure (single investigator)

**Files:**
- Create: `scripts/render-compare/` helper(s) - a checked-in, documented way
  to (a) run a request against a game binary and (b) render markup to plain
  text. Smallest thing that works; a tiny Rust bin (e.g.
  `rust/tools/render_plain`) is acceptable if markup cannot be rendered to
  plain text with existing binaries.
- Create: `docs/porting/RENDER_PARITY.md` - the written procedure.

**Steps:**

- [ ] Build the Go CLI for category_5: `go build -o /tmp/cat5_go ./brdgme-go/category_5_1/cmd`
- [ ] Build the Rust CLI: `cd rust && cargo build --package category-5-2 --bin category_5_2_cli`
- [ ] Send `{"New":{"players":2}}` to each; capture `player_renders[0].render`.
- [ ] Determine how to render both markup strings to identical-format plain
      text (player names substituted, tables laid out). Inspect
      `rust/lib/markup` (`from_string`, `transform`, `to_lines`, `plain`) and
      `rust/tools/repl` for reusable pieces. Create the minimal helper.
- [ ] Reproduce the category-5 spacing bug: side-by-side plain renders must
      show Go with column gaps and Rust without (as in the bug report).
- [ ] Write `docs/porting/RENDER_PARITY.md`: exact commands to build both
      binaries, send requests, render to plain text, and what to compare
      (wordings, spacing, alignment, ordering, colors optional).
- [ ] Build ALL binaries the audit agents need (10 games listed in Tasks
      3-12): Go `go build -o /tmp/render-parity/<game>_go ./brdgme-go/<game>_1/cmd`
      and Rust `cargo build` for each `_cli` bin. List the binary paths in
      the returned summary.
- [ ] Return: procedure summary, helper usage, binary paths, category-5
      side-by-side sample.

### Task 2: Porting docs requirement

**Files:**
- Modify: `docs/porting/GAME_PORTING.md` (add a mandatory render-parity
  verification step in "Porting steps" and the checklist, referencing
  `docs/porting/RENDER_PARITY.md`).

**Steps:**

- [ ] Add a step requiring: generate renders with the `_cli` binary and the
      Go binary, render both to plain text, and compare wordings, spacing,
      and alignments for pub render and every player render, at game start
      and after representative mid-game commands.
- [ ] Keep edits minimal and in the existing doc voice.

### Tasks 3-12: Per-game render parity audits (parallel, one agent per game)

| Task | Rust crate / cli bin | Go package |
|---|---|---|
| 3 | category-5-2 / category_5_2_cli | brdgme-go/category_5_1 |
| 4 | battleship-2 / battleship_2_cli | brdgme-go/battleship_1 |
| 5 | farkle-2 / farkle_2_cli | brdgme-go/farkle_1 |
| 6 | for-sale-2 / for_sale_2_cli | brdgme-go/for_sale_1 |
| 7 | greed-2 / greed_2_cli | brdgme-go/greed_1 |
| 8 | liars-dice-2 / liars_dice_2_cli | brdgme-go/liars_dice_1 |
| 9 | no-thanks-2 / no_thanks_2_cli | brdgme-go/no_thanks_1 |
| 10 | sushi-go-2 / sushi_go_2_cli | brdgme-go/sushi_go_1 |
| 11 | sushizock-2 / sushizock_2_cli | brdgme-go/sushizock_1 |
| 12 | zombie-dice-2 / zombie_dice_2_cli | brdgme-go/zombie_dice_1 |

Each agent (prebuilt binaries from Task 1; no builds, no game-code changes):

- [ ] New game (2 players; also min/max player counts if they differ).
- [ ] Render pub + all player renders to plain text on both sides.
- [ ] Play 2-4 representative commands (use `Play` with the state string
      from the previous response) to reach mid-game render states.
- [ ] Compare: wording, whitespace/column spacing, alignment, row/section
      ordering, headers/legends. Ignore random values and player-name
      length effects (use identical names both sides).
- [ ] Return: verdict (CLEAN / ISSUES), each issue with side-by-side
      excerpt and suspected layer (game render code vs shared markup lib).

### Task 13: Old-architecture ports feasibility (single agent)

Ported games WITHOUT in-repo Go counterparts: acquire-1, jaipur-2,
lost-cities-1, lost-cities-2, tic-tac-toe-2, lords-of-vegas-1 (undeployed).
Their Go source (where it exists) is the old monolith at
`~/Development/brdg.me/game/<name>` (`RenderForPlayer`, no JSON CLI).

- [ ] Confirm which of these have Go sources in the old repo.
- [ ] Assess the smallest harness that could print `RenderForPlayer` output
      there (do not build it yet).
- [ ] Return: feasibility report; orchestrator raises with user before
      spending the effort.

### Task 14: Aggregate findings (orchestrator)

- [ ] Write `docs/superpowers/plans/2026-07-11-render-parity-audit-findings.md`
      summarizing per-game verdicts, common root-cause candidates (e.g. a
      shared markup table transform bug vs per-game render bugs), and
      follow-up recommendations.
- [ ] Report to user; fixing found issues is a separate follow-up decision.

## Self-Review Notes

- Spec coverage: docs requirement (Task 2), procedure (Task 1), per-game
  audit via one subagent per game (Tasks 3-12), "all ported games" gap for
  old-architecture ports handled via Task 13 + user consultation.
- No fixes are in scope; audits only report.
- Type/name consistency: binary naming `<name>_N_cli` matches crate layout
  in `docs/porting/GAME_PORTING.md`.
