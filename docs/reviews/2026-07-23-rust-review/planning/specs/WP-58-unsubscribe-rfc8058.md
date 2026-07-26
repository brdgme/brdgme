# WP-58: RFC 8058 one-click unsubscribe

**Findings:** wfe F3 (major), wfe F25 (minor). **Decision:** D-10 answered
option A - build the **HTTPS one-click endpoint**, tokenised, no auth redirect,
**plus two visible links** in the mail: a type-specific "Unsubscribe from ..."
matching the email actually received, and "Manage my subscriptions" to
`/settings`. The headers still point at the one-click endpoint; the visible
links are additional. D-11 (WP-46) fixes which preference column each type owns.

**Landing order:** **WP-59 first** (its Task 5 shrinks
`handle_settings_reply_route`; WP-59 explicitly defers all unsubscribe work
here). **WP-56 first** (it maps `unsubscribe@brdg.me` to the ignore arm and adds
`ensure_settings_email_token` to `outbound.rs`, beside which this WP adds a
second token helper). **WP-51 / WP-46 / WP-38** rewrite `notify.rs` and
`sweep.rs` around two call sites this WP re-signatures - whichever lands second
**rebases on, not forks,** the other. New migration: `landing-order.md` 6.4/6.5.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

- **wfe F3** - every outbound mail advertises
  `List-Unsubscribe: <mailto:unsubscribe@brdg.me?subject=unsubscribe>` plus
  `List-Unsubscribe-Post: List-Unsubscribe=One-Click`. `unsubscribe@brdg.me` has
  no `g-`/`i-`/`s-` prefix, so it lands on the settings route, which reads the
  body and never the subject: the client's empty-body unsubscribe mail gets "I
  could not find a command" and the user stays subscribed. A mailto-only URI
  under `List-Unsubscribe-Post` also violates RFC 8058 (Gmail/Yahoo rules).
- **wfe F25** - `help_text` advertises `subscribe`/`unsubscribe` on the
  standalone (no-game) path, but `dispatch_standalone_server_command` handles
  only `new` and `bump`; both verbs fall through to
  `dispatch_settings_standalone`'s rejection, which also omits `bump`.

## 2. Why it's wrong

- **wfe F3 is correct as written; do not revert.** Verified live: both headers
  are emitted unconditionally by `render_game_email`
  (`rust/web/src/email/render.rs`), and a second hand-built copy sits in the
  `BTreeMap` inside `send_rules_reply_response` (`rust/web/src/email/inbound.rs`).
- **wfe F25 is correct as written; do not revert.** Verified live:
  `subscribe_toggle` (`rust/web/src/email/commands.rs`) is consulted only by the
  game-scoped `dispatch_email_command`; the standalone chain never calls it.
- **F3's own recommendation is superseded.** It wants `unsubscribe@` detected in
  the settings fallback; D-10 chose the HTTPS endpoint and WP-56 routes that
  recipient to the ignore arm. **Add no `unsubscribe@` inbound case** - the
  mailto URI is removed, not honoured.

## 3. Required end state

### 3a. `email/render.rs` - the email-kind discriminator

The type-specific link learns its type from an **explicit new parameter**; there
is no inference. Add, public:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailKind { Turn, GameEvent, Reminder, Invite }

pub struct Unsubscribe<'a> { pub kind: EmailKind, pub token: &'a str }
```

Four inherent methods on `EmailKind` - the single source of truth shared with
the endpoint in 3d - plus `from_slug(&str) -> Option<EmailKind>`:

| variant | `slug()` | `pref_column()` | `link_label()` |
|---|---|---|---|
| `Turn` | `"turn"` | `turn_emails_enabled` | Unsubscribe from turn notifications |
| `GameEvent` | `"game"` | `turn_emails_enabled` | Unsubscribe from game notifications |
| `Reminder` | `"reminder"` | `reminder_emails_enabled` | Unsubscribe from turn reminders |
| `Invite` | `"invite"` | `invite_emails_enabled` | Unsubscribe from game invitations |

The columns are D-11's: `reminder_emails_enabled` alone governs reminders;
`turn_emails_enabled` governs turn and end-of-game mail (what
`should_email_recipient` gates on today). `pref_column()` returns a
`&'static str` used **only** to pick one of three literal SQL statements in 3d -
never interpolated.

Add `unsubscribe_url(kind, token) -> {public_base_url}/api/unsubscribe/{slug}?t={token}`
and `manage_subscriptions_url() -> {public_base_url}/settings`, both via
`crate::config::public_base_url()` like `notify::browser_url`.

### 3b. `render_game_email` - one new parameter, conditional headers

Append a **7th** parameter `unsubscribe: Option<Unsubscribe<'_>>` after
`reply_address`.

- `Some(u)`: `List-Unsubscribe` becomes `<{unsubscribe_url(u.kind, u.token)}>`
  (**mailto deleted**), `List-Unsubscribe-Post` unchanged. Append the two links
  to both bodies after the footer, styled like the existing `rules_url` anchor
  (muted, 12px) in HTML and as `"{label}: {url}"` lines in text: first
  `u.kind.link_label()` -> `unsubscribe_url`, then "Manage my subscriptions" ->
  `manage_subscriptions_url`.
- `None`: emit **neither** header and **neither** link - a direct reply to a
  command the user just sent is not bulk mail, so RFC 8058 does not apply.

### 3c. Call sites

- `email/notify.rs::send_one` - `EmailKind::Turn` for `NotifyKind::Turn`,
  `EmailKind::GameEvent` for `Eliminated`/`Finished`. `NotifyKind` stays private
  and is **not** the discriminator: it covers neither reminders nor invites.
- `email/sweep.rs::send_reminder` - `EmailKind::Reminder`.
- `proposals.rs` - all six `RealInviteMailer` render sites (`send_invite`,
  `notify_changed_reinvite`, `notify_owner_decline`, `notify_cancelled`,
  `notify_started`, `notify_owner_ready`): `EmailKind::Invite`. Each already
  resolves the recipient `user_id` for `fetch_invite_recipient`.
- `email/inbound.rs` - **every** render site there passes `None`. Same file,
  `send_rules_reply_response`: **delete both hand-built header inserts** from its
  `BTreeMap`. Do **not** reroute it through `render_game_email` - its body is
  bespoke pre-rendered HTML, not `EmailContent` blocks. That is the "fix both
  header sites together" resolution.
- `render.rs mod tests` - existing calls pass `None` except the header tests
  (section 5).

At each `Some` site: `outbound::ensure_unsubscribe_token(pool, user_id).await`;
on `Err`, `tracing::warn!` and pass `None`. Never fail a send over this.

### 3d. New endpoint - `rust/web/src/email/unsubscribe.rs` (new module)

Declared in `email/mod.rs`, `#![cfg(feature = "ssr")]`.

- `POST /api/unsubscribe/{kind}?t={token}` - the RFC 8058 one-click target.
  `EmailKind::from_slug`, then the db helper; `200 text/plain` on success **and**
  on an unknown token (do not leak token validity), `400` on an unknown kind.
- `GET` on the same path **must not mutate** (scanners and clients prefetch
  links). It returns a small self-contained HTML page naming the subscription
  plus a POST form to the same URL; this is what 3b's visible link targets.

`db.rs`: `disable_email_pref_by_unsubscribe_token(pool, token, kind) ->
Result<bool>` - a `match` on `kind` selecting one of three literal
`UPDATE users SET <col> = false, updated_at = NOW() WHERE unsubscribe_token = $1`
statements; `Ok(rows_affected() == 1)`. It only ever writes `false`, so a
replayed or scanned link can never re-subscribe anyone.

### 3e. Token storage - new migration, next free number

`rust/web/migrations/0NN_unsubscribe_token.sql` (**next free number**; `022` is
highest today and WP-50/WP-56 also claim one):
`ALTER TABLE public.users ADD COLUMN IF NOT EXISTS unsubscribe_token text;` plus
`CREATE UNIQUE INDEX IF NOT EXISTS idx_users_unsubscribe_token ON
public.users(unsubscribe_token) WHERE unsubscribe_token IS NOT NULL;`

`email/outbound.rs`: `pub async fn ensure_unsubscribe_token(pool: &PgPool,
user_id: Uuid) -> anyhow::Result<String>`, a direct copy of `ensure_email_token`'s
lazy-populate shape reusing the private `generate_email_token`. **Sole
`outbound.rs` edit this WP makes** - note the WP-60 fence exception in the
commit message.

**Not WP-56's `users.settings_email_token`:** that authorises the whole settings
command surface, whereas this value is semi-public by design (Gmail POSTs it,
scanners fetch it) and must authorise nothing but "set one named column false".
Per-user, not per-`game_player`: D-10's unsubscribe is account-wide.

### 3f. `rust/web/src/router.rs::build_router`

Mount both methods beside `/api/webhooks/resend`, i.e. **before**
`.layer(session_layer)`. Unlike `/healthz` (deliberately after, so a Postgres
outage cannot fail the probe) this handler needs the pool anyway, and the
session layer only attaches a session - it never redirects - so an
unauthenticated one-click POST passes through, satisfying D-10's "no auth
redirect".

### 3g. wfe F25 - `email/commands.rs`

In `dispatch_standalone_server_command`, before falling through to
`dispatch_settings_standalone`, handle `subscribe_toggle(verb)`: on `Some(v)`
call `set_turn_emails_enabled(ctx.pool, ctx.user_id, v)` and return the same
`CommandReply::Status` text `dispatch_email_command`'s arm returns - factor that
arm's body into one private helper called from both, do not copy the strings.
Update `dispatch_settings_standalone`'s rejection string to name what it really
accepts: add `bump`, `subscribe`, `unsubscribe`, `rules`.

## 4. Non-goals

- No inbound `unsubscribe@` handling; no change to `parse_reply_address`, quote
  stripping, the settings-route auth (WP-56 / WP-59), `EmailContent`'s fields,
  or the reply-address domain constant (WP-59 Task 8).
- No `/settings` UI work beyond linking to it; no re-subscribe link; no signed,
  expiring or rotating tokens.
- No change to `should_email_recipient`, `EmailRecipient` or any send gate
  (WP-46 / WP-60); `emails on/off|invite|reminder` verbs unchanged.

## 5. Regression test cases

- **`render.rs mod tests`** (two `List-Unsubscribe` assertions already exist -
  update, do not delete): with `Some(Unsubscribe { kind: EmailKind::Reminder,
  token: "tok" })`, `List-Unsubscribe` is
  `<https://.../api/unsubscribe/reminder?t=tok>` and contains no `mailto:`,
  `List-Unsubscribe-Post` is still `List-Unsubscribe=One-Click`, and both the
  reminder label and "Manage my subscriptions" appear in `html` **and** `text`.
  With `None`, neither header key nor either link is present.
- **`EmailKind` unit test:** `from_slug(k.slug()) == Some(k)` for all four
  variants, `from_slug("nope")` is `None`, and the D-11 guard - `Reminder` maps
  to `reminder_emails_enabled`, `Turn` and `GameEvent` to `turn_emails_enabled`.
- **`outbound.rs mod tests`, `#[sqlx::test]`:** `ensure_unsubscribe_token` twice
  on one user returns the same persisted 32-char alphanumeric value (mirror
  WP-56's `ensure_settings_email_token` test).
- **`email/unsubscribe.rs mod tests`, `#[sqlx::test]`:** POST with a valid token
  sets only the matching column false and leaves the other two untouched; POST
  twice is idempotent; POST with an unknown token returns 200 and changes
  nothing; **GET mutates nothing**; an unknown kind slug is a 400.
- **`commands.rs mod tests`** (beside `subscribe_unsubscribe_toggles_turn_emails`):
  `dispatch_standalone_server_command` with `"unsubscribe"` returns
  `CommandReply::Status` and sets `turn_emails_enabled` false, `"subscribe"`
  sets it back, and the rejection string for a genuinely unsupported verb names
  `bump` and `unsubscribe`.

## 6. Riders

| File | One-line fix | Test? |
|---|---|---|
| `email/commands.rs` `dispatch_settings_standalone` (wfe F25) | Rejection copy must list the verbs the standalone path really accepts (`bump`, `subscribe`, `unsubscribe`, `rules`). | y |
| `email/inbound.rs` `send_rules_reply_response` (wfe F3, 2nd header site) | Delete both `List-Unsubscribe*` inserts from the hand-built `BTreeMap`; no replacement. | y (assert absent) |
