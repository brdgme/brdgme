# Unit 09 - Web frontend / email + SSE migration

Findings continue from F-158.

## Progress

- [x] Obligation 1: sweep `efad81f` -> **ONE instance only (F-109). Demonstrated, not asserted.** Plus F-163 near-miss.
- [x] Obligation 2: WP-56 DMARC soundness -> **NOT SOUND, F-161 (High); F-129 + F-130 escalate to account takeover**
- [x] Obligation 3: F-131 SSE authenticate-once consequence -> **F-158 (High)**
- [x] **Obligation 4 DISCHARGED: F-15 stays LATENT - no live violation at the
      real emitter.** `rust/web/style/main.scss` read in full (1,095 lines) and
      the whole tree swept. Every `--mk-soften-*` token referenced anywhere
      (`foreground-90` x21 in main.scss, `orange-86` x3, `foreground-96` x1) is
      covered by `IN_USE_SOFTENS` u `CHROME_SOFTENS`, so **no undefined CSS
      variable is emitted**. Game crates emit exactly
      `{(Pink,80),(Foreground,80),(Foreground,90)}` from three sites
      (`acquire-1/src/render.rs:19-20`, `:158-166`,
      `lords-of-vegas-1/src/render.rs:211-212`) - **identical to
      `IN_USE_SOFTENS`** - and no game emits a `mix` at all, so the empty
      `IN_USE_MIXES` is correct too. `main.scss` is the only stylesheet.
      Products: **F-164** (the one hardcoded literal colour) and **F-167** (a
      dead whitelist entry + a false doc comment). Do not re-derive.
- [x] `events_public_handler` anonymous DB amplification -> **F-160 (Medium)**
- [x] SSE task lifecycle -> **F-159 (Medium)**
- [x] Frontend commits WP-54/55, T3-B6, `7da90b2d`, `dec967b6` -> **F-165
      (Medium), F-166 (Medium), F-168 (Low)**. WP-54 and WP-55 are otherwise
      clean and neither spec has a `Test?` column; see "Verified good".
- [x] Email **WP-57 `65c22edc`, WP-58 `390dd3b8`+`5786a1b6`, WP-59 `f56ff375`**
      -> **F-169 (High), F-170 (Medium), F-171 (Medium), F-172 (Low),
      F-173 (Low), F-174 (Low)**. Worker detail:
      `/tmp/claude-1000/-home-beefsack-Development-brdgme/8c01a959-a4bf-4e7b-9bcf-e39c82461037/scratchpad/w3-email-a.md`
- [x] **WP-60 `e5513ec6`** (outbound tokens / metrics / render hardening) ->
      **F-175 (Medium), F-176 (Medium), F-177 (Low), F-178 (Low)**. **No WP-60
      spec exists**; criteria are the WP-60 rows of
      `checklists/T3-B6-outbound-email-websocket.md` (`wfe F44`-`F51`, `F63`).
      All nine rows landed except `wfe F49`, which is half-applied. The token
      pattern-2 hypothesis in 09b's brief is **REFUTED** - see "Verified good".
      Worker detail:
      `/tmp/claude-1000/-home-beefsack-Development-brdgme/8c01a959-a4bf-4e7b-9bcf-e39c82461037/scratchpad/w1-wp60.md`
- [x] **WP-76 `bc051164` + `ca7925bc`** (notify wiring, game-start paths) ->
      **F-179 (Medium), F-180 (Low), F-181 (Low), F-182 (Low)**. **WP-76 has no
      spec and no row in any of the eight `T3-B*` checklists** (all eight
      grepped) - `EXECUTION-README.md:408` records the omission as deliberate.
      Pattern 2 and pattern 4e both **REFUTED** with enumeration; the
      `RouteOutcome` sweep of `inbound.rs` is **closed** with no third defect.
      Worker detail:
      `/tmp/claude-1000/-home-beefsack-Development-brdgme/8c01a959-a4bf-4e7b-9bcf-e39c82461037/scratchpad/w2-wp76.md`
- [x] **WP-77 `33150afe`** (default `bot_name`) -> **F-183 (High), F-184 (Low),
      F-185 (Low)**. **No spec, and no WP-77 row in any of the eight `T3-B*`
      checklists** (all eight grepped) - no `Test?` column exists. The
      F-104/F-138 "fifth write path" hypothesis is **REFUTED** for WP-77's own
      default, but enumerating all six bot-name write sites found the real one
      (F-183, High). Worker detail:
      `/tmp/claude-1000/-home-beefsack-Development-brdgme/8c01a959-a4bf-4e7b-9bcf-e39c82461037/scratchpad/w3-wp77.md`
- [x] **WP-59 Tasks 9-14 ownership question SETTLED: no coverage hole.**
      `f56ff37` owns Tasks 9, 11, 12, 13; Task 10 was dissolved by WP-56
      (`da1ea24`); Task 14 is a deliberate non-implementation per the spec's own
      carve-out to WP-85. Detail in "Verified good". Do not re-open.

**Unit 09 is CLOSED.** All four 09c commits opened, all obligations discharged.
- [x] **The `ssr` feature-gate question: SETTLED, and it is REFUTED.** The
      ssr-gated tests **DO run**. See "Verified good" below. **423 gated test
      functions across 25 modules are live, not skipped. No "Test? y" row is
      retro-voided by this.** Do not re-derive.

## Findings

### F-158 (High) SSE authenticates once at connect; a revoked session keeps streaming private events for the life of the connection (obligation 3 / F-131 concretised)

`rust/web/src/events.rs:33-41` (capture), `:47-112` (the spawned loop)

`events_handler` resolves the viewer **once**, before spawning:

```rust
let viewer: Option<Uuid> = match get_user_from_session(&session).await {
    Some(su) => match validate_session_token(&pool, su.auth_token_id).await {
        Ok(true) => Some(su.id),
        _ => None,
    },
    None => None,
};
```

`viewer` is then moved into the `tokio::spawn` at `:47` and consulted on every
message (`:81`, `:96`, `:100`) for the entire life of the task. Nothing
re-runs `validate_session_token`.

Concrete consequences, now that `VisibilityCache` has been read
(`rust/web/src/visibility_cache.rs:7-8`, `TTL = 30s`, `MAX_ENTRIES = 256`):

- **Visibility changes: bounded staleness, ~30s.** A game turning private, or a
  viewer being removed from a game, is re-looked-up after the entry's 30s TTL
  expires. This half is acceptable and should be recorded as such.
- **Session revocation: unbounded.** Logout, token revocation, password change
  or admin lockout have **no effect at all** on an already-open SSE stream. The
  connection continues delivering every event for every game that user could see
  at connect time. SSE `KeepAlive::default()` (`:114`) is specifically designed
  to hold the connection open indefinitely, and no maximum connection lifetime
  is imposed anywhere in the handler, so "for how long" is: until the client
  chooses to disconnect, NATS drops the subscription, or the process shuts down.
  A stolen-then-revoked session token is therefore only revoked for HTTP
  requests, not for the live event feed.

This is the intended reading of F-131, and it is worse than "stale by a cache
TTL" - the two staleness windows have completely different bounds and only one
of them is bounded.

Suggested fix: re-run `validate_session_token` on a timer inside the loop (a
`tokio::time::interval` arm in the existing `select!` is a three-line change,
and the 30s `VisibilityCache` TTL is the natural period), breaking the loop when
it returns anything but `Ok(true)`. Alternatively impose a maximum SSE
connection lifetime and force the client to reconnect (and re-authenticate).

### F-159 (Medium) Both SSE tasks leak past client disconnect - and the `sse_connections` gauge counts the leak as live connections

`rust/web/src/events.rs:47-112` and `:144-180`

The only exit conditions for either spawned task are: the NATS subscription
ending (`:72`, `:90`, `:160`), global shutdown (`:107`, `:175`), or
`tx.send(..).is_err()` (`:82`, `:101`, `:170`).

`tx.send` is only *reached* when a message has already passed the visibility
gate. So when a client disconnects, the task does not notice until the next
event **that the disconnected viewer was allowed to see** arrives. Until then
the task stays alive holding two NATS subscriptions (`game.>` and
`proposal.>`), decoding every payload (`:74`, `:92`) and running
`cache.check_game` for every game event in the entire system (`:81`).

For a viewer with no visible games - an idle account, or the anonymous
`viewer: None` case - `tx.send` is **never** reached, so the task never
terminates. Each such connection is a permanent leak until process shutdown.

Because `SseConnectionGuard` (`:13-26`) decrements only on `Drop` of the
spawned task's local, the `sse_connections` gauge reports these leaked tasks as
live connections. The metric intended to observe connection hygiene is exactly
the metric that hides this.

Note the interaction with F-109: `efad81f` deleted WP-36's ws F55 shutdown
drain, and the replacement path has a *new* lifecycle hole of the same family.

Suggested fix: pass the connection's cancellation signal into the task. Axum
does not give the handler a disconnect future directly, but wrapping the
returned stream in a guard whose `Drop` fires a `CancellationToken` (and
selecting on that token in the loop) is the standard pattern and also fixes the
gauge.

### F-160 (Medium) `events_public_handler` is unauthenticated, subscribes to `game.>`, and runs an uncached visibility query per matching message - while its sibling ten lines up uses `VisibilityCache` (pattern 2)

`rust/web/src/events.rs:117-183`, specifically `:147` and `:169`

```rust
let mut game_sub = match client.subscribe("game.>").await { ... };   // :147
...
&& crate::db::is_game_publicly_visible(&pool, game_id).await.unwrap_or(false)  // :169
```

Three separate problems, all in the unauthenticated handler:

1. **No `VisibilityCache`.** `events_handler` at `:65` creates a cache and
   routes every check through it; `events_public_handler` calls the DB
   directly. Byte-for-byte the same concern, one site hardened and its sibling
   in the same file left raw - the confirmed pattern 2. One DB round trip per
   message per connection, for up to 16 subscribed game ids, with no
   deduplication across connections either.
2. **`game.>` rather than the 16 known subjects.** The handler has parsed the
   exact `requested_ids` set at `:123-135` before subscribing, yet subscribes to
   the firehose and filters in-process at `:168`. Every anonymous connection
   therefore receives, decodes (`:162`) and discards every game event in the
   system.
3. **No authentication and no rate limiting.** F-94 is confirmed: there is no
   rate-limiting middleware anywhere in `rust/web`, and the two doc comments
   asserting a per-IP limit are not to be trusted. Nothing bounds the number of
   concurrent anonymous SSE connections, so 1 and 2 multiply by attacker-chosen
   N. Combined with F-159, the connections also do not reliably go away.

The `requested_ids.contains(&game_id)` guard at `:168` does correctly precede
the DB call in the `&&` chain, so the query fires only for requested games -
worth recording, because it means the DB amplification factor is
(16 x event rate x connections), not (all games x connections). Problem 2
still applies to the decode path regardless.

`unwrap_or(false)` at `:169` fails closed, matching `VisibilityCache`'s own
error policy - that part is correct.

Suggested fix: give the public handler a `VisibilityCache` (it is already a
per-task local, so this is a two-line change), subscribe to the specific
`game.{id}` subjects it parsed, and put a connection limit / rate limit in
front of the route.

### F-161 (High, and it escalates F-129 + F-130) WP-56's inbound auth gate is fail-open in three independent ways - the DMARC classification Unit 07 relied on does not hold up

`rust/web/src/email/inbound.rs:164-219` (`classify_inbound_auth`),
`:704-730` (`fetch_inbound_text`), `:532-545` (`from_matches_verified_email`),
`:772-782` and `:873-880` (the two callers read so far).

**This is obligation 2, and the answer is: not sound.**

The whole inbound trust model is: the `From` header is attacker-controlled at
SMTP level, so `from_matches_verified_email` (a plain
`LOWER(email) = LOWER($2)` lookup against `user_emails.verified_at IS NOT NULL`,
`:537-543`) is only a real authenticator if something upstream proves the `From`
was not spoofed. That something is `classify_inbound_auth`. It does not prove it.

**(a) `Unknown` proceeds.** `fetch_inbound_text:719-723`:

```rust
AuthVerdict::Unknown => {
    tracing::warn!("resend webhook: inbound auth unknown; proceeding from={from} ...");
}
```

`Unknown` is returned when the message cannot be parsed (`:167`), when there is
**no** `Authentication-Results` header (`:177`), when its value is not text
(`:182`), and - the important one - when the authserv-id is not exactly
`amazonses.com` (`:187-189`). The receiving pipeline in this file is **Resend**
(`ResendInbound`, `https://api.resend.com/emails/receiving/{email_id}`,
`:243-263`); the hardcoded `amazonses.com` authserv-id is an unverified
assumption about what Resend's MTA stamps. If it stamps anything else, **every
inbound email in production classifies `Unknown` and the entire gate is inert**,
with a `warn!` line as the only symptom. Nothing tests `classify_inbound_auth`
against a captured real Resend message, and no metric or alert fires on
`Unknown`.

**(b) `Pass` means "not explicitly failed", not "authenticated".** `:213-218`:

```rust
let failed = |r: &Option<String>| r.as_deref() == Some("fail");
if failed(&dmarc) || (failed(&spf) && failed(&dkim)) {
    AuthVerdict::Fail
} else {
    AuthVerdict::Pass
}
```

Every one of these yields `Pass`:

| `Authentication-Results` content | verdict | why it matters |
|---|---|---|
| `dmarc=none` | Pass | the victim's domain publishes no DMARC record - the common case outside major providers |
| `spf=softfail; dkim=none` | Pass | `~all` softfail is the single most common "probably forged" signal |
| `spf=neutral` / `spf=none` / `spf=permerror` / `spf=temperror` | Pass | no assertion of any kind is treated as a pass |
| **`spf=fail; dkim=none`, no `dmarc` method reported** | **Pass** | an outright SPF hard-fail, which is a DMARC fail, is accepted because the `&&` requires DKIM to *also* say `fail`, and a message with no DKIM signature reports `dkim=none` |

The last row is the cleanest: it is an unconditional forgery path derivable from
this file alone, needing no assumption about the deployment. `spf=fail` with no
DKIM signature is precisely the profile of a spoofed sender, and it is accepted.
Requiring *both* SPF and DKIM to say `fail` inverts the DMARC rule, which is that
a message passes if *either* aligns and fails if neither does.

**(c) The topmost-header defence is only sound if the trusted stamp is
guaranteed present.** `:170-178` correctly comments "lower ones may be forged"
and takes the first matching header. But because `Unknown` proceeds (a), a
message where the trusted MTA stamped nothing leaves the attacker's own
`Authentication-Results: amazonses.com; spf=pass; dkim=pass; dmarc=pass` as the
only such header, and it is honoured verbatim. The "take the topmost" rule
defends against an attacker adding a *second* header; it does not defend against
an attacker supplying the *only* header.

**(d) The two tests that appear to cover the lenient boundary are decoys -
both inputs contain an independently passing result.** `inbound.rs:1794-1808`:

```rust
fn classify_inbound_auth_softfail_is_not_fail() {
    "... spf=softfail ...; dkim=pass ...; dmarc=none ..."   -> Pass
}
fn classify_inbound_auth_single_fail_is_not_fail() {
    "... spf=fail ...; dkim=pass ...; dmarc=pass ..."       -> Pass
}
```

Both names promise exactly the risk in (b), and both verdicts are **defensible
on their own inputs**: the first has `dkim=pass`, the second has `dkim=pass`
*and* `dmarc=pass`. A reviewer grepping for "is the lenient case covered" finds
two tests with the right names and moves on. The inputs where **nothing
authenticated** are absent from the suite entirely:

- `amazonses.com; spf=fail; dkim=none` (no `dmarc` method reported) -> **Pass**
- `amazonses.com; spf=none; dkim=none; dmarc=none` -> **Pass**

Neither is tested and both are accepted. This is the F-151 decoy family
(a test that name-matches the risk without exercising it) crossed with pattern
4f (a test blessing the lenient half of the behaviour).

Recorded as clean, because they are: `classify_inbound_auth_ignores_injected_lower_header`
(`:1785-1792`) is a real, correct test of the topmost-header rule, and
`classify_inbound_auth_wrong_authserv_id` (`:1777-1783`) correctly pins the
`Unknown` verdict for a foreign authserv-id - it just does not follow through to
assert what the *caller* then does with `Unknown`, which is proceed.

**Consequence for F-129/F-130.** Unit 07 rated the
`find_user_by_settings_token` mechanism (`:520-530`) Medium *entirely* on the
assumption that this gate holds. It does not. Given a settings token, an
attacker who spoofs `From: victim@example.com` reaches
`handle_settings_reply_route` with `from_matches_verified_email` returning true,
under any of (a), (b) or (c). **F-129 + F-130 escalate to account takeover** per
the condition Unit 07 itself set out. Note the settings token is a bare
`SELECT id FROM users WHERE settings_email_token = $1` (`:525`) with no
expiry, no single-use, and no rate limit (F-94).

Suggested fix, in order of value:
1. Require an affirmative result, not the absence of `fail`:
   `dmarc == Some("pass") || (spf == Some("pass") || dkim == Some("pass"))` -
   and decide deliberately which of those is sufficient, rather than inheriting
   it from a negation.
2. Treat `Unknown` as `Fail` on the **settings** route at minimum (the one that
   mutates account state); it is the route where fail-open is account takeover.
3. Verify the real authserv-id against a captured production message and add a
   test using that message verbatim; emit a counter on `Unknown` so a pipeline
   change is visible rather than silent.
4. Make settings tokens single-use or short-lived, so (b) is not a standing
   takeover primitive.

### F-162 (Medium) `handle_invite_reply` returns `Done` on seven pre-commit transient failures, silently discarding an authenticated invite response forever

`rust/web/src/email/inbound.rs:992-1060` - the `return RouteOutcome::Done` at
`:996`, `:1008`, `:1017`, `:1021`, `:1028`, `:1042` and `:1057`.

`RouteOutcome`'s own doc comment (`:742-750`) states the contract precisely:

> `Done` = finished (successfully or failed unrecoverably); mark the event and
> return 200. `Retry` = transient failure **before any state mutation**; do not
> mark, return 5xx so svix retries.

All seven of these sites are inside the `tx` opened earlier in the handler and
**before** `tx.commit()` at `:1063`. Nothing is persisted when they fire - the
transaction is dropped and rolled back. They therefore meet the definition of
`Retry` exactly, and every one of them returns `Done` instead.

The consequences are concrete because the failures are genuinely transient:
`:1042` is `fetch_game_from_service` (the game service being down or slow),
`:1021` and `:1028` are DB lookups, `:996` is the response write itself. When
any of them fires, the webhook returns 200, `mark_event_processed` records the
`svix-id`, svix never redelivers, and **no reply email is sent** (all seven
return before `send_invite_reply_response` at `:1110`). A user who replied
"accept" gets silence and their acceptance is lost with no way to retry other
than finding the invite in the web UI. The same handler already uses `Retry`
correctly for its pre-mutation token lookup (`:862-865`) and its From
verification, so the contract is understood - it is just not applied past the
point where the transaction is opened.

Suggested fix: change all seven to `RouteOutcome::Retry`. They are pre-commit,
so redelivery is safe, and the `processed_webhook_events` dedup at `:610-617`
already protects the post-commit path.

### F-163 (Low) The SSE migration's replacement for a landed regression test carries `#[ignore]`, so the property is no longer checked on a default run

`rust/web/tests/sse_events.rs:456-457` vs the deleted
`rust/web/tests/websocket_hygiene.rs:67-68`

`efad81f9` deleted `live_websocket_survives_idle_past_request_timeout`, a
regression test guarding `TimeoutLayer` against bounding long-lived connections
(introduced `0093291`, 2026-07-10). The replacement,
`sse_stream_survives_past_request_timeout_with_keepalive`, asserts the same
property but is marked `#[ignore = "takes 32+ seconds"]`, where the original was
a plain `#[sqlx::test]` that ran by default.

This is **not** pattern 4e - the original test predates the remediation
programme, so no work package's closed row is falsified by it. It is recorded
separately because it is the same *effect* by a different route: a property that
CI checked before the migration is not checked after it, and the diff reads as a
faithful port. A reviewer grepping for "was the test carried over" finds it and
stops.

Suggested fix: run it in a nightly/slow job rather than `#[ignore]`, or shorten
the timeout under test via configuration so it can run by default.

### F-164 (Low) `main.scss` hardcodes a literal CSS colour on the one element the theme system cannot reach, in the file whose own comment forbids exactly that

`rust/web/style/main.scss:1091-1094`

```scss
.friend-request-badge {
  color: orange;
  font-weight: bold;
}
```

Every other colour declaration in the file's 1,095 lines resolves through
`var(--mk-*)` or `color-mix(in srgb, var(--mk-*) N%, transparent)` - I read the
file in full and this is the sole exception. The file states the rule itself at
`:761-762`: "Colors only via `--mk-*` vars; selection and friend markers carry a
non-hue cue ... so no meaning rides on hue alone." `.friend-request-badge` is a
friend marker, it uses the CSS keyword `orange` (`#ffa500`) rather than
`var(--mk-orange)`, and it carries no non-hue cue at all.

Consequence: the badge is a fixed mid-saturation orange under all 34 themes
including the three tritanopia and the six deutan/protan palettes, whose whole
purpose is to substitute hues that the default palette's orange is chosen
against. On the dark themes it is also the only element that does not
participate in the per-theme contrast gate `chrome_softens_meet_contrast_floor`
(`theme.rs:380-396`) enforces for every other chrome surface.

Suggested fix: `color: var(--mk-orange);` plus a non-hue cue (the file already
uses `border-style: double` for `.chip-friend` at `:875-878`).

### F-165 (Medium) T3-B6's SSE reconnect fix gates one thing more than its checklist row asked for, killing the only refresh path the friend-request badge had - and it cannot recover a half-open socket

`rust/web/src/websocket_client.rs:84-101`

The T3-B6 checklist row (`T3-B6-outbound-email-websocket.md:83`, ws F60, `Test? n`)
says exactly this and no more:

> Bind `ready_state` instead of destructuring it as `_` and call `open()` only
> when `ready_state.get_untracked()` is `Closed`

The commit put the *data-refresh bump* inside the same guard:

```rust
let _ = use_event_listener(use_document(), visibilitychange, move |_| {
    ...
    if doc.visibility_state() == web_sys::VisibilityState::Visible
        && ready_state.get_untracked() == ConnectionReadyState::Closed
    {
        open_vis();
        trigger.set_last_update.update(|n| *n += 1);   // <- not in the row
    }
});

window_event_listener(leptos::ev::online, move |_| {
    if ready_state.get_untracked() == ConnectionReadyState::Closed {
        open();
        trigger.set_last_update.update(|n| *n += 1);   // <- not in the row
    }
});
```

`last_update` is not a connection concern - it is the app's global refetch
trigger. Two `LocalResource`s track it and nothing else re-arms them:
`active_games` (`rust/web/src/app.rs:135-139`) and `friend_request_count`
(`:156-160`, whose own comment at `:154-155` says "Tracking `last_update` also
makes the badge live").

**The SSE stream emits only two named events**, `game` and `proposal`
(`rust/web/src/events.rs:82`, `:101`, `:170`), matching the client's
`named_events(vec!["game", "proposal"])`. **There is no SSE event for an incoming
friend request.** The unconditional visibility/online bump was therefore the only
live path that refreshed the badge in an already-open tab, and it is now gone
whenever the connection is healthy - i.e. in the normal case:

1. Alice has brdgme open, SSE connected (`ready_state == Open`).
2. She switches tabs for an hour; Bob sends a friend request. No SSE event exists
   for it.
3. She switches back. `ready_state` is `Open`, so the guard is false: no `open()`
   and no bump. The badge stays stale until a full navigation. Before this commit
   step 3 refreshed it.

**Second, worse path - the half-open socket.** leptos-use only moves its
`ready_state` off `Open` from the `onerror` handler
(`leptos-use-0.19.0/src/use_event_source.rs:311-313`), which never fires while a
black-holed socket merely sits idle. So after a laptop suspend or a wifi handoff,
`ready_state` is still `Open`, the `online` guard is false, and neither `open()`
nor the refresh runs. `open()` is `close(); ... init();` (`:394-410`) - tearing
down the dead `EventSource` and rebuilding it was precisely the recovery the two
listeners existed for, and a `Closed`-only guard structurally cannot do it.

The file's own doc comment already warns about this exact trade-off in the
sibling context (`websocket_client.rs:20-25`): "gating the local bump on WS
ready_state would re-open a half-open-socket window where a player's own move
doesn't render." The commit does that gating anyway.

Note the row is arguably wrong on its own terms too (a `Closed`-only guard cannot
recover a half-open socket), but the *extra* damage - gating the refresh bump -
is the implementer's, not the row's. `Test? n`, so no test was owed and none
exists; nothing would have caught it.

Suggested fix: keep the bump unconditional and guard only `open()`, exactly as
the row is worded. Separately, reconsider the row: a liveness probe or an
unconditional `open()` is what actually recovers a half-open `EventSource`.

### F-166 (Medium) Pattern 2 - `dec967b6` aligned one "latest game version" query with the operator's ordering and left its sibling in the same crate on the old ordering, with no test pinning either

`rust/web/src/db/game_types.rs:27-46` (fixed) vs
`rust/web/src/game_info/queries.rs:14-24` (not fixed)

The operator picks "newest version" by comparing the **row**
(`rust/operator/src/controller.rs:249-260`):

```sql
AND (newer.created_at, newer.name) > (
    SELECT cur.created_at, cur.name FROM game_versions cur
    WHERE cur.game_type_id = $1 AND cur.name = $5
)
```

`dec967b6` correctly added `, name DESC` to
`find_latest_non_deprecated_game_version` so it agrees. Its byte-identical-in-
purpose sibling was not touched:

```rust
"SELECT id FROM game_versions
 WHERE game_type_id = $1 AND is_public = true AND is_deprecated = false
 ORDER BY created_at DESC LIMIT 1"
```

These two are the only `ORDER BY ... LIMIT 1` picks over `game_versions` in
`rust/web`, so the sweep is complete and the miss is exactly one site.

Failing path: the operator applies two `GameVersion` CRs for one game type in a
single reconcile burst; `created_at` defaults to the transaction timestamp, so
two rows can tie to the microsecond. `find_latest_non_deprecated_game_version`
(reached from new-game creation, restart, import and the email `new game`
command - `game/server_fns.rs:1396`, `:1645`, `game/import.rs:42`,
`email/commands.rs:374`, `:860`, `db/mod.rs:204`) then picks the higher `name`,
while `game_info_rules_version_id` (`game_info/mod.rs:46`, the public game-info
page's rules link) can pick the other. **The rules a prospective player reads are
not the rules of the version they will be dealt into.**

**No test pins the tiebreak `dec967b6` added**, and both existing tests are
structurally blind to it:

- `db/game_types.rs:260-283` inserts a *deprecated* newer row and asserts it is
  skipped. It constructs no `created_at` tie, so it passes identically with and
  without `, name DESC`. Pre-existing, so not a decoy - but it is the changed
  function's only coverage.
- `game_info/queries.rs:336-378` `rules_version_id_picks_newest_created_at`
  deliberately gives the two rows **different** `created_at`
  (`now() - interval '1 day'` vs `now()`), and its comment describes the opposite
  scenario ("Lexicographic name order would pick '10.0.0' (wrong)").

Secondary (Low, maintainability): the three queries also disagree on predicate -
`find_latest_non_deprecated_game_version` does not filter `is_public`,
`game_info_rules_version_id` does, the operator's guard filters neither. "Latest
version" now has three definitions in the tree, and `game_types.rs:27-46` carries
no doc comment explaining the new `, name DESC` at all.

Suggested fix: add `, name DESC` to `game_info_rules_version_id`, add one test
that inserts two rows with an identical `created_at` and asserts both functions
pick the same id, and write the definition down in one place.

### F-167 (Low) Obligation 4 concluded: `CHROME_SOFTENS` carries a dead entry and its doc comment misdescribes the set

`rust/web/src/theme.rs:12-19`

```rust
/// Chrome-only soften expressions (main.scss surfaces: my-turn/finished/hover
/// tints) ...
const CHROME_SOFTENS: &[(NamedColor, u8)] = &[
    (NamedColor::Orange, 86),
    (NamedColor::Red, 86),      // dead
    (NamedColor::Foreground, 96),
];
```

`--mk-soften-red-86` has **zero consumers anywhere in the repo** outside a
historical planning doc (`docs/superpowers/plans/2026-07-13-26-web-chrome-theming.md:34`).
Its only user was deleted by `0d5da49` (2026-07-22, "feat(sidebar): show pending
and finished games alongside active"), which removed

```scss
.layout-game.finished { background-color: var(--mk-soften-red-86); font-weight: 700; }
```

without updating the Rust const. That predates the remediation window, so the
programme did not introduce it - but obligation 4 did not clean it up either.

Cost: `palette_css_vars` runs once per theme (`theme.rs:153-169`), so 34 themes
plus `:root` plus the dark media block each emit `--mk-soften-red-86` and
`--mk-soften-red-86-contrast` - **72 dead declarations in the SSR head of every
page**, in a const whose entire justification (`css.rs:5-8`) is "so the web
layer's generated CSS only carries the variables/classes that are ever
referenced."

The doc comment is doubly wrong: it presents `CHROME_SOFTENS` as "the main.scss
surfaces" set, but the token `main.scss` uses most - `--mk-soften-foreground-90`,
21 sites - comes from `IN_USE_SOFTENS`, not from this const.

`chrome_softens_meet_contrast_floor` (`theme.rs:379-394`) iterates
`CHROME_SOFTENS` and so appears to "cover" Red/86, but it is a contrast-gate
test, not a usage test - it cannot detect deadness by construction. **Not counted
as a decoy**; it tests a different property honestly.

Suggested fix: delete `(NamedColor::Red, 86)`, and correct the doc comment to
say this const is the *chrome-only additions* to `IN_USE_SOFTENS`, not the whole
chrome set.

### F-168 (Low) WP-54's two accessibility regression tests assert the absence of a styling marker rather than the presence of the fix

`rust/web/tests/ssr_pages.rs:256-266` and `:283-292`

The wfe F61 fix is `href="#"` plus `ev.prevent_default()` on three anchors that
previously had no `href` (and so were keyboard-unreachable). The tests assert
only that the inline marker is gone:

```rust
assert!(
    !body.contains("cursor:pointer"),
    "the sidebar logout anchor still has no href: {body}"
);
```

Failing path: revert `components/layout.rs:188-194` to
`<a on:click=...>"logout"</a>` - no `href`, and also no inline style, because the
global `a` rule in `main.scss:17-21` already supplies the cursor. Both assertions
still pass; the paired one is `body.contains("logout")`, which matches the link
text either way. Same for `login_page_anonymous`, whose paired assertion is
`body.contains("I already have a login code")` - again the link text, not the
`href`.

Recorded as Low and **not** as a decoy in the F-151 sense, because this is the
spec's own design ("their inline `style=\"cursor:pointer\"` is the marker") -
the weakness is inherited from the acceptance criterion rather than invented by
the implementation. It belongs in the corpus as an instance of *the criterion
being falsifiable only in one direction*.

Verified the marker really is absent at HEAD: `rg 'cursor:pointer'` over
`rust/web/src/` returns only `components/layout.rs:185`, inside a `//` comment
that never reaches HTML. All three anchors (`layout.rs:188-194`, `app.rs:671`,
`app.rs:693-699`) do carry `href="#"` + `prevent_default()`.

Suggested fix: assert `body.matches("href=\"#\"").count()`.

### F-169 (High) Pattern 2 - WP-57's at-least-once fix landed on the game and invite routes and not on the settings route, so the settings route can never return `Retry` and silently loses commands on any transient DB error

`rust/web/src/email/inbound.rs:1392-1433`

WP-57's spec §3b names the criterion verbatim:

> Only these, and only before any mutation: any `sqlx` `Err(_)` on a lookup -
> `find_game_player_by_email_token`, `from_matches_verified_email`,
> `resolve_user_by_verified_from` ...

`handle_settings_reply` performs two of those exact lookups and returns `()` on
`Err` for both, and its wrapper returns `Done` unconditionally:

```rust
async fn handle_settings_reply_route(...) -> RouteOutcome {
    let text = match fetch_inbound_text(state, from, email_id).await {
        InboundText::FetchFailed => return RouteOutcome::Retry,
        ...
    };
    handle_settings_reply(state, token, from, &text).await;   // returns ()
    RouteOutcome::Done                                        // unconditional
}

async fn handle_settings_reply(state: &AppState, token: &str, from: &str, text: &str) {
    let user_id = match find_user_by_settings_token(&state.pool, token).await {
        ...
        Err(e) => { tracing::error!("...settings token lookup failed: {e}"); return; }
    };
    match from_matches_verified_email(&state.pool, user_id, from).await {
        ...
        Err(e) => { tracing::error!("...settings From verification failed: {e}"); return; }
    }
```

Structurally confirmed: the last `RouteOutcome::Retry` in the whole file is at
`:1399` (the fetch). There is **no `Retry` anywhere in `handle_settings_reply`**
(`:1408-1520`).

Failing path: a user emails `theme dracula` to `s-<token>@brdg.me`. Postgres has
a momentary failure (pool timeout, failover, `too many connections`).
`find_user_by_settings_token` returns `Err`; the handler logs and returns; the
wrapper returns `Done`; `resend_webhook:663` writes the
`processed_webhook_events` row and returns **200**. Svix never redelivers. The
command is lost, and no reply email is sent. The byte-equivalent path in
`handle_game_reply:766-769` returns `Retry` for the same error class - **the fix
landed at two of three sibling sites.**

This is the same defect as F-162 (which is about `Done`-instead-of-`Retry` in
`handle_invite_reply`) arriving by a different route: F-162 is seven wrong
returns, this is a handler whose signature (`-> ()`) makes the right return
*unrepresentable*. Remediate together.

Two related, lesser observations, recorded but not raised separately:

- `fetch_inbound_text`'s doc comment (`:699-701`) says "Each handler does its
  token/From lookup first, then calls this". `handle_settings_reply_route:1398`
  calls it **before** any token lookup or From check, so a signed
  `email.received` naming `s-anythingatall@brdg.me` triggers an outbound Resend
  API fetch before the token is known to exist - doc/code divergence plus a small
  pre-auth work amplification.
- `handle_invite_reply:931-941` returns `Done` from `lock_proposal_for_update`'s
  `Ok(None)` arm while still holding `tx`, without calling `rollback_invite_tx`
  as the three WP-59 Task 6 branches do. Drop-rollback covers it; inconsistent,
  not a bug.

Suggested fix: give `handle_settings_reply` a `RouteOutcome` return and map both
`Err(e)` arms to `Retry`, exactly as `handle_game_reply` does.

### F-170 (Medium) WP-58's `EmailKind::pref_column()` is a documentation-only function - the real column mapping is an independent second copy, and the test that appears to guard it guards nothing

`rust/web/src/email/render.rs:35-42` vs `rust/web/src/db/users.rs:345-377`

WP-58's spec §3a says `pref_column()` returns a `&'static str` "used **only** to
pick one of three literal SQL statements in 3d". It is not used there:
`rg pref_column rust/` returns exactly the definition (`render.rs:35`) and four
asserts inside `render.rs mod tests` (`:730-733`). **Nothing in `src/` calls
it.** Because it is `pub`, no `dead_code` warning fires.

The mapping that actually runs is a separate `match` in `db/users.rs:345-377`
that hardcodes the three column names inside its SQL literals.

Failing path: flip `EmailKind::Invite`'s arm in `db/users.rs:367-374` to the
`reminder_emails_enabled` UPDATE. **Every test in the programme still passes.**
`render.rs:733` asserts `EmailKind::Invite.pref_column() == "invite_emails_enabled"`
against the unused function, and `unsubscribe.rs mod tests` only ever exercises
the `reminder` slug (`:162`, `:178`, `:196`, `:209`) - no test posts `invite`,
`turn` or `game`. A user clicking "Unsubscribe from game invitations" would
silently lose reminders instead, undetected.

This is F-153's **"documentation-only constant"** pattern in `pub fn` form -
a declared single source of truth with zero consumers, shadowed by a real,
untested duplicate - crossed with the decoy-test class: the D-11 guard test
asserts a mapping that governs nothing.

Suggested fix: make `db/users.rs` build its SQL from `pref_column()` (or take the
column name as a parameter), and parameterise the unsubscribe endpoint test over
all four `EmailKind` slugs.

### F-171 (Medium) A fifth confirmed "Test? y" row with no test - WP-58 rider row 2, `Test? y (assert absent)`

`rust/web/src/email/inbound.rs:1377-1380`

WP-58 §6 row 2, verbatim:

| `email/inbound.rs` `send_rules_reply_response` (wfe F3, 2nd header site) | Delete both `List-Unsubscribe*` inserts from the hand-built `BTreeMap`; no replacement. | **y (assert absent)** |

The deletion landed - HEAD's `send_rules_reply_response` inserts only
`In-Reply-To` and `References`. **The test does not exist:**

```
$ rg -n "List-Unsubscribe" rust/web/src/email/inbound.rs rust/web/tests/
(no output)
```

The only absence assertions in the tree are `render.rs:582-583` and `:674`,
which cover `render_game_email(..., None)` - a different function on a different
code path. `send_rules_reply_response` has no test at all.

This joins F-142, F-148, F-149 and F-150. The row is notable because it is the
*most* explicit form of the promise - it specifies not just that a test exists
but what it must assert - and it is still unfulfilled.

### F-172 (Low) WP-59's CRLF sanitiser truncates where the spec required replacement, so a legally folded `From` header is silently dropped

`rust/web/src/email/inbound.rs:135`

```rust
let sanitized = value.split(['\r', '\n']).next().unwrap_or("");
```

WP-59 Task 1's acceptance row, verbatim:

> a value containing `\r` or `\n` | CR/LF replaced with a space **before**
> parsing, so it can never become a second header

The injection half is satisfied. The behavioural half is not. For an RFC 5322
**folded** `From` value - `"Alice\r\n <alice@example.com>"`, which is legal
folding, not an attack - the spec's replace-with-space yields
`Alice  <alice@example.com>` and parses to `Some("alice@example.com")`, while the
implementation truncates to `"Alice"` and yields `None`. `resend_webhook:633-639`
then marks the event and returns 200 with no reply: a silently dropped move.

The test `extract_addr_spec_strips_crlf_before_parsing` (`:1947-1952`) exercises
only the injection case, where truncation and replacement produce the same
answer, so the divergence is invisible to the suite. A near-decoy: the test is
correct about the property it names, and blind to the one the spec's wording
actually distinguishes.

Suggested fix: `value.replace(['\r', '\n'], " ")` - literally what the row says.

### F-173 (Low) F-128 is NOT closed - the inbound path normalizes in SQL, every other path normalizes in Rust, and the two disagree

`rust/web/src/email/inbound.rs:532-545` vs `rust/web/src/auth/email_addr.rs:3-5`

`rg canonicalize_email rust/web/src/email/` returns **nothing**.
`from_matches_verified_email` compares with `LOWER(email) = LOWER($2)`, while
every write path (`auth/server.rs`, `settings.rs`, `proposals.rs`,
`new_game.rs`, `game/server_fns.rs`) stores
`auth::email_addr::canonicalize_email(raw)` = `raw.trim().to_lowercase()`.

Concrete divergent address: `İ@example.com` (U+0130 LATIN CAPITAL LETTER I WITH
DOT ABOVE). Rust's `to_lowercase()` maps U+0130 to `i` + U+0307, so the stored
row is `i\u{0307}@example.com`; Postgres `lower('İ@example.com')` under
glibc/ICU yields `i@example.com`. They never compare equal, so a verified user
whose address contains U+0130 has **every** inbound reply rejected as "From does
not match a verified address". ASCII is unaffected and the wider non-ASCII
behaviour is collation-dependent (nothing was executed), so this is recorded as a
structural divergence demonstrated at one code point, not a broad claim.

Attribution: WP-59's landing-order table names `from_matches_verified_email` as
"the package's most important fence: do not touch", so this is **outside WP-59's
mandate** - it is F-128 still open, with no owner. It reinforces Unit 07's
`CanonicalEmail` newtype proposal: the contract is enforced only by doc comment,
and the one place it is not enforced is an authentication comparison.

### F-174 (Low) `help_text()` still advertises `rules` to standalone users on the exact path `5786a1b6` corrected

`rust/web/src/email/commands.rs:179-208`, specifically `:192`

`5786a1b6` removed `rules` from `dispatch_settings_standalone`'s rejection string
(`:315-317`) - a genuine correction, because neither
`dispatch_standalone_server_command` (`:329-346`) nor
`dispatch_settings_standalone` (`:296-318`) handles `rules`. But `help_text()`,
which `dispatch_settings_standalone:309` returns to a no-game user who emails
`help`, still lists

> `rules [basic|advanced] - email the game rules and strategy`

under "Server commands", alongside `concede`/`end`/`undo`/`restart`. A no-game
user is told `rules` exists, sends it, and is told it is unavailable. This is
wfe F25's exact shape, narrowed to `rules` plus the four game-only verbs. The
commit subject says "help text"; the diff edits the rejection string, not
`help_text()`.

### F-175 (Medium) Pattern 2 - WP-60's atomic-token fix landed on `ensure_email_token` and its `_tx` twin, and not on the two byte-identical siblings 20 lines below

Commit `e5513ec6` (WP-60). **No WP-60 spec exists** - verified against
`git show 868094a6:.../planning/specs/` (45 files, none for WP-60); the
acceptance criteria are the WP-60 rows of
`docs/reviews/2026-07-23-rust-review/planning/checklists/T3-B6-outbound-email-websocket.md`
(`wfe F44`-`F51`, `wfe F63`).

`wfe F44` (atomic mint) and `F45` (`None` => error) were fixed at
`rust/web/src/email/outbound.rs:83-96` (`ensure_email_token`) and `:102-118`
(`ensure_email_token_tx`), both rewritten to
`UPDATE ... SET email_token = COALESCE(email_token, $1) ... RETURNING email_token`
plus an `anyhow::bail!` on `None`.

**`ensure_settings_email_token` (`:123-139`) and `ensure_unsubscribe_token`
(`:144-160`) still carry the verbatim pre-fix body**: `SELECT <col> ... `,
`fetch_optional`, `if let Some((Some(tok),))` early return, unconditional
`UPDATE ... WHERE id = $2`, `Ok(token)`. Both defects survive unchanged:

- F45 shape: a nonexistent `user_id` yields `None`, falls through, the `UPDATE`
  matches zero rows, `execute` returns `Ok`, and the function returns `Ok(token)`
  for a token that was never persisted (`:133-138`, `:154-159`).
- F44 shape: two concurrent callers both read NULL and both `UPDATE`; last writer
  wins and the first caller holds a token no longer in the DB. Concurrent callers
  on separate tasks/requests exist: `sweep.rs:193`, `notify.rs:398`,
  `proposals.rs:318,417,492,561,629,704`, `inbound.rs:1500`. A stale
  `unsubscribe_token` yields a dead unsubscribe link in the email footer
  (`render.rs:253-261`) - RFC 8058 / CAN-SPAM adjacent, not cosmetic.

The checklist scoped F44/F45 to the function *by name*, so the commit satisfies
the row literally while leaving the finding's class duplicated three times inside
one 80-line region. Compounding it, the doc comments at `outbound.rs:121-122` and
`:143` still read "Plain query, matching `ensure_email_token`" - **now false**,
since `ensure_email_token` is no longer a plain select-then-update. This is the
same shape as F-116, F-166 and F-169.

### F-176 (Medium) A sixth, seventh, eighth and ninth "Test? y" row with no test - all four of WP-60's tested rows

`git show e5513ec6` adds **no test whatsoever**. Its only test hunk *relocates*
`parse_duration_parses_units` from `outbound.rs` to `sweep.rs:616-641`, and that
belongs to `wfe F50`, a `Test? n` row.

The four `Test? y` rows in T3-B6 are `wfe F44`, `F45`, `F46`, `F63`:

- **F44/F45**: the only nearby test, `ensure_email_token_generates_and_reuses`
  (`rust/web/src/email/outbound.rs:301-364`), **pre-dates the commit** - present
  at `e5513ec6^:rust/web/src/email/outbound.rs:362` and untouched by the diff. It
  asserts generate-then-reuse and that the token was stored, and **passes
  identically against the old select-then-update code**. Nothing asserts the
  not-found path returns `Err`, and nothing asserts concurrent minting converges.
  This is the decoy shape (F-151, F-161d class): correct-sounding name, input that
  passes independently of the fix.
- **F46**: no test references `game_emails_sent_total` or
  `game_emails_failed_total`; the only occurrences in the tree are the two
  increments (`outbound.rs:51`, `:55`).
- **F63**: no test references `js_string_escape` or `sentry_init_snippet`; the
  only references are the definition and call sites (`app.rs:55,61,63,67,78`).
  Nothing asserts `</script>` is neutralised - the exact injection F63 names.

The "Test? y with no test" pattern now has **nine confirmed instances** (F-142,
F-148, F-149, F-150, F-171, plus these four rows).

### F-177 (Low) Pattern 2 inside `wfe F49` - two of the four `href` interpolations in the same function were left unescaped, ten lines apart

`rust/web/src/email/render.rs`. Escaped via the new `escape_html_attr`
(`:152-164`): `:238-243` (`browser_url`) and `:244-249` (`rules_url`). **Not
escaped**: `:252-262`, `<a href="{unsub}" ...>` and `<a href="{manage}" ...>`,
interpolated raw into the same `body` string inside the same `if let` block.

Impact is theoretical today, matching F49's own UNCERTAIN status: `unsubscribe_url`
(`render.rs:64-67`) is `public_base_url()` + a fixed slug + a 32-char alphanumeric
token from `generate_email_token` (`outbound.rs:72-79`), and
`manage_subscriptions_url` (`:69-72`) is base + `/settings`. No attacker-controlled
byte reaches either. But F49 offered two options - escape, or document the
trusted-URL precondition on `EmailContent` - and the commit chose escaping,
applied it to half the sites, and added no precondition doc either. Neither option
is discharged, and the file now teaches the wrong pattern by example.

### F-178 (Low) WP-60 added a second HTML-escape helper next to an existing one, with the comment saying none existed still in view

`render.rs:152-164` `escape_html_attr` duplicates
`rust/web/src/email/unsubscribe.rs:68-74` `html_escape` - same module tree, four of
five arms identical (`html_escape` also escapes `'`). `unsubscribe.rs:66-67`
explicitly documents "No public HTML-escape helper exists in this crate", so the
second copy was written with that comment on screen. `html_escape` has a test
(`unsubscribe.rs:234`); `escape_html_attr` has none. Maintenance duplicate, not a
live defect.

### F-179 (Medium) `ca7925bc` makes the email invite-accept auto-start fan out three separate emails to the same recipient, gated by three different preference columns

WP-76 (`bc051164`) has **no spec and no checklist row anywhere** - verified: the
spec listing at `868094a6` has no `WP-76-*.md`, `planning/EXECUTION-README.md:408`
records the omission as deliberate ("WP-76, WP-77, WP-79, WP-80 have no spec
file"), and grepping all eight `T3-B*.md` checklists for `WP-76` returns zero
hits. Its only acceptance criteria are `planning/work-packages.md:1217-1223`.
There is therefore **no `Test?` column to falsify** for this WP.

On the auto-start branch of `handle_invite_reply` the accepting invitee now
receives three mails for one event:

- `notify_game_emails(.., gid, None)` -> `NotifyKind::Turn`, "It is your turn"
  plus the board (`rust/web/src/email/inbound.rs:1076`, **added by `ca7925bc`**);
- `notify_started(proposal_id, gid, invitee_ids)` -> "The game has started!"
  (`:1090`). The invitee list is `roster.filter(response == "accepted")` minus the
  owner, and this player's response was written to `"accepted"` moments earlier at
  `:990-996`, so they are **in** it;
- `send_invite_reply_response(.., "Invite accepted. The game has started!", ..)`
  (`:1110`).

The last two already duplicated each other before `ca7925bc`; the commit added the
third. They are gated by `invite_emails_enabled`, `turn_emails_enabled` and
nothing respectively, so **no single unsubscribe damps the burst**. Three mails
describing one event three different ways is the classic spam-complaint and
list-reputation vector, and it lands on the sending-domain reputation that WP-57
and WP-58 were spent protecting.

### F-180 (Low) `ca7925bc`'s solo-game notify site is inert in practice - it can essentially never send

`rust/web/src/email/proposals.rs:1471`. The branch is reached only when
`human_invitees.is_empty()` (`:1453`), i.e. the game's only human is the creator.
`SendMode::Normal` runs `suppress_for_web_presence` (`notify.rs:351`), and the
creator arrived at `create_proposal` from a hydrated page whose presence effect
pings immediately on hydrate before its first sleep (`app.rs:236-242`,
`PRESENCE_PING_INTERVAL_MS` = 5 min) against a `RECENTLY_ACTIVE_WINDOW` of 600 s
(`db/users.rs:131`). The call is therefore suppressed every time in the normal
flow. Harmless, but it is not the "solo-vs-bots game start now notifies" behaviour
the commit message implies, and nothing tests it.

### F-181 (Low) The new game-start notify races the bot-turn notify and can double-mail the same transition

All four start sites call `broadcast_and_trigger` - which publishes `bot.turn`
(`rust/web/src/game/mod.rs:51-59`) - **before** `notify_game_emails`:
`proposals.rs:1470-1478`, `proposals.rs:1716-1718`, `inbound.rs:1073-1083`, and
the pre-existing `server_fns.rs:1339-1348` / `commands.rs:469-471`. If the bot
service completes its move before the notify's `find_game_extended` read
(`notify.rs:561`), the start-path notify (`before = None`) sees post-move state
and mails the newly-on-turn human, while `handle_bot_command_event`
(`game/mod.rs:393`, `before = Some(..)`) mails the same human for the same
transition. The window is a NATS round-trip against an in-process DB read, and the
ordering pre-dates `ca7925bc`, hence Low - but `ca7925bc` widened it to two more
sites.

### F-182 (Low) Both commits bypass the codebase's own injectable mailer seam, so none of the wiring they added is testable - and none of it is tested

No test in the tree exercises `dispatch_email_command`'s notify arm or any of the
four game-start notify sites. `rg 'fn .*notif'` over `proposals.rs` and
`email/inbound.rs` returns only the `ProposalMailer` trait methods and
`invite_notification_suppressed_by_recipient_presence` (`proposals.rs:2536`, about
invite mail). The crate **does** have an injectable seam - the `ProposalMailer`
trait (`proposals.rs:111-120`) with `mailer()`/`mailer_from()` (`:741`/`:749`) -
and both commits bypassed it by calling the free function directly, so the new
behaviour is not spyable even in principle.

**This is a disclosed gap, not a false `Test? y`**: WP-76 has no `Test?` column at
all, and `EXECUTION-STATE.md:18` explicitly records "No test added: no notify spy
infra in commands.rs harness". It belongs in the remediation plan as testability
debt, not as a checklist-integrity finding.

### F-183 (High) F-104 + F-138 have a concrete consequence: the email `new` command lowercases the bot name, the bot service looks it up case-sensitively, and the game silently stalls forever

Found while enumerating every bot-name write site for WP-77. **Not introduced by
`33150afe`** - it is WP-59-era code in `rust/web/src/email/commands.rs`. It is
recorded here because it is the concrete failing path that F-104 and F-138
predicted but did not demonstrate.

`classify_opponent` (`rust/web/src/email/commands.rs:82-93`) lowercases the token
on both of its bot arms:

```rust
let lower = t.to_ascii_lowercase();
if let Some(inner) = lower.strip_prefix("bot:") {
    return OpponentToken::Bot(inner.trim().to_string());   // :86 - lowercased
}
if bot_names.iter().any(|b| b.to_ascii_lowercase() == lower) {
    OpponentToken::Bot(lower)                              // :89 - lowercased
}
```

The lowercased string is written straight into the slot at `:398-401`
(`bot_name: difficulty`), and `validate_bot_slots` at `:420` accepts it because
`db/bots.rs:61-63` uses `eq_ignore_ascii_case`. Concrete path with a bot named
`Claude` and the email `new chess Claude`:

1. `classify_opponent("Claude", ["Claude"])` -> `Bot("claude")` (`:88-89`).
2. `validate_bot_slots` accepts - `"Claude".eq_ignore_ascii_case("claude")`.
3. Row stored as `game_bots.bot_name = 'claude'`; the NATS `BotTurnRequest`
   carries `'claude'` verbatim (`db/bots.rs:24`, `nats.rs:31`).
4. `rust/bot/src/main.rs:173` -> `load_bot_config(pool, "claude")` ->
   `rust/bot/src/config.rs:28` `WHERE name = $1` -> **no row**.
5. `bots_table_empty` is false, so `main.rs:188-193` takes the `else` branch:
   `tracing::warn!(outcome = "skipped", reason = "bot not found or disabled")`
   and `return Ok(())`.

The game is created, the bot is seated, and **the bot never takes a turn** - no
error to the user, no retry, one warn line. The game is permanently wedged. The
only surfacing anywhere is `admin.rs:200`'s orphaned-bot check.

This is reachable today only if an enabled bot has a non-lowercase name.
`admin::create_bot` (`rust/web/src/admin.rs:293-303`) permits it: `require_text`
(`:230-241`) neither lowercases nor restricts the charset. **Remediate as one item
with F-104 and F-138** - canonicalize in `validate_bot_slots` and return the
canonical name to the caller.

The five other bot-name write sites are canonical and clean:
`components/opponent_slot.rs:105` (post-`33150afe`, modulo F-184) and `:383`,
`new_game.rs:50-52`, `email/proposals.rs:1258-1260`, `db/bots.rs:88-94`.

### F-184 (Low) WP-77's fix does not cover the pre-settle window - `set_mode` can still store the hard-coded `"medium"`, producing a blank control and a submit failure

`33150afe` (WP-77) is one file, `rust/web/src/components/opponent_slot.rs`,
+32/-26. **No spec** (`EXECUTION-README.md:408` deliberate gap) and **no WP-77 row
in any of the eight `T3-B*` checklists** (all eight grepped), so there is no
`Test? y` row and no test is required. The commit adds none, and the file has no
`mod tests` at HEAD. The sole criterion is
`work-packages.md:1225-1232`: "Nothing guarantees the hard-coded default is in the
returned list; the default should come from the same source as the list."

On the settled path the fix is correct - see the refutation in "Verified good".
But `default_bot_name` (`:93-97`) falls back to `.unwrap_or_else(|| "medium"...)`
at `:96` whenever `bot_names.get()` is still `None`, and `set_mode(SlotMode::Bot)`
(`:99-107`) is wired to an `on:change` on a radio input (`:227`) rendered
**unconditionally** - it is not gated on the resource. The `<select>` (`:375-397`)
*is* gated, so it does not exist yet and cannot correct the state, and nothing
rewrites the stored value once the resource settles: the only reconciler, the
`Effect` at `:187-194`, pushes state *into* the DOM (`el.set_value(...)`), never
the reverse.

Failing path on a deployment whose bots are `["Claude", "Gemini"]`: user clicks
"Bot" during the fetch -> state `bot_name = "medium"`; resource settles, select
renders with the two real options, the `Effect` calls `set_value("medium")`, which
matches no option, so `selectedIndex` becomes -1 and the control renders **blank**;
submit is rejected by `validate_bot_slots` (`db/bots.rs:61-70`) with "'medium' is
not a valid bot type". This is precisely the defect WP-77 was written to remove,
surviving in the loading window.

Two riders: the error/empty fallback list at `:89` is still the hard-coded
`["easy","medium","hard"]`, so `default_bot_name()` returns `"easy"` on a fetch
error regardless of what exists (that literal pre-dates the commit); and the commit
**silently changed the error-path default from `"medium"` to `"easy"`** as an
unremarked side effect.

### F-185 (Low) `classify_opponent_detects_bots` is a decoy - its fixture is all-lowercase, so it cannot distinguish canonicalising from lowercasing, and it asserts the lowercased output as correct

`rust/web/src/email/commands.rs:1435-1455`. The fixture is
`vec!["easy", "medium", "hard"]` (`:1436`), and the assertions are
`classify_opponent("HARD", &bn) == Bot("hard")` (`:1442-1443`) and
`classify_opponent("BOT:easy", &bn) == Bot("easy")` (`:1450-1451`). Because every
DB name in the fixture is already lowercase, lowercasing the token and
canonicalising it to the DB spelling are indistinguishable - the assertions pass
either way. The test is named for exactly the risk in F-183 and its fixture makes
that bug invisible. Pattern 4b: the test agrees with the code rather than with the
invariant the four case-sensitive consumers need.

Its partner, `validate_bot_slots_accepts_case_mismatch` (`db/bots.rs:251-258`,
already filed as F-104), asserts `"EASY"` is accepted. The pair jointly blesses the
asymmetry - accept any case, store what you were given, consume case-sensitively -
that is the root cause of F-183.

## Verified good

### 09c refutations - do not re-derive

- **WP-77's default bot name is CANONICAL - the F-104/F-138 "fifth write path"
  hypothesis is REFUTED on the settled path.** Chain verified end to end:
  `opponent_slot.rs:105` `bot_name: default_bot_name()` -> `:93-97`
  `bot_name_options().and_then(|n| n.into_iter().next())` -> `:85-91`
  `bot_names.get()` -> `get_available_bots` (`game/server_fns.rs:704-718`) ->
  `db::find_enabled_bots` (`db/bots.rs:37-42`),
  `SELECT name FROM bots WHERE enabled = true ORDER BY display_order` - the
  **identical** query `validate_bot_slots` validates against (`db/bots.rs:53`). The
  default is a byte-for-byte copy of the DB `bots.name` column, i.e. the exact case
  the four case-sensitive consumers match. `33150afe` is the correct fix for its
  stated criterion. (The residual gap is the pre-settle window, F-184; the real
  case defect is at a different write site, F-183.)
- **The four case-sensitive bot-name consumers, for the record:**
  `rust/bot/src/config.rs:28` (`load_bot_config`, called `bot/src/main.rs:173`),
  `rust/bot/src/config.rs:67` (`load_providers`, called `main.rs:199`),
  `rust/web/src/admin.rs:200` (orphan detection), and `rust/web/src/db/bots.rs:24`
  (feeds `nats.rs:31`'s `BotTurnRequest`).
- **`33150afe` moved the long `wfe F58` comment block verbatim, not rewritten** -
  old `:139-162` and new `:69-84` are identical text. No claim weakened to fit new
  code. No `#[allow(dead_code)]` and no zero-caller item introduced;
  `bot_name_options` has 2 callers (`:94`, `:375`), `default_bot_name` has 2
  (`:105`, `:174`).
- **A default bot name that does not exist is NOT a hard failure at game
  creation**, and the two sub-cases differ: not in the enabled list at all ->
  `validate_bot_slots` (`db/bots.rs:61-70`) returns a user-facing message and
  callers abort (clean refusal); matches case-insensitively but not exactly ->
  creation **succeeds** and the failure is deferred to the bot service, which
  silently skips (`bot/src/main.rs:188-193`). That second case is F-183.
- **WP-59 Tasks 9-14 are NOT a coverage hole - `f56ff37` is the owner.** Settled,
  do not re-open:
  - Task 9 (`ServerFnError` classifier) **IMPLEMENTED** - `classify_server_fn_error`
    at `email/commands.rs:37-50`, `INTERNAL_ERROR_MESSAGE` at `error.rs:6`, call
    sites `:752`/`:823`/`:899`, tests `:1129-1145`.
  - Task 10 (`emails confirm`) **OBSOLETE, not skipped** - the whole
    `emails add/confirm/active/use/remove` family was removed by `da1ea24`
    (WP-56 From-auth redesign); `run_settings_emails` (`commands.rs:634-637`) now
    rejects those subcommands, asserted by
    `run_settings_emails_rejects_removed_subcommands` (`:1663-1693`) and
    `help_text_omits_address_management` (`:1695-1702`).
  - Task 11 (db.rs helper routing) **IMPLEMENTED** -
    `db::find_game_version_id_for_game` (`db/games.rs:530`, test `:1138`) used from
    `commands.rs:934`; `db::delete_login_confirmation` (`db/emails.rs:250`, `_tx` at
    `:260`) used from `auth/server.rs:522`, `:920`.
  - Task 12 (self-mention) **IMPLEMENTED** - `commands.rs:408-412`, exact spec
    wording.
  - Task 13 (`bump` cap disclosure) **IMPLEMENTED** - `commands.rs:498`, `:501`,
    `:502`, `:519-523`; same pattern at `auth/server.rs:954-965`.
  - Task 14 (`COMMANDS.md` reserved verbs) **DELIBERATELY NOT IMPLEMENTED** - the
    spec heading itself says "CARVED OUT to WP-85 ... Do NOT execute Task 14 as
    previously written, under any circumstances" (WP-85 is
    DEFERRED-BLOCKED-ON-MICHAEL). `docs/authoring/COMMANDS.md` correctly contains no
    reserved-verb text. Absence is the specified outcome.

- **WP-60 did NOT give the outbound tokens expiry / single-use / rate-limiting, so
  there is no pattern-2 gap against `settings_email_token` on that axis.**
  `ensure_email_token` (`outbound.rs:83-96`) and `_tx` (`:102-118`) still mint a
  bare 32-char alphanumeric token with no expiry column, no consumption and no rate
  limit; the change is purely `SELECT`+`UPDATE` -> `UPDATE ... COALESCE ...
  RETURNING`. `generate_email_token` (`:72-79`) is untouched by the commit. **F-161's
  substance is unaffected by WP-60 and remains open elsewhere** - correctly, since no
  T3-B6 row asks for those properties. The real pattern-2 gap on that file is F-175
  (atomicity), not token lifetime.
- **`RouteOutcome`: no THIRD defective route exists.** Every `RouteOutcome` return
  in `rust/web/src/email/inbound.rs` (lines 654-1405) was re-read. The game route
  (`:757-845`) and the second route (`:853-918`) use `Retry` for transient failures
  correctly. The only `Done`-on-transient sites are the already-filed **F-162**
  (`:996, :1008, :1017, :1021, :1028, :1042, :1057, :1065`) and **F-169** (settings
  route, `:1397-1405`). `ca7925bc` touches `handle_invite_reply` but only inserts a
  `()`-returning call after `tx.commit()`; it adds and alters no `RouteOutcome`
  return. **The `RouteOutcome` sweep is closed.**
- **`ca7925bc`'s game-start sweep is COMPLETE - pattern 2 REFUTED**, demonstrated by
  enumerating every caller of `insert_game_from_service` (`server_fns.rs:645`):
  `proposals.rs:1271` (`start_proposal_tx`), `proposals.rs:1454` (`create_proposal`
  solo), `email/commands.rs:451`, `server_fns.rs:1171` (`restart_core`). All four
  notify - `proposals.rs:1717` + `inbound.rs:1076` (the two `start_proposal_tx`
  callers), `proposals.rs:1471`, `commands.rs:470`, `server_fns.rs:1341`,
  `commands.rs:906`. Every other `INSERT INTO games` is `#[cfg(test)]`
  (`sweep.rs:613`, `admin.rs:2464`) or the import tool.
- **Pattern 4e REFUTED for `ca7925bc`.** It is `+20/-0` across two files
  `bc051164` never touched; it removes nothing, weakens nothing, deletes no test and
  does not alter `notify.rs`. `EXECUTION-STATE.md:18` had already disclosed "Item 2
  (game-start paths) parked for user" and `:38` records it closed. Rows agree with
  code.
- **F-170 is NOT extended by the game-start mail.** That mail is `NotifyKind::Turn`
  -> `SendMode::Normal` (`notify.rs:614-617`) -> `should_email_recipient`
  (`outbound.rs:208-210`), which reads `turn_emails_enabled` directly off the
  recipient row. An unsubscribed user does not get it. `EmailKind::pref_column`
  (`render.rs:35-42`) remains documentation-only - its only tree references are its
  own test at `render.rs:730-733` - but it is not on this path.
- **No hidden-information leak in the game-start mail.** `build_content`
  (`notify.rs:204-209`) calls `render_board_and_you_can(.., recipient_player
  .game_player.position as usize)`, passing `Some(position)` to `game::client::render`
  (`notify.rs:104-110`); the digest uses `get_game_logs(pool, game_id,
  recipient_player...id)` (`notify.rs:156`).
- **WP-60 metrics are correct on all three paths.** `game_emails_sent_total`
  appears once (`outbound.rs:51`, the `Ok` arm), `game_emails_failed_total` once
  (`:55`, the `Err` arm); `send_rendered_email` (`:62-68`) is a thin wrapper that
  adds no increment; the dev-mode early return (`:29-35`) increments neither, matching
  the documented intent of `auth/server.rs:88-92`. No double-count, no leak counted as
  success.
- **`notify_game_emails_treats_none_as_a_brand_new_game` (`notify.rs:899-913`) is
  NOT a decoy** - it calls the function under test and its two assertions
  discriminate (`gp1` on turn -> token minted; `gp0` not on turn -> none).
- **Noted, not a defect:** `auth/server.rs:92` still increments
  `login_emails_sent_total` *before* `resend.emails.send` with no failure counter -
  the exact shape `wfe F46` fixed in `try_send_rendered_email`. It is outside WP-60's
  declared scope, F46 permits attempt-counting when intended, and the attempt
  semantics are documented in place at `:88-91` by a commit predating the review (so
  not pattern 4b). Programme-level consistency item only.

**REFUTED, DO NOT RE-DERIVE: the `ssr` feature-gate question. The gated tests
run. No checklist row is voided.**

Unit 09a's premise was half right and the conclusion was wrong. Both halves,
with evidence:

1. **Confirmed:** `rust/web/Cargo.toml:99-154` has exactly two feature keys,
   `hydrate` (`:100`) and `ssr` (`:109`), and **no `default`**. Nothing in the
   workspace root turns `ssr` on implicitly, and `rust/.cargo/` does not exist.
2. **Refuted:** the tests are not silently skipped, because the test runner
   passes the feature explicitly. `scripts/rust-ci-commands.sh:26-31`:

   ```sh
   echo "==> Running tests (workspace, excluding web)..."
   cargo test --workspace --exclude web

   echo "==> Running tests (web, ssr feature)..."
   cargo test -p web --features ssr
   ```

   `web` is deliberately excluded from the workspace run and run separately with
   `--features ssr`. Clippy is split the same way (`:18`, `:21`), as is the sqlx
   prepare check (`:24`). `scripts/rust-test.sh:69` is only a wrapper that
   provisions Postgres/NATS and delegates to that script, and CI runs the same
   file directly (`.github/workflows/ci.yml:93-94`). No `cargo nextest` and no
   `cargo leptos test` exist anywhere in the repo.
3. **The failure mode would have been loud anyway.** The four integration test
   files in `rust/web/tests/` carry **no** feature gate but use `sqlx::PgPool`
   and `#[sqlx::test]` directly; `sqlx` is `optional = true`
   (`rust/web/Cargo.toml:30`) and enabled only by `ssr` (`:117`). So a plain
   `cargo test -p web` fails to compile rather than passing with 423 tests
   quietly removed. This is documented as intentional at `docs/DEV.md:106`
   ("**Plain `cargo check --workspace` and `cargo test -p web` fail by
   design.**") and tracked at `docs/BACKLOG.md:85` (item 58).
4. **No `default` feature was ever removed during the remediation.**
   `git log -S'default =' -- rust/web/Cargo.toml` returns only `c121d07`
   ("Initial Dioxus module", added `default = ["web"]`) and `56a8197`
   ("Switch to Leptos", removed it) - a Dioxus renderer feature, unrelated to
   `ssr`, years before this programme.

Scale of what is *not* voided: **25 modules gated `#[cfg(all(test, feature =
"ssr"))]`, 423 test functions**, the largest being `email/inbound.rs:1535` (69),
`email/commands.rs:1086` (41), `game/server_fns.rs:1525` (29),
`proposals.rs:2496` (27), `email/sweep.rs:612` (27), `db/game_write.rs:771` (24).
Plus 72 ungated integration test fns in `rust/web/tests/` and 20 plain
`#[cfg(test)]` modules in `src/`. Grand total ~600 test functions in `rust/web`.

One genuine, pre-existing and already-tracked residue: the **non-`ssr`**
(`hydrate`/wasm) target of `rust/web` has no CI gate at all
(`docs/BACKLOG.md:85` item 58, "~323 errors"). That is a different and lesser
concern than the one 09a raised, it predates the remediation programme, and it
is already on the backlog - **not a finding**.

**Obligation 1 - `efad81f9` contains exactly ONE pattern-4e instance (F-109),
and this is demonstrated by enumeration, not asserted.**

Ordering fact that bounds the search: `efad81f9` is the **earliest** remediation
commit on 2026-07-27. Confirmed ancestors (revertible): `13a1e69` (WP-36),
`347970a` (WP-39), `3c6b304` (WP-42). Confirmed descendants (cannot have been
reverted by it): `0a0f7e6` WP-35, `390dd3b` WP-58, `4fb252d` WP-64, `a9609e5`
WP-43, `2c28ae8` WP-65, `667c8f4` WP-66, `3610b95` WP-53, `2b116b2` T3-B6.

All 12 files touched by `efad81f9` were accounted for individually:

| file | deleted content | introducer | remediation fix/test? | reimplemented? |
|---|---|---|---|---|
| `tests/websocket_hygiene.rs` | whole file (2 tests) | `0093291` created it; the F55 test by **`13a1e69` WP-36** | yes (the F55 test) | partial - **F-109** |
| `src/websocket.rs` | `ws_tasks: TaskTracker`, `track_future`, `drain_ws_tasks`, `ws_tasks.close()` | **`13a1e69` (WP-36, ws F55)** | **yes** | **no** - **F-109** |
| `src/websocket.rs` | `ws_handler`, `handle_socket`, ping interval | `78537d7`/`9eb59f2`, pre-programme | no | superseded by design |
| `src/websocket.rs` | `WsConnectionGuard` / `ws_connections` gauge | `d45e47a`, pre-programme | no | **yes** - `events.rs:13-26` |
| `src/main.rs` | 5s `timeout(.., drain_ws_tasks())` | **`13a1e69` (WP-36)** | **yes** | **no** - **F-109** |
| `src/router.rs` | `/ws` route + doc sentence | `78537d7`/`0093291` | no | `/events` + `/events/public` |
| `web/Cargo.toml` | `tokio-util` `features=["rt"]` | **`13a1e69`** (added for `TaskTracker`) | consequence of F-109 | n/a |
| `web/Cargo.toml` | axum `ws`, `use_websocket`, `tokio-tungstenite` | pre-programme / `13a1e69` dev-dep | no | spec §7 mandated |
| `Cargo.lock` | tungstenite, dup `const-oid` | lockfile echo | no | n/a |
| `infra/cloudflare.tf` | `cloudflare_zone_setting.websockets` | `8765cdb`/`d86cf77`, pre-programme infra | no | spec §7 mandated |
| `src/websocket_client.rs` | `use_websocket_with_options` body | `7351bb4`, pre-programme | no | `use_event_source_with_options`; visibility/online listeners preserved |
| `app.rs`, `lib.rs`, `events.rs`, `sse_events.rs` | nothing deleted | - | - | pure additions |

Only three paths carried remediation-range content and all three came from
`13a1e69` / ws F55, i.e. one finding. The `CancellationToken` half of ws F55
**survives** (`websocket.rs:25`, `:78-80`; consumed at `events.rs:107`, `:175`),
as does the graceful-shutdown ordering (`main.rs:131-135`) - so F-109 is
correctly scoped to the `TaskTracker` drain, not to shutdown handling in
general.

Two near-misses, neither qualifying: the `ws_connections` gauge (faithfully
reimplemented as `sse_connections`, and pre-programme in origin), and the
`#[ignore]`d timeout test recorded above as F-163.

Spec context worth carrying to sign-off: WP-84's spec §3g **listed**
`ws_tasks`/`drain_ws_tasks`/the 5s block as deletion candidates, marked the
premise UNKNOWN, and required a real-listener proof test first - and that test
does exist (`rust/web/tests/sse_events.rs:504`). So the deletion was
spec-anticipated and deliberate. What is unaccounted for is that **WP-36's row
still reads closed** with its fix and test gone. F-109's remediation is
therefore a bookkeeping fix on WP-36's row plus a decision on the second,
never-implemented half of ws F55, not a revert of `efad81f9`.

**No WP-84 checklist file exists.** `git ls-tree 868094a6` yields exactly one
WP-84 doc (`planning/specs/WP-84-sse-migration.md`); `planning/checklists/`
holds only T3-B1..T3-B8, and `git grep -l 'Test?'` matches those eight plus
specs WP-46/57/58 - not WP-84. **WP-84 has no "Test? y" row to falsify**; its
test obligations are prose in §8 (8 cases) and §3g, and all 8 have
implementations in `rust/web/tests/sse_events.rs` (`:200-249`, `:251`, `:267`,
`:290`, `:330`, `:367`, `:416`, `:457`, `:504`). The nearest closed row is
`EXECUTION-STATE.md:71`. The two WebSocket rows in T3-B6
(`EXECUTION-STATE.md:56`, `:127` - ws F61, ws F62) are RULED MOOT, both with no
test.

- **`rust/web/src/visibility_cache.rs`** is genuinely well built and its tests
  are real, not decoys: 30s TTL with re-lookup (`entry_past_ttl_is_re_looked_up`
  uses `start_paused` + `advance`, deterministic), bounded at 256 entries with
  an eviction test that actually overshoots the cap, fail-closed on lookup error
  **and** the error result deliberately not cached
  (`lookup_error_yields_false_and_is_not_cached`), and a
  `game_and_proposal_ids_do_not_alias` test pinning the two-map split. Unit 07's
  refutation of the cross-user leak is confirmed from this side too: the cache
  is a local at `events.rs:65`, inside the per-connection spawn.

**Frontend commits - what is genuinely clean, read at HEAD not just diff-read.**

- **WP-54 `fddc42df` (frontend UX error handling).** The spec
  (`WP-54-frontend-ux-error-handling.md`, 2,058 lines) **has no `Test?` column**
  - its criteria are a 17-row disposition table plus per-task regression cases.
  All 17 findings are addressed at HEAD: five `GameMeta` actions share one error
  slot (`components/game.rs:55-133`, rendered `:154-156`); three inline confirms
  now route through `components/confirm.rs` (`:172`, `:182`, `:231`); six friends
  mutations share a slot (`friends.rs:373-467`); Colors and EmailPrefs revert
  optimistic writes from a `StoredValue` snapshot taken *before* the update
  (`settings.rs:133-162`, `:218-274`) while Theme is advisory-only as specified
  (`:560-575`); the two auth latches are keyed on `Option<Uuid>` and cleared on
  logout (`app.rs:178-211`, `:223-246`).
  `wfe F55`'s test (`tests/ssr_pages.rs:423-427`) is a **genuine two-direction
  guard** - it asserts both the absence of `"error running server function"` and
  the presence of the new copy. `clamp_player_count`
  (`new_game.rs:59-77`) was re-derived by hand: the key `(distance, -c)` breaks
  distance ties upward as the spec's assumption A3 requires, `i64` widening makes
  `.abs()` panic-free, and its four tests (`:680-711`) cover exact match,
  above-max, below-min, a non-contiguous tie in both directions, and the empty
  passthrough - **not a decoy**. `action_error_message`'s `_` arm
  (`error.rs:37-44`) is **not pattern 5**: it is a real narrowing
  (`ServerError(msg)` verbatim, everything else generic) and its stated rationale
  - `WrappedServerError` is `#[deprecated]` in server_fn 0.8 - is true.
  One declared residual, not a finding: `components/game.rs:323` still renders
  `e.to_string()` for the add-friend error, which the spec explicitly fences out
  in its Non-Goals and routes to Cross-package #3. It is client-only (the action
  value is `None` during SSR) and is the sole surviving site across the four
  edited component files.

- **WP-55 `f0a468b2` (Turnstile hard-nav) - fully clean.** No `Test?` column;
  §5 names two SSR tests and marks the browser behaviour "Not testable here".
  All six sub-criteria met verbatim: `hard_navigate` (`app.rs:338-347`) is
  byte-for-byte the spec's code including the SSR `let ... else { return; }`
  guard, and both `web-sys` features it needs are already declared
  (`rust/web/Cargo.toml:85`); `attr:rel="external"` on the index CTA
  (`app.rs:388`); the sidebar link converted from `<A>` to a plain anchor
  (`components/layout.rs:203`); `hard_navigate` on the post-logout bounce
  (`:135`) and the admin bounce (`admin.rs:1244`), with the dead `use_navigate`
  imports removed from both `layout.rs` and `settings.rs:7,19`.
  **The site count is still exactly five** - `rg '"/login"' rust/web/src/`
  returns `settings.rs:19`, `app.rs:388`, `admin.rs:1244`, `layout.rs:135`,
  `layout.rs:203` and no sixth, and there is no server-side redirect. The two
  tests (`tests/ssr_pages.rs:245-277`) count `rel="external"` and `href="/login"`
  occurrences in both the anonymous and logged-in shells, so either half of the
  fix regressing breaks them - **non-decoy**. Nothing in the diff exceeds the
  spec (no change to the `cf-turnstile` div, `site_key` resource,
  `get_turnstile_response`, or the `api.js` tag) and no other link was made
  external.

- **`7da90b2d` (clippy on `websocket_client.rs`) - semantics-preserving.** Both
  transformations were re-derived by hand at HEAD (`:55-71`, `:122-131`):
  `if A { if let Ok(m) = T { B } }` -> `if A && let Ok(m) = T { B }` preserves
  short-circuit order and leaves `B` byte-identical including the inner
  `else if let` attachment; no `unwrap_or` changed, no clone dropped
  (`e.clone()` still passed to `try_from`), no capture changed. Removing
  `view! {}` from the `hydrate` `PublicEventsWatcher` (`:145-161`) is inert - the
  macro expands to `()`, which implements `IntoView`, as the byte-identical
  `#[cfg(not(feature = "hydrate"))]` stub at `:163-165` already demonstrates.
  The `docs/BACKLOG.md:85` edit is an honest partial claim: it records the four
  lints as fixed while leaving the CI gate and part (a) explicitly open.

- **`dec967b6`'s fixed direction is correct.** The added `, name DESC` in
  `db/game_types.rs:27-46` exactly matches the operator's row comparison
  `(newer.created_at, newer.name) > (cur.created_at, cur.name)`
  (`rust/operator/src/controller.rs:249-260`). See F-166 for the unaligned
  sibling.

**REFUTED, do not re-derive:** `settings.rs:173-176`'s
`<select prop:value=...>` with `collect_view()` children is **not** a pattern-2
miss of WP-54's `wfe F58` build-order fix. The tachys attributes-before-children
hazard applies to `build` (client-side element creation), not `hydrate` (which
reuses SSR DOM already carrying the `<option>`s). `SettingsPage` renders
`ColorsSection` with no `<Suspense>`/`<Transition>` wrapper
(`settings.rs:34-37`; `rg 'Suspense|Transition' settings.rs` returns nothing),
so that select is server-rendered and hydrated. `OpponentSlotEditor`'s select
genuinely is built client-side (it sits inside `<Show when=...>`), which is why
it alone needed the `NodeRef` + `Effect` treatment it received.

**Email commits WP-57/58/59 - REFUTED suspicions, do not re-derive.** Each was
checked concretely and each is clean:

- **`svix-id` is attacker-chosen / unvalidated.** No. `resend_webhook:596-608`
  requires all three svix headers and calls `verify_webhook` **before**
  `event_already_processed:610`. `msg_id` is part of the signed payload
  (`verify_webhook:308-320`), so a forged or replayed id 401s without touching
  the DB. All three `.unwrap()`s in `verify_webhook` are gone (`:311`, `:315`,
  `:319`) and `verify_webhook_rejects_invalid_header_value`
  (`inbound.rs:1858-1864`) is a real test - it passes `"msg\ninjection"` as
  `msg_id` and asserts the error variant.
- **`processed_webhook_events` grows forever.** No. `sweep.rs:443-453` prunes at
  7 days via `db::delete_old_processed_webhook_events` (`db/emails.rs:234-245`),
  tested at `db/emails.rs:548-575`. 7 days exceeds svix's retry window. (WP-46's
  work, verified landed.)
- **Marker and effect in separate transactions = lost effect.** Not a defect
  against this spec: D-2 chose option A, §3c says "Keep `mark_event_processed`
  itself as-is", and the accepted double-process window is documented at
  `inbound.rs:547-552`.
- **A GET unsubscribes (RFC 8058's prefetcher hazard).** No - and structurally
  so: `unsubscribe_get` (`unsubscribe.rs:48-62`) takes no `State<AppState>` at
  all, so it cannot reach the DB. Tested at `:207-217`.
- **The unsubscribe token is guessable or reuses `settings_email_token`.** No.
  `generate_email_token()` yields 32 alnum chars (`outbound.rs:144-160`) into its
  own column with a partial unique index (`migrations/025_unsubscribe_token.sql`).
  `db/users.rs:345-377` only ever writes `false`, so the token cannot re-enable
  anything - F-129/F-130's problems do not transfer to it. The reflected `t` is
  HTML-escaped before landing in the form `action`
  (`unsubscribe.rs:55-59`, `:68-74`). The route is mounted **before**
  `.layer(session_layer)` (`router.rs:154-158` vs `:163`), so the unauthenticated
  one-click POST is not redirected. CSRF is not a finding: the POST is
  unauthenticated by design and gated solely on a secret that appears only in the
  user's own mail (D-10).
- **Only some bulk-mail sites got the unsubscribe link (pattern 2).** No - all
  eight are wired, each with the warn-and-send-without-link fallback:
  `notify.rs:397-426`, `sweep.rs:193-213`, `proposals.rs:318, 417, 492, 561, 629,
  704`.
- **`5786a1b6` is pattern 4b or 4e.** Neither. At HEAD, `rules` is handled by
  neither standalone dispatcher (`commands.rs:296-346`), so spec §3g's
  instruction to advertise it was factually wrong and the follow-up corrects the
  string to the truth. The F25 behaviour fix and its tests
  (`commands.rs:342-343`, `:987`, `:1047-1048`, `:1181-1240`) are intact. See
  F-174 for the residual it did not reach.
- **WP-59's addr-spec extraction is real and correctly wired.**
  `extract_addr_spec` (`:134-155`) uses `mail_parser` and is applied at both
  required sites - `resend_webhook:633` (the `from` fed to all three handlers and
  used as the reply recipient) and inside `select_route`. `parse_reply_commands`
  (`:30-82`) implements all three new stop conditions plus the retraction rule,
  with eight tests (`:1608-1672`) covering wrapped-Gmail, localized, Outlook and
  bare-header cases. `rollback_invite_tx` (`:736-740`) exists and is called on the
  three early-exit branches. See F-172 for the one divergence.
- **WP-57's regression tests exist and are not decoys**, though two are weaker
  than specified (recorded, not raised): `retry_not_short_circuited_as_duplicate`
  (`tests/inbound_webhook.rs:144-162`) re-POSTs while the condition is *still*
  failing, so it proves the second delivery was not short-circuited but never
  exercises "processed once the condition clears"; `success_marks_exactly_once`
  (`:166-183`) drives an *unknown* token, so its "success" is the token-miss
  `Done` arm rather than a processed command. `transient_failure_returns_5xx_without_marker`
  (`:126-140`) and the permanent-failure case (`:187-204`) are clean.

## Coverage gaps

### None open - Unit 09 is closed

**The 09c brief has been discharged.** All four remaining commits were opened by
09c: `e5513ec6` (WP-60) -> F-175..F-178, `bc051164` + `ca7925bc` (WP-76 pair) ->
F-179..F-182, `33150afe` (WP-77) -> F-183..F-185. The WP-59 Tasks 9-14 ownership
question is settled (no hole; `f56ff37` is the owner). No commit in Unit 09's
scope is unexamined.

### Carry into the unified report / later units

- **F-183 (High) must be remediated as ONE item with F-104 and F-138.** It is the
  concrete consequence those two predicted: a non-lowercase enabled bot name plus
  the email `new` command produces a permanently wedged game with no user-facing
  error. The fix is to canonicalize inside `validate_bot_slots` and return the
  canonical name to callers - which closes all three at once. F-185 is the decoy
  test that hid it and must be re-fixtured in the same change.
- **F-179 (Medium) is Unit 10-adjacent**: the triple-email fan-out lands on the
  same sending-domain reputation that WP-57 and WP-58 were spent protecting.
- **F-182 is testability debt, not a checklist-integrity finding.** The
  `ProposalMailer` seam exists and both WP-76 commits bypassed it; the gap is
  disclosed in `EXECUTION-STATE.md:18`.
- **The "Test? y with no test" tally is now NINE** - F-142, F-148, F-149, F-150,
  F-171 plus WP-60's four rows (F-176). Note the counterweight 09c established:
  **WP-76 and WP-77 have no spec AND no checklist row at all**, which
  `EXECUTION-README.md:408` records as a deliberate gap. Those are untested by
  design, not falsified rows - the unified report should not conflate the two.
- **`admin::create_bot` (`rust/web/src/admin.rs:293-303`) permits arbitrary bot
  name casing** - `require_text` (`:230-241`) neither lowercases nor restricts the
  charset. This is the precondition that makes F-183 reachable. Route with F-183.

### Closed out by 09b - do not redo

- The `ssr` feature-gate question (REFUTED, see Verified good).
- Obligation 4 / F-15 at the real emitter (LATENT ONLY, see Progress).
- WP-54, WP-55, `7da90b2d` (clean), `dec967b6` (F-166).
- The `settings.rs` `<select>` pattern-2 suspicion (REFUTED).

### Not verifiable under this session's constraints

- Anything requiring execution. No test, lint or `cargo` invocation was run, per
  the hard constraint, so every "this test passes" claim in this report is a
  claim about the test's *content*, not its result.
- Browser-level verification of WP-55's hard-navigation (no wasm/browser harness
  exists in `rust/web/tests/`).
- `7da90b2d`'s claim that `cargo clippy -p web --features hydrate --target
  wasm32-unknown-unknown -D warnings` now passes.
- The deployment DB's collation, which bounds how broad F-173 is.
- Whether `PUBLIC_BASE_URL` is an `https://` origin in production.
  `config::public_base_url()` defaults to `http://localhost:3000`, which would
  make WP-58's `List-Unsubscribe` header non-HTTPS and RFC 8058-invalid. Same
  helper as `notify::browser_url`. A deployment-manifest question of the F-96 /
  4d family, not a code defect - **route it to the same deployment checklist
  F-96 produced.**

### Deliberately excluded

- WP-54's Cross-package items #1-#8 (22 unstyled `class="error"` sites,
  `admin.rs`'s 15 `e.to_string()` sites) - the spec routes them elsewhere.
- WP-59 Tasks 9-14 (`commands.rs` classifier, `emails confirm`, db.rs helper
  routing, self-mention, `bump` cap, `COMMANDS.md`) - outside the three commits
  W3 was given. **RESOLVED by 09c: not a coverage hole.** `f56ff37` owns Tasks 9,
  11, 12, 13; Task 10 was dissolved by WP-56 (`da1ea24`) removing the feature;
  Task 14 is a deliberate non-implementation per the spec's own carve-out to
  WP-85. Per-task evidence is in "Verified good". Implementation status only - no
  deep code review of those tasks was performed.
