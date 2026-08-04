# Review session state

Orchestrated review of the 2026-07-25..2026-07-30 remediation effort (127 commits)
against `docs/reviews/2026-07-23-rust-review/SUMMARY.md`.

**Everything in this directory is intentionally uncommitted.**

## Ground rules for any resumed session

- Do NOT run tests, benchmarks or lints. Running Rust tests crashes the machine.
  `git`, `rg`, `wc`, `ls` and file reads only.
- Do not commit, stage or push. Do not modify source. Read-and-report only.
- One subagent at a time, serially, at every tier.
- Leads write their report incrementally, flushing each finding to disk as it is
  confirmed. Quota loss is expected; unflushed context is lost work.

## Key context every Lead needs

- **The WP specs and the full 570-finding corpus were compacted out of the tree**
  (`d89fa345`, 2026-07-29). They survive in git history:
  `git show 868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/<file>`.
  Recover the spec before judging "did the fix fix the finding".
  Spec-directory listing cached at `03a-specs.md` in the session scratchpad.
- **WP-24, WP-27, WP-44, WP-53 and WP-79 have no spec file at all** - their
  `T3-B*` checklist rows are the only acceptance criteria. **WP-51 DOES have a
  spec** (`planning/specs/WP-51-invite-mailer-notify-dedup.md`, Tier-2) - an
  Orchestrator brief wrongly said otherwise; verify spec existence per WP rather
  than trusting a brief's summary of this list.
- Mechanical sweeps are precomputed in `00-sweeps.md` - read, do not re-derive.
  Caveat: its "populates stats meaningfully" column is unreliable; the rest is sound.
- Unit definitions and commit-to-finding mapping: `00-breakdown.md`.

## Method that produced the real findings

Recover the acceptance criteria, then check the **end state** against each
criterion individually, reading both the diff and the final code. Every
high-value finding in this session came from a commit that satisfied its
checklist row *literally* while breaking or missing what the row was for.
Commit messages and checklists read clean in all of those cases. Reasoning from
commit messages alone has produced false findings.

## HANDOVER POINT - 2026-07-31

**Unit 10 is fully closed (F-186..F-207). The Orchestrator session was retired here
at ~150k context; `00-HANDOVER.md` holds the paste-able prompt for its successor
and is current as of this point.**

Remaining: **Unit 11** (`hanamikoji-1` + unassociated tail fixes - the last review
unit), then **the unified report + remediation work breakdown**. Both are briefed
in `00-HANDOVER.md`.

## Owner concern raised 2026-07-31: vendoring policy

The owner considers vendoring third-party code something that **should be
forbidden except in rare circumstances where there is no alternative and the work
is completely blocked**. This is a policy position for the remediation plan, not a
review finding, and it must appear in the unified report's process-fixes section.

What the review has already established (do not re-derive):

- **WP-66's spec did gate it.** Step 0 was binding: "Bump ... and re-resolve before
  designing anything. If that alone puts every crate on one sqlx major, this spec
  collapses to section 3a and you are done - do NOT vendor anything. Only if no
  sqlx-0.9-compatible store release exists does 3b apply."
- **The gate was honoured.** `tower-sessions-sqlx-store` 0.15.0 pins `sqlx = "0.8.0"`
  upstream, so branch 3b was correctly live. The port itself was minimal and
  faithful (verified by direct diff against the registry copy), MIT licence and
  attribution are present, and the schema is unchanged.
- **The cost landed anyway: F-200.** The "minimal port, not a rewrite" criterion -
  correctly followed - *guaranteed* an upstream defect came along, and it is now
  first-party code in an authentication-adjacent path with no tests
  (`rust/lib/session_store` has no `tests/` and no `#[cfg(test)]` module).
- **`rust/deny.toml:45-49`** sets `[licenses.private] ignore = true`, so because the
  crate is `publish = false`, cargo-deny skips it entirely - the vendored MIT
  obligations are satisfied by hand and **never machine-checked**.

Open for the owner, to be put in the remediation plan rather than decided by a Lead:
whether "no compatible upstream release yet" should be sufficient grounds to vendor
at all, versus waiting, pinning the old major, or upstreaming a patch. **The scope
of vendoring across the repo has not been swept** - only `session_store` is known.
A sweep for other vendored/copied third-party code is a remediation-plan item.

## Session note

The session hit its quota during Unit 07b and again during Unit 09a (2026-07-30)
and resumed both times. 07b died before producing any finding and was
re-dispatched from scratch; 09a's report survived intact and its successor resumed
with zero rework - evidence that the incremental-flush rule works.

`00-HANDOVER.md`'s "where we are" section is stale from Unit 06 onward; **this
file's progress table is authoritative.** Resuming needs only these two files.

## Unit progress

| Unit | Scope | Status | Report | Findings |
|------|-------|--------|--------|----------|
| 00 | Survey / breakdown | done | `00-breakdown.md` | - |
| 01a | lib-game, support, parser (WP-01/02/03/04/09a/09b) | done | `01-core-libraries.md` | F-01..F-08 |
| 01b | lib/color, lib/cmd, game_client (WP-05/06/07) | done | `01b-color-cmd-gameclient.md` | F-09..F-17 |
| 01c | epilogue dedup (WP-08/08b) | done | `01c-epilogue-dedup.md` | F-18..F-21 |
| 02 | alhambra, modern-art, seven-wonders, starship-catan, pub_state | done | `02-games-critical-hidden-info.md` | F-22..F-35 |
| 03a | splendor-2, texas-holdem-2, acquire-1 | done | `03a-games-splendor-holdem-acquire.md` | F-36..F-47 |
| 03b | cathedral-2, sushizock-2, lords-of-vegas-1, jaipur-2 | done | `03b-games-cathedral-vegas-jaipur.md` | F-48..F-59 |
| 04a | sushi-go-2, love-letter-2, age-of-war-2, lost-cities-1/-2 | done | `04a-games-sushigo-loveletter-lostcities.md` | F-60..F-65 |
| 04b | red7-1, zombie-dice-2, battleship-2, for-sale-2, category-5-2 | done | `04b-games-red7-zombiedice-forsale.md` | F-66..F-77 |
| 04c | Unit 04 cleanup + parity, `abffb7aa` (WP-33), crate coverage table | done | `04c-games-cleanup-parity-wp33.md` | F-78..F-84 |
| 05a | Web server: auth + crypto | done | `05a-web-auth-crypto.md` | F-85..F-96 |
| 05b | Web server: admin, bot supervision, db.rs | done | `05b-web-admin-bot-db.md` | F-97..F-110 |
| 06 | Web domain: undo/concede integrity (1 large commit, high severity) | done (4th attempt; F-111..F-115 salvaged from attempt 3) | `06-web-undo-concede.md` | F-111..F-120 |
| 07 | Web domain: remaining (proposals, visibility, export/import, email canon) | done | `07-web-domain-remainder.md` | F-121..F-143 |
| 07b | WP-51 `dcd8844c` + WP-53 `3610b957` (07's unexamined tail) | done | `07b-wp51-wp53-tail.md` | F-144..F-149 |
| 08 | Web domain: stats/query perf | done | `08-web-stats-query-perf.md` | F-150..F-157 |
| 09a | SSE + events + inbound auth (obligations 1,2,3) | done (killed by quota, report intact) | `09-web-frontend-email-sse.md` | F-158..F-163 |
| 09b | Frontend + email tail + obligation 4 (theme.rs) | done | same file | F-164..F-174 |
| 09c | WP-60 `e5513ec6`, WP-76 `bc051164`, WP-77 `33150afe`, `ca7925bc` | done - **Unit 09 fully CLOSED** | same file | F-175..F-185 |
| 10a | Bot/operator/tools (obligations 1-7) | done | `10-bot-operator-tools.md` | F-186..F-196 |
| 10b | Dep/workspace hygiene: WP-65, WP-73, WP-66 (findings only) | done (killed by quota, report intact) | `10b-dependency-workspace-hygiene.md` | F-197..F-202 |
| 10c | WP-66 wrap-up + WP-64, WP-67, WP-69 x2, WP-70, WP-72, `22b68689` | done - **Unit 10 fully CLOSED** | same file | F-203..F-207 |
| 11 | Unassociated (hanamikoji-1 new game + tail fixes) | done - **ALL REVIEW UNITS CLOSED** | `11-hanamikoji-unassociated.md` | F-208..F-211 |

Final step after all units: compile a unified report and propose a remediation
work breakdown. Still uncommitted.

## Systemic patterns confirmed - belong in the remediation plan as process fixes

1. **The routing leak.** Findings deferred from one work package to another were
   treated as closed by the *sending* package, with nothing tracking whether the
   receiving package ever picked them up. Three confirmed cases (F-55, F-57,
   F-60); two are High. WP-09a/09b are the most common unfulfilled receiver.
2. **Inconsistent hardening within a single file.** WP-09 guarded one function
   while its neighbours on the same render path stayed raw-indexed (F-61 is the
   clearest case). `check_player`
   (`rust/lib/cmd/src/requester/gamer.rs:24-36`) gives **no** protection against
   short parallel vectors - it bounds the player index against `player_count()`,
   not the actual vector lengths. (Earlier briefs in this session said "a crate
   constant"; Unit 04b corrected that. The conclusion is unchanged.)

2b. **`validate` overrides cover the parallel-vector sweep but miss the one
   cross-field invariant each crate's remaining panic actually depends on**
   (F-66/67/68/76). **No crate reviewed so far has a `validate` test** - which is
   exactly why these got through. This is a distinct failure mode from F-06:
   the override exists and is still insufficient.
3. **Nobody checked `Log::public` content.** The programme targeted
   hidden-information leaks, but every fix and every test looked only at
   `pub_state` struct fields. No game crate tests the log layer. F-22 (High) and
   F-28 (Medium) are leaks that survived because of this.
4. **WP-10 3a was declared "for every game crate" and applied to 3 of 28.** No
   later WP swept the rest. 13 crates have no redaction test.
4b. **Tests and docs adjusted to agree with the code, instead of the code being
   fixed** - which erases the discrepancy the finding cited and leaves code, tests
   and docs mutually consistent but all wrong. **Three confirmed instances**
   (F-72a edited `RULES.md` down to match the code; F-83's new test asserts the
   unchanged value where the spec prescribed the changed one; F-79's new test
   re-hardcodes the legacy values). Belongs in the process-fixes section.

4c. **Pattern 4b now has FOUR confirmed instances** - F-95 is the fourth: the WP-35
   F1 concurrency test asserts a lower bound where the spec prescribed an upper
   bound, because the prescribed bound was unachievable under the design the same
   spec mandated. Note the escalation: this is not sloppiness, it is the
   acceptance criterion being quietly renegotiated by the implementation.

4e. **A landed, tested fix silently reverted by a later commit in the same
   programme** (F-109, High). WP-36 shipped ws F55's shutdown drain plus a
   dedicated regression test (`rust/web/tests/websocket_hygiene.rs`); the later SSE
   migration `efad81f` deleted the fix and the test together. The checklist row and
   both commits still read as closed. Distinct from the routing leak (never picked
   up) and from 4b (test edited to agree with code). Mitigation: at sign-off assert
   each closed finding's citation or regression test still exists. **Sweep the other
   units for a second instance - Unit 09 owns `efad81f`.** Note F-109 also found
   ws F55's *second* half (bot consumer and email sweep tasks get no shutdown
   signal) was never implemented at all.

4f. **A test that blesses the lenient half of a cross-boundary inconsistency**
   (F-104, High): `validate_bot_slots_accepts_case_mismatch` pins case-insensitive
   bot-name validation as intended, while all four consumers of the stored value
   match case-sensitively. A 4b relative worth naming separately - the test is not
   wrong about its own function, it is wrong about the system.

4d. **Hardening that converts a soft default into a startup panic, shipped with no
   deployment acceptance criterion** (F-96). Turning a missing env var into a
   `panic!` is correct security practice and a production outage if nothing sets
   the var. No WP had a deployment-manifest criterion.

5. **The `_ => <default>` substitution.** Converting a lookup-with-default into a
   `match` with a catch-all arm satisfies "make this exhaustive so no caller can
   silently fall back" rows without changing any behaviour (F-65).
6. **F-06 (High, remediated by R-21):** `Gamer::validate` formerly defaulted to
   `Ok(())`, making the D-36 trust boundary fail-open for 13 of 28 game crates
   (`00-sweeps.md`). R-21 makes it required: 27 supported games provide explicit
   validation, while `lords-of-vegas-1` retains only its explicit, commented
   owner-approved WIP exception and does not claim complete validation.

## Owner rulings - do not re-litigate

- **R-31 / `category-5-2` stays capped at 8 players.** The current platform
  supports only eight pre-approved player colors, so 9-10 players are unsuitable
  now. Do not implement the published 2-10-player variant; revisit the broader
  player-limit question in a future per-game review.
- **R-32 / F-20 is not applicable to `starship-catan-1`.** The game is fixed at
  two players, so its `0..2` placings range is correct. Do not change its source,
  player limit, gameplay, or add a non-two-player fixture or test.
- **`lords-of-vegas-1` is work in progress.** Its missing endgame (never assigns
  `finished = true`) is out of scope. Do not raise findings about missing or
  incomplete functionality there. F-50/F-57 and the no-`finished` observation are
  to be marked "WIP crate, excluded" in the unified report, not routed to
  remediation.
- **F-35 / `Status::Finished { stats: vec![] }`** (24 sites, 21 crates) is parked
  in WP-20 (`c F12`). Record occurrences; do not demand fixes or re-raise per
  crate.
- **F-81 / reconstructing hidden information from the public log is ACCEPTABLE**
  (owner ruling, 2026-07-30). A great deal of hidden information is reconstructible
  from public logs by design; this is equivalent to reconstructing it from memory,
  and brdgme does not intend to defend against it via ephemeral logging or any
  similar mechanism. F-81 is **not a finding** - record it as intended behaviour.
  **This ruling is general, not specific to `no-thanks-2`.** It does NOT excuse
  hidden information appearing *directly* in `Log::public` content (F-22, F-28
  remain valid) - only its reconstruction by inference from legitimately public
  entries. Any later unit weighing a log-reconstruction finding must apply this
  distinction: direct leak = finding, inferable = not a finding.
- **D-39 is unverifiable and that is the finding.** Its only record is a one-line
  SUMMARY entry plus the commit author's gloss; `docs/CODING.md` has no rule
  bearing on delete-vs-rewrite.

## Game half: closed out

All 28 `rust/game/*` crates have an owning sub-unit (table in
`04c-games-cleanup-parity-wp33.md`). Two qualifications:

- `hanamikoji-1` is Unit 11's.
- **`roll-through-the-ages-2` has never had a crate-level review**: 3,290 lines, no
  `validate` override, no redaction test, and the one function anyone did read
  contained F-83. Out of scope for a review-of-the-remediation (the crate was
  barely touched), but recommend a dedicated pass in the remediation plan.
- **01c's `V` marks are epilogue-shape only** and must not be read as crate-level
  coverage in the unified report.

## F-96 - resolved out of band (report: `F-96-turnstile-key.md`)

Owner-requested investigation, done 2026-07-30. Conclusions, do not re-derive:

- **F-96 as originally written should be DOWNGRADED, not remediated.** The startup
  `panic!` (`rust/web/src/main.rs:40-45`) is gated by `ALLOW_INSECURE_DEFAULT_KEY`,
  which dev and CI already set (`k8s/dev/web-patch.yaml:18-19`,
  `scripts/rust-test.sh:64`). Turnstile verification **fails closed** on every error
  path (`auth/server.rs:256-277`); the sole fail-open is `secret.is_empty() -> true`,
  which is precisely what the panic prevents. The code implements the actual house
  pattern correctly. It remains a **pre-rollout deployment blocker** (no manifest
  sets the var in prod), not a code defect.
- **The "dev default plus log warning" premise was FALSE for `rust/web`.** The house
  pattern is *panic unless an explicit opt-in flag is set* (`crypto.rs:56-75`), and
  `docs/CODING.md:701` explicitly forbids the dev-default pattern.
- **New finding (fold into the corpus): `TURNSTILE_SITE_KEY` has no startup check**
  and silently defaults to empty, rendering no widget and rejecting every login.
  Setting only the secret key is a total login outage - both must land together.
- **New finding, route to Unit 10 with F-90:** `rust/bot/src/crypto.rs:66-76` falls
  back to the hardcoded dev key **ungated in any environment** - a real
  `docs/CODING.md` violation and another instance of the bot/web crypto divergence.
- Deployment: `brdgme-config` is a **GitOps repo, not a Secret**
  (`/home/beefsack/Development/brdgme-config`). Commands are in the report.
- Suggested, not urgent: split `ALLOW_INSECURE_DEFAULT_KEY` - one flag currently
  disables two unrelated guards.

## Carry-forwards not yet consumed

- **From Unit 09c (F-175..F-185) - Unit 09 is now fully closed:**
  - **F-183 (High) must be remediated as ONE item with F-104 and F-138.** The
    email `new` command lowercases the bot name (`email/commands.rs:82-93`,
    written at `:398-401`); `validate_bot_slots` accepts it via
    `eq_ignore_ascii_case`; the bot service looks it up case-sensitively
    (`bot/src/config.rs:28`) and silently skips (`bot/src/main.rs:188-193`). The
    game is created, the bot is seated, and it **never takes a turn** - no error,
    no retry. Fix = canonicalize inside `validate_bot_slots` and return the
    canonical name; that closes all three. Precondition: `admin::create_bot`
    (`admin.rs:293-303`) permits arbitrary casing. F-185 is the decoy test that
    hid it (all-lowercase fixture) and must be re-fixtured in the same change.
  - **Counterweight to the "Test? y with no test" tally (now NINE):
    WP-76 and WP-77 have no spec AND no row in any of the eight `T3-B*`
    checklists** - `EXECUTION-README.md:408` records this as a deliberate gap for
    WP-76/77/79/80. Untested by design is not a falsified row; the unified report
    must not conflate them. WP-60 also has no spec (its criteria are the WP-60
    rows of `checklists/T3-B6-outbound-email-websocket.md`).
  - **SETTLED, do not re-open: WP-59 Tasks 9-14 are NOT a coverage hole.**
    `f56ff37` owns Tasks 9, 11, 12, 13; Task 10 was dissolved by WP-56
    (`da1ea24`) removing the whole `emails add/confirm/...` family; Task 14 is a
    deliberate non-implementation per the spec's own carve-out to WP-85.
  - **SETTLED: the `RouteOutcome` sweep of `email/inbound.rs` is closed.** Every
    return in lines 654-1405 re-read; the only `Done`-on-transient sites are the
    already-filed F-162 and F-169. **No third route has the defect.**
  - **REFUTED, do not re-derive:** (1) WP-60 gave the outbound tokens no expiry,
    single-use or rate limit, so there is no pattern-2 gap against
    `settings_email_token` on that axis - F-161's substance is untouched by WP-60;
    (2) `ca7925bc`'s game-start sweep is complete (all four
    `insert_game_from_service` callers notify) and it is not a pattern-4e revert
    (`+20/-0`, deletes nothing); (3) F-170 is not extended - the game-start mail
    reads `turn_emails_enabled` directly, so unsubscribed users do not get it;
    (4) no hidden-information leak in the game-start mail (rendered from the
    recipient's own seat); (5) WP-77's own default IS canonical - it is a
    byte-for-byte copy of the `bots.name` column.

- **From Unit 10c (F-203..F-207) - Unit 10 is now fully CLOSED:**
  - **F-206 (Medium) is the headline and a NEW PATTERN for the process-fixes section: a spec's own
    STOP-AND-REPORT trigger fired and the implementation answered it with a comment.** WP-69's
    spec §3b said to stop if `multiple-versions = "deny"` needed more than "roughly a dozen" skip
    entries; `e2ee5342` shipped **29** and wrote *"not papered-over sibling work"* directly above
    them (`rust/deny.toml:71-76`). That claim is falsified by `:131`'s own annotation
    (`tower-http 0.7.0`, "via web (first-party, pins 0.7.0 directly)", against
    `rust/web/Cargo.toml:44`) - the only one of the 29 with a first-party cause, all 29 checked.
    Compounding: WP-69 §5's "the flip must actually bite" negative checks are recorded in
    `EXECUTION-STATE.md` as **parked, never run**. Remediate as ONE item with F-199 and 10b's
    Coverage gap 3 - three views of one unenforced `bans` section. Beyond 4b/4c: the criterion was
    a stop-work condition, not a test.
  - **F-205 is a second new pattern: the finding whose premise was disproved, closed anyway, never
    amended.** `dp F12`'s "sentry drags actix-web + ureq into every build" was **never true** -
    neither is a sentry 0.48 default and nothing enables them; both are still inert `[[package]]`
    entries in `rust/Cargo.lock` at HEAD after a later regeneration. WP-67's own rider 2 required
    the downgrade be written back into the finding; `SUMMARY.md:44-46,139` and
    `findings/dependencies.md:103-108,157` are unamended. **Sign-off fix: a disproved mechanism
    must amend the finding, not merely close it.**
  - **F-207 (Low) is a deployment-checklist item, not a code finding** - group with the F-96
    family. Three different sqlx migrators write `_sqlx_migrations`: `sqlx-cli` **0.8.6** pinned in
    prod (`rust/Dockerfile:132`), **unpinned latest** in CI (`ci.yml:90-92`), and the **0.9**
    library in `#[sqlx::test]`. **No commit in the entire 127-commit range touches
    `rust/Dockerfile` at all** and no spec mentions the pin. Mitigating: `rg 'migrate!' rust` is
    empty, so nothing validates checksums at runtime.
  - **REFUTED in 10c, do not re-derive:** (1) WP-64 has **no** pattern 2, **no** silent default
    change, **no** feature narrowing and **no** pattern 4b - all four proved negative (it touches
    zero `.rs` files, and `git grep` for `#![deny/warn/allow]` at `4fb252da^` returns zero hits, so
    there was no stricter config to displace); (2) WP-66's `default-features = false` sqlx
    narrowing is **inert** - all four dropped features are compile-time-only and neither `bot` nor
    non-test `operator` uses a single sqlx macro/derive/`Any`; (3) `serde_yaml_ng` is a faithful
    fork - full `diff -ru` of both source trees shows only `i64::max_value()` -> `i64::MAX` plus an
    additive API, and all 7 call sites at HEAD were in WP-70's diff, all serialisation-only;
    (4) `[licenses.private] ignore = true` is correct config and **no** `cargo-deny` setting could
    machine-check the vendored MIT obligations; (5) `22b68689` (cargo-deny into `devenv.nix:31`)
    and WP-69's unspecified `allow-wildcard-paths = true` deviation are both **correct** - the
    latter improves on a wrong rider.
  - **F-203/F-204 (both Low) are WP-64 spec-vs-code gaps**, not regressions: the prescribed
    `[workspace.lints.rust]` table was never created (only the clippy half exists,
    `rust/Cargo.toml:78-79`), and rider 1's "never leave a bare-major spelling" is violated by ten
    of the 21 workspace entries - though §3b of the same spec endorses exactly that, so the
    criterion is internally inconsistent.
  - **Correction to 10b**: `deny.toml`'s skip list has **29** entries, not 24.
  - **CARRY TO UNIT 11**: `hanamikoji-1` has **no `rust/Dockerfile` stage** - 26 game stages
    (`:174-303`) against 28 game members; the other absentee is `lords-of-vegas-1` (WIP, excluded).
    It **is** built by `cargo build --release --workspace --exclude web` and then never copied into
    an image. Also check `docker-bake.hcl` and `k8s/base/game/` - 10b counted **43** k8s game
    Deployments against 26 image stages, so those two lists already disagree.

- **From Unit 10b (F-197..F-202):**
  - **"Untested by design" vs falsified rows - the unified report MUST NOT conflate
    these.** WP-65's nine checklist rows are all `Test? = n`; WP-64/66/67/69/70/73
    have no checklist row at all (explicitly deferred, BLOCKED-ON-DECISION
    D-19/D-20/D-23); WP-72 appears in no checklist and has no spec. **No 10b row
    counts toward the nine-strong "Test? y with no test" tally.** Group these with
    WP-76/77/79/80.
  - **F-200 (Medium) is the unit's real find and a new named pattern: a vendoring
    WP inherits an upstream defect and the "minimal port, not a rewrite" criterion
    *guarantees* it comes along.** The vendored session store's `migrate()` returns
    `Ok(())` before `create table` and without committing on the duplicate-key path;
    it is the **sole** creator of `tower_sessions.session` (nothing in
    `rust/web/migrations/`). Cold-start race with >1 web replica = startup reports
    success, table never created. **Recommend a "known upstream defects inherited"
    criterion for any future vendoring spec.**
  - **WP-72 is self-certifying - no spec, no checklist row, one-line commit
    message.** A work package that exists only as a commit cannot be verified by any
    sign-off procedure. Name this in the process-fixes section.
  - **REFUTED, do not re-derive:** WP-73's deleted `*_repl` binaries are a
    capability *move*, not a loss (`rust/tools/repl` supersedes all 27). A Worker
    claim that `docs/porting/GAME_PORTING.md` documents a non-existent package
    `brdgmen` is **wrong** - the Lead checked the file directly; `:215` matches
    `rust/tools/repl/Cargo.toml:2`. Do not carry it into the unified report.
  - **WP-73 verified good by exhaustive proof, not sampling:** all 108 pre-commit
    game bins normalise to exactly four distinct contents, 27 each. The `0.0.0.0:80`
    -> `:8080` default change is inert - all 43 k8s Deployments set `ADDR`
    explicitly to `8080`. `[lints] workspace = true` present on all 44 members.
  - **Coverage gaps to carry:** nothing tests the vendored `rust/lib/session_store`
    (authentication-adjacent, now first-party); `deny.toml`'s 24-entry `skip` list
    has no expiry and the weekly job never runs `bans`; two stale docs still assert
    the `0.0.0.0:80` default and cite a WP-73-deleted path.

- **From Unit 10a (F-186..F-196):**
  - **`prompt.rs` is REFUTED as a leak vector - do not re-raise.** Full field
    enumeration: pure renderer over a closed field list (`pub_state`,
    `player_state(acting seat)`, per-seat `command_spec`, `is_public OR
    targeted-at-me` logs); pattern-2 sibling check passes; the file predates the
    programme. **But two adjacent real leaks were found instead: F-192/F-193.**
  - **F-192/F-193 (Medium pair) - the hidden-info-to-third-party class DID land,
    just not where expected.** `lib/game_client/src/lib.rs:25-35,188-191` embed the
    whole game-service response body in error `Display`; for `fetch_game_data` that
    body carries **every** seat's `player_renders` plus raw state, and it reaches
    `tracing::error!(error = ?e)` with `sentry_tracing` installed. F-193 is the
    cause: `fetch_game_data` requests all seats and discards all but one.
  - **The sqlx-cache carry-forward is REFUTED, causality inverted:** WP-52 is an
    *ancestor* of WP-66; `rust/.sqlx` would today be an 81-entry 0.8-format orphan.
    Only `web` uses the macros, prepare always runs from `rust/web`, CI runs
    `--check` under `SQLX_OFFLINE`. Do not re-derive.
  - **`test_support` carry-forward DISCHARGED:** 28/28 consumers are
    `[dev-dependencies]`, the feature is not in `default`, and it is correctly
    `#[cfg]`-gated. No risk.
  - **F-189 extends F-183:** two case-sensitive sites, not one (`bot/src/config.rs:28`
    **and `:67`**, the second never cited), and `main.rs:186-194` returns `Ok(())`,
    which **acks and discards** the turn. The sibling "no providers" path returns
    `Err` and is retried - so the wrong-case path is the one that fails silently.
    Remediate as one item with F-104, F-138, F-183.
  - **F-196 (accepted, uncommitted):** immutable migration 031 adds nullable
    per-version descriptor snapshots; apply backfills them despite an unchanged
    observed generation, and the atomic lifecycle reconciliation re-points
    `game_types` from the newest fully snapshotted public, non-deprecated version.
    Availability remains public/non-deprecated-only while snapshots are incomplete.

- **From Unit 09b (F-164..F-174):**
  - **The `ssr` feature-gate question is REFUTED, definitively - do not re-raise.**
    `scripts/rust-ci-commands.sh:30` runs `cargo test -p web --features ssr`, and CI
    runs the same script (`.github/workflows/ci.yml:93-94`). 423 gated test
    functions across 25 modules are live. The failure mode would have been loud
    anyway: ungated integration tests need `sqlx`, which only `ssr` enables, so a
    plain `cargo test -p web` fails to compile (`docs/DEV.md:106`). **No "Test? y"
    row is retro-voided.**
  - **Obligation 4 DISCHARGED: F-15 stays LATENT, no live violation.** Every
    `--mk-soften-*` token referenced anywhere is emitted; game crates emit exactly
    `{(Pink,80),(Foreground,80),(Foreground,90)}` from three sites, identical to
    `IN_USE_SOFTENS`, and no game emits a `mix`. Do not re-run this sweep.
  - **F-173: F-128 is NOT closed and has no owner.** `from_matches_verified_email`
    compares in SQL (`LOWER`) while every write path canonicalizes in Rust;
    `İ@example.com` breaks. Strengthens Unit 07's `CanonicalEmail` newtype proposal
    - fold both into one remediation item.
  - **Remediation pairing:** F-169 with F-162 (same `RouteOutcome` contract,
    settings route vs invite route).
  - **F-171 is the fifth confirmed "Test? y" with no test**, and the most explicit -
    the row specified what to assert.
  - **New deployment-checklist item, F-96 family:** `config::public_base_url()`
    defaults to `http://localhost:3000`, which would make WP-58's
    `List-Unsubscribe` non-HTTPS and RFC 8058-invalid in prod.
  - **Possible coverage hole for the unified report:** WP-59 Tasks 9-14 have no
    confirmed owner.

- **From Unit 09a (F-158..F-163):**
  - **F-161 (High) ESCALATES F-129 + F-130 TO ACCOUNT TAKEOVER.** WP-56's inbound
    auth gate is fail-open three independent ways; the cleanest is
    `spf=fail; dkim=none` -> `Pass`, because the code requires SPF *and* DKIM to
    both say "fail" (inverting the DMARC rule). Combined with the settings token's
    lack of expiry/single-use/rate-limit, spoofing `From:` is account takeover.
    **This is the session's most severe finding and belongs at the top of the
    unified report's remediation order.** Unit 07 set this escalation condition
    explicitly and it fired.
  - **Obligation 1 SETTLED: `efad81f9` contains exactly ONE pattern-4e instance
    (F-109), demonstrated by enumerating all 12 touched files, not asserted.** Do
    not re-run this sweep. Also settled: WP-84's spec §3g *anticipated* the
    deletion and required a proof test which does exist, so **F-109's remediation is
    a bookkeeping fix on WP-36's row plus a decision on the never-implemented second
    half of ws F55 - NOT a revert of `efad81f9`.**
  - **OPEN QUESTION, potentially retro-voids many "Test? y" rows at once:**
    `rust/web/Cargo.toml:99-154` declares `hydrate` and `ssr` but **no `default`
    feature**, and test modules gated `#[cfg(all(test, feature = "ssr"))]` would
    then be silently compiled out. Half-established by 09a; **Unit 09b must settle
    it** by checking whether `scripts/rust-test.sh` passes `--features ssr`.
  - **Decoy tests are now a confirmed *class*, not incidents:** F-161(d) found two
    more (`classify_inbound_auth_softfail_is_not_fail`,
    `..._single_fail_is_not_fail`) whose inputs each contain an independently
    passing result, so both name-match the risk without exercising it.

- **From Unit 08 (F-150..F-157):**
  - **The "Test? y" gap is now the most-confirmed pattern in the session** - F-142,
    F-148, F-149 and F-150, the last being all seven rows of one WP. Elevate it to a
    top-level systemic pattern in the unified report.
  - **Two sharpenings of the F-109 sign-off rule, both from real decoys.** (i) A
    citation must be *reachable*, not merely present (F-147). (ii) **A regression
    test must actually call the function under test** - `wd F51`'s
    `rating_before_aggregates_exclude_nulls` name-matches its risk exactly and never
    calls `game_history`.
  - **New named pattern, "the documentation-only constant"** (F-153): `wd F50`'s
    "one const used by all eight sites" shipped as an `#[allow(dead_code)]` string
    used by zero sites, with a doc comment stating manual sync is now required.
    Sweep `rg "allow\(dead_code\)"` across the commit range at sign-off.
  - **Unit 10:** WP-52 deletes 82 entries from a workspace-root `rust/.sqlx/` cache,
    outside its own scope - likely WP-66 fallout. Confirm which directory
    `cargo sqlx prepare` targets and that nothing resolves against the deleted one.
  - **`Gamer::points()` handed BACK, not consumed.** WP-52 touches no `lib/game`
    surface; stats read DB columns only. It needs a remediation-plan owner, not
    another review unit. Do not route it to a further unit.
  - **REFUTED, do not re-derive:** `wd F51`'s LATERAL rewrite is genuinely
    semantics-preserving - the missing `IS NOT NULL` does not matter because
    `min`/`max`/`avg` ignore NULLs and `count(*)` was never filtered.

- **From Unit 07b (F-144..F-149):**
  - **Two process fixes for the unified report.** (i) Grep the checklists for
    "Test? y" rows and confirm a test actually exists for each - catches F-148,
    F-149 and Unit 07's F-142. (ii) **F-109's sign-off check must assert each closed
    finding's citation is *reachable*, not merely present** - F-147 defeats it as
    written (`send_turn_reminder` exists, has never had a caller, and its doc
    comment states the dedup as accomplished fact).
  - **Remediation pairing:** F-145 must be fixed in the same change as F-136 - the
    surviving duplicate of the abandoned wfe F36 dedup is where F-136 lives.
  - **REFUTED, do not re-derive:** (1) no pattern-4e revert in `dcd8844c` - it
    edited `sweep.rs::send_reminder` in place and `send_turn_reminder` was dead from
    birth; (2) `undo_game`'s unguarded `is_eliminated` write is correct by context
    (a finished game cannot be undone), not a wd F6 sibling miss; (3)
    `restart_core`'s pool-read-under-`FOR UPDATE` is not a deadlock - different
    table, no write in-tx.
  - **Attribution note:** WP-51 introduced none of F-144/145/146; they belong to
    WP-46 (`69bcd1e`) and the original #24 invite work.

- **From Unit 07 (F-121..F-143):**
  - **Unit 09:** F-130's Medium rating depends entirely on WP-56's
    `from_matches_verified_email` and its DMARC classification holding up. **If Unit
    09 weakens either, F-129 + F-130 escalate to account takeover.** Unit 09 must
    report explicitly on this either way. Also F-131 (SSE authenticates once at
    connect and never re-checks) and F-128's inbound normalization divergence.
  - **Remediation pairing:** F-138 closes the loop on Unit 05b's F-104 from the
    write side - `validate_bot_slots` is case-insensitive and does not canonicalize
    what callers store, so all four write paths can persist a name no consumer
    matches. One defect; remediate together.
  - **REFUTED, do not re-derive:** the `VisibilityCache` cross-user leak. Each
    instance is a local inside the per-request spawn at `events.rs:65`. Also
    **WP-42 was NOT reverted by the SSE migration** - a useful negative against
    pattern 4e.
  - **Remediation proposal:** a `CanonicalEmail` newtype whose only constructor is
    `canonicalize_email` would permanently close the F-124/F-127 class. The
    contract is currently enforced only by doc comment.
  - **Pattern 5 (`_ => <default>`) now has a High-severity instance in the web
    half:** F-136. Worth promoting in the unified report - it is no longer a
    game-crate curiosity.

- **From Unit 06 (F-111..F-120):**
  - **Unit 05 (already done - handle at unified-report time):** cross-check WP-38's
    bot-turn wedge-recovery sweep against F-119. If it gates on `is_turn` rather
    than re-deriving from the game service, F-119 has **no production mitigation**.
    This is Unit 06's one open dependency.
  - **Unit 07:** `game/import.rs:109,124` is the only site outside
    `update_game_command_success` writing a non-NULL `undo_game_state`, taken
    verbatim from an import bundle; `undo_game` replays it after checking only
    non-NULL. Attacker-controlled state replay via import.
  - **Pattern 2 gains a clean instance (F-116):** WP-40 added `AND NOT $9` to the
    `left_at` CASE in `update_game_command_success` and left the byte-identical
    sibling in `undo_game` alone.
  - **Pattern 4b gains a mirror-image variant (F-120):** instead of a test edited to
    agree with the code, a new `docs/CODING.md` rule scoped narrowly enough (three
    named functions) that `end_game` - a fourth unguarded lifecycle writer that
    rates the game - is invisible to the grep procedure the doc itself prescribes.
    Name this variant in the unified report's process-fixes section.
  - **Checked and clean, do not re-derive:** the sqlx cache is not duplicated at
    HEAD; `rating.rs:484`'s reversed assertion is Task 5 working as designed, not
    pattern 4b. The breakdown's Unit 06 "shared-core extraction" gotcha was a
    **false premise** - `9ba3736b` touches zero `rust/game/*` files.

- **Unit 09:** F-15's real emitter is `rust/web/src/theme.rs` (the
  `IN_USE_SOFTENS`/`IN_USE_MIXES` whitelist is unenforced).
- **Unit 10:** `http.rs`'s final form is axum via WP-71, so a malformed request
  *envelope* now yields a 400 with a text body rather than a `Response::SystemError`
  JSON - untested, and different from what WP-06's test implies. Also confirm which
  crates enable `rust/lib/cmd/src/test_support.rs`: it has 14 panic constructs and
  ships behind a cargo feature, not `#[cfg(test)]`.
- **Unit 08 (or whoever owns `lib/game`'s trait surface):** `Gamer::points()` has
  no documented ordering contract, and cathedral-2's sign is inverted relative to
  its own `calc_placings`.
- **Unit 11:** `hanamikoji-1` has a single unguarded epilogue site (`:833`) and no
  `finish_epilogue` - it copied the pre-WP-08 pattern.
- **Unit 07:** `auth/email_addr.rs::canonicalize_email` (WP-50) now runs on every
  auth entry path *before* uniqueness checks, so a canonicalization bug is account
  takeover, not a formatting nit. Also `email/inbound.rs:520
  find_user_by_settings_token` (WP-44) is a second, entirely unreviewed
  authentication mechanism. And `a9609e57`'s `import_game.rs` 100 MiB guard is
  Unit 07's (05a read the commit and closed out its auth/crypto content: none).
- **Unit 10 / any unit reasoning about abuse limits:** there is **no rate-limiting
  middleware anywhere in `rust/web`** (F-94, confirmed), yet two doc comments
  assert a per-IP limit as design justification. Do not trust those comments.
- **`rust/bot/src/crypto.rs` is a divergent duplicate of `rust/web/src/crypto.rs`**
  (F-90) - fixes landed only in the web copy. The duplicated-module sweep is **done**
  (05b): exactly one further duplicate, `rust/bot/src/nats.rs` vs
  `rust/web/src/nats.rs` (F-108, not yet diverged - the two `Bot*Event` structs are
  the wire protocol, copy-pasted with no shared crate and no round-trip test).
  `bot/config.rs` vs `web/config.rs` share only a filename. Unit 10 should fix F-90
  and F-108 together, not re-run the sweep.
- **Unit 09:** `events_public_handler` (`rust/web/src/events.rs:117-183`) is
  unauthenticated, subscribes every connection to `game.>`, and runs an **uncached**
  `is_game_publicly_visible` query per matching message while the authenticated
  handler beside it uses `VisibilityCache` - anonymous DB amplification, pattern 2,
  and no rate limiting anywhere (F-94). Also owns `efad81f`, the commit behind F-109.
- **Unit 06:** `db::pick_replacement_bot` (`rust/web/src/db/bots.rs:76-98`) takes
  `&PgPool`, not a transaction, and does SELECT-then-INSERT as two autocommit
  statements - it cannot be made atomic with a caller's `game_players` update.
- **Unit 10 / any unit reading `rust/bot`:** `rust/bot/src/prompt.rs` (442 lines)
  builds the LLM prompt from game state and has no owning sub-unit. It is the
  natural place for a hidden-information leak into a third-party API - the class
  Units 02-04 spent their budget on for `pub_state` and `Log::public`.
- **Units 03/04:** F-18 remediation ownership for `for-sale-2`, `sushizock-2`,
  `category-5-2`, `farkle-2` **and `battleship-2`** (F-71 - the list was one short)
  - the unmigrated crates carrying a copy-pasted epilogue without WP-08's
  `!was_finished` gate.
- **Unit 04c must cover `abffb7aa` (WP-33)**, which touches `farkle-2`, `greed-2`,
  `liars-dice-2`, `no-thanks-2`, `tic-tac-toe-2`. No sub-unit of Unit 04 owned
  those crates, so this would otherwise be a coverage hole. (`62b293df` touches no
  `rust/` files and needs no code review.)
- **Unit 04c:** confirm whether `for-sale-2`'s `pass()` rounding the half-bid in
  the player's favour - opposite to the published rules - sits inside the WP-11
  park. Confirm, do not assume.
