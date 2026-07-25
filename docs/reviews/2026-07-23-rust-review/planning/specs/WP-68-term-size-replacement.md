# WP-68: term_size replacement

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Replace the unmaintained `term_size 0.3.2` crate (RUSTSEC-2020-0163) with its maintained successor `terminal_size` in `brdgme_cmd` (dp F13 major, ls F24 minor), and remove the corresponding `deny.toml` advisory ignore so `cargo deny check` enforces the advisory again.

**Architecture — how the pieces fit today (verified against live source 2026-07-25):**

- **The advisory:** RUSTSEC-2020-0163 is an "unmaintained" notice (not a vulnerability): `term_size` has been archived since 2020, last release 2018. The advisory itself names `terminal_size` as the replacement, and the existing deny.toml ignore comment says the same ("Replacement is terminal_size; requires a lib/cmd code change, deferred to a dependency-bump pass"). This WP is that deferred code change.
- **The dependency:** declared once, at `rust/lib/cmd/Cargo.toml:16`:
  ```toml
  term_size = "0.3.2"
  ```
  `brdgme_cmd` is linked by every game binary (all 27 `rust/game/*` crates) and the repl, so the unmaintained crate is in every game service image — which is why dp F13 rated it major despite the tiny surface.
- **The sole call site:** `rust/lib/cmd/src/repl.rs:186`, inside `fn output_nodes(nodes: &[Node], players: &[Player])`:
  ```rust
  let (term_w, _) = term_size::dimensions().unwrap_or_default();
  ```
  `term_w: usize` is the terminal width in columns. It is used only to pad each rendered line with background-colored spaces out to the full terminal width (lines 193-200: `if l_len < term_w { ...push " ".repeat(term_w - l_len)... }`). When the process is not attached to a tty (piped output, CI), `dimensions()` returns `None`, `unwrap_or_default()` yields `(0, 0)`, `term_w == 0`, the `l_len < term_w` guard is always false, and NO padding is emitted. That non-tty behavior must be preserved exactly.
  A repo-wide grep confirms there are no other `term_size` references anywhere in `rust/` (only this call site, the Cargo.toml line, Cargo.lock entries, and the deny.toml comment).
- **API difference (re-derived, this is the whole swap):**
  - old: `term_size::dimensions() -> Option<(usize, usize)>` — (width, height) as plain `usize`, queried against **stdout**.
  - new: `terminal_size::terminal_size() -> Option<(Width, Height)>` — newtype wrappers `Width(pub u16)` / `Height(pub u16)` (tuple structs with a public field, so `.0` accesses the `u16`), also queried against **stdout** (the crate additionally offers `terminal_size_of(fd)` for other fds — not needed here, the call site prints to stdout via `print!`).
  So the swap needs a `u16 -> usize` widening (`as usize`, lossless) and an explicit `None -> 0` fallback replacing `unwrap_or_default()`. Both crates return `None` on non-tty, so piped-output behavior is identical. Both query stdout, so tty behavior is identical.
- **The deny.toml ignore to remove:** `rust/deny.toml:31-34` (comment + entry, inside `[advisories].ignore`):
  ```toml
      # term_size 0.3.2, direct dependency of lib/cmd (brdgme_cmd), used across
      # all game binaries. Replacement is terminal_size; requires a lib/cmd
      # code change, deferred to a dependency-bump pass.
      { id = "RUSTSEC-2020-0163", reason = "direct dep of lib/cmd; unmaintained notice only, replacement (terminal_size) needs a code change deferred to a dependency-bump pass" },
  ```
  With the crate gone from the lock, a leftover ignore would itself produce a cargo-deny "advisory not encountered" warning — remove it in the same commit as the swap. cargo-deny runs in CI (`.github/workflows/ci.yml:95-108`, `cargo deny check` with working-directory `rust`), so this ignore removal is enforced, not cosmetic.
- **Version choice:** `Cargo.lock` contains NO `terminal_size` today (grep count 0 — nothing transitive to align with), so pick the current major: `terminal_size = "0.4"`. Its unix backend is `rustix` 1.x, already in the lock at `rustix 1.1.4` — no new rustix. Its Windows backend is `windows-sys`; the lock already carries three windows-sys versions (0.48.0, 0.52.0, 0.61.2), and terminal_size 0.4.x may pin a fourth (0.59/0.60) — that is a Windows-target-only, lock-file-only entry (this workspace builds linux + wasm32), acceptable; note it if `cargo tree -d` shows it, but it does not block this WP (the duplicate-set findings dp F18/F24/F25 are WP-69's problem and WP-69 deliberately lands after this).

**Tech Stack:** Rust 1.97.0 workspace at `/home/beefsack/Development/brdgme/rust`; crate `brdgme_cmd` at `rust/lib/cmd`; cargo-deny config `rust/deny.toml`.

**Global Constraints:**

- All cargo commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only (`-p brdgme_cmd`); NEVER workspace-wide builds/tests on dev machines (AGENTS.md resource constraints — ~30 binaries link brdgme_cmd).
- `cargo fmt --all -- --check` clean before committing.
- The authoritative pre-commit gate is `/home/beefsack/Development/brdgme/scripts/rust-test.sh` (throwaway Postgres/NATS containers); AGENTS.md requires it passes before committing any Rust change. Its DB-backed tests fail in a bare run without the containers — pre-existing, known (backlog #40), not caused by this WP.
- Do not run `cargo update` broadly; only the targeted dependency change (`cargo add`/edit + the lock delta it produces).

**Non-Goals:**

- Other lib/cmd findings — repl EOF spin, panic paths, warp handler, etc. (WP-06). Do not touch anything in `repl.rs` beyond line 186.
- Other deny.toml items — stale diesel/encoding ignores, warn->deny hardening (WP-69, BLOCKED-ON-DECISION D-23). Remove ONLY the RUSTSEC-2020-0163 entry and its comment.
- warp->axum consolidation (ls F25 / WP-71).

**Snapshot drift:** Checked 2026-07-25 against `/home/beefsack/Development/brdgme-review-snapshot/rust` (commit f8763a5). `lib/cmd/Cargo.toml:16`, `lib/cmd/src/repl.rs:186`, and the deny.toml RUSTSEC-2020-0163 entry are identical in snapshot and live tree. No drift. (One citation correction vs the finding bodies: dp F13 says the ignore is at "deny.toml:19-27" — in the live file the entry is lines 31-34 as quoted above; the findings' content is otherwise accurate.)

---

### Task 1: swap term_size -> terminal_size and drop the deny ignore

**Problem (restated):** `brdgme_cmd` directly depends on archived `term_size 0.3.2` (RUSTSEC-2020-0163), silenced by a deny.toml ignore. Single call site computes terminal width for line padding in the repl renderer.

**Files:**
- `rust/lib/cmd/Cargo.toml`
- `rust/lib/cmd/src/repl.rs`
- `rust/deny.toml`

**Steps:**

- [ ] In `rust/lib/cmd/Cargo.toml`, replace line 16:
  ```toml
  term_size = "0.3.2"
  ```
  with:
  ```toml
  terminal_size = "0.4"
  ```
- [ ] In `rust/lib/cmd/src/repl.rs`, replace line 186:
  ```rust
  let (term_w, _) = term_size::dimensions().unwrap_or_default();
  ```
  with:
  ```rust
  let term_w = terminal_size::terminal_size().map_or(0, |(w, _)| w.0 as usize);
  ```
  Rationale: `terminal_size()` returns `Option<(Width, Height)>`; `w.0` is the `u16` width; `map_or(0, ...)` reproduces the old `unwrap_or_default()` -> `0` non-tty fallback so piped output still gets no padding. `term_w` stays `usize`, so the consuming code (lines 193-200: `l_len < term_w`, `term_w - l_len`) compiles unchanged — do not touch it.
- [ ] From `/home/beefsack/Development/brdgme/rust`, run `cargo check -p brdgme_cmd` — expected: clean compile, and the lock delta swaps `term_size` (+ its now-orphaned `winapi` edge) for `terminal_size 0.4.x`. Expected `Cargo.lock` result: no `term_size` package remains (`grep -c '^name = "term_size"' Cargo.lock` -> 0), one `terminal_size` entry appears.
- [ ] In `rust/deny.toml`, delete lines 31-34 exactly (the four lines quoted in the Architecture section: the three comment lines and the `{ id = "RUSTSEC-2020-0163", ... },` entry). Leave every other ignore untouched.
- [ ] Run the verification suite from `/home/beefsack/Development/brdgme/rust`:
  ```
  cargo test -p brdgme_cmd
  cargo clippy -p brdgme_cmd --all-targets -- -D warnings
  cargo fmt --all -- --check
  cargo deny check advisories
  ```
  Expected: tests pass (brdgme_cmd's tests don't exercise the tty path — terminal width is inherently environment-dependent and untestable in CI; compile + clippy is the realistic automated coverage), clippy clean, fmt clean, and `cargo deny check advisories` reports no RUSTSEC-2020-0163 (neither as violation nor as unused-ignore warning). If `cargo deny` is not in the local dev shell, note that CI runs it (`.github/workflows/ci.yml:108`) and rely on the other three commands locally.
- [ ] Manual behavior check (both crates query stdout; confirm parity). Each game crate ships a dedicated repl binary (verified: `rust/game/tic-tac-toe-2/src/bin/tic_tac_toe_2_repl.rs` calls `brdgme_cmd::repl`):
  ```
  cargo run -p tic-tac-toe-2 --bin tic_tac_toe_2_repl
  ```
  In a terminal: the rendered output is background-padded to the full terminal width, as before (Ctrl-C to exit — confirming the first render is enough). Then confirm non-tty: `echo "" | cargo run -p tic-tac-toe-2 --bin tic_tac_toe_2_repl | head -20` produces output lines WITHOUT trailing padded spaces (term_w=0 path; the piped run may then loop/exit on EOF — that behavior is ls F22's, untouched here).
- [ ] Run `/home/beefsack/Development/brdgme/scripts/rust-test.sh` (authoritative pre-commit gate).
- [ ] Commit:
  ```
  fix(deps): replace unmaintained term_size with terminal_size

  RUSTSEC-2020-0163: term_size is archived; terminal_size is the
  advisory-recommended successor. Sole call site repl.rs:186 swapped
  (same stdout query, same None-on-non-tty fallback to zero width).
  Drops the deny.toml advisory ignore now that the crate is gone.

  Review: dp F13, ls F24 (WP-68)
  ```

---

## Findings disposition

| Finding | Severity | Disposition |
|---|---|---|
| dp F13 — term_size 0.3.2 unmaintained (RUSTSEC-2020-0163), direct dep of brdgme_cmd; replace with terminal_size, drop deny ignore | major | FIXED by Task 1 (dependency swap + repl.rs:186 rewrite + deny.toml:31-34 removal) |
| ls F24 — term_size unmaintained, sole call site repl.rs:186, drop-in replacement | minor | FIXED by Task 1 (same change; the two findings are the same defect seen from the deps unit and the lib/cmd unit) |
