# T3-B6: outbound email/render + websocket pass

- **Batch**: T3-B6 = WP-60 (outbound tokens, metrics, render - 9 findings) +
  WP-42 (websocket pass - 4 findings)
- **Crate**: `rust/web` (single crate; two packages, disjoint file sets)
- **Sources**:
  - `findings/web-frontend-email.md` (`wfe Fnn`) for WP-60 - **no verification
    file exists** for web-frontend-email, so the raw findings file is
    authoritative.
  - `findings/web-server.md` (`ws Fnn`) for WP-42 - **`findings/verification/web-server.md`
    EXISTS and supersedes the raw file.** All four in-scope rows are CONFIRMED
    there (`ws F60` with evidence upgraded; no severity changes).
- **Numbering**: neither findings file carries inline ids, so `Fnn` = the nth
  `###` finding heading in that file. `wfe F44`-`wfe F51` are contiguous
  (`email/render.rs`, `email/outbound.rs`, `theme.rs` sections); `wfe F63` is the
  final `app.rs` nit. `ws F59`-`ws F62` are the four websocket findings. Raw and
  verification numbering for web-server are identical - no offset hazard.
- **Rows**: 12 (6 minor / 6 nit) - WP-60 contributes 9 (5m/4n), WP-42
  contributes 3 (1m/2n). `ws F59` (minor) is **not** a row: it is escalated, see
  `## Escalate`.
- No rows are decision-blocked. D-13 is ANSWERED (option B shape) so WP-42 is
  READY; WP-60 was always READY. No finding in this batch belongs to a
  `BLOCKED-ON-USER-RULES-REVIEW` package.
- Review-wide rejections were `d F13` and `ws F30`; neither is in scope. `ws F67`
  (UNVERIFIABLE dependency currency) belongs to WP-43, not here.
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by package then source file so one session sweeps a file at a
time.

## WP-60 - `web/src/email/outbound.rs`

`wfe F44` and `wfe F45` are two rows pointing at **one** rewrite of
`ensure_email_token`: land them together in a single edit.

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wfe F44` | `rust/web/src/email/outbound.rs` fn `ensure_email_token` | Replace SELECT-then-UPDATE with one atomic `UPDATE game_players SET email_token = COALESCE(email_token, $1), updated_at = NOW() WHERE id = $2 RETURNING email_token` so concurrent sends cannot each mint a token | y |
| `wfe F45` | `rust/web/src/email/outbound.rs` fn `ensure_email_token` | Same rewrite as `wfe F44`: `fetch_optional` returning `None` (nonexistent `game_player_id`) must be an error, not `Ok(token)` for a token that was never persisted | y |
| `wfe F46` | `rust/web/src/email/outbound.rs` fn `try_send_rendered_email` | Move the `game_emails_sent_total` increment onto the success arm and add `game_emails_failed_total` on the failure arm (or rename to `..._attempts_total` if attempt-counting is intended) | y |
| `wfe F50` | `rust/web/src/email/outbound.rs` fn `parse_duration` -> `rust/web/src/email/sweep.rs` | Move `parse_duration` to `sweep.rs` next to its five sweep-config call sites and update the imports | n |

## WP-60 - `web/src/email/render.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wfe F47` | `rust/web/src/email/render.rs` fn `render_game_email` (the `mrml::parse(&mjml).ok()` / `.render(...).ok()` pair) | Keep the `fallback_html` fallback but log the discarded error at `tracing::warn!` (error only, never the body) and/or bump a fallback counter | n |
| `wfe F48` | `rust/web/src/email/render.rs` fn `render_block` | Keep the empty-render fallback of `brdgme_markup::from_string(markup).unwrap_or_default()` but log a warning naming the block kind on parse failure | n |
| `wfe F49` | `rust/web/src/email/render.rs` fn `render_game_email` (the `browser_url` / `rules_url` `<a href=...>` interpolations) | Attribute-escape both URLs at interpolation, or document the trusted-URL precondition on `EmailContent` - finding is UNCERTAIN and self-reports no live injection path, so the doc-only option is acceptable | n |

## WP-60 - `web/src/theme.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wfe F51` | `rust/web/src/theme.rs` fn `random_pref_colors` | Replace the hand-rolled Fisher-Yates + modulo with `SliceRandom::shuffle(&mut rand::rng())` then truncate (rand 0.9 API in use); existing test fn `random_pref_colors_three_distinct_valid` must still pass | n |

## WP-60 - `web/src/app.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wfe F63` | `rust/web/src/app.rs` fn `js_string_escape` (called from fn `sentry_init_snippet`) | Also escape `<` (standard JSON-in-script hardening) or serialize via `serde_json::to_string`, so a DSN/release containing `</script>` cannot break out of the inline script | y |

## WP-42 - `web/src/websocket.rs`

**Sequencing**: land WP-47 (`is_game_visible_to_user`) before any WP-42
filtering work; WP-42 must **reuse** that predicate, never fork it. The three
rows below are mechanical and independent of that ordering and of the escalated
`ws F59`.

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `ws F61` | `rust/web/src/websocket.rs` fn `ws_handler` (the `ws.on_upgrade` call) | Set explicit small limits before upgrading, e.g. `ws.max_message_size(4 * 1024).max_frame_size(4 * 1024)`, instead of tungstenite's ~64 MiB/16 MiB defaults | n |
| `ws F62` | `rust/web/src/websocket.rs` fn `handle_socket` (the 30s ping arm) | Optional hardening: record the last Pong timestamp and close the socket once it is idle for more than 2-3 ping intervals | n |

## WP-42 - `web/src/websocket_client.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `ws F60` | `rust/web/src/websocket_client.rs` fn `use_websocket` (the `visibilitychange` and `online` listeners) | Bind `ready_state` instead of destructuring it as `_` and call `open()` only when `ready_state.get_untracked()` is `Closed` - the listeners are non-reactive, so `get_untracked` is required (verification note) | n |

## Decision-blocked rows

None. D-13 is ANSWERED (2026-07-25, option B shape: authenticate the upgrade +
server-side filtering, plus `sub`/`unsub` for public-game pages), which cleared
WP-42's READY-PENDING-CONFIRMATION flag; WP-60 carries no decision. No
`decisions-needed.md` entry gates any row above. The parked rules-review
packages (WP-11, WP-12, WP-16, WP-20, WP-26, WP-30) own no findings here.

The only cross-package constraint is a **sequencing** one, not a decision:
`ws F59`'s filtering work consumes WP-47's `is_game_visible_to_user`.

## Not in this checklist (owned elsewhere)

- `ws F55` (graceful shutdown does not cover WS connections or background
  tasks) - already shipped by `specs/WP-36-crypto-deploy-hardening.md`; the
  `GameBroadcaster::begin_shutdown` / `drain_ws_tasks` fns and `handle_socket`'s
  shutdown select arm exist in live source. Do not disturb them while doing
  `ws F61`/`ws F62`.
- All other `email/` work in `outbound.rs`'s **call sites**, `notify.rs`,
  `inbound.rs`, `commands.rs` and `sweep.rs` delivery semantics - owned by
  `specs/WP-51-invite-mailer-notify-dedup.md`,
  `specs/WP-59-inbound-processing-quality.md`, WP-46, WP-56, WP-57. Those specs
  explicitly disclaim `wfe F44`-`F51` and `wfe F63` to WP-60, so there is no
  contested row - but note WP-51 changes `notify.rs` to call
  `try_send_rendered_email`, so `wfe F46`'s metric fix lands on the fn WP-51
  newly depends on: coordinate if both are in flight.
- `wfe F52`-`wfe F62` (frontend UX / error-handling items in `app.rs`,
  `components/`) - owned by `specs/WP-54-frontend-ux-error-handling.md`. That
  spec conversely disclaims `wfe F63` (`js_string_escape`) to WP-60, so
  `js_string_escape`, `sentry_init_snippet` and `shell()` are **ours** and
  nothing else in `app.rs` is.
- `ws F63`-`ws F67` (`import_game`, cargo deps) - WP-43, batch T3-B7.
- `wd F17` / `wd F45` (`is_game_visible_to_user` itself) -
  `specs/WP-47-game-visibility-gates.md`. WP-42 calls the predicate; it does not
  define or modify it.

## Escalate

**`ws F59` (`/ws` has no authentication; every connection gets the site-wide
firehose) is not Tier 3 - WP-42 needs a compact, Tier 2-style spec** despite the
package containing no majors. Neither half compresses to one honest line:

- **Task A (do first)** is not a mechanical edit: add `Session` +
  `State<PgPool>` extractors to `ws_handler` and resolve identity **before**
  `ws.on_upgrade` (the connection is hijacked after, and the session layer's
  response-side save pass has already run), using
  `auth::session::get_user_from_session` + `validate_session_token` - **not**
  `get_current_user`, a `#[server]` fn whose leptos context does not cover the
  plain `/ws` route. No router or layer reordering is needed: `/ws` is already
  registered before `.layer(session_layer)`. It then has to thread that identity
  into `handle_socket` and filter every `game.>` / `proposal.>` frame through
  WP-47's `is_game_visible_to_user` - a per-frame async DB predicate whose
  caching/invalidation strategy is an open design question that a checklist row
  cannot carry.
- **Anonymous upgrades must keep returning 101, never 401.**
  `rust/web/tests/websocket_hygiene.rs` asserts that a cookie-less connect gets
  `101 Switching Protocols`. The shape is authenticate-if-session, degrade to
  public-only otherwise.
- **Task B (separable; must not block Task A)** is entirely new work: a client
  `sub {game_id}` / `unsub {game_id}` protocol for public-game pages. No vestige
  of the old brdg.me `sub`/`unsub` survives in `rust/`. It requires the server to
  start acting on inbound frames (`handle_socket` currently drains and discards
  them), per-socket subscription state, a client->server message enum, and the
  client to bind the `send` handle of `UseWebSocketReturn` that `use_websocket`
  currently drops.
- Note `ws F59` is **also a load fix, not only privacy**: `trigger.last_update`
  is bumped on every frame and keys the `active_games` and `public_index`
  resources, so today every site-wide event forces a server-fn refetch on every
  connected client.
- Filtering approach fixed by D-13: wildcard subscribe + per-socket membership
  filter, **not** per-user NATS fan-out subjects (which would force all eleven
  publish sites to learn the recipient set). `websocket.rs` tests assert `user.>`
  stays empty - keep that true.

Everything else in this batch compresses to one line. One caveat worth flagging
rather than escalating: `wfe F49`'s recommendation is the weakest in the batch
(the finding itself is UNCERTAIN and states no live injection path exists), so
documenting the trusted-URL precondition is a legitimate resolution.
