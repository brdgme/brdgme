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

fn read_bundle_limited<R: std::io::Read>(
    mut reader: R,
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
    let bundle: web::game::export::ExportBundle =
        serde_json::from_str(&raw).map_err(|e| anyhow::anyhow!("parsing {path}: {e}"))?;

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
}
