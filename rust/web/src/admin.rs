#[cfg(feature = "ssr")]
use crate::error::internal;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type BotUpdateAction =
    Action<(Uuid, String, f32, bool, bool, bool, bool), Result<(), ServerFnError>>;
type BotCreateAction = Action<(String, f32, bool, bool, bool), Result<BotRow, ServerFnError>>;
type ProviderUpdateAction =
    Action<(Uuid, String, String, ApiKeyUpdate, bool), Result<(), ServerFnError>>;
type BotProviderCreateAction = Action<
    (
        Uuid,
        Uuid,
        String,
        Option<String>,
        Option<serde_json::Value>,
        i32,
    ),
    Result<BotProviderRow, ServerFnError>,
>;
type BotProviderUpdateAction = Action<
    (
        Uuid,
        String,
        Option<String>,
        Option<serde_json::Value>,
        i32,
        bool,
    ),
    Result<(), ServerFnError>,
>;

/// The exact message a non-admin caller gets from every admin server fn.
/// Shared so the client-side redirect in `AdminPage` cannot drift from it
/// (ws F31); see the `ServerFnError::ServerError` match there.
pub const ADMIN_REQUIRED: &str = "Admin access required";

/// Authenticate, then require `users.is_admin`. Fail-closed.
///
/// `context` is threaded through to `internal` so each call site keeps its
/// own server-side log breadcrumb; the client-visible error is identical at
/// every site. Mirrors `friends::require_user` (ws F28).
#[cfg(feature = "ssr")]
async fn require_admin(pool: &sqlx::PgPool, context: &'static str) -> Result<(), ServerFnError> {
    let user = crate::auth::server::get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    if !crate::db::is_user_admin(pool, user.id)
        .await
        .map_err(internal(context))?
    {
        return Err(ServerFnError::new(ADMIN_REQUIRED));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRow {
    pub id: Uuid,
    pub name: String,
    pub display_order: i32,
    pub enabled: bool,
    pub include_basic_strategy: bool,
    pub include_advanced_strategy: bool,
    pub temperature: f32,
    pub can_replace_humans: bool,
}

/// A bot type referenced by one or more unfinished games that no longer
/// resolves to an enabled `bots` row (renamed away, deleted, or disabled).
/// Dangling references are a supported state (D-05/D-08): the bot service
/// falls back to a synthetic config, so the admin page only warns. An empty
/// `bots` table is deliberately never reported (the same fallback applies).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DanglingBotName {
    pub bot_name: String,
    pub game_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRow {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub api_key_masked: Option<String>,
    pub enabled: bool,
}

/// What an update should do to a provider's stored API key. `Option<String>`
/// could only express two of these three intentions, so "revoke this key"
/// was unrepresentable on the public API surface (ws F21).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyUpdate {
    /// Leave `api_key_encrypted` exactly as it is.
    Keep,
    /// Encrypt and store this new key.
    Set(String),
    /// Set `api_key_encrypted` to NULL.
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotProviderRow {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub provider_id: Uuid,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub extra_body: Option<serde_json::Value>,
    pub priority: i32,
    pub enabled: bool,
    pub bot_name: String,
    pub provider_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestBotProviderResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub elapsed_ms: u64,
}

#[cfg(feature = "ssr")]
type BotDbRow = (Uuid, String, i32, bool, bool, bool, f32, bool);
#[cfg(feature = "ssr")]
type ProviderDbRow = (Uuid, String, String, Option<Vec<u8>>, bool);
#[cfg(feature = "ssr")]
type BotProviderDbRow = (
    Uuid,
    Uuid,
    Uuid,
    String,
    Option<String>,
    Option<serde_json::Value>,
    i32,
    bool,
    String,
    String,
);

#[cfg(feature = "ssr")]
pub async fn list_bots(pool: &sqlx::PgPool) -> Result<Vec<BotRow>, ServerFnError> {
    let rows: Vec<BotDbRow> = sqlx::query_as(
        "SELECT id, name, display_order, enabled, include_basic_strategy, include_advanced_strategy, temperature, can_replace_humans FROM bots ORDER BY display_order",
    )
    .fetch_all(pool)
    .await
    .map_err(internal("admin_list_bots: query"))?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                name,
                display_order,
                enabled,
                include_basic_strategy,
                include_advanced_strategy,
                temperature,
                can_replace_humans,
            )| {
                BotRow {
                    id,
                    name,
                    display_order,
                    enabled,
                    include_basic_strategy,
                    include_advanced_strategy,
                    temperature,
                    can_replace_humans,
                }
            },
        )
        .collect())
}

/// Distinct `game_bots.bot_name` values used by an unfinished game that have
/// no enabled `bots` row, each with the count of affected unfinished games.
///
/// A name is dangling when no `bots` row matches `name = gb.bot_name AND
/// enabled = true`. The `EXISTS (SELECT 1 FROM bots)` guard makes an EMPTY
/// `bots` table yield zero rows: with no bots configured the bot service falls
/// back to a synthetic config, so nothing is dangling and no warning is wanted
/// (D-05/D-08). `NOT EXISTS` alone would flag every referenced name in that
/// case.
#[cfg(feature = "ssr")]
pub async fn list_dangling_bot_names(
    pool: &sqlx::PgPool,
) -> Result<Vec<DanglingBotName>, ServerFnError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT gb.bot_name, COUNT(DISTINCT g.id) \
         FROM game_bots gb \
         JOIN games g ON g.id = gb.game_id \
         WHERE g.is_finished = false \
         AND EXISTS (SELECT 1 FROM bots) \
         AND NOT EXISTS (SELECT 1 FROM bots b WHERE b.name = gb.bot_name AND b.enabled = true) \
         GROUP BY gb.bot_name \
         ORDER BY gb.bot_name",
    )
    .fetch_all(pool)
    .await
    .map_err(internal("admin_list_dangling_bot_names: query"))?;

    Ok(rows
        .into_iter()
        .map(|(bot_name, game_count)| DanglingBotName {
            bot_name,
            game_count,
        })
        .collect())
}

/// Advisory-lock key serializing every writer of `bots.display_order`.
/// `create_bot` reads `MAX(display_order)+1` and `reorder_bots` renumbers the
/// whole list; without this they can produce duplicate orders, and there is no
/// unique constraint on the column (migration 013). Transaction-scoped, so it
/// is released on commit or rollback (ws F18, ws F19).
#[cfg(feature = "ssr")]
const BOT_DISPLAY_ORDER_LOCK: i64 = 130_100_113;

/// ws F25: cheap server-side validation. Every constraint below is duplicated
/// in the HTML forms; these exist because the server fns are a public surface
/// and a crafted call otherwise stores NaN temperatures, empty bot names or
/// non-HTTP provider URLs.
#[cfg(feature = "ssr")]
fn require_text(value: &str, field: &'static str, max: usize) -> Result<String, ServerFnError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ServerFnError::new(format!("{field} is required")));
    }
    if trimmed.chars().count() > max {
        return Err(ServerFnError::new(format!(
            "{field} must be at most {max} characters"
        )));
    }
    Ok(trimmed.to_string())
}

#[cfg(feature = "ssr")]
fn validate_temperature(temperature: f32) -> Result<(), ServerFnError> {
    if !temperature.is_finite() || !(0.0..=2.0).contains(&temperature) {
        return Err(ServerFnError::new(
            "Temperature must be a number between 0.0 and 2.0",
        ));
    }
    Ok(())
}

#[cfg(feature = "ssr")]
fn validate_provider_url(url: &str) -> Result<String, ServerFnError> {
    let url = require_text(url, "URL", 512)?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ServerFnError::new(
            "URL must start with http:// or https://",
        ));
    }
    Ok(url)
}

#[cfg(feature = "ssr")]
fn validate_extra_body(
    extra_body: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, ServerFnError> {
    let Some(value) = extra_body else {
        return Ok(None);
    };
    if !value.is_object() {
        return Err(ServerFnError::new("Extra body must be a JSON object"));
    }
    if value.to_string().len() > 8192 {
        return Err(ServerFnError::new(
            "Extra body must be at most 8192 bytes of JSON",
        ));
    }
    Ok(Some(value))
}

#[cfg(feature = "ssr")]
fn validate_reasoning_effort(
    reasoning_effort: Option<String>,
) -> Result<Option<String>, ServerFnError> {
    match reasoning_effort {
        // Free text on purpose: providers disagree on the vocabulary.
        Some(v) => Ok(Some(require_text(&v, "Reasoning effort", 32)?)),
        None => Ok(None),
    }
}

#[cfg(feature = "ssr")]
pub async fn create_bot(
    pool: &sqlx::PgPool,
    name: String,
    temperature: f32,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
    can_replace_humans: bool,
) -> Result<BotRow, ServerFnError> {
    let name = require_text(&name, "Bot name", 64)?;
    validate_temperature(temperature)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("admin_create_bot: begin"))?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(BOT_DISPLAY_ORDER_LOCK)
        .execute(&mut *tx)
        .await
        .map_err(internal("admin_create_bot: lock"))?;

    let row: BotDbRow = sqlx::query_as(
        "INSERT INTO bots (name, display_order, temperature, include_basic_strategy, include_advanced_strategy, can_replace_humans) \
         VALUES ($1, COALESCE((SELECT MAX(display_order) + 1 FROM bots), 0), $2, $3, $4, $5) \
         RETURNING id, name, display_order, enabled, include_basic_strategy, include_advanced_strategy, temperature, can_replace_humans",
    )
    .bind(&name)
    .bind(temperature)
    .bind(include_basic_strategy)
    .bind(include_advanced_strategy)
    .bind(can_replace_humans)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal("admin_create_bot: insert"))?;

    tx.commit()
        .await
        .map_err(internal("admin_create_bot: commit"))?;

    Ok(BotRow {
        id: row.0,
        name: row.1,
        display_order: row.2,
        enabled: row.3,
        include_basic_strategy: row.4,
        include_advanced_strategy: row.5,
        temperature: row.6,
        can_replace_humans: row.7,
    })
}

#[cfg(feature = "ssr")]
#[allow(clippy::too_many_arguments)]
pub async fn update_bot(
    pool: &sqlx::PgPool,
    id: Uuid,
    name: String,
    temperature: f32,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
    enabled: bool,
    can_replace_humans: bool,
) -> Result<(), ServerFnError> {
    let name = require_text(&name, "Bot name", 64)?;
    validate_temperature(temperature)?;

    let result = sqlx::query(
        "UPDATE bots SET name = $2, temperature = $3, include_basic_strategy = $4, include_advanced_strategy = $5, enabled = $6, can_replace_humans = $7, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(&name)
    .bind(temperature)
    .bind(include_basic_strategy)
    .bind(include_advanced_strategy)
    .bind(enabled)
    .bind(can_replace_humans)
    .execute(pool)
    .await
    .map_err(internal("admin_update_bot: update"))?;
    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(
            "Bot not found - it may have been deleted; reload and try again",
        ));
    }
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn reorder_bots(
    pool: &sqlx::PgPool,
    ordered_ids: Vec<Uuid>,
) -> Result<(), ServerFnError> {
    // A duplicated id would match one `bots` row from two ordinals; Postgres
    // applies exactly one of them and does not say which, so the resulting
    // order is nondeterministic. Reject before doing any work. (The old loop
    // was deterministic here only by accident - last write won.)
    let distinct: std::collections::HashSet<&Uuid> = ordered_ids.iter().collect();
    if distinct.len() != ordered_ids.len() {
        return Err(ServerFnError::new(
            "Bot list contains a duplicate entry, please reload and try again",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("admin_reorder_bots: begin"))?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(BOT_DISPLAY_ORDER_LOCK)
        .execute(&mut *tx)
        .await
        .map_err(internal("admin_reorder_bots: lock"))?;

    // ws F18: one statement, so a partial renumber is impossible. WITH
    // ORDINALITY is 1-based; the stored order stays 0-based.
    let result = sqlx::query(
        "UPDATE bots SET display_order = o.ord - 1, updated_at = now() \
         FROM unnest($1::uuid[]) WITH ORDINALITY AS o(id, ord) \
         WHERE bots.id = o.id",
    )
    .bind(&ordered_ids)
    .execute(&mut *tx)
    .await
    .map_err(internal("admin_reorder_bots: update"))?;

    // ws F29: an id that no longer exists means the admin is acting on a
    // stale list; reject instead of reporting success for a partial reorder.
    // `distinct.len() == ordered_ids.len()` was proven above, so comparing
    // against either is equivalent - use `distinct` to keep the intent local.
    if result.rows_affected() as usize != distinct.len() {
        tx.rollback()
            .await
            .map_err(internal("admin_reorder_bots: rollback"))?;
        return Err(ServerFnError::new(
            "Bot list has changed, please reload and try again",
        ));
    }

    tx.commit()
        .await
        .map_err(internal("admin_reorder_bots: commit"))?;
    Ok(())
}

// Deletes stay idempotent: a row that is already gone satisfies the request
// (ws F29 - only the updates report "not found").
#[cfg(feature = "ssr")]
pub async fn delete_bot(pool: &sqlx::PgPool, id: Uuid) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM bots WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(internal("admin_delete_bot: delete"))?;
    Ok(())
}

/// Minimum key length before any characters are revealed. At 8, the mask
/// shows at most half the key; below it, nothing is shown at all (ws F20:
/// the old `sk-...{last4}` mask round-tripped keys of <= 4 chars in full).
#[cfg(feature = "ssr")]
const API_KEY_MASK_MIN_LEN: usize = 8;

/// Render a stored API key for display. Never fabricates a vendor prefix
/// (the old mask hardcoded `sk-`, which is wrong for every non-OpenAI
/// provider) and never reveals anything for a key short enough that the
/// "last 4" would be most of it (ws F20).
#[cfg(feature = "ssr")]
fn mask_api_key(plaintext: &str) -> String {
    let len = plaintext.chars().count();
    if len < API_KEY_MASK_MIN_LEN {
        return "(set)".to_string();
    }
    let last4: String = plaintext.chars().skip(len - 4).collect();
    format!("...{last4}")
}

#[cfg(feature = "ssr")]
pub async fn list_providers(pool: &sqlx::PgPool) -> Result<Vec<ProviderRow>, ServerFnError> {
    let rows: Vec<ProviderDbRow> = sqlx::query_as(
        "SELECT id, name, url, api_key_encrypted, enabled FROM llm_providers ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(internal("admin_list_providers: query"))?;

    let key = crate::crypto::load_key().map_err(internal("admin_list_providers: load key"))?;

    let mut providers = Vec::with_capacity(rows.len());
    for (id, name, url, api_key_encrypted, enabled) in rows {
        // ws F26: a single unreadable row must not take down the whole admin
        // page. Degrade that provider's key column and log the id so the row
        // can be re-keyed through this same UI. `load_key` failure above stays
        // fatal - that is a deployment problem, not row corruption.
        let api_key_masked = api_key_encrypted.map(|encrypted| {
            match crate::crypto::decrypt(&key, &encrypted)
                .map_err(|e| e.to_string())
                .and_then(|d| String::from_utf8(d).map_err(|e| e.to_string()))
            {
                Ok(plaintext) => mask_api_key(&plaintext),
                Err(e) => {
                    tracing::error!(
                        "admin_list_providers: provider {id} api_key_encrypted is unreadable: {e}"
                    );
                    "(undecryptable)".to_string()
                }
            }
        });
        providers.push(ProviderRow {
            id,
            name,
            url,
            api_key_masked,
            enabled,
        });
    }
    Ok(providers)
}

#[cfg(feature = "ssr")]
pub async fn create_provider(
    pool: &sqlx::PgPool,
    name: String,
    url: String,
    api_key: Option<String>,
) -> Result<ProviderRow, ServerFnError> {
    let name = require_text(&name, "Provider name", 64)?;
    let url = validate_provider_url(&url)?;

    let api_key_encrypted: Option<Vec<u8>> = match &api_key {
        Some(key_str) => {
            let enc_key =
                crate::crypto::load_key().map_err(internal("admin_create_provider: load key"))?;
            let encrypted = crate::crypto::encrypt(&enc_key, key_str.as_bytes())
                .map_err(internal("admin_create_provider: encrypt"))?;
            Some(encrypted)
        }
        None => None,
    };

    let row: (Uuid, String, String, bool) = sqlx::query_as(
        "INSERT INTO llm_providers (name, url, api_key_encrypted) VALUES ($1, $2, $3) RETURNING id, name, url, enabled",
    )
    .bind(&name)
    .bind(&url)
    .bind(&api_key_encrypted)
    .fetch_one(pool)
    .await
    .map_err(internal("admin_create_provider: insert"))?;

    let api_key_masked = api_key.as_deref().map(mask_api_key);

    Ok(ProviderRow {
        id: row.0,
        name: row.1,
        url: row.2,
        api_key_masked,
        enabled: row.3,
    })
}

#[cfg(feature = "ssr")]
pub async fn update_provider(
    pool: &sqlx::PgPool,
    id: Uuid,
    name: String,
    url: String,
    api_key: ApiKeyUpdate,
    enabled: bool,
) -> Result<(), ServerFnError> {
    let name = require_text(&name, "Provider name", 64)?;
    let url = validate_provider_url(&url)?;

    let rows = match api_key {
        ApiKeyUpdate::Set(key_str) => {
            let enc_key =
                crate::crypto::load_key().map_err(internal("admin_update_provider: load key"))?;
            let encrypted = crate::crypto::encrypt(&enc_key, key_str.as_bytes())
                .map_err(internal("admin_update_provider: encrypt"))?;
            sqlx::query(
                "UPDATE llm_providers SET name = $2, url = $3, api_key_encrypted = $4, enabled = $5, updated_at = now() WHERE id = $1",
            )
            .bind(id)
            .bind(&name)
            .bind(&url)
            .bind(&encrypted)
            .bind(enabled)
            .execute(pool)
            .await
            .map_err(internal("admin_update_provider: update"))?
            .rows_affected()
        }
        ApiKeyUpdate::Clear => sqlx::query(
            "UPDATE llm_providers SET name = $2, url = $3, api_key_encrypted = NULL, enabled = $4, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(&name)
        .bind(&url)
        .bind(enabled)
        .execute(pool)
        .await
        .map_err(internal("admin_update_provider: clear key"))?
        .rows_affected(),
        ApiKeyUpdate::Keep => sqlx::query(
            "UPDATE llm_providers SET name = $2, url = $3, enabled = $4, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(&name)
        .bind(&url)
        .bind(enabled)
        .execute(pool)
        .await
        .map_err(internal("admin_update_provider: update"))?
        .rows_affected(),
    };
    if rows == 0 {
        return Err(ServerFnError::new(
            "Provider not found - it may have been deleted; reload and try again",
        ));
    }
    Ok(())
}

// Deletes stay idempotent: a row that is already gone satisfies the request
// (ws F29 - only the updates report "not found").
#[cfg(feature = "ssr")]
pub async fn delete_provider(pool: &sqlx::PgPool, id: Uuid) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM llm_providers WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(internal("admin_delete_provider: delete"))?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn list_bot_providers(pool: &sqlx::PgPool) -> Result<Vec<BotProviderRow>, ServerFnError> {
    let rows: Vec<BotProviderDbRow> = sqlx::query_as(
        "SELECT bp.id, bp.bot_id, bp.provider_id, bp.model, bp.reasoning_effort, bp.extra_body, bp.priority, bp.enabled, b.name, p.name \
         FROM bot_providers bp JOIN bots b ON bp.bot_id = b.id JOIN llm_providers p ON bp.provider_id = p.id \
         ORDER BY b.display_order, bp.priority",
    )
    .fetch_all(pool)
    .await
    .map_err(internal("admin_list_bot_providers: query"))?;

    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                bot_id,
                provider_id,
                model,
                reasoning_effort,
                extra_body,
                priority,
                enabled,
                bot_name,
                provider_name,
            )| {
                BotProviderRow {
                    id,
                    bot_id,
                    provider_id,
                    model,
                    reasoning_effort,
                    extra_body,
                    priority,
                    enabled,
                    bot_name,
                    provider_name,
                }
            },
        )
        .collect())
}

#[cfg(feature = "ssr")]
pub async fn create_bot_provider(
    pool: &sqlx::PgPool,
    bot_id: Uuid,
    provider_id: Uuid,
    model: String,
    reasoning_effort: Option<String>,
    extra_body: Option<serde_json::Value>,
    priority: i32,
) -> Result<BotProviderRow, ServerFnError> {
    let model = require_text(&model, "Model", 128)?;
    let reasoning_effort = validate_reasoning_effort(reasoning_effort)?;
    let extra_body = validate_extra_body(extra_body)?;

    let row: BotProviderDbRow = sqlx::query_as(
        "INSERT INTO bot_providers (bot_id, provider_id, model, reasoning_effort, extra_body, priority) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, bot_id, provider_id, model, reasoning_effort, extra_body, priority, enabled, \
         (SELECT name FROM bots WHERE id = $1), (SELECT name FROM llm_providers WHERE id = $2)",
    )
    .bind(bot_id)
    .bind(provider_id)
    .bind(&model)
    .bind(&reasoning_effort)
    .bind(&extra_body)
    .bind(priority)
    .fetch_one(pool)
    .await
    .map_err(internal("admin_create_bot_provider: insert"))?;

    Ok(BotProviderRow {
        id: row.0,
        bot_id: row.1,
        provider_id: row.2,
        model: row.3,
        reasoning_effort: row.4,
        extra_body: row.5,
        priority: row.6,
        enabled: row.7,
        bot_name: row.8,
        provider_name: row.9,
    })
}

#[cfg(feature = "ssr")]
pub async fn update_bot_provider(
    pool: &sqlx::PgPool,
    id: Uuid,
    model: String,
    reasoning_effort: Option<String>,
    extra_body: Option<serde_json::Value>,
    priority: i32,
    enabled: bool,
) -> Result<(), ServerFnError> {
    let model = require_text(&model, "Model", 128)?;
    let reasoning_effort = validate_reasoning_effort(reasoning_effort)?;
    let extra_body = validate_extra_body(extra_body)?;

    let result = sqlx::query(
        "UPDATE bot_providers SET model = $2, reasoning_effort = $3, extra_body = $4, priority = $5, enabled = $6 WHERE id = $1",
    )
    .bind(id)
    .bind(&model)
    .bind(&reasoning_effort)
    .bind(&extra_body)
    .bind(priority)
    .bind(enabled)
    .execute(pool)
    .await
    .map_err(internal("admin_update_bot_provider: update"))?;
    if result.rows_affected() == 0 {
        return Err(ServerFnError::new(
            "Bot-provider link not found - it may have been deleted; reload and try again",
        ));
    }
    Ok(())
}

// Deletes stay idempotent: a row that is already gone satisfies the request
// (ws F29 - only the updates report "not found").
#[cfg(feature = "ssr")]
pub async fn delete_bot_provider(pool: &sqlx::PgPool, id: Uuid) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM bot_providers WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(internal("admin_delete_bot_provider: delete"))?;
    Ok(())
}

/// Cap on bytes read from an admin-configured upstream during a test call.
/// The 10s reqwest timeout bounds how long a hostile endpoint can stream, not
/// how much (ws F23). Comfortably above any real completion or error envelope.
#[cfg(feature = "ssr")]
const MAX_TEST_BODY_BYTES: usize = 8 * 1024;

/// Response headers worth showing an admin. Everything else is dropped rather
/// than round-tripped: an upstream can set arbitrary headers, including
/// cookies and echoed credentials (ws F23).
#[cfg(feature = "ssr")]
const TEST_HEADER_ALLOWLIST: &[&str] = &[
    "content-type",
    "content-length",
    "date",
    "retry-after",
    "x-request-id",
    "x-ratelimit-limit",
    "x-ratelimit-remaining",
    "x-ratelimit-reset",
];

/// Read at most `MAX_TEST_BODY_BYTES` of a response body, then stop. Dropping
/// the response cancels the remainder of the transfer.
#[cfg(feature = "ssr")]
async fn read_capped_body(mut resp: reqwest::Response) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut truncated = false;
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let remaining = MAX_TEST_BODY_BYTES.saturating_sub(buf.len());
                if chunk.len() > remaining {
                    buf.extend_from_slice(&chunk[..remaining]);
                    truncated = true;
                    break;
                }
                buf.extend_from_slice(&chunk);
                if buf.len() >= MAX_TEST_BODY_BYTES {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => return String::from_utf8_lossy(&buf).into_owned() + "\n<error reading body>",
        }
    }
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if truncated {
        text.push_str(&format!("\n<truncated at {MAX_TEST_BODY_BYTES} bytes>"));
    }
    text
}

/// Filter response headers down to `TEST_HEADER_ALLOWLIST`.
#[cfg(feature = "ssr")]
fn allowlisted_headers(headers: &reqwest::header::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|(k, _)| TEST_HEADER_ALLOWLIST.contains(&k.as_str()))
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
        .collect()
}

#[cfg(feature = "ssr")]
pub async fn test_provider(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    provider_id: Uuid,
    model: Option<String>,
) -> Result<String, ServerFnError> {
    let row: Option<(String, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT url, api_key_encrypted FROM llm_providers WHERE id = $1")
            .bind(provider_id)
            .fetch_optional(pool)
            .await
            .map_err(internal("admin_test_provider: query"))?;

    let (url, api_key_encrypted) = row.ok_or_else(|| ServerFnError::new("Provider not found"))?;

    // ws F22: never fabricate a model id. An explicit model wins; otherwise
    // use the provider's highest-priority enabled link model. `bot_providers`
    // is the only place a model is configured (migration 013) - neither
    // `llm_providers` nor `bots` has a model column.
    let model = match model.map(|m| require_text(&m, "Model", 128)).transpose()? {
        Some(m) => m,
        None => sqlx::query_scalar::<_, String>(
            "SELECT model FROM bot_providers \
             WHERE provider_id = $1 AND enabled \
             ORDER BY priority, model LIMIT 1",
        )
        .bind(provider_id)
        .fetch_optional(pool)
        .await
        .map_err(internal("admin_test_provider: resolve model"))?
        .ok_or_else(|| {
            ServerFnError::new(
                "No enabled bot-provider link for this provider, so there is no configured \
                 model to test with. Enter a model above, or add a link first.",
            )
        })?,
    };

    let key = crate::crypto::load_key().map_err(internal("admin_test_provider: load key"))?;
    let api_key = match api_key_encrypted {
        Some(encrypted) => {
            let decrypted = crate::crypto::decrypt(&key, &encrypted)
                .map_err(internal("admin_test_provider: decrypt"))?;
            String::from_utf8(decrypted).map_err(internal("admin_test_provider: utf8"))?
        }
        None => return Err(ServerFnError::new("Provider has no API key configured")),
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Say hello"}],
        "stream": false,
        "max_tokens": 5
    });

    let resp = http_client
        .post(format!("{url}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&body)
        .send()
        .await
        .map_err(internal("admin_test_provider: request"))?;

    let status = resp.status();
    let text = read_capped_body(resp).await;

    if !status.is_success() {
        return Ok(format!("HTTP {status}: {text}"));
    }

    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(internal("admin_test_provider: parse response"))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No content in response");
    Ok(content.to_string())
}

#[cfg(feature = "ssr")]
pub async fn test_bot_provider(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    bot_provider_id: Uuid,
    prompt: &str,
) -> Result<TestBotProviderResponse, ServerFnError> {
    if prompt.chars().count() > 4096 {
        return Err(ServerFnError::new(
            "Test prompt must be at most 4096 characters",
        ));
    }

    type BotProviderTestRow = (
        String,
        Option<Vec<u8>>,
        String,
        Option<String>,
        Option<serde_json::Value>,
    );
    let row: Option<BotProviderTestRow> = sqlx::query_as(
        "SELECT p.url, p.api_key_encrypted, bp.model, bp.reasoning_effort, bp.extra_body \
             FROM bot_providers bp \
             JOIN llm_providers p ON p.id = bp.provider_id \
             WHERE bp.id = $1",
    )
    .bind(bot_provider_id)
    .fetch_optional(pool)
    .await
    .map_err(internal("admin_test_bot_provider: query"))?;

    let (url, api_key_encrypted, model, reasoning_effort, extra_body) =
        row.ok_or_else(|| ServerFnError::new("Bot provider not found"))?;

    let key = crate::crypto::load_key().map_err(internal("admin_test_bot_provider: load key"))?;
    let api_key = match api_key_encrypted {
        Some(encrypted) => {
            let decrypted = crate::crypto::decrypt(&key, &encrypted)
                .map_err(internal("admin_test_bot_provider: decrypt"))?;
            String::from_utf8(decrypted).map_err(internal("admin_test_bot_provider: utf8"))?
        }
        None => return Err(ServerFnError::new("Provider has no API key configured")),
    };

    let mut body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
    });
    if let Some(ref effort) = reasoning_effort {
        body["reasoning_effort"] = serde_json::json!(effort);
    }
    if let Some(ref patch) = extra_body
        && let (Some(base), Some(patch_obj)) = (body.as_object_mut(), patch.as_object())
    {
        for (k, v) in patch_obj {
            if v.is_null() {
                base.remove(k);
            } else {
                base.insert(k.clone(), v.clone());
            }
        }
    }

    let start = std::time::Instant::now();
    let resp = http_client
        .post(format!("{url}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&body)
        .send()
        .await
        .map_err(internal("admin_test_bot_provider: request"))?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let status = resp.status().as_u16();
    let headers = allowlisted_headers(resp.headers());
    let body = read_capped_body(resp).await;

    Ok(TestBotProviderResponse {
        status,
        headers,
        body,
        elapsed_ms,
    })
}

#[server(AdminListBots, "/api")]
pub async fn admin_list_bots() -> Result<Vec<BotRow>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_list_bots: check admin").await?;

    list_bots(&pool).await
}

#[server(AdminListDanglingBotNames, "/api")]
pub async fn admin_list_dangling_bot_names() -> Result<Vec<DanglingBotName>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_list_dangling_bot_names: check admin").await?;

    list_dangling_bot_names(&pool).await
}

#[server(AdminCreateBot, "/api")]
pub async fn admin_create_bot(
    name: String,
    temperature: f32,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
    can_replace_humans: bool,
) -> Result<BotRow, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_create_bot: check admin").await?;

    create_bot(
        &pool,
        name,
        temperature,
        include_basic_strategy,
        include_advanced_strategy,
        can_replace_humans,
    )
    .await
}

#[server(AdminUpdateBot, "/api")]
pub async fn admin_update_bot(
    id: Uuid,
    name: String,
    temperature: f32,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
    enabled: bool,
    can_replace_humans: bool,
) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_update_bot: check admin").await?;

    update_bot(
        &pool,
        id,
        name,
        temperature,
        include_basic_strategy,
        include_advanced_strategy,
        enabled,
        can_replace_humans,
    )
    .await
}

#[server(AdminReorderBots, "/api")]
pub async fn admin_reorder_bots(ordered_ids: Vec<Uuid>) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_reorder_bots: check admin").await?;

    reorder_bots(&pool, ordered_ids).await
}

#[server(AdminDeleteBot, "/api")]
pub async fn admin_delete_bot(id: Uuid) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_delete_bot: check admin").await?;

    delete_bot(&pool, id).await
}

#[server(AdminListProviders, "/api")]
pub async fn admin_list_providers() -> Result<Vec<ProviderRow>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_list_providers: check admin").await?;

    list_providers(&pool).await
}

#[server(AdminCreateProvider, "/api")]
pub async fn admin_create_provider(
    name: String,
    url: String,
    api_key: Option<String>,
) -> Result<ProviderRow, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_create_provider: check admin").await?;

    create_provider(&pool, name, url, api_key).await
}

#[server(AdminUpdateProvider, "/api")]
pub async fn admin_update_provider(
    id: Uuid,
    name: String,
    url: String,
    api_key: ApiKeyUpdate,
    enabled: bool,
) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_update_provider: check admin").await?;

    update_provider(&pool, id, name, url, api_key, enabled).await
}

#[server(AdminDeleteProvider, "/api")]
pub async fn admin_delete_provider(id: Uuid) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_delete_provider: check admin").await?;

    delete_provider(&pool, id).await
}

#[server(AdminListBotProviders, "/api")]
pub async fn admin_list_bot_providers() -> Result<Vec<BotProviderRow>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_list_bot_providers: check admin").await?;

    list_bot_providers(&pool).await
}

#[server(AdminCreateBotProvider, "/api")]
pub async fn admin_create_bot_provider(
    bot_id: Uuid,
    provider_id: Uuid,
    model: String,
    reasoning_effort: Option<String>,
    extra_body: Option<serde_json::Value>,
    priority: i32,
) -> Result<BotProviderRow, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_create_bot_provider: check admin").await?;

    create_bot_provider(
        &pool,
        bot_id,
        provider_id,
        model,
        reasoning_effort,
        extra_body,
        priority,
    )
    .await
}

#[server(AdminUpdateBotProvider, "/api")]
pub async fn admin_update_bot_provider(
    id: Uuid,
    model: String,
    reasoning_effort: Option<String>,
    extra_body: Option<serde_json::Value>,
    priority: i32,
    enabled: bool,
) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_update_bot_provider: check admin").await?;

    update_bot_provider(
        &pool,
        id,
        model,
        reasoning_effort,
        extra_body,
        priority,
        enabled,
    )
    .await
}

#[server(AdminDeleteBotProvider, "/api")]
pub async fn admin_delete_bot_provider(id: Uuid) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    require_admin(&pool, "admin_delete_bot_provider: check admin").await?;

    delete_bot_provider(&pool, id).await
}

#[server(AdminTestProvider, "/api")]
pub async fn admin_test_provider(
    provider_id: Uuid,
    model: Option<String>,
) -> Result<String, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    require_admin(&pool, "admin_test_provider: check admin").await?;

    test_provider(&pool, &http_client, provider_id, model).await
}

#[server(AdminTestBotProvider, "/api")]
pub async fn admin_test_bot_provider(
    bot_provider_id: Uuid,
    prompt: String,
) -> Result<TestBotProviderResponse, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    require_admin(&pool, "admin_test_bot_provider: check admin").await?;

    test_bot_provider(&pool, &http_client, bot_provider_id, &prompt).await
}

#[component]
pub fn AdminPage() -> impl IntoView {
    use crate::components::MainLayout;
    use leptos_router::{NavigateOptions, hooks::use_navigate};

    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();

    let navigate = use_navigate();
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(None))) {
            crate::app::hard_navigate("/login");
        }
    });

    let version = RwSignal::new(0u32);
    let bots: LocalResource<Result<Vec<BotRow>, ServerFnError>> = LocalResource::new(move || {
        version.track();
        admin_list_bots()
    });
    let providers: LocalResource<Result<Vec<ProviderRow>, ServerFnError>> =
        LocalResource::new(move || {
            version.track();
            admin_list_providers()
        });

    // ws F31: match the structured error variant against the shared
    // ADMIN_REQUIRED constant. `Display` for ServerFnError prefixes
    // "error running server function: ", so a string comparison on
    // `to_string()` would be both fragile and (for equality) always false.
    Effect::new(move |_| {
        if let Some(Err(ServerFnError::ServerError(msg))) = bots.get()
            && msg == ADMIN_REQUIRED
        {
            navigate("/", NavigateOptions::default());
        }
    });

    view! {
        <MainLayout>
            <div class="admin content-page">
                <h1>"Admin"</h1>
                <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                    {move || {
                        let bots_res = bots.get();
                        let providers_res = providers.get();
                        match (bots_res, providers_res) {
                            (Some(Err(e)), _) | (_, Some(Err(e))) => {
                                view! { <p class="error">{e.to_string()}</p> }.into_any()
                            }
                            (Some(Ok(bot_list)), Some(Ok(provider_list))) => view! {
                                <BotsSection bots=bot_list.clone() version=version/>
                                <ProvidersSection providers=provider_list.clone() version=version/>
                                <BotProvidersSection
                                    bots=bot_list
                                    providers=provider_list
                                    version=version
                                />
                            }.into_any(),
                            _ => view! { <p>"Loading..."</p> }.into_any(),
                        }
                    }}
                </Suspense>
            </div>
        </MainLayout>
    }
}

#[component]
fn BotsSection(bots: Vec<BotRow>, version: RwSignal<u32>) -> impl IntoView {
    let show_create = RwSignal::new(false);
    let editing_id = RwSignal::new(None::<Uuid>);
    let error = RwSignal::new(None::<String>);

    // Dangling bot names re-fetch on the same `version` signal as the rest of
    // the section (same pattern as `BotProvidersSection`'s `links`), so a
    // create/update/delete/reorder that bumps `version` refreshes the banner.
    let dangling: LocalResource<Result<Vec<DanglingBotName>, ServerFnError>> =
        LocalResource::new(move || {
            version.track();
            admin_list_dangling_bot_names()
        });

    let create_action = Action::new(
        |(name, temperature, basic, advanced, replace): &(String, f32, bool, bool, bool)| {
            let name = name.clone();
            let temperature = *temperature;
            let basic = *basic;
            let advanced = *advanced;
            let replace = *replace;
            async move { admin_create_bot(name, temperature, basic, advanced, replace).await }
        },
    );

    let update_action = Action::new(
        |(id, name, temperature, basic, advanced, enabled, replace): &(
            Uuid,
            String,
            f32,
            bool,
            bool,
            bool,
            bool,
        )| {
            let id = *id;
            let name = name.clone();
            let temperature = *temperature;
            let basic = *basic;
            let advanced = *advanced;
            let enabled = *enabled;
            let replace = *replace;
            async move {
                admin_update_bot(id, name, temperature, basic, advanced, enabled, replace).await
            }
        },
    );

    let delete_action = Action::new(|id: &Uuid| {
        let id = *id;
        async move { admin_delete_bot(id).await }
    });

    let reorder_action = Action::new(|ids: &Vec<Uuid>| {
        let ids = ids.clone();
        async move { admin_reorder_bots(ids).await }
    });

    Effect::new(move |_| {
        if let Some(result) = create_action.value().get() {
            match result {
                Ok(_) => {
                    show_create.set(false);
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = update_action.value().get() {
            match result {
                Ok(_) => {
                    editing_id.set(None);
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = delete_action.value().get() {
            match result {
                Ok(_) => {
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = reorder_action.value().get() {
            match result {
                Ok(_) => {
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    let bot_ids: Vec<Uuid> = bots.iter().map(|b| b.id).collect();
    let bots = StoredValue::new(bots);

    view! {
        <h2>"Bots"</h2>
        {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
        {move || {
            let names = dangling.get().and_then(|res| res.ok())?;
            if names.is_empty() {
                return None;
            }
            Some(view! {
                <div class="warning">
                    <p>
                        "These bot types are referenced by unfinished games but have no enabled bot configuration:"
                    </p>
                    <ul>
                        {names.into_iter().map(|d| {
                            view! {
                                <li>{format!("{} ({} unfinished game(s))", d.bot_name, d.game_count)}</li>
                            }
                        }).collect_view()}
                    </ul>
                </div>
            }.into_any())
        }}
        <table class="admin-table">
            <thead>
                <tr>
                    <th>"Name"</th>
                    <th>"Enabled"</th>
                    <th>"Temp"</th>
                    <th>"Basic"</th>
                    <th>"Advanced"</th>
                    <th>"Actions"</th>
                </tr>
            </thead>
            <tbody>
                {bots.with_value(|bots| bots.iter().enumerate().map(|(i, bot)| {
                    let id = bot.id;
                    let name = bot.name.clone();
                    let enabled = bot.enabled;
                    let temperature = bot.temperature;
                    let basic = bot.include_basic_strategy;
                    let advanced = bot.include_advanced_strategy;
                    let bot_name = bot.name.clone();
                    let bot_temperature = bot.temperature;
                    let bot_basic = bot.include_basic_strategy;
                    let bot_advanced = bot.include_advanced_strategy;
                    let bot_enabled = bot.enabled;
                    let bot_replace = bot.can_replace_humans;
                    let ids_up = bot_ids.clone();
                    let ids_down = bot_ids.clone();
                    let can_up = i > 0;
                    let can_down = i < bot_ids.len() - 1;
                    view! {
                        <tr>
                            <td>{name}</td>
                            <td>{if enabled { "Yes" } else { "No" }}</td>
                            <td>{format!("{:.1}", temperature)}</td>
                            <td>{if basic { "Yes" } else { "No" }}</td>
                            <td>{if advanced { "Yes" } else { "No" }}</td>
                            <td>
                                <div class="form-actions">
                                    <button on:click=move |_| editing_id.set(Some(id))>"Edit"</button>
                                    <button
                                        disabled=move || !can_up
                                        on:click=move |_| {
                                            let mut new_order = ids_up.clone();
                                            if i > 0 {
                                                new_order.swap(i, i - 1);
                                                reorder_action.dispatch(new_order);
                                            }
                                        }
                                    >"Up"</button>
                                    <button
                                        disabled=move || !can_down
                                        on:click=move |_| {
                                            let mut new_order = ids_down.clone();
                                            if i < new_order.len() - 1 {
                                                new_order.swap(i, i + 1);
                                                reorder_action.dispatch(new_order);
                                            }
                                        }
                                    >"Down"</button>
                                    <button on:click=move |_| {
                                        let confirmed = web_sys::window()
                                            .and_then(|w| w.confirm_with_message("Delete this bot?").ok())
                                            .unwrap_or(false);
                                        if confirmed {
                                            delete_action.dispatch(id);
                                        }
                                    }>"Delete"</button>
                                </div>
                            </td>
                        </tr>
                        <Show when=move || editing_id.get() == Some(id)>
                            <BotEditForm
                                bot_id=id
                                bot_name=bot_name.clone()
                                bot_temperature=bot_temperature
                                bot_basic=bot_basic
                                bot_advanced=bot_advanced
                                bot_enabled=bot_enabled
                                bot_can_replace_humans=bot_replace
                                update_action=update_action
                            />
                        </Show>
                    }
                }).collect_view())}
            </tbody>
        </table>
        <div class="form-actions">
            <button on:click=move |_| show_create.update(|v| *v = !*v)>"Add Bot"</button>
        </div>
        <Show when=move || show_create.get()>
            <BotCreateForm create_action=create_action/>
        </Show>
    }
}

#[component]
fn BotCreateForm(create_action: BotCreateAction) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
    let temp_input = NodeRef::<html::Input>::new();
    let basic_input = NodeRef::<html::Input>::new();
    let advanced_input = NodeRef::<html::Input>::new();
    let replace_input = NodeRef::<html::Input>::new();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name = name_input.get().map(|el| el.value()).unwrap_or_default();
        let temperature = temp_input
            .get()
            .and_then(|el| el.value().parse::<f32>().ok())
            .unwrap_or(0.2);
        let basic = basic_input.get().map(|el| el.checked()).unwrap_or(true);
        let advanced = advanced_input.get().map(|el| el.checked()).unwrap_or(false);
        let replace = replace_input.get().map(|el| el.checked()).unwrap_or(false);
        create_action.dispatch((name, temperature, basic, advanced, replace));
    };

    view! {
        <form on:submit=on_submit>
            <FormField label="Name">
                <input type="text" node_ref=name_input required/>
            </FormField>
            <FormField label="Temperature" help="0.0 to 2.0">
                <input type="number" node_ref=temp_input step="0.1" min="0" max="2" value="0.2"/>
            </FormField>
            <FormField label="Include basic strategy">
                <input type="checkbox" node_ref=basic_input checked=true/>
            </FormField>
            <FormField label="Include advanced strategy">
                <input type="checkbox" node_ref=advanced_input/>
            </FormField>
            <FormField label="Can replace humans">
                <input type="checkbox" node_ref=replace_input/>
            </FormField>
            <div class="form-actions">
                <input type="submit" value="Create" disabled=move || create_action.pending().get()/>
            </div>
        </form>
    }
}

#[component]
fn BotEditForm(
    bot_id: Uuid,
    bot_name: String,
    bot_temperature: f32,
    bot_basic: bool,
    bot_advanced: bool,
    bot_enabled: bool,
    bot_can_replace_humans: bool,
    update_action: BotUpdateAction,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
    let temp_input = NodeRef::<html::Input>::new();
    let basic_input = NodeRef::<html::Input>::new();
    let advanced_input = NodeRef::<html::Input>::new();
    let enabled_input = NodeRef::<html::Input>::new();
    let replace_input = NodeRef::<html::Input>::new();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name = name_input.get().map(|el| el.value()).unwrap_or_default();
        let temperature = temp_input
            .get()
            .and_then(|el| el.value().parse::<f32>().ok())
            .unwrap_or(0.2);
        let basic = basic_input.get().map(|el| el.checked()).unwrap_or(true);
        let advanced = advanced_input.get().map(|el| el.checked()).unwrap_or(false);
        let enabled = enabled_input.get().map(|el| el.checked()).unwrap_or(true);
        let replace = replace_input.get().map(|el| el.checked()).unwrap_or(false);
        update_action.dispatch((bot_id, name, temperature, basic, advanced, enabled, replace));
    };

    view! {
        <tr>
            <td colspan="6">
                <form on:submit=on_submit>
                    <FormField label="Name">
                        <input type="text" node_ref=name_input required prop:value=bot_name/>
                    </FormField>
                    <FormField label="Temperature" help="0.0 to 2.0">
                        <input
                            type="number"
                            node_ref=temp_input
                            step="0.1"
                            min="0"
                            max="2"
                            prop:value=format!("{:.1}", bot_temperature)
                        />
                    </FormField>
                    <FormField label="Include basic strategy">
                        <input type="checkbox" node_ref=basic_input prop:checked=bot_basic/>
                    </FormField>
                    <FormField label="Include advanced strategy">
                        <input type="checkbox" node_ref=advanced_input prop:checked=bot_advanced/>
                    </FormField>
                    <FormField label="Enabled">
                        <input type="checkbox" node_ref=enabled_input prop:checked=bot_enabled/>
                    </FormField>
                    <FormField label="Can replace humans">
                        <input
                            type="checkbox"
                            node_ref=replace_input
                            prop:checked=bot_can_replace_humans
                        />
                    </FormField>
                    <div class="form-actions">
                        <input type="submit" value="Save" disabled=move || update_action.pending().get()/>
                    </div>
                </form>
            </td>
        </tr>
    }
}

#[component]
fn ProvidersSection(providers: Vec<ProviderRow>, version: RwSignal<u32>) -> impl IntoView {
    let show_create = RwSignal::new(false);
    let editing_id = RwSignal::new(None::<Uuid>);
    let error = RwSignal::new(None::<String>);

    let create_action = Action::new(|(name, url, api_key): &(String, String, Option<String>)| {
        let name = name.clone();
        let url = url.clone();
        let api_key = api_key.clone();
        async move { admin_create_provider(name, url, api_key).await }
    });

    let update_action = Action::new(
        |(id, name, url, api_key, enabled): &(Uuid, String, String, ApiKeyUpdate, bool)| {
            let id = *id;
            let name = name.clone();
            let url = url.clone();
            let api_key = api_key.clone();
            let enabled = *enabled;
            async move { admin_update_provider(id, name, url, api_key, enabled).await }
        },
    );

    let delete_action = Action::new(|id: &Uuid| {
        let id = *id;
        async move { admin_delete_provider(id).await }
    });

    // Optional model override for the provider health check; blank means
    // "use the provider's configured link model" (ws F22).
    let test_model = RwSignal::new(String::new());
    // ws F24: the id travels with the result, so a completed test can never be
    // attributed to another row (and cannot be dropped because `input()` has
    // already been cleared by the time `value()` lands).
    let test_action = Action::new(|(id, model): &(Uuid, Option<String>)| {
        let id = *id;
        let model = model.clone();
        async move { (id, admin_test_provider(id, model).await) }
    });

    let test_result = RwSignal::new(None::<(Uuid, Result<String, String>)>);

    Effect::new(move |_| {
        if let Some(result) = create_action.value().get() {
            match result {
                Ok(_) => {
                    show_create.set(false);
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = update_action.value().get() {
            match result {
                Ok(_) => {
                    editing_id.set(None);
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = delete_action.value().get() {
            match result {
                Ok(_) => {
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some((provider_id, result)) = test_action.value().get() {
            let res = result.map_err(|e| e.to_string());
            test_result.set(Some((provider_id, res)));
        }
    });

    let providers = StoredValue::new(providers);

    view! {
        <h2>"Providers"</h2>
        {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
        <div class="form-actions">
            <label>"Test model (blank = provider's configured model): "</label>
            <input
                type="text"
                prop:value=move || test_model.get()
                on:input=move |ev| test_model.set(event_target_value(&ev))
            />
        </div>
        <table class="admin-table">
            <thead>
                <tr>
                    <th>"Name"</th>
                    <th>"URL"</th>
                    <th>"API Key"</th>
                    <th>"Enabled"</th>
                    <th>"Actions"</th>
                </tr>
            </thead>
            <tbody>
                {providers.with_value(|providers| providers.iter().map(|provider| {
                    let id = provider.id;
                    let name = provider.name.clone();
                    let url = provider.url.clone();
                    let api_key_masked = provider.api_key_masked.clone().unwrap_or_else(|| "None".to_string());
                    let enabled = provider.enabled;
                    let provider_name = provider.name.clone();
                    let provider_url = provider.url.clone();
                    let provider_enabled = provider.enabled;
                    view! {
                        <tr>
                            <td>{name}</td>
                            <td>{url}</td>
                            <td>{api_key_masked}</td>
                            <td>{if enabled { "Yes" } else { "No" }}</td>
                            <td>
                                <div class="form-actions">
                                    <button on:click=move |_| editing_id.set(Some(id))>"Edit"</button>
                                    <button
                                        disabled=move || {
                                            test_action.pending().get()
                                                && test_action.input().get().is_some_and(|(tid, _)| tid == id)
                                        }
                                        on:click=move |_| {
                                            let m = test_model.get();
                                            let m = if m.trim().is_empty() { None } else { Some(m) };
                                            test_action.dispatch((id, m));
                                        }
                                    >
                                        {move || {
                                            if test_action.pending().get()
                                                && test_action.input().get().is_some_and(|(tid, _)| tid == id)
                                            {
                                                "Testing..."
                                            } else {
                                                "Test"
                                            }
                                        }}
                                    </button>
                                    <button on:click=move |_| {
                                        let confirmed = web_sys::window()
                                            .and_then(|w| w.confirm_with_message("Delete this provider?").ok())
                                            .unwrap_or(false);
                                        if confirmed {
                                            delete_action.dispatch(id);
                                        }
                                    }>"Delete"</button>
                                </div>
                            </td>
                        </tr>
                        <Show when=move || editing_id.get() == Some(id)>
                            <ProviderEditForm
                                provider_id=id
                                provider_name=provider_name.clone()
                                provider_url=provider_url.clone()
                                provider_enabled=provider_enabled
                                update_action=update_action
                            />
                        </Show>
                        <Show when=move || {
                            test_result.with(|r| r.as_ref().is_some_and(|(pid, _)| *pid == id))
                        }>
                            <tr>
                                <td colspan="5">
                                    {move || {
                                        test_result.with(|r| match r {
                                            Some((_, Ok(msg))) => {
                                                view! { <p>{msg.clone()}</p> }.into_any()
                                            }
                                            Some((_, Err(e))) => {
                                                view! { <p class="error">{e.clone()}</p> }.into_any()
                                            }
                                            None => ().into_any(),
                                        })
                                    }}
                                </td>
                            </tr>
                        </Show>
                    }
                }).collect_view())}
            </tbody>
        </table>
        <div class="form-actions">
            <button on:click=move |_| show_create.update(|v| *v = !*v)>"Add Provider"</button>
        </div>
        <Show when=move || show_create.get()>
            <ProviderCreateForm create_action=create_action/>
        </Show>
    }
}

#[component]
fn ProviderCreateForm(
    create_action: Action<(String, String, Option<String>), Result<ProviderRow, ServerFnError>>,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
    let url_input = NodeRef::<html::Input>::new();
    let key_input = NodeRef::<html::Input>::new();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name = name_input.get().map(|el| el.value()).unwrap_or_default();
        let url = url_input.get().map(|el| el.value()).unwrap_or_default();
        let api_key = key_input
            .get()
            .map(|el| el.value())
            .filter(|v| !v.is_empty());
        create_action.dispatch((name, url, api_key));
    };

    view! {
        <form on:submit=on_submit>
            <FormField label="Name">
                <input type="text" node_ref=name_input required/>
            </FormField>
            <FormField label="URL" help="e.g. https://openrouter.ai/api">
                <input type="text" node_ref=url_input required/>
            </FormField>
            <FormField label="API Key" help="Optional">
                <input type="password" node_ref=key_input/>
            </FormField>
            <div class="form-actions">
                <input type="submit" value="Create" disabled=move || create_action.pending().get()/>
            </div>
        </form>
    }
}

#[component]
fn ProviderEditForm(
    provider_id: Uuid,
    provider_name: String,
    provider_url: String,
    provider_enabled: bool,
    update_action: ProviderUpdateAction,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
    let url_input = NodeRef::<html::Input>::new();
    let key_input = NodeRef::<html::Input>::new();
    let clear_input = NodeRef::<html::Input>::new();
    let enabled_input = NodeRef::<html::Input>::new();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name = name_input.get().map(|el| el.value()).unwrap_or_default();
        let url = url_input.get().map(|el| el.value()).unwrap_or_default();
        // ws F21: blank still means "keep" - the explicit checkbox is the
        // only way to revoke, and it wins over anything typed in the field.
        let api_key = if clear_input.get().map(|el| el.checked()).unwrap_or(false) {
            ApiKeyUpdate::Clear
        } else {
            match key_input.get().map(|el| el.value()) {
                Some(v) if !v.is_empty() => ApiKeyUpdate::Set(v),
                _ => ApiKeyUpdate::Keep,
            }
        };
        let enabled = enabled_input.get().map(|el| el.checked()).unwrap_or(true);
        update_action.dispatch((provider_id, name, url, api_key, enabled));
    };

    view! {
        <tr>
            <td colspan="5">
                <form on:submit=on_submit>
                    <FormField label="Name">
                        <input type="text" node_ref=name_input required prop:value=provider_name/>
                    </FormField>
                    <FormField label="URL">
                        <input type="text" node_ref=url_input required prop:value=provider_url/>
                    </FormField>
                    <FormField label="API Key" help="Leave blank to keep existing key">
                        <input type="password" node_ref=key_input/>
                    </FormField>
                    <FormField
                        label="Clear API key"
                        help="Removes the stored key. Overrides anything typed above."
                    >
                        <input type="checkbox" node_ref=clear_input/>
                    </FormField>
                    <FormField label="Enabled">
                        <input type="checkbox" node_ref=enabled_input prop:checked=provider_enabled/>
                    </FormField>
                    <div class="form-actions">
                        <input type="submit" value="Save" disabled=move || update_action.pending().get()/>
                    </div>
                </form>
            </td>
        </tr>
    }
}

#[component]
fn BotProvidersSection(
    bots: Vec<BotRow>,
    providers: Vec<ProviderRow>,
    version: RwSignal<u32>,
) -> impl IntoView {
    let show_create = RwSignal::new(false);
    let editing_id = RwSignal::new(None::<Uuid>);
    let error = RwSignal::new(None::<String>);

    let links: LocalResource<Result<Vec<BotProviderRow>, ServerFnError>> =
        LocalResource::new(move || {
            version.track();
            admin_list_bot_providers()
        });

    let create_action = Action::new(
        |(bot_id, provider_id, model, reasoning_effort, extra_body, priority): &(
            Uuid,
            Uuid,
            String,
            Option<String>,
            Option<serde_json::Value>,
            i32,
        )| {
            let bot_id = *bot_id;
            let provider_id = *provider_id;
            let model = model.clone();
            let reasoning_effort = reasoning_effort.clone();
            let extra_body = extra_body.clone();
            let priority = *priority;
            async move {
                admin_create_bot_provider(
                    bot_id,
                    provider_id,
                    model,
                    reasoning_effort,
                    extra_body,
                    priority,
                )
                .await
            }
        },
    );

    let update_action = Action::new(
        |(id, model, reasoning_effort, extra_body, priority, enabled): &(
            Uuid,
            String,
            Option<String>,
            Option<serde_json::Value>,
            i32,
            bool,
        )| {
            let id = *id;
            let model = model.clone();
            let reasoning_effort = reasoning_effort.clone();
            let extra_body = extra_body.clone();
            let priority = *priority;
            let enabled = *enabled;
            async move {
                admin_update_bot_provider(
                    id,
                    model,
                    reasoning_effort,
                    extra_body,
                    priority,
                    enabled,
                )
                .await
            }
        },
    );

    let delete_action = Action::new(|id: &Uuid| {
        let id = *id;
        async move { admin_delete_bot_provider(id).await }
    });

    let test_prompt = RwSignal::new("Say hello".to_string());
    let test_action = Action::new(|(id, prompt): &(Uuid, String)| {
        let id = *id;
        let prompt = prompt.clone();
        async move { (id, admin_test_bot_provider(id, prompt).await) }
    });
    let test_result = RwSignal::new(None::<(Uuid, Result<TestBotProviderResponse, String>)>);

    Effect::new(move |_| {
        if let Some(result) = create_action.value().get() {
            match result {
                Ok(_) => {
                    show_create.set(false);
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = update_action.value().get() {
            match result {
                Ok(_) => {
                    editing_id.set(None);
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = delete_action.value().get() {
            match result {
                Ok(_) => {
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if let Some((bp_id, result)) = test_action.value().get() {
            let res = result.map_err(|e| e.to_string());
            test_result.set(Some((bp_id, res)));
        }
    });

    let bots = StoredValue::new(bots);
    let providers = StoredValue::new(providers);

    view! {
        <h2>"Bot-Provider Links"</h2>
        {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
        <div class="form-actions">
            <label>"Test prompt: "</label>
            <input
                type="text"
                prop:value=move || test_prompt.get()
                on:input=move |ev| test_prompt.set(event_target_value(&ev))
            />
        </div>
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                links.get().map(|res| match res {
                    Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                    Ok(link_list) => view! {
                        <table class="admin-table">
                            <thead>
                                <tr>
                                    <th>"Bot"</th>
                                    <th>"Provider"</th>
                                    <th>"Model"</th>
                                    <th>"Reasoning"</th>
                                    <th>"Priority"</th>
                                    <th>"Extra Body"</th>
                                    <th>"Enabled"</th>
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {link_list.iter().map(|link| {
                                    let id = link.id;
                                    let bot_name = link.bot_name.clone();
                                    let provider_name = link.provider_name.clone();
                                    let model = link.model.clone();
                                    let reasoning = link.reasoning_effort.clone().unwrap_or_default();
                                    let priority = link.priority;
                                    let has_extra = link.extra_body.is_some();
                                    let enabled = link.enabled;
                                    let link_model = link.model.clone();
                                    let link_reasoning = link.reasoning_effort.clone();
                                    let link_extra = link.extra_body.clone();
                                    let link_priority = link.priority;
                                    let link_enabled = link.enabled;
                                    view! {
                                        <tr>
                                            <td>{bot_name}</td>
                                            <td>{provider_name}</td>
                                            <td>{model}</td>
                                            <td>{reasoning}</td>
                                            <td>{priority}</td>
                                            <td>{if has_extra { "Yes" } else { "None" }}</td>
                                            <td>{if enabled { "Yes" } else { "No" }}</td>
                                            <td>
                                                <div class="form-actions">
                                                    <button on:click=move |_| editing_id.set(Some(id))>"Edit"</button>
                                                    <button
                                                        disabled=move || {
                                                            test_action.pending().get()
                                                                && test_action.input().get()
                                                                    .is_some_and(|(tid, _)| tid == id)
                                                        }
                                                        on:click=move |_| {
                                                            test_action.dispatch((id, test_prompt.get()));
                                                        }
                                                    >
                                                        {move || {
                                                            if test_action.pending().get()
                                                                && test_action.input().get()
                                                                    .is_some_and(|(tid, _)| tid == id)
                                                            {
                                                                "Testing..."
                                                            } else {
                                                                "Test"
                                                            }
                                                        }}
                                                    </button>
                                                    <button on:click=move |_| {
                                                        let confirmed = web_sys::window()
                                                            .and_then(|w| w.confirm_with_message("Delete this link?").ok())
                                                            .unwrap_or(false);
                                                        if confirmed {
                                                            delete_action.dispatch(id);
                                                        }
                                                    }>"Delete"</button>
                                                </div>
                                            </td>
                                        </tr>
                                        <Show when=move || {
                                            test_result.with(|r| r.as_ref().is_some_and(|(rid, _)| *rid == id))
                                        }>
                                            <tr>
                                                <td colspan="8">
                                                    {move || {
                                                        test_result.with(|r| match r {
                                                            Some((_, Ok(resp))) => view! {
                                                                <div class="test-result">
                                                                    <p><strong>"Status: "</strong>{resp.status}
                                                                        " | "<strong>"Time: "</strong>{resp.elapsed_ms}"ms"</p>
                                                                    <details>
                                                                        <summary>"Headers"</summary>
                                                                        <pre>{resp.headers.iter()
                                                                            .map(|(k, v)| format!("{k}: {v}"))
                                                                            .collect::<Vec<_>>()
                                                                            .join("\n")}</pre>
                                                                    </details>
                                                                    <details>
                                                                        <summary>"Body"</summary>
                                                                        <pre>{resp.body.clone()}</pre>
                                                                    </details>
                                                                </div>
                                                            }.into_any(),
                                                            Some((_, Err(e))) => {
                                                                view! { <p class="error">{e.clone()}</p> }.into_any()
                                                            }
                                                            None => ().into_any(),
                                                        })
                                                    }}
                                                </td>
                                            </tr>
                                        </Show>
                                        <Show when=move || editing_id.get() == Some(id)>
                                            <BotProviderEditForm
                                                link_id=id
                                                link_model=link_model.clone()
                                                link_reasoning=link_reasoning.clone()
                                                link_extra=link_extra.clone()
                                                link_priority=link_priority
                                                link_enabled=link_enabled
                                                update_action=update_action
                                            />
                                        </Show>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    }.into_any(),
                })
            }}
        </Suspense>
        <div class="form-actions">
            <button on:click=move |_| show_create.update(|v| *v = !*v)>"Add Link"</button>
        </div>
        <Show when=move || show_create.get()>
            <BotProviderCreateForm
                bots=bots
                providers=providers
                create_action=create_action
            />
        </Show>
    }
}

#[component]
fn BotProviderCreateForm(
    bots: StoredValue<Vec<BotRow>>,
    providers: StoredValue<Vec<ProviderRow>>,
    create_action: BotProviderCreateAction,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let bot_select = NodeRef::<html::Select>::new();
    let provider_select = NodeRef::<html::Select>::new();
    let model_input = NodeRef::<html::Input>::new();
    let reasoning_input = NodeRef::<html::Input>::new();
    let extra_input = NodeRef::<html::Textarea>::new();
    let priority_input = NodeRef::<html::Input>::new();
    let json_error = RwSignal::new(None::<String>);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let bot_id = bot_select
            .get()
            .and_then(|el| Uuid::parse_str(&el.value()).ok());
        let provider_id = provider_select
            .get()
            .and_then(|el| Uuid::parse_str(&el.value()).ok());
        let model = model_input.get().map(|el| el.value()).unwrap_or_default();
        let reasoning_effort = reasoning_input
            .get()
            .map(|el| el.value())
            .filter(|v| !v.trim().is_empty());
        let extra_raw = extra_input.get().map(|el| el.value()).unwrap_or_default();
        let extra_body = if extra_raw.trim().is_empty() {
            json_error.set(None);
            None
        } else {
            match serde_json::from_str::<serde_json::Value>(&extra_raw) {
                Ok(v) => {
                    json_error.set(None);
                    Some(v)
                }
                Err(_) => {
                    json_error.set(Some("Invalid JSON".to_string()));
                    return;
                }
            }
        };
        let priority = priority_input
            .get()
            .and_then(|el| el.value().parse::<i32>().ok())
            .unwrap_or(0);
        if let (Some(bot_id), Some(provider_id)) = (bot_id, provider_id) {
            create_action.dispatch((
                bot_id,
                provider_id,
                model,
                reasoning_effort,
                extra_body,
                priority,
            ));
        }
    };

    view! {
        <form on:submit=on_submit>
            <FormField label="Bot">
                <select node_ref=bot_select required>
                    <option value="" disabled selected>"Select a bot"</option>
                    {bots.with_value(|bots| bots.iter().map(|b| {
                        let id = b.id.to_string();
                        let name = b.name.clone();
                        view! { <option value=id>{name}</option> }
                    }).collect_view())}
                </select>
            </FormField>
            <FormField label="Provider">
                <select node_ref=provider_select required>
                    <option value="" disabled selected>"Select a provider"</option>
                    {providers.with_value(|providers| providers.iter().map(|p| {
                        let id = p.id.to_string();
                        let name = p.name.clone();
                        view! { <option value=id>{name}</option> }
                    }).collect_view())}
                </select>
            </FormField>
            <FormField label="Model" help="e.g. openai/gpt-4o-mini">
                <input type="text" node_ref=model_input required/>
            </FormField>
            <FormField label="Reasoning effort" help="Optional: low, medium, high">
                <input type="text" node_ref=reasoning_input/>
            </FormField>
            <FormField
                label="Extra body"
                help="Optional JSON"
                error=Signal::derive(move || json_error.get())
            >
                <textarea node_ref=extra_input rows="3"></textarea>
            </FormField>
            <FormField label="Priority" help="Lower = tried first. Same = round-robin.">
                <input type="number" node_ref=priority_input value="0"/>
            </FormField>
            <div class="form-actions">
                <input type="submit" value="Create" disabled=move || create_action.pending().get()/>
            </div>
        </form>
    }
}

#[component]
fn BotProviderEditForm(
    link_id: Uuid,
    link_model: String,
    link_reasoning: Option<String>,
    link_extra: Option<serde_json::Value>,
    link_priority: i32,
    link_enabled: bool,
    update_action: BotProviderUpdateAction,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let model_input = NodeRef::<html::Input>::new();
    let reasoning_input = NodeRef::<html::Input>::new();
    let extra_input = NodeRef::<html::Textarea>::new();
    let priority_input = NodeRef::<html::Input>::new();
    let enabled_input = NodeRef::<html::Input>::new();
    let json_error = RwSignal::new(None::<String>);

    let extra_display = link_extra
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default())
        .unwrap_or_default();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let model = model_input.get().map(|el| el.value()).unwrap_or_default();
        let reasoning_effort = reasoning_input
            .get()
            .map(|el| el.value())
            .filter(|v| !v.trim().is_empty());
        let extra_raw = extra_input.get().map(|el| el.value()).unwrap_or_default();
        let extra_body = if extra_raw.trim().is_empty() {
            json_error.set(None);
            None
        } else {
            match serde_json::from_str::<serde_json::Value>(&extra_raw) {
                Ok(v) => {
                    json_error.set(None);
                    Some(v)
                }
                Err(_) => {
                    json_error.set(Some("Invalid JSON".to_string()));
                    return;
                }
            }
        };
        let priority = priority_input
            .get()
            .and_then(|el| el.value().parse::<i32>().ok())
            .unwrap_or(0);
        let enabled = enabled_input.get().map(|el| el.checked()).unwrap_or(true);
        update_action.dispatch((
            link_id,
            model,
            reasoning_effort,
            extra_body,
            priority,
            enabled,
        ));
    };

    view! {
        <tr>
            <td colspan="8">
                <form on:submit=on_submit>
                    <FormField label="Model">
                        <input type="text" node_ref=model_input required prop:value=link_model/>
                    </FormField>
                    <FormField label="Reasoning effort" help="Optional: low, medium, high">
                        <input
                            type="text"
                            node_ref=reasoning_input
                            prop:value=link_reasoning.unwrap_or_default()
                        />
                    </FormField>
                    <FormField
                        label="Extra body"
                        help="Optional JSON"
                        error=Signal::derive(move || json_error.get())
                    >
                        <textarea node_ref=extra_input rows="3" prop:value=extra_display></textarea>
                    </FormField>
                    <FormField label="Priority">
                        <input
                            type="number"
                            node_ref=priority_input
                            prop:value=link_priority.to_string()
                        />
                    </FormField>
                    <FormField label="Enabled">
                        <input type="checkbox" node_ref=enabled_input prop:checked=link_enabled/>
                    </FormField>
                    <div class="form-actions">
                        <input type="submit" value="Save" disabled=move || update_action.pending().get()/>
                    </div>
                </form>
            </td>
        </tr>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_encryption_key() -> crate::crypto::Zeroizing<[u8; 32]> {
        crate::crypto::load_key().unwrap()
    }

    #[sqlx::test]
    async fn test_admin_list_bots_rejects_non_admin(pool: sqlx::PgPool) {
        sqlx::query(
            "INSERT INTO users (id, name, pref_colors, is_admin) VALUES ($1, $2, $3, false)",
        )
        .bind(Uuid::new_v4())
        .bind("testuser")
        .bind(Vec::<String>::new())
        .execute(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE name = 'testuser'")
            .fetch_one(&pool)
            .await
            .unwrap();

        let is_admin = crate::db::is_user_admin(&pool, user_id).await.unwrap();
        assert!(!is_admin);
    }

    #[sqlx::test]
    async fn test_admin_list_dangling_bot_names_rejects_non_admin(pool: sqlx::PgPool) {
        sqlx::query(
            "INSERT INTO users (id, name, pref_colors, is_admin) VALUES ($1, $2, $3, false)",
        )
        .bind(Uuid::new_v4())
        .bind("testuser")
        .bind(Vec::<String>::new())
        .execute(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE name = 'testuser'")
            .fetch_one(&pool)
            .await
            .unwrap();

        let is_admin = crate::db::is_user_admin(&pool, user_id).await.unwrap();
        assert!(!is_admin);
    }

    /// Seed an unfinished (`is_finished = false`) or finished game with the
    /// minimal `game_types`/`game_versions` parents `games` requires.
    async fn seed_dangling_game(pool: &sqlx::PgPool, is_finished: bool) -> Uuid {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Dangling {}", Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(pool)
        .await
        .unwrap();
        let game_version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://127.0.0.1:1', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query_scalar(
            "INSERT INTO games (game_version_id, is_finished, game_state)
             VALUES ($1, $2, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .bind(is_finished)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    /// Reference a bot type (`bot_name`) from a game. `game_bots.name` is the
    /// per-game display name and is irrelevant here, so it is randomized.
    async fn seed_game_bot(pool: &sqlx::PgPool, game_id: Uuid, bot_name: &str) {
        sqlx::query("INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3)")
            .bind(game_id)
            .bind(format!("Bot {}", Uuid::new_v4()))
            .bind(bot_name)
            .execute(pool)
            .await
            .unwrap();
    }

    /// Every referenced bot resolves to an enabled `bots` row: nothing dangles.
    #[sqlx::test]
    async fn test_list_dangling_bot_names_none_when_all_enabled(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO bots (name, enabled) VALUES ($1, true)")
            .bind("present-bot")
            .execute(&pool)
            .await
            .unwrap();
        let game_id = seed_dangling_game(&pool, false).await;
        seed_game_bot(&pool, game_id, "present-bot").await;

        let dangling = list_dangling_bot_names(&pool).await.unwrap();
        assert!(
            dangling.is_empty(),
            "an enabled bot must not be reported dangling, got {dangling:?}"
        );
    }

    /// An EMPTY `bots` table must yield zero rows even though an unfinished
    /// game references a bot name: the bot service falls back to a synthetic
    /// config then, so there is nothing to warn about (D-05/D-08).
    #[sqlx::test]
    async fn test_list_dangling_bot_names_none_when_bots_table_empty(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots")
            .execute(&pool)
            .await
            .unwrap();
        let game_id = seed_dangling_game(&pool, false).await;
        seed_game_bot(&pool, game_id, "orphan-bot").await;

        let dangling = list_dangling_bot_names(&pool).await.unwrap();
        assert!(
            dangling.is_empty(),
            "an empty bots table must yield zero dangling names, got {dangling:?}"
        );
    }

    /// A renamed-away name (no `bots.name` match) and a disabled bot are both
    /// dangling, counted by distinct unfinished game; an enabled bot is not;
    /// and a FINISHED game referencing a dangling name is not counted.
    #[sqlx::test]
    async fn test_list_dangling_bot_names_counts_renamed_and_disabled(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO bots (name, enabled) VALUES ($1, false)")
            .bind("disabled-bot")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO bots (name, enabled) VALUES ($1, true)")
            .bind("live-bot")
            .execute(&pool)
            .await
            .unwrap();

        // Two unfinished games + one FINISHED game reference a renamed-away
        // name; only the two unfinished ones count.
        let g1 = seed_dangling_game(&pool, false).await;
        let g2 = seed_dangling_game(&pool, false).await;
        let g3 = seed_dangling_game(&pool, true).await;
        seed_game_bot(&pool, g1, "renamed-away").await;
        seed_game_bot(&pool, g2, "renamed-away").await;
        seed_game_bot(&pool, g3, "renamed-away").await;

        // One unfinished game references the disabled bot.
        let g4 = seed_dangling_game(&pool, false).await;
        seed_game_bot(&pool, g4, "disabled-bot").await;

        // One unfinished game references the enabled bot - not dangling.
        let g5 = seed_dangling_game(&pool, false).await;
        seed_game_bot(&pool, g5, "live-bot").await;

        let dangling = list_dangling_bot_names(&pool).await.unwrap();
        let by_name: std::collections::HashMap<String, i64> = dangling
            .into_iter()
            .map(|d| (d.bot_name, d.game_count))
            .collect();
        assert_eq!(
            by_name.get("renamed-away").copied(),
            Some(2),
            "renamed-away must count the 2 unfinished games, not the finished one: {by_name:?}"
        );
        assert_eq!(
            by_name.get("disabled-bot").copied(),
            Some(1),
            "a disabled bot is dangling: {by_name:?}"
        );
        assert!(
            !by_name.contains_key("live-bot"),
            "an enabled bot must not be dangling: {by_name:?}"
        );
        assert_eq!(
            by_name.len(),
            2,
            "expected exactly 2 dangling names: {by_name:?}"
        );
    }

    #[sqlx::test]
    async fn test_admin_list_providers_never_returns_full_key(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let api_key = "sk-test-secret-key-1234";
        let encrypted = crate::crypto::encrypt(&key, api_key.as_bytes()).unwrap();

        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(Uuid::new_v4())
        .bind("test-provider")
        .bind("http://localhost:8080")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

        let providers = list_providers(&pool).await.unwrap();

        assert_eq!(providers.len(), 1);
        let masked = providers[0].api_key_masked.as_ref().unwrap();
        assert_eq!(masked, "...1234");
        assert!(!masked.contains(api_key));
    }

    #[sqlx::test]
    async fn test_admin_create_provider_encrypts_key(pool: sqlx::PgPool) {
        let key = test_encryption_key();

        let api_key = "sk-another-secret-key-5678";
        let provider = create_provider(
            &pool,
            "enc-test".to_string(),
            "http://localhost:9090".to_string(),
            Some(api_key.to_string()),
        )
        .await
        .unwrap();

        let raw: Vec<u8> =
            sqlx::query_scalar("SELECT api_key_encrypted FROM llm_providers WHERE id = $1")
                .bind(provider.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_ne!(raw, api_key.as_bytes());
        let decrypted = crate::crypto::decrypt(&key, &raw).unwrap();
        assert_eq!(String::from_utf8(decrypted).unwrap(), api_key);
    }

    #[sqlx::test]
    async fn test_admin_update_provider_preserves_key_when_none(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let api_key = "sk-original-key-9999";
        let encrypted = crate::crypto::encrypt(&key, api_key.as_bytes()).unwrap();

        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("preserve-test")
        .bind("http://localhost:8080")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

        update_provider(
            &pool,
            provider_id,
            "preserve-test-renamed".to_string(),
            "http://localhost:8081".to_string(),
            ApiKeyUpdate::Keep,
            true,
        )
        .await
        .unwrap();

        let raw: Vec<u8> =
            sqlx::query_scalar("SELECT api_key_encrypted FROM llm_providers WHERE id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(raw, encrypted);
    }

    #[sqlx::test]
    async fn test_admin_update_provider_replaces_key_when_some(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let original_key = "sk-original-key-1111";
        let encrypted = crate::crypto::encrypt(&key, original_key.as_bytes()).unwrap();

        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("replace-test")
        .bind("http://localhost:8080")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

        let new_key = "sk-new-key-2222";
        update_provider(
            &pool,
            provider_id,
            "replace-test".to_string(),
            "http://localhost:8080".to_string(),
            ApiKeyUpdate::Set(new_key.to_string()),
            true,
        )
        .await
        .unwrap();

        let raw: Vec<u8> =
            sqlx::query_scalar("SELECT api_key_encrypted FROM llm_providers WHERE id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_ne!(raw, encrypted);
        let decrypted = crate::crypto::decrypt(&key, &raw).unwrap();
        assert_eq!(String::from_utf8(decrypted).unwrap(), new_key);
    }

    /// ws F21: Clear must actually NULL the column, and the listing must then
    /// report no key rather than a mask.
    #[sqlx::test]
    async fn test_admin_update_provider_clears_key(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let encrypted = crate::crypto::encrypt(&key, b"sk-to-be-revoked-1234").unwrap();
        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("clear-test")
        .bind("http://localhost:8080")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

        update_provider(
            &pool,
            provider_id,
            "clear-test".to_string(),
            "http://localhost:8080".to_string(),
            ApiKeyUpdate::Clear,
            true,
        )
        .await
        .unwrap();

        let raw: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT api_key_encrypted FROM llm_providers WHERE id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(raw, None, "Clear must NULL api_key_encrypted");

        let providers = list_providers(&pool).await.unwrap();
        assert_eq!(providers[0].api_key_masked, None);

        // And the name/url/enabled columns still updated in the Clear arm.
        update_provider(
            &pool,
            provider_id,
            "clear-test-renamed".to_string(),
            "http://localhost:9999".to_string(),
            ApiKeyUpdate::Clear,
            false,
        )
        .await
        .unwrap();
        let (name, url, enabled): (String, String, bool) =
            sqlx::query_as("SELECT name, url, enabled FROM llm_providers WHERE id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name, "clear-test-renamed");
        assert_eq!(url, "http://localhost:9999");
        assert!(!enabled);
    }

    /// ws F25: crafted server-fn arguments are rejected before they reach SQL.
    #[sqlx::test]
    async fn test_bot_input_validation(pool: sqlx::PgPool) {
        // Empty / whitespace-only name.
        for name in ["", "   "] {
            let err = create_bot(&pool, name.to_string(), 0.2, true, false, false)
                .await
                .expect_err("empty bot name must be rejected");
            assert!(err.to_string().contains("Bot name is required"), "{err}");
        }
        // Over-long name.
        let err = create_bot(&pool, "x".repeat(65), 0.2, true, false, false)
            .await
            .expect_err("over-long bot name must be rejected");
        assert!(err.to_string().contains("at most 64"), "{err}");
        // Non-finite and out-of-range temperature.
        for t in [f32::NAN, f32::INFINITY, -0.1, 2.1, 1e9] {
            let err = create_bot(&pool, "tempbot".to_string(), t, true, false, false)
                .await
                .expect_err("bad temperature must be rejected");
            assert!(
                err.to_string().contains("between 0.0 and 2.0"),
                "{t}: {err}"
            );
        }
        // Boundaries accepted, and the name is stored trimmed.
        let bot = create_bot(&pool, "  edge  ".to_string(), 0.0, true, false, false)
            .await
            .unwrap();
        assert_eq!(bot.name, "edge");
        create_bot(&pool, "edge2".to_string(), 2.0, true, false, false)
            .await
            .unwrap();
    }

    #[sqlx::test]
    async fn test_provider_input_validation(pool: sqlx::PgPool) {
        let err = create_provider(
            &pool,
            "  ".to_string(),
            "https://a.example".to_string(),
            None,
        )
        .await
        .expect_err("empty provider name must be rejected");
        assert!(
            err.to_string().contains("Provider name is required"),
            "{err}"
        );

        for url in ["", "   ", "a.example", "ftp://a.example", "//a.example"] {
            let err = create_provider(&pool, "p".to_string(), url.to_string(), None)
                .await
                .expect_err("bad url must be rejected");
            assert!(err.to_string().contains("URL"), "{url}: {err}");
        }

        let p = create_provider(
            &pool,
            " trimmed ".to_string(),
            " https://a.example ".to_string(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(p.name, "trimmed");
        assert_eq!(p.url, "https://a.example");
    }

    #[sqlx::test]
    async fn test_bot_provider_input_validation(pool: sqlx::PgPool) {
        let bot = create_bot(&pool, "linkbot".to_string(), 0.2, true, false, false)
            .await
            .unwrap();
        let provider = create_provider(
            &pool,
            "linkprovider".to_string(),
            "https://a.example".to_string(),
            None,
        )
        .await
        .unwrap();

        let err = create_bot_provider(&pool, bot.id, provider.id, "  ".to_string(), None, None, 0)
            .await
            .expect_err("empty model must be rejected");
        assert!(err.to_string().contains("Model is required"), "{err}");

        let err = create_bot_provider(
            &pool,
            bot.id,
            provider.id,
            "gpt-4o-mini".to_string(),
            None,
            Some(serde_json::json!([1, 2, 3])),
            0,
        )
        .await
        .expect_err("non-object extra_body must be rejected");
        assert!(err.to_string().contains("JSON object"), "{err}");

        let big = serde_json::json!({ "pad": "x".repeat(9000) });
        let err = create_bot_provider(
            &pool,
            bot.id,
            provider.id,
            "gpt-4o-mini".to_string(),
            None,
            Some(big),
            0,
        )
        .await
        .expect_err("oversized extra_body must be rejected");
        assert!(err.to_string().contains("8192"), "{err}");

        // Valid link still works, and model/reasoning are trimmed.
        let link = create_bot_provider(
            &pool,
            bot.id,
            provider.id,
            " gpt-4o-mini ".to_string(),
            Some(" low ".to_string()),
            Some(serde_json::json!({"top_p": 0.9})),
            3,
        )
        .await
        .unwrap();
        assert_eq!(link.model, "gpt-4o-mini");
        assert_eq!(link.reasoning_effort.as_deref(), Some("low"));
    }

    /// ws F28: the gate now lives in one place, so the thing worth pinning is
    /// that no admin server fn skipped it. Source-level check: every admin
    /// server fn body must contain a call to the shared gate helper.
    ///
    /// The two needles are built with `concat!` so they do not match this
    /// test's own source - see the spec note above.
    #[test]
    fn every_admin_server_fn_calls_require_admin() {
        let src = include_str!("admin.rs");
        let server_fn_needle = concat!("#[", "server(Admin");
        let gate_needle = concat!("require_admin", "(&pool,");
        let server_fns = src.matches(server_fn_needle).count();
        let gates = src.matches(gate_needle).count();
        assert_eq!(
            server_fns, 16,
            "expected 16 admin server fns, found {server_fns} - update this test \
             deliberately if an admin server fn was added or removed"
        );
        assert_eq!(
            server_fns, gates,
            "{server_fns} admin server fns but {gates} gates - an admin server \
             fn is missing its authorization check"
        );
    }

    /// ws F31: pin the exact shape the client redirect matches on. If this
    /// breaks, `AdminPage`'s redirect Effect breaks with it.
    #[test]
    fn admin_required_error_is_a_server_error_variant_with_the_constant() {
        let err = ServerFnError::new(ADMIN_REQUIRED);
        match err {
            ServerFnError::ServerError(msg) => assert_eq!(msg, ADMIN_REQUIRED),
            other => panic!("expected ServerError variant, got {other:?}"),
        }
    }

    /// ws F20: the mask must never return a short key verbatim, and must not
    /// invent a vendor prefix.
    #[test]
    fn mask_api_key_rules() {
        assert_eq!(mask_api_key(""), "(set)");
        assert_eq!(mask_api_key("k"), "(set)");
        assert_eq!(mask_api_key("abcd"), "(set)");
        assert_eq!(mask_api_key("abcde"), "(set)");
        assert_eq!(mask_api_key("abcdefg"), "(set)");
        assert_eq!(mask_api_key("abcdefgh"), "...efgh");
        assert_eq!(mask_api_key("sk-test-secret-key-1234"), "...1234");
        // No fabricated prefix for a non-OpenAI key.
        assert_eq!(mask_api_key("AIzaSyAveryLongGoogleKey9876"), "...9876");
        // Multi-byte safe: last 4 chars, not last 4 bytes.
        assert_eq!(mask_api_key("aaaaaaaaéèçà"), "...éèçà");
        // Nothing short is ever echoed back.
        for k in ["", "k", "ab", "abc", "abcd", "abcde", "ab cdef"] {
            assert_eq!(mask_api_key(k), "(set)", "leaked short key {k:?}");
        }
    }

    /// ws F26: one corrupt row must degrade to a marker, and every other
    /// provider must still list. Before the fix this returned Err and the
    /// whole admin page rendered as a single error.
    #[sqlx::test]
    async fn test_admin_list_providers_degrades_one_undecryptable_row(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let good = crate::crypto::encrypt(&key, b"sk-good-key-1234").unwrap();

        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(Uuid::new_v4())
        .bind("aaa-good")
        .bind("http://localhost:8080")
        .bind(&good)
        .execute(&pool)
        .await
        .unwrap();

        // Not ciphertext at all: 4 bytes, and `crypto::decrypt` returns
        // DecryptionFailed for anything under 12 (crypto.rs:31-33), so this
        // row fails independently of which key is loaded.
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(Uuid::new_v4())
        .bind("bbb-corrupt")
        .bind("http://localhost:8081")
        .bind(vec![0u8, 1, 2, 3])
        .execute(&pool)
        .await
        .unwrap();

        // No key at all: still None, not a marker.
        sqlx::query("INSERT INTO llm_providers (id, name, url, enabled) VALUES ($1, $2, $3, true)")
            .bind(Uuid::new_v4())
            .bind("ccc-nokey")
            .bind("http://localhost:8082")
            .execute(&pool)
            .await
            .unwrap();

        // Ordered by name, so aaa/bbb/ccc.
        let providers = list_providers(&pool)
            .await
            .expect("must not fail wholesale");
        assert_eq!(providers.len(), 3);
        assert_eq!(providers[0].api_key_masked.as_deref(), Some("...1234"));
        assert_eq!(
            providers[1].api_key_masked.as_deref(),
            Some("(undecryptable)")
        );
        assert_eq!(providers[2].api_key_masked, None);
    }

    async fn insert_bot(pool: &sqlx::PgPool, name: &str) -> Uuid {
        create_bot(pool, name.to_string(), 0.2, true, false, false)
            .await
            .unwrap()
            .id
    }

    /// ws F18: reorder writes 0-based orders matching the given sequence.
    #[sqlx::test]
    async fn test_reorder_bots_renumbers_zero_based(pool: sqlx::PgPool) {
        // migration 013 seeds easy/medium/hard at 0/1/2; clear for determinism.
        sqlx::query("DELETE FROM bots")
            .execute(&pool)
            .await
            .unwrap();
        let a = insert_bot(&pool, "aaa").await;
        let b = insert_bot(&pool, "bbb").await;
        let c = insert_bot(&pool, "ccc").await;

        reorder_bots(&pool, vec![c, a, b]).await.unwrap();

        let names: Vec<String> = sqlx::query_scalar("SELECT name FROM bots ORDER BY display_order")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(names, vec!["ccc", "aaa", "bbb"]);
        let orders: Vec<i32> =
            sqlx::query_scalar("SELECT display_order FROM bots ORDER BY display_order")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(orders, vec![0, 1, 2]);
    }

    /// ws F18 + ws F29: an unknown id rolls the whole reorder back.
    #[sqlx::test]
    async fn test_reorder_bots_rejects_unknown_id_atomically(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots")
            .execute(&pool)
            .await
            .unwrap();
        let a = insert_bot(&pool, "aaa").await;
        let b = insert_bot(&pool, "bbb").await;

        let err = reorder_bots(&pool, vec![b, Uuid::new_v4(), a])
            .await
            .expect_err("unknown id must be rejected");
        assert!(err.to_string().contains("please reload"));

        // Nothing moved.
        let names: Vec<String> = sqlx::query_scalar("SELECT name FROM bots ORDER BY display_order")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(names, vec!["aaa", "bbb"]);
    }

    /// ws F18: a duplicated id is rejected before any UPDATE runs, because
    /// Postgres would pick one of the two ordinals nondeterministically.
    #[sqlx::test]
    async fn test_reorder_bots_rejects_duplicate_id(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots")
            .execute(&pool)
            .await
            .unwrap();
        let a = insert_bot(&pool, "aaa").await;
        let b = insert_bot(&pool, "bbb").await;

        let err = reorder_bots(&pool, vec![a, b, a])
            .await
            .expect_err("duplicate id must be rejected");
        assert!(err.to_string().contains("duplicate entry"), "{err}");

        let names: Vec<String> = sqlx::query_scalar("SELECT name FROM bots ORDER BY display_order")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(names, vec!["aaa", "bbb"]);
    }

    /// ws F19: sequential creates never reuse a display_order, and the
    /// advisory lock is the thing that makes that true under concurrency.
    /// (A truly concurrent test would need two pool connections racing; the
    /// deterministic assertion here is the no-duplicate invariant.)
    #[sqlx::test]
    async fn test_create_bot_display_orders_are_unique(pool: sqlx::PgPool) {
        sqlx::query("DELETE FROM bots")
            .execute(&pool)
            .await
            .unwrap();
        for n in ["a", "b", "c", "d"] {
            insert_bot(&pool, n).await;
        }
        let dupes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM (SELECT display_order FROM bots GROUP BY display_order HAVING count(*) > 1) d",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(dupes, 0);
        let orders: Vec<i32> =
            sqlx::query_scalar("SELECT display_order FROM bots ORDER BY display_order")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(orders, vec![0, 1, 2, 3]);
    }

    /// ws F29: updating a row that no longer exists must not report success.
    #[sqlx::test]
    async fn test_update_bot_unknown_id_is_not_found(pool: sqlx::PgPool) {
        let err = update_bot(
            &pool,
            Uuid::new_v4(),
            "ghost".to_string(),
            0.2,
            true,
            false,
            true,
            false,
        )
        .await
        .expect_err("unknown bot id must be rejected");
        assert!(err.to_string().contains("Bot not found"), "{err}");
    }

    #[sqlx::test]
    async fn test_update_provider_unknown_id_is_not_found(pool: sqlx::PgPool) {
        // No-key branch.
        let err = update_provider(
            &pool,
            Uuid::new_v4(),
            "ghost".to_string(),
            "http://localhost:1".to_string(),
            ApiKeyUpdate::Keep,
            true,
        )
        .await
        .expect_err("unknown provider id must be rejected");
        assert!(err.to_string().contains("Provider not found"), "{err}");

        // Key-set branch.
        let err = update_provider(
            &pool,
            Uuid::new_v4(),
            "ghost".to_string(),
            "http://localhost:1".to_string(),
            ApiKeyUpdate::Set("sk-whatever-1234".to_string()),
            true,
        )
        .await
        .expect_err("unknown provider id must be rejected on the key branch too");
        assert!(err.to_string().contains("Provider not found"), "{err}");
    }

    #[sqlx::test]
    async fn test_update_bot_provider_unknown_id_is_not_found(pool: sqlx::PgPool) {
        let err = update_bot_provider(
            &pool,
            Uuid::new_v4(),
            "gpt-4o-mini".to_string(),
            None,
            None,
            0,
            true,
        )
        .await
        .expect_err("unknown link id must be rejected");
        assert!(err.to_string().contains("link not found"), "{err}");
    }

    /// ws F29: deletes stay idempotent on purpose.
    #[sqlx::test]
    async fn test_deletes_are_idempotent(pool: sqlx::PgPool) {
        delete_bot(&pool, Uuid::new_v4()).await.unwrap();
        delete_provider(&pool, Uuid::new_v4()).await.unwrap();
        delete_bot_provider(&pool, Uuid::new_v4()).await.unwrap();
    }

    /// ws F22: the model must come from configuration, never from a literal.
    /// Asserted through the resolution failure path, which runs before any
    /// HTTP request is attempted.
    #[sqlx::test]
    async fn test_provider_requires_a_configured_model(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        let encrypted = crate::crypto::encrypt(&key, b"sk-modeltest-1234").unwrap();
        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("modeltest")
        .bind("http://127.0.0.1:1")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

        let client = reqwest::Client::new();

        // No links at all: actionable error, and no request attempted.
        let err = test_provider(&pool, &client, provider_id, None)
            .await
            .expect_err("no configured model must be an error, not a guess");
        assert!(err.to_string().contains("no configured"), "{err}");

        // A disabled link does not count.
        let bot = create_bot(&pool, "modelbot".to_string(), 0.2, true, false, false)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO bot_providers (bot_id, provider_id, model, priority, enabled) VALUES ($1, $2, $3, 0, false)",
        )
        .bind(bot.id)
        .bind(provider_id)
        .bind("disabled-model")
        .execute(&pool)
        .await
        .unwrap();
        let err = test_provider(&pool, &client, provider_id, None)
            .await
            .expect_err("a disabled link must not supply the model");
        assert!(err.to_string().contains("no configured"), "{err}");

        // An empty explicit model is a validation error, not a fallback.
        let err = test_provider(&pool, &client, provider_id, Some("  ".to_string()))
            .await
            .expect_err("blank explicit model must be rejected");
        assert!(err.to_string().contains("Model is required"), "{err}");

        // With an enabled link, resolution succeeds and we get past the model
        // step - the request to 127.0.0.1:1 then fails, which is the proof
        // that resolution no longer short-circuits.
        sqlx::query(
            "INSERT INTO bot_providers (bot_id, provider_id, model, priority, enabled) VALUES ($1, $2, $3, 1, true)",
        )
        .bind(bot.id)
        .bind(provider_id)
        .bind("configured-model")
        .execute(&pool)
        .await
        .unwrap();
        let err = test_provider(&pool, &client, provider_id, None)
            .await
            .expect_err("connection to port 1 must fail");
        assert!(
            !err.to_string().contains("no configured"),
            "model resolution should have succeeded: {err}"
        );
    }

    /// Spawn a throwaway HTTP server on an ephemeral port that answers
    /// POST /v1/chat/completions with a fixed status/body/headers, and return
    /// its base URL. Same in-process pattern as `spawn_mock_game_service` in
    /// `tests/ssr_pages.rs:104-128`; never calls a real LLM (docs/CODING.md).
    async fn spawn_upstream(
        status: u16,
        body: Vec<u8>,
        extra_headers: Vec<(&'static str, &'static str)>,
    ) -> String {
        use axum::routing::post;
        let app = axum::Router::new().route(
            "/v1/chat/completions",
            post(move || {
                let body = body.clone();
                let extra_headers = extra_headers.clone();
                async move {
                    let mut headers = axum::http::HeaderMap::new();
                    for (k, v) in extra_headers {
                        headers.insert(
                            axum::http::HeaderName::from_static(k),
                            axum::http::HeaderValue::from_static(v),
                        );
                    }
                    (
                        axum::http::StatusCode::from_u16(status).unwrap(),
                        headers,
                        body,
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn provider_with_link(pool: &sqlx::PgPool, url: &str) -> Uuid {
        let key = test_encryption_key();
        let encrypted = crate::crypto::encrypt(&key, b"sk-capped-1234").unwrap();
        let provider_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO llm_providers (id, name, url, api_key_encrypted, enabled) VALUES ($1, $2, $3, $4, true)",
        )
        .bind(provider_id)
        .bind("capped")
        .bind(url)
        .bind(&encrypted)
        .execute(pool)
        .await
        .unwrap();
        let bot = create_bot(pool, "cappedbot".to_string(), 0.2, true, false, false)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO bot_providers (bot_id, provider_id, model, priority, enabled) VALUES ($1, $2, 'm', 0, true)",
        )
        .bind(bot.id)
        .bind(provider_id)
        .execute(pool)
        .await
        .unwrap();
        provider_id
    }

    /// ws F23: an oversized upstream body is truncated, not buffered whole.
    #[sqlx::test]
    async fn test_provider_truncates_huge_error_body(pool: sqlx::PgPool) {
        let url = spawn_upstream(500, vec![b'x'; 1_000_000], vec![]).await;
        let provider_id = provider_with_link(&pool, &url).await;
        let out = test_provider(&pool, &reqwest::Client::new(), provider_id, None)
            .await
            .unwrap();
        assert!(out.starts_with("HTTP 500"), "{out}");
        assert!(out.contains("<truncated at 8192 bytes>"), "not truncated");
        assert!(
            out.len() < 9_000,
            "body was not capped: {} bytes",
            out.len()
        );
    }

    /// ws F23: only allowlisted headers reach the client.
    #[sqlx::test]
    async fn test_bot_provider_filters_headers_and_caps_body(pool: sqlx::PgPool) {
        let url = spawn_upstream(
            200,
            vec![b'y'; 1_000_000],
            vec![
                ("content-type", "application/json"),
                ("set-cookie", "session=leaked"),
                ("x-upstream-secret", "nope"),
            ],
        )
        .await;
        let provider_id = provider_with_link(&pool, &url).await;
        let link_id: Uuid =
            sqlx::query_scalar("SELECT id FROM bot_providers WHERE provider_id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let resp = test_bot_provider(&pool, &reqwest::Client::new(), link_id, "hi")
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("<truncated at 8192 bytes>"));
        assert!(resp.body.len() < 9_000, "body not capped");
        let names: Vec<&str> = resp.headers.iter().map(|(k, _)| k.as_str()).collect();
        assert!(names.contains(&"content-type"), "{names:?}");
        assert!(
            !names.contains(&"set-cookie"),
            "leaked set-cookie: {names:?}"
        );
        assert!(
            !names.contains(&"x-upstream-secret"),
            "leaked unknown header: {names:?}"
        );
    }

    /// A small body is returned intact with no truncation marker.
    #[sqlx::test]
    async fn test_bot_provider_small_body_intact(pool: sqlx::PgPool) {
        let url = spawn_upstream(200, br#"{"ok":true}"#.to_vec(), vec![]).await;
        let provider_id = provider_with_link(&pool, &url).await;
        let link_id: Uuid =
            sqlx::query_scalar("SELECT id FROM bot_providers WHERE provider_id = $1")
                .bind(provider_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let resp = test_bot_provider(&pool, &reqwest::Client::new(), link_id, "hi")
            .await
            .unwrap();
        assert_eq!(resp.body, r#"{"ok":true}"#);
        assert!(!resp.body.contains("truncated"));
    }
}
