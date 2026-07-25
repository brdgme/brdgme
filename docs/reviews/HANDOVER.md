# Rust review - session handover (2026-07-24)

Handover of the in-progress exhaustive Rust code review from the prior tool
session (Kimi K3) to this session. The authoritative shared-context doc is
`docs/reviews/2026-07-23-rust-review/handover.md`; this file captures the
live state at handover. `PROGRESS.md` in that directory is STALE (says
"PAUSED after unit 7 of 13"); trust the raw LOGs and this doc instead.

## Charter (verbatim values)

Exhaustive review of all Rust code in `rust/`. Review-only: write findings
to markdown, make NO code changes.

- Correctness - the code needs to do what it says.
- Quality - high quality, robust, reliable code.
- Simplicity - easy to read and follow, does one thing perfectly.
  Abstractions must earn their cost; modules/types neither oversized
  grab-bags nor over-fragmented.
- Consistency - consistent within the project and with library/framework
  idioms; hacks and workarounds aggressively flagged for idiomatic
  replacement.
- Dependencies - only popular, modern, well-maintained, battle-hardened
  deps; lean modern; avoid bespoke solutions where off-the-shelf fits.
  Exception: the custom serialisable command parser combinator in
  `rust/lib/game` is core to the vision - do NOT flag it, nor the
  duplicated `impl Parser for CommandSpec` in
  `rust/lib/game/src/command/parser/mod.rs`.

## Snapshot under review

- Worktree: `/home/beefsack/Development/brdgme-review-snapshot` (code in
  `rust/`), detached at `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`.
- Line numbers in findings must match the snapshot. Never review the main
  working tree (modified concurrently by other agents).
- Findings are written to the MAIN repo under
  `docs/reviews/2026-07-23-rust-review/` so they survive worktree removal.
- Never run workspace-wide cargo builds/tests.

## Review-file format and conventions

Curated per-unit file: `2026-07-23-rust-review/findings/<unit>.md`.
Structure: `# Findings: <unit>` title, `## <crate-or-module>` sections,
one `### <short title>` block per finding:

```
### <short title>
- severity: critical | major | minor | nit
- category: correctness | quality | simplicity | consistency | dependencies
- location: path:line        (relative to rust/, per the snapshot)
- finding: <what is wrong, with enough context to act on>
- recommendation: <concrete fix or direction>
```

Severity guide: critical = bug/data-loss/security; major = clear defect or
significant maintainability problem; minor = should fix, low urgency;
nit = polish. Clean areas are stated explicitly ("Areas reviewed and found
clean" / "Checked and found CLEAN" sections), and files end with per-unit
severity tallies.

Example finding (verbatim from `findings/lib-game.md`):

```
### Token::parse panics when the token byte-length cuts a multi-byte char in the input
- severity: critical
- category: correctness
- location: lib/game/src/command/parser/mod.rs:50
- finding: `if input.len() < self.token.len() || UniCase::new(&input[..t_len]) != ...` - the length check is in bytes, so an input whose first char is multi-byte can pass the check while `&input[..t_len]` splits that char. Example: token `"no"`, user types `"nn~"` (3 bytes) -> `&input[..2]` panics. Same server/WASM reachability as the Space finding.
- recommendation: Use `input.get(..t_len)` and treat `None` as a mismatch, or compare via `input.chars().zip(self.token.chars())`. Add a non-ASCII test.
```

## Logging / durability convention

- Raw worker dumps: `findings/raw/<unit>-<topic>.md`. Workers append
  findings incrementally, never one final dump.
- Per-unit Lead log: `findings/raw/<unit>-LOG.md` (exists for units 7-10;
  units 1-6 predate the protocol). Markdown prose/bullets, dates at
  session level only (no per-line timestamps). Dispatch entries use
  `### W<n> dispatched` / returned headings; decision bullets prefixed
  VERIFIED / RECONCILED / MERGED / CONFIRMED. Unit closes with a
  `## Curation complete (<date>)` entry with the severity tally.
- Lose-at-most-the-in-flight-worker: log every dispatch, return, and
  verify/reject decision as it happens.

## Per-unit status

| # | Unit | Status | Curated file |
|---|---|---|---|
| 1 | lib-game | DONE | findings/lib-game.md |
| 2 | lib-support | DONE | findings/lib-support.md |
| 3 | games-batch-a | DONE | findings/games-batch-a.md |
| 4 | games-batch-b | DONE | findings/games-batch-b.md |
| 5 | games-batch-c | DONE | findings/games-batch-c.md |
| 6 | games-batch-d | DONE | findings/games-batch-d.md |
| 7 | games-batch-e | DONE | findings/games-batch-e.md |
| 8 | games-batch-f | DONE | findings/games-batch-f.md |
| 9 | web-server | DONE (67 findings: 1 crit / 7 major / 37 minor / 22 nit) | findings/web-server.md |
| 10 | web-domain | IN PROGRESS - see below | (none yet; raw only) |
| 11 | web-frontend-email | PENDING | - |
| 12 | bot-operator-tools | PENDING | - |
| 13 | dependencies | PENDING | - |
| - | Consolidated final review | PENDING | - |

## Unit 10 web-domain - exact state

Scope: `rust/web/src` domain logic, ~14,220 LOC, 19 files. Log:
`findings/raw/web-domain-LOG.md`. No curated `findings/web-domain.md` yet.

Serial worker plan from the log (RESUME here, do not restart):

| Worker | Scope (rust/web/src/) | LOC | Status |
|---|---|---:|---|
| W1 | game/mod.rs (1101) + game/export.rs (223) + game/import.rs (369) + NATS handoff | 1,693 | RETURNED: 13 findings (0c/4M/4m/5n) in raw/web-domain-game-mod.md. Lead verification PENDING - spot-check key lines before curating. |
| W2 | game/server_fns.rs | 2,479 | pending -> raw/web-domain-game-serverfns.md |
| W3 | proposals.rs | 2,961 | pending -> raw/web-domain-proposals.md |
| W4 | stats/ (queries 2076, mod 353, viz 326) | 2,755 | pending -> raw/web-domain-stats.md |
| W5 | players.rs (1189) + friends.rs (581) + new_game.rs (660) | 2,430 | pending -> raw/web-domain-social.md |
| W6 | game_info/ (540) + models/ (135) + rules.rs (548) + settings.rs (572) + index.rs (107) | 1,902 | pending -> raw/web-domain-misc.md |

W1 headline findings: bot-turn wedge in 3 loss modes; NATS consumer never
restarted; no term()/DLQ (messages strand); is_eliminated wipe; export
includes private logs.

Handoffs from web-server (both resolved by W1, recorded in the log):
NATS term()/poison-message question and ack_wait cadence - no term/nak/
in_progress anywhere in web/src; ack-once after all work; "stranded
messages" confirmed minor.

## Remaining unit scopes (snapshot paths + LOC)

### Unit 11 web-frontend-email (~9,740 LOC, 17 files)

The residual of `rust/web/src` after units 9-10. Per prior handover: read
`docs/hydration.md` and `docs/email.md` first.

- `email/` - 7 files, 6,844 LOC (inbound 2014, commands 2189, sweep 1046,
  notify 679, render 553, outbound 355, mod)
- `components/` - 7 files, 1,437 LOC (game 660, opponent_slot 352,
  layout 316, form 75, others small)
- `app.rs` 924, `theme.rs` 465, `lib.rs` 70

### Unit 12 bot-operator-tools (~2,662 LOC)

- `bot/` 1,708; `operator/` 412; `tools/fuzz` 358; `tools/render_plain`
  32; `tools/repl` 10.
- `lib/rand_bot` (142) was already covered by unit 2 lib-support - confirm
  before dispatch, do not double-review.

### Unit 13 dependencies

- 40 Cargo.toml files (998 lines total; web 178 ln / ~53 deps; 27 game
  crates with near-identical 18-22 ln manifests), workspace root
  Cargo.toml (65 ln, NO `[workspace.dependencies]`), Cargo.lock (8,008 ln,
  709 packages), deny.toml (83 ln), rust-toolchain.toml.
- Known drift to audit: sqlx 0.8 (web) vs 0.9 (operator), getrandom 0.3
  (bot) vs 0.4 (web); 108 boilerplate game binaries; `lib/cost`
  single-consumer duplication vs splendor-2's local cost module.
- `inventory.md` (lines ~30-70 crate table; ~613+ "Edition & dependency
  management") already contains a full per-crate dependency breakdown -
  lean on it heavily.

### Consolidation

Final unit: merge the 13 curated findings files into one review markdown
(structure not yet prescribed; follow the finding format above and roll up
severity tallies).

## Gotchas

- `PROGRESS.md` is stale (stops at unit 7). Raw LOGs are the truth.
- Units 1-6 have no LOG files; their curated findings files are complete.
- Severity scale is critical/major/minor/nit - not High/Medium/Low.
- Known non-issues (do not re-flag): custom parser combinator; duplicated
  `impl Parser for CommandSpec` (see
  `docs/decisions/COMMAND_PARSER_SPEC_DEDUP.md`); DB-backed tests failing
  in plain local runs (backlog #40).
- `inventory.md` says web = 55 files / 45,645 lines, but that includes
  non-.rs assets; web/src is 52 .rs files / 43,455 LOC.
- Findings location paths are relative to `rust/`; verify line numbers
  against the snapshot worktree, not the main tree.
