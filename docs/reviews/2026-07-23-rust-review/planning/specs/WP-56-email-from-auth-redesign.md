# WP-56: email From-auth redesign

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Stop the inbound-email pipeline from deriving user identity from the
attacker-controlled `From` header, and take account-security mutations off the
email channel entirely.

**Scope (3 findings):** wfe F1 (critical), wfe F17 (critical), wfe F5 (major).
Both remaining criticals of the review are this one defect.

**Binding decision — D-1, answered 2026-07-25, option B.** User's words,
verbatim:

> "Require the s- token for the settings route, verify SPF/DKIM on inbound, AND
> remove account-security commands (add/confirm/activate email address) from the
> email interface entirely."

**REFINED 2026-07-25 (D-1 refinement).** The command removal is **narrower than
"all settings commands"**. Only four verbs leave email: `emails add`,
`emails confirm`, `emails active`/`use`, `emails remove`. **Username (`name`),
theme (`theme`), colours (`colors`/`colours`) and notification preferences
(`emails on|off`, `emails invite`, `emails reminder`) are KEPT** — the user ruled
they are not sensitive. The token + SPF/DKIM authentication work is **unchanged**;
it now exists to protect the retained commands. Cold start is resolved: settings
live in the web UI, which surfaces the tokenised inbound settings address as an
**opt-in reveal**; the token never goes in an email footer. See Task 1's
cold-start bullet and Task 4's scope table.

**Files touched** (all paths under `/home/beefsack/Development/brdgme`)

- `rust/web/src/email/inbound.rs` — route dispatch, settings-route auth, SPF/DKIM gate
- `rust/web/src/email/commands.rs` — delete account-security subcommands, helpers, tests, help text
- `rust/web/src/email/outbound.rs` — reuse `generate_email_token`; add `ensure_settings_email_token`
- `rust/web/migrations/0NN_*.sql` — new column `users.settings_email_token` (next free number; `022` is the highest today)
- `docs/CODING.md` — the new reviewable rule (Task 6)

`outbound.rs` and `migrations/` are **outside** WP-56's declared paths in
`work-packages.md` (`web/src/email/{inbound.rs,commands.rs}`). Flagged, not
silently absorbed: the token needs a column, and the generator already lives in
`outbound.rs` beside the per-game one it copies. No accepted peer spec touches
either.

**How to use this spec.** Code is identified by file + function name, not line
range. **Locate and read each named function before editing it. If what you find
does not match the description here, STOP and report — do not adapt the edit.**
Line numbers are navigational hints only, always marked "approximate, verify".

---

## Predecessor / overlap constraints

Three packages edit `inbound.rs`; same file, different concerns.

- **WP-59 Task 1** (`specs/WP-59-inbound-processing-quality.md`) adds
  `extract_addr_spec`, rewrites `select_route`, and adds a From-extraction +
  `warn`/OK arm inside `resend_webhook`. **Either order** — WP-59 explicitly
  fences `from_matches_verified_email` and `resolve_user_by_verified_from` off as
  "D-1's". WP-56 must not touch `extract_addr_spec` or `parse_reply_address`.
- **WP-59 Task 5** shrinks `handle_settings_reply_route` to a
  `fetch_inbound_text(state, email_id)` call and changes
  `handle_settings_reply`'s third parameter from `raw_body: &str` to
  `text: &str`. **Direct overlap on both functions WP-56 rewrites.**
- **WP-57** (D-2) moves `mark_event_processed` after successful processing and
  returns 5xx on transient failure. **No overlap** — delivery semantics, not
  identity. Do not touch the marker or the status codes.

**If WP-59 already landed**, before editing: `handle_settings_reply` takes
`text: &str` (do not re-add `extract_plain_text` inside it — it moved to the
caller); `handle_settings_reply_route` is ~3 lines around `fetch_inbound_text`,
so add the token parameter there, not to a duplicated `RESEND_API_KEY` block;
`select_route` now works on normalised addresses, and WP-56 changes only what
`resend_webhook` *does* with `Some(Settings)`/`None`. Also: WP-59 Task 10
(wfe F23) and the `emails confirm` half of Task 9 (wfe F24) become **dead code
WP-56 deletes** — expected, not a conflict; note it in the commit message so
nobody re-adds the command to satisfy the older spec. Task 9's `restart` half is
unaffected. **If WP-56 lands first**, whoever executes WP-59 drops those two as
no-ops.

---

## 1. Root cause

Not "the `From` header is spoofable" — that is the symptom. Three design
defects compose:

1. **The routing token was never a secret on one of three routes.** `g-`/`i-`
   addresses carry a 32-char random `email_token` (`game_players.email_token`,
   `game_proposal_players.email_token`, migrations 014/015). The `s-` address
   carries `user_id` — `format!("s-{user_id}@brdg.me")` in
   `send_settings_response` — an identifier, not a secret. Because that "token"
   could not authenticate anything, `handle_settings_reply` needed some other
   identity source and the only thing left in the payload was `from`. **The
   authentication decision came to rest on transport metadata because the
   channel's own credential was designed as an identifier.**
2. **The route table had a default-allow arm.**
   `Some(InboundRoute::Settings(_)) | None => handle_settings_reply_route(...)`
   in `resend_webhook` (approximate, verify: inbound.rs:484). Treating "cannot
   classify" as "route to the weakest-authenticated handler" turns every
   unprefixed local part on the domain into an authentication endpoint —
   `parse_reply_address` carefully rejects empty tokens and the caller then
   discards that care.
3. **The command surface was flat.** `dispatch_standalone_server_command` ->
   `dispatch_settings_standalone` -> `dispatch_settings_command_for_user` ->
   `run_settings_emails` exposes *every* settings verb on *every* email path.
   No layer separates "change my colours" from "change where my mail and my
   bearer tokens are delivered". The weakest channel inherited the strongest
   capability, and `emails add` — which mails a confirmation code to the
   *newly claimed* address — became a self-service takeover chain for anyone
   who could forge one header.

wfe F5 is defect 1 applied to `g-`/`i-`: the `From` check there is genuine
defense-in-depth, but it is *only* that, so the per-game `email_token` is the
sole real credential. That is acceptable **once tokens are treated as bearer
secrets** — which is exactly what wd F26 (`get_proposal` serialising every
invitee's `email_token` to any authenticated viewer) breaks. **wd F26 is fixed
in WP-44, not here.** Note the composition and stop.

---

## 2. Complete solution

### Task 1 — per-user secret settings token (wfe F1, part 1)

- **Migration.** New file `rust/web/migrations/0NN_settings_email_token.sql`
  (`NN` = next free; highest existing is `022_concede_bot_replacement.sql`).
  Follow the shape of `014_email_play.sql`:

  ```sql
  -- WP-56 / D-1: per-user secret token for the settings reply address
  -- (s-{token}@brdg.me). Populated lazily on first settings email, not
  -- backfilled.
  ALTER TABLE public.users
      ADD COLUMN IF NOT EXISTS settings_email_token text;
  CREATE UNIQUE INDEX IF NOT EXISTS idx_users_settings_email_token
      ON public.users(settings_email_token) WHERE settings_email_token IS NOT NULL;
  ```

  Migrations are immutable once applied (AGENTS.md) — new number, never an edit.

- **Token shape.** Identical to the per-game token: 32 chars of `[A-Za-z0-9]`.
  Change `generate_email_token` in `outbound.rs` from `fn` to `pub(crate) fn` and
  reuse it. No second generator, and never derived from `user_id`.

- **Generation/lookup helpers.** In `outbound.rs` beside `ensure_email_token`
  (the pattern to copy — read it first; plain `sqlx::query`, not the macros, per
  CODING.md "Plain (non-macro) sqlx queries"):
  `pub async fn ensure_settings_email_token(pool: &PgPool, user_id: Uuid) ->
  anyhow::Result<String>` — `SELECT settings_email_token FROM users WHERE id =
  $1`, return it if present, else generate and `UPDATE users SET
  settings_email_token = $1, updated_at = NOW() WHERE id = $2`.
  `ensure_email_token`'s known lost-update race (wfe F30) is owned elsewhere;
  do not fix it, do not copy anything worse.
  In `inbound.rs` beside `find_game_player_by_email_token`:
  `async fn find_user_by_settings_token(pool: &PgPool, token: &str) ->
  anyhow::Result<Option<Uuid>>` — `SELECT id FROM users WHERE
  settings_email_token = $1`.

- **Emit the token.** In `send_settings_response`, replace
  `format!("s-{user_id}@brdg.me")` (approximate, verify: inbound.rs:1191) with
  the token form; it already takes `pool`, so call
  `ensure_settings_email_token` there. On error, log and omit the reply address
  — never fall back to the `user_id` form. If WP-59 Task 8 landed, use its
  `settings_reply_address(token)` helper. Leave
  `thread_id = format!("settings-{user_id}")` alone: threading id, not a
  credential.

- **Verify the token.** `handle_settings_reply_route` and
  `handle_settings_reply` gain a `token: &str` parameter.
  `handle_settings_reply` replaces its `resolve_user_by_verified_from` call with:
  1. `find_user_by_settings_token(pool, token)` -> `None` => log at info, no
     reply, return. (Never reply to an unknown token; a reply is an oracle.)
  2. `from_matches_verified_email(pool, user_id, from)` -> `false` => log at
     info, no reply, return. This keeps the `From` check as **defense in
     depth**, exactly as `handle_game_reply` does, and is what the finding's
     recommendation asks for.

  `resolve_user_by_verified_from` then has no non-test callers. **Delete it**
  and its tests (`resolve_user_by_verified_from_truth_table`,
  `resolve_user_by_verified_from_is_the_should_respond_gate`; approximate,
  verify: inbound.rs:1891, :1965). Grep to confirm no other caller before
  deleting.

- **Cold start — RESOLVED by the user (2026-07-25 refinement), still not this
  package's build.** Once the `s-` address is secret, a user can only reach the
  settings route by replying to a settings email brdg.me already sent, or by
  using settings verbs on a `g-` game reply (`dispatch_email_command` falls
  through to `dispatch_settings_command`; approximate, verify: commands.rs:1260).
  A brand-new user with no games has no discoverable settings address. **The
  user's resolution:** settings are managed in the **web UI**, and the tokenised
  inbound settings address is **surfaced there as an opt-in** (the web settings
  page reveals it on request). **Do NOT put the token in email footers** — not in
  `notify.rs`, not in `List-Unsubscribe`, not in any turn or proposal email. A
  bearer secret in a footer is a bearer secret in every forwarded mail.
  Implementing the web-UI reveal is **not in WP-56**; record it in the handover.
  What WP-56 must do is *not* build any fallback discovery path.

### Task 2 — verify SPF/DKIM (wfe F1 part 2, wfe F5)

**What this spec cannot determine from source, and you must verify:** the repo
never deserialises an authentication verdict. `ResendInboundData` (approximate,
verify: inbound.rs:167) has exactly four fields — `email_id`, `from`, `to`,
`received_for`. **There is no SPF/DKIM field name to copy and this spec will not
invent one.**

Do this, in order:

1. Check Resend's current `email.received` payload documentation for an
   authentication-results field (other providers use `spf`/`dkim` verdict
   objects or an `authentication_results` string — **confirm, do not assume**).
2. Cross-check against a real payload: bodies are not persisted and
   `resend_webhook` logs none of them, so capture one from the Resend dashboard
   webhook log, or via a temporary env-gated `tracing::debug!` of the raw body
   in a dev deploy.
3. **If a verdict field exists:** add it to `ResendInboundData` as
   `#[serde(default)] Option<...>` (absent must not fail deserialisation), plus
   a pure `pub enum AuthVerdict { Pass, Fail, Unknown }` and
   `pub fn classify_inbound_auth(...) -> AuthVerdict` in `inbound.rs`. Gate in
   `resend_webhook` **after** the `event_type` check and **before**
   `select_route`: `Fail` => `tracing::warn!` with `from` + `email_id`, return
   `StatusCode::OK` (permanent rejection, must not be retried), no reply of any
   kind. `Unknown` => `warn!` and proceed, so a payload change degrades to
   today's behaviour rather than blackholing all mail.
4. **If no verdict field exists:** STOP and report before writing a fallback.
   The fallback would parse the topmost `Authentication-Results` /
   `Received-SPF` from the raw MIME `fetch_raw_email` already retrieves
   (`mail_parser` is a dependency), which is sound **only** if you trust exactly
   the topmost header stamped by Resend's MTA and ignore every lower one — the
   sender can inject their own. That trust anchor (Resend's authserv-id) is not
   knowable from this repo, so it needs the user's ruling, and it moves the
   check after the body fetch.

Whichever path: the verdict check is a **pure function with unit tests**. Do
not bury the parsing inline in `resend_webhook`.

### Task 3 — remove the unrouted-`None` fallthrough (wfe F1 part 3)

In `resend_webhook`, split the combined arm (approximate, verify:
inbound.rs:484):

```rust
Some(InboundRoute::Settings(token)) => {
    handle_settings_reply_route(&state, &token, &event.data.from, &event.data.email_id).await;
}
None => {
    tracing::info!("resend webhook: no route for recipient; ignoring");
}
```

No reply, no bounce generated by us. Note for the executor: WP-59 Task 1 adds
its own `warn`+OK arm for the display-name case in this same `match`; if it has
landed, extend the existing structure rather than reverting it.

Known consequence, deliberate: `unsubscribe@brdg.me` (advertised in
`List-Unsubscribe`) currently reaches the settings handler only *because* of
this fallthrough, and already cannot work (wfe F3 — the settings path reads the
body, never the subject). This change makes that dead end explicit. **wfe F3 /
D-10 is WP-58's.** Do not add an `unsubscribe@` special case here.

### Task 4 — delete account-security commands (wfe F17)

**SCOPE NARROWED by the user, 2026-07-25 refinement to D-1. Read this before
touching anything.** The email settings surface is **not** being emptied. Exactly
four subcommands leave the email interface, and nothing else:

| Verb | Fate | Why |
|---|---|---|
| `emails add <addr>` | **DELETE** | adds an additional email address — account-security mutation |
| `emails confirm <code>` | **DELETE** | completes the add — same chain |
| `emails active <addr>` / `emails use <addr>` | **DELETE** | changes the default/active address — redirects credential delivery |
| `emails remove <addr>` | **DELETE** | confirmed by the user; strips the victim's secondaries |
| `name <display name>` | **KEEP** | username/display name — not sensitive, not a credential |
| `theme <name>` | **KEEP** | display preference |
| `colors <c1,c2,c3>` / `colours` | **KEEP** | display preference |
| `emails` (bare listing) | **KEEP** | read-only |
| `emails on` / `emails off` | **KEEP** | notification preference |
| `emails invite on|off` | **KEEP** | notification preference |
| `emails reminder on|off` | **KEEP** | notification preference |
| `settings` (summary) | **KEEP** | read-only |

Changing username, theme and notification preferences by email **stays** — the
user ruled these are not sensitive. The `s-` token + SPF/DKIM work (Tasks 1-3) is
**unchanged and still required**: it is what protects the retained commands. Do
not narrow Tasks 1-3 on the argument that the surviving verbs are low-value.

Edit the **shared** implementation, not the standalone dispatcher, so both the
`s-` path and the `g-` game path lose the capability at once:

- `rust/web/src/email/commands.rs`, `run_settings_emails` (approximate, verify:
  commands.rs:547): delete the `"add"`, `"confirm"`, `"active" | "use"` and
  `"remove"` match arms. Keep `on`/`off`/`invite`/`reminder` and the bare
  `emails` listing (`run_emails_list` — read-only, harmless).
- Delete `run_emails_add`, `run_emails_confirm`, `run_emails_active`,
  `run_emails_remove` entirely.
- Rewrite `run_settings_emails`'s fallback usage string to the surviving forms
  plus one sentence naming the replacement, e.g. `"Email addresses are managed
  on the website under Settings."` Same for `help_text`: delete its four
  `emails add|confirm|active|remove` lines (approximate, verify:
  commands.rs:176-179), leave one line pointing at the web settings page.
- The orphaned callees — `crate::auth::server::{request_confirmation_code,
  validate_confirmation_code}` and `crate::db::{find_email_owner,
  insert_unverified_email, mark_email_verified, set_primary_email,
  remove_user_email, SetPrimaryOutcome, RemoveEmailOutcome}` — are all still
  used by the web server fns. **They stay.** Only `commands.rs`'s references go.
  Do not delete anything in `db.rs` or `auth/server.rs`.
- Delete the matching tests in `commands.rs`'s `mod tests` (approximate, verify:
  the `emails add`/`confirm`/`active`/`use`/`remove` cases, ~:1757-:2050) and fix
  the `help_text`/`settings_verb` assertions naming the deleted verbs
  (approximate, verify: :1692-1706). `settings_verb` itself is unchanged —
  `emails <anything>` still maps to `"emails"`, the unknown subcommand now hits
  the usage error.
- `run_emails_confirm`'s inline SQL and `login_confirmations` cleanup go with the
  function; WP-59 Task 11 (wfe F26) may name it, in which case it is a no-op.

**Not a feature regression.** The web equivalents already exist and ship today:
`add_email_address`, `confirm_email_address`, `make_email_address_active`,
`remove_email_address` in `rust/web/src/auth/server.rs`, wired from
`rust/web/src/settings.rs`. Per D-12+D-14 that flow moves to a
confirmation-link form and an email change will require re-verification —
**that work is WP-35's, not WP-56's. Do not build any web UI or server fn
here.** WP-56 only removes the email-side commands.

**`emails remove` — CONFIRMED by the user, no revert path.** The original answer
named `add`/`confirm`/`activate` and not `remove`; the spec extended it, and the
2026-07-25 refinement **confirms `remove` is removed from email too**. The
earlier "one-arm revert" caveat is **withdrawn** — do not keep `remove`, do not
raise it again.

**Explicitly retained (do not delete, do not deprecate):** `name`, `theme`,
`colors`/`colours`, bare `emails`, `emails on`/`off`, `emails invite on|off`,
`emails reminder on|off`, `settings`. wfe F17 called `name` "arguably"
account-security; the user has now ruled it is not sensitive. Deleting any verb
in this list is a scope violation, not thoroughness.

### Task 5 — plumbing checklist

- `handle_settings_reply_route(state, token, from, email_id)` and
  `handle_settings_reply(state, token, from, body_or_text)` signatures.
- Every existing `handle_settings_reply*` test updated for the new parameter.
- `select_route_routes_invite_and_settings` (approximate, verify: ends around
  inbound.rs:1441) still passes — routing is unchanged; only dispatch changed.

### Task 6 — the forward constraint (see section 3) into `docs/CODING.md`.

---

## 3. Constraint going forward

Three rules, each mechanically checkable by a reviewer with `rg`:

1. **Identity never comes from unauthenticated transport metadata.** On any
   non-interactive ingress (webhook, inbound email, queue message) the acting
   principal is resolved from a server-generated secret the request presents — a
   token row, a signed cookie, a session. Sender-supplied envelope/header fields
   (`From`, `To`, `Reply-To`, `X-*`, `Received`) may only be a *secondary*
   must-also-match check. **Review check:** every function resolving a
   `user_id`/`game_player_id` from an ingress payload takes a token argument; a
   `WHERE ... email = $1` lookup keyed on a header value with no token in scope
   is the bug.
2. **Credential-mutating operations are web-session-only.** Anything changing
   where credentials are delivered or what can authenticate — email addresses,
   passkey material, sessions, API tokens, token rotation — is reachable only
   behind `get_current_user`. Non-interactive channels get read-only and
   preference-only verbs. **Review check:** for each verb in an email/webhook
   dispatch table, does its handler write to `user_emails`, `sessions`,
   `login_confirmations`, or any `*_token` column? Then it does not belong there.
3. **No default-allow arm in a route table.** Unclassifiable input terminates in
   an explicit ignore, never a fallthrough to the least-authenticated handler.
   **Review check:** `match` arms shaped `Some(WeakRoute(_)) | None =>` in
   ingress routing.

---

## 4. Documentation updates

Read first: `docs/CODING.md` (sections "General Principles", "Server
Functions", "Database", "Testing Conventions"); `docs/email.md` (**rendering
only** — MJML, dark mode, the `font-size:0` hazard; nothing on inbound auth, so
nothing to change); `docs/authoring/COMMANDS.md` (game-author facing, and WP-59
Task 14 owns its email section — do not edit). No inbound-email architecture doc
exists; do not create one.

**Add to `docs/CODING.md`, as a new `## Inbound and Webhook Authentication`
section immediately after `## Server Functions`:**

```markdown
## Inbound and Webhook Authentication

**Identity on a non-interactive channel comes from a server-generated secret,
never from sender-supplied metadata.** Inbound email, webhooks and queue
messages are routed by a random token the server minted and mailed/handed out
(`game_players.email_token`, `game_proposal_players.email_token`,
`users.settings_email_token` — all 32-char `[A-Za-z0-9]` from
`email::outbound::generate_email_token`). The sender's `From` address is
checked *in addition* (`from_matches_verified_email`) as defense in depth, and
is never sufficient on its own: SMTP `From` is attacker-controlled. A lookup
that resolves a user from a header value with no token in scope is a
vulnerability, not a shortcut.

**A provider signature authenticates the provider, not the sender.** The svix
verification on `/api/webhooks/resend` proves Resend sent the request. It says
nothing about who sent the email. SPF/DKIM verdicts are checked separately, and
a failure rejects before any routing.

**Unroutable input terminates.** Ingress routing must not fall through to its
weakest-authenticated handler for input it cannot classify: log and ignore.

**Credential-mutating operations are web-only.** Adding, confirming, switching
or removing an email address, and anything else that changes where credentials
are delivered or what can authenticate, lives behind `get_current_user` in a
server fn. The email command surface carries game moves, notification
preferences and display preferences only (`email::commands`) — display name,
theme, colours, and the per-notification-type toggles all remain available by
email. This is a deliberate capability boundary drawn at *credentials*, not a
blanket ban on settings over email, and not an unfinished feature.
```

---

## 5. Regression-test plan

**Where tests live.** Inline `#[cfg(all(test, feature = "ssr"))] mod tests` at
the bottom of both `inbound.rs` and `commands.rs`; DB cases use `#[sqlx::test]`
taking a `pool: sqlx::PgPool`. Integration tests are
`/home/beefsack/Development/brdgme/rust/web/tests/`.

**The fixture gap, verified.** specs-LOG.md records "no AppState test fixture
exists for the email handlers". Confirmed for the inline modules (`rg AppState
rust/web/src/email` returns only production signatures) but **narrower than it
reads**: `tests/ssr_pages.rs::make_state` and `tests/websocket_hygiene.rs` both
build a real `AppState`, so one is constructible at the integration layer (needs
live Postgres **and** NATS, i.e. `scripts/rust-test.sh`). Two hard limits
remain and they shape this plan: `resend_webhook` needs a valid svix signature
plus `RESEND_WEBHOOK_SECRET`, and it constructs `ResendInbound` inline from
`RESEND_API_KEY` instead of taking the injectable `InboundEmailSource` trait
(the `StaticInbound` double exists but the handler cannot reach it). So
**end-to-end webhook coverage is not achievable here.** Do not build that
harness, do not stub `AppState`. Push the new logic into pure/pool-only
functions, cover those, record the residual gap in the handover.

Cases to add:

| Test | Level | Input | Expected |
|---|---|---|---|
| `ensure_settings_email_token_generates_and_reuses` | `#[sqlx::test]` | new user, call twice | 32 chars, `[A-Za-z0-9]` only, both calls equal, column persisted |
| `find_user_by_settings_token_lookup` | `#[sqlx::test]` | seeded token / unknown token / empty string | `Some(user_id)` / `None` / `None` |
| `settings_token_is_not_the_user_id` | `#[sqlx::test]` | token for user `u` | `!= u.to_string()`; `find_user_by_settings_token(u.to_string())` is `None` |
| `settings_reply_requires_token_and_from` | `#[sqlx::test]`, pool-level (call the two auth helpers in sequence, mirroring `handle_settings_reply`) | (valid token, verified From) / (valid token, foreign From) / (wrong token, verified From) | accept / reject / reject |
| `classify_inbound_auth_*` | unit | pass verdict / fail verdict / absent field | `Pass` / `Fail` / `Unknown` (only once Task 2 step 1 fixes the real field shape) |
| `unrouted_recipient_is_ignored` | unit on `select_route` + a table-driven assertion on the dispatch decision | `["hello@brdg.me"]`, `["unsubscribe@brdg.me"]` | `None`; must map to the ignore arm, not settings |
| `run_settings_emails_rejects_removed_subcommands` | `#[sqlx::test]` | `emails add x@y.com`, `emails confirm 123456`, `emails active x@y.com`, `emails use x@y.com`, `emails remove x@y.com` | all `Err(CommandError::User(_))` whose message names the website; **and no row written to `user_emails`** (assert the count is unchanged — this is the actual takeover regression test) |
| `help_text_omits_address_management` | unit | — | `!text.contains("emails add")`, same for `confirm`/`active`/`remove`; still contains `emails on` |
| `retained_settings_verbs_still_work` | `#[sqlx::test]` | `name Bob`, `theme system`, `colors ...`, `emails on`, `emails off`, `emails invite on`, `emails reminder off`, bare `emails`, `settings` | all `Ok` — this is the narrowed-scope regression guard; a future agent must not "finish the job" by deleting these |
| `game_path_cannot_manage_addresses` | `#[sqlx::test]` | `dispatch_email_command` with `emails add attacker@evil.com` | `Err(User)`, `user_emails` unchanged (proves Task 4 closed both paths) |

Update, do not delete: the `settings_verb`/`help_text` assertions (approximate,
verify: commands.rs:1692-1706) and `settings_standalone_rejects_game_command`
(approximate, verify: inbound.rs:1946). Delete with their subject:
`resolve_user_by_verified_from_truth_table`,
`resolve_user_by_verified_from_is_the_should_respond_gate`. Keep
`from_matches_verified_email_truth_table` — that function survives as the
secondary check.

**Commands the implementer runs (this spec's author ran none of them):**

```
cd /home/beefsack/Development/brdgme/rust
cargo fmt --all -- --check
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
/home/beefsack/Development/brdgme/scripts/rust-test.sh
```

The new migration means `sqlx migrate run --source web/migrations` must run
before the DB tests, and `(cd web && cargo sqlx prepare --check -- --tests
--features ssr --all-targets)` must pass — `rust-test.sh` does both. All new
queries use plain `sqlx::query`/`query_as`, so no `.sqlx` regeneration is
needed. DB-backed test failures without the containers are a known
pre-existing condition (AGENTS.md, backlog #40), not a regression.

---

## 6. Non-goals

Do not absorb any of these; each is owned elsewhere.

- **No web-UI email-change flow, confirmation-link form, or
  re-verification-on-change logic — WP-35 (D-12+D-14).** The existing
  `auth/server.rs` fns are the current replacement and stay untouched.
- **No wd F26 re-fix — WP-44.** Do not touch `get_proposal` or its serialised
  fields, and do not rotate existing `email_token` values.
- **No delivery-semantics changes — WP-57 (D-2).** Do not move
  `mark_event_processed`, add retries, or change any `StatusCode` other than the
  new SPF-fail arm's explicit `OK`.
- **No general inbound cleanup — WP-59.** No `extract_addr_spec`, quote-stripping,
  `RESEND_API_KEY` de-duplication, reply-address constants, dead-code deletion or
  `parse_reply_commands` edits — unless WP-59 landed, then build on it.
- **No `unsubscribe@` / `List-Unsubscribe` work — WP-58 (D-10).**
- **No web-UI reveal of the tokenised settings address.** The user resolved cold
  start that way (Task 1) but building it is not WP-56; hand it to the
  `settings.rs` owner. And under no circumstances put the token in an email
  footer.
- **No `ensure_email_token` lost-update-race fix (wfe F30). No removal of the
  `name`, `theme`, `colors`/`colours`, `settings` or `emails on|off|invite|reminder`
  verbs. No AppState/webhook test harness or `InboundEmailSource`
  injection — record that gap, do not close it. No changes under
  `rust/web/src/db.rs` or `rust/web/src/auth/`.**

---

## 7. Finding-recommendation audit

**wfe F1 (critical) — RIGHT, adopted in full.** Secret `s-` token + require it +
keep the From match + drop the `None` fallthrough + consult SPF/DKIM: every
clause survives re-derivation; Tasks 1-3 are that recommendation verbatim. Two
notes, neither a correction: its hedge "*if* the payload exposes it" is
**unresolved** — no such field exists on `ResendInboundData` and this spec
refuses to guess one (Task 2); its severity caveat about Resend possibly
rejecting unauthenticated mail upstream is untested and irrelevant to the fix,
since taking no defense of one's own is the defect either way. **Do not revert
the `None` arm split** on the argument that `unsubscribe@` needs it — that path
was already broken (wfe F3) and is WP-58's to rebuild.

**wfe F17 (critical) — diagnosis RIGHT, two of its three recommendations
WRONG.** It offered (a) exclude the subcommands from the standalone path,
(b) require a confirmation round-trip to the current primary, (c) "at minimum
require the game-token-authenticated path". **(a) and (c) are unsafe as
written:** settings verbs are also dispatched on the game path
(`dispatch_email_command` -> `dispatch_settings_command`), so "restrict to the
game-token path" leaves the whole takeover chain intact for any attacker holding
one `g-` token — and per wfe F5 + wd F26 those tokens leak. (b) is sound but
strictly more code than deletion for a capability that already has a working web
UI. D-1 option B (delete) is the adopted, stronger fix. The chain it described —
code delivered to the attacker's own address, then per-game reply tokens
following the redirected primary — is exact and is why this is critical. Its
aside that `name` is "arguably" the same class is **not** adopted: no decision
covers it, and it is not a credential.

**wfe F5 (major) — RIGHT, deliberately split across two packages.** WP-56
delivers the SPF/DKIM half (Task 2, same work as F1's third clause) and the
design rule (section 3). The token-leak fix and rotation are **WP-44's**. Its
judgement that the `From` check "adds no defense once a token is known" is
correct and is exactly why this spec **keeps** it as a must-also-match: one
query, and the only thing between a leaked token and a forged reply. **Do not
remove `from_matches_verified_email` from `handle_game_reply` or from the new
settings path.** One clause not adopted: "consider rotating a game/invite token
after use" — per-use rotation breaks replying twice to one turn email, which is
normal behaviour. Route it to WP-44 as a decision, do not implement it silently.
