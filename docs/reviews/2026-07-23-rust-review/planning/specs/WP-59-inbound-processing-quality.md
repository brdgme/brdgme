# WP-59: inbound email processing quality

> **CITATION WARNING - line numbers in this spec are approximate and unverified.**
> Corpus-wide they measured **33-46% wrong**, and two "delete lines A-B" ranges
> would have destroyed live code. **Navigate by the named function, type or
> symbol** - never by line number alone. If the code at a cited location does not
> match this spec's description, **STOP and report**; do not improvise a fix or
> guess at the intended target.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Make the Resend inbound-email pipeline parse real-world mail correctly and fail legibly. Specifically: extract the bare addr-spec from `"Display Name <addr>"` header forms before routing or matching (wfe F4); harden the quote/attribution stripping for Gmail-wrapped, Outlook top-posted and localized replies (wfe F6); honour the first-stated accept/decline intent (wfe F15); release the proposal row lock before the early-exit invite emails (wfe F7); log the silent missing-roster return (wfe F12); stop the `" invite"` subject degradation and log its causes (wfe F13); give reply addresses one domain constant and per-route builders (wfe F14); delete the superseded command-loop scaffolding (wfe F8); de-duplicate the three `RESEND_API_KEY` + `ResendInbound` blocks (wfe F9); stop `restart` and `emails confirm` from re-classifying infrastructure failures as user refusals and from emailing `ServerFnError`'s `"error running server function: ..."` Display wrapper (wfe F21, wfe F24); let `emails confirm <code>` match **any** of the user's pending addresses (wfe F23); route the remaining inline SQL in `commands.rs` through `db.rs` helpers (wfe F26); reject a self-mention in `new` instead of silently dropping it (wfe F27); disclose the `bump` digest cap (wfe F28); and surface the email dispatcher's verb-collision problem (wfe F29) — **which this spec's verification proves is already live in two games; see "Cross-package / newly discovered". The dispatch-precedence work itself is CARVED OUT to WP-85 (D-54), so WP-59 no longer documents a reserved-verb set.**

---

## Architecture — how the inbound email pipeline works (read this before editing)

All code in this package lives in the `ssr`-gated `web` crate
(`/home/beefsack/Development/brdgme/rust/web`, package `web`, lib `web`,
edition 2024). `crate::email` is declared `#[cfg(feature = "ssr")]`
(`src/lib.rs:35-36`), so **every symbol below only exists under
`--features ssr`** and every test module in these files is gated
`#[cfg(all(test, feature = "ssr"))]`.

### The request path

`POST /api/webhooks/resend` -> `resend_webhook` (`src/email/inbound.rs:427-488`):

1. Reads `RESEND_WEBHOOK_SECRET` from env (:434-441).
2. Pulls `svix-id` / `svix-timestamp` / `svix-signature` via `header_value`
   (:418-423) and calls `verify_webhook` (:133-157).
3. `mark_event_processed` (:408-416) — `INSERT ... ON CONFLICT DO NOTHING`
   idempotency marker on `processed_webhook_events`; `Ok(false)` means duplicate
   delivery -> `200 OK`, do nothing.
4. Deserializes `ResendEvent` (:159-164) and its `data: ResendInboundData` (:166-174): `data.email_id: String`,
   `data.from: String`, `data.to: Vec<String>`, `data.received_for: Vec<String>`.
   **These are raw JSON strings from the webhook payload — nothing in the repo
   normalises them.**
5. `select_route(&data.to, &data.received_for)` (:178-182) -> first recipient
   whose local part parses via `parse_reply_address` (:37-51): `g-` = game,
   `i-` = invite, `s-` = settings. `None` and `Settings` both fall through to
   the settings route.
6. Dispatches to `handle_game_reply` (:491-588), `handle_invite_reply`
   (:591-820) or `handle_settings_reply_route` (:1087-1109), each passed
   `&event.data.from` and `&event.data.email_id`. Every one of them returns
   `()` — inbound processing failures never fail the webhook.

### Inside each route

- **Auth check.** Game/invite: `from_matches_verified_email(pool, user_id, from)`
  (:378-391, `LOWER(email) = LOWER($2)` against `user_emails`). Settings:
  `resolve_user_by_verified_from(pool, from)` (:393-404). **Both compare the
  raw webhook `from` string against stored bare addresses.**
- **Body fetch.** Reads `RESEND_API_KEY` from env, builds
  `ResendInbound { api_key, http: state.http_client.clone() }`, calls
  `fetch_raw_email` (the `InboundEmailSource` trait, :59-61; Resend impl
  :79-101; `StaticInbound` test double :103-114). This exact block is
  triplicated at :518-535, :625-642 and :1088-1107.
- **Body parse.** `extract_plain_text` (:53-56) hands the raw MIME to
  `mail_parser::MessageParser::default().parse(...)` and takes `body_text(0)`.
  Then `parse_reply_commands` (:8-28) turns the text into command lines.
- **Dispatch.** Game route: `run_game_reply_commands` (:301-349) drives
  `crate::email::commands::dispatch_email_command` (`commands.rs:1204-1284`)
  per line, stopping at the first error; `GameCommandLoopOutcome`
  (:278-296) selects the reply shape. Settings route: a hand-rolled loop over
  `dispatch_standalone_server_command` (`commands.rs:306-320`) at
  inbound.rs:1153-1170 (the `for line in &commands` loop; the `StandaloneCommandCtx` it uses is built at :1141-1148).
- **Reply.** `send_game_reply_response` (:887), `send_game_failure_report`
  (:972), `send_rules_reply_response` (:1042), `send_invite_reply_response`
  (:822-885) and `send_settings_response` (:1176-1214) each build a
  `crate::email::render::RenderedEmail` and call
  `crate::email::outbound::send_rendered_email(resend, rendered, from)` — the
  webhook's `from` value **is** the recipient.

### The command layer (`src/email/commands.rs`, 2259 lines)

`CommandError` (:19-24) is `User(String)` (emailed verbatim to the sender) or
`Internal(#[from] anyhow::Error)` (logged via `tracing::error!` at
inbound.rs:333 / :1165 and replaced with a generic sentence). The
`#[from] anyhow::Error` impl is what makes `?` on a `crate::db` result produce
`Internal`.

`dispatch_email_command` (:1204-1284) resolves the verb in this order:
`concede`, `end`, `undo`, `restart`, `rules`, `help`/`commands`, `new`, `bump`,
`list` (:1215-1246) -> `subscribe_toggle` (`subscribe`/`unsubscribe`, :42-48)
-> `dispatch_settings_command` -> `settings_verb` (`name`, `colors`/`colours`,
`theme`, `emails`, `settings`, :224-235) -> otherwise
`crate::game::execute_command` (the game move path). **18 reserved verbs, all
matched before any game grammar sees the line.**

`run_restart` (:1052-1141) and `run_emails_confirm` (:721-757) both funnel a
`leptos::prelude::ServerFnError` from a shared helper into `CommandError`.
Critical fact used throughout this spec: `crate::error::internal`
(`src/error.rs:6-12`; the `#[cfg]` is on :6, the fn body :7-12) is the **only** producer of the message
`"Internal server error"`, it already calls `tracing::error!` with the real
cause, and `ServerFnError`'s `Display`
(`~/.cargo/registry/src/*/server_fn-0.8.13/src/error.rs:218-234`) renders
`ServerError(s)` as **`"error running server function: {s}"`**.

### Related files

- `src/email/notify.rs` (679 lines): `reply_address(token) -> "g-{token}@brdg.me"`
  (:10-12), `turn_header_text`, `format_player_result`, `rules_url`,
  `browser_url`, `notify_game_emails`, `send_turn_digest_forced`. Inline
  `#[cfg(all(test, feature = "ssr"))] mod tests` with
  `reply_address_formats_token` at :500-503.
- `src/email/render.rs` (553 lines): `render_game_email(content, palette,
  players, thread_id, is_first_message, reply_address) -> RenderedEmail`,
  `palette_for_slug`, the `List-Unsubscribe` headers (:235-242) and the
  `<{thread_id}@brdg.me>` Message-Id domain (:227). **This package does not
  edit render.rs at all** — see Non-Goals.
- `src/db.rs` (6877 lines): the data-access layer. Relevant existing helpers:
  `get_user_email_prefs` (:2836-2844), `set_user_turn_emails_enabled`
  (:2847-2858), `set_user_invite_emails_enabled` (:2861-2872),
  `set_user_reminder_emails_enabled` (:2875-2886), `list_user_emails`
  (:2947-2958) returning `UserEmailRow { id, email, is_primary, verified_at }`
  (:2896-2901), `find_email_owner` (:2962), `insert_unverified_email`
  (:2975), `mark_email_verified` (:2998), `find_active_turn_games`
  (:3101-3119), `SWITCH_DIGEST_CAP = 20` (:2906), `cap_digest` (:2939).
  The `#22d` block carries the convention comment "Plain (non-macro) queries
  throughout, matching `get_user_theme` above" (:2888-2889) — **honour it: all
  new SQL in this package is plain `sqlx::query`/`query_scalar`, never the
  `query!` macro.** Inline `#[cfg(all(test, feature = "ssr"))] mod tests` from
  :3139 (`mod tests {` at :3140) to the file's final `}` at :6877, already
  containing two email-prefs tests: `email_prefs_default_all_true` (:6821-6836)
  and `set_email_prefs_toggles` (:6838-6876).

**No serialized type, DB schema, migration or public HTTP contract changes in
this package.**

---

## Tech Stack

Rust 1.97.0 (pinned by `rust/rust-toolchain.toml`), edition 2024. Axum 0.8 +
Leptos 0.8 (`server_fn` 0.8.13), sqlx 0.8 (Postgres), `mail-parser` 0.11.5
(already an `ssr` dependency, `web/Cargo.toml:51`, in the `ssr` feature list at `:118`), `resend-rs` 0.28,
`svix` 1.98, `anyhow` 1, `thiserror` 2, `tracing` 0.1. Let-chains,
`let ... else` and `Option::map_or` are all available and already used in these
files.

---

## Global Constraints

- Run all commands from `/home/beefsack/Development/brdgme/rust`.
- **Per-package only.** `cargo test -p web --features ssr`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`. NEVER a
  workspace-wide `cargo build`/`check`/`test` (AGENTS.md "Resource
  constraints": it links ~30 binaries and spikes RAM/disk).
- `web` server code does not compile without `--features ssr`. A bare
  `cargo test -p web` will not even see these files.
- Each task ends with `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  and `cargo fmt --all -- --check` clean.
- **No `sqlx::query!` macro SQL is added or changed anywhere in this package**
  (Task 11's new helpers use plain `sqlx::query_scalar`/`sqlx::query`, matching
  the surrounding `#22d` convention at db.rs:2888-2889). Therefore
  `cargo sqlx prepare` is **not** required. If you deviate and touch macro SQL
  anyway, you must run, from `/home/beefsack/Development/brdgme/rust/web`:
  `cargo sqlx prepare -- --tests --features ssr --all-targets`
  and commit the resulting `rust/web/.sqlx/` changes; CI enforces it with
  `(cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)`.
- **`db.rs` changes must land with tests** (`docs/CODING.md` "Testing
  Conventions": changes to `rust/web/src/db.rs` are rejected by reviewers
  without them). Task 11 is the only task touching db.rs and it carries two
  `#[sqlx::test]`s.
- DB-backed tests (`#[sqlx::test]`) need Postgres. A bare local run fails to
  connect — that is pre-existing (AGENTS.md, backlog #40), **not** a
  regression. The full gate is
  `/home/beefsack/Development/brdgme/scripts/rust-test.sh`, which provisions a
  throwaway Postgres 18 (port 15432) and NATS 2.11 (14222). Run it before the
  **final** commit of the package.
- Every existing test must keep passing **unmodified**, with exactly three
  exceptions, all explicitly called out: Task 4 deletes four tests whose
  subjects it deletes; Task 8 **extends** (does not rewrite) the assertion in
  `notify.rs`'s `reply_address_formats_token`; and Task 11 renames two call
  sites inside `commands.rs`'s `subscribe_unsubscribe_toggles_turn_emails`
  (:1338, :1349) because it deletes the private fn they call — the assertions in
  that test are untouched. Nothing else.
- Line numbers below are LIVE-file numbers as of the drift check. Tasks that
  shift numbering say so; later tasks locate by symbol name.
- Do not add a dependency. Everything needed is already in `web/Cargo.toml`.

---

## Non-Goals (owned elsewhere — do NOT absorb)

- **The From-authentication redesign — owned by WP-56 (BLOCKED-ON-DECISION
  D-1).** WP-56 owns wfe F1 (settings route authenticated solely by a spoofable
  From), wfe F5 (`from_matches_verified_email` is forgeable; email tokens are
  the only real auth) and wfe F17 (email-address management reachable via that
  path). Concretely, WP-56 owns **`from_matches_verified_email`
  (inbound.rs:378-391)** and **`resolve_user_by_verified_from`
  (inbound.rs:393-404)** and will change what they check (DKIM/SPF /
  authentication-results, token rotation, gating the `emails *` verbs).
  **This package does not modify either function, and does not modify the three
  call sites' logic (`inbound.rs:508`, `:611`, `:1112`) beyond passing an
  already-extracted address.** Task 1 is deliberately designed so that the F4
  fix lands entirely inside `resend_webhook` (:477-487) and `select_route`
  (:178-182) plus one new pure helper — **zero textual overlap with WP-56's
  functions.**
  - **wfe F4 is a parsing-correctness fix, not an auth fix.** It makes
    `"Alice <alice@x.com>"` match the stored `alice@x.com` instead of never
    matching. It does **not** make the From check trustworthy, and it must not
    pretend to: **do NOT, while in `resend_webhook`, add DKIM/SPF checks, add
    an authentication-results header read, rotate tokens, or "improve" the
    verified-address comparison.** Those are D-1's design space and a partial
    version of them landed here would have to be unpicked.
  - Landing order: **WP-59 Task 1 first.** WP-56 is decision-blocked; whatever
    D-1 decides operates on *an address*, and Task 1 is what guarantees the
    value handed to it is an addr-spec. If WP-56 somehow lands first, Task 1
    still applies unchanged — it edits different lines.
- **Inbound webhook delivery semantics — owned by WP-57 (BLOCKED-ON-DECISION
  D-2):** wfe F2 (idempotency marker inserted *before* processing, so failures
  are permanently dropped), wfe F10 (all processing inline before the webhook
  responds vs svix's ~15s timeout), wfe F16 (`verify_webhook`'s three
  `HeaderValue::from_str(...).unwrap()` at inbound.rs:144, :145 and :146). **Do not move
  `mark_event_processed` (:456-463), do not change any returned `StatusCode`,
  do not `tokio::spawn` the route handlers, and do not touch `verify_webhook`.**
  Task 1 adds one `return StatusCode::OK` path for an unparseable From value —
  that is deliberately the *same* status the surrounding code already returns
  for every unroutable delivery (the parse-failure `return StatusCode::OK` at
  :469, the wrong-event-type one at :474, and the function's tail at :488), so it does not pre-empt
  D-2's 5xx-for-retry decision.
- **Unsubscribe / RFC 8058 — owned by WP-58 (BLOCKED-ON-DECISION D-10):** wfe
  F3 (the advertised `mailto:unsubscribe@brdg.me` can never be honoured; the
  `List-Unsubscribe-Post: One-Click` header is invalid without an HTTPS URI)
  and wfe F25 (the standalone path rejects the `subscribe`/`unsubscribe` that
  `help_text` advertises). **Do not touch the `List-Unsubscribe` /
  `List-Unsubscribe-Post` headers at `render.rs:235-242` or
  `inbound.rs:1068-1075`, do not add an `unsubscribe@` special case to the
  settings route, and do not add `subscribe_toggle` to
  `dispatch_standalone_server_command` (`commands.rs:306-320`).** Task 8's
  reply-address consolidation stops at reply addresses and does not reach the
  unsubscribe mailto — see that task's edge cases.
- **Concede/undo TOCTOU and the duplicated server fns — owned by WP-40
  (BLOCKED-ON-DECISION D-3):** wfe F19, wfe F20, wfe F22. WP-40 owns
  **`run_concede` (commands.rs:887-943), `run_end` (:945-991) and `run_undo`
  (:993-1050)** and will extract `concede_core`/`undo_core` shared with
  `game/server_fns.rs`. **This package does not edit those three functions.**
  - Fence against Task 11 (wfe F26): F26's "inline SQL -> db helpers" work
    touches **only** these six sites — `commands.rs:751` (login_confirmations
    DELETE, inside `run_emails_confirm`), `:828-833` (email-prefs SELECT,
    inside `run_settings_summary`), `:853`, `:866`, `:879` (the three local
    `set_*_emails_enabled` UPDATEs) and `:1149` (game-version SELECT, inside
    `run_rules`). None is inside `run_concede`/`run_end`/`run_undo`, none is
    inside `run_restart`. **Do not "while I'm here" refactor any of the four
    game-lifecycle commands.**
  - `run_restart` (:1052-1141) is in **this** package for wfe F21 only, and
    Task 9's edit is exactly one `map_err` closure at **:1113**. Do not
    restructure it, and do not change `restart_core`'s signature in
    `game/server_fns.rs:986-995` — see Task 9's rejected alternative.
- **Bot-slot validation — owned by WP-45 (D-8):** wfe F18 (`bot:<name>`
  opponents are not validated against enabled bots). That is
  `classify_opponent` (`commands.rs:56-67`) and the `OpponentToken::Bot` arm of
  `run_new_command` (:367-376). Task 12 (wfe F27) edits **only** the
  `OpponentToken::Human` arm's self-mention branch at **:382-384**. Do not
  touch the bot arm, `classify_opponent`, or `find_enabled_bots`.
- **`processed_webhook_events` pruning (wfe F11), and wfe F30-F32 —** owned by
  **WP-46 / the sweep package**. Do not add a delete to `sweep.rs` and do not
  edit `sweep.rs` at all in this package.
- **Outbound tokens, metrics and render — owned by WP-60:** wfe F44-F51 and
  wfe F63, in `email/outbound.rs`, `email/render.rs`, `email/sweep.rs`,
  `theme.rs`, `app.rs`. **`render.rs` is WP-60's; this package must not modify
  it.** Task 8 (wfe F14) therefore covers reply addresses in `notify.rs` and
  `inbound.rs` only. The two remaining `brdg.me` literals in `render.rs` are
  **not reply addresses** and are each owned elsewhere: `:227`'s
  `<{thread_id}@brdg.me>` is a Message-Id domain (threading identity, WP-60)
  and `:237`'s `<mailto:unsubscribe@brdg.me?subject=unsubscribe>` is the
  unsubscribe target (WP-58). Same for `inbound.rs:1065`'s
  `<game-{}@brdg.me>` Message-Id: **leave it** — it is a threading identity, not
  a reply address, and rewriting it through `REPLY_DOMAIN` would falsely couple
  the two concepts.
- **The db.rs quality pass — owned by WP-41** (already finalized:
  `planning/specs/WP-41-db-quality-pass.md`). WP-41 adds a module-doc header at
  the very top of db.rs, sweeps 25 dead `updated_at = NOW()` clauses, edits
  `send_friend_request`, `update_game_command_success`,
  `friend_recent_visible_game`, `is_user_admin`,
  `delete_expired_unverified_emails`, `choose_colors`, `apply_rating_changes`,
  and appends 11 `#[sqlx::test]`s to db.rs's test module. See the coordination
  table below for the landing order and the two purely textual collision
  points. **No function WP-41 edits is edited here.**
- **Making the reply domain configurable.** Task 8 introduces a `const`. It
  does **not** add an env var, does not read `crate::config::public_base_url()`,
  and does not change any address the app currently emits. Deployment-time
  configurability is a separate decision.
- **Forgiving trailing noise after valid commands.** wfe F6's fallback
  suggestion ("at minimum treat a trailing block of unmatched noise after valid
  commands forgivingly") is **REJECTED-WITH-REASON** (this spec's own re-derived
  judgement — **not** a recorded user decision; there is no entry for it in
  `decisions-needed.md`) — see the disposition table. Do not change `run_game_reply_commands`' stop-at-first-error contract
  (inbound.rs:318-344) or `failure_report_header` (:252-275).
- **Any migration.** No file under `rust/web/migrations/` is created or
  modified.

### Coordination / landing order

| File / symbol | This package's edit | Other package also edits it | Order |
|---|---|---|---|
| `inbound.rs` `from_matches_verified_email` (:378-391), `resolve_user_by_verified_from` (:393-404) | **none** — Task 1 extracts the address upstream in `resend_webhook` instead | **WP-56 / D-1** rewrites both | Either order. Designed for zero overlap. **This is the package's most important fence: do not touch these two functions.** |
| `inbound.rs` `resend_webhook` (:427-488) | Task 1 adds the From extraction + one `warn`+`OK` path (:477-487); Task 5 does not touch it | **WP-57 / D-2** rewrites the marker placement (:456-463) and the status codes | **WP-59 first.** Task 1's insertion is after the marker block and before the dispatch `match`; D-2 restructures a disjoint region. |
| `inbound.rs` `select_route` (:178-182) | Task 1 rewrites the closure body | none | — |
| `inbound.rs` `handle_settings_reply_route` (:1087-1109) / `handle_settings_reply` (:1111-1174) | Task 5 replaces the fetch block and changes `handle_settings_reply`'s third parameter from `raw_body: &str` to `text: &str` | **WP-58 / D-10** adds unsubscribe detection to this route | **WP-59 first.** Task 5 shrinks `handle_settings_reply_route` to three lines, which is a strictly easier base for D-10 than the current duplicated fetch block. |
| `commands.rs` `run_concede` / `run_end` / `run_undo` (:887-1050) | **none** | **WP-40 / D-3** extracts `concede_core`/`undo_core` | Either order, disjoint. |
| `commands.rs` `run_restart` (:1052-1141) | Task 9 changes exactly one `map_err` at :1113 | **WP-40 / D-3** may reference `restart_core` as the factoring exemplar but does not edit `run_restart` | **WP-59 first** (one-line change). |
| `commands.rs` `run_new_command` (:337-443) | Task 12 changes the self-mention branch at :382-384 | **WP-45 / D-8** validates the bot arm (:367-376) | Either order, disjoint arms of the same `match`. Trivial merge. |
| `db.rs` — new helpers appended after `delete_expired_unverified_emails` (ends :3137), new tests appended to `mod tests` | Task 11 adds `delete_login_confirmation` and `find_game_version_id_for_game` + 2 `#[sqlx::test]`s | **WP-41** appends 11 tests to the same `mod tests`, and its **Task 6 rewrites `delete_expired_unverified_emails`' body (:3128-3136)** — the function immediately above WP-59's insertion point | **WP-41 first.** Two real collision points, both trivial, **verified against WP-41's accepted text**: (1) both packages append to the end of `mod tests` (WP-41's spec confirms `mod tests {` at :3140 and the file's final `}` at :6877); (2) WP-41's `delete_expired_unverified_emails` rewrite ends at :3136 with its closing `}` left at :3137, which is *adjacent* to WP-59's insertion point, so git's 3-line context can report a conflict even though the hunks are disjoint. **Correction to an earlier draft of this spec:** it claimed WP-41's *top-of-file module doc* collides. It does not — that insertion is at db.rs:1, ~3100 lines from anything WP-59 touches, and merges cleanly. Land WP-41 first anyway: rebasing two isolated additions is trivial, the reverse forces WP-41 to re-verify its whole line-numbered `updated_at` sweep. If WP-59 must land first, tell WP-41's implementer to re-derive db.rs line numbers. |
| `db.rs` `set_user_*_emails_enabled` (:2847-2886), `get_user_email_prefs` (:2836-2844) | **none** — Task 11 only adds *callers* | **WP-41** Task 1 removes the dead `updated_at = NOW()` from the three setters | Either order. Task 11 changes no db.rs SQL, so there is no collision. |
| `db.rs` `find_active_turn_games` (:3101-3119) | **none** — Task 13 changes only the `cap` argument passed from `bump_reply` | none | — |
| `render.rs` | **none** | **WP-60** (F44-F51), **WP-58** (F3/F25) | n/a |
| `rust/web/.sqlx/` | **none** (no macro SQL touched) | every db.rs-macro-touching package | n/a |
| `docs/authoring/COMMANDS.md` | Task 14 appends one section | none | — |
| `rust/web/src/error.rs` | Task 9 adds `pub const INTERNAL_ERROR_MESSAGE` and uses it inside `internal` (behaviour-identical: same literal) | none — **verified**: `WP-41-db-quality-pass.md` only *reads* `error.rs:7` in its reasoning and modifies no line of it; `WP-37-admin-pass.md` puts its `pub const ADMIN_REQUIRED` in **`admin.rs`**, not `error.rs`, and its only `Modify:` targets are `rust/web/src/admin.rs` | — |
| **Declared-path note** | WP-59's package definition (`work-packages.md`) lists paths `web/src/email/{inbound.rs,commands.rs,notify.rs,render.rs}` + `web/src/db.rs`. This spec **drops `render.rs`** (owned by WP-60/WP-58, see Non-Goals) and **adds `web/src/error.rs`** (one const, Task 9) and **`docs/authoring/COMMANDS.md`** (Task 14). Both additions are outside the declared list and both are collision-free per the rows above. Flagged here rather than silently absorbed. | | |

---

## Snapshot drift (verified 2026-07-25 against snapshot commit `f8763a5`)

| File | `diff -u snapshot live` | Consequence |
|---|---|---|
| `rust/web/src/email/inbound.rs` | **empty, exit 0** | All findings' inbound.rs line citations are still exact. |
| `rust/web/src/email/notify.rs` | **empty, exit 0** | — |
| `rust/web/src/email/render.rs` | **empty, exit 0** | — (not edited anyway) |
| `rust/web/src/email/commands.rs` | **126 diff lines** — the #47 concede-replacement + `end` work | **Findings' commands.rs citations are STALE.** Mapping below. |
| `rust/web/src/db.rs` | **606 diff lines** — #47 (`bots.can_replace_humans`, `concede_game_replace`, `replacement_bot_available`, ranked placings) | Findings' db.rs citations are stale; none of this package's db.rs sites was affected. |

Stale-citation map for `commands.rs` (finding -> live):

| Finding | Cited | LIVE | Note |
|---|---|---|---|
| wfe F21 | :1044 | **:1113** (`.map_err(|e| CommandError::User(e.to_string()))`), `run_restart` at :1052 | +69 lines from the `run_end` insertion |
| wfe F23 | :731 | **:732** (the unverified-address `query_scalar`), `run_emails_confirm` at :721 | |
| wfe F24 | :744 | **:747** (`.map_err(|_| CommandError::User("Invalid or expired confirmation code."))`) | |
| wfe F26 | :731, :750, :826-833, :847-884, :1079-1080 | **:732-738**, **:751-755**, **:827-834**, **:848-885**, **:1148-1154** | |
| wfe F27 | :381 | **:382-384** (`if id == ctx.user_id { continue; }`) | |
| wfe F28 | :454 | **`bump_reply` at :449-475**, the fetch at :455-457, `cap_digest` at :458, the reply `match` at :470-474 | |
| wfe F29 | :1146 | **`dispatch_email_command` at :1204**, the verb `match` at :1215-1246 | The `"end"` arm at **:1217 is NEW** (note: **:1219 is the `"restart"` arm**, not `end` - an earlier draft of this spec had this wrong and it is the package's most load-bearing citation) (post-snapshot) — this is what creates the live collision. |

Reproduce with, for each file:
`diff -u /home/beefsack/Development/brdgme-review-snapshot/rust/web/src/email/inbound.rs /home/beefsack/Development/brdgme/rust/web/src/email/inbound.rs`

**Independently re-confirmed in the lead review (2026-07-25):** `inbound.rs`,
`notify.rs` and `render.rs` diff empty with exit 0; `commands.rs` diffs
**126** lines and `db.rs` **606** lines. Live line counts also match this
spec's Architecture section exactly: `commands.rs` 2259, `notify.rs` 679,
`render.rs` 553, `db.rs` 6877, `inbound.rs` 2014.

**Unit 12 (`web-frontend-email`) had no verification pass** — verification
covered units 1-9 only. Every premise below was verified for the first time
while writing this spec, from live source.

---

## Disposition table — every finding re-derived from live source

| # | Claim | Verdict | What the spec does, and why |
|---|---|---|---|
| **wfe F4** | From/recipient matching "likely breaks" on `"Display Name <addr>"` forms | **CONFIRMED — hedge resolved: the code definitely breaks on those forms.** | The hedge was about Resend's payload shape, not the code. The code side is unconditional: `parse_reply_address` (inbound.rs:38) does `addr.split('@').next()`, so `"Name <g-tok@brdg.me>"` yields the local part `"Name <g-tok"`, which matches no `g-`/`i-`/`s-` prefix -> `None` -> the delivery silently falls into the settings route (:484-486). `from_matches_verified_email` (:387) binds the raw `from` into `LOWER(email) = LOWER($2)`, so `"Alice <a@x.com>"` can never equal the stored `a@x.com`. And the same raw string is the recipient handed to `send_rendered_email` (:884, :962, :1039, :1084, :1214). Task 1 adds a pure `extract_addr_spec` and applies it once in `resend_webhook` and once in `select_route`. `mail_parser` **is** already a dependency (`web/Cargo.toml:51`, in the `ssr` feature list at :118) exactly as the finding says, and it is used to implement the extractor — no new dependency, no hand-rolled RFC 5322 grammar. |
| **wfe F6** | Quote-stripping misparses Gmail/Outlook/localized reply formats | **CONFIRMED, recommendation ADJUSTED (one part skipped).** `parse_reply_commands` (:8-28) has exactly two stop conditions: a single line that both `starts_with("On ")` **and** `ends_with("wrote:")` (:14-16), and a `--`/`-- ` signature line (:19-21). A Gmail attribution that wraps puts `wrote:` on the next line, so the attribution text becomes a command; Outlook's unquoted `-----Original Message-----` / `From:` / `Sent:` block is not detected at all; a localized attribution never says `wrote:`. Task 2 adds three stop conditions plus one retraction rule, the most important being language-independent: **at the first `>`-quoted line, retract the block of collected lines since the last blank line if any of them is attribution-shaped** (ends with `:` or carries `<...@...>`). The finding's second suggestion — "treat a trailing block of unmatched noise forgivingly" — is **REJECTED-WITH-REASON (re-derived, NOT a recorded user decision — `decisions-needed.md` has no entry for it; see the note under the counts)**: it would silently swallow genuine typos, and `failure_report_header` (:252-275) already tells the sender exactly which line failed and which commands did apply. |
| **wfe F7** | `FOR UPDATE` row lock held across an outbound email send in the invite early-exit paths | **CONFIRMED, and the fix is unambiguous — no trade-off exists.** `tx` begins at :663, `lock_proposal_for_update` takes the `FOR UPDATE` lock at :671. Three paths then `return` with `tx` still alive: the "no longer open" send (:683-694), the missing-roster return (:704-707) and the "already responded" send (:709-720). Each awaited `send_invite_reply_response` does 3-4 DB round-trips plus an HTTPS call to Resend. **Derived: none of the three paths has written anything** (`update_proposal_player_response` is at :723-724, after all of them), so the correct release is a rollback, and rolling back before the send introduces **no** "committed but not emailed" window — there is nothing to commit. The genuine commit path already commits at :780 *before* its send at :819. Task 6 inserts an explicit `tx.rollback()` in each of the three branches. |
| **wfe F8** | Dead code: `run_commands_in_order`, `CommandLoopOutcome`, `error_reply_text` | **CONFIRMED dead outside their own tests — delete, as the finding's first option says.** `rg -n "run_commands_in_order|CommandLoopOutcome|error_reply_text" /home/beefsack/Development/brdgme/rust` returns hits **only** in inbound.rs: the definitions (:184-204, :227-238) and four tests inside this file's own `mod tests` (:1443-1492, :1645-1658). Nothing in `web/src`, `web/tests/`, `web/src/bin/`, or any other crate. The real loop is `run_game_reply_commands` (:301). Task 4 deletes the three items and those four tests. |
| **wfe F9** | `RESEND_API_KEY` fetch + `ResendInbound` construction duplicated three times vs `AppState` | **CONFIRMED (triplicated verbatim at :518-535, :625-642, :1088-1107); recommendation ADJUSTED — helper fn only, NOT an `AppState` field.** Re-derived: `AppState` (`src/state.rs:6-16`) is entirely `#[cfg(feature = "ssr")]` (`lib.rs:18-19`), so an inbound field *would* compile. It is rejected anyway: `InboundEmailSource` is a trait (`inbound.rs:59-61`), so the field would have to be `Option<Arc<dyn InboundEmailSource>>`, which (a) forces edits to all three construction sites — `main.rs:85`, `tests/ssr_pages.rs:45`, `tests/websocket_hygiene.rs:41` — for no functional gain, (b) makes `AppState`'s `Clone` derive carry a trait object, and (c) buys nothing measurable: `std::env::var` is a hashmap lookup, once per inbound email. Task 5 extracts `fetch_inbound_text(state, email_id) -> Option<String>`, which removes all three copies including their identical logging. |
| **wfe F12** | Silent return when the player row is missing from the roster, no log | **CONFIRMED.** `inbound.rs:704-707`: `let me = match players.iter().find(...) { Some(p) => p, None => return }` — the only early return in the whole 230-line function with no log line, while its ten siblings all log at info/warn/error. Task 6 adds a `tracing::warn!` (and the F7 rollback, same branch). |
| **wfe F13** | Invite response subject degrades to `" invite"` on lookup failure, unlogged | **CONFIRMED.** `send_invite_reply_response` (:822-885) folds three lookups, in the `let (game_type_name, game_version_id) = ...;` statement at :841-857, into `(String::new(), None)` via `.unwrap_or(None).unwrap_or_default()` and a `_ =>` arm, then builds `subject: format!("{game_type_name} invite")` (:867) -> `" invite"` with a leading space. Task 7 restructures the lookup to log each failure and fall back to the neutral subject `"Your brdg.me invite"`. |
| **wfe F14** | Reply-address formats hardcoded/duplicated; no domain single source of truth | **CONFIRMED, scope ADJUSTED (render.rs excluded).** Live: `notify::reply_address` (`notify.rs:10-12`) is used for the game route (inbound.rs:960, :1037, :1082) while the invite address is inline at `inbound.rs:882` (`format!("i-{}@brdg.me", ...)`) and the settings one at `:1191` (`format!("s-{user_id}@brdg.me")`). Task 8 adds `REPLY_DOMAIN` plus `invite_reply_address`/`settings_reply_address` next to `reply_address`. **render.rs is excluded** — its two `brdg.me` literals are a Message-Id domain (:227) and the unsubscribe mailto (:237), neither a reply address, and the file belongs to WP-60/WP-58. The finding's observation that `parse_reply_address` accepts any domain is left alone deliberately: it is what makes the existing `parse_reply_address("i-xyz@example.com")` test (:1298-1303) and any future domain migration work, and tightening it would be an auth-surface change (WP-56). |
| **wfe F15** | "accept" wins over "decline" regardless of order in the body | **CONFIRMED.** `inbound.rs:646-647` computes `accept`/`decline` with two independent `.any()` scans; `:722` resolves with `if accept { "accepted" } else { "declined" }`. A body containing both words in either order always accepts. Task 3 extracts a pure `parse_invite_intent` that returns the **first** line matching either verb, preserving the existing exact-line, ASCII-case-insensitive matching (so `"decline politely"` still counts as nothing, exactly as today). |
| **wfe F21** | `run_restart` maps internal errors to User errors; "emailed verbatim and unlogged" | **ADJUSTED — both halves of the premise are FALSE; a different, real defect is confirmed in the same line.** Every error path in `restart_core` (`game/server_fns.rs:986-1158`) was enumerated (table in Task 9). Infrastructure failures **all** go through `crate::error::internal` (`src/error.rs:6-12`), which already calls `tracing::error!` with the real cause and replaces the message with the fixed string `"Internal server error"`. So internals are **not** emailed verbatim and **not** unlogged. What *is* wrong at `commands.rs:1113`: (a) they are mis-classified `User`, so the email says `"error running server function: Internal server error"` where the generic apology belongs, and inbound.rs's Internal branch never fires; (b) **`ServerFnError`'s `Display` prepends `"error running server function: "` to *every* message** (`server_fn-0.8.13/src/error.rs:233-234`), so genuine user refusals are emailed as e.g. `"error running server function: This game supports 2, 3, 4 players, but the request has 5 (including you)"`. That leak-of-framework-noise is real, user-visible on every failed `restart`, and was not in the finding. Task 9 fixes both with a small classifier. |
| **wfe F23** | `emails confirm` only matches the most recently added unverified address | **CONFIRMED; recommendation ADJUSTED to avoid a security regression.** `run_emails_confirm` (:732-737) selects `ORDER BY created_at DESC LIMIT 1` and validates the code against that one address. The finding's fix — "match the code across the user's unverified addresses (join `login_confirmations`)" — would select *by code*, which **bypasses `validate_confirmation_code`'s attempt counter** (`auth/server.rs:378-387` - the `if confirmation.code != token` branch, whose `UPDATE ... attempts = attempts + 1` is :379-385 - bumps `attempts` only when a row is found and the code mismatches): a wrong code would find no row, never increment, and `CONFIRM_MAX_ATTEMPTS_PER_CODE` would stop protecting the email path. Task 10 instead **iterates the user's unverified addresses and calls `validate_confirmation_code` on each**, which keeps every existing rate-limit and cap intact. Code collisions across two of a user's addresses are handled by first-match-wins in `list_user_emails` order; the only side effect is that a wrong code bumps the attempt counter of each pending address it was tried against, which is documented in the code. |
| **wfe F24** | `validate_confirmation_code` DB errors masked as "invalid code", never logged | **CONFIRMED, with the same nuance as F21.** `commands.rs:747` is `.map_err(|_| CommandError::User("Invalid or expired confirmation code."))`, discarding the error. Re-derived from `auth/server.rs:354-389`: genuine validation failures are `ServerFnError::new("Invalid or expired token")`, while the DB lookup and the attempt-bump `UPDATE` use `internal(...)` -> logged + `"Internal server error"`. So the cause **is** already logged; what is wrong is that a DB outage is reported to the user as a wrong code, and inbound.rs's `Internal` handling never fires. Task 9 routes it through the same classifier while **preserving the email-specific wording** `"Invalid or expired confirmation code."` for the genuine-failure case (auth's own wording says "token", which is wrong for this flow). |
| **wfe F26** | Inline SQL in commands.rs instead of db helpers; drift risk with the settings path | **CONFIRMED, and MOSTLY ALREADY SOLVED IN db.rs — scope reduced accordingly.** Re-derived: db.rs **already has** `get_user_email_prefs` (:2836-2844) with SQL byte-identical to `commands.rs:827-834`, and `set_user_turn_emails_enabled`/`set_user_invite_emails_enabled`/`set_user_reminder_emails_enabled` (:2847-2886) byte-identical to `commands.rs:853`/`:866`/`:879`, **with tests already covering them** (`email_prefs_default_all_true` db.rs:6821-6836 and `set_email_prefs_toggles` db.rs:6838-6876). So four of the six sites need **no new db.rs code at all** — just deleting the local copies and calling the existing helpers, which is precisely the drift the finding warns about and removes 3 dead `updated_at = NOW()` clauses from commands.rs as a side effect. Site :732 is dissolved by Task 10 (it becomes `list_user_emails`). Only two genuinely new helpers are needed: `delete_login_confirmation` (for :751) and `find_game_version_id_for_game` (for :1149). Both are plain queries, so no `.sqlx` churn. **Neither duplicates anything WP-41 touches.** |
| **wfe F27** | Self-mention in `new` opponents silently dropped | **CONFIRMED.** `commands.rs:382-384`: `if id == ctx.user_id { continue; }`. `new chess me myuser` silently builds a 2-player roster from 3 named slots, and the resulting `roster_error` message counts differently from what the user typed. Task 12 returns a user error instead. No existing test asserts the silent skip (checked the whole `mod tests`). |
| **wfe F28** | `bump` reply does not mention the digest cap | **CONFIRMED; recommendation PARTIALLY OVERTURNED.** `bump_reply` (:449-475) calls `find_active_turn_games(pool, user_id, SWITCH_DIGEST_CAP)` at :455 — the cap is a SQL `LIMIT` (db.rs:3113), so the current code **cannot even tell** whether more were waiting; `cap_digest` at :458 is a second, redundant truncation. Task 13 fetches `SWITCH_DIGEST_CAP + 1` to detect the overflow. The finding's suggested wording — "reply bump again for the rest" — is **rejected as factually wrong**: `find_active_turn_games` has a deterministic `ORDER BY gp.is_turn_at ASC NULLS LAST` with no offset or cursor, so a second `bump` re-sends **the same** games. The message says the truth instead. |
| **wfe F29** | Game-scoped dispatch reserves verbs that could collide with game move grammars | **CONFIRMED, and the hedge is RESOLVED AGAINST the finding: a real collision exists TODAY.** The reserved set is 18 verbs (`dispatch_email_command`'s `match verb_lower.as_str()` at :1215-1246, `subscribe_toggle` :42-48, `settings_verb` :224-235). `rg -oin '"(concede\|end\|undo\|restart\|rules\|help\|commands\|new\|bump\|list\|subscribe\|unsubscribe\|name\|colors\|colours\|theme\|emails\|settings)"' rust/game/*/src/command.rs` returns **3 hits, all `"end"`**: `acquire-1/src/command.rs:192-197` (`Doc::name_desc("end", "trigger the end of the game at the end of your turn", Map::new(Token::new("end"), ...))`) and `starship-catan-1/src/command.rs:309-313` (`Doc::name_desc("end", "end the flight early", Token::new("end"))`). Both are legal, player-issuable moves that email players **cannot** make: `dispatch_email_command`'s `"end"` arm (**:1217** - `"end" => return run_end(ctx).await,`) intercepts first. The `"end"` arm is **post-snapshot** (added by #47), which is why the finding could truthfully say "no current game is known to collide". Task 14 is **CARVED OUT to WP-85** (D-54): D-15 was answered the other way — game parser FIRST, platform commands FALLBACK, plus a small hard-reserved escape-hatch set — **not** the documented reservation an earlier draft assumed. `wfe F29` **stays counted in WP-59's scope** for coverage bookkeeping even though the work moved (same stance as WP-83). The collision remains escalated to the Lead in "Cross-package / newly discovered". |

Counts: **11 CONFIRMED**, **5 ADJUSTED** (F9, F14, F21, F23, F26 — F6, F24, F28
each additionally carry a rejected sub-recommendation), **0 fully OVERTURNED
findings**, **4 OVERTURNED / REJECTED recommendations** (F21's "emailed verbatim
and unlogged" premise, F23's join-on-code fix, F28's "bump again for the rest"
wording, F6's forgiving trailing noise), **0 SKIPPED-BY-DECISION**.

**Label correction (lead review, 2026-07-25):** an earlier draft of this spec
marked F6's "forgiving trailing noise" sub-recommendation
**SKIPPED-BY-DECISION**. That label is wrong and has been changed to
**REJECTED-WITH-REASON**: `planning/decisions-needed.md` contains **no** entry
for it (verified — the only F6 hits in that file are a seven-wonders finding,
not wfe F6), so nothing was skipped by a *user* decision. The rejection is this
spec's own re-derived judgement and stands on its stated reasoning (silent
swallowing of typos vs `failure_report_header` already naming the failing line).
If the Lead wants it treated as a user-facing choice rather than an engineering
judgement, it must be raised as a new decision — it is **not** one today.

**Also note (amended 2026-07-26):** **WP-59 now has NO open user decision.**
D-15 is **ANSWERED** — see `planning/decisions-ANSWERED.md` (answered
2026-07-26). The ruling went the *opposite* way to what Task 14 implemented: on
game-scoped messages the **game command parser is tried FIRST and platform
commands are the FALLBACK**, with one small hard-reserved set of escape-hatch
verbs (`help` and equivalents) that always wins. The only task D-15 gated was
**Task 14**, which is **carved out of this package to WP-85** (see the Task 14
banner below). Nothing else in WP-59 depends on it.

---

## Task 1: extract the bare addr-spec before routing or matching (wfe F4, major)

**Problem (restated):** `data.from`, `data.to` and `data.received_for` are raw
JSON strings from the Resend webhook payload (`ResendInboundData`,
inbound.rs:166-176). Nothing normalises them. `parse_reply_address` (:38) takes
the local part with `addr.split('@').next()`, so a recipient rendered as
`"brdg.me <g-abc@brdg.me>"` produces the local part `"brdg.me <g-abc"`, matches
no prefix, and the game reply is misrouted to the settings handler.
`from_matches_verified_email` (:387) and `resolve_user_by_verified_from` (:400)
bind the raw From into `LOWER(email) = LOWER($1/$2)`, so
`"Alice <alice@x.com>"` never matches the stored `alice@x.com` and the reply is
dropped with an info log. Nearly every mail client sets a display name.

**Fix (re-derived):** one pure helper, `extract_addr_spec`, applied at exactly
two places — once to `event.data.from` in `resend_webhook`, once per candidate
recipient inside `select_route`. `mail_parser` 0.11.5 is already an `ssr`
dependency and already parses RFC 5322 address fields including quoted display
names, comments, encoded words and address lists; the helper reuses it by
handing it a one-header synthetic message rather than hand-rolling a grammar
(verified APIs: `MessageParser::default().parse(&[u8]) -> Option<Message>`;
`Message::from() -> Option<&Address>` at
`mail-parser-0.11.5/src/core/message.rs:156`; `Address::first() -> Option<&Addr>`
and `Addr::address() -> Option<&str>` at
`mail-parser-0.11.5/src/core/address.rs:11` for `Address::first`, `:145` for
`Addr::address`; `MessageParser::parse` is
`parse<'x>(&self, raw_message: &'x (impl AsRef<[u8]> + ?Sized)) -> Option<Message<'x>>`
at `mail-parser-0.11.5/src/parsers/message.rs:111`, so both `&str` (as
`extract_plain_text` does) and `&[u8]` are accepted).

A **fast path** handles the already-bare form without the parser. This is not
an optimisation — it is a regression guard: the bare form is the one that works
today, and it must keep working byte-for-byte regardless of any parser edge
case.

**Inputs the extractor must handle, and what it returns:**

| Input | Returns |
|---|---|
| `alice@example.com` | `Some("alice@example.com")` (fast path) |
| `  alice@example.com  ` | `Some("alice@example.com")` |
| `Alice <alice@example.com>` | `Some("alice@example.com")` |
| `"Doe, Alice" <alice@example.com>` | `Some("alice@example.com")` |
| `Alice (at home) <alice@example.com>` | `Some("alice@example.com")` |
| `=?utf-8?q?Alice?= <alice@example.com>` | `Some("alice@example.com")` |
| `a@x.com, b@y.com` | `Some("a@x.com")` — first address wins |
| `<alice@example.com>` | `Some("alice@example.com")` |
| `Alice` | `None` (no `@`) |
| `<>` | `None` |
| `""` | `None` |
| a value containing `\r` or `\n` | CR/LF replaced with a space **before** parsing, so it can never become a second header |

**Why the recipient is also switched to the extracted address:** `from` is the
recipient handed to `send_rendered_email` (:884, :962, :1039, :1084, :1214).
Replying to the address that was actually authenticated — rather than to a
client-supplied display-name string — keeps the reply target and the auth
subject identical. `resend-rs` posts JSON, so this is not a header-injection
fix; it is a "reply to what you verified" fix.

**Edge cases:**
- Do **not** lowercase the result. Both SQL comparisons already use
  `LOWER(...)` (:387, :400); lowercasing in Rust would be redundant and would
  corrupt the reply recipient's local part, which is case-sensitive per RFC.
- `select_route` falls back to the raw string when extraction returns `None`, so
  its three existing tests (:1408-1441) and `parse_reply_address`'s six
  (:1289-1326) keep passing untouched. `parse_reply_address` itself is **not**
  modified — its contract (`"hello"` -> `None`, any domain accepted) stays.
- **Do NOT change `from_matches_verified_email` or `resolve_user_by_verified_from`,
  and do NOT add any authentication logic here.** See Non-Goals / WP-56.
- The new `return StatusCode::OK` matches the surrounding convention for
  unroutable deliveries; do not make it a 4xx/5xx (WP-57 owns status codes).

**Files:**
- Modify: `rust/web/src/email/inbound.rs` (add `extract_addr_spec` after
  `extract_plain_text`; rewrite `select_route`; edit `resend_webhook`; add tests)

**Steps:**

- [ ] Insert `extract_addr_spec` immediately after `extract_plain_text`
      (after inbound.rs:56, before the `#[async_trait::async_trait]` at :58):

```rust
/// Extracts the bare addr-spec from one RFC 5322 address-field value.
///
/// The Resend inbound webhook hands us `data.from`, `data.to` and
/// `data.received_for` as raw JSON strings and does not document any
/// normalisation, so every client form has to be handled: `alice@example.com`,
/// `Alice <alice@example.com>`, `"Doe, Alice" <alice@example.com>`,
/// `Alice (at home) <alice@example.com>`, `=?utf-8?q?Alice?= <alice@x.com>`,
/// and multi-address values (the FIRST address wins). Returns `None` when no
/// address can be extracted.
///
/// wfe F4. This is a PARSING fix only: it changes what string is compared, not
/// what it is compared against. The From-authentication redesign is D-1/WP-56 -
/// do not add DKIM/SPF or token checks here.
pub fn extract_addr_spec(value: &str) -> Option<String> {
    // A CR/LF in a webhook-supplied value must never become a second header in
    // the synthetic message below.
    let sanitized = value.replace(['\r', '\n'], " ");
    let value = sanitized.trim();
    if value.is_empty() {
        return None;
    }

    // Fast path: an already-bare addr-spec. Not an optimisation - this is what
    // guarantees the pre-existing behaviour for the common case, independent of
    // any parser edge case.
    if !value.contains(['<', '>', '"', '(', ')', ',', ':', ';'])
        && !value.contains(char::is_whitespace)
        && value.matches('@').count() == 1
    {
        return Some(value.to_string());
    }

    // Everything else goes through `mail_parser` (already a dependency, used by
    // `extract_plain_text` above) rather than a hand-rolled RFC 5322 grammar.
    let raw = format!("From: {value}\r\n\r\n");
    let msg = mail_parser::MessageParser::default().parse(raw.as_bytes())?;
    let addr = msg.from()?.first()?.address()?.trim();
    if addr.is_empty() || !addr.contains('@') {
        return None;
    }
    Some(addr.to_string())
}
```

- [ ] Replace `select_route` **including its two-line doc comment — inbound.rs:176-182**
      (`/// First recipient address that parses to a routing token wins; ...` at
      :176-177, `pub fn select_route` at :178, closing `}` at :182) with:

```rust
/// First recipient address that parses to a routing token wins; `to` is checked
/// before `received_for`. Each candidate goes through `extract_addr_spec` first
/// so `"brdg.me <g-tok@brdg.me>"` routes the same as `"g-tok@brdg.me"`
/// (wfe F4); an unparseable value is still tried verbatim.
pub fn select_route(to: &[String], received_for: &[String]) -> Option<InboundRoute> {
    to.iter().chain(received_for.iter()).find_map(|addr| {
        let bare = extract_addr_spec(addr).unwrap_or_else(|| addr.to_string());
        parse_reply_address(&bare)
    })
}
```

- [ ] In `resend_webhook`, insert the From extraction between the
      `event.event_type` guard (whose closing `}` is inbound.rs:**475**) and the
      `match select_route(...)` at :**477**:

```rust
    // wfe F4: `data.from` is a raw header value; extract the addr-spec once
    // here so every route below compares (and replies to) a bare address.
    let Some(from) = extract_addr_spec(&event.data.from) else {
        tracing::warn!(
            "resend webhook: could not extract an address from the From value; no response"
        );
        return StatusCode::OK;
    };
```

- [ ] In the same `match select_route(...)` block (inbound.rs:**477-487**), replace
      all three `&event.data.from` arguments with `&from`. The block becomes:

```rust
    match select_route(&event.data.to, &event.data.received_for) {
        Some(InboundRoute::Game(token)) => {
            handle_game_reply(&state, &token, &from, &event.data.email_id).await;
        }
        Some(InboundRoute::Invite(token)) => {
            handle_invite_reply(&state, &token, &from, &event.data.email_id).await;
        }
        Some(InboundRoute::Settings(_)) | None => {
            handle_settings_reply_route(&state, &from, &event.data.email_id).await;
        }
    }
```

- [ ] Add the unit tests. Append inside `mod tests`, immediately after
      `select_route_routes_invite_and_settings` (which ends at inbound.rs:1441):

```rust
    #[test]
    fn extract_addr_spec_bare_address_is_unchanged() {
        assert_eq!(
            extract_addr_spec("alice@example.com").as_deref(),
            Some("alice@example.com")
        );
        assert_eq!(
            extract_addr_spec("  alice@example.com  ").as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn extract_addr_spec_display_name_forms() {
        for input in [
            "Alice <alice@example.com>",
            "\"Doe, Alice\" <alice@example.com>",
            "Alice (at home) <alice@example.com>",
            "=?utf-8?q?Alice?= <alice@example.com>",
            "<alice@example.com>",
        ] {
            assert_eq!(
                extract_addr_spec(input).as_deref(),
                Some("alice@example.com"),
                "input: {input}"
            );
        }
    }

    #[test]
    fn extract_addr_spec_first_of_several() {
        assert_eq!(
            extract_addr_spec("a@x.com, b@y.com").as_deref(),
            Some("a@x.com")
        );
    }

    #[test]
    fn extract_addr_spec_rejects_valueless_input() {
        assert_eq!(extract_addr_spec(""), None);
        assert_eq!(extract_addr_spec("   "), None);
        assert_eq!(extract_addr_spec("Alice"), None);
        assert_eq!(extract_addr_spec("<>"), None);
    }

    #[test]
    fn extract_addr_spec_strips_crlf_before_parsing() {
        // A newline must not be able to introduce a second header.
        assert_eq!(
            extract_addr_spec("Alice <alice@example.com>\r\nBcc: evil@x.com").as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn select_route_handles_display_name_recipients() {
        assert_eq!(
            select_route(&["brdg.me <g-abc@brdg.me>".to_string()], &[]),
            Some(InboundRoute::Game("abc".to_string()))
        );
        assert_eq!(
            select_route(&[], &["Invites <i-xyz@brdg.me>".to_string()]),
            Some(InboundRoute::Invite("xyz".to_string()))
        );
    }
```

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr email::inbound` — the 6 new tests PASS
      and all 9 pre-existing `parse_reply_commands_*` / 6 `parse_reply_address_*`
      / 3 `select_route_*` tests still PASS **unmodified**.
- [ ] If `extract_addr_spec_display_name_forms` fails on any row, the
      synthetic-message shape is the suspect — print `raw` and the parsed
      `msg.from()` before changing the fast path. Do **not** widen the fast
      path to "fix" it; the fast path must stay restricted to unambiguous bare
      addresses.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] `git diff --stat` shows **only** `rust/web/src/email/inbound.rs`.
- [ ] `rg -n "from_matches_verified_email|resolve_user_by_verified_from" rust/web/src/email/inbound.rs`
      shows the definitions and call sites **unchanged in body** — you only
      changed the argument passed in.

**Commit:**

- [ ] `git add rust/web/src/email/inbound.rs && git commit -m "fix(email): extract addr-spec from display-name From/To forms (wfe F4)"`

---

## Task 2: harden reply quote/attribution stripping (wfe F6, minor)

**Problem (restated):** `parse_reply_commands` (inbound.rs:8-28) stops only at a
single line that both starts with `"On "` and ends with `"wrote:"`, or at a
`--`/`-- ` signature. Real clients defeat both: Gmail wraps a long attribution
so `wrote:` lands on the following line (the attribution text then becomes a
"command"); Outlook top-posts an **unquoted** `-----Original Message-----` /
`From:` / `Sent:` / `To:` / `Subject:` block; non-English clients never write
`wrote:`. Because `run_game_reply_commands` stops at the first failure
(:318-344), the sender gets a "command failed" report naming text they never
typed.

**Fix (re-derived):** three additional stop conditions, ordered cheapest-first,
with the language-independent one doing most of the work:

1. **When the first `>`-quoted line is reached, retract the block of collected
   lines directly above it (the run since the last blank line) if any line in
   that block looks like an attribution.** "Looks like an attribution" =
   the trimmed line ends with `:` **or** it contains an angle-bracketed address
   (`<`...`@`...`>`). This is the language-independent rule: every mainstream
   client puts the attribution in a contiguous block directly above the first
   quoted line, separated from the sender's own text by a blank line, and every
   language's attribution either ends in `:` (`wrote:`, `a écrit :`, `schrieb:`)
   or carries the sender's address in angle brackets.

   **Why not the simpler "a line immediately followed by a `>`-quoted line is an
   attribution":** that was this spec's first draft and it is WRONG — it breaks
   the pre-existing `parse_reply_commands_strips_quoted_lines` test
   (inbound.rs:1226-1230), whose input is `"play d4\n> previous move was e4\n> another quote"`.
   There the command `play d4` is *itself* immediately followed by a quoted line
   with no attribution in between, so the naive rule returns `[]` instead of
   `["play d4"]`. The attribution-shape test is what distinguishes the two
   cases: `play d4` neither ends with `:` nor carries `<...@...>`.

   **Why retract a block rather than break on one line:** Gmail's wrapped
   attribution spans *two* lines (`On Wed, 22 Jul 2026 at 13:16, brdg.me <mail@brdg.me>`
   then `wrote:`). Only the second ends with `:`, and only the second is
   immediately above the quote. Retracting the whole block since the last blank
   line removes both.

   **Why `continue` (not `break`) is kept for quoted lines:** the pre-existing
   function skips `>` lines and keeps scanning, so a sender who types a command
   *below* a quoted block is still served. Changing that to `break` would be an
   unrequested behaviour change. The retraction therefore happens once, on the
   first quoted line only.
2. **`-----Original Message-----`** (case-insensitive prefix match on the
   trimmed line) — Outlook's unquoted separator.
3. **A trimmed line starting with `From:`, `Sent:`, `To:`, `Subject:`, `Cc:` or
   `Date:`** (case-insensitive) — the Outlook header block that follows, and
   also the bare form some clients emit without the dashed separator. Safe
   against game grammars: no game command grammar in the repo uses a colon
   (`rg -n 'Token::new\("[a-z]+:' rust/game/*/src/command.rs` finds nothing).

**Edge cases:**
- All 9 existing `parse_reply_commands_*` tests (inbound.rs:1221-1287) must pass
  unmodified. Re-derived line by line against the body below:
  - `_clean_single` (:1222): no quote, no blank -> `["play e4"]`. OK.
  - `_strips_quoted_lines` (:1227): quote at line 1, block since last blank is
    `["play d4"]`, `play d4` is not attribution-shaped -> **no retraction** ->
    `["play d4"]`. OK. **This is the test the naive lookahead rule broke.**
  - `_cuts_at_on_wrote` (:1233) and `_realistic_reply_body_strips_quote_block`
    (:1257): the pre-existing `On ... wrote:` rule fires and `break`s before any
    quoted line is seen, so the retraction never runs. OK.
  - `_cuts_at_signature` (:1239), `_multiple_in_order` (:1248),
    `_drops_blank_lines` (:1274), `_keeps_arguments` (:1280), `_empty_input`
    (:1285): no quoted line at all, so only the pre-existing rules apply. OK.
- The retraction needs to index into the accumulated `commands`, not the input,
  so no lookahead over `text.lines()` is required — but a `block_start` cursor
  is. Keep the signature
  `pub fn parse_reply_commands(text: &str) -> Vec<String>` — it is called from
  three places (:537, :644, :1127) and is the file's most-tested function.
- `block_start` can never exceed `commands.len()`: it is only ever assigned
  `commands.len()` (at a blank line, and after a truncation), and `commands`
  only grows otherwise. The slice `&commands[block_start..]` therefore cannot
  panic.
- **Colon safety, re-verified:** the `HEADER_PREFIXES` rule and the
  `ends_with(':')` attribution shape are both safe against game grammars.
  `rg -n 'Token::new\("[a-z]+:' rust/game/*/src/command.rs rust/game/*/src/lib.rs`
  returns **nothing** — no game grammar has a colon-terminated top-level token.
  The one colon in the email grammar, `bot:<name>` in `new`
  (`commands.rs:59`), has the colon mid-token, not at end of line, and is a
  standalone-path command that never reaches a quote block.
- An attribution block that is **not** separated from the sender's own text by a
  blank line will retract the sender's trailing command along with it (there is
  no way to tell them apart). Every mainstream client inserts that blank line.
  Record the limit in the doc comment; do not escalate the heuristic.
- An attribution that neither ends with `:` nor carries `<...@...>` is still not
  detected. Accepted limit; also recorded in the doc comment.

**Files:**
- Modify: `rust/web/src/email/inbound.rs` (`parse_reply_commands` + tests)

**Steps:**

- [ ] Replace `parse_reply_commands` (inbound.rs:8-28) in full with:

```rust
/// Splits an inbound reply body into command lines, dropping the quoted
/// original and everything after the attribution or signature.
///
/// Stop conditions (wfe F6):
/// 1. a single-line `On ... wrote:` attribution (the pre-existing rule) - stop;
/// 2. Outlook's unquoted `-----Original Message-----` separator - stop;
/// 3. an Outlook-style header line (`From:`, `Sent:`, `To:`, `Subject:`,
///    `Cc:`, `Date:`) - stop. Safe because no game grammar in the repo has a
///    colon-terminated top-level token;
/// 4. a `--` / `-- ` signature marker (the pre-existing rule) - stop;
/// 5. at the FIRST `>`-quoted line, retract the block of already-collected
///    lines since the last blank line if any of them looks like an attribution
///    (ends with `:`, or carries a `<...@...>` address). This is the
///    language-independent rule and it is what catches Gmail's two-line wrapped
///    attribution and localized clients that never write `wrote:`. Quoted lines
///    themselves are still skipped rather than terminating the scan, so a
///    command typed below a quote block still works, exactly as before.
///
/// Known limits: an attribution block that is NOT preceded by a blank line will
/// take the sender's last command with it (they are indistinguishable), and an
/// attribution that neither ends with `:` nor carries an address is not
/// detected.
pub fn parse_reply_commands(text: &str) -> Vec<String> {
    const HEADER_PREFIXES: [&str; 6] = ["from:", "sent:", "to:", "subject:", "cc:", "date:"];

    /// A collected line that is more likely a reply attribution than a command.
    fn looks_like_attribution(line: &str) -> bool {
        if line.ends_with(':') {
            return true;
        }
        match (line.find('<'), line.rfind('>')) {
            (Some(open), Some(close)) if close > open => line[open..close].contains('@'),
            _ => false,
        }
    }

    let mut commands: Vec<String> = Vec::new();
    // Index into `commands` where the current block starts (the run of
    // collected lines since the last blank input line).
    let mut block_start = 0usize;
    let mut retracted = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') {
            if !retracted {
                retracted = true;
                if commands[block_start..]
                    .iter()
                    .any(|c| looks_like_attribution(c))
                {
                    commands.truncate(block_start);
                }
                block_start = commands.len();
            }
            continue;
        }
        if trimmed.starts_with("On ") && trimmed.ends_with("wrote:") {
            break;
        }
        let t = line.trim();
        if t == "-- " || t == "--" {
            break;
        }
        if t.is_empty() {
            block_start = commands.len();
            continue;
        }
        let lower = t.to_ascii_lowercase();
        if lower.starts_with("-----original message") {
            break;
        }
        if HEADER_PREFIXES.iter().any(|p| lower.starts_with(p)) {
            break;
        }
        commands.push(t.to_string());
    }
    commands
}
```

- [ ] Add the new tests. Append inside `mod tests` immediately after
      `parse_reply_commands_empty_input` (ends at inbound.rs:1287):

```rust
    #[test]
    fn parse_reply_commands_cuts_at_wrapped_gmail_attribution() {
        // Gmail wraps a long attribution so `wrote:` lands on the next line.
        let input = "play e4\n\
                     \n\
                     On Wed, 22 Jul 2026 at 13:16, brdg.me <mail@brdg.me>\n\
                     wrote:\n\
                     > board\n\
                     > more board";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_localized_attribution() {
        let input = "play e4\n\
                     \n\
                     Le 22 juillet 2026 a 13:16, brdg.me a ecrit :\n\
                     > board";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_outlook_original_message_block() {
        let input = "play e4\n\
                     \n\
                     -----Original Message-----\n\
                     From: brdg.me <mail@brdg.me>\n\
                     Sent: Wednesday, 22 July 2026 13:16\n\
                     Subject: Your turn\n\
                     \n\
                     board text that is not quoted";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_bare_header_block() {
        // Some clients top-post the header block without the dashed separator.
        let input = "play e4\n\
                     \n\
                     From: brdg.me <mail@brdg.me>\n\
                     Sent: Wednesday, 22 July 2026 13:16\n\
                     board text";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_keeps_last_command_before_blank_then_quote() {
        // A blank line separates the sender's text from the quote block, so the
        // lookahead rule must not eat the final command.
        let input = "play e4\n\
                     done\n\
                     \n\
                     On Wed, 22 Jul 2026 at 13:16, brdg.me <mail@brdg.me> wrote:\n\
                     > board";
        assert_eq!(parse_reply_commands(input), vec!["play e4", "done"]);
    }

    #[test]
    fn parse_reply_commands_keeps_a_command_typed_below_a_quote_block() {
        // Regression guard for the `continue` (not `break`) semantics on quoted
        // lines, which this change deliberately preserves.
        let input = "> board\n\
                     > more board\n\
                     play e4";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_does_not_retract_a_command_directly_above_a_quote() {
        // The naive "line followed by a quote is an attribution" rule broke
        // this; `play d4` is not attribution-shaped, so it must survive. Same
        // shape as the pre-existing `_strips_quoted_lines` test, asserted here
        // as an explicit guard for the retraction rule.
        let input = "play d4\n> previous move was e4";
        assert_eq!(parse_reply_commands(input), vec!["play d4"]);
    }
```

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr parse_reply_commands` — 16 tests
      (9 pre-existing + 7 new) all PASS. **The 9 pre-existing tests are not
      edited.** If `parse_reply_commands_strips_quoted_lines` fails, the
      retraction rule is firing on a non-attribution block: check
      `looks_like_attribution`, do **not** adjust the old test.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/inbound.rs && git commit -m "fix(email): stop reply parsing at wrapped, localized and Outlook attributions (wfe F6)"`

---

## Task 3: honour the first-stated invite intent (wfe F15, nit)

**Problem (restated):** `handle_invite_reply` computes `accept` and `decline`
with two independent scans (inbound.rs:646-647) and resolves with
`if accept { "accepted" } else { "declined" }` (:722). A body containing both
words — "decline / actually no, accept" or the reverse — always accepts,
regardless of what the sender said first. Accepting an invite can start the
game (:762-777), so guessing wrong is not recoverable by email.

**Fix (re-derived):** extract a pure `parse_invite_intent(&[String]) ->
Option<InviteIntent>` that returns the **first** line matching either verb.
Matching stays exactly as today — whole-line, ASCII-case-insensitive — so
`"decline politely"` still matches nothing, `"ACCEPT"` still matches, and no
behaviour other than the tie-break changes. Extracting it is the point: the
current logic is inlined in a 230-line DB-driven handler and cannot be
unit-tested.

**Files:**
- Modify: `rust/web/src/email/inbound.rs`

**Steps:**

- [ ] Add the type and function immediately after `parse_reply_address`
      (after inbound.rs:51, before `extract_plain_text`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteIntent {
    Accept,
    Decline,
}

/// The sender's invite response: the FIRST command line that is exactly
/// `accept` or `decline` (ASCII-case-insensitive), so a body mentioning both
/// honours what was stated first instead of always accepting (wfe F15).
/// Matching is whole-line, as before: `decline politely` matches nothing.
pub fn parse_invite_intent(commands: &[String]) -> Option<InviteIntent> {
    commands.iter().find_map(|c| {
        if c.eq_ignore_ascii_case("accept") {
            Some(InviteIntent::Accept)
        } else if c.eq_ignore_ascii_case("decline") {
            Some(InviteIntent::Decline)
        } else {
            None
        }
    })
}
```

- [ ] Replace inbound.rs:646-647:

```rust
    let accept = commands.iter().any(|c| c.eq_ignore_ascii_case("accept"));
    let decline = commands.iter().any(|c| c.eq_ignore_ascii_case("decline"));
```

  with:

```rust
    let intent = parse_invite_intent(&commands);
```

- [ ] Replace the `if !accept && !decline {` guard at inbound.rs:**649** with
      `let Some(intent) = intent else {`, and change its closing `}` (at :**660**,
      immediately after the `return;` at :659) to `};`. The block becomes:

```rust
    let Some(intent) = intent else {
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            no_command_header_text(),
            None,
        )
        .await;
        return;
    };
    let accept = intent == InviteIntent::Accept;
```

  The trailing `let accept = ...` line keeps the remaining 70 lines of the
  function (`:722` the `response` string, `:733` the `if accept` game-start
  block, `:804` the `else if !accept` decline notification, `:809` the header
  `if accept`) working with **no further
  edits** — that is deliberate, so this task does not collide with anything.

- [ ] Confirm `decline` is now unused: `rg -n "\bdecline\b" rust/web/src/email/inbound.rs`
      must show no remaining local binding (`:722` reads
      `if accept { "accepted" } else { "declined" }`, a string literal, not the
      binding). If a `decline` binding remains, clippy will fail on the unused
      variable at the checkpoint.

- [ ] Add tests. Append inside `mod tests` after the `extract_addr_spec_*` tests
      added by Task 1:

```rust
    #[test]
    fn parse_invite_intent_first_verb_wins() {
        let decline_first: Vec<String> =
            vec!["decline".into(), "accept".into()];
        assert_eq!(
            parse_invite_intent(&decline_first),
            Some(InviteIntent::Decline)
        );
        let accept_first: Vec<String> = vec!["accept".into(), "decline".into()];
        assert_eq!(
            parse_invite_intent(&accept_first),
            Some(InviteIntent::Accept)
        );
    }

    #[test]
    fn parse_invite_intent_is_case_insensitive_and_whole_line() {
        assert_eq!(
            parse_invite_intent(&["ACCEPT".to_string()]),
            Some(InviteIntent::Accept)
        );
        assert_eq!(parse_invite_intent(&["decline politely".to_string()]), None);
        assert_eq!(parse_invite_intent(&[]), None);
    }
```

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr email::inbound` — new tests PASS,
      everything else unchanged.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean
      (this is what catches a leftover unused `decline`).
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/inbound.rs && git commit -m "fix(email): honour the first-stated accept/decline in invite replies (wfe F15)"`

---

## Task 4: delete the superseded command-loop scaffolding (wfe F8, minor)

**Problem (restated):** `CommandLoopOutcome` (inbound.rs:184-187),
`run_commands_in_order` (doc comment :189, fn :190-204) and `error_reply_text`
(:227-238) have no
production callers. The real loop is `run_game_reply_commands` (:301-349) with
its own `GameCommandLoopOutcome` (:278-296), and the real error mapping lives in
`dispatch_email_command` (`commands.rs:1275-1281`) plus
`run_game_reply_commands`' `Internal` arm (:332-342). Three `pub` items and four
tests are kept alive by nothing but each other.

**Proof of deadness (run this first, do not skip):**

- [ ] `rg -n "run_commands_in_order|CommandLoopOutcome|error_reply_text" /home/beefsack/Development/brdgme/rust`
      must return hits **only** in `rust/web/src/email/inbound.rs`, and within
      that file only at the definition sites (184, 190, 193, 200, 203, 227) and
      inside `mod tests` (1446, 1447, 1449, 1450, 1461, 1474, 1478, 1486, 1487,
      1489, 1490, 1646, 1649, 1652, 1654). **If any other file appears — a bin,
      `web/tests/`, another crate — STOP and report; the finding is wrong and
      the items must be kept.**

**Files:**
- Modify: `rust/web/src/email/inbound.rs`

**Steps:**

- [ ] Delete inbound.rs:184-205 — the `pub enum CommandLoopOutcome<E>` block,
      the `/// Runs `commands` in order...` doc comment, `run_commands_in_order`
      in full, and the blank line after it. Stop at the
      `pub fn confirmed_header_text` at :206; leave it.
- [ ] Delete inbound.rs:**227-239** — `pub fn error_reply_text` (body :227-238)
      plus the blank line at :239. The preceding item is
      `settings_response_header` (:217-225) and the following one is the
      `failure_report_header` **doc comment block, which starts at :240**
      (`/// Builds the header block for a command-failure report email. Layout (each a`)
      and runs to :251, with the `pub fn failure_report_header` signature at
      :252; **both stay.** **Do NOT delete :240 or :241** — an earlier draft of
      this spec said "227-241", which would have eaten the first two lines of
      that doc comment and left a `///` block attached to nothing (a compile
      error on the following item). Verify after deleting: the line now at 227
      must be the `/// Builds the header block...` line.
- [ ] Delete the three tests `run_commands_all_succeed` (inbound.rs:1443-1452),
      `run_commands_stops_at_first_error` (:1454-1481) and
      `run_commands_empty_list` (:1483-1492), including their `#[tokio::test]`
      attributes. The next test, `game_reply_loop_all_succeed_counts_moves`
      (:1494), stays.
- [ ] Delete the test `error_reply_text_maps_each_variant` (inbound.rs:1645-1658)
      including its `#[test]` attribute. The preceding
      `no_command_header_text_mentions_command` (:1640-1643) and the following
      `seed_game_with_player` helper comment block stay.

**Verification checkpoint:**

- [ ] `rg -n "run_commands_in_order|CommandLoopOutcome|error_reply_text" /home/beefsack/Development/brdgme/rust`
      returns **nothing**.
- [ ] `cargo test -p web --features ssr email::inbound` — compiles and every
      remaining test PASSES. Four tests fewer than before; that is expected and
      is the only sanctioned test deletion in this package besides Task 8's
      one-assertion edit.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] Line numbers shift by ~37 after this task. Later tasks locate by symbol
      name; re-grep before editing.

**Commit:**

- [ ] `git add rust/web/src/email/inbound.rs && git commit -m "refactor(email): delete superseded command-loop scaffolding (wfe F8)"`

---

## Task 5: one inbound-body fetch helper (wfe F9, minor)

**Problem (restated):** the same eighteen-line block — read `RESEND_API_KEY`,
reject empty, `tracing::error!`, build `ResendInbound { api_key, http:
state.http_client.clone() }`, `fetch_raw_email`, `tracing::error!` on failure —
appears verbatim three times: `handle_game_reply` (inbound.rs:518-535),
`handle_invite_reply` (:625-642) and `handle_settings_reply_route`
(:1088-1107). Two of the three then call `extract_plain_text(&raw)
.unwrap_or_default()` immediately (:536, :643) and the third passes `raw` on to
`handle_settings_reply`, which does the same extraction one frame later
(:1126).

**Fix (re-derived):** a single `async fn fetch_inbound_text(state, email_id)
-> Option<String>` that owns the env read, the client construction, the fetch,
both log lines and the text extraction. `handle_settings_reply` changes its
third parameter from `raw_body: &str` to `text: &str` — verified safe: its only
caller is `handle_settings_reply_route` (:1108), it is a private `async fn`, and
`rg -n "handle_settings_reply" rust/web` finds no test or external reference.

**Rejected alternative (do not do this):** putting the inbound source on
`AppState`. `AppState` is `ssr`-only so it *would* compile, but
`InboundEmailSource` is a trait, so the field would be
`Option<Arc<dyn InboundEmailSource>>`, forcing edits to `main.rs:85`,
`tests/ssr_pages.rs:45` and `tests/websocket_hygiene.rs:41` and putting a trait
object inside a `#[derive(Clone)]` struct — all to avoid one hashmap lookup per
inbound email. **Do not modify `src/state.rs`.**

**Files:**
- Modify: `rust/web/src/email/inbound.rs`

**Steps:**

- [ ] Add the helper immediately before `handle_game_reply` (i.e. after
      `resend_webhook`'s closing brace, which is at inbound.rs:**489** (:488 is
      the `StatusCode::OK` tail expression), allowing for Task 4's ~37-line
      upward shift):

```rust
/// Fetches an inbound email's raw MIME source from Resend and returns its
/// plain-text body, or `None` (already logged) if the key is unconfigured or
/// the fetch fails. The single place the inbound direction reads
/// `RESEND_API_KEY`; this block used to be duplicated verbatim in all three
/// routes (wfe F9).
async fn fetch_inbound_text(state: &AppState, email_id: &str) -> Option<String> {
    let api_key = match std::env::var("RESEND_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            tracing::error!("resend webhook: RESEND_API_KEY not configured; cannot fetch body");
            return None;
        }
    };
    let source = ResendInbound {
        api_key,
        http: state.http_client.clone(),
    };
    match source.fetch_raw_email(email_id).await {
        Ok(raw) => Some(extract_plain_text(&raw).unwrap_or_default()),
        Err(e) => {
            tracing::error!("resend webhook: failed to fetch raw email {email_id}: {e}");
            None
        }
    }
}
```

- [ ] In `handle_game_reply`, replace inbound.rs:**518-536** (from
      `let api_key = match std::env::var("RESEND_API_KEY") {` at :518 through
      `let text = extract_plain_text(&raw).unwrap_or_default();` at :536). Leave
      `let commands = parse_reply_commands(&text);` at :537 alone. Replacement:

```rust
    let Some(text) = fetch_inbound_text(state, email_id).await else {
        return;
    };
```

- [ ] In `handle_invite_reply`, replace inbound.rs:**625-643** (the same block:
      `let api_key` at :625 through `let text = extract_plain_text(&raw).unwrap_or_default();`
      at :643, leaving `let commands = parse_reply_commands(&text);` at :644)
      with the identical three lines.

- [ ] Replace `handle_settings_reply_route` (inbound.rs:1087-1109) in full with:

```rust
async fn handle_settings_reply_route(state: &AppState, from: &str, email_id: &str) {
    let Some(text) = fetch_inbound_text(state, email_id).await else {
        return;
    };
    handle_settings_reply(state, from, &text).await;
}
```

- [ ] Change `handle_settings_reply`'s signature (inbound.rs:1111) from
      `async fn handle_settings_reply(state: &AppState, from: &str, raw_body: &str)`
      to
      `async fn handle_settings_reply(state: &AppState, from: &str, text: &str)`,
      and delete its now-redundant extraction line (:1126,
      `let text = extract_plain_text(raw_body).unwrap_or_default();`). The next
      line, `let commands = parse_reply_commands(&text);`, becomes
      `let commands = parse_reply_commands(text);` (`text` is already a `&str`).

**Verification checkpoint:**

- [ ] `rg -n "RESEND_API_KEY" rust/web/src/email/inbound.rs` returns exactly
      **one** hit, inside `fetch_inbound_text`.
- [ ] `rg -n "ResendInbound \{" rust/web/src/email/inbound.rs` returns exactly
      **one** hit, inside `fetch_inbound_text`. (`StaticInbound` at :103 and the
      `impl` at :79 are untouched.)
- [ ] `rg -n "extract_plain_text" rust/web/src/email/inbound.rs` shows the
      definition, the one call in `fetch_inbound_text`, and only the two
      existing tests.
- [ ] `cargo test -p web --features ssr email::inbound` — all PASS.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `git diff rust/web/src/state.rs` is **empty**.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/inbound.rs && git commit -m "refactor(email): single inbound body fetch helper (wfe F9)"`

---

## Task 6: release the proposal lock before the early-exit invite emails, and log the missing-roster return (wfe F7 minor, wfe F12 nit)

**Problem (restated):** in `handle_invite_reply`, `tx` begins at inbound.rs:663
and `lock_proposal_for_update` takes a `FOR UPDATE` row lock on the proposal at
:671. Three paths then return with `tx` — and therefore the lock — still alive:

| Path | Live lines | What runs while holding the lock |
|---|---|---|
| proposal no longer open | :683-694 | `send_invite_reply_response`: `get_user_theme`, `find_proposal`, `find_game_version`, `find_game_type_name`, MJML render, **one HTTPS POST to Resend** |
| player row missing from the roster | :704-707 | nothing — but the lock is still held until the frame unwinds, and **no log line is emitted at all** (wfe F12) |
| already responded | :709-720 | same as the first row |

Every other responder to the same proposal blocks behind a Resend round-trip.

**Fix (re-derived):** roll back explicitly in each of the three branches before
doing anything else. **No idempotency trade-off exists**: the first write in the
function is `update_proposal_player_response` at :723-724, *after* all three
branches, so there is nothing to commit and no "committed but not emailed"
window is created. (The path that *does* write already commits at :780 before
its send at :819 — leave that alone.) `me` borrows from `players`, an owned
`Vec` returned by `find_proposal_players_tx`, not from `tx`, so consuming `tx`
in a branch that has already read `me.response` compiles.

**Files:**
- Modify: `rust/web/src/email/inbound.rs`

**Steps:**

- [ ] Add this private helper immediately after `fetch_inbound_text` (Task 5):

```rust
/// Releases the proposal `FOR UPDATE` lock on an invite early-exit path that
/// has written nothing, so the outbound response email is not sent while
/// holding it (wfe F7). Rollback, not commit: no path that calls this has
/// written anything.
async fn rollback_invite_tx(tx: sqlx::Transaction<'_, sqlx::Postgres>, context: &str) {
    if let Err(e) = tx.rollback().await {
        tracing::warn!("resend webhook: invite rollback failed ({context}): {e}");
    }
}
```

- [ ] In the `proposal.status != "open"` branch (inbound.rs:683-694), insert the
      rollback as the first statement of the block, before
      `send_invite_reply_response`:

```rust
    if proposal.status != "open" {
        rollback_invite_tx(tx, "invite no longer open").await;
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            "This invite is no longer open.".to_string(),
            None,
        )
        .await;
        return;
    }
```

- [ ] Replace the missing-roster match (inbound.rs:704-707) with a logged,
      lock-releasing version:

```rust
    let me = match players.iter().find(|p| p.id == player.id) {
        Some(p) => p,
        None => {
            tracing::warn!(
                "resend webhook: invite token's player {} is not in proposal {proposal_id}'s roster; no response",
                player.id
            );
            rollback_invite_tx(tx, "invite player not in roster").await;
            return;
        }
    };
```

  **Borrow note:** `me` is bound from `players` on the line that consumes `tx`
  in the `None` arm. This compiles because `players` is an owned `Vec` and `tx`
  is only borrowed by the earlier `find_proposal_players_tx(&mut tx, ...)` call,
  which has already returned. If the borrow checker objects, move
  `rollback_invite_tx(tx, ...).await;` above the `tracing::warn!` — do **not**
  clone `players`.

- [ ] In the `me.response != "pending"` branch (inbound.rs:709-720), insert the
      rollback as the first statement. `me.response` is read in the `if`
      condition, before `tx` is consumed inside the body:

```rust
    if me.response != "pending" {
        rollback_invite_tx(tx, "invite already responded").await;
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            "That invite has already been responded to.".to_string(),
            None,
        )
        .await;
        return;
    }
```

- [ ] Check that no *other* `return` between inbound.rs:670 and :780 sends an
      email while holding `tx`:
      `rg -n "send_invite_reply_response" rust/web/src/email/inbound.rs`
      must show exactly four call sites — **`:650`** (before `tx` exists, inside
      the no-command guard at :649-660), **`:684`** and **`:710`** (the two that
      now roll back first) and **`:819`** (after the commit at :780). Note the
      missing-roster branch at :704-707 does **not** send an email; it only
      returns, which is why wfe F12's fix is a `warn` and not a reply.
      Every other early return in that range only
      logs (`:674` proposal-not-found, `:678` lock-failed, `:699`
      players-lookup, `:726` update-response, `:738` count-pending,
      `:747`/`:751` game-version, `:758` roster, `:773` start-proposal). Those may keep relying on the
      implicit drop-rollback — **do not** convert them; that would be churn
      without benefit and would widen the diff WP-40/WP-57 have to rebase.

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr email::inbound` — all PASS (there is no
      unit test for this path today; it is a DB+HTTP handler and the change is
      lock-lifetime only — see the note below).
- [ ] **Manual proof, by reading:** in each of the three branches,
      `rollback_invite_tx(tx, ...)` textually precedes any `.await` on
      `send_invite_reply_response`, and `tx` is not referenced afterwards in
      that branch. The compiler enforces the second half (use-after-move).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] **Test-coverage note for the reviewer:** no automated test is added here.
      `handle_invite_reply` needs an `AppState` (pool + NATS jetstream +
      broadcaster + Resend) and asserting "the lock was released before the
      send" requires a second connection racing the first. That harness does
      not exist in `rust/web/tests/` and building it is disproportionate to a
      minor lock-hold fix. The wfe F12 half **is** observable — the `warn` — but
      only through log capture, which this crate does not do in tests. State
      this explicitly in the commit body rather than writing a test that
      asserts something else.

**Commit:**

- [ ] `git add rust/web/src/email/inbound.rs && git commit -m "fix(email): release proposal lock before invite early-exit sends, log missing roster (wfe F7, wfe F12)"`

---

## Task 7: stop the `" invite"` subject and log its causes (wfe F13, nit)

**Problem (restated):** `send_invite_reply_response` (inbound.rs:822-885) folds, at :841-857,
three separate lookups — `find_proposal`, `find_game_version`,
`find_game_type_name` — into `(String::new(), None)` via
`.unwrap_or(None).unwrap_or_default()` and a catch-all `_ =>` arm. Every failure
is swallowed unlogged, and `subject: format!("{game_type_name} invite")` (:867)
then produces the literal `" invite"` — with a leading space — which is what the
invitee sees in their inbox.

**Fix (re-derived):** replace the folded expression with an explicit,
per-failure-logged version, and make the subject fall back to
`"Your brdg.me invite"` when the game type name is unknown. `game_version_id`
is used at :870 for `rules_url`, so it must still be `Option`-shaped
independently of the name.

**Files:**
- Modify: `rust/web/src/email/inbound.rs`

**Steps:**

- [ ] Replace inbound.rs:**841-857** (from `let (game_type_name, game_version_id) =` at :841
      through the closing `};`) with:

```rust
    // wfe F13: each of these three lookups used to be folded into an empty
    // name, producing the subject " invite" with no log line.
    let mut game_type_name: Option<String> = None;
    let mut game_version_id: Option<uuid::Uuid> = None;
    match crate::proposals::find_proposal(pool, proposal_id).await {
        Ok(Some(proposal)) => {
            game_version_id = Some(proposal.game_version_id);
            match crate::db::find_game_version(pool, proposal.game_version_id).await {
                Ok(Some(gv)) => {
                    match crate::proposals::find_game_type_name(pool, gv.game_type_id).await {
                        Ok(Some(name)) => game_type_name = Some(name),
                        Ok(None) => tracing::warn!(
                            "resend webhook: game type {} not found for invite subject",
                            gv.game_type_id
                        ),
                        Err(e) => tracing::error!(
                            "resend webhook: invite game type lookup failed: {e}"
                        ),
                    }
                }
                Ok(None) => tracing::warn!(
                    "resend webhook: game version {} not found for invite subject",
                    proposal.game_version_id
                ),
                Err(e) => {
                    tracing::error!("resend webhook: invite game version lookup failed: {e}")
                }
            }
        }
        Ok(None) => {
            tracing::warn!("resend webhook: proposal {proposal_id} not found for invite subject")
        }
        Err(e) => tracing::error!("resend webhook: invite proposal lookup failed: {e}"),
    }
```

- [ ] Replace the subject line (inbound.rs:**867**) —
      `subject: format!("{game_type_name} invite"),` — with:

```rust
        subject: match &game_type_name {
            Some(name) => format!("{name} invite"),
            None => "Your brdg.me invite".to_string(),
        },
```

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr email::inbound` — all PASS. The one
      `#[sqlx::test]` in this file that exercises a send path
      (`failure_report_is_dethreaded_and_sets_reply_to`) is on the *game* route,
      not the invite route, so it is unaffected.
- [ ] `rg -n 'unwrap_or\(None\)' rust/web/src/email/inbound.rs` returns
      **nothing**.
- [ ] `rg -n '\{game_type_name\} invite' rust/web/src/email/inbound.rs` returns
      **nothing** (the `" invite"` degradation is gone by construction).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean —
      this also proves `game_version_id` is still consumed at the `rules_url`
      line.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/inbound.rs && git commit -m "fix(email): neutral invite subject fallback and logged lookup failures (wfe F13)"`

---

## Task 8: one reply-address domain and one builder per route (wfe F14, nit)

**Problem (restated):** the game reply address goes through
`crate::email::notify::reply_address` (`notify.rs:10-12`, used at
inbound.rs:960, :1037, :1082), but the invite address is built inline as
`format!("i-{}@brdg.me", ...)` at inbound.rs:882 and the settings address as
`format!("s-{user_id}@brdg.me")` at :1191. Three route prefixes, three
formatting sites, three copies of the domain.

**Fix (re-derived):** a `REPLY_DOMAIN` const plus `invite_reply_address` and
`settings_reply_address` next to the existing `reply_address`, all in
`notify.rs` (where `reply_address` already lives, and which `inbound.rs` already
imports through the fully-qualified path).

**Edge cases:**
- **`render.rs` is NOT touched.** Its `<mailto:unsubscribe@brdg.me?subject=unsubscribe>`
  (:237) is an unsubscribe target owned by WP-58/D-10, and its
  `<{thread_id}@brdg.me>` (:227) is a Message-Id domain owned by WP-60. Same for
  `inbound.rs:1065`'s `<game-{}@brdg.me>`. A reply address and a Message-Id
  domain being the same string today is a coincidence, not a shared concept;
  coupling them through one const would be wrong.
- `parse_reply_address` stays domain-agnostic (see the disposition table).
- One existing assertion changes: `notify.rs:500-503`'s
  `reply_address_formats_token` is extended rather than rewritten (its existing
  assertion is kept verbatim).
- The const is `pub` so the new builders and any future caller share it, but it
  is **not** wired to config — see Non-Goals.

**Files:**
- Modify: `rust/web/src/email/notify.rs` (const + 2 fns + test)
- Modify: `rust/web/src/email/inbound.rs` (2 call sites)

**Steps:**

- [ ] Replace notify.rs:8-12 (the `reply_address` doc comment and function)
      with:

```rust
/// The single inbound reply domain. Every reply address the app advertises
/// (`g-`/`i-`/`s-`) is on this domain (wfe F14). Deliberately NOT the
/// Message-Id domain (`render.rs`) or the unsubscribe mailto domain, which are
/// different concepts that merely happen to share the string today.
pub const REPLY_DOMAIN: &str = "brdg.me";

/// The per-player game reply address (`g-{token}@brdg.me`) the inbound webhook
/// routes on.
pub fn reply_address(token: &str) -> String {
    format!("g-{token}@{REPLY_DOMAIN}")
}

/// The per-invitee proposal reply address (`i-{token}@brdg.me`).
pub fn invite_reply_address(token: &str) -> String {
    format!("i-{token}@{REPLY_DOMAIN}")
}

/// The per-user settings reply address (`s-{user_id}@brdg.me`), used for
/// replies that carry no game or invite context.
pub fn settings_reply_address(user_id: uuid::Uuid) -> String {
    format!("s-{user_id}@{REPLY_DOMAIN}")
}
```

- [ ] Extend the existing test at notify.rs:500-503 (keep its assertion, add
      two):

```rust
    #[test]
    fn reply_address_formats_token() {
        assert_eq!(reply_address("tok"), "g-tok@brdg.me");
        assert_eq!(invite_reply_address("tok"), "i-tok@brdg.me");
        let user_id = uuid::Uuid::nil();
        assert_eq!(
            settings_reply_address(user_id),
            format!("s-{user_id}@brdg.me")
        );
    }
```

- [ ] In `send_invite_reply_response`, replace inbound.rs:882
      (`&format!("i-{}@brdg.me", player.email_token.as_deref().unwrap_or("")),`)
      with:

```rust
        &crate::email::notify::invite_reply_address(
            player.email_token.as_deref().unwrap_or(""),
        ),
```

- [ ] In `send_settings_response`, replace inbound.rs:1191
      (`let reply_address = format!("s-{user_id}@brdg.me");`) with:

```rust
    let reply_address = crate::email::notify::settings_reply_address(user_id);
```

**Verification checkpoint:**

- [ ] `rg -n '@brdg\.me' rust/web/src/email/notify.rs rust/web/src/email/inbound.rs`
      shows: the three builders' `{REPLY_DOMAIN}` interpolations (no literal),
      the extended test's three literal expectations, inbound.rs's
      `<game-{}@brdg.me>` Message-Id (**intentionally left**), the
      `mailto:unsubscribe@brdg.me` header (**intentionally left, WP-58**), and
      the existing `parse_reply_address`/`select_route` test fixtures. **No
      remaining `format!("i-...` or `format!("s-...` reply-address literal.**
- [ ] `git diff rust/web/src/email/render.rs` is **empty**.
- [ ] `cargo test -p web --features ssr email::notify` and
      `cargo test -p web --features ssr email::inbound` — all PASS.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/notify.rs rust/web/src/email/inbound.rs && git commit -m "refactor(email): single reply domain and per-route address builders (wfe F14)"`

---

## Task 9: classify `ServerFnError`s instead of emailing framework noise (wfe F21 major, wfe F24 minor)

**Problem (restated), re-derived error-by-error.** `run_restart`
(`commands.rs:1052-1141`) ends its `restart_core` call with
`.map_err(|e| CommandError::User(e.to_string()))` at **:1113**. Every error
`restart_core` can produce, classified:

| `restart_core` error site (`game/server_fns.rs`) | Producer | True class | What the email says today |
|---|---|---|---|
| :1000 find player counts | `internal(...)` | **Internal** | `error running server function: Internal server error` |
| :1001 `"Game type not found"` | `ServerFnError::new` | User | `error running server function: Game type not found` |
| :1002-1003 `roster_error(...)` msg | `ServerFnError::new` | User | `error running server function: This game supports 2, 3, 4 players, but the request has 5 (including you)` |
| :1009 begin transaction | `internal(...)` | **Internal** | as above |
| :1016 lock game | `internal(...)` | **Internal** | as above |
| :1018 `"Game not found"` | `ServerFnError::new` | User | prefixed |
| :1021 `"Game is not finished"` | `ServerFnError::new` | User | prefixed |
| :1031 find open restart proposal | `internal(...)` | **Internal** | as above |
| :1042 check invite policy | `internal(...)` | **Internal** | as above |
| :1044 policy violation msg | `ServerFnError::new` | User | prefixed |
| :1050 `find_or_create_user_by_email_tx` via bare `?` | `internal(...)` inside (`proposals.rs:896`, `:904`, `:911`, `:919`) | **Internal** | as above — but **unreachable from email**: the email path passes `opponent_emails = &[]` (commands.rs:1109) |
| :1059-1061 `"Please ensure each player in the game is unique"` | `ServerFnError::new` | User | prefixed |
| :1065 `create_game_from_service` via bare `?` | `internal(...)` at :631, :658, :662 plus `ServerFnError::new("Unexpected response from game service")` at :635 | **Internal** (the `new` one is an internal protocol failure worded as prose) | as above |
| :1085, :1089, :1100, :1114, :1130, :1146, :1152 | `internal(...)` | **Internal** | as above |

So: infrastructure failures are **already redacted** to the fixed string
`"Internal server error"` and **already logged** by
`crate::error::internal` (`src/error.rs:6-12`) — the finding's "emailed verbatim
and unlogged" is wrong on both counts. The two real defects are (a)
mis-classification, so inbound.rs's `Internal` branch (:332-342) never
substitutes the generic apology and the sender is told "Internal server error"
as though it were a refusal, and (b) **every** message — user refusals included
— reaches the sender wrapped in `ServerFnError`'s `Display` prefix
`"error running server function: "` (`server_fn-0.8.13/src/error.rs:233-234`).

The identical shape applies to `run_emails_confirm`'s
`.map_err(|_| CommandError::User("Invalid or expired confirmation code."))` at
**:747**: `validate_confirmation_code` (`auth/server.rs:354-389`) returns
`ServerFnError::new("Invalid or expired token")` for the three genuine
validation failures (expired, the `return Err(invalid())` at :372; attempt cap,
:376; code mismatch, :386) and `internal(...)` for the two DB failures (the
confirmation lookup's `.map_err` at :368 and the attempt-bump `UPDATE`'s at :385).

**Fix (re-derived):** one small classifier in `commands.rs`, plus a named const
in `error.rs` so the coupling to the redaction string is explicit rather than a
magic literal.

**Rejected alternative (do not do this):** giving `restart_core` a typed error
enum. It is the correct long-term shape, but it changes a `pub(crate)` signature
consumed by `restart_game_with_roster` (`game/server_fns.rs:1211`) and three
tests (:1547, :1962, :2034), inside the file **WP-40/D-3 is about to
restructure**. A one-`map_err` fix here has zero collision surface. Record the
typed-error idea in the commit body as a follow-up, do not implement it.

**Files:**
- Modify: `rust/web/src/error.rs` (add one const, use it in `internal`)
- Modify: `rust/web/src/email/commands.rs` (classifier + 2 `map_err` sites + tests)

**Steps:**

- [ ] In `rust/web/src/error.rs`, add the const above `internal` and use it in
      the body. The file becomes:

```rust
use leptos::prelude::ServerFnError;

/// The single opaque message `internal` substitutes for an infrastructure
/// failure. Named so callers that need to tell "a redacted internal failure"
/// apart from "a deliberate user-facing message" can compare against it
/// instead of a magic literal (see `email::commands::classify_server_fn_error`).
pub const INTERNAL_ERROR_MESSAGE: &str = "Internal server error";

/// For `.map_err(...)` on infrastructure failures inside server functions:
/// logs the real error server-side and replaces it with an opaque message,
/// so database/service internals never reach the client.
#[cfg(feature = "ssr")]
pub fn internal<E: std::fmt::Display>(context: &'static str) -> impl FnOnce(E) -> ServerFnError {
    move |e| {
        tracing::error!("{}: {}", context, e);
        ServerFnError::new(INTERNAL_ERROR_MESSAGE)
    }
}

pub fn user_facing_server_error(_e: &ServerFnError) -> String {
    "Something went wrong, please try again".to_string()
}
```

  **Do not** add `#[cfg(feature = "ssr")]` to the const — `error.rs` is not
  feature-gated as a whole (`lib.rs:31`) and `user_facing_server_error` is
  ungated too; gating the const would break a hydrate build if anything
  non-`ssr` ever reads it.

- [ ] In `commands.rs`, add the classifier immediately after the `CommandError`
      enum. **The enum's closing brace is at :25** (`#[derive]` :19,
      `pub enum CommandError {` :20, `User(String)` :21-22,
      `Internal(#[from] anyhow::Error)` :23-24, `}` :25) — insert **after :25**,
      before the `#[derive(...)] pub enum RulesFilter` block at :27. An earlier
      draft said "after :24", which would place the fn inside the enum body:

```rust
/// Splits a `ServerFnError` from a shared helper into this module's
/// user-vs-internal classification (wfe F21, wfe F24).
///
/// `crate::error::internal` is the only producer of
/// `INTERNAL_ERROR_MESSAGE`, and it has already logged the real cause, so that
/// exact message means "infrastructure failure": classify it `Internal` so the
/// caller substitutes the generic apology and inbound.rs logs the correlation.
/// Every other `ServerError` message is a deliberate user-facing string and is
/// returned as its BARE text - `ServerFnError`'s `Display` would otherwise
/// email it wrapped in "error running server function: ...".
fn classify_server_fn_error(
    context: &'static str,
    e: leptos::prelude::ServerFnError,
) -> CommandError {
    use leptos::prelude::ServerFnError;
    match e {
        ServerFnError::ServerError(msg) if msg == crate::error::INTERNAL_ERROR_MESSAGE => {
            CommandError::Internal(anyhow::anyhow!(
                "{context}: internal failure (cause already logged by crate::error::internal)"
            ))
        }
        ServerFnError::ServerError(msg) => CommandError::User(msg),
        other => CommandError::Internal(anyhow::anyhow!("{context}: {other}")),
    }
}
```

  **No `#[cfg(feature = "ssr")]` on this fn.** An earlier draft of this spec put
  one there. It is wrong for this file: `rg -c 'cfg\(feature = "ssr"\)' rust/web/src/email/commands.rs`
  returns **zero** — nothing in `commands.rs`, `inbound.rs` or `notify.rs`
  carries a per-item gate, because the whole `email` module is already gated at
  `lib.rs:35-36`. Adding one here would be the only such attribute in the file.

  **Note on the catch-all:** `ServerFnError`'s first variant
  (`WrappedServerError`) is `#[deprecated]`
  (`server_fn-0.8.13/src/error.rs:171-178`). Naming it in a pattern emits a
  deprecation warning, which clippy at `-D warnings` turns into an error — the
  `other =>` arm exists to avoid that. **Do not expand it into explicit
  variants.**

- [ ] Replace `commands.rs:1113` — `.map_err(|e| CommandError::User(e.to_string()))?;`
      — with:

```rust
    .map_err(|e| classify_server_fn_error("restart", e))?;
```

- [ ] Replace `commands.rs:745-747`:

```rust
    crate::auth::server::validate_confirmation_code(pool, &email, code)
        .await
        .map_err(|_| CommandError::User("Invalid or expired confirmation code.".to_string()))?;
```

  with:

```rust
    // wfe F24: only a genuine validation failure is a user error; a DB failure
    // must not be reported as a wrong code. The email flow keeps its own
    // wording rather than auth's "token" phrasing.
    if let Err(e) = crate::auth::server::validate_confirmation_code(pool, &email, code).await {
        return Err(match classify_server_fn_error("emails confirm: validate code", e) {
            CommandError::User(_) => {
                CommandError::User("Invalid or expired confirmation code.".to_string())
            }
            internal => internal,
        });
    }
```

  (If Task 10 lands first, this replaces the equivalent call inside Task 10's
  loop — see that task's step for the merged form.)

- [ ] Add unit tests. Append them inside `commands.rs`'s `mod tests`, at the end
      of the non-DB test group (before the first `#[sqlx::test]`; locate with
      `rg -n '#\[sqlx::test\]' rust/web/src/email/commands.rs | head -1`):

```rust
    #[test]
    fn classify_server_fn_error_redacted_internal_is_internal() {
        let e = leptos::prelude::ServerFnError::new(crate::error::INTERNAL_ERROR_MESSAGE);
        match classify_server_fn_error("ctx", e) {
            CommandError::Internal(_) => {}
            CommandError::User(m) => panic!("expected Internal, got User({m})"),
        }
    }

    #[test]
    fn classify_server_fn_error_user_message_is_bare() {
        let e = leptos::prelude::ServerFnError::new("Game is not finished");
        match classify_server_fn_error("ctx", e) {
            // Bare, NOT "error running server function: Game is not finished".
            CommandError::User(m) => assert_eq!(m, "Game is not finished"),
            CommandError::Internal(e) => panic!("expected User, got Internal({e})"),
        }
    }
```

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr classify_server_fn_error` — both new
      tests PASS. `classify_server_fn_error_user_message_is_bare` is the one
      that proves the `Display`-prefix leak is gone; if it fails asserting
      `"error running server function: Game is not finished"`, you matched on
      `e.to_string()` instead of destructuring the variant — fix the classifier,
      not the test.
- [ ] `cargo test -p web --features ssr emails_confirm` — the three existing
      `#[sqlx::test]`s (`emails_confirm_verifies_address`,
      `emails_confirm_no_unverified`, `emails_confirm_wrong_code`, live
      commands.rs:1825-1902) PASS **unmodified**. `emails_confirm_wrong_code`
      asserts the user error still `contains("Invalid or expired")`, which is
      exactly what the preserved wording guarantees.
- [ ] `rg -n 'CommandError::User\(e\.to_string\(\)\)' rust/web/src/email/commands.rs`
      returns **nothing**.
- [ ] `git diff rust/web/src/game/server_fns.rs` is **empty** — `restart_core`'s
      signature must be untouched.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean
      (this is what would catch a deprecation warning from naming
      `WrappedServerError`).
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/error.rs rust/web/src/email/commands.rs && git commit -m "fix(email): classify internal vs user errors from restart and code validation (wfe F21, wfe F24)"`

---

## Task 10: `emails confirm <code>` matches any pending address (wfe F23, minor)

**Problem (restated):** `run_emails_confirm` (`commands.rs:721-757`) picks the
single newest unverified address with inline SQL
(`ORDER BY created_at DESC LIMIT 1`, :732-737) and validates the code against
only that one. With two pending addresses, the older one's code always fails
`"Invalid or expired confirmation code."` and cannot be confirmed by email at
all unless the newer address is removed first.

**Fix (re-derived), and why not the finding's:** the finding suggests selecting
the address **by code** (a join on `login_confirmations`). That would regress
security: `validate_confirmation_code` bumps `login_confirmations.attempts` only
when it finds a row whose code mismatches (`auth/server.rs:378-387`), so a
selected-by-code lookup would make every wrong code a no-op and
`CONFIRM_MAX_ATTEMPTS_PER_CODE` would stop protecting the email path. Instead,
**iterate the user's unverified addresses and call
`validate_confirmation_code` on each**, keeping every existing window check,
attempt bump and cap intact. The inline SQL is replaced by the existing
`crate::db::list_user_emails` helper — which also discharges wfe F26's :732
site with no new db.rs code.

**Edge cases:**
- Iteration order is `list_user_emails`' order (`is_primary DESC, created_at
  ASC`; unverified rows are never primary, so effectively oldest-first). Order
  only decides which attempt counters get bumped and which address wins an
  (essentially impossible) code collision. Documented in the code.
- Side effect, accepted and documented: a wrong code bumps the attempt counter
  of **every** pending address it was tried against. With the normal case of one
  pending address this is identical to today's behaviour.
- The two distinct user messages must be preserved so the existing tests pass:
  "no pending address at all" -> `"No unverified address to confirm. Add one
  first with 'emails add <address>'."`; "pending addresses exist but no code
  matched" -> `"Invalid or expired confirmation code."`
- A real `Internal` from any attempt must abort immediately rather than being
  treated as "this address did not match" — otherwise a DB outage degrades into
  a wrong-code message, which is the wfe F24 defect reintroduced.

**Files:**
- Modify: `rust/web/src/email/commands.rs`

**Steps:**

- [ ] Replace `run_emails_confirm` (`commands.rs:721-757`) in full with:

```rust
/// `emails confirm <code>`: verifies `code` against EVERY pending address on
/// the account, not just the newest (wfe F23). Each candidate goes through
/// `validate_confirmation_code`, which keeps the 1-hour window and the
/// per-code attempt cap in force - selecting the address *by* code would
/// bypass the cap entirely. Side effect, accepted: a wrong code bumps the
/// attempt counter of each pending address it was tried against.
async fn run_emails_confirm(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    code: &str,
) -> Result<CommandReply, CommandError> {
    let code = code.trim();
    if code.is_empty() {
        return Err(CommandError::User(
            "Usage: emails confirm <code>".to_string(),
        ));
    }

    let rows = crate::db::list_user_emails(pool, user_id)
        .await
        .map_err(CommandError::Internal)?;
    let pending: Vec<String> = rows
        .into_iter()
        .filter(|r| r.verified_at.is_none())
        .map(|r| r.email)
        .collect();
    if pending.is_empty() {
        return Err(CommandError::User(
            "No unverified address to confirm. Add one first with 'emails add <address>'."
                .to_string(),
        ));
    }

    let mut confirmed: Option<String> = None;
    for email in &pending {
        match crate::auth::server::validate_confirmation_code(pool, email, code).await {
            Ok(_) => {
                confirmed = Some(email.clone());
                break;
            }
            Err(e) => {
                // wfe F24: a DB failure must abort, not read as a wrong code.
                if let CommandError::Internal(e) =
                    classify_server_fn_error("emails confirm: validate code", e)
                {
                    return Err(CommandError::Internal(e));
                }
            }
        }
    }
    let Some(email) = confirmed else {
        return Err(CommandError::User(
            "Invalid or expired confirmation code.".to_string(),
        ));
    };

    crate::db::mark_email_verified(pool, user_id, &email)
        .await
        .map_err(CommandError::Internal)?;
    crate::db::delete_login_confirmation(pool, &email)
        .await
        .map_err(CommandError::Internal)?;
    Ok(CommandReply::Status(format!("Address {email} confirmed.")))
}
```

  **Ordering note:** this body calls `classify_server_fn_error` (Task 9) and
  `crate::db::delete_login_confirmation` (Task 11). Execute Task 9 before this
  task; for `delete_login_confirmation`, either execute Task 11's db.rs step
  first or land this task with the pre-existing inline
  `sqlx::query("DELETE FROM login_confirmations WHERE email = $1")` and let
  Task 11 swap it. **Recommended order: Task 9, then Task 11's db.rs additions,
  then Task 10.**

- [ ] Add a `#[sqlx::test]` covering the two-pending-addresses case. Insert it
      immediately after the existing `emails_confirm_verifies_address`
      (`commands.rs:1825-1863`):

```rust
    #[sqlx::test]
    async fn emails_confirm_matches_the_older_pending_address(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())")
            .bind(user_id)
            .bind("primary@example.com")
            .execute(&pool)
            .await
            .unwrap();
        // Older pending address, then a newer one. Pre-fix, only the newer
        // address's code could ever be confirmed.
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind("older@example.com")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind("newer@example.com")
            .execute(&pool)
            .await
            .unwrap();
        for (email, code) in [("older@example.com", "111111"), ("newer@example.com", "222222")] {
            sqlx::query("INSERT INTO login_confirmations (email, code, sent_count, last_sent_at) VALUES ($1, $2, 1, NOW())")
                .bind(email)
                .bind(code)
                .execute(&pool)
                .await
                .unwrap();
        }

        let reply =
            dispatch_settings_command_for_user(&pool, None, user_id, "emails confirm 111111")
                .await
                .unwrap()
                .unwrap();
        assert_eq!(status_msg(reply), "Address older@example.com confirmed.");

        let verified: Option<time::PrimitiveDateTime> = sqlx::query_scalar(
            "SELECT verified_at FROM user_emails WHERE user_id = $1 AND email = $2",
        )
        .bind(user_id)
        .bind("older@example.com")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(verified.is_some());

        // The newer address is untouched.
        let newer: Option<time::PrimitiveDateTime> = sqlx::query_scalar(
            "SELECT verified_at FROM user_emails WHERE user_id = $1 AND email = $2",
        )
        .bind(user_id)
        .bind("newer@example.com")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(newer.is_none());
    }
```

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr emails_confirm` (needs Postgres — use
      `/home/beefsack/Development/brdgme/scripts/rust-test.sh` if a local
      Postgres is not up). Four tests: the three existing ones PASS
      **unmodified** plus the new one. Expected outcomes:
      - `emails_confirm_verifies_address`: one pending address, correct code ->
        `"Address pending@example.com confirmed."`
      - `emails_confirm_no_unverified`: no pending address -> user error
        containing `"No unverified address"`
      - `emails_confirm_wrong_code`: one pending address, code `999999` -> user
        error containing `"Invalid or expired"`
      - `emails_confirm_matches_the_older_pending_address`: two pending, older
        address's code -> older confirmed, newer still unverified
- [ ] `rg -n 'ORDER BY created_at DESC LIMIT 1' rust/web/src/email/commands.rs`
      returns **nothing**.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/commands.rs && git commit -m "fix(email): emails confirm matches any pending address (wfe F23)"`

---

## Task 11: route the remaining inline SQL through db.rs (wfe F26, minor)

**Problem (restated), enumerated site by site.** `commands.rs` delegates almost
everything to `crate::db`, but six places run SQL inline. Re-derived against
live db.rs, **four of the six already have a byte-identical db.rs helper** —
this is exactly the drift the finding warns about, already half-materialised:

| commands.rs site | Inline SQL | Action |
|---|---|---|
| **:732-737** (`run_emails_confirm`) | `SELECT email FROM user_emails WHERE user_id = $1 AND verified_at IS NULL ORDER BY created_at DESC LIMIT 1` | **Dissolved by Task 10** — replaced by `crate::db::list_user_emails`. Nothing to do here. |
| **:751-755** (`run_emails_confirm`) | `DELETE FROM login_confirmations WHERE email = $1` | **New helper** `db::delete_login_confirmation` |
| **:827-834** (`run_settings_summary`) | `SELECT turn_emails_enabled, invite_emails_enabled, reminder_emails_enabled FROM users WHERE id = $1` | Call the **existing** `db::get_user_email_prefs` (db.rs:2836-2844) — identical SQL |
| **:848-859** (`set_turn_emails_enabled`) | `UPDATE users SET turn_emails_enabled = $1, updated_at = NOW() WHERE id = $2` | Delete; call the **existing** `db::set_user_turn_emails_enabled` (db.rs:2847-2858) — identical SQL |
| **:861-872** (`set_invite_emails_enabled`) | same for `invite_emails_enabled` | Delete; call **existing** `db::set_user_invite_emails_enabled` (db.rs:2861-2872) |
| **:874-885** (`set_reminder_emails_enabled`) | same for `reminder_emails_enabled` | Delete; call **existing** `db::set_user_reminder_emails_enabled` (db.rs:2875-2886) |
| **:1148-1154** (`run_rules`) | `SELECT game_version_id FROM games WHERE id = $1` | **New helper** `db::find_game_version_id_for_game` |

So only **two** new db.rs helpers are needed, and neither duplicates or touches
anything **WP-41** edits (WP-41's db.rs functions are `send_friend_request`,
`update_game_command_success`, `friend_recent_visible_game`, `is_user_admin`,
`delete_expired_unverified_emails`, `choose_colors`, `apply_rating_changes`,
plus the `updated_at` sweep and its own test additions).

**Why:** deleting the three local setters removes the exact drift risk the
finding names — the web settings server fns toggle the same three columns
through the db.rs helpers, so a change to one path currently misses the other —
and it deletes three dead `updated_at = NOW()` clauses from commands.rs as a
side effect (the `users` table has the BEFORE UPDATE trigger; see WP-41 Task 1).

**Edge cases:**
- Both new helpers use **plain** `sqlx::query`/`query_scalar`, matching the
  `#22d` convention comment at db.rs:2888-2889. **No `sqlx::query!` macro** ->
  no `.sqlx` cache change -> no `cargo sqlx prepare`.
- `db.rs` returns `anyhow::Result`; `CommandError::Internal` has
  `#[from] anyhow::Error`, so `.map_err(CommandError::Internal)?` works at every
  new call site, matching the surrounding style.
- `find_game_version_id_for_game` has two more identical inline copies at
  `game/server_fns.rs:2333` and `:2375`. **Do not convert them** — that file is
  WP-40/WP-53 territory. Routed in "Cross-package / newly discovered".
- `auth/server.rs:486` and `:850` delete `login_confirmations` for an email with
  the `sqlx::query!` **macro**. **Do not convert them to the new helper** —
  `auth/` changes are mandatory-test territory under a different package, and
  touching macro SQL would force a `cargo sqlx prepare`.
- **db.rs changes must land with tests** (CODING.md). Two `#[sqlx::test]`s are
  specified below and are non-optional.

**Files:**
- Modify: `rust/web/src/db.rs` (2 helpers + 2 tests)
- Modify: `rust/web/src/email/commands.rs` (6 call sites, 3 function deletions)

**Steps:**

- [ ] In db.rs, add both helpers immediately after
      `delete_expired_unverified_emails` (`#[cfg]` :3123, fn :3124-3127, body
      :3128-3136, closing `}` :3137) and before the
      `#[cfg(all(test, feature = "ssr"))]` at :3139 (`mod tests {` at :3140).
      They belong inside the `// --- #22d multiple emails per account ---`
      section that starts at :2888, so its plain-query convention comment at
      :2888-2889 governs them.

      **WP-41 adjacency:** WP-41's Task 6 rewrites
      `delete_expired_unverified_emails`' body at :3128-3136 and leaves the
      closing `}` at :3137. That is the line immediately above this insertion
      point, so if WP-41 has not landed yet, expect git to report a
      context-overlap conflict here even though the two edits are disjoint.
      Resolve by keeping both.

```rust
/// Removes the outstanding login/confirmation code for `email` (the 22d
/// "confirm address" cleanup). No-op when there is none.
#[cfg(feature = "ssr")]
pub async fn delete_login_confirmation(pool: &PgPool, email: &str) -> Result<()> {
    sqlx::query("DELETE FROM login_confirmations WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await?;
    Ok(())
}

/// The game version a game was created from. `Ok(None)` when the game does not
/// exist.
#[cfg(feature = "ssr")]
pub async fn find_game_version_id_for_game(
    pool: &PgPool,
    game_id: Uuid,
) -> Result<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT game_version_id FROM games WHERE id = $1")
            .bind(game_id)
            .fetch_optional(pool)
            .await?,
    )
}
```

- [ ] Add the two mandatory tests at the **end** of db.rs's `mod tests` (locate
      the closing brace of the module — it is the last line of the file). If
      **WP-41** has already landed, append after its additions; the tests are
      self-contained and order-independent.

```rust
    #[sqlx::test]
    async fn delete_login_confirmation_removes_only_that_email(pool: sqlx::PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO login_confirmations (email, code, sent_count, last_sent_at) VALUES ($1, $2, 1, NOW())")
            .bind("a@example.com")
            .bind("111111")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO login_confirmations (email, code, sent_count, last_sent_at) VALUES ($1, $2, 1, NOW())")
            .bind("b@example.com")
            .bind("222222")
            .execute(&pool)
            .await?;

        delete_login_confirmation(&pool, "a@example.com").await.unwrap();

        let remaining: Vec<String> =
            sqlx::query_scalar("SELECT email FROM login_confirmations ORDER BY email")
                .fetch_all(&pool)
                .await?;
        assert_eq!(remaining, vec!["b@example.com".to_string()]);

        // Deleting a non-existent row is a no-op, not an error.
        delete_login_confirmation(&pool, "missing@example.com")
            .await
            .unwrap();
        Ok(())
    }

    #[sqlx::test]
    async fn find_game_version_id_for_game_returns_none_for_unknown_game(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        assert_eq!(
            find_game_version_id_for_game(&pool, Uuid::new_v4())
                .await
                .unwrap(),
            None
        );
        Ok(())
    }
```

  **Game seeding in db.rs's test module — resolved, not conditional.** There is
  **no** `seed_game` helper, but the test module *does* already seed games
  through `create_game_with_users` + `CreateGameOpts` at three places:
  **db.rs:3343**, **:4642** and **:6407** (`rg -n "CreateGameOpts" rust/web/src/db.rs`
  — the struct itself is defined at :824). **Extend the second test to assert the
  positive case as well**, by copying the nearest of those three
  `CreateGameOpts { ... }` literals verbatim (field-for-field; the struct has 12
  fields and omitting one will not compile) and asserting
  `find_game_version_id_for_game(&pool, game.id)` returns the
  `game_version_id` it was created with. Do **not** hand-roll a new seeding
  fixture, and do **not** change `CreateGameOpts`.

- [ ] In `commands.rs`, replace the `run_settings_summary` prefs fetch
      (**:827-834**, the `let (emails_enabled, invite_emails_enabled, reminder_emails_enabled): (bool, bool, bool) =`
      statement through its `.map_err(...)?;`) with the existing helper:

```rust
    let (emails_enabled, invite_emails_enabled, reminder_emails_enabled) =
        crate::db::get_user_email_prefs(pool, user_id)
            .await
            .map_err(CommandError::Internal)?;
```

- [ ] Delete `set_turn_emails_enabled`, `set_invite_emails_enabled` and
      `set_reminder_emails_enabled` from `commands.rs` in full (:848-885, the
      three `async fn`s and the blank lines between them). Then update **all six**
      call sites to the db.rs helpers — **four in production code:**
      - `run_emails_toggle` (:637): `set_turn_emails_enabled(pool, user_id, enabled)` -> `crate::db::set_user_turn_emails_enabled(pool, user_id, enabled)`
      - `run_emails_invite_toggle` (:653): -> `crate::db::set_user_invite_emails_enabled(pool, user_id, enabled)`
      - `run_emails_reminder_toggle` (:669): -> `crate::db::set_user_reminder_emails_enabled(pool, user_id, enabled)`
      - `dispatch_email_command`'s `subscribe_toggle` branch (**:1249**, inside the
        `if let Some(enabled) = subscribe_toggle(&verb_lower)` at :1248):
        `set_turn_emails_enabled(ctx.pool, ctx.user_id, enabled)` -> `crate::db::set_user_turn_emails_enabled(ctx.pool, ctx.user_id, enabled)`

      **and two inside `mod tests`, which an earlier draft of this spec missed —
      without these the crate does not compile:**
      - `subscribe_unsubscribe_toggles_turn_emails` (test at :1327-1358),
        **:1338** `set_turn_emails_enabled(&pool, user_id, false)` and
        **:1349** `set_turn_emails_enabled(&pool, user_id, true).await.unwrap();`
        -> `crate::db::set_user_turn_emails_enabled(...)` in both, arguments and
        `.await.unwrap()` unchanged.

      This is a **mechanical call-path rename inside a test, not a change of what
      the test asserts** — the two `SELECT turn_emails_enabled` assertions at
      :1342 and :1351 are untouched, and the SQL executed is byte-identical
      (db.rs:2853 vs the deleted commands.rs:853). It is therefore the **third**
      sanctioned test edit in this package, alongside Task 4's four deletions and
      Task 8's one extended assertion; update the Global Constraints note if you
      are tracking them.

      Each production call keeps its existing `.await.map_err(CommandError::Internal)?`.
      Confirm you found them all:
      `rg -n "set_(turn|invite|reminder)_emails_enabled" rust/web/src/email/commands.rs`
      must afterwards show **only** `crate::db::set_user_*` calls, and
      `rg -n "set_(turn|invite|reminder)_emails_enabled" rust/web/src` must show
      no definition outside db.rs.

- [ ] In `run_rules`, replace the game-version fetch (:1148-1154) with:

```rust
    let version_id = crate::db::find_game_version_id_for_game(ctx.pool, ctx.game_id)
        .await
        .map_err(CommandError::Internal)?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;
```

- [ ] Replace the login_confirmations DELETE in `run_emails_confirm` (:751-755)
      with `crate::db::delete_login_confirmation(pool, &email).await.map_err(CommandError::Internal)?;`
      — already written that way if Task 10 landed first.

**Verification checkpoint:**

- [ ] `rg -n "sqlx::query" rust/web/src/email/commands.rs` shows hits **only**
      inside `mod tests` (the first hit must be at a line above which no
      non-test code remains — check with
      `rg -n '#\[cfg\(all\(test, feature = "ssr"\)\)\]' rust/web/src/email/commands.rs`).
      **Zero inline SQL in production code in this file.**
- [ ] `git diff rust/web/.sqlx/` is **empty** (no macro SQL was touched).
- [ ] `cargo test -p web --features ssr db::` — the two new `#[sqlx::test]`s
      PASS plus every pre-existing db.rs test, including the email-prefs test at
      db.rs:6821-6836 and :6838-6876.
- [ ] `cargo test -p web --features ssr email::commands` — all PASS, in
      particular the existing settings/emails `#[sqlx::test]`s that assert the
      `turn_emails_enabled` column after a toggle (commands.rs:1342, :1351,
      :1526, :1538) — they now exercise the db.rs helper and must give the same
      answers.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] `git diff --stat` shows **only** `rust/web/src/db.rs` and
      `rust/web/src/email/commands.rs`. If `game/server_fns.rs` or
      `auth/server.rs` appears, you absorbed a Non-Goal — revert it.

**Commit:**

- [ ] `git add rust/web/src/db.rs rust/web/src/email/commands.rs && git commit -m "refactor(email): route commands.rs SQL through db.rs helpers (wfe F26)"`

---

## Task 12: reject a self-mention in `new` instead of dropping it (wfe F27, nit)

**Problem (restated):** `run_new_command`'s `OpponentToken::Human` arm
(`commands.rs:382-384`) does `if id == ctx.user_id { continue; }`. The sender is
always seated (`creator_id: ctx.user_id`, :413), so naming yourself is
meaningless — but silently dropping the slot means `new chess me myuser` builds a
2-player roster from three named slots, and the `roster_error` that may follow
(:398-400) counts differently from what the user typed.

**Fix:** return a user error naming the cause.

**Edge cases:**
- `check_duplicate_players(&human_ids)` (:391, defined :115-124) already rejects
  *other* duplicates with "Please ensure each player in the game is unique";
  the new message must be distinguishable from it so a user knows which mistake
  they made.
- Do **not** touch the `OpponentToken::Bot` arm (:367-376) or
  `classify_opponent` — WP-45/D-8 owns bot-slot validation.
- No existing test asserts the silent skip (whole `mod tests` checked), so
  nothing breaks.

**Files:**
- Modify: `rust/web/src/email/commands.rs`

**Steps:**

- [ ] Replace `commands.rs:382-384` (the `if` at :382, `continue;` at :383,
      closing `}` at :384):

```rust
                if id == ctx.user_id {
                    continue;
                }
```

  with:

```rust
                // wfe F27: silently dropping the slot builds a different roster
                // than the sender asked for and makes the player-count error
                // that may follow look wrong.
                if id == ctx.user_id {
                    return Err(CommandError::User(
                        "You are included in the game automatically; do not list yourself as an opponent."
                            .to_string(),
                    ));
                }
```

- [ ] Add a `#[sqlx::test]`. **The harness this needs already exists** — verified
      against live source, so none of what follows is conditional:
      - `make_standalone_ctx_deps()` (`commands.rs:2096-2105`) returns
        `(crate::websocket::GameBroadcaster, async_nats::jetstream::Context)`,
        connecting to `$NATS_URL` (default `nats://localhost:4222`). It is the
        helper `bump_verb_is_case_insensitive` (:2107-2134) already uses to build
        a `StandaloneCommandCtx` at :2112-2119. **Copy that construction shape
        verbatim.**
      - `seed_user(&pool, name)` (:1413-1420) inserts a `users` row with that
        exact `name`, which is what `crate::db::find_user_id_by_name` (called at
        :378) resolves against — so seeding the sender under a known name is
        enough to make `find_user_id_by_name` return `ctx.user_id` and reach the
        branch under test.
      - There is **no seeded real game type** in a `#[sqlx::test]` database, so
        the test must create one. `make_game_version(&pool)` (:2076-2094) does
        that but names the type `format!("Test Game {uuid}")` and returns only
        the version id, so it cannot be used to build the `new <type>` line.
        Insert a **single-word** game type inline instead, as below.
      - **Do NOT use `expect_user_err`** (:1429-1435): its parameter is
        `Option<Result<CommandReply, CommandError>>` (it is for
        `dispatch_settings_command_for_user`), while `run_new_command` returns a
        bare `Result`. Passing the `Result` to it does not compile. Match
        directly, as below.

      Insert immediately after `bump_verb_is_case_insensitive` (ends at
      `commands.rs:2134`) — locate with
      `rg -n "async fn bump_verb_is_case_insensitive" rust/web/src/email/commands.rs`:

```rust
    #[sqlx::test]
    async fn new_rejects_naming_yourself_as_an_opponent(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "self-namer").await;
        // A single-word game type name so the `new <type> <opponent>` line
        // splits the way `resolve_game_type`/`split_new_args` expect.
        let type_name = format!("selftest{}", uuid::Uuid::new_v4().simple());
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(&type_name)
        .bind(vec![2, 3, 4])
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) VALUES ($1, $2, $3, true, false)",
        )
        .bind(game_type_id)
        .bind("1.0.0")
        .bind("http://127.0.0.1:1")
        .execute(&pool)
        .await
        .unwrap();

        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let sctx = StandaloneCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            user_id,
        };

        // Naming yourself must be rejected, not silently dropped. The game
        // service at 127.0.0.1:1 is never reached: the error is returned while
        // resolving opponents, well before `create_game_from_service`.
        match run_new_command(&sctx, &format!("{type_name} self-namer")).await {
            Err(CommandError::User(msg)) => assert!(
                msg.contains("included in the game automatically"),
                "unexpected user error: {msg}"
            ),
            Err(CommandError::Internal(e)) => panic!("expected User error, got Internal: {e}"),
            Ok(_) => panic!("expected User error, got Ok"),
        }
    }
```

  **Why the unreachable game service is safe:** the self-mention check is at
  :382, inside the opponent loop; `create_game_from_service` is not called until
  :407. The `Err` returned at :382 short-circuits before any HTTP request, so the
  bogus `http://127.0.0.1:1` URI is never dialled. (`make_game_version` uses the
  same bogus URI for the same reason.)

  **If this test cannot connect to NATS**, that is the pre-existing condition
  documented in Global Constraints (backlog #40) and shared with the three
  `bump_*` tests — run it under
  `/home/beefsack/Development/brdgme/scripts/rust-test.sh`, which provisions
  NATS on 14222. Do **not** drop the test, do **not** invent a different
  fixture, and do **not** weaken the production code to make it testable.

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr email::commands` — all PASS (plus the
      new test, if the harness exists).
- [ ] `rg -n "if id == ctx.user_id" rust/web/src/email/commands.rs` shows the
      new `return Err(...)` form and no remaining `continue`.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/commands.rs && git commit -m "fix(email): reject self-mention in new opponents (wfe F27)"`

---

## Task 13: disclose the `bump` digest cap (wfe F28, nit)

**Problem (restated):** `bump_reply` (`commands.rs:449-475`) calls
`find_active_turn_games(pool, user_id, SWITCH_DIGEST_CAP)` at **:455**, where the
cap is a SQL `LIMIT` (db.rs:3113). The reply then says "Re-sent {n} games to your
active address." with no hint that games were left out — and because the LIMIT
is applied in SQL, the current code **cannot even detect** the overflow.
`cap_digest` at :458 is a second, redundant truncation of an already-limited
list.

**Fix (re-derived):** fetch `SWITCH_DIGEST_CAP + 1` rows, use the extra row
purely as an overflow flag, then let the existing `cap_digest` do the actual
truncation (so it stops being redundant and starts being the single enforcement
point). Append a sentence when capped.

**The finding's suggested wording is rejected.** It proposes "(capped at N;
reply bump again for the rest)". `find_active_turn_games` orders by
`gp.is_turn_at ASC NULLS LAST` (db.rs:3112) with no offset or cursor, and the `LIMIT $2` at db.rs:3113 is the only bound, so a
second `bump` re-sends **the same** first N games. Telling the user otherwise
would be a lie that generates support noise. The message states the cap and
points at the web UI instead.

**Edge cases:**
- The uncapped messages must not change: `emails_confirm`-style existing
  assertions include `assert_eq!(status_msg(reply), "Re-sent 2 games to your
  active address.")` in `bump_resends_only_my_turn_games` (commands.rs:2202) and
  `bump_sends_regardless_of_web_presence` (test at :2208-, its assertion inside). Only the capped branch adds text.
- `find_active_turn_games` takes `cap` as a parameter and has other callers
  (`rg -n "find_active_turn_games" rust/web/src`); **change only the argument
  passed from `bump_reply`**, never the db.rs function.

**Files:**
- Modify: `rust/web/src/email/commands.rs`

**Steps:**

- [ ] Replace `bump_reply`'s body — **`commands.rs:455-474`**, from
      `let games = crate::db::find_active_turn_games(` at :455 through the
      `}))` that closes `Ok(CommandReply::Status(match n { ... }))` at :474.
      **Leave the function's own closing brace at :475 in place** (an earlier
      draft of this spec said "455-476", which deletes that brace and the blank
      line after it while the replacement text below supplies neither — the file
      would not compile). Replacement:

```rust
    // Fetch one past the cap purely to detect the overflow: the cap is a SQL
    // LIMIT, so without the extra row we cannot tell a full page from a
    // truncated one (wfe F28). `cap_digest` below is the actual enforcement.
    let games = crate::db::find_active_turn_games(
        pool,
        user_id,
        crate::db::SWITCH_DIGEST_CAP + 1,
    )
    .await
    .map_err(|e| CommandError::Internal(anyhow::anyhow!("bump: find turn games: {e}")))?;
    let capped_out = games.len() > crate::db::SWITCH_DIGEST_CAP;
    let capped = crate::db::cap_digest(games, crate::db::SWITCH_DIGEST_CAP);
    let n = capped.len();
    for (game_id, game_player_id) in capped {
        crate::email::notify::send_turn_digest_forced(
            resend,
            pool,
            http_client,
            game_id,
            game_player_id,
        )
        .await;
    }
    let mut msg = match n {
        0 => "No games are waiting on your turn.".to_string(),
        1 => "Re-sent 1 game to your active address.".to_string(),
        n => format!("Re-sent {n} games to your active address."),
    };
    if capped_out {
        // Deliberately does NOT say "reply bump again for the rest":
        // `find_active_turn_games` has a fixed ORDER BY and no cursor, so a
        // second bump re-sends the same games.
        msg.push_str(&format!(
            " More games are waiting; this reply is capped at {}. Open the site to see them all.",
            crate::db::SWITCH_DIGEST_CAP
        ));
    }
    Ok(CommandReply::Status(msg))
```

- [ ] Add a `#[sqlx::test]` for the capped branch **only if seeding 21 games is
      cheap in this test module** — check the existing helper with
      `rg -n "async fn seed_game_with_turn|async fn seed_game" -A 30 rust/web/src/email/commands.rs`.
      If seeding one game takes a multi-statement helper, seeding 21 is a slow,
      brittle test for a nit: **skip it** and instead add a pure-logic assertion
      that the capped sentence is produced, by asserting the uncapped path is
      unchanged (already covered by `bump_resends_only_my_turn_games`) and
      recording in the commit body that the capped branch is verified by
      inspection. Do **not** lower `SWITCH_DIGEST_CAP` to make a test cheaper —
      it is a production quota constant (db.rs:2906).

**Verification checkpoint:**

- [ ] `cargo test -p web --features ssr bump` — `bump_verb_is_case_insensitive`,
      `bump_resends_only_my_turn_games` (asserts exactly
      `"Re-sent 2 games to your active address."`) and
      `bump_sends_regardless_of_web_presence` all PASS **unmodified**. If the
      2-game assertion fails, you changed the uncapped message — revert.
- [ ] `git diff rust/web/src/db.rs` is **empty** for this task.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

**Commit:**

- [ ] `git add rust/web/src/email/commands.rs && git commit -m "fix(email): disclose the bump digest cap (wfe F28)"`

---

## Task 14: document the reserved email verbs for game authors (wfe F29 / D-15 option A) — CARVED OUT

> **CARVED OUT to WP-85 (`specs/WP-85-email-parser-first-dispatch.md`), 2026-07-26.**
>
> Michael's ruling: *"WP-59 Task 14 sounds like a risk, let's pull it out to a
> separate item if we can."*
>
> **Why:** this task was written as a documentation edit, but the work it
> actually implies is a **behaviour change** in `dispatch_email_command`
> (`rust/web/src/email/commands.rs`). D-15 has since been **ANSWERED**
> (2026-07-26, `planning/decisions-ANSWERED.md`) and it settled the *opposite*
> of what this task implemented: on game-scoped messages the **game command
> parser is tried FIRST and platform commands are the FALLBACK**, with one
> small hard-reserved set of escape-hatch verbs (`help` and equivalents) that
> always wins. So the 18-verb reservation gets **deleted, not documented**.
>
> **WP-85 is DEFERRED — BLOCKED ON MICHAEL. Do not execute it.** Michael
> deliberately deferred the escape-hatch verb set: no game uses those verbs
> yet, and he wants time with the current behaviour before deciding.
>
> **Do NOT execute Task 14 as previously written, under any circumstances.**
> The option-A `docs/authoring/COMMANDS.md` text it specified is now
> known-wrong; it is deleted here (preserved in git history and summarised in
> WP-85).
>
> **Nothing else in WP-59 depends on Task 14** — the rest of the package is
> unaffected and proceeds as written.

---

## Final gate (before the last commit — mandatory)

- [ ] `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — the full
      pre-commit suite (throwaway Postgres 18 on 15432 + NATS 2.11 on 14222,
      migrations, `cargo fmt --check`, both clippy passes,
      `cargo sqlx prepare --check`, both test passes). AGENTS.md requires this
      passes before committing any Rust change. Pre-existing DB-test failures in
      a *bare* local run without this script are backlog #40 and not a
      regression, but this script provisions the containers, so failures here
      are real.
- [ ] `git diff --stat origin/master...HEAD` touches **only**:
      `rust/web/src/email/inbound.rs`, `rust/web/src/email/notify.rs`,
      `rust/web/src/email/commands.rs`, `rust/web/src/db.rs`,
      `rust/web/src/error.rs`, `docs/authoring/COMMANDS.md`.
      **If any of these appears, you absorbed a Non-Goal — revert it:**
      `rust/web/src/email/render.rs`, `rust/web/src/email/sweep.rs`,
      `rust/web/src/email/outbound.rs`, `rust/web/src/state.rs`,
      `rust/web/src/main.rs`, `rust/web/src/game/server_fns.rs`,
      `rust/web/src/auth/server.rs`, `rust/web/src/proposals.rs`,
      `rust/web/.sqlx/`, anything under `rust/game/`, anything under
      `rust/web/migrations/`.
- [ ] `rg -n "sqlx::query" rust/web/src/email/commands.rs` — production code
      has none (test-module hits only).
- [ ] `rg -n "from_matches_verified_email|resolve_user_by_verified_from" rust/web/src/email/inbound.rs`
      — both function **bodies** are byte-identical to master (`git diff` shows
      only the arguments passed at the call sites). This is the WP-56 fence.
- [ ] `git diff rust/web/.sqlx/` is empty.

---

## Cross-package / newly discovered

Report these to the Lead. **Do not fix any of them in this package.**

1. **LIVE reserved-verb collision: `end` is unplayable by email in acquire-1
   and starship-catan-1.** Evidence: `commands.rs:1217` (`"end" => return
   run_end(ctx).await`, added post-snapshot by #47) intercepts before
   `crate::game::execute_command` (:1264); `acquire-1/src/command.rs:192-197`
   and `starship-catan-1/src/command.rs:309-313` both expose `end` as a
   top-level `Token`-matched move. **This invalidates D-15's recorded basis
   ("no current collision"), so the "A now, B only when a real game needs it"
   reasoning no longer holds** — a real game needs it, twice. Task 14 documents
   the constraint and the collision as instructed, but the Lead should re-open
   **D-15** with this evidence: option B (an escape prefix such as `move
   <text>`) or renaming the email verb (`end game`) or renaming the two game
   verbs are now live choices with a user-visible bug behind them. Nearest
   owners: **WP-59** (the dispatcher) for an escape prefix; the acquire-1 and
   starship-catan-1 fix packages for a grammar rename. **User decision
   required.**

   **Amendment, 2026-07-26.** The evidence above stands; its tail is now stale.
   **D-15 was re-opened and ANSWERED**: the game command parser runs FIRST on
   game-scoped messages and platform commands are the FALLBACK — exactly what
   this evidence argued for. Task 14 is **carved out to WP-85** (D-54), and
   **WP-85 is DEFERRED pending Michael's decision on the escape-hatch verb set**
   (D-55), so this collision stays open in the meantime. **No user decision is
   outstanding on WP-59.**

2. **`ServerFnError`'s `Display` prefix leaks into user-facing text wherever a
   `ServerFnError` is stringified.** `server_fn-0.8.13/src/error.rs:233-234`
   renders `ServerError(s)` as `"error running server function: {s}"`. Task 9
   fixes the two occurrences in `email/commands.rs`. A sweep for other
   stringification sites is warranted:
   `rg -n 'ServerFnError' rust/web/src --type rust | rg 'to_string\(\)|format!'`.
   Note `crate::error::user_facing_server_error` (`error.rs:14-16`) exists
   precisely for this and appears to be under-used — **WP-54** (frontend error
   surfacing) is the nearest owner; WP-37's spec already flagged AdminPage
   rendering raw `ServerFnError` Display text, so this is the same defect class
   seen from a second angle.

3. **`SELECT game_version_id FROM games WHERE id = $1` is inlined three times.**
   Task 11 converts the `commands.rs:1149` copy to the new
   `db::find_game_version_id_for_game`. Two identical copies remain at
   `rust/web/src/game/server_fns.rs:2333` and `:2375`. Owner: **WP-40 / WP-53**
   (whoever edits `server_fns.rs` next). Mechanical, no behaviour change.

4. **`DELETE FROM login_confirmations WHERE email = $1` is inlined twice in
   `auth/server.rs`** (`:486` inside `confirm_login_inner`, `:850` inside
   `confirm_email_address`), both with the `sqlx::query!` macro. Task 11's new
   `db::delete_login_confirmation` is the natural home. Not converted here:
   `auth/` is another package's scope and touching macro SQL forces a
   `cargo sqlx prepare`. Owner: **WP-34/WP-35/WP-36** (the auth packages).

5. **`cap_digest` was dead weight in `bump_reply` before Task 13.** With the
   old `find_active_turn_games(pool, user_id, SWITCH_DIGEST_CAP)` the SQL
   `LIMIT` had already truncated, so `cap_digest` at `commands.rs:458` could
   never remove anything. Task 13 makes it load-bearing. **Now closed by inspection: the one other
   production caller has the identical redundancy.** `auth/server.rs:884` calls
   `crate::db::find_active_turn_games(&pool, user.id, crate::db::SWITCH_DIGEST_CAP)`
   and then `crate::db::cap_digest(games, crate::db::SWITCH_DIGEST_CAP)` at
   `:887` — the SQL `LIMIT` has already truncated, so `cap_digest` there can
   never remove anything either, and that path likewise cannot tell a full page
   from a truncated one. (`rg -n "cap_digest" rust/web/src` shows only db.rs's
   definition + its own test, `commands.rs:458`, and `auth/server.rs:887`.)
   Owner: **the auth package** (WP-34/WP-35/WP-36) — whichever owns
   `auth/server.rs`'s digest path — since the fix is the same `+ 1` shape as
   Task 13. Not a defect, a dead-code plus missing-disclosure observation.

6. **`run_emails_confirm` had no coverage for the multi-pending-address case**
   before Task 10, and `handle_invite_reply` has **no** automated coverage at
   all despite being the most stateful handler in the file (230 lines, a
   `FOR UPDATE` lock, a game-start side effect and five distinct reply shapes).
   Task 6's checkpoint records why it did not add one (no `AppState` harness in
   `rust/web/tests/` for the email handlers). Owner: a test-infrastructure item
   for the backlog — an `AppState` test fixture would unblock coverage for
   `handle_game_reply`, `handle_invite_reply` and `handle_settings_reply` at
   once, and **WP-57**'s enqueue redesign will need exactly that fixture.
