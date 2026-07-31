# Unit 05b - Web server: admin, bot supervision, db.rs

Review of the 2026-07-25..2026-07-30 remediation as it lands in `rust/web`'s admin
surface, bot consumer supervision, and the database layer. Findings continue from
F-96 (05a), i.e. this unit starts at **F-97**.

## Commits reviewed

| Commit | WP | Nominal scope |
|--------|----|----------------|
| `b49df619` | WP-37 | `admin.rs` |
| `baa5fc64` | WP-41 | `db.rs` |
| `347970a0` | WP-39 | bot consumer supervision |
| `914aa0c6` | WP-38 | bot-turn wedge recovery |
| `618156a7` | WP-68 | `term_size` |
| `4d31f6eb` | WP-82 | `db.rs` split (non-auth remainder) |
| `13a1e693` | WP-36 | non-crypto half: WebSocket close-frame shutdown, `tests/websocket_hygiene.rs`, `admin.rs` |

Out of scope (owned elsewhere): `a9609e57` auth/crypto (05a) and its
`import_game.rs` guard (Unit 07).

## Findings

### F-97 (Medium) - `test_provider` / `test_bot_provider` are an admin-triggered read-SSRF into the pod network, and `/metrics` is a named target

`rust/web/src/admin.rs:254-262` (`validate_provider_url`), used by
`create_provider`/`update_provider`; fetched at `:881-887` and `:970-976`.

`validate_provider_url` checks only `starts_with("http://")` /
`starts_with("https://")`. The stored URL is then POSTed to
(`{url}/v1/chat/completions`) from inside the pod, and the result is handed back
to the caller: `test_provider` returns `HTTP {status}: {text}` on any non-2xx
(`:892-894`) and `test_bot_provider` returns status, allowlisted headers and up
to 8 KiB of body as a struct (`:983-988`).

ws F23 hardened what is done with a *hostile upstream's* response (body cap,
header allowlist) but nothing constrains *which* upstream. `http://127.0.0.1:9090/metrics`,
`http://169.254.169.254/...` and any in-cluster service are all reachable. This
directly defeats the stated containment of the metrics port -
`rust/web/src/main.rs:195-198` documents `/metrics` as "Not exposed via any k8s
Service or HTTPRoute - only reachable by something with direct pod-network
access". The admin UI is exactly that. The POST/404 shape still leaks the body
for most internal endpoints because non-2xx responses are echoed verbatim.

Admin-gated, so not privilege escalation - but it converts "admin" into
"arbitrary in-cluster HTTP read", which is a materially larger blast radius than
the role is documented to have, and it is reachable by a single stolen admin
session.

Remediation: resolve the host at save time and reject loopback, link-local,
RFC1918 and unique-local addresses unless an explicit allowlist env var permits
them; re-resolve before the request to close the DNS-rebinding window. At
minimum move `/metrics` off `0.0.0.0` (`METRICS_ADDR` default,
`rust/web/src/main.rs:209`) and document the SSRF surface as accepted.

### F-98 (Medium) - ws F25's server-side validation sweep skipped the one field that is a credential

`rust/web/src/admin.rs:515-533` (`create_provider`), `:568-586`
(`update_provider`, `ApiKeyUpdate::Set`).

`require_text` is applied to provider name, URL, bot name, model and reasoning
effort; `validate_temperature`, `validate_extra_body` and
`validate_provider_url` cover the rest. `api_key` is the only user-supplied
string on this surface with no validation at all: `ApiKeyUpdate::Set("")` is
accepted, encrypted, and stored. `mask_api_key("")` then renders `"(set)"`
(`:463-470`, `len < 8` branch), so the admin page positively asserts a key is
configured, and `test_provider` sends `Authorization: Bearer ` with an empty
credential. There is also no upper bound, so an arbitrarily large blob is
encrypted and stored.

This is systemic pattern 2 (inconsistent hardening within a single file) landing
inside the very commit that added the validation helpers.

Remediation: `ApiKeyUpdate::Set(k) => require_text(&k, "API key", 512)?` before
encrypting, in both `create_provider` and `update_provider`.

### F-99 (Medium) - ws F35's coverage gap was closed by one smoke test that asserts the empty case of 22 functions, and its own doc comment says so

`rust/web/src/db/mod.rs:161-256`
(`ws_f35_previously_untested_functions_are_reachable`).

ws F35 was "27 public `db.rs` functions have zero test references". The fix is a
single `#[sqlx::test]` that names 22 of them and asserts, for almost all, the
degenerate result:

- `assert!(!is_user_admin(&pool, a.id).await.unwrap())` - only the `false` path
  of the function that gates the entire admin surface (`require_admin`,
  `admin.rs:46-57`). The `true` path - the security-relevant one - is untested
  here.
- `mark_game_read(&pool, game.id, a.id).await.unwrap()` - asserts nothing about
  the row it is supposed to write.
- `insert_game_logs_tx(&mut tx, game.id, vec![])` - called with an empty vec, so
  it inserts nothing.
- `set_user_pref_colors(&pool, a.id, &[])` - empty slice, no read-back.
- `get_pending_request_source(..., Uuid::new_v4(), ...)` / `find_user_by...`
  variants asserted `is_none()` against a random id.
- `find_open_restart_proposal`, `find_open_restart_proposal_tx`,
  `get_user_by_email`, `has_block_conn`, `should_hide_add_friend`,
  `replacement_bot_available` - all asserted on their negative/absent case only.

The doc comment states the intent plainly: "This test is a *reminder*, not a
mechanism: it re-asserts the cheapest invariant of each newly covered function".
That satisfies "has a test reference" literally while leaving the behaviour of
every listed function unpinned - the session's recurring failure mode, and the
reason nothing in this file would catch a regression in `is_user_admin`.

Remediation: keep the reminder test as a reachability guard, but add real
behavioural tests for at least the security- and write-path members of the list -
`is_user_admin` (true and false), `mark_game_read` (row observably updated),
`insert_game_logs_tx` (non-empty vec, rows present), `set_user_pref_colors`
(write then read back).

### F-100 (Low) - the WP-41 test documents and asserts that session tokens never expire server-side, instead of the expiry being implemented (pattern 4b)

`rust/web/src/db/mod.rs:119-159` (`session_token_validation`).

The test back-dates `user_auth_tokens.created_at` by 40 days and then asserts
the token is *still* valid, with the message "DB layer has no created_at expiry
check - session expiry is cookie-side only", plus a 6-line NOTE explaining that
"the 30-day window described in the plan is enforced only by the
`tower_sessions` cookie expiry". The plan's window became a comment; the test
now pins the absence of the check as intended behaviour.

Consequences: `validate_session_token` is a pure existence check, nothing prunes
`user_auth_tokens`, and server-side revocation depends entirely on
`invalidate_auth_token` being called. A row leaked from the table is a
credential with no server-side lifetime.

This is a fifth confirmed instance of systemic pattern 4b/4c and belongs in the
process-fixes section. Severity is Low only because the reachable path is
additionally gated by the `tower_sessions` store expiry (05a, F-85/F-86 territory).

Remediation: either add the `created_at > now() - interval '30 days'` predicate
to `validate_session_token` and flip the assertion, or record the deviation as
an explicit accepted decision rather than as a test comment.

### F-101 (Medium) - a transient bot-command failure costs 5 minutes of bot inaction, because ws F58's `ack_wait` and wd F5's "leave unacked" were never reconciled

`rust/web/src/game/mod.rs:329-355` (`run_bot_command_consumer`, `Other` arm) and
`rust/web/src/nats.rs:150` (`ack_wait = Duration::from_secs(5 * 60)`).

Below the delivery ceiling the message is deliberately left unacked (WP-38 spec
3b: "Below it, leave unacked as today"). Redelivery therefore waits the full
`ack_wait`, which ws F58 raised to 5 minutes precisely so a long handler is not
redelivered mid-flight. The two decisions were made in different work packages
and never composed: any transient failure (DB blip, game-service 500) now stalls
that bot's turn for 5 minutes, and two of them for 10 minutes, before the third
delivery Terms the message and wedges the game permanently. The same 5-minute
penalty applies to every process restart that interrupts an in-flight
`bot.command` - i.e. every deploy - and each restart also burns one of the three
deliveries. Confirmed that this is unavoidable today: the consumer task is a bare
`tokio::spawn` with no cancellation token and no drain (`main.rs:72-90`; see F-109),
so SIGTERM cannot do anything *but* abandon the in-flight message.

Remediation: replace the do-nothing branch with
`message.ack_with(AckKind::Nak(Some(backoff)))` and a short exponential backoff
derived from `info.delivered` (seconds, not minutes). `Nak` with a delay is the
JetStream-native way to say "retry soon"; it leaves `ack_wait` free to keep
doing its actual job of bounding handler duration.

### F-102 (Medium) - NATS stream/consumer drift is detected and then ignored, so the deployment that predates the `ack_wait` fix keeps the bug the fix was for

`rust/web/src/nats.rs:121-179` (`ensure_stream_and_consumers`).

`get_or_create_stream` / `get_or_create_consumer` never update an existing
durable. The ws F57 remediation added `stream_config_drift` /
`consumer_config_drift`, which produce a `tracing::warn!` and then fall through
to use the server's values. The warning text says so explicitly ("code changes
to consumer config are NOT applied to an existing durable consumer;
delete/recreate it manually to apply").

That makes the ws F58 fix inert on any environment where the `bot-turn` /
`bot-command` consumers already exist with the pre-fix `ack_wait`: the whole
point of raising it to 5 minutes was to stop JetStream redelivering mid-handler
and running a bot command twice, and the running cluster keeps the old value with
only a startup log line to show for it. `MAX_DELIVER` is likewise used as the
`Term` ceiling in `run_bot_command_consumer` (`game/mod.rs:330`), so a server-side
`max_deliver` below 3 strands messages before the code's Term can fire - the
exact stranding wd F5 was raised to eliminate.

Detection without reconciliation is symptom-papering for a criterion whose
purpose was to make the two values agree.

Remediation: use `Stream::create_consumer` (which updates an existing durable's
config on NATS >= 2.10) rather than `get_or_create_consumer`; if the deployment
cannot guarantee that server version, make drift on `ack_wait` or `max_deliver`
a startup failure rather than a warning, since both are correctness-bearing.

### F-103 (Low) - `create_pool` panics from inside a `Result`-returning function and configures nothing

`rust/web/src/db/mod.rs:94-101`.

`std::env::var("DATABASE_URL").expect("DATABASE_URL must be set")` inside
`pub async fn create_pool() -> Result<PgPool>`: the error channel exists and is
unused, so a missing var aborts instead of producing the `anyhow` context every
other failure in this function gets. `PgPool::connect` also takes every default
(max 10 connections, default acquire timeout, no `test_before_acquire` tuning),
which is the pool backing every request in the monolith. WP-82 correctly moved
this verbatim; flagging it here so it is not lost, and because Unit 08 owns query
performance and will want the pool sizing.

Remediation: `let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;`
and a `PgPoolOptions` with an explicit `max_connections` / `acquire_timeout`.

### F-104 (High) - bot-slot validation is case-insensitive but every consumer of `bot_name` is case-sensitive, so a crafted game creation produces a permanently wedged game that the WP-38 safety net specifically refuses to rescue

Chain, all confirmed:

1. `rust/web/src/db/bots.rs:57-71` (`validate_bot_slots`) accepts a slot whose
   `bot_name` matches an enabled `bots.name` **case-insensitively**
   (`n.eq_ignore_ascii_case(&slot.bot_name)`), and does not normalise it.
2. `rust/web/src/db/game_write.rs:213-219` stores it **verbatim**:
   `INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3)`, with
   `bot_name` cloned unmodified from the user-supplied
   `BotSlot.bot_name` (`rust/web/src/game/server_fns.rs:11`,
   `game_write.rs:123`).
3. `rust/bot/src/config.rs:24-28` (`load_bot_config`) resolves it with
   `WHERE name = $1 AND enabled = true` - **case-sensitive**. It yields `None`,
   and `rust/bot/src/main.rs:187-190` logs
   `reason = "bot not found or disabled"` and acks. The bot never moves.
4. D-5 declares "a bot player name resolving to nothing" an explicitly
   **supported no-op**, so nothing retries and nothing alerts.
5. `rust/web/src/email/sweep.rs:369-384` (the WP-38 reconciliation sweep) joins
   `LEFT JOIN bots b ON gb.bot_name = b.name` - also case-sensitive - so the row
   is classified `is_dangling` and the sweep **deliberately does not
   re-publish** for dangling names. The one mechanism designed to unwedge stuck
   bot turns is the mechanism that skips this game.
6. `rust/web/src/admin.rs:194-202` (`list_dangling_bot_names`) uses the same
   case-sensitive `b.name = gb.bot_name`, so the admin page reports `"EASY"` as a
   dangling bot type while `easy` sits enabled in the table - a warning that
   reads like a false positive and gives the admin nothing actionable.

Impact: a single crafted `create_game` server-fn call with
`bot_slots[].bot_name = "EASY"` creates a game that is unfinished forever, with
the bot permanently on turn, no `bot_turn_wedge_total`, no `max_deliveries`
advisory, and no sweep recovery. This is reachable by any authenticated user, not
just an admin - `admin.rs:226-228`'s own reasoning ("the server fns are a public
surface and a crafted call otherwise stores ...") applies verbatim here.

Aggravating: `db/bots.rs:251-258` (`validate_bot_slots_accepts_case_mismatch`)
is a test that **blesses** the case mismatch as accepted input, so the
divergence is pinned as intended behaviour on the validation side while every
consumer disagrees. Another instance of the family in systemic pattern 4b.

Remediation: make the validator canonical - have `validate_bot_slots` return the
matched `bots.name` and store that, so `game_bots.bot_name` is always a byte-exact
`bots.name`. (Rejecting a case mismatch outright also closes it, but silently
normalising is friendlier and matches the test's stated intent.) Failing that,
make all four lookups `lower(...)`-normalised together - but a single canonical
write point is the simpler fix.

### F-105 (Medium) - `bot.turn` has no idempotency key, so a duplicated trigger amplifies into up to four LLM calls and four command attempts per turn

`rust/web/src/game/mod.rs:200-255` (`publish_bot_turns`), `:182-194`
(`trigger_bot_turns`), `rust/web/src/email/sweep.rs` (the WP-38 sweep).

There are now three independent publishers of `bot.turn` for the same
(game, position): `broadcast_and_trigger` after every command
(`game/mod.rs:51-59`), the conflict/user-error re-publish inside
`handle_bot_command_event`, and the 15-minute reconciliation sweep. Confirmed by
grep, **no JetStream deduplication is used anywhere**: `publish_with_headers`
carries only Sentry trace headers (`game/mod.rs:227-236`); there is no
`Nats-Msg-Id` and no `expected_last_subject_sequence` in `rust/web/src` or
`rust/bot/src`.

So two `bot.turn` events for one turn produce two LLM completions and two
`bot.command` events. The second loses the `updated_at` race, comes back as
`Conflict`, and `handle_bot_command_event` re-publishes it at `attempt + 1` -
another LLM call - repeating up to `MAX_TURN_ATTEMPTS`. The WP-38 spec's
justification for the sweep ("Re-publishing is safe: a duplicate `bot.turn` for
an in-flight turn is caught by the bot's own DB re-check and the
`is_turn`/`updated_at` guards") is correct about *state* safety and silent about
*cost*: the guards fire after the completion has been paid for, and the conflict
path then deliberately spends three more.

Remediation: set `Nats-Msg-Id` to `{game_id}:{position}:{games.updated_at}` on
every `bot.turn` publish and give the BOT stream a `duplicate_window` of a few
minutes in `ensure_stream_and_consumers`. All three publishers then collapse onto
one message per actual turn, for free, with no change to the guards.

### F-106 (Low) - `read_capped_body` discards the transport error entirely

`rust/web/src/admin.rs:787-813`.

`Err(_) => return String::from_utf8_lossy(&buf).into_owned() + "\n<error reading body>"`
throws away the `reqwest::Error`. The admin gets the string `<error reading
body>` and the server logs nothing, so "connection reset mid-body" and "TLS
error" and "timeout" are indistinguishable in the one tool whose entire purpose
is diagnosing a misconfigured provider. Every other failure in this file gets an
`internal(context)` breadcrumb.

Remediation: `Err(e) => { tracing::warn!("admin test body read failed: {e}"); ... }`,
and include the error in the returned text - it is admin-only output.

### F-107 (Low) - `MAX_DELIVER`'s doc comment still says the term ceiling is future work, after WP-38 implemented it

`rust/web/src/nats.rs:21-25`: "Shared by the consumer config and the (future)
term ceiling so the two cannot drift (WP-38)."

WP-38 landed the ceiling: `rust/web/src/game/mod.rs:330` compares
`info.delivered >= crate::nats::MAX_DELIVER` and Terms. Similarly
`rust/web/src/nats.rs:181-208` still says the stranded-message case awaits "a
future recovery mechanism (WP-38/D-5)" and `:208` says "recovery (term/DLQ/
re-publish) is WP-38/D-5" - all three now describe shipped code as pending.
Cosmetic, but these are the comments a future reader consults before touching the
ack semantics, and they currently understate what depends on `MAX_DELIVER`.

### F-108 (Medium) - the web/bot wire contract is a copy-pasted struct pair in two crates, and WP-38's "so the two cannot drift" guarantee stops at the crate boundary

`rust/bot/src/nats.rs` (35 lines) vs `rust/web/src/nats.rs` (413).

The bot crate re-declares, rather than imports, everything it shares with the
monolith: `STREAM_NAME`, `SUBJECT_COMMAND`, `CONSUMER_TURN`, `connect()`, and -
critically - **both event structs**, `BotTurnEvent` and `BotCommandEvent`, field
for field. All five agree with the web copy today (verified value by value and
field by field), so this is latent, not live. Nothing makes them keep agreeing.
There is no
shared crate, no `serde` round-trip test across the two definitions, and no
`#[serde(default)]` anywhere, so adding a field on the publishing side and
forgetting the consuming side is a deserialization failure at runtime, in a
consumer whose only recovery is `MAX_DELIVER` redeliveries and a Term.

This is narrower than F-90's `crypto.rs` case: `MAX_DELIVER`, `MAX_TURN_ATTEMPTS`,
`SUBJECT_TURN` and `CONSUMER_COMMAND` are **not** duplicated (the bot needs none of
them), and `rust/bot/src/config.rs` turns out **not** to be a duplicate of
`rust/web/src/config.rs` at all - they share only a filename. The exposure is the
protocol types.

It nonetheless undercuts the reasoning WP-38's spec used for `MAX_DELIVER`
("Export `pub const MAX_DELIVER` from `rust/web/src/nats.rs` and use it in
`ensure_stream_and_consumers` too so the two cannot drift"). That works inside the
`web` crate. Across the crate boundary, where the actual protocol lives, the
remediation programme never looked - the same blind spot that let
`rust/bot/src/crypto.rs` diverge unnoticed (F-90). Note also that the invariant
`BotCommandEvent::attempt` exists to enforce - "echoes `BotTurnEvent::attempt`
from the `bot.turn` event this command resulted from" - is documented **only** in
the web copy (`rust/web/src/nats.rs:40-44`); the bot copy, which is the side that
must actually do the echoing, has a bare `pub attempt: i32`.

Remediation: extract a `brdgme-bot-proto` (or reuse an existing shared lib crate)
holding the subjects, consumer names, `MAX_DELIVER`, `MAX_TURN_ATTEMPTS` and both
event structs, and have both `rust/web` and `rust/bot` depend on it. This is the
same fix F-90 needs for `crypto.rs`, so do them together.

### F-109 (High) - WP-36's ws F55 fix and the regression test that pinned it were both deleted by a later commit in the same remediation programme

`13a1e693` (WP-36) implemented ws F55 properly. Per the commit and its diff, it
added to `rust/web/src/websocket.rs`: a `CancellationToken` **and a
`TaskTracker`** on `GameBroadcaster`, `begin_shutdown()` cancelling the token and
closing the tracker, `drain_ws_tasks()` awaiting `ws_tasks.wait()`,
`ws_handler` registering every connection future via `tracker.track_future(...)`,
and a per-connection `select!` that sends `Message::Close(None)` before breaking.
It also added `rust/web/tests/websocket_hygiene.rs` with
`shutdown_sends_close_frame_to_connected_websockets` - a real client that
connects, triggers `begin_shutdown()`, and fails if no `Message::Close` arrives
within 5s. The commit message describes it as "send WS close frames on graceful
shutdown via CancellationToken/TaskTracker with **bounded 5s drain**".

A later commit in the same programme, `efad81f` (the SSE migration), deleted all
of it. In the current tree:

- `TaskTracker`, `drain_ws_tasks`, `ws_handler`, `handle_socket` and the
  `Message::Close` send are **gone** from `websocket.rs`. `begin_shutdown()`
  (`websocket.rs:78-80`) is now only `self.shutdown.cancel()`.
- `rust/web/tests/websocket_hygiene.rs` **no longer exists** (confirmed by
  `git log --follow`).
- `rust/web/src/main.rs:131-137` calls `broadcaster.begin_shutdown()` and returns.
  There is **no `drain_ws_tasks` equivalent and nothing awaits or bounds any
  client drain**. The 5s bound the commit message advertised is not present
  anywhere.

Worse, **ws F55 had two halves and only one was ever implemented.** The finding
also named the bot consumer and email sweep tasks as getting no shutdown signal.
That half was never fixed and is still open: `rust/web/src/main.rs:72-96` spawns
the `bot-command` and `max-deliveries-advisory` supervisors as bare
`tokio::spawn`s with the `JoinHandle` discarded, and `spawn_periodic_sweeps`
(`:97-103`) returns `()`, spawning its own untracked tasks. No `CancellationToken`
reaches any of them - the only token in the crate is `GameBroadcaster`'s, consumed
solely by `events.rs`. So SIGTERM kills the bot-command consumer mid
`execute_command` with no drain, which is precisely the mechanism behind F-101's
five-minute post-deploy bot stall.

Three problems, then, and the last is the worst.

The behavioural one is partly mitigated by accident: the SSE handlers do observe
the token (`events.rs:107-109`, `:175-177`), so cancelling it ends each stream and
lets axum's own `with_graceful_shutdown` complete. But the SSE tasks are detached
`tokio::spawn`s with no tracker, so nothing bounds them - a task blocked in
`client.subscribe(...)` (`events.rs:50`, `:147`) at the moment of shutdown is
neither cancelled nor waited for, and the explicit bound WP-36 shipped to make
that case deterministic is gone.

The process one: **a remediation fix and its dedicated regression test were both
removed by a subsequent commit in the same programme, and nothing noticed.** The
test existed precisely to make this regression loud, and deleting it alongside the
code silenced the alarm with the fire. The transport changed, so a WebSocket close
frame is genuinely no longer meaningful - but the *acceptance criterion* underneath
it ("in-flight client connections are drained, with a bound") survives the
transport change and was dropped instead of re-expressed for SSE.

This is a **new systemic pattern** for the process-fixes section, distinct from the
routing leak (a finding never picked up by its receiver) and from pattern 4b (a
test edited to agree with the code): **a landed, tested fix silently reverted by a
later commit in the same programme, with no mechanism tracking that a finding's
fix is still present.** Recommend the unified report add a cheap check - for every
closed finding, assert its citation or its test still exists at sign-off. Note
`13a1e693`'s WP-36 checklist row presumably still reads as closed.

Remediation: re-express the criterion for SSE. Put a `TaskTracker` back on
`GameBroadcaster`, register both `events.rs` spawns with it, and have `main.rs`
await `tracker.wait()` under a `tokio::time::timeout(Duration::from_secs(5), ...)`
after `begin_shutdown()`. Then add a test to
`rust/web/tests/` that asserts an open `/events` stream terminates within the
bound after `begin_shutdown()` - the SSE analogue of the deleted test, and the
thing that stops this happening a third time.

### F-110 (Low) - the WP-37 finding citations are the only sign-off trail, and they are load-bearing

Not a defect - recorded because it is the mechanism that made this review cheap.
`admin.rs` cites ws F18, F19, F20, F21, F22, F23, F24, F25, F26, F28, F29 and F31
at their fix sites. Checking the three that appeared uncited (ws F24, F32, F33)
against the recovered corpus showed **all three were in fact fixed**: F24 (test
result attributed to the wrong row via the `input()`/`value()` race) is fixed and
cited at `admin.rs:1688`; F32 (10 copies of
`action.value().get().is_some() && ... .unwrap()`) is fixed - 12 sites now use
`if let Some(result) = action.value().get()` and the `.unwrap()` shape is gone;
F33 is fixed - the local tuple alias is renamed `BotProviderTestRow`
(`admin.rs:918`), no longer shadowing the public `BotProviderRow` struct
(`admin.rs:105`).

So WP-37 delivered 14 of 14. Worth stating explicitly because it is the only work
package in Unit 05 that did, and because F-109 shows what happens where no such
trail exists.

## Verified good

These were checked against the recovered acceptance criteria and hold up. Several
are notably better than the session average.

**Admin authorization (WP-37 ws F28) - clean, and self-enforcing.**
All 16 `#[server(...)]` fns in `admin.rs` fetch the pool and call
`require_admin(&pool, "<context>")` as their first act, verified line by line.
`require_admin` (`admin.rs:46-57`) is correctly two-stage: authenticate via
`get_current_user()`, then a *separate* `db::is_user_admin(pool, user.id)` check,
fail-closed on either. The admin bit is read from the database per request, never
from a request parameter, cookie or client-supplied field - so there is no
confused-deputy surface here and no privilege escalation via a supplied id.
Better still, `admin.rs:2991-3007` (`every_admin_server_fn_calls_require_admin`)
is a **source-level self-check** asserting `#[server(` count == `require_admin`
count == 16, so adding an ungated admin route breaks the build's test run. That is
the right shape for this class of invariant and the only instance of it seen so
far in this session.

**WP-38 (`914aa0c6`) is the most completely delivered work package reviewed in
Unit 05.** Every item of the recovered spec's section 3 is present:
- 3a: `email/sweep.rs` reconciliation sweep with `DEFAULT_BOT_TURN_THRESHOLD` /
  `DEFAULT_BOT_TURN_SWEEP_INTERVAL` (900s), `BOT_TURN_THRESHOLD` /
  `BOT_TURN_SWEEP_INTERVAL` env overrides, `MissedTickBehavior::Skip`, registered
  in `spawn_periodic_sweeps`, `bot_turn_sweep_republished_total` counter and
  `bot_turn_dangling_bot_names` gauge, re-publishing through
  `crate::game::publish_bot_turns` rather than a duplicated body. The candidate
  SQL correctly restricts to unfinished games, bot players
  (`gp.game_bot_id IS NOT NULL`), `is_turn_at` past the threshold, and partitions
  dangling via `LEFT JOIN bots ... (b.id IS NULL OR b.enabled = false)`.
- 3b: the `UserError` arm takes the same bounded path as `Conflict`,
  `bot_turn_wedge_total` on both exhaustion branches, both `publish_bot_turns`
  failure arms and both `find_bot_turns` failure arms raised to `error!` +
  `bot_turn_publish_failures_total`, `AckKind::Term` at the ceiling with
  `bot_command_terminated_total`, `MAX_DELIVER` exported and used in both places.
  Note the publish path also awaits the inner persistence ack
  (`game/mod.rs:242-248`), which is the part most implementations get wrong.
- 3c: `ACK_HEARTBEAT_INTERVAL = 60s` (`rust/bot/src/main.rs:33`), heartbeat
  spawned before `run_bot_turn` and `abort()`ed after (`:935-949`), a unit test
  asserting the cadence is below the 5-minute `ack_wait` (`:1039-1042`), and the
  dangling-bot skip log raised to `warn!` (`:187-190`).
- 3d: `list_dangling_bot_names` with the subtle `EXISTS (SELECT 1 FROM bots)`
  guard so an *empty* `bots` table never warns - the D-5 requirement that is easy
  to miss and that a plain `NOT EXISTS` would have got wrong.
- Section 5's tests are all present and assert the prescribed things, including
  `user_error_republishes_bot_turn_with_incremented_attempt` (exactly one event,
  right position, `attempt == 1`) and `user_error_attempt_limit_exhaustion_gives_up`
  (nothing published at the cap), plus
  `conflict_republish_targets_only_the_conflicting_bot` and
  `bot_command_delivered_exactly_once_across_two_fetchers`.

**WP-39 supervision (`347970a0`).** `supervise_consumer`
(`rust/web/src/nats.rs:258-297`) is correct on the point that matters: each run is
`tokio::spawn`ed so a **panic surfaces as a `JoinError` and is restarted** rather
than being swallowed by a dropped handle - the failure mode named in the brief.
Exponential 1s..30s backoff, reset only after a run that survived 60s, an
`error!` and a `nats_consumer_restarts_total{consumer}` counter on every restart
including the clean-`Ok` case. `supervisor_restarts_on_err_ok_and_panic` exercises
all three death modes and `supervisor_keeps_retrying_under_persistent_failure`
pins 20 consecutive failures, both on `start_paused` time so they are fast and
deterministic.

**`reorder_bots` (`admin.rs:383-438`) is the strongest single function in the
unit.** Duplicate ids rejected up front with a stated reason (Postgres applies one
of two matching ordinals nondeterministically); one `UPDATE ... FROM unnest($1)
WITH ORDINALITY` statement so a partial renumber is impossible; a transaction-scoped
`pg_advisory_xact_lock` shared with `create_bot`'s `MAX(display_order)+1` read,
with the reason (no unique constraint on the column) recorded; and
`rows_affected != distinct.len()` rolls back rather than reporting success for a
stale list. Tests cover zero-based renumbering, the unknown-id rollback, the
duplicate rejection and `create_bot` uniqueness.

**Other WP-37 items verified:** `mask_api_key` never fabricates a vendor prefix
and reveals nothing below 8 characters (ws F20); `ApiKeyUpdate` makes
Keep/Set/Clear all representable (ws F21); `test_provider` refuses to invent a
model id and resolves from `bot_providers` or errors with an actionable message
(ws F22); `read_capped_body` + `TEST_HEADER_ALLOWLIST` bound a hostile upstream's
response (ws F23) and are tested for truncation, header filtering and the
small-body pass-through; `list_providers` degrades a single undecryptable row
instead of failing the page, with the `load_key` failure deliberately left fatal
(ws F26) - and that degradation is tested; deletes stay idempotent while updates
report not-found via `rows_affected` (ws F29); the `ADMIN_REQUIRED` constant is
matched against the structured `ServerFnError::ServerError` variant rather than a
`to_string()` comparison, with the reason recorded (ws F31).

**F-91 (AAD-less ciphertexts) checked concretely at these call sites and is not
exploitable here.** All `crypto::encrypt`/`decrypt` use in `admin.rs` is confined
to one column, `llm_providers.api_key_encrypted`
(`:481-502`, `:524-533`, `:568-586`, `:864-872`, `:939-947`). There is no path by
which a caller supplies ciphertext - the admin UI only accepts plaintext - so the
relocation primitive F-91 describes needs prior database write access, which is
already game over. `load_key()` is called per request rather than cached, which is
slightly wasteful but leaks nothing.

**`db/mod.rs` doc comment.** The `updated_at` trigger convention was preserved
verbatim per WP-82 3f, including which 14 tables have the BEFORE UPDATE trigger,
which later tables need manual maintenance, and the three *conditional* triggers
that are not substitutes for an explicit write. This is genuinely load-bearing
documentation and it survived the split intact.

**`db/bots.rs` tests** are real behavioural tests, not reachability smoke:
`bot_lookups_respect_enabled_and_can_replace_humans` walks the seeded baseline,
the disabled-but-flagged case, the enabled-and-flagged case, and the
`display_order` ordering, and correctly accounts for the three bots that
`migrations/013` seeds into every test database. Contrast F-99.

**Both SSE handlers observe the shutdown token** - `events.rs:107-109` and
`:175-177` each select on `shutdown.cancelled()` and break, dropping the sender so
the `UnboundedReceiverStream` ends and the SSE response completes. This is what
lets `axum::serve(...).with_graceful_shutdown` (`main.rs:131-137`) finish at all,
since an SSE stream is otherwise an in-flight request that never completes. Having
*both* handlers covered rather than one is the part that is easy to get half-right.
See F-109 for what is missing around it.

**WP-82's `db.rs` split (`4d31f6eb`, +8312/-8149) is a genuine pure move** - the
one large mechanical diff in this unit, checked on the assumption it was guilty,
and it holds up on every axis the spec named:
- **No visibility widening.** All 21 checked symbols are correct. Items private in
  the old `db.rs` became exactly `pub(crate)` (`build_user_from_row`,
  `build_game_bot_from_row`, `build_game_type_user`,
  `build_game_player_from_row`, `choose_colors`, `apply_rating_changes`,
  `write_ranked_placings`); `rating` is additionally double-capped by
  `pub(crate) use rating::*` in `mod.rs`; `LocPref`, `remove_highest_prefs` and
  `PlayerSlotInternal` stayed fully private (a `pub use mod::*` glob cannot
  re-export an item with no visibility modifier); and everything reachable as
  `pub crate::db::X` today - including `pick_replacement_bot`, all the `_tx`/`_conn`
  helpers, `generate_unique_username` and `cap_digest` - was already `pub` before.
- **No SQL changed.** `rust/web/.sqlx/` (137 cached entries, keyed by the SHA-256
  of the query text) was **not touched by the commit**, which is conclusive: any
  edit to a `sqlx::query!` string would have forced a re-prepare. A 193-line
  spot-check of `create_game_with_users_tx` is byte-identical old vs new.
- **No test lost or added.** 128 test fns before, 128 after, identical sets, when
  compared at the commit boundary. (Comparing against current `HEAD` instead shows
  24 extra tests - all added by *later* commits, not by the split. Worth noting for
  anyone re-running this check.)
- **Nothing outside `rust/web/src/db/` modified**, and the `updated_at` trigger
  documentation was preserved verbatim per 3f.

**WP-68 (`618156a7`) is completely clean** and needs no follow-up:
`terminal_size = "0.4"` replaces `term_size = "0.3.2"` in
`rust/lib/cmd/Cargo.toml`; the sole call site (`rust/lib/cmd/src/repl.rs:219`) is
`terminal_size::terminal_size().map_or(0, |(w, _)| w.0 as usize)`, which preserves
the non-tty `term_w == 0` no-padding behaviour exactly as the spec required; the
`RUSTSEC-2020-0163` ignore is gone from `rust/deny.toml` in the same commit, so
`cargo deny` neither flags the advisory nor warns about an unused ignore; and
`term_size` appears **nowhere** in the tree, `rust/Cargo.lock` included.

**`rust/bot/src/main.rs` is in better shape than the web side.** Zero
`unwrap()`/`unreachable!()`/`todo!()`/`unimplemented!()`. Three `expect()`, none in
a fallible or retry path (two are one-time startup signal-handler installs, one is
a post-condition guaranteed four lines above, one is a semaphore invariant on a
semaphore that is never closed). It also has real graceful shutdown -
`shutdown_signal()` on SIGTERM or Ctrl+C, pinned once and selected against at both
the message-receive loop and the turn-permit loop - which is more than
`rust/web/src/main.rs` does for its own background tasks (F-109).

## Coverage gaps

- **`rust/web/src/db/game_write.rs`** - the largest module produced by the WP-82
  split (`create_game_with_users_tx` ~205 lines, `update_game_command_success`
  ~115) was read only where the bot path touches it
  (`:213-219`, the `game_bots` insert). Its transaction boundaries, the
  `StaleStateConflict` optimistic-concurrency check, and `concede_game` /
  `end_game` / `delete_game` / `undo_game` belong to **Unit 06** (undo/concede
  integrity) and are not covered here. `update_game_command_success` in particular
  is the single write path every finding in this unit funnels through and it has
  had no line-level review in this session.
- **`admin.rs` lines 1560-3488** (the Leptos `#[component]` UI) were not read.
  The server-fn surface, all stratum-2 helpers, and the test module inventory were
  covered; the form components were not. Client-side validation there is
  irrelevant to security (the server fns validate independently, ws F25) but a
  rendering bug that shows one provider's masked key against another row would not
  have been caught.
- **`rust/bot/src/prompt.rs` (442 lines) and `routing.rs` (68 lines)** have no
  counterpart in `rust/web/src` and no owning sub-unit named them. `prompt.rs`
  constructs the LLM prompt from game state, so it is the natural place for a
  hidden-information leak into a third-party API - exactly the class Unit 02/03/04
  spent its budget on for `pub_state` and `Log::public`. Recommend Unit 10 give it
  a dedicated read.
- **`rust/web/src/email/sweep.rs`** was verified only for the WP-38 bot-turn sweep
  (consts, registration, candidate SQL, metrics). Its other sweeps and the
  `spawn_sweep` helper are WP-46's and are not reviewed here.
- **No test exercises `require_admin`'s `true` path against a real admin user
  for more than three of the sixteen server fns.** The rejection side is well
  covered (`test_admin_list_bots_rejects_non_admin`,
  `test_admin_list_dangling_bot_names_rejects_non_admin`, plus the source-level
  self-check), which is the right priority; noting the asymmetry only because
  F-99 shows `is_user_admin(.. ) == true` is untested at the db layer too.

## Carry-forwards for Units 06-11

- **Unit 09 (SSE):** `events_public_handler` (`rust/web/src/events.rs:117-183`) is
  **unauthenticated**, subscribes each connection to `game.>` (every game update
  in the system), and runs an **uncached** `db::is_game_publicly_visible` query
  per matching message - while the authenticated `events_handler` next to it uses
  `VisibilityCache` for exactly this. With up to 16 topics per connection, no
  connection cap, and no rate limiting anywhere in `rust/web` (F-94), that is an
  anonymous DB-amplification lever. It is also systemic pattern 2 - one handler
  hardened, its neighbour on the same path left raw - in a file neither 05a nor
  05b owns.
- **Unit 10 (`rust/bot`):** the duplicated-module sweep the brief asked for is
  done. Result: exactly **one** further duplicate beyond F-90's `crypto.rs`, and it
  has not yet diverged - `rust/bot/src/nats.rs` vs `rust/web/src/nats.rs`, see
  F-108. `rust/bot/src/config.rs` (209 lines) and `rust/web/src/config.rs` (6
  lines) share only a filename and have zero overlap; `prompt.rs` and `routing.rs`
  have no web counterpart. So Unit 10 does not need to re-run this sweep - it needs
  to act on F-108 and F-90 together.
- **Unit 06:** `db::pick_replacement_bot` (`rust/web/src/db/bots.rs:76-98`) takes
  `&PgPool`, not a transaction, and performs a `SELECT` then an
  `INSERT INTO game_bots` as two separate autocommit statements. Any caller in the
  concede/replacement flow that needs the bot creation to be atomic with the
  `game_players` update cannot get it from this signature. WP-82 correctly moved it
  verbatim, so this is pre-existing - but it lands in Unit 06's blast radius.
- **Unit 08:** `db::create_pool` takes every `PgPool` default (F-103); any
  query-performance conclusion about the monolith is bounded by an unconfigured
  10-connection pool.
- **Process fixes (systemic patterns):** F-99 and F-100 are two further instances
  of pattern 4b/4c, bringing the confirmed count to six. F-104 is a new variant
  worth naming separately: **a test that blesses the lenient half of an
  inconsistency** (`validate_bot_slots_accepts_case_mismatch`), which pins the bug
  as intended behaviour on one side of a boundary while every consumer on the
  other side disagrees.
- **New systemic pattern for the process-fixes section (from F-109):** a landed,
  tested fix silently reverted by a later commit in the same programme. This is
  distinct from the routing leak and from pattern 4b, and it is invisible to every
  check the programme currently runs, because the checklist row and the commit both
  still read as closed. Cheap mitigation: at sign-off, assert each closed finding's
  citation or its regression test still exists in the tree. Worth sweeping the other
  units for it - Unit 09 owns the SSE migration commit (`efad81f`) that caused this
  instance, so it is the most likely place to find a second.
- **WP-37 is the only work package in Unit 05 that delivered its full scope**
  (14 of 14). The thing that made that verifiable in a fraction of the budget was
  the file's discipline of citing the finding id at each fix site. Recommend the
  remediation plan mandate it.
