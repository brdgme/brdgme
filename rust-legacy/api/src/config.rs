use failure::{Error, ResultExt};

use std::env;

lazy_static! {
  pub static ref CONFIG: Config = from_env().unwrap();
}

pub enum Mail {
    File,
    Smtp {
        addr: String,
        user: String,
        pass: String,
    },
}

impl Mail {
    fn smtp_from_env() -> Result<Self, Error> {
        Ok(Mail::Smtp {
            addr: env::var("SMTP_ADDR").context("SMTP_ADDR must be set")?,
            user: env::var("SMTP_USER").context("SMTP_USER must be set")?,
            pass: env::var("SMTP_PASS").context("SMTP_PASS must be set")?,
        })
    }

    pub fn from_env() -> Self {
        Self::smtp_from_env().unwrap_or(Mail::File)
    }
}

pub struct Config {
    pub database_url: String,
    pub database_url_r: Option<String>,
    pub redis_url: String,
    pub mail: Mail,
    pub mail_from: String,
}

fn from_env() -> Result<Config, Error> {
    Ok(Config {
        database_url: env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
        database_url_r: env::var("DATABASE_URL_R").ok(),
        redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string()),
        mail: Mail::from_env(),
        mail_from: env::var("MAIL_FROM").unwrap_or_else(|_| "play@brdg.me".to_string()),
    })
}
