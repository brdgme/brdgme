use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;

use std::env;

lazy_static! {
    pub static ref CONFIG: Config = from_env().unwrap();
}

pub enum Mail {
    Log,
    Smtp {
        addr: String,
        user: String,
        pass: String,
    },
}

impl Mail {
    fn smtp_from_env() -> Result<Self> {
        Ok(Mail::Smtp {
            addr: env::var("SMTP_ADDR").context("SMTP_ADDR must be set")?,
            user: env::var("SMTP_USER").context("SMTP_USER must be set")?,
            pass: env::var("SMTP_PASS").context("SMTP_PASS must be set")?,
        })
    }

    pub fn from_env() -> Self {
        Self::smtp_from_env().unwrap_or(Mail::Log)
    }
}

#[derive(Debug)]
pub struct PostgresConfig {
    pub host: String,
    pub port: Option<String>,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl PostgresConfig {
    pub fn from_env(suffix: &str) -> Result<Option<Self>> {
        Ok(Some(Self {
            host: env::var(&format!("POSTGRES_HOST{}", suffix))
                .unwrap_or_else(|_| "postgres".to_string()),
            database: env::var(&format!("POSTGRES_DB{}", suffix))
                .unwrap_or_else(|_| "brdgme".to_string()),
            port: env::var(&format!("POSTGRES_TCP_PORT{}", suffix)).ok(),
            user: match env::var(&format!("POSTGRES_USER{}", suffix)) {
                Ok(u) => u,
                Err(env::VarError::NotPresent) => return Ok(None),
                Err(e) => return Err(e.into()),
            },
            password: match env::var(&format!("POSTGRES_PASSWORD{}", suffix)) {
                Ok(p) => p,
                Err(env::VarError::NotPresent) => return Ok(None),
                Err(e) => return Err(e.into()),
            },
        }))
    }

    pub fn url(&self) -> String {
        let port = self
            .port
            .as_ref()
            .map(|p| format!(":{}", p))
            .unwrap_or_else(|| "".to_string());
        format!(
            "postgres://{}:{}@{}{}/{}",
            self.user, self.password, self.host, port, self.database
        )
    }
}

pub struct Config {
    pub postgres: PostgresConfig,
    pub postgres_r: Option<PostgresConfig>,
    pub redis_url: String,
    pub mail: Mail,
    pub mail_from: String,
}

fn from_env() -> Result<Config> {
    Ok(Config {
        postgres: PostgresConfig::from_env("")?
            .ok_or_else(|| anyhow!("expected a Postgres config"))?,
        postgres_r: PostgresConfig::from_env("_R")?,
        redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis".to_string()),
        mail: Mail::from_env(),
        mail_from: env::var("MAIL_FROM").unwrap_or_else(|_| "play@brdg.me".to_string()),
    })
}
