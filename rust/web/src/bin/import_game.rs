//! #34 dev-side game import CLI (spec D5).
//!
//! Usage: cargo run -p web --features ssr --bin import-game -- bundle.json
//!
//! Reads DATABASE_URL (via .env / environment), ingests the bundle into
//! local Postgres under fresh IDs. Dev-only; never deployed.

fn usage() -> ! {
    eprintln!("usage: import-game <bundle.json>");
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let Some(path) = std::env::args().nth(1) else {
        usage()
    };

    let raw = std::fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
    let bundle: web::game::export::ExportBundle =
        serde_json::from_str(&raw).map_err(|e| anyhow::anyhow!("parsing {path}: {e}"))?;

    let pool = web::db::create_pool().await?;
    let outcome = web::game::import::import_bundle(&pool, &bundle).await?;

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
