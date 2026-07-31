# Unit 10 - bot / operator / tools + dependency & workspace hygiene

Findings continue from F-186.

## Progress

- [x] Obligation 1: crypto divergence + ungated dev key - CONFIRMED, F-186 (High), F-187 (Medium)
- [x] Obligation 2: nats duplication - CONFIRMED as still-open duplication, no live wire drift yet, F-188 (Medium)
- [x] Obligation 3: prompt.rs leak - **REFUTED** with a full field enumeration; two adjacent real leaks found
      instead (F-192, F-193) plus a new decoy test
- [x] Obligation 4: axum `http.rs` envelope handling - CONFIRMED gap, F-191 (Low); the WP-06 test is NOT a decoy
- [x] Obligation 5: DISCHARGED - 28/28 consumers are `[dev-dependencies]`; feature not in `default`; no risk
- [x] Obligation 6: **REFUTED** - correct consolidation, and the causality in the premise is backwards
- [x] Obligation 7: bot-side half of F-183 - CONFIRMED and extended (second site), F-189 (High)

**All seven obligations discharged. Findings F-186..F-196.** Commits reviewed: WP-61 `4f5f6d45`, WP-62 `e682f6bc`,
WP-63 `d2decf85`, WP-71 `dcec1adf` (end state), plus the WP-52/WP-66 sqlx question.

**STOPPED AT THE BUDGET LINE - propose as Unit 10b** (the dependency/workspace-hygiene half, 10 commits, none of
which any obligation named): WP-64 `4fb252da` (workspace.dependencies/package/lints), WP-65 `2c28ae85` (workspace
hygiene), **WP-66 `667c8f42` (101 files, sqlx 0.8 -> 0.9 - the "guilty until read" mechanical commit; its `.sqlx`
half is now cleared, but its call-site half is not)**, WP-67 `634c72db` (sentry feature trim), WP-69
`e2ee5342`+`be185ccb` (deny.toml), WP-70 `8304baf5` (serde_yaml -> serde_yaml_ng), WP-72 `a5d6f102` (comment only),
**WP-73 `22d00b8d` (140 files, 108 game binaries collapsed to `brdgme_game_bin` - the other "guilty until read"
commit)**, `22b68689` (devenv cargo-deny). Per `00-breakdown.md`, WP-66 and WP-73 are to be *sampled*: spot-check
4-5 game crates for WP-73 and the `rust/web` sqlx call sites for WP-66. dp-F14 (unsafe-libyaml still in Cargo.lock)
is BACKLOG #57, deliberately incomplete - do not re-flag.

## Findings

### F-186 (High) - `rust/bot/src/crypto.rs:66-70` falls back to the hardcoded dev key in every environment

Read directly by the Lead; both files read in full.

```rust
// rust/bot/src/crypto.rs:66-70
pub fn load_key() -> Result<LoadedKey, CryptoError> {
    let hex_str = match std::env::var("DATABASE_ENCRYPTION_KEY") {
        Ok(v) => v,
        Err(_) => return Ok(LoadedKey::Default(default_key())),
    };
```

vs the web copy, which is fail-closed:

```rust
// rust/web/src/crypto.rs:56-64
        Err(_) => {
            if std::env::var("ALLOW_INSECURE_DEFAULT_KEY").as_deref() == Ok("true") {
                return Ok(default_key());
            }
            return Err(CryptoError::MissingKey);
        }
```

- **What**: the bot service silently uses `b"brdgme-dev-key-not-for-prod!!!"` zero-padded to 32 bytes whenever
  `DATABASE_ENCRYPTION_KEY` is unset, with no opt-in flag and no error variant (`CryptoError` has no `MissingKey`).
- **Why it matters**: this is the confirmed `docs/CODING.md:701` violation carried forward from the F-96
  investigation. The key protects stored bot API credentials; both services read the *same* database column, so a
  bot pod started without the env var will happily decrypt/encrypt with a publicly-known constant. It also makes the
  two services disagree about what is a valid deployment.
- **The forbidden pattern is explicit at the call site** (`rust/bot/src/main.rs:800-808`):

  ```rust
  let loaded_key = match crypto::load_key() {
      Ok(key) => {
          if key.is_default() {
              tracing::warn!(
                  "DATABASE_ENCRYPTION_KEY not set - using insecure default key, DO NOT USE IN PRODUCTION"
              );
          }
          Some(key)
  ```

  This is verbatim the "dev default plus log warning" pattern that the F-96 investigation established is
  **forbidden** by `docs/CODING.md:701` and is *not* the house pattern. The `LoadedKey::Default`/`is_default()`
  enum exists solely to support it - `rust/web/src/crypto.rs` has no such type.
- **Fix**: delete `rust/bot/src/crypto.rs` and depend on a single shared crypto crate (see F-187); failing that,
  port the `ALLOW_INSECURE_DEFAULT_KEY` gate and the `MissingKey` variant verbatim, and delete `LoadedKey`.

### F-187 (Medium) - `rust/bot/src/crypto.rs` is a divergent duplicate of `rust/web/src/crypto.rs` (F-90 not closed)

Diverges on four independent axes, i.e. every hardening WP applied to the web copy is absent from the bot copy:

| Axis | `rust/web/src/crypto.rs` | `rust/bot/src/crypto.rs` |
|---|---|---|
| Missing-key behaviour | `Err(MissingKey)` unless `ALLOW_INSECURE_DEFAULT_KEY=true` (`:56-64`) | silent dev-key fallback (`:66-70`) |
| Key material in memory | `Zeroizing<[u8;32]>`, decoded hex `bytes.zeroize()` (`:66-74`) | plain `[u8;32]`, hex buffer left in memory (`:71-75`) |
| Nonce source | `getrandom::fill` with error propagation (`:77-81`) | `Aes256Gcm::generate_nonce(&mut OsRng)`, panics on RNG failure (`:19`) |
| Length check | explicit `bytes.len() != 32` then zeroize (`:67-70`) | `try_into()` (`:72-74`) - equivalent, but leaves the hex buffer live |

- **Why it matters**: F-90 was recorded as addressed by fixes that landed only in the web copy. The duplicate is
  still there at HEAD and has drifted further, not less. Any future crypto fix has a 50% chance of landing in one
  copy only - this is pattern 2 at file granularity.
- **Fix**: extract to `rust/lib/crypto` (or move into an existing shared lib crate) and have both binaries depend on
  it. Remediate as one item with F-186 and F-188.

### F-188 (Medium) - F-108 still open: `rust/bot/src/nats.rs` duplicates the wire protocol with no shared type and no round-trip test

Both files read in full by the Lead (`rust/bot/src/nats.rs:1-36`, `rust/web/src/nats.rs:1-413`).

- **Current state**: `BotTurnEvent` (bot `:14-20` / web `:27-33`) and `BotCommandEvent` (bot `:22-28` / web `:35-45`)
  are still field-for-field identical in name, type and order, so **there is no live wire incompatibility today**.
  The constants `STREAM_NAME`, `SUBJECT_COMMAND`, `CONSUMER_TURN` also match.
- **Already diverged (non-wire)**: the bot copy omits `SUBJECT_TURN`, `CONSUMER_COMMAND`, `MAX_TURN_ATTEMPTS` and
  `MAX_DELIVER` (`rust/web/src/nats.rs:12-25`). `MAX_DELIVER` carries the comment "Shared by the consumer config
  and the (future) term ceiling **so the two cannot drift** (WP-38)" - a guarantee that holds only within the web
  crate. The bot's `BotCommandEvent::attempt` doc comment exists only in the web copy, so the bot side has no
  in-repo statement of what `attempt` means.
- **Why it matters**: this is the JSON wire contract between two separately-deployed services, maintained by
  copy-paste. Nothing in CI would catch a field rename, a type change (`i32` -> `i64`), or a `#[serde(rename)]`
  added on one side. There is no round-trip test in either crate serializing with one definition and deserializing
  with the other (the web `nats.rs` test module tests advisories, drift detection and the supervisor only -
  `:299-413`). During a rolling deploy the two versions are live simultaneously by construction.
- **Concrete instance of the risk already in the tree**: `rust/bot/src/main.rs:1039-1042`

  ```rust
  #[test]
  fn ack_heartbeat_interval_below_ack_wait() {
      assert!(ACK_HEARTBEAT_INTERVAL < std::time::Duration::from_secs(5 * 60));
  }
  ```

  The `5 * 60` is a hardcoded copy of `let ack_wait = Duration::from_secs(5 * 60)` at
  `rust/web/src/nats.rs:150` - a *local variable in another crate*, where the comment says "Do NOT lower this, and
  revisit alongside any ack-cadence change". Lowering it in `web` leaves this test green while the bot's 60s
  heartbeat (`rust/bot/src/main.rs:33`) silently stops preventing redelivery, which duplicates the turn (the
  precise failure `rust/web/src/nats.rs:142-149` warns about). Promote `ack_wait` to a shared const in the same
  change.
- **Fix**: move `BotTurnEvent`/`BotCommandEvent` and the shared subject/stream/consumer/ack_wait constants into one
  shared crate that both binaries depend on. Remediate as one item with F-186/F-187 (same root cause: no shared
  crate between `rust/bot` and `rust/web`).

### F-189 (High) - F-183's bot-side half CONFIRMED: case-sensitive lookup + silent ack wedges the game permanently

Read directly by the Lead.

```sql
-- rust/bot/src/config.rs:26-29  (load_bot_config)
"SELECT name, include_basic_strategy, include_advanced_strategy, temperature \
 FROM bots WHERE name = $1 AND enabled = true"
```

Postgres `=` on `text` is case-sensitive, so a stored `game_bots.name` of `"claude"` never matches a `bots.name`
of `"Claude"`. **Second site, same defect, not previously cited**: `rust/bot/src/config.rs:67`
(`load_providers`) uses `WHERE b.name = $1` - so even if `load_bot_config` were fixed alone, provider lookup would
still fail and the turn would take the "No LLM providers configured" error path. Both must be fixed together.

The consequence at `rust/bot/src/main.rs:186-194`:

```rust
            } else {
                tracing::warn!(..., outcome = "skipped",
                    reason = "bot not found or disabled", "bot_turn_end");
                return Ok(());
            }
```

- **`Ok(())` means the `bot.turn` message is acked and discarded.** No error, no retry, no NATS redelivery, no
  `bot.command` published. The turn is silently dropped at `warn` level. The game is created, the bot is seated,
  and it never acts - permanently wedged, exactly as F-183 predicted. Note the asymmetry with the very next block
  (`:210-221`), which returns `Err` for "no providers" and therefore *does* get JetStream redelivery: the
  wrong-case path is the one that fails silently.
- **Aggravating**: the `table_empty` escape hatch (`:179-185`) means the failure only manifests once a real `bots`
  row exists, i.e. never in a fresh dev database and always in production.
- **Fix**: as F-183 specifies - canonicalize inside `validate_bot_slots` and return the canonical name, closing
  F-104, F-138, F-183 and this at once. Independently, `main.rs:186-194` should return `Err` (or explicitly
  term the message) rather than a silent `Ok(())`, so a misconfigured bot is alertable; and the two `WHERE ... name
  = $1` sites should be citext/`lower()`-normalized as defence in depth.

### F-190 (Low) - an invalid `DATABASE_ENCRYPTION_KEY` degrades the bot to "warn and continue", hard-failing every turn

`rust/bot/src/main.rs:809-816`: a malformed key (bad hex, wrong length) is caught, logged at `warn`, and
`encryption_key` is set to `None`. `run_bot_turn` then takes `None => Vec::new()` (`:198-203`) and, unless the
`bots` table is empty, returns `Err("No LLM providers configured")` (`:210-221`) for **every** turn. Those errors
leave the message unacked, so each one burns `max_deliver = 3` redeliveries and then strands in the WorkQueue
stream permanently. A single typo in a secret therefore produces a slow, silent, unrecoverable outage whose only
startup symptom is one `warn` line. The web crate treats the same input as a hard `Err` at load. Prefer failing
startup outright, consistent with F-186's fix.

### F-191 (Low) - obligation 4: a malformed request *envelope* bypasses the `Response` contract entirely, and no test covers it

`rust/lib/cmd/src/http.rs:26-29` takes `Json(req): Json<Request>` as an extractor, so **envelope** deserialization
happens before `handle` runs. An unparseable or unrecognized envelope is answered by axum's default `JsonRejection`:
HTTP 400/422 with a `text/plain` body. Every in-band error path in this file produces HTTP 200 +
`Response::SystemError` JSON.

- **Not a crash, and not a decoy test.** `malformed_game_json_returns_system_error_not_panic` (`:92-118`) is honest
  about its scope: its payload is a *well-formed* `Request::Status` whose `game` **string** is invalid, which is
  handled inside the requester and does return `SystemError`. The caller side degrades safely too:
  `rust/lib/game_client/src/lib.rs:187-189` maps any non-2xx to `GameClientError::HttpStatus { status, body }`.
- **What is actually wrong**: the WP-06 acceptance narrative reads as though "malformed request -> SystemError" is
  established, and it is not - only "malformed *game state*" is. There is **no test at any layer** that sends a
  malformed envelope (`{"garbage":1}`, truncated JSON, wrong content-type) to `route::<G>()`. Any future change to
  the `Request` enum that breaks compatibility (a renamed variant, a new required field) would surface at runtime
  as an opaque 422 with a text body, unattributable to a specific request, rather than as a structured
  `SystemError` message. That is a real observability regression across a service boundary.
- **Fix**: add `.method_not_allowed_fallback` / a custom `JsonRejection` handler that renders
  `Response::SystemError { message }` with a 200, matching the documented contract; and add the missing envelope
  test. Cheap, and it makes the WP-06 claim true.

### Obligation 3 - REFUTED: no hidden-information leak into the LLM prompt

Every field reaching the prompt string was enumerated (Worker 1, full read of `rust/bot/src/prompt.rs`,
`main.rs::build_messages`, `lib/cmd/src/requester/gamer.rs`, `lib/game_client/src/lib.rs`). `prompt.rs` is a pure
minijinja renderer with **no access to game state**: both `render_system` (`prompt.rs:122-132`) and `render_user`
(`:134-146`) pass an explicit closed field list to `context!{...}` - no struct-wide `Serialize`, no spread, no
template global. Origins:

| Prompt field | `main.rs` | Origin | Verdict |
|---|---|---|---|
| `game_rules` / `basic_strategy` / `advanced_strategy` / `data_docs` | 564-569 | `G::rules()`/`basic_strategy()`/... statics (`gamer.rs:235-255`); the handlers **ignore** the `game`/`player` fields those requests carry | public |
| `my_name`, `my_colour`, `players[].name/colour` | 573-588 | `names[]` + `LIGHT.player_color(seat)` | public |
| `players[].score` | 585 | `gamer.points()` (`lib/cmd/src/api.rs:185`) - bypasses `pub_state` | see F-194 |
| `pub_state_yaml` | 599 | `Gamer::pub_state()` (`gamer.rs:107-111`) | correct |
| `player_state_yaml` | 600 | `Gamer::player_state(p)` indexed at the acting seat (`game_client/src/lib.rs:326-331`) | correct |
| `command_spec` | 589-594 | `game.command_spec(player)`, per seat (`gamer.rs:118`) | correct |
| `recent_logs` | 602 | `is_public OR targeted-at-me` SQL (`main.rs:525-537`) | correct |
| `failed_commands` | 603 | the bot's own rejected commands + `UserError` text | own seat |

`BotContext.game_state` (the raw persisted state) is in scope but reaches only the change-detection compare
(`:355`) and the `Request::Play` validation call (`:396`) - it never enters a context struct. **Pattern-2 sibling
check passes**: the web's log query carries the byte-identical predicate (`rust/web/src/db/games.rs:463-466`).
Spot-checked `texas-holdem-2` - the `Deck` and RNG live on `Game`, absent from both `PubState` and `PlayerState`,
so YAML-serialising `PlayerState` cannot exceed the human render. The guarantee is delegated to each game crate's
`pub_state`/`player_state`, which is the per-crate audit Units 02-04 already own, not a bot defect. `prompt.rs`
also predates the remediation programme (first landed `1823dcd`, 2026-04-02); WP-61 `4f5f6d4` grew its tests and
added `spec_to_yaml` without changing what data reaches the prompt.

### F-192 (Medium) - the *whole* game-service response body, containing every seat's private state, can reach Sentry

`rust/lib/game_client/src/lib.rs:25-35` embeds the full body in two error variants:

```rust
#[error("game service returned {status}: {body}")]
HttpStatus { status: reqwest::StatusCode, body: String },
#[error("error parsing game service response: {source}; body: {body}")]
ParseResponse { body: String, #[source] source: serde_json::Error },
```

constructed at `:188` and `:190-191` with the entire body. For the bot's `fetch_game_data` that body is a
`Response::Status`, which by construction contains `player_renders` for **every** seat (`gamer.rs:112-120`) *and*
`game.state`, the complete unredacted state. It propagates through `.context("Failed to fetch game data")`
(`rust/bot/src/main.rs:523`) to `tracing::error!(..., error = ?e, ...)` at `:961`, and the subscriber has
`sentry_tracing::layer()` installed (`:797`).

- **Why it matters**: identical class of harm to the leak obligation 3 hunted for - all players' hidden state to a
  third party - via a different egress. Triggered by a game-service 502 or a mid-deploy schema mismatch, not by an
  attacker. It also lands in stdout logs. Note F-191's axum change makes the `HttpStatus` path *more* likely, since
  a malformed envelope now returns non-2xx instead of an in-band `SystemError`.
- **Fix**: truncate (`body.chars().take(512)`) or elide the body in the `Display` impl, keeping the full body only
  behind a debug feature.

### F-193 (Medium) - `fetch_game_data` pulls every seat's private state into the bot process for no reason

`rust/lib/game_client/src/lib.rs:310-331` issues `Request::Status`, whose handler renders **all** `player_renders`
(`gamer.rs:112-120`), then discards all but `player_renders[player]`. The bot needs only `PubRender` +
`PlayerRender{player}` - both already exist as endpoints (`:248-282`).

- **Why it matters**: this is what makes F-192 severe, and it is the one line that would turn a benign future
  refactor ("just serialise `GameData`/`status_resp` into the prompt") into a total-information leak. Removing it
  makes the leak structurally impossible rather than merely not-currently-happening. Defence in depth for exactly
  the class Units 02-04 spent their budget on.
- **Fix**: concurrent `PubRender` + `PlayerRender{player}`; `points` needs a separate source or a `Status` variant
  that omits other seats' renders.

### F-194 (Low) - `points()` is the one prompt input that bypasses the `pub_state` redaction boundary

`rust/bot/src/main.rs:585` renders `score` for **every** seat from `game_data.points`, which originates in
`gamer.points()` (`rust/lib/cmd/src/api.rs:185`) rather than `pub_state`. Not a bot defect: the platform already
treats points as public (`rust/web/src/db/game_write.rs:715,740` persists every seat's points and serialises them
to every client). Recorded because if any game ever computes mid-game `points()` from hidden state, the bot and the
web UI leak together and neither has a redaction test covering it. Pairs with the carried-forward observation that
`Gamer::points()` has no documented contract.

### F-195 (Low) - the bot's own private state is logged verbatim at TRACE

`rust/bot/src/main.rs:276-282` logs `system_prompt` and `user_prompt` in full; `user_prompt` embeds
`player_state_yaml`, i.e. the bot's hand. Own-seat only and off by default, but enabling TRACE in a shared
environment during a live game makes the bot's hand readable to anyone with log access. Gate behind an explicit
`BOT_LOG_PROMPTS` flag rather than a log level.

### F-196 (Medium) - WP-62's authoritative-version guard only writes *forward*; deprecating the newest version strands `game_types`

`rust/operator/src/controller.rs:240` runs the descriptive-column update only `if !is_deprecated`, and `apply`
short-circuits on an unchanged generation (`:111-114`):

```rust
if generation.is_some() && generation == observed_generation {
    info!(name, "Spec unchanged since last reconcile, skipping");
    return Ok(requeue_with_jitter());
}
```

**Concrete failing path** (not inferred from the commit message):

1. `lost-cities-1` and `-2` both non-deprecated; `game_types` holds `-2`'s `player_counts = [2,3]`.
2. Flip the `lost-cities-2` CR to `isDeprecated: true`. Its generation bumps, `apply` runs,
   `game_versions.is_deprecated` becomes true, and the guarded UPDATE at `:240` is **skipped** because
   `is_deprecated`. `game_types` still says `[2,3]`.
3. `find_latest_non_deprecated_game_version` (`rust/web/src/db/game_types.rs:28`) now returns `lost-cities-1`, but
   `find_game_type_player_counts` (`:49-57`) reads the shared *type* row and still returns `[2,3]`.
4. `lost-cities-1`'s generation never changes, so its `apply` hits the early return at `:111` forever - the hourly
   requeue does not repair it. A 3-player Lost Cities is accepted by web validation and then rejected by the game
   service.

This is precisely the invariant the WP-62 spec's "Design call" section set out to establish ("makes `game_types`
describe precisely what `find_latest_non_deprecated_game_version` will hand out"). The row satisfies its checklist
literally - the guard exists and is correct for the forward direction - while missing what the guard was for.
`cleanup` (`:174`) has the same shape: it sets `is_public = false` but leaves `is_deprecated = false`, so a deleted
newest version still blocks older ones from reclaiming the type row.

- **Fix**: resolve the authoritative row inside SQL (`ORDER BY created_at DESC, name DESC LIMIT 1` over
  non-deprecated versions of the type) and copy its values unconditionally, so a deprecation flip or a delete on
  any version re-points the type row. Covers `cleanup` in the same change.

## Verified good

- `rust/bot/src/main.rs:525-546` (`load_bot_context`) fetches logs with
  `WHERE game_id = $1 AND (is_public = true OR id IN (SELECT game_log_id FROM game_log_targets WHERE
  game_player_id = $2))` - correctly scoped to public logs plus logs targeted at *this* bot's seat. No
  cross-player private-log leak on the log axis.
- `rust/bot/src/main.rs:108-120` and `:327-350` - the turn is re-checked against `gp.is_turn` both before the LLM
  call and after it, and `:355-384` refreshes the context and clears `failed_commands` if `game_state` moved under
  the bot. Prevents acting on stale state.
- `rust/bot/src/main.rs:967-972` - in-flight turns *are* drained on shutdown via
  `turn_permits.acquire_many(max_concurrent)`. The ws F55 "bot consumer gets no shutdown signal" concern does not
  apply to the bot binary's own loop (`:876-885` selects on `shutdown_signal()`).
- `rust/bot/src/main.rs:936-947` - a 60s `AckKind::Progress` heartbeat keeps long LLM turns from being redelivered
  mid-flight, with a test pinning it below `ack_wait` (see F-188 for the hardcoding caveat).
- `rust/bot/src/main.rs:893-901` - an unparseable `bot.turn` payload is acked and dropped rather than poisoning the
  consumer. Correct: it can never succeed on redelivery.
- `merge_json_patch` (`rust/bot/src/main.rs:623-651`) implements RFC 7396 correctly, including nested-null
  stripping, and its five tests are genuine (each asserts the transformed value, not a tautology).
- `rust/bot/src/main.rs:786` - `send_default_pii: false` on the Sentry client.
- **Obligation 5, gating half**: `rust/lib/cmd/src/test_support.rs` is correctly gated. `lib.rs:15-16` has
  `#[cfg(feature = "test-support")] pub mod test_support;` and `Cargo.toml:23-26` declares
  `default = ["http-server"]` with `test-support = []` **not** in `default`. So the 14 panic constructs cannot
  reach a release build unless a dependent explicitly turns the feature on. All 14 are `panic!`/`assert!` inside
  `assert_gamer_contract`, which is a test harness - panicking is the correct behaviour there, not a defect. The
  residual risk is only "a non-dev dependency enables the feature"; see Worker 2's sweep.
- `rust/lib/game_client/src/lib.rs` retry/backoff tests (`:431-1089`) are genuine, not decoys: each drives a real
  TCP listener and counts attempts (`test_no_retry_on_http_error_response` asserts exactly 1;
  `test_bounded_max_attempts_on_permanent_failure` and `test_crate_timeout_bounds_client_without_timeout` assert
  exactly `max_attempts`). `test_invalid_version_name_rejected_before_send` covers six distinct bad inputs.
- `rust/lib/cmd/src/http.rs:18-23` - `DefaultBodyLimit::max(16 MiB)` with a test asserting 413 at
  `MAX_CONTENT_LENGTH + 1` (`:145-156`).

### Obligation 6 - REFUTED: the `rust/.sqlx` deletion is correct consolidation, not a hazard

Do not re-derive. Evidence:

- At HEAD there is exactly **one** `.sqlx` directory, `rust/web/.sqlx` (137 `query-*.json`). Before WP-52
  (`f374434`) there were two: `rust/.sqlx` (81) and `rust/web/.sqlx` (135).
- **The premise's causality is backwards.** `f374434` (WP-52, 07-28) is an *ancestor* of `667c8f42` (WP-66, 07-29),
  so the deletion is not WP-66 fallout. WP-66 then regenerated **only** `rust/web/.sqlx` (88 paths, all under it).
  Had `rust/.sqlx` survived WP-52 it would today be an 81-entry **sqlx-0.8-format orphan** - deleting it was the
  right call and arguably prevented a hazard.
- **Nothing resolves against the deleted directory.** Only `web` uses the compile-time macros (16 files);
  `bot`, `operator`, `session_store` and `tools/fuzz` have zero `query!`/`query_as!`/`query_scalar!` hits and use
  runtime `sqlx::query(...).bind(...)`, which needs no offline cache. sqlx checks
  `$CARGO_MANIFEST_DIR/.sqlx` first, which for `web` is `rust/web/.sqlx`, so the workspace-root fallback is never
  consulted.
- **`cargo sqlx prepare` targets `rust/web`**, always run from that directory and never with `--workspace`:
  `scripts/rust-ci-commands.sh:24` (`cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets`),
  `docs/DEV.md:73,94`, `Tiltfile:147`. There is no `.cargo/config.toml` anywhere in the tree.
- **CI would catch staleness**: `.github/workflows/ci.yml:52` sets `SQLX_OFFLINE: "true"` for the job and `:94`
  runs the script containing `prepare --check`. `rust/Dockerfile:76` also sets `SQLX_OFFLINE=true`.
- Only residual: WP-52's commit message does not mention removing an 81-file directory. A process nit, not a defect.

### Obligation 5 - DISCHARGED: every `test-support` consumer is a dev-dependency

`rg -n 'test-support' --glob '**/Cargo.toml' rust/` gives 29 hits: the declaration at
`rust/lib/cmd/Cargo.toml:26` plus 28 game crates, each
`brdgme_cmd = { path = "../../lib/cmd", features = ["test-support"] }`. **28/28 sit under `[dev-dependencies]`;
zero under `[dependencies]`.** The 14 panic constructs cannot reach a release build. `assert_gamer_contract` is
called from 28 files (`rust/game/*/tests/contract.rs`) - i.e. every game crate in the workspace. No coverage gap.

### WP-62 `e682f6bc` - what did land correctly

- **bo F18, the finalizer race, is genuinely closed.** `rust/operator/src/controller.rs:77` now dispatches through
  `kube::runtime::finalizer::finalizer(&api, FINALIZER, obj, ...)`; both hand-rolled
  `Patch::Merge(json!({"metadata":{"finalizers": ...}}))` calls are gone. Verified conflict-safe by reading the
  vendored crate (`kube-runtime-4.0.0/src/finalizer.rs:160-207`): it uses `Patch::Json` with
  `PatchOperation::Test` guards. Merge-patch array clobbering is eliminated.
- **The ordering matches its consumer exactly**: the guard's `(newer.created_at, newer.name) > (cur.created_at,
  cur.name)` (`controller.rs:249-260`) is the precise inverse of
  `rust/web/src/db/game_types.rs:37-38`'s `ORDER BY created_at DESC, name DESC`. No tie-break divergence.
- **No pattern-2 sibling**: `interceptor_uri` has one caller (`:116`); both status patches (`:92`, `:164`) use the
  typed `GameVersionStatus`. **No `#[allow(dead_code)]` or zero-caller additions** in the commit.
- **Not a decoy test**: `authoritative_version_wins_regardless_of_order_deprecated_first` (`:400`) would fail under
  the pre-fix `ON CONFLICT ... SET player_counts = EXCLUDED...` at its `:465` assertion.

### WP-63 `d2decf85` - the fuzz hang is genuinely fixed, not mitigated

The bug was the driver holding its own `step_tx`, so the channel never disconnected when all workers panicked.
`rust/tools/fuzz/src/lib.rs:47` is `drop(step_tx)`, with `:85-89` breaking on `Err(_)` from `recv()`. That is the
complete fix - no timeout is involved, so there is no partial-coverage timeout to critique. Every spec item 3a-3f
landed. `fuzz_returns_when_all_workers_exit` is **not** a decoy: its `StubRequester` returns
`Err(RequestError::Stdin)`, making `Fuzzer::try_new` fail and every worker panic at `.expect(...)` (`:35`); without
`drop(step_tx)` the driver blocks in `recv()` and the `recv_timeout(10s)` assertion fails. Residuals below are all
covered by the spec's explicit non-goal ("do not restructure worker shutdown beyond 3a/3e").

## Coverage gaps

1. **Zero tests on `build_messages`** (`rust/bot/src/main.rs:555-617`). The bot's whole test module (`:978-1043`)
   covers only `merge_json_patch` and the ack-heartbeat constant. The entire state -> prompt wiring, the only place
   a redaction mistake could be introduced, is untested.
2. **New decoy test, confirmed instance of the class**: `render_user_includes_state_in_yaml_fences`
   (`rust/bot/src/prompt.rs:291-302`) name-matches state-redaction coverage, but its fixture `user_ctx()`
   (`:163-190`) hand-writes `pub_state_yaml` and `player_state_yaml` as string literals (`:181-182`) and the test
   asserts only that those literals land inside ```` ```yaml ```` fences. Swapping `player_state_yaml` in
   `build_messages` for another seat's state - or for the raw `game_state` - would pass every test in the file.
   Textbook shape: the input contains an independently passing result.
3. **`fetch_game_data`'s tests are one assertion short of being real** (`rust/lib/game_client/src/lib.rs:832-867`).
   The mock has two seats with deliberately distinct hands (`["A","K"]` vs `["Q"]`) but only the positive is
   asserted (`contains("score: 10")`). Adding `assert!(!data.player_state_yaml.contains("Q"))` would turn it into a
   genuine seat-selection test. Cheapest high-value test in the unit.
4. **Nothing anywhere asserts the negative** - "opponent hidden state does not appear in the rendered user prompt".
   A `build_messages` test with a two-player mock whose seat-1 `player_state` carries a sentinel token, asserting
   its absence from the output, is ~30 lines.
5. **No test on the log SQL filter**, in either the bot or the web copy: nothing inserts a private log targeted at
   another player and asserts exclusion. The bot crate has no DB tests at all.
6. **No test for the F-189 case-mismatch path** on the bot side, and none for the silent-`Ok(())` skip.
7. **No round-trip test between the two `Bot*Event` definitions** (F-188).
8. **No envelope-level test on `route::<G>()`** (F-191).
9. **`rust/operator/src/controller.rs`**: no test flips an already-newest version to deprecated (the F-196
   sequence), and `cleanup` (`:174`) has **zero test callers**.
10. **`rust/tools/fuzz/src/lib.rs`**: `recv()` at `:53-57` has no timeout, so a worker wedged *inside*
    `requester.request()` (child binary alive but silent) still hangs the driver. Different trigger from bo F26 and
    not claimed fixed by WP-63; would need `recv_timeout`. Also `:37-39`: surviving workers panic on
    `step_tx.send(...).expect(...)` after the driver breaks on a real find - noisy stderr on every finding, not a
    hang. Both are explicit spec non-goals.
11. **Unit 10b's 10 unreviewed commits** (see Progress).

## Carry-forwards for the unified report

- **F-186 + F-187 + F-188 remediate as ONE item**: extract a shared crate between `rust/bot` and `rust/web`. F-90
  and F-108 are both still open at HEAD.
- **F-189 remediates as ONE item with F-104, F-138 and F-183**, per the Unit 09c ruling. F-189 adds a **second
  case-sensitive site** (`rust/bot/src/config.rs:67`) that the F-183 write-up did not have, plus the silent
  `Ok(())` ack at `rust/bot/src/main.rs:186-194`. Both must be in the same change or the fix is incomplete.
- **F-192 + F-193 remediate together** and belong in the hidden-information section alongside F-22/F-28, not in a
  logging section: they are a third-party egress for *all* seats' private state.
- **New decoy test for the tally** (`render_user_includes_state_in_yaml_fences`) - the class is now confirmed in the
  bot crate too, not just web/games.
- **F-196 is a fresh instance of the "satisfied the row literally, missed what it was for" pattern**, and of
  pattern 2 in its cross-file form (the guard matches its consumer's ordering exactly, but only in one direction).
- **The `#[allow(dead_code)]` sweep** flagged for sign-off: `rust/bot/src/main.rs:4-7` applies it at **module**
  granularity to `mod config` and `mod crypto`, with a comment justifying it. That is broader than the F-153/F-170
  cases and would hide any genuinely dead item in either module - including `crypto::encrypt` and
  `LoadedKey::is_default`, whose only production caller is the F-186 warning.
