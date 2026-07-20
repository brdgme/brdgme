# Bot Efficacy (#43) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the brdgme markup render with structured YAML data in bot prompts, make difficulty a full bot configuration (model, provider, strategy docs), and support multi-provider failover.

**Architecture:** The bot crate reads bot/provider config from Postgres (with env-var fallback for dev), fetches structured game data via a version-aware `game_client`, and builds a split prompt (static system + dynamic user). Games expose V2 endpoints (data_docs, basic_strategy, advanced_strategy) through the existing HTTP contract. The operator stores `interface_version` from the CRD so `game_client` can abstract V1/V2 differences.

**Tech Stack:** Rust (edition 2024), sqlx/Postgres, AES-256-GCM (aes-gcm crate), MiniJinja templates, serde_yaml, kube-rs operator, NATS JetStream, warp HTTP server (games), reqwest (bot/operator clients).

## Global Constraints

- NO COMMITS OR PUSHES - the user manages all git operations.
- Target single packages: `cargo build/check/clippy/test -p <crate>`. Never run workspace-wide builds.
- DB test failures are pre-existing (backlog #40) - do not chase them.
- `SQLX_OFFLINE=true` for web crate checks (no live DB in agent runs).
- Edition 2024 for all crates.
- Follow existing patterns in the codebase (naming, error handling, module structure).
- wasm-bindgen pinned to =0.2.121 - do not touch.
- Migration files are immutable once applied - new work goes in a new numbered migration.
- The next migration number is 013.

---

### Task 1: DB Migration

**Files:**
- Create: `rust/web/migrations/013_bot_efficacy.sql`

**Interfaces:**
- Consumes: nothing (first task)
- Produces: `bots`, `llm_providers`, `bot_providers` tables; `game_bots.bot_name` column (replaces `difficulty`); `game_versions.interface_version` column. Seed data for easy/medium/hard bots.

- [ ] **Step 1: Write the migration file**

Create `rust/web/migrations/013_bot_efficacy.sql`:

```sql
-- Bot efficacy: bot configuration tables, game_bots.difficulty -> bot_name,
-- game_versions.interface_version.

CREATE TABLE bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    display_order INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    include_basic_strategy BOOLEAN NOT NULL DEFAULT true,
    include_advanced_strategy BOOLEAN NOT NULL DEFAULT false,
    temperature REAL NOT NULL DEFAULT 0.2,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER update_bots_updated_at
    BEFORE UPDATE ON bots
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TABLE llm_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    api_key_encrypted BYTEA,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER update_llm_providers_updated_at
    BEFORE UPDATE ON llm_providers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TABLE bot_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES llm_providers(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    reasoning_effort TEXT,
    extra_body JSONB,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (bot_id, provider_id, model)
);

-- Rename game_bots.difficulty -> bot_name, drop the CHECK constraint.
ALTER TABLE game_bots DROP CONSTRAINT IF EXISTS game_bots_difficulty_check;
ALTER TABLE game_bots RENAME COLUMN difficulty TO bot_name;

-- Interface version for game service contract evolution.
ALTER TABLE game_versions
    ADD COLUMN IF NOT EXISTS interface_version INTEGER NOT NULL DEFAULT 1;

-- Seed bot configurations matching the previous easy/medium/hard difficulties.
INSERT INTO bots (name, display_order, include_basic_strategy, include_advanced_strategy, temperature)
VALUES
    ('easy',   0, true,  false, 0.2),
    ('medium', 1, true,  false, 0.2),
    ('hard',   2, true,  true,  0.2)
ON CONFLICT (name) DO NOTHING;
```

- [ ] **Step 2: Verify migration syntax**

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr`
Expected: compiles (sqlx offline mode does not validate migration SQL at compile time, but confirms no Rust breakage from the schema change yet - that comes in Task 7).

Note: The `game_bots.difficulty` -> `bot_name` rename will break existing sqlx macros in the web crate. That is expected and fixed in Task 7. For now, verify the SQL is syntactically valid by inspection. The migration itself is validated when applied to a real database.

---

### Task 2: Encryption Module

**Files:**
- Create: `rust/bot/src/crypto.rs`
- Modify: `rust/bot/src/main.rs` (add `mod crypto;`)
- Modify: `rust/bot/Cargo.toml` (add `aes-gcm`, `base64` deps)

**Interfaces:**
- Consumes: `BOT_ENCRYPTION_KEY` env var (32 bytes, base64-encoded)
- Produces: `pub fn decrypt_api_key(encrypted: &[u8], key: &[u8; 32]) -> Result<String>` and `pub fn encrypt_api_key(plaintext: &str, key: &[u8; 32]) -> Result<Vec<u8>>`

- [ ] **Step 1: Add dependencies to bot Cargo.toml**

Add to `[dependencies]` in `rust/bot/Cargo.toml`:

```toml
aes-gcm = "0.10"
base64 = "0.22"
```

- [ ] **Step 2: Create the crypto module**

Create `rust/bot/src/crypto.rs`:

```rust
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm::aead::rand_core::RngCore;
use anyhow::{Context, Result, anyhow};

const NONCE_LEN: usize = 12;

pub fn load_key() -> Result<[u8; 32]> {
    let raw = std::env::var("BOT_ENCRYPTION_KEY")
        .context("BOT_ENCRYPTION_KEY must be set")?;
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &raw,
    )
    .context("BOT_ENCRYPTION_KEY must be valid base64")?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("BOT_ENCRYPTION_KEY must decode to exactly 32 bytes"))?;
    Ok(key)
}

pub fn encrypt_api_key(plaintext: &str, key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("encryption failed: {}", e))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_api_key(encrypted: &[u8], key: &[u8; 32]) -> Result<String> {
    if encrypted.len() < NONCE_LEN + 1 {
        return Err(anyhow!("ciphertext too short"));
    }
    let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("decryption failed: {}", e))?;
    String::from_utf8(plaintext).context("decrypted key is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = [42u8; 32];
        let encrypted = encrypt_api_key("sk-secret-key-123", &key).unwrap();
        let decrypted = decrypt_api_key(&encrypted, &key).unwrap();
        assert_eq!(decrypted, "sk-secret-key-123");
    }

    #[test]
    fn wrong_key_fails() {
        let key = [42u8; 32];
        let wrong_key = [99u8; 32];
        let encrypted = encrypt_api_key("sk-secret", &key).unwrap();
        assert!(decrypt_api_key(&encrypted, &wrong_key).is_err());
    }

    #[test]
    fn too_short_fails() {
        let key = [42u8; 32];
        assert!(decrypt_api_key(&[0u8; 5], &key).is_err());
    }
}
```

- [ ] **Step 3: Register the module in main.rs**

Add `mod crypto;` at the top of `rust/bot/src/main.rs` (after `mod prompt;`).

- [ ] **Step 4: Verify**

Run: `cargo test -p bot`
Expected: 3 crypto tests pass (plus existing tests).

Run: `cargo clippy -p bot`
Expected: no warnings.

---

### Task 3: game_client Version Awareness

**Files:**
- Modify: `rust/lib/game_client/src/lib.rs`
- Modify: `rust/lib/game_client/Cargo.toml` (add `serde_yaml` dep)

**Interfaces:**
- Consumes: `brdgme_cmd::api::{Request, Response}` (existing), new V2 Request/Response variants (from Task 4)
- Produces: `pub struct GameData` and `pub async fn game_data(...) -> Result<GameData>`

- [ ] **Step 1: Add serde_yaml dependency**

Add to `[dependencies]` in `rust/lib/game_client/Cargo.toml`:

```toml
serde_yaml = "0.9"
```

- [ ] **Step 2: Add GameData struct and version-aware fetch function**

Add to `rust/lib/game_client/src/lib.rs` (after the existing `RenderResponse` section):

```rust
#[derive(Debug, Clone)]
pub struct GameData {
    pub pub_state_yaml: String,
    pub player_state_yaml: String,
    pub data_docs: String,
    pub basic_strategy: String,
    pub advanced_strategy: String,
    pub command_spec: Option<brdgme_game::command::Spec>,
    pub rules: String,
}

const V1_PLACEHOLDER: &str = "Not supported in game interface V1";

fn json_to_yaml(json_str: &str) -> String {
    let value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return json_str.to_string(),
    };
    serde_yaml::to_string(&value).unwrap_or_else(|_| json_str.to_string())
}

pub async fn game_data(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    game_state: &str,
    player: usize,
    interface_version: i32,
) -> Result<GameData> {
    let status_resp = request(
        client,
        uri,
        version_name,
        &Request::Status {
            game: game_state.to_string(),
        },
    )
    .await
    .context("Status call failed")?;

    let (pub_state_json, player_state_json, command_spec, points) = match status_resp {
        Response::Status {
            game,
            public_render,
            player_renders,
        } => {
            let pr = player_renders
                .into_iter()
                .nth(player)
                .ok_or_else(|| anyhow!("player position out of range"))?;
            (
                public_render.pub_state,
                pr.player_state,
                pr.command_spec,
                game.points,
            )
        }
        _ => return Err(anyhow!("unexpected response to Status")),
    };

    let pub_state_yaml = json_to_yaml(&pub_state_json);
    let player_state_yaml = json_to_yaml(&player_state_json);

    let (data_docs, basic_strategy, advanced_strategy) = if interface_version >= 2 {
        let docs = match request(client, uri, version_name, &Request::DataDocs).await? {
            Response::DataDocs { data_docs } => data_docs,
            _ => String::new(),
        };
        let basic = match request(client, uri, version_name, &Request::BasicStrategy).await? {
            Response::BasicStrategy { strategy } => strategy,
            _ => String::new(),
        };
        let advanced =
            match request(client, uri, version_name, &Request::AdvancedStrategy).await? {
                Response::AdvancedStrategy { strategy } => strategy,
                _ => String::new(),
            };
        (docs, basic, advanced)
    } else {
        (
            V1_PLACEHOLDER.to_string(),
            V1_PLACEHOLDER.to_string(),
            V1_PLACEHOLDER.to_string(),
        )
    };

    let rules = match request(client, uri, version_name, &Request::Rules).await? {
        Response::Rules { rules } => rules,
        _ => String::new(),
    };

    Ok(GameData {
        pub_state_yaml,
        player_state_yaml,
        data_docs,
        basic_strategy,
        advanced_strategy,
        command_spec,
        rules,
    })
}
```

Note: `points` is extracted from the Status response but not stored in `GameData` - the bot fetches points separately from the DB (it already does this). If needed later, add a `points` field.

- [ ] **Step 3: Verify compilation**

This will NOT compile yet because `Request::DataDocs`, `Request::BasicStrategy`, `Request::AdvancedStrategy` and their Response variants do not exist. Those are added in Task 4. Verify after Task 4 is complete.

Run (after Task 4): `cargo check -p brdgme_game_client`
Expected: compiles cleanly.

---

### Task 4: Gamer Trait V2 + HTTP Contract

**Files:**
- Modify: `rust/lib/game/src/game.rs` (add trait methods)
- Modify: `rust/lib/cmd/src/api.rs` (add Request/Response variants)
- Modify: `rust/lib/cmd/src/requester/gamer.rs` (handle new variants)

**Interfaces:**
- Consumes: nothing new (extends existing traits)
- Produces: `Gamer::data_docs()`, `Gamer::basic_strategy()`, `Gamer::advanced_strategy()` trait methods; `Request::DataDocs`, `Request::BasicStrategy`, `Request::AdvancedStrategy` variants; `Response::DataDocs`, `Response::BasicStrategy`, `Response::AdvancedStrategy` variants.

- [ ] **Step 1: Add default methods to the Gamer trait**

In `rust/lib/game/src/game.rs`, add three methods to the `Gamer` trait (after `fn rules()`):

```rust
    fn data_docs() -> String {
        String::new()
    }

    fn basic_strategy() -> String {
        String::new()
    }

    fn advanced_strategy() -> String {
        String::new()
    }
```

- [ ] **Step 2: Add V2 Request variants to api.rs**

In `rust/lib/cmd/src/api.rs`, add to the `Request` enum (after `Rules`):

```rust
    DataDocs,
    BasicStrategy,
    AdvancedStrategy,
```

- [ ] **Step 3: Add V2 Response variants to api.rs**

In `rust/lib/cmd/src/api.rs`, add to the `Response` enum (after `Rules { rules: String }`):

```rust
    DataDocs {
        data_docs: String,
    },
    BasicStrategy {
        strategy: String,
    },
    AdvancedStrategy {
        strategy: String,
    },
```

- [ ] **Step 4: Handle new variants in GameRequester**

In `rust/lib/cmd/src/requester/gamer.rs`, add match arms to the `request` method (after `Request::Rules => Ok(handle_rules::<G>())`):

```rust
            Request::DataDocs => Ok(Response::DataDocs {
                data_docs: G::data_docs(),
            }),
            Request::BasicStrategy => Ok(Response::BasicStrategy {
                strategy: G::basic_strategy(),
            }),
            Request::AdvancedStrategy => Ok(Response::AdvancedStrategy {
                strategy: G::advanced_strategy(),
            }),
```

- [ ] **Step 5: Verify**

Run: `cargo check -p brdgme_game -p brdgme_cmd`
Expected: compiles cleanly.

Run: `cargo clippy -p brdgme_game -p brdgme_cmd`
Expected: no warnings.

Run: `cargo check -p brdgme_game_client`
Expected: compiles cleanly (Task 3's code now resolves).

Run: `cargo test -p brdgme_cmd`
Expected: all tests pass.

---

### Task 5: Operator + CRD Update

**Files:**
- Modify: `rust/operator/src/crd.rs` (add `interface_version` field)
- Modify: `rust/operator/src/controller.rs` (pass `interface_version` to upsert)
- Modify: `k8s/base/operator/crd.yaml` (add `interfaceVersion` to schema)

**Interfaces:**
- Consumes: `game_versions.interface_version` column (from Task 1)
- Produces: operator stores `interface_version` from CRD spec into the DB on reconcile.

- [ ] **Step 1: Add interfaceVersion to the CRD Rust struct**

In `rust/operator/src/crd.rs`, add to `GameVersionSpec`:

```rust
    /// Game service interface version (1 = legacy, 2 = structured data + strategies).
    #[serde(default = "default_interface_version")]
    pub interface_version: i32,
```

Add the default function (above or below the struct):

```rust
fn default_interface_version() -> i32 {
    1
}
```

- [ ] **Step 2: Update the CRD YAML schema**

In `k8s/base/operator/crd.yaml`, add under `spec.properties`:

```yaml
              interfaceVersion:
                type: integer
                default: 1
                description: "Game service interface version (1 = legacy, 2 = structured data + strategies)."
```

- [ ] **Step 3: Pass interface_version through the controller**

In `rust/operator/src/controller.rs`, update the `upsert_game_type_and_version` call in `reconcile` to pass `obj.spec.interface_version`:

```rust
    upsert_game_type_and_version(
        &ctx.pool,
        &obj.spec.type_name,
        &player_counts,
        obj.spec.weight,
        &obj.spec.blurb,
        &name,
        &uri,
        obj.spec.is_deprecated,
        &rules,
        obj.spec.interface_version,
    )
    .await?;
```

- [ ] **Step 4: Update the upsert function signature and SQL**

In `rust/operator/src/controller.rs`, update `upsert_game_type_and_version`:

Add parameter `interface_version: i32` to the function signature.

Update the INSERT/UPSERT SQL:

```rust
    sqlx::query(
        r#"
        INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated, rules, interface_version)
        VALUES ($1, $2, $3, true, $4, $5, $6)
        ON CONFLICT (game_type_id, name) DO UPDATE
            SET uri               = EXCLUDED.uri,
                is_public         = true,
                is_deprecated     = EXCLUDED.is_deprecated,
                rules             = EXCLUDED.rules,
                interface_version = EXCLUDED.interface_version,
                updated_at        = NOW()
        "#,
    )
    .bind(game_type_id)
    .bind(version_name)
    .bind(uri)
    .bind(is_deprecated)
    .bind(rules)
    .bind(interface_version)
    .execute(pool)
    .await?;
```

- [ ] **Step 5: Update the existing test**

In `rust/operator/src/controller.rs`, update the `upsert_writes_weight_and_blurb` test to pass the new parameter:

```rust
        upsert_game_type_and_version(
            &pool,
            "Test Game",
            &[2, 3],
            2.5,
            "A test blurb.",
            "test-game-1",
            "http://localhost:0/mock",
            false,
            "rules text",
            1,
        )
        .await
        .unwrap();
```

And the second call in the same test:

```rust
        upsert_game_type_and_version(
            &pool,
            "Test Game",
            &[2, 3],
            3.0,
            "New blurb.",
            "test-game-1",
            "http://localhost:0/mock",
            false,
            "rules text",
            2,
        )
        .await
        .unwrap();
```

Add an assertion after the second upsert:

```rust
        let iv: i32 =
            sqlx::query_scalar("SELECT interface_version FROM game_versions WHERE name = 'test-game-1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(iv, 2);
```

- [ ] **Step 6: Verify**

Run: `cargo check -p brdgme-operator`
Expected: compiles cleanly.

Run: `cargo clippy -p brdgme-operator`
Expected: no warnings.

Note: `cargo test -p brdgme-operator` requires a database (sqlx::test). DB test failures are pre-existing (backlog #40).

---

### Task 6: Bot Crate Restructure

**Files:**
- Modify: `rust/bot/src/main.rs` (major restructure: DB config, provider routing, GameData fetch, new prompt)
- Modify: `rust/bot/src/prompt.rs` (new template context, split system/user)
- Modify: `rust/bot/src/nats.rs` (rename `difficulty` -> `bot_name` in `BotTurnEvent`)
- Create: `rust/bot/system_prompt.md` (rewrite - static system template)
- Create: `rust/bot/user_prompt.md` (new - dynamic user template)
- Modify: `rust/bot/Cargo.toml` (add `serde_yaml` already present; no new deps beyond Task 2)

**Interfaces:**
- Consumes: `bots`/`llm_providers`/`bot_providers` tables (Task 1), `crypto::decrypt_api_key` (Task 2), `brdgme_game_client::game_data` + `GameData` (Task 3), `BotTurnEvent.bot_name` (Task 7 renames the monolith side)
- Produces: bot makes plays using structured data, provider routing, and the new prompt.

- [ ] **Step 1: Rename difficulty -> bot_name in nats.rs**

In `rust/bot/src/nats.rs`, change `BotTurnEvent`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotTurnEvent {
    pub game_id: Uuid,
    pub player_position: i32,
    pub bot_name: String,
    pub attempt: i32,
}
```

- [ ] **Step 2: Rewrite the system prompt template**

Replace `rust/bot/system_prompt.md` with:

```markdown
# Persona

You are an expert board gamer. Play to win.

# Task

Respond with exactly one valid command as a single line of plain text. No explanation.

# Game Rules

{{ game_rules }}

{% if basic_strategy %}
# Basic Strategy

{{ basic_strategy }}
{% endif %}

{% if advanced_strategy %}
# Advanced Strategy

{{ advanced_strategy }}
{% endif %}

# Data Dictionary

The game state is provided as YAML. Each field is documented below:

{{ data_docs }}

# Command Parser Rules

You will be provided structured command parser rules in YAML format. Commands are case insensitive.

## Token

Represents a single token, and requires a full match.

```yaml
Token: sometext
```

## OneOf

An array of child parsers, and parses the first that matches.

```yaml
OneOf:
  - Token: one
  - Token: two
```

## Doc

A wrapper parser that adds helpful documentation.

```yaml
Doc:
  name: buy
  desc: buy shares
  spec:
    Token: buy
```

## Space

Parses one or more whitespace characters.

```yaml
Space
```

## Chain

Parsers run sequentially. All must succeed.

```yaml
Chain:
  - Token: one
  - Space
  - Token: two
```

## Opt

Allows the child parser to fail (consumes nothing).

```yaml
Opt:
  Token: blah
```

## Enum

Parses one of a set of values. `exact: false` allows partial unique matches.

```yaml
Enum:
  values:
    - one
    - two
  exact: false
```

## Int

Parses an integer between optional min and max.

```yaml
Int:
  min: 3
  max: 5
```

## Player

Parses a player name from the list of players (partial unique matches allowed).

```yaml
Player
```

## Many

Parses a child parser multiple times with optional delimiter, min, and max.

```yaml
Many:
  spec:
    Token: x
  min: 1
  max: 3
  delim: Space
```
```

- [ ] **Step 3: Create the user prompt template**

Create `rust/bot/user_prompt.md`:

```markdown
# Players

You are **{{ my_name }}**.

{% for player in players %}
- {{ player.name }}{% if player.name == my_name %} (you){% endif %} - Score: {{ player.score }}
{% endfor %}

# Public Game State

```yaml
{{ pub_state_yaml }}
```

# Your Player State

```yaml
{{ player_state_yaml }}
```

# Command Parser Rules (your current turn)

```yaml
{{ command_spec }}
```

{% if recent_logs %}
# Recent Game Logs

{% for log in recent_logs %}
- {{ log }}
{% endfor %}
{% endif %}

{% if failed_commands %}
# Your Previous Failed Commands

{% for failed in failed_commands %}
- Command: {{ failed.command }}
  - Error: {{ failed.error }}
{% endfor %}
{% endif %}

Please provide your command now.
```

- [ ] **Step 4: Rewrite prompt.rs**

Replace `rust/bot/src/prompt.rs` with:

```rust
use brdgme_game::command::Spec;
use minijinja::{Environment, context};
use serde::Serialize;

const SYSTEM_TEMPLATE: &str = include_str!("../system_prompt.md");
const USER_TEMPLATE: &str = include_str!("../user_prompt.md");

#[derive(Debug, Serialize)]
pub struct PlayerInfo {
    pub name: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct FailedCommand {
    pub command: String,
    pub error: String,
}

#[derive(Debug)]
pub struct SystemContext {
    pub game_rules: String,
    pub basic_strategy: String,
    pub advanced_strategy: String,
    pub data_docs: String,
}

#[derive(Debug)]
pub struct UserContext {
    pub my_name: String,
    pub players: Vec<PlayerInfo>,
    pub pub_state_yaml: String,
    pub player_state_yaml: String,
    pub command_spec: String,
    pub recent_logs: Vec<String>,
    pub failed_commands: Vec<FailedCommand>,
}

pub fn spec_to_yaml(spec: &Spec) -> String {
    let json_val = serde_json::to_value(spec).unwrap_or_default();
    serde_yaml::to_string(&json_val).unwrap_or_default()
}

pub fn render_system_prompt(ctx: &SystemContext) -> Result<String, minijinja::Error> {
    let mut env = Environment::new();
    env.add_template("system", SYSTEM_TEMPLATE)?;
    let tmpl = env.get_template("system")?;
    tmpl.render(context! {
        game_rules => &ctx.game_rules,
        basic_strategy => &ctx.basic_strategy,
        advanced_strategy => &ctx.advanced_strategy,
        data_docs => &ctx.data_docs,
    })
}

pub fn render_user_prompt(ctx: &UserContext) -> Result<String, minijinja::Error> {
    let mut env = Environment::new();
    env.add_template("user", USER_TEMPLATE)?;
    let tmpl = env.get_template("user")?;
    tmpl.render(context! {
        my_name => &ctx.my_name,
        players => &ctx.players,
        pub_state_yaml => &ctx.pub_state_yaml,
        player_state_yaml => &ctx.player_state_yaml,
        command_spec => &ctx.command_spec,
        recent_logs => &ctx.recent_logs,
        failed_commands => &ctx.failed_commands,
    })
}
```

- [ ] **Step 5: Add DB config types and provider routing to main.rs**

Add these structs and functions to `rust/bot/src/main.rs` (replace the existing `AppState` and config-reading logic):

```rust
#[derive(Debug, Clone)]
struct BotConfig {
    name: String,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
    temperature: f32,
}

#[derive(Debug, Clone)]
struct ProviderConfig {
    url: String,
    api_key: Option<String>,
    model: String,
    reasoning_effort: Option<String>,
    extra_body: Option<serde_json::Value>,
    priority: i32,
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    http: reqwest::Client,
    game_http: reqwest::Client,
    jetstream: async_nats::jetstream::Context,
    encryption_key: Option<[u8; 32]>,
}
```

Add a function to load bot config from DB with env-var fallback:

```rust
async fn load_bot_config(pool: &PgPool, bot_name: &str) -> Result<Option<BotConfig>> {
    let row = sqlx::query(
        "SELECT name, include_basic_strategy, include_advanced_strategy, temperature \
         FROM bots WHERE name = $1 AND enabled = true",
    )
    .bind(bot_name)
    .fetch_optional(pool)
    .await
    .context("Failed to query bots table")?;

    Ok(row.map(|r| BotConfig {
        name: r.try_get("name").unwrap_or_default(),
        include_basic_strategy: r.try_get("include_basic_strategy").unwrap_or(true),
        include_advanced_strategy: r.try_get("include_advanced_strategy").unwrap_or(false),
        temperature: r.try_get("temperature").unwrap_or(0.2),
    }))
}

async fn load_providers(
    pool: &PgPool,
    bot_name: &str,
    encryption_key: Option<&[u8; 32]>,
) -> Result<Vec<ProviderConfig>> {
    let rows = sqlx::query(
        r#"
        SELECT bp.model, bp.reasoning_effort, bp.extra_body, bp.priority,
               lp.url, lp.api_key_encrypted
        FROM bot_providers bp
        JOIN bots b ON b.id = bp.bot_id
        JOIN llm_providers lp ON lp.id = bp.provider_id
        WHERE b.name = $1 AND b.enabled = true AND bp.enabled = true AND lp.enabled = true
        ORDER BY bp.priority ASC
        "#,
    )
    .bind(bot_name)
    .fetch_all(pool)
    .await
    .context("Failed to query bot_providers")?;

    let mut providers = Vec::new();
    for r in rows {
        let api_key_encrypted: Option<Vec<u8>> = r.try_get("api_key_encrypted").unwrap_or(None);
        let api_key = match (api_key_encrypted, encryption_key) {
            (Some(encrypted), Some(key)) => {
                Some(crate::crypto::decrypt_api_key(&encrypted, key)?)
            }
            _ => None,
        };
        providers.push(ProviderConfig {
            url: r.try_get("url").unwrap_or_default(),
            api_key,
            model: r.try_get("model").unwrap_or_default(),
            reasoning_effort: r.try_get("reasoning_effort").unwrap_or(None),
            extra_body: r.try_get("extra_body").unwrap_or(None),
            priority: r.try_get("priority").unwrap_or(0),
        });
    }
    Ok(providers)
}

fn env_fallback_provider() -> Result<ProviderConfig> {
    let url = std::env::var("LLM_URL").context("LLM_URL must be set (no DB providers configured)")?;
    let model = std::env::var("BOT_MODEL").context("BOT_MODEL must be set (no DB providers configured)")?;
    let api_key = std::env::var("LLM_API_KEY").ok();
    let reasoning_effort = std::env::var("REASONING_EFFORT").ok();
    let extra_body = std::env::var("LLM_EXTRA_BODY")
        .ok()
        .map(|raw| serde_json::from_str(&raw))
        .transpose()
        .context("LLM_EXTRA_BODY must be valid JSON")?;
    Ok(ProviderConfig {
        url,
        api_key,
        model,
        reasoning_effort,
        extra_body,
        priority: 0,
    })
}
```

- [ ] **Step 6: Rewrite run_bot_turn to use GameData and provider routing**

Replace the existing `run_bot_turn` function. The new version:
1. Loads `BotConfig` from DB (three-layer gate: if bot not found or disabled, skip).
2. Loads providers (DB or env fallback).
3. Fetches `GameData` via `brdgme_game_client::game_data`.
4. Builds system prompt (static) once, user prompt per attempt.
5. Routes through providers with round-robin within priority + failover.

The core loop structure:

```rust
async fn run_bot_turn(state: &AppState, req: BotTurnEvent, trace_id: Uuid) -> Result<()> {
    // 1. Load bot config (three-layer gate).
    let bot_config = match load_bot_config(&state.pool, &req.bot_name).await? {
        Some(c) => c,
        None => {
            tracing::info!(trace_id = %trace_id, bot_name = %req.bot_name, "Bot disabled or not found, skipping");
            return Ok(());
        }
    };

    // 2. Load providers.
    let providers = load_providers(&state.pool, &req.bot_name, state.encryption_key.as_ref()).await?;
    let providers = if providers.is_empty() {
        vec![env_fallback_provider()?]
    } else {
        providers
    };

    // ... (fetch game data, build prompts, retry loop with provider routing)
}
```

The full implementation follows the existing retry-loop pattern (20 attempts, validate via Play, re-check DB state) but replaces:
- `load_bot_context` with `brdgme_game_client::game_data`
- `build_messages` with `render_system_prompt` + `render_user_prompt`
- Single `call_llm` with provider-routed `call_llm_routed`

Add a provider-routed LLM call:

```rust
async fn call_llm_routed(
    http: &reqwest::Client,
    providers: &[ProviderConfig],
    messages: &[ChatMessage],
    temperature: f32,
    start_index: usize,
) -> Result<(String, usize)> {
    let len = providers.len();
    for i in 0..len {
        let idx = (start_index + i) % len;
        let p = &providers[idx];
        match call_llm(http, &p.url, &p.model, messages, p.api_key.as_deref(), p.reasoning_effort.clone(), p.extra_body.as_ref(), temperature).await {
            Ok(response) => return Ok((response, idx)),
            Err(e) => {
                tracing::warn!(provider_idx = idx, error = %e, "Provider failed, trying next");
            }
        }
    }
    Err(anyhow!("all providers failed"))
}
```

Update `call_llm` to accept `temperature` as a parameter instead of hardcoding `0.2`.

- [ ] **Step 7: Update main() to use new AppState**

Replace the env-var reading in `main()` with:

```rust
    let encryption_key = match std::env::var("BOT_ENCRYPTION_KEY") {
        Ok(_) => Some(crate::crypto::load_key()?),
        Err(_) => {
            tracing::info!("BOT_ENCRYPTION_KEY not set, DB provider API keys will not be decrypted");
            None
        }
    };
```

Remove `llm_url`, `llm_api_key`, `bot_model`, `reasoning_effort`, `llm_extra_body` from `AppState` construction. The env-var fallback is now handled per-turn in `run_bot_turn`.

- [ ] **Step 8: Update the DB query in run_bot_turn**

The initial game-data query changes `gb.name as bot_name` (the bot's display name) - keep it. The `req.difficulty` references become `req.bot_name`. The `interface_version` is fetched from `game_versions`:

Add `gv.interface_version` to the initial SELECT query.

- [ ] **Step 9: Remove dead code**

Remove `markup_resolve_players` from prompt.rs (no longer used - logs are passed raw or resolved elsewhere). Remove the `brdgme_color` import and `LIGHT` usage from main.rs (colours no longer in prompt). Remove `BotContext` struct and `load_bot_context` function. Remove `build_prompt_context` and `build_messages`.

Keep `merge_json_patch` (still used by `call_llm` for `extra_body`).

- [ ] **Step 10: Verify**

Run: `cargo check -p bot`
Expected: compiles cleanly.

Run: `cargo clippy -p bot`
Expected: no warnings.

Run: `cargo test -p bot`
Expected: crypto tests + merge_json_patch tests pass. Prompt tests need updating to match new templates.

---

### Task 7: Monolith Updates

**Files:**
- Modify: `rust/web/src/nats.rs` (rename `difficulty` -> `bot_name` in `BotTurnEvent`)
- Modify: `rust/web/src/db.rs` (rename `difficulty` -> `bot_name` in queries and structs)
- Modify: `rust/web/src/models/game.rs` (rename `GameBot.difficulty` -> `bot_name`)
- Modify: `rust/web/src/game/mod.rs` (update `publish_bot_turns` to use `bot_name`)
- Modify: `rust/web/src/game/server_fns.rs` (rename `BotSlot.difficulty` -> `bot_name`, update `PlayerViewData.difficulty`)
- Modify: `rust/web/src/game/export.rs` (rename field)
- Modify: `rust/web/src/game/import.rs` (rename field)
- Modify: `rust/web/src/new_game.rs` (rename `OpponentSlot::Bot.difficulty` -> `bot_name`, update dropdown)
- Modify: `rust/web/src/components/game.rs` (update display)
- Modify: `rust/web/src/stats/queries.rs` (rename in INSERT)

**Interfaces:**
- Consumes: `game_bots.bot_name` column (from Task 1 migration)
- Produces: monolith publishes `bot.turn` with `bot_name` field; game creation form uses `bot_name`.

- [ ] **Step 1: Rename in nats.rs**

In `rust/web/src/nats.rs`, change `BotTurnEvent`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotTurnEvent {
    pub game_id: Uuid,
    pub player_position: i32,
    pub bot_name: String,
    pub attempt: i32,
}
```

- [ ] **Step 2: Rename in db.rs**

In `rust/web/src/db.rs`:

1. `BotTurn` struct: rename `difficulty` -> `bot_name`.
2. `find_bot_turns` query: `SELECT gp.position, gb.bot_name` (was `gb.difficulty`).
3. `build_game_bot_from_row`: rename parameter and field `difficulty` -> `bot_name`.
4. `PlayerSlotInternal::Bot { name, difficulty }` -> `PlayerSlotInternal::Bot { name, bot_name }`.
5. INSERT query: `INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3) RETURNING id`.
6. All JOINs selecting `gb.difficulty` -> `gb.bot_name`.
7. `CreateGameOpts.bot_slots` uses `BotSlot` which is updated in step 4.

- [ ] **Step 3: Rename in models/game.rs**

```rust
pub struct GameBot {
    pub id: Uuid,
    pub game_id: Uuid,
    pub name: String,
    pub bot_name: String,
}
```

- [ ] **Step 4: Rename in game/server_fns.rs**

```rust
pub struct BotSlot {
    pub name: String,
    pub bot_name: String,
}
```

Update `PlayerViewData.difficulty` -> `PlayerViewData.bot_name: Option<String>`.

Update all test usages: `difficulty: "easy".to_string()` -> `bot_name: "easy".to_string()`.

- [ ] **Step 5: Rename in game/mod.rs**

In `publish_bot_turns`:

```rust
        let event = crate::nats::BotTurnEvent {
            game_id,
            player_position: turn.position,
            bot_name: turn.bot_name.clone(),
            attempt,
        };
```

Update tracing: `difficulty = %turn.difficulty` -> `bot_name = %turn.bot_name`.

- [ ] **Step 6: Rename in game/export.rs and game/import.rs**

In `export.rs`: `pub difficulty: String` -> `pub bot_name: String`. Update the SELECT query: `SELECT name, bot_name, personality FROM game_bots ...`.

In `import.rs`: `INSERT INTO game_bots (game_id, name, bot_name, personality) ...` and `bot.difficulty` -> `bot.bot_name`.

- [ ] **Step 7: Rename in new_game.rs**

`OpponentSlot::Bot { name, difficulty }` -> `OpponentSlot::Bot { name, bot_name }`.

Update the difficulty `<select>` to use `bot_name`:
- `prop:value` reads `bot_name`
- `on:change` writes `bot_name`
- Default value stays `"medium"`
- The `<option>` values stay `"easy"`, `"medium"`, `"hard"` (these now reference `bots.name`)

Update the `BotSlot` construction: `BotSlot { name, bot_name }`.

- [ ] **Step 8: Rename in components/game.rs**

```rust
" (bot: " {player.bot_name.clone().unwrap_or_default()} ")"
```

- [ ] **Step 9: Rename in stats/queries.rs**

Update the INSERT: `INSERT INTO game_bots (id, game_id, name, bot_name) ...` and the corresponding bind.

- [ ] **Step 10: Verify**

Run: `SQLX_OFFLINE=true cargo check -p web --features ssr`
Expected: compiles cleanly (sqlx offline mode uses cached query metadata; the `.sqlx/` prepared queries may need regeneration with `cargo sqlx prepare` against a live DB, but the check should pass in offline mode if the existing `.sqlx/` cache is present).

Run: `SQLX_OFFLINE=true cargo clippy -p web --features ssr`
Expected: no warnings.

Note: If sqlx offline check fails due to stale `.sqlx/` cache, the queries need `cargo sqlx prepare --features ssr` against a live DB. This is a known limitation of agent runs without a database.

---

### Task 8: Per-Game V2 Upgrade (Template - lost-cities-1)

**Files:**
- Modify: `rust/game/lost-cities-1/src/lib.rs` (add doc comments to PubState/PlayerState, implement V2 trait methods)
- Create: `rust/game/lost-cities-1/BASIC_STRATEGY.md`
- Create: `rust/game/lost-cities-1/ADVANCED_STRATEGY.md`
- Modify: `k8s/base/games/lost-cities-1.yaml` (add `interfaceVersion: 2`)

**Interfaces:**
- Consumes: `Gamer::data_docs()`, `Gamer::basic_strategy()`, `Gamer::advanced_strategy()` (Task 4)
- Produces: lost-cities-1 serves V2 endpoints. Pattern for all other games (Task 9).

- [ ] **Step 1: Add doc comments to PubState fields**

In `rust/game/lost-cities-1/src/lib.rs`, add `///` doc comments to every field of `PubState`:

```rust
#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Current round number (1-3).
    pub round: usize,
    /// Whether the game has finished.
    pub is_finished: bool,
    /// Current phase of the turn: PlayOrDiscard or DrawOrTake.
    pub phase: Phase,
    /// Number of cards remaining in the draw pile.
    pub deck_remaining: usize,
    /// Top card value on each expedition's shared discard pile. Key is expedition colour, value is the card value (Investment or N(2-10)).
    pub discards: HashMap<Expedition, Value>,
    /// Cumulative scores per player per round. scores[player][round_index].
    pub scores: Vec<Vec<isize>>,
    /// Cards played to each player's expeditions. expeditions[player] is a list of cards in play order.
    pub expeditions: Vec<Vec<Card>>,
    /// Index of the player whose turn it is (0 or 1).
    pub current_player: usize,
}
```

- [ ] **Step 2: Add doc comments to PlayerState fields**

```rust
#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The public game state visible to all players.
    pub public: PubState,
    /// This player's index (0 or 1).
    pub player: usize,
    /// Cards in this player's hand, sorted by expedition then value.
    pub hand: Vec<Card>,
}
```

- [ ] **Step 3: Implement data_docs() using a build-time approach**

Since a derive macro is complex, use a manual `data_docs()` implementation that returns a static string matching the doc comments. This keeps the docs in the code (reviewable alongside the struct) without requiring a proc-macro crate:

```rust
    fn data_docs() -> String {
        r#"## PubState (public game state)

- round: Current round number (1-3).
- is_finished: Whether the game has finished.
- phase: Current phase of the turn - "PlayOrDiscard" or "DrawOrTake".
- deck_remaining: Number of cards remaining in the draw pile.
- discards: Top card value on each expedition's shared discard pile. Key is expedition colour (Red/Green/White/Blue/Yellow), value is "Investment" or a number 2-10.
- scores: Cumulative scores per player per round. scores[player_index][round_index].
- expeditions: Cards played to each player's expeditions. expeditions[player_index] is a list of cards in play order.
- current_player: Index of the player whose turn it is (0 or 1).

## PlayerState (your private state)

- public: The public game state (same as PubState above).
- player: Your player index (0 or 1).
- hand: Cards in your hand, sorted by expedition then value. Each card has an expedition (Red/Green/White/Blue/Yellow) and a value (Investment or 2-10).
"#.to_string()
    }
```

- [ ] **Step 4: Create BASIC_STRATEGY.md**

Create `rust/game/lost-cities-1/BASIC_STRATEGY.md`:

```markdown
# Basic Strategy

- Never start an expedition with fewer than 3-4 cards in that colour unless you have wager cards to multiply a small gain.
- Do not play a wager card (X) unless you have at least 4 numbered cards in that expedition in hand or already played.
- When discarding, prefer discarding cards from expeditions you have no intention of building.
- Do not take a discard card that is lower than the highest card you have already played in that expedition (you cannot play it).
- In the DrawOrTake phase, only take a discard if you can immediately use it (it is higher than your current highest in that expedition).
- Avoid starting more than 3 expeditions in a single round unless you have strong hands in all of them.
```

- [ ] **Step 5: Create ADVANCED_STRATEGY.md**

Create `rust/game/lost-cities-1/ADVANCED_STRATEGY.md`:

```markdown
# Advanced Strategy

- Track which cards have been played and discarded to calculate the probability of drawing needed cards.
- In later rounds, be more aggressive with expeditions since you have more information about remaining cards.
- Use wager cards strategically: 2-3 wagers on a strong expedition (6+ cards) can yield 60+ points.
- The 8-card bonus (+20) is worth pursuing if you already have 6+ cards in an expedition with wagers.
- Discard high cards in colours you are not building to deny your opponent (in 2-player, their discards are your draws).
- In the final round, prioritize finishing strong expeditions over starting new ones.
- Calculate the break-even point: an expedition needs sum > 20 to profit without wagers, sum > 10 with 1 wager, sum > 7 with 2 wagers.
- Watch your opponent's expeditions: if they are building a colour heavily, avoid discarding useful cards in that colour.
```

- [ ] **Step 6: Implement basic_strategy() and advanced_strategy()**

In the `impl Gamer for Game` block in `rust/game/lost-cities-1/src/lib.rs`:

```rust
    fn basic_strategy() -> String {
        include_str!("../BASIC_STRATEGY.md").to_string()
    }

    fn advanced_strategy() -> String {
        include_str!("../ADVANCED_STRATEGY.md").to_string()
    }
```

- [ ] **Step 7: Update the k8s GameVersion CR**

Find the lost-cities-1 GameVersion YAML (likely `k8s/base/games/lost-cities-1.yaml` or similar) and add:

```yaml
spec:
  interfaceVersion: 2
```

If the file does not exist yet, check `k8s/` for the game deployment pattern. The CRD instance needs `interfaceVersion: 2` in its spec.

- [ ] **Step 8: Verify**

Run: `cargo check -p lost-cities-1`
Expected: compiles cleanly.

Run: `cargo clippy -p lost-cities-1`
Expected: no warnings.

Run: `cargo test -p lost-cities-1`
Expected: all tests pass.

---

### Task 9: Remaining Games V2 Upgrade

**Files:**
- Modify: each game crate under `rust/game/` (all except `lost-cities-1` which is done in Task 8)
- Create: `BASIC_STRATEGY.md` and `ADVANCED_STRATEGY.md` in each game directory
- Modify: each game's k8s GameVersion CR (add `interfaceVersion: 2`)

**Interfaces:**
- Consumes: the pattern established in Task 8
- Produces: all Rust games serve V2 endpoints.

Games to upgrade (from workspace members):
- acquire-1
- alhambra-1
- age-of-war-2
- battleship-2
- cathedral-2
- category-5-2
- farkle-2
- for-sale-2
- greed-2
- jaipur-2
- liars-dice-2
- lords-of-vegas-1
- modern-art-2
- lost-cities-2
- love-letter-2
- no-thanks-2
- red7-1
- seven-wonders-1
- roll-through-the-ages-2
- splendor-2
- starship-catan-1
- sushi-go-2
- sushizock-2
- texas-holdem-2
- tic-tac-toe-2
- zombie-dice-2

**Note:** This task is large and can be parallelized across multiple workers. Each game follows the exact same pattern as Task 8. Split into groups of 4-6 games per worker.

- [ ] **Step 1: For each game, add doc comments to PubState and PlayerState**

Read the game's `src/lib.rs`, identify `PubState` and `PlayerState` structs, add `///` doc comments to every field explaining what it represents from the player's perspective.

- [ ] **Step 2: For each game, implement data_docs()**

Write a `fn data_docs() -> String` that returns a static documentation string matching the doc comments. Format:

```rust
    fn data_docs() -> String {
        r#"## PubState (public game state)

- field_name: description
...

## PlayerState (your private state)

- field_name: description
...
"#.to_string()
    }
```

- [ ] **Step 3: For each game, write BASIC_STRATEGY.md**

Write hard rules that prevent obviously terrible moves. Examples:
- "Don't sell a property for less than you paid" (Monopoly-like)
- "Always challenge a suspected liar in Liar's Dice when the bid is statistically impossible"
- "Never discard a card that completes an opponent's visible set"

Keep it to 5-10 bullet points of concrete, actionable rules.

- [ ] **Step 4: For each game, write ADVANCED_STRATEGY.md**

Write higher-level strategic advice for optimal play. Examples:
- Probability calculations
- Positional advantage
- Endgame timing
- Opponent modelling

Keep it to 5-10 bullet points.

- [ ] **Step 5: For each game, implement basic_strategy() and advanced_strategy()**

```rust
    fn basic_strategy() -> String {
        include_str!("../BASIC_STRATEGY.md").to_string()
    }

    fn advanced_strategy() -> String {
        include_str!("../ADVANCED_STRATEGY.md").to_string()
    }
```

- [ ] **Step 6: For each game, update the k8s GameVersion CR**

Add `interfaceVersion: 2` to the spec of each game's GameVersion custom resource.

- [ ] **Step 7: Verify each game**

For each game crate:

Run: `cargo check -p <game-name>`
Run: `cargo clippy -p <game-name>`
Run: `cargo test -p <game-name>`

Expected: all pass.

- [ ] **Step 8: Verify the full set compiles**

Run each game individually (never workspace-wide):

```bash
for game in acquire-1 alhambra-1 age-of-war-2 battleship-2 cathedral-2 category-5-2 farkle-2 for-sale-2 greed-2 jaipur-2 liars-dice-2 lords-of-vegas-1 modern-art-2 lost-cities-2 love-letter-2 no-thanks-2 red7-1 seven-wonders-1 roll-through-the-ages-2 splendor-2 starship-catan-1 sushi-go-2 sushizock-2 texas-holdem-2 tic-tac-toe-2 zombie-dice-2; do
    cargo check -p "$game" || echo "FAILED: $game"
done
```
