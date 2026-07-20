use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::{PgPool, Row};

use crate::crypto;

#[derive(Debug, Clone)]
pub struct BotConfig {
    pub name: String,
    pub include_basic_strategy: bool,
    pub include_advanced_strategy: bool,
    pub temperature: f32,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub extra_body: Option<Value>,
    pub priority: i32,
}

pub async fn load_bot_config(pool: &PgPool, bot_name: &str) -> Result<Option<BotConfig>> {
    let row = sqlx::query(
        "SELECT name, include_basic_strategy, include_advanced_strategy, temperature \
         FROM bots WHERE name = $1 AND enabled = true",
    )
    .bind(bot_name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| BotConfig {
        name: row.try_get("name").unwrap_or_default(),
        include_basic_strategy: row.try_get("include_basic_strategy").unwrap_or(true),
        include_advanced_strategy: row.try_get("include_advanced_strategy").unwrap_or(false),
        temperature: row.try_get("temperature").unwrap_or(0.2),
    }))
}

pub async fn bots_table_empty(pool: &PgPool) -> Result<bool> {
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM bots) AS has_bots")
        .fetch_one(pool)
        .await?;
    let has_bots: bool = row.try_get("has_bots").unwrap_or(true);
    Ok(!has_bots)
}

pub async fn load_providers(
    pool: &PgPool,
    bot_name: &str,
    encryption_key: &[u8; 32],
) -> Result<Vec<ProviderConfig>> {
    let rows = sqlx::query(
        "SELECT lp.url, lp.api_key_encrypted, bp.model, bp.reasoning_effort, bp.extra_body, bp.priority \
         FROM bot_providers bp \
         JOIN bots b ON b.id = bp.bot_id \
         JOIN llm_providers lp ON lp.id = bp.provider_id \
         WHERE b.name = $1 AND b.enabled = true AND bp.enabled = true AND lp.enabled = true \
         ORDER BY bp.priority ASC, bp.created_at ASC",
    )
    .bind(bot_name)
    .fetch_all(pool)
    .await?;

    let mut providers = Vec::with_capacity(rows.len());
    for row in rows {
        let url: String = row.try_get("url").context("provider url")?;
        let model: String = row.try_get("model").context("provider model")?;
        let reasoning_effort: Option<String> = row.try_get("reasoning_effort").unwrap_or(None);
        let extra_body: Option<Value> = row.try_get("extra_body").unwrap_or(None);
        let priority: i32 = row.try_get("priority").unwrap_or(0);
        let api_key_encrypted: Option<Vec<u8>> = row.try_get("api_key_encrypted").unwrap_or(None);

        let api_key = match api_key_encrypted {
            Some(bytes) => {
                let plaintext = crypto::decrypt(encryption_key, &bytes)
                    .map_err(|e| anyhow::anyhow!("failed to decrypt api key: {}", e))?;
                Some(String::from_utf8(plaintext).context("api key is not valid utf-8")?)
            }
            None => None,
        };

        providers.push(ProviderConfig {
            url,
            api_key,
            model,
            reasoning_effort,
            extra_body,
            priority,
        });
    }

    Ok(providers)
}

pub fn env_fallback_provider() -> Option<ProviderConfig> {
    let url = std::env::var("LLM_URL").ok()?;
    let model = std::env::var("BOT_MODEL").ok()?;
    let api_key = std::env::var("LLM_API_KEY").ok();
    let reasoning_effort = std::env::var("REASONING_EFFORT").ok();
    let extra_body = std::env::var("LLM_EXTRA_BODY")
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok());

    Some(ProviderConfig {
        url,
        api_key,
        model,
        reasoning_effort,
        extra_body,
        priority: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const VARS: &[&str] = &[
        "LLM_URL",
        "BOT_MODEL",
        "LLM_API_KEY",
        "REASONING_EFFORT",
        "LLM_EXTRA_BODY",
    ];

    fn clear_env() {
        for var in VARS {
            unsafe { std::env::remove_var(var) };
        }
    }

    #[test]
    fn env_fallback_provider_returns_none_without_required_vars() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();
        assert!(env_fallback_provider().is_none());
    }

    #[test]
    fn env_fallback_provider_builds_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();
        unsafe {
            std::env::set_var("LLM_URL", "http://localhost:8080");
            std::env::set_var("BOT_MODEL", "test-model");
            std::env::set_var("LLM_API_KEY", "secret");
            std::env::set_var("REASONING_EFFORT", "low");
            std::env::set_var("LLM_EXTRA_BODY", r#"{"temperature":0.5}"#);
        }

        let provider = env_fallback_provider().unwrap();
        assert_eq!(provider.url, "http://localhost:8080");
        assert_eq!(provider.model, "test-model");
        assert_eq!(provider.api_key.as_deref(), Some("secret"));
        assert_eq!(provider.reasoning_effort.as_deref(), Some("low"));
        assert_eq!(
            provider
                .extra_body
                .as_ref()
                .and_then(|v| v.get("temperature")),
            Some(&serde_json::json!(0.5))
        );
        assert_eq!(provider.priority, 0);

        clear_env();
    }

    #[test]
    fn env_fallback_provider_invalid_extra_body_is_none() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env();
        unsafe {
            std::env::set_var("LLM_URL", "http://localhost:8080");
            std::env::set_var("BOT_MODEL", "test-model");
            std::env::set_var("LLM_EXTRA_BODY", "not json");
        }

        let provider = env_fallback_provider().unwrap();
        assert!(provider.extra_body.is_none());

        clear_env();
    }
}
