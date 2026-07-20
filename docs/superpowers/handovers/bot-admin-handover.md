# Bot Admin Interface - Context Handover

## File Paths

### Files to create
- `rust/web/src/admin.rs` (or `rust/web/src/admin/mod.rs`) - admin page + server fns
- Register module in `rust/web/src/lib.rs`

### Files to modify
- `rust/web/src/app.rs:193-205` - add Route for admin page
- `rust/web/src/lib.rs` - add `pub mod admin;`
- `rust/web/Cargo.toml` - add `aes-gcm`, `hex`, `getrandom` deps (ssr-gated)
- `rust/web/style/main.scss` - admin page styles (if needed beyond existing `.content-page`)

### Files to reference
- `rust/web/src/settings.rs` - page structure pattern
- `rust/web/src/auth/server.rs` - server fn patterns, auth guard
- `rust/web/src/game/server_fns.rs:822-874` - admin gate in server fns
- `rust/web/src/game/export.rs:174-222` - admin gate in axum handler
- `rust/web/src/db.rs:505-510` - `find_enabled_bots` (plain query pattern)
- `rust/web/src/db.rs:527-534` - `is_user_admin`
- `rust/web/src/components/form.rs:7-21` - FormField component
- `rust/web/src/error.rs:1-11` - `internal()` error helper
- `rust/web/src/router.rs:102-183` - route registration
- `rust/web/src/state.rs:7-16` - AppState (has `http_client: reqwest::Client`)
- `rust/bot/src/crypto.rs` - encrypt/decrypt to replicate or extract
- `rust/bot/src/config.rs` - DB query patterns for bots/providers tables
- `rust/bot/src/main.rs:584-634` - `call_llm` (test button reference)
- `rust/web/migrations/013_bot_efficacy.sql` - schema

---

## Patterns to Copy

### Admin Gate Pattern (server fn)

From `rust/web/src/game/server_fns.rs:822-846`:

```rust
#[server(BumpBotTurns, "/api")]
pub async fn bump_bot_turns(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("bump_bot_turns: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }
    // ... admin logic
}
```

### Server Fn Pattern (auth guard, error handling, return types)

From `rust/web/src/auth/server.rs:511-534`:

```rust
#[server(GetSettings, "/api")]
pub async fn get_settings() -> Result<SettingsData, ServerFnError> {
    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    // ... query, map_err(internal("context: step"))
}
```

Key conventions:
- `#[server(Name, "/api")]` attribute
- `expect_context::<PgPool>()` for DB
- `get_current_user().await?.ok_or_else(...)` for auth
- `.map_err(internal("fn_name: step"))` for infra errors (logs server-side, returns opaque "Internal server error")
- Return `Result<T, ServerFnError>`
- `use` statements inside fn body (ssr-gated imports)

### Leptos Page Pattern

From `rust/web/src/settings.rs:11-42`:

```rust
#[component]
pub fn SettingsPage() -> impl IntoView {
    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();

    let navigate = use_navigate();
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(None))) {
            navigate("/login", NavigateOptions::default());
        }
    });

    let settings: LocalResource<Result<SettingsData, ServerFnError>> =
        LocalResource::new(crate::auth::get_settings);

    view! {
        <MainLayout>
            <div class="settings content-page">
                <h1>"Settings"</h1>
                // sections...
            </div>
        </MainLayout>
    }
}
```

For admin: redirect non-admins (check via a server fn or the `viewer_is_admin` pattern). Use `LocalResource` for data that only fetches client-side post-hydration.

### FormField Usage

From `rust/web/src/settings.rs:80-103` and `rust/web/src/components/form.rs:7-21`:

```rust
<FormField
    label="Username"
    help="1-16 characters: letters, numbers, - and _. Must be unique."
    error=Signal::derive(move || error.get())
>
    <div class="form-actions">
        <input type="text" node_ref=name_input ... />
        <input type="submit" value="Save" ... />
    </div>
</FormField>
```

Props: `label: String`, `help: Option<String>`, `error: Signal<Option<String>>`, `children: Children`.
CSS classes: `.form-field`, `.form-label`, `.form-control`, `.form-help`, `.form-error`.

### DB Query Pattern

Two styles coexist:

1. **Macro queries** (`sqlx::query!`/`query_as!`/`query_scalar!`) - require `.sqlx` cache entries. Used in `auth/server.rs`, `db.rs` for most queries.

2. **Plain queries** (`sqlx::query`/`query_as`/`query_scalar`) - no cache needed. Used when the query is simple or the `.sqlx` cache is inconvenient. See `db.rs:505-510`:

```rust
pub async fn find_enabled_bots(pool: &PgPool) -> Result<Vec<String>> {
    sqlx::query_scalar("SELECT name FROM bots WHERE enabled = true ORDER BY display_order")
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("find_enabled_bots: {e}"))
}
```

And `db.rs:527-534`:

```rust
pub async fn is_user_admin(pool: &PgPool, user_id: Uuid) -> sqlx::Result<bool> {
    let row: Option<(bool,)> = sqlx::query_as("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(a,)| a).unwrap_or(false))
}
```

**Recommendation for admin CRUD**: use plain queries to avoid `.sqlx` cache churn (see Gotchas).

### Route Registration Pattern

From `rust/web/src/app.rs:193-205` (Leptos client routes):

```rust
<Routes fallback=|| "Page not found.".into_view()>
    <Route path=StaticSegment("") view=HomePage/>
    <Route path=StaticSegment("settings") view=crate::settings::SettingsPage/>
    // ...
</Routes>
```

For a non-Leptos axum route (like export): `rust/web/src/router.rs:130-133`:

```rust
.route(
    "/admin/games/{id}/export",
    axum::routing::get(crate::game::export::admin_export_game),
)
```

---

## Schema (Migration 013)

```sql
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

CREATE TABLE llm_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    api_key_encrypted BYTEA,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
```

Seed data: easy (order 0), medium (order 1), hard (order 2).

---

## Crypto Module

`rust/bot/src/crypto.rs` (128 lines, self-contained):

- **`encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError>`** - AES-256-GCM, random 12-byte nonce prepended to output. Output = `nonce (12 bytes) || ciphertext+tag`.
- **`decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, CryptoError>`** - splits first 12 bytes as nonce, decrypts remainder.
- **`load_key() -> Result<[u8; 32], CryptoError>`** - reads `DATABASE_ENCRYPTION_KEY` env var, hex-decodes to 32 bytes.
- **Nonce**: `getrandom::fill` into `[u8; 12]`.
- **Deps**: `aes-gcm 0.10`, `hex 0.4`, `getrandom`, `thiserror`.

The web crate needs `encrypt` (to write API keys) and `decrypt` (to test providers). `load_key` reads the same `DATABASE_ENCRYPTION_KEY` env var.

---

## Existing Admin Patterns

### bump-bot-to-play (server fn gate)

`rust/web/src/game/server_fns.rs:822-850` - `BumpBotTurns` server fn:
1. Auth guard (`get_current_user`)
2. Player-in-game check
3. `is_user_admin` check -> `Err(ServerFnError::new("Admin access required"))`
4. Action

UI gate: `rust/web/src/components/game.rs:35,140-148` - `viewer_is_admin` bool from `GameViewData`, `<Show when=move || viewer_is_admin>`.

### Game export (axum route gate)

`rust/web/src/game/export.rs:177-202` - `admin_export_game` axum handler:
1. `get_user_from_session(&session)` -> 401
2. `validate_session_token` -> 401
3. `is_user_admin` -> 403
4. Build response

### force_delete_game (server fn gate)

`rust/web/src/game/server_fns.rs:854-874` - `force_delete_game_impl`:
1. `is_user_admin` -> `Err(ServerFnError::new("Admin access required"))`
2. Delete

---

## Gotchas

### Hydration
- `LocalResource` fetches only on client (post-hydration). Use for admin data to avoid SSR leaking admin state into HTML.
- `Effect` is inert during SSR. Client-only redirects (non-admin bounce) must use Effect.
- SSR and hydration output must be structurally identical. Conditional admin UI should use `<Show>` with a signal that's `false` during SSR.

### sqlx Offline Mode
- Macro queries (`query!`, `query_as!`, `query_scalar!`) require `.sqlx/` cache entries. The cache for migration 013 was hand-written (see `docs/decisions/BOT_EFFICACY.md` tech debt). New macro queries need `cargo sqlx prepare` against a live DB.
- **Plain queries** (`sqlx::query`, `query_as`, `query_scalar`) bypass the cache entirely. Prefer these for admin CRUD to avoid cache churn.

### Feature Gates
- All server-only code needs `#[cfg(feature = "ssr")]` or must live inside `#[server]` fns.
- The `hydrate` feature builds the WASM binary. Any server-only dep (aes-gcm, sqlx) must be optional and gated under `ssr`.
- `web/Cargo.toml` already has `reqwest` (ssr-gated) and `serde_json` (unconditional).

### Web Crate Deps (already present)
- `reqwest` (ssr) - for test button HTTP calls
- `serde_json` - for extra_body handling
- `uuid` - everywhere
- `sqlx` (ssr) - DB access
- `thiserror` (ssr) - for CryptoError if duplicated

### Web Crate Deps (must add, ssr-gated)
- `aes-gcm = "0.10"` - encryption
- `hex = "0.4"` - key decoding
- `getrandom` - already present (unconditional, for wasm); the ssr path uses it for nonces

### DATABASE_ENCRYPTION_KEY
- Must be set in the web pod's env (same sealed-secret as the bot pod). Currently only the bot reads it. K8s manifests need updating.

---

## Decisions

### Crypto: duplicate into web (not shared lib)

**No shared lib exists for this.** The workspace has `rust/lib/` crates (`cmd`, `color`, `cost`, `game`, `game_client`, `markup`, `rand_bot`) but none is a general "shared utils" crate. The bot's `crypto.rs` is 57 lines of logic (excluding tests). Duplicating it into `rust/web/src/admin.rs` (or a `rust/web/src/crypto.rs` module) is the pragmatic choice:
- Avoids creating a new workspace crate for 57 lines
- The bot crate uses `getrandom 0.3`, web uses `getrandom 0.4` - version mismatch in a shared crate would be annoying
- The code is stable (AES-256-GCM with prepended nonce) and unlikely to diverge

Copy `encrypt`, `decrypt`, `load_key`, `rand_nonce`, and `CryptoError` into the web crate. Gate under `#[cfg(feature = "ssr")]`.

### Route path: `/admin/bots`

Existing admin route: `/admin/games/{id}/export` (axum handler, `router.rs:131`).
Leptos pages use flat segments: `/settings`, `/games`, `/dashboard`, `/friends`.

Use `/admin` as the Leptos route path (StaticSegment). This keeps it under the `/admin` namespace already established. The page component handles all bot/provider CRUD. If more admin pages appear later, nest under `/admin/bots`, `/admin/providers`, etc.

**Recommended**: `<Route path=StaticSegment("admin") view=crate::admin::AdminPage/>` in `app.rs`.

### Test button: server fn calling LLM directly

The bot crate's `call_llm` (`rust/bot/src/main.rs:584-634`) shows the pattern:
- POST to `{provider_url}/v1/chat/completions`
- Body: `{ model, messages: [{role: "user", content: "Say hello"}], stream: false, temperature: 0.1 }`
- Auth: `Authorization: Bearer {api_key}` header
- Parse `choices[0].message.content` from response

The web crate already has `reqwest::Client` in context (`AppState.http_client`, provided via `provide_context` in `router.rs:113`). The test button server fn should:
1. Admin gate
2. Load provider row, decrypt API key
3. POST a trivial completion (`"Say hello in one word"`)
4. Return success/failure + response snippet

No need to replicate the full `ChatRequest` struct - a minimal `serde_json::json!({...})` body suffices.

### CSS class pattern

Existing pages use: `<div class="settings content-page">` (settings.rs:33), `<div class="content-page">` (app.rs:257).

Use: `<div class="admin content-page">` as the page wrapper. Sections use `<h2>` headings. Forms use `FormField` component with `.form-actions` for button rows. No new layout primitives needed.
