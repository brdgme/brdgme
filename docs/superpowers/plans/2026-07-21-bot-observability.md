# Bot Observability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add structured start/end logging with elapsed timing to the bot service at two levels: turn-level and LLM-request-level.

**Architecture:** Instrument the existing `run_bot_turn` and its retry loop in `rust/bot/src/main.rs` using `tracing` structured fields and `std::time::Instant`. No new files, no new dependencies. A tracing span on `run_bot_turn` carries identity fields; explicit start/end events at turn and LLM-request boundaries carry elapsed_ms.

**Tech Stack:** Rust, tracing crate (already a dependency), std::time::Instant

## Global Constraints

- Target single package: `cargo build/check/clippy -p bot`
- No new dependencies
- All fields emitted as tracing structured fields, not interpolated into message strings
- No comments added to code
- Run `cargo fmt --all -- --check` and `cargo clippy -p bot --all-targets -- -D warnings` before finishing

---

### Task 1: Turn-level span and start/end logging

**Files:**
- Modify: `rust/bot/src/main.rs:69-406` (the `run_bot_turn` function)

**Interfaces:**
- Consumes: `BotTurnEvent` fields (`game_id`, `player_position`, `bot_name`, `attempt`)
- Produces: structured log events `bot_turn_start` and `bot_turn_end`

- [ ] **Step 1: Add a tracing span and start log to `run_bot_turn`**

Replace the function signature and the first log statement (lines 69-153) with a span-instrumented version. The span carries identity fields so all child logs inherit them. The existing `tracing::info!` at line 143 ("Bot turn triggered") becomes the `bot_turn_start` event.

Change the function signature to add `#[tracing::instrument]`:

```rust
#[tracing::instrument(
    name = "bot_turn",
    skip(state, req),
    fields(
        trace_id = %trace_id,
        game_id = %req.game_id,
        player_position = req.player_position,
        bot_name = %req.bot_name,
        nat_attempt = req.attempt,
    )
)]
async fn run_bot_turn(state: &AppState, req: BotTurnEvent, trace_id: Uuid) -> Result<()> {
```

Add `use std::time::Instant;` at the top of the file (near line 12, alongside the other `use` statements).

Immediately after the function opening brace (before the first DB query), add:

```rust
    let turn_start = Instant::now();
```

Replace the existing `tracing::info!` block at lines 143-153 with:

```rust
    tracing::info!(
        game = %game_name,
        player_name = %bot_name,
        players = ?names,
        "bot_turn_start"
    );
```

- [ ] **Step 2: Add `bot_turn_end` at the success exit point**

At the success path (currently lines 356-373, inside the `Ok(Response::Play { .. })` arm), replace the existing `tracing::info!` and `publish_bot_command` + `return Ok(())` with:

```rust
            Ok(Response::Play { .. }) => {
                publish_bot_command(
                    state,
                    req.game_id,
                    req.player_position,
                    command.clone(),
                    req.attempt,
                )
                .await?;
                tracing::info!(
                    elapsed_ms = turn_start.elapsed().as_millis() as u64,
                    outcome = "success",
                    command = %command,
                    "bot_turn_end"
                );
                return Ok(());
            }
```

- [ ] **Step 3: Add `bot_turn_end` at the hard-failure exit point**

At the max-attempts-exhausted path (currently lines 380-387), replace the existing `return Err(...)` with:

```rust
        if attempt + 1 == MAX_ATTEMPTS {
            let elapsed_ms = turn_start.elapsed().as_millis() as u64;
            tracing::error!(
                elapsed_ms,
                outcome = "failure",
                attempts = MAX_ATTEMPTS,
                last_command = %command,
                last_error = %error_body,
                "bot_turn_end"
            );
            return Err(anyhow!(
                "Command rejected after {} attempts. Last command: {:?}. Last error: {}",
                MAX_ATTEMPTS,
                command,
                error_body
            ));
        }
```

- [ ] **Step 4: Add `bot_turn_end` at the early-exit "no longer active" paths**

There are two early exits where the turn is no longer active (lines 92-100 and lines 299-309). These are not failures - the game state changed. Add a log before each `return Ok(())`:

At the first early exit (line 92-100, `is_turn` check at function top):

```rust
    if !is_turn {
        tracing::info!(
            elapsed_ms = turn_start.elapsed().as_millis() as u64,
            outcome = "skipped",
            reason = "turn no longer active",
            "bot_turn_end"
        );
        return Ok(());
    }
```

At the second early exit (lines 299-309, re-check after LLM responds):

```rust
        if !is_still_turn {
            tracing::info!(
                elapsed_ms = turn_start.elapsed().as_millis() as u64,
                outcome = "skipped",
                reason = "game state changed while LLM thinking",
                "bot_turn_end"
            );
            return Ok(());
        }
```

- [ ] **Step 5: Add `bot_turn_end` at the "bot not found" and "no providers" early exits**

At the "bot not found or disabled" exit (line 172-179):

```rust
                tracing::info!(
                    elapsed_ms = turn_start.elapsed().as_millis() as u64,
                    outcome = "skipped",
                    reason = "bot not found or disabled",
                    "bot_turn_end"
                );
                return Ok(());
```

At the "no providers" exit (lines 195-200), before the `return Err(...)`:

```rust
    if providers.is_empty() {
        tracing::error!(
            elapsed_ms = turn_start.elapsed().as_millis() as u64,
            outcome = "failure",
            reason = "no LLM providers configured",
            "bot_turn_end"
        );
        return Err(anyhow!(
            "No LLM providers configured for bot {}",
            req.bot_name
        ));
    }
```

- [ ] **Step 6: Verify compilation and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p bot --all-targets -- -D warnings`
Expected: no errors, no warnings.

- [ ] **Step 7: Commit**

```bash
git add rust/bot/src/main.rs
git commit -m "feat(bot): turn-level structured start/end logging with elapsed timing"
```

---

### Task 2: LLM request-level start/end logging

**Files:**
- Modify: `rust/bot/src/main.rs` (the retry loop inside `run_bot_turn`, around lines 222-274)

**Interfaces:**
- Consumes: `provider.url`, `provider.model`, loop `attempt` index
- Produces: structured log events `llm_request_start` and `llm_request_end`

- [ ] **Step 1: Add `llm_request_start` before the `call_llm` invocation**

Inside the retry loop, after the provider fields are extracted (lines 223-230) and before `build_messages` is called, add:

```rust
        tracing::info!(
            provider_url = %url,
            model = %model,
            attempt,
            "llm_request_start"
        );
```

- [ ] **Step 2: Add timing around `call_llm` and log `llm_request_end`**

Wrap the `call_llm` invocation (currently lines 249-274) with an `Instant` and replace the existing match arms:

```rust
        let llm_start = Instant::now();
        let raw_response = match call_llm(
            &state.http,
            &url,
            &model,
            &messages,
            api_key.as_deref(),
            bot_cfg.temperature,
            reasoning_effort,
            extra_body.as_ref(),
        )
        .await
        {
            Ok(r) => {
                tracing::info!(
                    provider_url = %url,
                    model = %model,
                    attempt,
                    elapsed_ms = llm_start.elapsed().as_millis() as u64,
                    outcome = "success",
                    "llm_request_end"
                );
                r
            }
            Err(e) => {
                tracing::warn!(
                    provider_url = %url,
                    model = %model,
                    attempt,
                    elapsed_ms = llm_start.elapsed().as_millis() as u64,
                    outcome = "error",
                    error = %e,
                    "llm_request_end"
                );
                router.mark_failed();
                continue;
            }
        };
```

Remove the old `tracing::info!` at lines 276-282 ("LLM response received") and the old `tracing::warn!` at lines 263-273 ("LLM API error, failing over") since they are replaced by the above.

- [ ] **Step 3: Verify compilation and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p bot --all-targets -- -D warnings`
Expected: no errors, no warnings.

- [ ] **Step 4: Commit**

```bash
git add rust/bot/src/main.rs
git commit -m "feat(bot): LLM request-level start/end logging with provider, model, elapsed"
```

---

### Task 3: Game service call timing

**Files:**
- Modify: `rust/bot/src/main.rs` (the `load_bot_context` call sites and the Play-validation call)

**Interfaces:**
- Consumes: `load_bot_context` and `brdgme_game_client::request` return values
- Produces: `elapsed_ms` field on existing log events near those calls

- [ ] **Step 1: Add elapsed timing to the initial `load_bot_context` call**

Wrap the initial `load_bot_context` call (lines 205-217) with timing:

```rust
    let ctx_start = Instant::now();
    let mut bot_ctx = load_bot_context(
        state,
        &game_service_uri,
        &version_name,
        req.game_id,
        req.player_position,
        game_player_id,
        game_state,
        &names,
        interface_version,
    )
    .await
    .context("Failed to load initial bot context")?;
    tracing::info!(
        elapsed_ms = ctx_start.elapsed().as_millis() as u64,
        phase = "load_context",
        "game_service_call"
    );
```

- [ ] **Step 2: Add elapsed timing to the Play-validation call**

Wrap the `brdgme_game_client::request` call (lines 342-353) with timing:

```rust
        let validate_start = Instant::now();
        let validate_result = brdgme_game_client::request(
            &state.game_http,
            &game_service_uri,
            &version_name,
            &Request::Play {
                player: req.player_position as usize,
                game: bot_ctx.game_state.clone(),
                command: command.clone(),
                names: names.clone(),
            },
        )
        .await;
        tracing::info!(
            elapsed_ms = validate_start.elapsed().as_millis() as u64,
            phase = "validate_command",
            "game_service_call"
        );
```

- [ ] **Step 3: Add elapsed timing to the context-refresh call inside the retry loop**

The context refresh (lines 322-334) also calls `load_bot_context`. Wrap it:

```rust
            let refresh_start = Instant::now();
            bot_ctx = load_bot_context(
                state,
                &game_service_uri,
                &version_name,
                req.game_id,
                req.player_position,
                game_player_id,
                current_game_state,
                &names,
                interface_version,
            )
            .await
            .context("Failed to refresh bot context")?;
            tracing::info!(
                elapsed_ms = refresh_start.elapsed().as_millis() as u64,
                phase = "refresh_context",
                "game_service_call"
            );
```

- [ ] **Step 4: Verify compilation and lint**

Run: `cargo fmt --all -- --check && cargo clippy -p bot --all-targets -- -D warnings`
Expected: no errors, no warnings.

- [ ] **Step 5: Commit**

```bash
git add rust/bot/src/main.rs
git commit -m "feat(bot): game service call timing for context load and validation"
```

---

### Task 4: Final verification

- [ ] **Step 1: Run full lint pass**

Run: `cargo fmt --all -- --check && cargo clippy -p bot --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 2: Verify log output shape with a dry build**

Run: `cargo build -p bot`
Expected: compiles successfully.

- [ ] **Step 3: Manual verification note**

After deploying to beta, trigger a bot game and confirm `kubectl logs` shows:
- `bot_turn_start` with trace_id, game_id, player_position, bot_name
- `llm_request_start` / `llm_request_end` with provider_url, model, attempt, elapsed_ms
- `game_service_call` with phase and elapsed_ms
- `bot_turn_end` with elapsed_ms and outcome
