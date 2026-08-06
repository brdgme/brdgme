//! #34 dev-side game import CLI (spec D5).
//!
//! Usage: cargo run -p web --features ssr --bin import-game -- bundle.json
//!
//! Reads DATABASE_URL (via .env / environment), ingests the bundle into
//! local Postgres under fresh IDs. Dev-only; never deployed.

use std::io::Read;

fn usage() -> ! {
    eprintln!("usage: import-game <bundle.json>");
    std::process::exit(2);
}

const MAX_BUNDLE_BYTES: u64 = 100 * 1024 * 1024;

/// Minimal version envelope decoded before the full `ExportBundle`, so a v1 or
/// other unsupported schema is rejected with a targeted error instead of a
/// confusing full-bundle deserialization failure (DRM-04c).
#[derive(serde::Deserialize)]
struct VersionEnvelope {
    schema_version: u32,
}

fn parse_bundle(raw: &str, path: &str) -> anyhow::Result<web::game::export::ExportBundle> {
    let envelope: VersionEnvelope =
        serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("parsing {path}: {e}"))?;
    if envelope.schema_version != web::game::export::BUNDLE_SCHEMA_VERSION {
        anyhow::bail!(
            "{path}: unsupported bundle schema_version {} (this build supports {})",
            envelope.schema_version,
            web::game::export::BUNDLE_SCHEMA_VERSION
        );
    }
    serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("parsing {path}: {e}"))
}

fn read_bundle_limited<R: std::io::Read>(
    reader: R,
    path: &str,
    max_bytes: u64,
) -> anyhow::Result<String> {
    let mut raw = String::new();
    let bytes_read = reader
        .take(max_bytes + 1)
        .read_to_string(&mut raw)
        .map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
    if bytes_read as u64 > max_bytes {
        anyhow::bail!("{path}: exceeds the {max_bytes} byte sanity limit");
    }
    Ok(raw)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let Some(path) = std::env::args().nth(1) else {
        usage()
    };

    let file = std::fs::File::open(&path).map_err(|e| anyhow::anyhow!("opening {path}: {e}"))?;
    let raw = read_bundle_limited(file, &path, MAX_BUNDLE_BYTES)?;
    let bundle = parse_bundle(&raw, &path)?;

    let pool = web::db::create_pool().await?;
    let http_client = reqwest::Client::new();
    let outcome = web::game::import::import_bundle(&pool, &http_client, &bundle).await?;

    for warning in &outcome.warnings {
        eprintln!("warning: {warning}");
    }
    println!(
        "imported {} game {} as local game {}",
        bundle.game_type_name, bundle.game.id, outcome.game_id
    );
    println!("open: /games/{}", outcome.game_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_path_rejects_over_limit_without_file_metadata() {
        let over_limit = vec![b'{'; 8];
        let err = read_bundle_limited(std::io::Cursor::new(over_limit), "test-bundle", 4)
            .expect_err("over-limit input must be rejected by the read path itself");
        assert!(
            err.to_string().contains("exceeds the 4 byte sanity limit"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn read_path_accepts_under_limit_source() {
        let under_limit = vec![b'{'; 4];
        let raw = read_bundle_limited(std::io::Cursor::new(under_limit), "test-bundle", 8)
            .expect("under-limit input must be accepted");
        assert_eq!(raw, "{{{{");
    }

    #[test]
    fn parse_bundle_rejects_v1_envelope_before_full_decode() {
        // A v1 bundle carries only the version envelope here - full v2
        // deserialization would fail on missing fields, so the targeted
        // unsupported-version error proves the envelope check runs first.
        let err = parse_bundle(r#"{"schema_version":1}"#, "bundle.json").unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported bundle schema_version 1 (this build supports 2)"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_bundle_rejects_future_version_before_full_decode() {
        let err = parse_bundle(r#"{"schema_version":999}"#, "bundle.json").unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported bundle schema_version 999 (this build supports 2)"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_bundle_accepts_current_version() {
        let bundle = web::game::export::ExportBundle {
            schema_version: web::game::export::BUNDLE_SCHEMA_VERSION,
            exported_at: time::OffsetDateTime::now_utc(),
            game_type_name: "Lost Cities".to_string(),
            game_version_name: "v1".to_string(),
            game_version_uri: "http://localhost:0/mock".to_string(),
            game: web::game::export::BundleGame {
                id: uuid::Uuid::new_v4(),
                is_finished: false,
                finished_at: None,
                end_reason: None,
                game_state: "state".to_string(),
                created_at: time::PrimitiveDateTime::new(
                    time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap(),
                    time::Time::MIDNIGHT,
                ),
                updated_at: time::PrimitiveDateTime::new(
                    time::Date::from_calendar_date(2020, time::Month::January, 1).unwrap(),
                    time::Time::MIDNIGHT,
                ),
            },
            players: vec![],
            bots: vec![],
            logs: vec![],
        };
        let raw = serde_json::to_string(&bundle).unwrap();
        let parsed = parse_bundle(&raw, "bundle.json").expect("v2 bundle must parse");
        assert_eq!(parsed.schema_version, 2);
    }
}
