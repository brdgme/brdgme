use std::collections::HashMap;

pub fn parse_reply_commands(text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') {
            continue;
        }
        if trimmed.starts_with("On ") && trimmed.ends_with("wrote:") {
            break;
        }
        let t = line.trim();
        if t == "-- " || t == "--" {
            break;
        }
        if t.is_empty() {
            continue;
        }
        commands.push(t.to_string());
    }
    commands
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundRoute {
    Game(String),
    Invite(String),
    Settings(String),
}

pub fn parse_reply_address(addr: &str) -> Option<InboundRoute> {
    let local = addr.split('@').next().unwrap_or(addr);
    let (tok, route) = if let Some(tok) = local.strip_prefix("g-") {
        (tok, InboundRoute::Game(tok.to_string()))
    } else if let Some(tok) = local.strip_prefix("i-") {
        (tok, InboundRoute::Invite(tok.to_string()))
    } else {
        let tok = local.strip_prefix("s-")?;
        (tok, InboundRoute::Settings(tok.to_string()))
    };
    if tok.is_empty() {
        return None;
    }
    Some(route)
}

pub fn extract_plain_text(raw: &str) -> Option<String> {
    let msg = mail_parser::MessageParser::default().parse(raw)?;
    msg.body_text(0).map(|s| s.to_string())
}

#[async_trait::async_trait]
pub trait InboundEmailSource: Send + Sync {
    async fn fetch_raw_email(&self, email_id: &str) -> anyhow::Result<String>;
}

pub struct ResendInbound {
    pub api_key: String,
    pub http: reqwest::Client,
}

#[derive(serde::Deserialize)]
struct ResendEmailResponse {
    raw: ResendRaw,
}

#[derive(serde::Deserialize)]
struct ResendRaw {
    download_url: String,
}

#[async_trait::async_trait]
impl InboundEmailSource for ResendInbound {
    async fn fetch_raw_email(&self, email_id: &str) -> anyhow::Result<String> {
        let url = format!("https://api.resend.com/emails/receiving/{email_id}");
        let resp: ResendEmailResponse = self
            .http
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let raw = self
            .http
            .get(&resp.raw.download_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(raw)
    }
}

pub struct StaticInbound(pub HashMap<String, String>);

#[async_trait::async_trait]
impl InboundEmailSource for StaticInbound {
    async fn fetch_raw_email(&self, email_id: &str) -> anyhow::Result<String> {
        self.0
            .get(email_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("email not found: {email_id}"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("invalid secret")]
    InvalidSecret,
    #[error("missing header: {0}")]
    MissingHeader(&'static str),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("timestamp too old")]
    TimestampTooOld,
    #[error("timestamp in future")]
    FutureTimestamp,
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("verification failed: {0}")]
    Other(String),
}

pub fn verify_webhook(
    secret: &str,
    msg_id: &str,
    signature: &str,
    timestamp: &str,
    raw_body: &[u8],
) -> Result<(), VerifyError> {
    use axum::http::HeaderValue;

    let webhook = svix::webhooks::Webhook::new(secret).map_err(|_| VerifyError::InvalidSecret)?;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("svix-id", HeaderValue::from_str(msg_id).unwrap());
    headers.insert("svix-timestamp", HeaderValue::from_str(timestamp).unwrap());
    headers.insert("svix-signature", HeaderValue::from_str(signature).unwrap());
    webhook.verify(raw_body, &headers).map_err(|e| match e {
        svix::webhooks::WebhookError::InvalidSecret(_)
        | svix::webhooks::WebhookError::EmptySecret => VerifyError::InvalidSecret,
        svix::webhooks::WebhookError::MissingHeader(_) => VerifyError::MissingHeader("svix"),
        svix::webhooks::WebhookError::InvalidSignature => VerifyError::InvalidSignature,
        svix::webhooks::WebhookError::TimestampTooOldError => VerifyError::TimestampTooOld,
        svix::webhooks::WebhookError::FutureTimestampError => VerifyError::FutureTimestamp,
        svix::webhooks::WebhookError::InvalidTimestamp => VerifyError::InvalidTimestamp,
        other => VerifyError::Other(other.to_string()),
    })
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn parse_reply_commands_clean_single() {
        assert_eq!(parse_reply_commands("play e4"), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_strips_quoted_lines() {
        let input = "play d4\n> previous move was e4\n> another quote";
        assert_eq!(parse_reply_commands(input), vec!["play d4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_on_wrote() {
        let input = "play e4\nOn Mon, Jul 20, 2026 at 10:00 AM Alice wrote:\n> play d4";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_signature() {
        let input = "play e4\n-- \nSent from my phone";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);

        let input2 = "play e4\n--\nSent from my phone";
        assert_eq!(parse_reply_commands(input2), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_multiple_in_order() {
        let input = "play e4\nplay d5\nresign";
        assert_eq!(
            parse_reply_commands(input),
            vec!["play e4", "play d5", "resign"]
        );
    }

    #[test]
    fn parse_reply_commands_drops_blank_lines() {
        let input = "play e4\n\n   \nplay d5";
        assert_eq!(parse_reply_commands(input), vec!["play e4", "play d5"]);
    }

    #[test]
    fn parse_reply_commands_keeps_arguments() {
        assert_eq!(parse_reply_commands("play e4 to e5"), vec!["play e4 to e5"]);
    }

    #[test]
    fn parse_reply_commands_empty_input() {
        assert_eq!(parse_reply_commands(""), Vec::<String>::new());
    }

    #[test]
    fn parse_reply_address_game() {
        assert_eq!(
            parse_reply_address("g-abc@play.brdg.me"),
            Some(InboundRoute::Game("abc".to_string()))
        );
    }

    #[test]
    fn parse_reply_address_invite() {
        assert_eq!(
            parse_reply_address("i-xyz@example.com"),
            Some(InboundRoute::Invite("xyz".to_string()))
        );
    }

    #[test]
    fn parse_reply_address_settings() {
        assert_eq!(
            parse_reply_address("s-tok@anything"),
            Some(InboundRoute::Settings("tok".to_string()))
        );
    }

    #[test]
    fn parse_reply_address_no_prefix() {
        assert_eq!(parse_reply_address("hello@play.brdg.me"), None);
    }

    #[test]
    fn parse_reply_address_bare_no_at() {
        assert_eq!(parse_reply_address("hello"), None);
    }

    #[test]
    fn parse_reply_address_empty_token() {
        assert_eq!(parse_reply_address("g-@x.com"), None);
    }

    #[test]
    fn extract_plain_text_multipart() {
        let raw = "MIME-Version: 1.0\r\n\
Content-Type: multipart/alternative; boundary=\"BOUNDARY\"\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hello plain world\r\n\
--BOUNDARY\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>Hello html world</p>\r\n\
--BOUNDARY--\r\n";
        assert_eq!(
            extract_plain_text(raw),
            Some("Hello plain world".to_string())
        );
    }

    #[test]
    fn extract_plain_text_single_part() {
        let raw = "MIME-Version: 1.0\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Just a plain body";
        assert_eq!(
            extract_plain_text(raw),
            Some("Just a plain body".to_string())
        );
    }

    #[test]
    fn verify_webhook_valid() {
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let body = b"{\"test\": true}";
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let wh = svix::webhooks::Webhook::new(secret).unwrap();
        let sig = wh.sign("msg_123", ts, body).unwrap();
        assert!(verify_webhook(secret, "msg_123", &sig, &ts.to_string(), body).is_ok());
    }

    #[test]
    fn verify_webhook_tampered_body() {
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let body = b"{\"test\": true}";
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let wh = svix::webhooks::Webhook::new(secret).unwrap();
        let sig = wh.sign("msg_123", ts, body).unwrap();
        let tampered = b"{\"test\": false}";
        assert!(verify_webhook(secret, "msg_123", &sig, &ts.to_string(), tampered).is_err());
    }

    #[test]
    fn verify_webhook_wrong_secret() {
        let body = b"{\"test\": true}";
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let wh = svix::webhooks::Webhook::new("whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw").unwrap();
        let sig = wh.sign("msg_123", ts, body).unwrap();
        assert!(
            verify_webhook(
                "whsec_C2FVsBQIhrscChlQIMV+b5sSYspob7oD",
                "msg_123",
                &sig,
                &ts.to_string(),
                body
            )
            .is_err()
        );
    }
}
