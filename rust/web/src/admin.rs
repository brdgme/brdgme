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
    let rows: Vec<(Uuid, String, String, Option<Vec<u8>>, bool)> = sqlx::query_as(
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
    let rows: Vec<(Uuid, Uuid, Uuid, String, Option<String>, Option<serde_json::Value>, i32, bool, String, String)> = sqlx::query_as(
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
    let row: (Uuid, Uuid, Uuid, String, Option<String>, Option<serde_json::Value>, i32, bool, String, String) = sqlx::query_as(
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
