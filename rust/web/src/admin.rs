#[cfg(feature = "ssr")]
use crate::error::internal;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRow {
    pub id: Uuid,
    pub name: String,
    pub display_order: i32,
    pub enabled: bool,
    pub include_basic_strategy: bool,
    pub include_advanced_strategy: bool,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRow {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub api_key_masked: Option<String>,
    pub enabled: bool,
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
    let rows: Vec<(Uuid, String, i32, bool, bool, bool, f32)> = sqlx::query_as(
        "SELECT id, name, display_order, enabled, include_basic_strategy, include_advanced_strategy, temperature FROM bots ORDER BY display_order",
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
            )| {
                BotRow {
                    id,
                    name,
                    display_order,
                    enabled,
                    include_basic_strategy,
                    include_advanced_strategy,
                    temperature,
                }
            },
        )
        .collect())
}

#[cfg(feature = "ssr")]
pub async fn create_bot(
    pool: &sqlx::PgPool,
    name: String,
    temperature: f32,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
) -> Result<BotRow, ServerFnError> {
    let row: (Uuid, String, i32, bool, bool, bool, f32) = sqlx::query_as(
        "INSERT INTO bots (name, display_order, temperature, include_basic_strategy, include_advanced_strategy) \
         VALUES ($1, COALESCE((SELECT MAX(display_order) + 1 FROM bots), 0), $2, $3, $4) \
         RETURNING id, name, display_order, enabled, include_basic_strategy, include_advanced_strategy, temperature",
    )
    .bind(&name)
    .bind(temperature)
    .bind(include_basic_strategy)
    .bind(include_advanced_strategy)
    .fetch_one(pool)
    .await
    .map_err(internal("admin_create_bot: insert"))?;

    Ok(BotRow {
        id: row.0,
        name: row.1,
        display_order: row.2,
        enabled: row.3,
        include_basic_strategy: row.4,
        include_advanced_strategy: row.5,
        temperature: row.6,
    })
}

#[cfg(feature = "ssr")]
pub async fn update_bot(
    pool: &sqlx::PgPool,
    id: Uuid,
    name: String,
    temperature: f32,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
    enabled: bool,
) -> Result<(), ServerFnError> {
    sqlx::query(
        "UPDATE bots SET name = $2, temperature = $3, include_basic_strategy = $4, include_advanced_strategy = $5, enabled = $6, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(&name)
    .bind(temperature)
    .bind(include_basic_strategy)
    .bind(include_advanced_strategy)
    .bind(enabled)
    .execute(pool)
    .await
    .map_err(internal("admin_update_bot: update"))?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn reorder_bots(
    pool: &sqlx::PgPool,
    ordered_ids: Vec<Uuid>,
) -> Result<(), ServerFnError> {
    for (i, id) in ordered_ids.iter().enumerate() {
        sqlx::query("UPDATE bots SET display_order = $2, updated_at = now() WHERE id = $1")
            .bind(*id)
            .bind(i as i32)
            .execute(pool)
            .await
            .map_err(internal("admin_reorder_bots: update"))?;
    }
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn delete_bot(pool: &sqlx::PgPool, id: Uuid) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM bots WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(internal("admin_delete_bot: delete"))?;
    Ok(())
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
        let api_key_masked = match api_key_encrypted {
            Some(encrypted) => {
                let decrypted = crate::crypto::decrypt(&key, &encrypted)
                    .map_err(internal("admin_list_providers: decrypt"))?;
                let plaintext =
                    String::from_utf8(decrypted).map_err(internal("admin_list_providers: utf8"))?;
                let last4: String = plaintext
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                Some(format!("sk-...{last4}"))
            }
            None => None,
        };
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

    let api_key_masked = api_key.map(|k| {
        let last4: String = k
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("sk-...{last4}")
    });

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
    api_key: Option<String>,
    enabled: bool,
) -> Result<(), ServerFnError> {
    match api_key {
        Some(key_str) => {
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
            .map_err(internal("admin_update_provider: update"))?;
        }
        None => {
            sqlx::query(
                "UPDATE llm_providers SET name = $2, url = $3, enabled = $4, updated_at = now() WHERE id = $1",
            )
            .bind(id)
            .bind(&name)
            .bind(&url)
            .bind(enabled)
            .execute(pool)
            .await
            .map_err(internal("admin_update_provider: update"))?;
        }
    }
    Ok(())
}

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
    sqlx::query(
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
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn delete_bot_provider(pool: &sqlx::PgPool, id: Uuid) -> Result<(), ServerFnError> {
    sqlx::query("DELETE FROM bot_providers WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(internal("admin_delete_bot_provider: delete"))?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn test_provider(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    provider_id: Uuid,
) -> Result<String, ServerFnError> {
    let row: Option<(String, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT url, api_key_encrypted FROM llm_providers WHERE id = $1")
            .bind(provider_id)
            .fetch_optional(pool)
            .await
            .map_err(internal("admin_test_provider: query"))?;

    let (url, api_key_encrypted) = row.ok_or_else(|| ServerFnError::new("Provider not found"))?;

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
        "model": "gpt-4o-mini",
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

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "unable to read body".to_string());
        return Ok(format!("HTTP {status}: {text}"));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(internal("admin_test_provider: parse response"))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No content in response");
    Ok(content.to_string())
}

#[server(AdminListBots, "/api")]
pub async fn admin_list_bots() -> Result<Vec<BotRow>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_list_bots: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    list_bots(&pool).await
}

#[server(AdminCreateBot, "/api")]
pub async fn admin_create_bot(
    name: String,
    temperature: f32,
    include_basic_strategy: bool,
    include_advanced_strategy: bool,
) -> Result<BotRow, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_create_bot: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    create_bot(
        &pool,
        name,
        temperature,
        include_basic_strategy,
        include_advanced_strategy,
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
) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_update_bot: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    update_bot(
        &pool,
        id,
        name,
        temperature,
        include_basic_strategy,
        include_advanced_strategy,
        enabled,
    )
    .await
}

#[server(AdminReorderBots, "/api")]
pub async fn admin_reorder_bots(ordered_ids: Vec<Uuid>) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_reorder_bots: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    reorder_bots(&pool, ordered_ids).await
}

#[server(AdminDeleteBot, "/api")]
pub async fn admin_delete_bot(id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_delete_bot: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    delete_bot(&pool, id).await
}

#[server(AdminListProviders, "/api")]
pub async fn admin_list_providers() -> Result<Vec<ProviderRow>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_list_providers: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    list_providers(&pool).await
}

#[server(AdminCreateProvider, "/api")]
pub async fn admin_create_provider(
    name: String,
    url: String,
    api_key: Option<String>,
) -> Result<ProviderRow, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_create_provider: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    create_provider(&pool, name, url, api_key).await
}

#[server(AdminUpdateProvider, "/api")]
pub async fn admin_update_provider(
    id: Uuid,
    name: String,
    url: String,
    api_key: Option<String>,
    enabled: bool,
) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_update_provider: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    update_provider(&pool, id, name, url, api_key, enabled).await
}

#[server(AdminDeleteProvider, "/api")]
pub async fn admin_delete_provider(id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_delete_provider: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    delete_provider(&pool, id).await
}

#[server(AdminListBotProviders, "/api")]
pub async fn admin_list_bot_providers() -> Result<Vec<BotProviderRow>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_list_bot_providers: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

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
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_create_bot_provider: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

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
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_update_bot_provider: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

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
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_delete_bot_provider: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    delete_bot_provider(&pool, id).await
}

#[server(AdminTestProvider, "/api")]
pub async fn admin_test_provider(provider_id: Uuid) -> Result<String, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("admin_test_provider: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    test_provider(&pool, &http_client, provider_id).await
}

#[component]
pub fn AdminPage() -> impl IntoView {
    use crate::components::MainLayout;
    use leptos_router::{NavigateOptions, hooks::use_navigate};

    let current_user =
        expect_context::<LocalResource<Result<Option<crate::auth::AuthUser>, ServerFnError>>>();

    let navigate = use_navigate();
    let navigate2 = navigate.clone();
    Effect::new(move |_| {
        if matches!(current_user.get(), Some(Ok(None))) {
            navigate("/login", NavigateOptions::default());
        }
    });

    let version = RwSignal::new(0u32);
    let bots: LocalResource<Result<Vec<BotRow>, ServerFnError>> = LocalResource::new(move || {
        version.track();
        admin_list_bots()
    });

    Effect::new(move |_| {
        if let Some(Err(e)) = bots.get() {
            let msg = e.to_string();
            if msg.contains("Admin access required") {
                navigate2("/", NavigateOptions::default());
            }
        }
    });

    view! {
        <MainLayout>
            <div class="admin content-page">
                <h1>"Admin"</h1>
                <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                    {move || {
                        bots.get().map(|res| match res {
                            Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                            Ok(bot_list) => view! {
                                <BotsSection bots=bot_list version=version/>
                                <h2>"Providers"</h2>
                                <p>"Coming soon"</p>
                                <h2>"Bot-Provider Links"</h2>
                                <p>"Coming soon"</p>
                            }.into_any(),
                        })
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

    let create_action = Action::new(
        |(name, temperature, basic, advanced): &(String, f32, bool, bool)| {
            let name = name.clone();
            let temperature = *temperature;
            let basic = *basic;
            let advanced = *advanced;
            async move { admin_create_bot(name, temperature, basic, advanced).await }
        },
    );

    let update_action = Action::new(
        |(id, name, temperature, basic, advanced, enabled): &(
            Uuid,
            String,
            f32,
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
            async move { admin_update_bot(id, name, temperature, basic, advanced, enabled).await }
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
        if create_action.value().get().is_some() && !create_action.pending().get() {
            match create_action.value().get().unwrap() {
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
        if update_action.value().get().is_some() && !update_action.pending().get() {
            match update_action.value().get().unwrap() {
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
        if delete_action.value().get().is_some() && !delete_action.pending().get() {
            match delete_action.value().get().unwrap() {
                Ok(_) => {
                    error.set(None);
                    version.update(|v| *v += 1);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        }
    });

    Effect::new(move |_| {
        if reorder_action.value().get().is_some() && !reorder_action.pending().get() {
            match reorder_action.value().get().unwrap() {
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
fn BotCreateForm(
    create_action: Action<(String, f32, bool, bool), Result<BotRow, ServerFnError>>,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
    let temp_input = NodeRef::<html::Input>::new();
    let basic_input = NodeRef::<html::Input>::new();
    let advanced_input = NodeRef::<html::Input>::new();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name = name_input.get().map(|el| el.value()).unwrap_or_default();
        let temperature = temp_input
            .get()
            .and_then(|el| el.value().parse::<f32>().ok())
            .unwrap_or(0.2);
        let basic = basic_input.get().map(|el| el.checked()).unwrap_or(true);
        let advanced = advanced_input.get().map(|el| el.checked()).unwrap_or(false);
        create_action.dispatch((name, temperature, basic, advanced));
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
    update_action: Action<(Uuid, String, f32, bool, bool, bool), Result<(), ServerFnError>>,
) -> impl IntoView {
    use crate::components::FormField;
    use leptos::html;

    let name_input = NodeRef::<html::Input>::new();
    let temp_input = NodeRef::<html::Input>::new();
    let basic_input = NodeRef::<html::Input>::new();
    let advanced_input = NodeRef::<html::Input>::new();
    let enabled_input = NodeRef::<html::Input>::new();

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
        update_action.dispatch((bot_id, name, temperature, basic, advanced, enabled));
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

    fn test_encryption_key() -> [u8; 32] {
        [0xAB; 32]
    }

    #[sqlx::test]
    async fn test_admin_list_bots_rejects_non_admin(pool: sqlx::PgPool) {
        sqlx::query("INSERT INTO users (id, name, email, is_admin) VALUES ($1, $2, $3, false)")
            .bind(Uuid::new_v4())
            .bind("testuser")
            .bind("test@example.com")
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

        unsafe {
            std::env::set_var("BOT_ENCRYPTION_KEY", hex::encode(key));
        }

        let providers = list_providers(&pool).await.unwrap();

        unsafe {
            std::env::remove_var("BOT_ENCRYPTION_KEY");
        }

        assert_eq!(providers.len(), 1);
        let masked = providers[0].api_key_masked.as_ref().unwrap();
        assert_eq!(masked, "sk-...1234");
        assert!(!masked.contains(api_key));
    }

    #[sqlx::test]
    async fn test_admin_create_provider_encrypts_key(pool: sqlx::PgPool) {
        let key = test_encryption_key();
        unsafe {
            std::env::set_var("BOT_ENCRYPTION_KEY", hex::encode(key));
        }

        let api_key = "sk-another-secret-key-5678";
        let provider = create_provider(
            &pool,
            "enc-test".to_string(),
            "http://localhost:9090".to_string(),
            Some(api_key.to_string()),
        )
        .await
        .unwrap();

        unsafe {
            std::env::remove_var("BOT_ENCRYPTION_KEY");
        }

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

        unsafe {
            std::env::set_var("BOT_ENCRYPTION_KEY", hex::encode(key));
        }

        update_provider(
            &pool,
            provider_id,
            "preserve-test-renamed".to_string(),
            "http://localhost:8081".to_string(),
            None,
            true,
        )
        .await
        .unwrap();

        unsafe {
            std::env::remove_var("BOT_ENCRYPTION_KEY");
        }

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

        unsafe {
            std::env::set_var("BOT_ENCRYPTION_KEY", hex::encode(key));
        }

        let new_key = "sk-new-key-2222";
        update_provider(
            &pool,
            provider_id,
            "replace-test".to_string(),
            "http://localhost:8080".to_string(),
            Some(new_key.to_string()),
            true,
        )
        .await
        .unwrap();

        unsafe {
            std::env::remove_var("BOT_ENCRYPTION_KEY");
        }

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
}
