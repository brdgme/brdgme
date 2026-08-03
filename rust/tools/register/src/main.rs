//! Registers locally running games from their canonical GameVersion manifests
//! into the local database, mirroring the operator's reconcile behavior.
//!
//! Single-game mode: register <game> --url <direct-localhost-url>
//!
//! Set mode:        register set <file.json | ->
//!
//! Reads `k8s/base/game/<game>/game-version.yaml`, queries the running game at
//! its probe URL for player counts and rules, and persists operator-equivalent
//! rows via `brdgme_registration`. Set mode takes a JSON document (file path
//! or `-` for stdin) that carries each selected game's identifier plus a
//! separate probe URL (Compose service DNS) and persisted URI (host-localhost)
//! per game, validates the whole input before any database mutation, fetches
//! all selected metadata first, then reconciles the stored set to exactly the
//! selected games in one transaction. Requires `DATABASE_URL`.
//!
//! Set input format:
//! ```json
//! {
//!   "games": [
//!     {"game": "tic-tac-toe-2", "probeUrl": "http://tic-tac-toe-2:8080", "uri": "http://127.0.0.1:8081"}
//!   ]
//! }
//! ```

use std::collections::HashSet;
use std::env;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use brdgme_cmd::api::{Request, Response};
use brdgme_game_client::GameClientError;
use brdgme_registration::{GameVersionManifest, Registration, SetStats};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;

fn manifest_path(base: &Path, game: &str) -> PathBuf {
    base.join("k8s")
        .join("base")
        .join("game")
        .join(game)
        .join("game-version.yaml")
}

#[derive(Debug, PartialEq)]
enum Command {
    Single { game: String, url: String },
    Set { input: String },
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    match args.get(1) {
        None => Err("missing game name or 'set' subcommand".to_string()),
        Some(subcommand) if subcommand == "set" => {
            let input = args
                .get(2)
                .ok_or("set requires an input path or '-' for stdin")?;
            if args.len() > 3 {
                return Err(format!("unexpected argument: {}", args[3]));
            }
            Ok(Command::Set {
                input: input.clone(),
            })
        }
        Some(game) => {
            let mut url = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--url" => {
                        i += 1;
                        url = Some(args.get(i).ok_or("--url requires a value")?.clone());
                    }
                    other => return Err(format!("unexpected argument: {other}")),
                }
                i += 1;
            }
            let url = url.ok_or("missing --url")?;
            Ok(Command::Single {
                game: game.clone(),
                url,
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameSelectionInput {
    game: String,
    probe_url: String,
    uri: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SetInput {
    games: Vec<GameSelectionInput>,
}

#[derive(Debug)]
struct Selection {
    probe_url: String,
    uri: String,
    manifest: GameVersionManifest,
}

fn read_set_input(input: &str) -> Result<String, String> {
    if input == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read set input from stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(input).map_err(|e| format!("failed to read {}: {e}", input))
    }
}

/// Parses and validates the set input: every selection must carry a non-empty
/// game identifier, probe URL and persisted URI, no game may repeat, and each
/// game must resolve to a canonical manifest. All validation happens here,
/// before any database mutation.
fn load_set(content: &str, base: &Path) -> Result<Vec<Selection>, String> {
    let input: SetInput =
        serde_json::from_str(content).map_err(|e| format!("invalid set input: {e}"))?;
    if input.games.is_empty() {
        return Err("set input must contain at least one game".to_string());
    }
    let mut selections = Vec::with_capacity(input.games.len());
    let mut seen_games = HashSet::new();
    for selection in input.games {
        if selection.game.is_empty() || selection.probe_url.is_empty() || selection.uri.is_empty() {
            return Err(format!(
                "selection for game {:?} must carry non-empty game, probeUrl and uri",
                selection.game
            ));
        }
        if !seen_games.insert(selection.game.clone()) {
            return Err(format!("duplicate game selection: {}", selection.game));
        }
        let manifest = GameVersionManifest::from_path(manifest_path(base, &selection.game))
            .map_err(|e| format!("failed to load manifest for {}: {e}", selection.game))?;
        selections.push(Selection {
            probe_url: selection.probe_url,
            uri: selection.uri,
            manifest,
        });
    }
    validate_set_identities(&selections)?;
    Ok(selections)
}

/// The set identity is the exact (game type, version) pair, so two selections
/// resolving to the same identity would be ambiguous.
fn validate_set_identities(selections: &[Selection]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for selection in selections {
        let identity = (
            selection.manifest.spec.type_name.clone(),
            selection.manifest.metadata.name.clone(),
        );
        if !seen.insert(identity.clone()) {
            return Err(format!(
                "duplicate game type+version selection: {} ({})",
                identity.0, identity.1
            ));
        }
    }
    Ok(())
}

async fn query_player_counts(
    client: &Client,
    url: &str,
    version_name: &str,
) -> Result<Vec<i32>, GameClientError> {
    match brdgme_game_client::request(client, url, version_name, &Request::PlayerCounts).await? {
        Response::PlayerCounts { player_counts } => {
            Ok(player_counts.into_iter().map(|c| c as i32).collect())
        }
        _other => Err(GameClientError::UnexpectedResponse {
            request: "PlayerCounts",
        }),
    }
}

async fn query_rules(
    client: &Client,
    url: &str,
    version_name: &str,
) -> Result<String, GameClientError> {
    match brdgme_game_client::request(client, url, version_name, &Request::Rules).await? {
        Response::Rules { rules } => Ok(rules),
        _other => Err(GameClientError::UnexpectedResponse { request: "Rules" }),
    }
}

/// Runs the full single-game registration sequence. The selected upsert and
/// the demotion of other stored versions run in one transaction, so a failing
/// demotion rolls the selected upsert back instead of leaving a newly public
/// version without its exclusivity. Returns the number of other stored
/// versions demoted to non-public.
async fn register_game(
    pool: &PgPool,
    manifest: &GameVersionManifest,
    url: &str,
    client: &Client,
) -> Result<u64, Box<dyn std::error::Error>> {
    let version_name = manifest.metadata.name.clone();
    let player_counts = query_player_counts(client, url, &version_name).await?;
    let rules = query_rules(client, url, &version_name).await?;
    let registration = Registration::from_manifest(manifest, url.to_string(), player_counts, rules);
    let mut tx = pool.begin().await?;
    brdgme_registration::upsert(&mut tx, &registration).await?;
    let demoted = brdgme_registration::mark_others_non_public(&mut tx, &version_name).await?;
    tx.commit().await?;
    Ok(demoted)
}

/// Fetches and validates the selected games' metadata from their probe URLs
/// first, then reconciles the stored set to exactly the selection in one
/// transaction. Any probe failure returns before any database mutation.
async fn register_set(
    pool: &PgPool,
    selections: &[Selection],
    client: &Client,
) -> Result<SetStats, Box<dyn std::error::Error>> {
    let mut registrations = Vec::with_capacity(selections.len());
    for selection in selections {
        let version_name = &selection.manifest.metadata.name;
        let player_counts = query_player_counts(client, &selection.probe_url, version_name).await?;
        let rules = query_rules(client, &selection.probe_url, version_name).await?;
        registrations.push(Registration::from_manifest(
            &selection.manifest,
            selection.uri.clone(),
            player_counts,
            rules,
        ));
    }
    Ok(brdgme_registration::bulk_set(pool, &registrations).await?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;
    let client = Client::new();
    match parse_args(&args)? {
        Command::Single { game, url } => {
            let manifest = GameVersionManifest::from_path(manifest_path(Path::new("."), &game))?;
            let demoted = register_game(&pool, &manifest, &url, &client).await?;
            println!(
                "Registered {} at {} ({} other version(s) marked non-public)",
                manifest.metadata.name, url, demoted
            );
        }
        Command::Set { input } => {
            let content = read_set_input(&input)?;
            let selections = load_set(&content, Path::new("."))?;
            let stats = register_set(&pool, &selections, &client).await?;
            println!(
                "Registered {} game(s) public ({} other version(s) marked non-public)",
                stats.registered, stats.demoted
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, routing::post};
    use tokio::net::TcpListener;

    const TIC_TAC_TOE_MANIFEST: &str =
        include_str!("../../../../k8s/base/game/tic-tac-toe-2/game-version.yaml");
    const LOST_CITIES_MANIFEST: &str =
        include_str!("../../../../k8s/base/game/lost-cities-2/game-version.yaml");
    const ZOMBIE_DICE_MANIFEST: &str =
        include_str!("../../../../k8s/base/game/zombie-dice-2/game-version.yaml");

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    fn mock_game_server() -> Router {
        Router::new().route(
            "/",
            post(|Json(req): Json<Request>| async move {
                Json(match req {
                    Request::PlayerCounts => Response::PlayerCounts {
                        player_counts: vec![1, 2],
                    },
                    Request::Rules => Response::Rules {
                        rules: "game rules".to_string(),
                    },
                    other => Response::SystemError {
                        message: format!("unsupported in mock: {other:?}"),
                    },
                })
            }),
        )
    }

    async fn start_mock_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, mock_game_server()).await.unwrap();
        });
        format!("http://{}", addr)
    }

    #[test]
    fn parse_args_requires_game_and_url() {
        assert_eq!(
            parse_args(&[
                "register".to_string(),
                "tic-tac-toe-2".to_string(),
                "--url".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ])
            .unwrap(),
            Command::Single {
                game: "tic-tac-toe-2".to_string(),
                url: "http://127.0.0.1:8080".to_string(),
            }
        );
        assert!(parse_args(&["register".to_string()]).is_err());
        assert!(parse_args(&["register".to_string(), "tic-tac-toe-2".to_string()]).is_err());
        assert!(
            parse_args(&[
                "register".to_string(),
                "tic-tac-toe-2".to_string(),
                "--url".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn parse_args_set_subcommand() {
        assert_eq!(
            parse_args(&[
                "register".to_string(),
                "set".to_string(),
                "games.json".to_string()
            ])
            .unwrap(),
            Command::Set {
                input: "games.json".to_string(),
            }
        );
        assert_eq!(
            parse_args(&["register".to_string(), "set".to_string(), "-".to_string()]).unwrap(),
            Command::Set {
                input: "-".to_string(),
            }
        );
        assert!(parse_args(&["register".to_string(), "set".to_string()]).is_err());
        assert!(
            parse_args(&[
                "register".to_string(),
                "set".to_string(),
                "games.json".to_string(),
                "extra".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn manifest_path_resolves_selected_game() {
        assert_eq!(
            manifest_path(Path::new("."), "tic-tac-toe-2"),
            PathBuf::from("./k8s/base/game/tic-tac-toe-2/game-version.yaml")
        );
    }

    #[test]
    fn load_set_resolves_canonical_manifests() {
        let content = r#"
            {
              "games": [
                {"game": "tic-tac-toe-2", "probeUrl": "http://tic-tac-toe-2:8080", "uri": "http://127.0.0.1:8081"},
                {"game": "lost-cities-2", "probeUrl": "http://lost-cities-2:8080", "uri": "http://127.0.0.1:8082"}
              ]
            }
        "#;
        let selections = load_set(content, &repo_root()).unwrap();
        assert_eq!(selections.len(), 2);
        assert_eq!(selections[0].probe_url, "http://tic-tac-toe-2:8080");
        assert_eq!(selections[0].uri, "http://127.0.0.1:8081");
        assert_eq!(selections[0].manifest.metadata.name, "tic-tac-toe-2");
        assert_eq!(selections[1].manifest.spec.type_name, "Lost Cities");
    }

    #[test]
    fn load_set_rejects_invalid_input() {
        assert!(load_set("not json", &repo_root()).is_err());
        assert!(
            load_set(r#"{"games": []}"#, &repo_root()).is_err(),
            "empty selection must be rejected"
        );
        assert!(
            load_set(
                r#"{"games": [{"game": "", "probeUrl": "http://x", "uri": "http://y"}]}"#,
                &repo_root()
            )
            .is_err(),
            "empty game identifier must be rejected"
        );
        assert!(
            load_set(
                r#"{"games": [{"game": "tic-tac-toe-2", "probeUrl": "", "uri": "http://y"}]}"#,
                &repo_root()
            )
            .is_err(),
            "empty probeUrl must be rejected"
        );
        assert!(
            load_set(
                r#"{"games": [{"game": "tic-tac-toe-2", "probeUrl": "http://x"}]}"#,
                &repo_root()
            )
            .is_err(),
            "missing uri must be rejected"
        );
        assert!(
            load_set(
                r#"{"games": [
                    {"game": "tic-tac-toe-2", "probeUrl": "http://x", "uri": "http://y"},
                    {"game": "tic-tac-toe-2", "probeUrl": "http://z", "uri": "http://w"}
                ]}"#,
                &repo_root()
            )
            .is_err(),
            "duplicate game selection must be rejected"
        );
        assert!(
            load_set(
                r#"{"games": [{"game": "not-a-game", "probeUrl": "http://x", "uri": "http://y"}]}"#,
                &repo_root()
            )
            .is_err(),
            "game without a canonical manifest must be rejected"
        );
    }

    #[test]
    fn validate_set_identities_rejects_duplicate_type_and_version() {
        let manifest: GameVersionManifest = TIC_TAC_TOE_MANIFEST.parse().unwrap();
        let selections = vec![
            Selection {
                probe_url: "http://a".to_string(),
                uri: "http://b".to_string(),
                manifest: manifest.clone(),
            },
            Selection {
                probe_url: "http://c".to_string(),
                uri: "http://d".to_string(),
                manifest,
            },
        ];
        assert!(
            validate_set_identities(&selections).is_err(),
            "two selections mapping to the same exact game type+version must be rejected"
        );
    }

    // Applies the web crate's migrations so the schema matches production.
    #[sqlx::test(migrations = "../../web/migrations")]
    async fn register_game_queries_mock_persists_and_demotes_others(pool: PgPool) {
        let uri = start_mock_server().await;
        let client = Client::new();
        let ttt = TIC_TAC_TOE_MANIFEST.parse().unwrap();
        let lc = LOST_CITIES_MANIFEST.parse().unwrap();

        register_game(&pool, &ttt, &uri, &client).await.unwrap();
        register_game(&pool, &lc, &uri, &client).await.unwrap();

        let ttt_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'tic-tac-toe-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let lc_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'lost-cities-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!ttt_public, "previous selection must be demoted");
        assert!(lc_public, "selected version must be public");

        let versions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(versions, 2, "demotion must not delete rows");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn register_game_stores_direct_url_and_queried_metadata(pool: PgPool) {
        let uri = start_mock_server().await;
        let client = Client::new();
        let ttt = TIC_TAC_TOE_MANIFEST.parse().unwrap();

        register_game(&pool, &ttt, &uri, &client).await.unwrap();

        let (game_uri, rules, player_counts): (String, String, Vec<i32>) = sqlx::query_as(
            "SELECT gv.uri, gv.rules, gt.player_counts \
             FROM game_versions gv JOIN game_types gt ON gt.id = gv.game_type_id \
             WHERE gv.name = 'tic-tac-toe-2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(game_uri, uri);
        assert_eq!(rules, "game rules");
        assert_eq!(player_counts, vec![1, 2]);
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn register_game_demotion_failure_rolls_back_selected_upsert(pool: PgPool) {
        let uri = start_mock_server().await;
        let client = Client::new();

        // A pre-existing public version that must survive the failed run.
        let zombie: GameVersionManifest = ZOMBIE_DICE_MANIFEST.parse().unwrap();
        brdgme_registration::upsert(
            &pool,
            &Registration::from_manifest(&zombie, uri.clone(), vec![2], "game rules".to_string()),
        )
        .await
        .unwrap();

        // Demotion is an UPDATE on game_versions; fail it after the selected
        // version's upsert has already run inside the transaction.
        sqlx::query(
            "CREATE OR REPLACE FUNCTION fail_game_version_update() RETURNS trigger AS \
             $$ BEGIN RAISE EXCEPTION 'forced failure'; END $$ LANGUAGE plpgsql",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TRIGGER fail_game_version_update \
             BEFORE UPDATE ON game_versions FOR EACH ROW EXECUTE FUNCTION fail_game_version_update()",
        )
        .execute(&pool)
        .await
        .unwrap();

        let ttt: GameVersionManifest = TIC_TAC_TOE_MANIFEST.parse().unwrap();
        assert!(
            register_game(&pool, &ttt, &uri, &client).await.is_err(),
            "failing demotion must abort registration"
        );

        let versions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(versions, 1, "selected version must be rolled back");
        let selected: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_versions WHERE name = 'tic-tac-toe-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(selected, 0, "selected version must not remain");
        let selected_type: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_types WHERE name = 'Tic-tac-toe'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(selected_type, 0, "selected type must not remain");
        let zombie_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'zombie-dice-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(zombie_public, "pre-existing row must retain its state");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn register_set_persists_separate_probe_and_persisted_uris(pool: PgPool) {
        let probe = start_mock_server().await;
        let client = Client::new();
        let content = format!(
            r#"
            {{
              "games": [
                {{"game": "tic-tac-toe-2", "probeUrl": "{probe}", "uri": "http://127.0.0.1:8081"}},
                {{"game": "lost-cities-2", "probeUrl": "{probe}", "uri": "http://127.0.0.1:8082"}}
              ]
            }}
            "#
        );
        let selections = load_set(&content, &repo_root()).unwrap();
        let stats = register_set(&pool, &selections, &client).await.unwrap();
        assert_eq!(stats.registered, 2);
        assert_eq!(stats.demoted, 0);

        let (ttt_uri, ttt_rules, ttt_public): (String, String, bool) = sqlx::query_as(
            "SELECT uri, rules, is_public FROM game_versions WHERE name = 'tic-tac-toe-2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let (lc_uri, lc_public): (String, bool) =
            sqlx::query_as("SELECT uri, is_public FROM game_versions WHERE name = 'lost-cities-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            ttt_uri, "http://127.0.0.1:8081",
            "persisted URI must not be the probe URL"
        );
        assert_eq!(
            ttt_rules, "game rules",
            "metadata must come from the probe URL"
        );
        assert!(ttt_public);
        assert_eq!(lc_uri, "http://127.0.0.1:8082");
        assert!(lc_public);

        // Second run with the same set is a no-op: no rows duplicated.
        let stats = register_set(&pool, &selections, &client).await.unwrap();
        assert_eq!(stats.demoted, 0);
        let versions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(versions, 2);
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn register_set_probe_failure_aborts_before_db_mutation(pool: PgPool) {
        let probe = start_mock_server().await;
        let client = Client::new();
        // tic-tac-toe-2 probes a live mock, but one selected game points at a
        // dead port, so the whole set must fail before any row is written.
        let content = format!(
            r#"
            {{
              "games": [
                {{"game": "tic-tac-toe-2", "probeUrl": "{probe}", "uri": "http://127.0.0.1:8081"}},
                {{"game": "lost-cities-2", "probeUrl": "http://127.0.0.1:1", "uri": "http://127.0.0.1:8082"}}
              ]
            }}
            "#
        );
        let selections = load_set(&content, &repo_root()).unwrap();
        assert!(register_set(&pool, &selections, &client).await.is_err());

        let versions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(versions, 0, "no row may persist when any probe fails");
        let types: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_types")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(types, 0);
    }
}
