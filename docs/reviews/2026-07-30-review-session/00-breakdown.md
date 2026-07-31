# 2026-07-30 review-session breakdown

Purpose: drive a multi-Lead code review verifying that the 2026-07-23 Rust
review's remediation program was actually implemented correctly, plus
reviewing everything that landed in the same window unassociated with that
program. Read-only planning doc; not committed.

## Source review doc

- Current file: `docs/reviews/2026-07-23-rust-review/SUMMARY.md`
  - Added (as SUMMARY.md, compacting the full corpus): commit `d89fa345`,
    2026-07-29, "docs(review): compact 2026-07-23 rust review to summary".
  - **True origin** of the review corpus (REVIEW.md, findings/, planning/):
    commit `f0589894c1937c2c1134cf99523f1fd4e9a8f944`, 2026-07-25,
    "docs(review): comprehensive Rust code review of rust/ tree". The
    directory name says `2026-07-23` but no commit on that date touches the
    path anywhere in history - treat 2026-07-25 as the real start.
  - The full per-finding/per-WP detail (570 findings, 13 units, ~85 work
    packages, decisions D-01..D-56) was deleted from the working tree by the
    compaction commit `d89fa345` (-78,135 / +198 lines) but is fully
    recoverable from git history in the range `f0589894..868094a6`.
  - Remediation was declared COMPLETE at `868094a6c8177858dededdd5321ce0c03882ada5`
    (2026-07-29, "docs(review): execution state - T3-B8 complete, Tier 3
    phase done").

## Commit range in scope

- Range: `f0589894` (2026-07-25, inclusive) .. HEAD.
- HEAD at breakdown time: `503748c4055a13cf5c64cf9155bfa4787578c839`
  (2026-07-30, "docs: remove personal contact information").
- Total commits after f0589894 (exclusive) to HEAD: 127, plus the review
  commit itself = 128 commits spanning 2026-07-25 to 2026-07-30 (6 days,
  not weeks - the whole review-to-remediation cycle was fast).
- 86 of 127 commits carry an explicit `WP-NN` tag in the subject; 41 do not.
  Of the untagged ones: most are `docs(review): execution state - ...`
  process/tracking commits (no source changes) or carry a `T3-BN` tag
  instead of a WP number (Tier-3 verification-phase fixes, each tied to a
  specific subsystem); a smaller tail is genuinely unassociated work (new
  game, CI/test tweaks, doc cleanups) - see Unit 11 and the "docs/process
  only" note below.
- Full per-commit data (hash, date, WP tag, subject, files/insertions/
  deletions) is at
  `/tmp/claude-1000/-home-beefsack-Development-brdgme/25bea2ae-b5c7-4916-a42d-eea493de757c/scratchpad/wp-commit-map.tsv`
  (128 rows incl. header) - point reviewing Leads at this file directly
  rather than re-deriving it.

## Repo-wide context for every reviewing Lead

- Rust workspace root: `rust/` (repo root itself is not a Cargo workspace -
  there is no top-level `Cargo.toml`). Workspace members (rust/Cargo.toml):
  `bot`, `operator`, `web`, `lib/{cmd,color,cost,game,game_bin,game_client,
  markup,rand_bot,session_store}`, `tools/{fuzz,render_plain,repl}`, and 27
  `game/<name>` crates (one per board game port). Edition 2024, resolver 2.
  `[workspace.dependencies]` was only introduced during this remediation
  (WP-64) - before that, versions were pinned per-crate.
- Key deps pinned at workspace level (post WP-64/66): sqlx 0.9 (unified from
  a 0.8/0.9 split - WP-66), axum 0.8.9 (web replaced warp in lib/cmd via
  WP-71), sentry 0.48 trimmed to explicit features (WP-67), serde_yaml_ng
  (replacing archived serde_yaml, WP-70), async-nats 0.49.1.
- Session-bootstrap conventions live in `/home/beefsack/Development/brdgme/AGENTS.md`
  (read in full before any Lead touches this repo) - key rules relevant to
  review Leads:
  - GitHub org is `brdgme`, not a personal account.
  - Never run `scripts/rust-test.sh` or workspace-wide `cargo build/test` -
    this review session is read-only anyway (no tests/lints permitted per
    the task's hard constraints), so this is moot but worth knowing why
    Workers must never be asked to run them.
  - `rust/lib/game/src/command/parser/mod.rs:813-1040` duplicated
    `impl Parser for CommandSpec` is deliberate (suggest-engine advancement
    mechanism), not dead code - a reviewer flagging it as duplication would
    be re-raising a rejected finding; see `docs/decisions/COMMAND_PARSER_SPEC_DEDUP.md`.
  - DB migrations under `rust/web/migrations/` are immutable once applied -
    any reviewer seeing an edited existing migration file (vs. a new
    numbered one) should flag it as a hard violation.
- Code-style / dependency-strategy conventions: `docs/CODING.md`. Decisions
  from this review were migrated there and into `docs/ARCHITECTURE.md` and
  `docs/decisions/*.md` - see the "Decisions and policies" table in
  SUMMARY.md for the exact mapping (e.g. D-03/D-04 undo semantics, D-05/D-08
  bot-by-name references, D-09 email canonicalization, D-33 pub_state
  redaction, D-36 deserialized-state trust, D-44..D-52 SSE topology).
- Architecture overview: `docs/ARCHITECTURE.md`. Backlog (parked items,
  including several opened by this review): `docs/BACKLOG.md`.
- Each reviewing Lead should treat `docs/CODING.md` + `docs/ARCHITECTURE.md`
  + the relevant `docs/decisions/*.md` file as ground truth for "was this
  fixed the way the project decided to fix it", not just "does this look
  like reasonable code".

## Docs/process-only commits (no code review needed)

These touch only `docs/reviews/**` planning/execution-tracking files or
top-level docs, with zero `rust/` changes - skip in every unit unless a Lead
specifically wants to audit the paper trail itself:
`43bcf72e, 37118d33, d5b19f7c, b0091a50, bc04704d, 10505410, b35d71ed,
8aa6eff0, 868094a6, 23f8ab78, d89fa345, 97637730, 4f44c2e2, 35743727,
8cde432a, 4c6af992, 0062822a, 7b1dfe2b, e3b95bd9, d856d6c3, 503748c4`, plus
merge commits `0f22cf95, 6f0b96cb, 394e7db7`.

---

## Review units

### Unit 01 - Core libraries (lib-game, lib-support/markup, lib-color, lib-cmd)
- **Maps to**: WP-01, WP-02, WP-03, WP-04, WP-05, WP-06, WP-07, WP-08/08b, WP-09a/09b
- **Commits** (11): `9abe8b4a` WP-01 char/byte panics; `91f26820` WP-02
  markup robustness; `c39786f9` WP-03 parser mechanical; `82157548` WP-04
  parser design items; `4a978cbe` WP-05 lib/color dead-API delete (1911
  deletions); `a543120f` WP-06 lib/cmd HTTP+CLI hardening; `63063a4b` WP-07
  game_client/rand_bot; `f13450a1` + `c14bc655` WP-08/08b epilogue dedup
  (11 + 2 files); `ff8f83ba` WP-09a requester-boundary trust; `c078c3ee`
  WP-09b per-crate `validate()` (16 files).
- **Files/size**: ~8,000 lines changed across ~90 file-touches; largest
  single commits are WP-05 (447+1911) and WP-08 (1158+852). No single
  commit is huge; the unit is large in aggregate - if it doesn't fit budget,
  split at WP-05/WP-06 boundary (parser+markup+color vs. cmd+game_client+
  epilogue+validate).
- **Gotchas**: WP-09a/b is the deserialized-state-trust boundary (D-36) -
  check every game crate's `validate()` actually rejects the invariants the
  review found broken, not just that a hook exists. WP-01's panics were
  char-index-used-as-byte-index; check non-ASCII test coverage was actually
  added, not just the crash path patched.

### Unit 02 - Game crates: critical + hidden-info fixes
- **Maps to**: WP-10 (pub_state redaction), WP-13 (starship-catan-1), WP-14
  (alhambra-1), WP-15 (seven-wonders-1), WP-25 (modern-art-2, 5 commits)
- **Commits** (9): `90dae6d2` WP-10; `4e0abe6d` WP-13; `c52f1a53` WP-14
  (1393+88, the duplicate-card-mint critical); `52680e57` WP-15 (686+258);
  `7821938a, af2c014b, b0babb89, e560a75a, 6c0c19c4` WP-25 (modern-art
  infinite-busy-loop critical + round-boundary/auction-state/UI fixes).
- **Gotchas**: these were the 3 non-dependency criticals in the original
  review (alhambra dup-mint, modern-art busy-loop) plus the hidden-info
  leak class (zombie-dice cup order, for-sale bids, starship Sensor peek -
  though zombie-dice/for-sale redaction itself is WP-10 here; starship
  Sensor peek render is folded into WP-13). Verify the fix actually closes
  the exploit (e.g. can a client still infer hidden state from `pub_state`
  by other means), not just that the reported repro is gone.

### Unit 03 - Game crates batch A (splendor, texas-holdem, acquire, cathedral/sushizock, lords-of-vegas, jaipur)
- **Maps to**: WP-17, WP-18, WP-19, WP-21, WP-22, WP-23 (+ T3-B3 follow-up)
- **Commits** (9): `614cf4f7` WP-17 splendor-2 onto lib/cost; `0688e03e`
  T3-B3 splendor-2/lib-cost follow-up hardening; `84b68b99` WP-18
  texas-holdem-2; `07ad4760` WP-19 acquire-1; `f5472388` WP-21
  cathedral-2 (Box::leak) + sushizock-2 (overflow); `7337c7ac` WP-22
  lords-of-vegas-1; `a692b638` WP-23 jaipur-2; `d5b19f7c`/`b0091a50` docs-
  only landed-markers (skip, listed for completeness).
- **Gotchas**: WP-17 changed splendor-2's cost representation to share
  `lib/cost` - check other games that also use `lib/cost` weren't
  regressed. cathedral-2's Box::leak was a real memory leak, not just a
  clippy nit - confirm the replacement doesn't reintroduce it under a
  different shape.

### Unit 04 - Game crates batch B (sushi-go, love-letter/age-of-war, lost-cities, red7, zombie-dice/battleship, for-sale/category-5, small-crate cleanup, dead-stats removal, parity fixes)
- **Maps to**: WP-24, WP-27, WP-28, WP-29, WP-31, WP-32, WP-33, WP-81, WP-83 (+ T3-B4)
- **Commits** (11 substantive + 2 docs-only): `66053159` WP-24; `eb49ceca`
  WP-27; `3174b3fc` T3-B4 love-letter-2 discard_card fix; `ed88fab9` WP-28;
  `071ace6e` WP-29 red7-1; `f16cb02c` WP-31; `807ab4e9` WP-32; `62b293df`
  WP-32 (also carries docs/execution-state payload, 1088+49 - mixed
  commit, check the code delta only); `abffb7aa` WP-33; `63f4aa91` WP-81
  dead stats-machinery deletion; `650e924e` WP-83 parity fixes
  (roll-through-the-ages-2, seven-wonders-1, red7-1); `bc04704d`,
  `1964dde5` docs-only (skip).
- **Gotchas**: WP-83 parity fixes are explicitly "released from the rules
  park" (BACKLOG #53) - confirm each matches official rules per the
  decision record, since most parity items were deliberately parked, not
  fixed (see Parked/deferred section of SUMMARY.md - don't flag the parked
  ones as regressions).

### Unit 05 - Web server (auth, admin, crypto, bot supervision, db.rs, cargo deps)
- **Maps to**: WP-34, WP-35, WP-36, WP-37, WP-38, WP-39, WP-41, WP-43, WP-68, WP-82
- **Commits** (12): `b49df619` WP-37 admin.rs; `baa5fc64` WP-41 db.rs;
  `347970a0` WP-39 bot consumer supervision; `13a1e693` WP-36 crypto/deploy;
  `ea9f7a2b` WP-34 auth races/session; `0a0f7e6d` WP-35 fail-closed
  posture; `c3b90122` WP-35 test; `914aa0c6` WP-38 bot-turn wedge recovery;
  `618156a7` WP-68 term_size->terminal_size; `4d31f6eb` WP-82 db.rs module
  split (8312+8149 - huge but mechanical file move, verify with
  `git diff --stat` / directory diff, not a full line-by-line read);
  `a9609e57` WP-43 web cargo deps.
- **Gotchas**: WP-38 (bot-turn wedge recovery) and WP-39 (consumer
  supervision) are the two most safety-critical items here - the review
  flagged "no recovery for any wedge mode" as a finding, confirm the sweep/
  retry/heartbeat mechanism actually recovers from each wedge mode the
  review enumerated, not just the reported one. WP-82's diff size is
  almost entirely file relocation (db.rs split into a module) - budget
  accordingly, don't read it as 16k lines of new logic.

### Unit 06 - Web domain: undo/concede integrity
- **Maps to**: WP-40 (D-03/D-04)
- **Commits** (1, but large): `9ba3736b` - 91 files, 3838+175 lines.
- **Gotchas**: this is the single largest and most severity-sensitive
  commit in the whole range - it closes the critical "`undo_game` on a
  finished game causes permanent rating corruption" finding plus TOCTOU
  guards on undo/concede skipping optimistic-locking. Given file breadth
  (91 files), this is likely a shared-core extraction touching every game
  crate's finish path - confirm the shared core is actually used
  everywhere (no game crate quietly kept its own bypass). Standalone unit
  because of both size and blast-radius; do not merge with anything else.

### Unit 07 - Web domain: remaining (proposals, bot-slot validation, sweep delivery, visibility, export/import, rules pages, email canon, invite-mailer, misc, game-service hoist)
- **Maps to**: WP-42, WP-44, WP-45, WP-46, WP-47, WP-48, WP-49, WP-50, WP-51, WP-53, WP-79 (+ T3-B5)
- **Commits** (14): `f4e76406` WP-44 proposals/email_token; `c1c1d200`
  WP-45 bot-slot validation; `69bcd1e9` WP-46 sweep delivery semantics;
  `34e41c5e` WP-47 visibility gates/stats anonymization; `3c6b3047` WP-42
  `is_proposal_visible_to_user` + cache; `9354a5c9` WP-49 public rules
  pages; `33f22f1c` WP-50 email canonicalization; `dcd8844c` WP-51
  invite-mailer/notify dedup; `3610b957` WP-53 misc server fns; `70922945`
  + `5e9bae2c` WP-48 export/import admin-only + de-flake test; `91c723d4`
  WP-79 hoist HTTP call out of `FOR UPDATE` transaction; `46847d40` T3-B5
  player-history page-ceiling clamp; `10505410` docs-only (skip).
- **Gotchas**: WP-47's `game_visibility` model and WP-44's `email_token`
  leak were both "built but never wired" findings - confirm every read
  path now actually checks visibility/token validity, not just that new
  columns/predicates exist. WP-79's fix moves an HTTP call out of a
  `FOR UPDATE` transaction - check no other transaction in the same module
  still holds a row lock across a network call.

### Unit 08 - Web domain: stats/query performance
- **Maps to**: WP-52
- **Commits** (1, but huge): `f374434d` - 95 files, 220 insertions / 2947
  deletions.
- **Gotchas**: net deletion-heavy - likely consolidating duplicated query/
  stats code rather than adding logic. Confirm no behavior silently
  changed in the consolidation (e.g. stats now computed differently) and
  that the anonymization from WP-47 (Unit 07) wasn't undone by this
  overlapping change (both touch stats surfaces - check for merge/ordering
  conflicts between WP-47 and WP-52).

### Unit 09 - Web frontend / email + SSE migration
- **Maps to**: WP-54, WP-55, WP-56, WP-57, WP-58, WP-59, WP-60, WP-76, WP-77, WP-84
- **Commits** (15): `fddc42df` WP-54 frontend UX errors; `f0a468b2` WP-55
  Turnstile hard-nav; `da1ea24f` + `4ca73ec7` WP-56 email From-auth
  redesign + SPF/DKIM classification (the account-takeover critical);
  `65c22edc` WP-57 at-least-once webhook delivery; `390dd3b8` + `5786a1b6`
  WP-58 RFC 8058 unsubscribe; `f56ff375` WP-59 inbound processing; `e5513ec6`
  WP-60 outbound tokens/metrics; `bc051164` WP-76 notify_game_emails;
  `33150afe` WP-77 default bot_name; `efad81f9` WP-84 WebSocket->SSE
  migration (two streams); `2b116b2f` T3-B6 SSE reconnect fix; `7da90b2d`
  clippy on websocket_client.rs (post-SSE cleanup); `dec967b6` latest-
  version-pick alignment; `ca7925bc` notify-on-game-start (extends WP-76).
- **Gotchas**: WP-56 fixed the account-takeover critical (settings route
  authenticated by a spoofable `From` header) - this is the single most
  security-sensitive item in this unit, review it first and carefully;
  confirm the new per-user token can't be forged/replayed. WP-84's SSE
  migration fully replaced WebSockets - check no dead WebSocket server
  code/routes remain reachable, and cross-check against Unit 05 (bot
  consumer supervision) since bot-originated events now ride the SSE path.

### Unit 10 - Bot / operator / tools + dependency & workspace hygiene
- **Maps to**: WP-61, WP-62, WP-63, WP-64, WP-65, WP-66, WP-67, WP-69, WP-70, WP-71, WP-72, WP-73
- **Commits** (14): `4f5f6d45` WP-61 bot service; `e682f6bc` WP-62 operator
  finalizer race; `d2decf85` WP-63 fuzz tool hang fix; `4fb252da` WP-64
  workspace.dependencies/package/lints; `2c28ae85` WP-65 workspace hygiene;
  `667c8f42` WP-66 sqlx 0.8/0.9 unification (101 files, 2304+831 - large
  but mechanical version-bump/import-path churn); `634c72db` WP-67 sentry
  feature trim; `e2ee5342` + `be185ccb` WP-69 deny.toml hardening;
  `8304baf5` WP-70 serde_yaml->serde_yaml_ng; `dcec1adf` WP-71 warp->axum
  in lib/cmd; `a5d6f102` WP-72 combine accepted-risk record (comment only);
  `22d00b8d` WP-73 108 game binaries collapsed to macro-free
  `brdgme_game_bin` (140 files, 231+1031 - largest file-count commit in
  the whole range, but each game crate's diff is a small, near-identical
  boilerplate deletion); `22b68689` devenv cargo-deny addition (docs/
  tooling, trivial).
- **Gotchas**: WP-66 and WP-73 are the two commits to sample rather than
  read in full - spot-check 4-5 game crates for WP-73 (confirm the
  generated/macro-free binary behaves identically) and check sqlx call
  sites in `rust/web` for WP-66 (confirm no 0.8-only API survived).
  dp-F14 (unsafe-libyaml still in Cargo.lock via serde_yaml_ng, WP-70's
  "backend half") is a known-open item (BACKLOG #57) - do not re-flag it
  as a miss, it's tracked as deliberately incomplete.

### Unit 11 - Unassociated work (new game + post-review tail fixes)
- **Maps to**: none (not part of the review/remediation program - genuinely
  new/independent work landed in the same window)
- **Commits**: 
  - New feature: `f4cbc51d` feat(game): add hanamikoji-1 (1761 lines, new
    crate `game/hanamikoji-1`), `c882d413` test expansion (213 lines,
    incl. multibyte-char test - notable given Unit 01's char/byte panic
    class, confirm this new crate doesn't reintroduce it), `16dae9dd` docs
    (rules/data/strategy, 268 lines).
  - Tail fixes with no WP/T3 tag: `a99bf754` test-container cleanup
    script; `e2aef66b` e2e test assertion fix; `3f52d2b7` fix(ci): allow
    insecure default key in test environments (verify this is scoped to
    test/CI config only, not a prod fail-open regression given WP-35's
    fail-closed-key finding in Unit 05).
  - `ae04843c` fix(game): sushi-go-2 draw_count dead arm -> `unreachable!()`
    (BACKLOG #59 - closes a gap noted in the review's parked-items list,
    small, 2 files).
- **Gotchas**: this unit is the catch-all for anything not covered above.
  hanamikoji-1 is a full new game port - it should be held to the same
  bar the review applied to every other game crate (char/byte safety,
  pub_state redaction, deserialized-state validate() hook, epilogue/
  finish-path reuse from Unit 01's WP-08 dedup) even though it predates
  no review finding of its own; treat this as a first-time review of a
  new crate, not a remediation check.

---

## Sizing summary (for the Orchestrator)

| Unit | Title | Commits | Rough size |
|---|---|---|---|
| 01 | Core libraries | 11 | Large (~8k lines, aggregate) - splittable |
| 02 | Game crates: critical + hidden-info | 9 | Medium |
| 03 | Game crates batch A | 9 | Small-medium |
| 04 | Game crates batch B | 13 | Medium |
| 05 | Web server | 12 | Large (one huge mechanical commit) |
| 06 | Web domain: undo/concede integrity | 1 | Large (91 files, high severity) |
| 07 | Web domain: remaining | 14 | Medium-large |
| 08 | Web domain: stats/query perf | 1 | Large (95 files, mostly deletions) |
| 09 | Web frontend/email + SSE | 15 | Large |
| 10 | Bot/operator/tools + deps | 14 | Large (two huge mechanical commits) |
| 11 | Unassociated (hanamikoji-1 + tail) | 7 | Small-medium |
