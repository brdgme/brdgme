use crate::game::server_fns::{GameLogEntry, GameViewData};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrdgmeGameUpdate {
    pub game_id: Uuid,
    pub game_view: GameViewData,
    pub logs: Vec<GameLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketMessage {
    BrdgmeUpdate(BrdgmeGameUpdate),
}

#[cfg(feature = "ssr")]
pub use ssr::*;

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;
    use axum::{
        extract::{
            State,
            ws::{Message, WebSocket, WebSocketUpgrade},
        },
        response::IntoResponse,
    };
    use brdgme_cmd::api::{PlayerRender, PubRender};
    use futures_util::{sink::SinkExt, stream::StreamExt};
    use serde::Serialize;
    use time::PrimitiveDateTime;
    use tower_sessions::Session;

    use crate::auth::session::get_user_from_session;
    use crate::db::GameExtended;
    use crate::game::server_fns::{GameLogEntry, GameViewData, PlayerViewData};
    use crate::models::game::GameLog;

    // Legacy-compatible serialization structs matching the format the React frontend expects.
    // Field names and structure mirror the legacy rust/api ShowResponse.
    // DELETE at Phase 16 decommission (all structs below, through LegacyGameUpdateMessage).

    #[derive(Serialize)]
    struct LegacyGame {
        id: Uuid,
        created_at: PrimitiveDateTime,
        updated_at: PrimitiveDateTime,
        game_version_id: Uuid,
        is_finished: bool,
        finished_at: Option<PrimitiveDateTime>,
        chat_id: Option<Uuid>,
        restarted_game_id: Option<Uuid>,
    }

    #[derive(Serialize)]
    struct LegacyGameType {
        id: Uuid,
        created_at: PrimitiveDateTime,
        updated_at: PrimitiveDateTime,
        name: String,
        player_counts: Vec<i32>,
    }

    #[derive(Serialize)]
    struct LegacyGameVersion {
        id: Uuid,
        created_at: PrimitiveDateTime,
        updated_at: PrimitiveDateTime,
        game_type_id: Uuid,
        name: String,
        is_public: bool,
        is_deprecated: bool,
    }

    #[derive(Serialize, Clone)]
    struct LegacyGamePlayer {
        id: Uuid,
        created_at: PrimitiveDateTime,
        updated_at: PrimitiveDateTime,
        game_id: Uuid,
        user_id: Option<Uuid>,
        position: i32,
        color: String,
        has_accepted: bool,
        is_turn: bool,
        is_turn_at: PrimitiveDateTime,
        last_turn_at: PrimitiveDateTime,
        is_eliminated: bool,
        is_read: bool,
        points: Option<f32>,
        can_undo: bool,
        place: Option<i32>,
        rating_change: Option<i32>,
    }

    #[derive(Serialize, Clone)]
    struct LegacyUser {
        id: Uuid,
        created_at: PrimitiveDateTime,
        updated_at: PrimitiveDateTime,
        name: String,
        pref_colors: Vec<String>,
    }

    #[derive(Serialize, Clone)]
    struct LegacyGameTypeUser {
        id: Uuid,
        created_at: PrimitiveDateTime,
        updated_at: PrimitiveDateTime,
        game_type_id: Uuid,
        user_id: Uuid,
        rating: i32,
        peak_rating: i32,
    }

    #[derive(Serialize, Clone)]
    struct LegacyGamePlayerEntry {
        game_player: LegacyGamePlayer,
        user: LegacyUser,
        game_type_user: LegacyGameTypeUser,
    }

    #[derive(Serialize)]
    struct LegacyGameLog {
        id: Uuid,
        created_at: PrimitiveDateTime,
        updated_at: PrimitiveDateTime,
        game_id: Uuid,
        is_public: bool,
        logged_at: PrimitiveDateTime,
        body: String,
    }

    #[derive(Serialize)]
    struct LegacyRenderedGameLog {
        game_log: LegacyGameLog,
        html: String,
    }

    #[derive(Serialize)]
    struct LegacyShowResponse {
        game: LegacyGame,
        game_type: LegacyGameType,
        game_version: LegacyGameVersion,
        game_player: Option<LegacyGamePlayer>,
        game_players: Vec<LegacyGamePlayerEntry>,
        game_logs: Vec<LegacyRenderedGameLog>,
        pub_state: Option<String>,
        html: String,
        command_spec: Option<brdgme_game::command::Spec>,
        chat: Option<serde_json::Value>,
    }

    #[derive(Serialize)]
    struct LegacyGameUpdateMessage {
        #[serde(rename = "GameUpdate")]
        game_update: LegacyShowResponse,
    }

    fn build_markup_players(game_extended: &GameExtended) -> Vec<brdgme_markup::Player> {
        game_extended
            .game_players
            .iter()
            .map(|p| {
                use std::str::FromStr;
                brdgme_markup::Player {
                    name: p.name().to_string(),
                    color: brdgme_color::Color::from_str(&p.game_player.color)
                        .unwrap_or(brdgme_color::WHITE),
                }
            })
            .collect()
    }

    fn render_markup(markup_str: &str, players: &[brdgme_markup::Player]) -> String {
        let (nodes, _) = brdgme_markup::from_string(markup_str).unwrap_or_else(|_| (vec![], ""));
        brdgme_markup::html(&brdgme_markup::transform(&nodes, players))
    }

    fn build_legacy_game_players(game_extended: &GameExtended) -> Vec<LegacyGamePlayerEntry> {
        game_extended
            .game_players
            .iter()
            .map(|p| LegacyGamePlayerEntry {
                game_player: LegacyGamePlayer {
                    id: p.game_player.id,
                    created_at: p.game_player.created_at,
                    updated_at: p.game_player.updated_at,
                    game_id: p.game_player.game_id,
                    user_id: p.game_player.user_id,
                    position: p.game_player.position,
                    color: p.game_player.color.clone(),
                    has_accepted: p.game_player.has_accepted,
                    is_turn: p.game_player.is_turn,
                    is_turn_at: p.game_player.is_turn_at,
                    last_turn_at: p.game_player.last_turn_at,
                    is_eliminated: p.game_player.is_eliminated,
                    is_read: p.game_player.is_read,
                    points: p.game_player.points,
                    can_undo: p.game_player.undo_game_state.is_some(),
                    place: p.game_player.place,
                    rating_change: p.game_player.rating_change,
                },
                user: match &p.user {
                    Some(u) => LegacyUser {
                        id: u.id,
                        created_at: u.created_at,
                        updated_at: u.updated_at,
                        name: u.name.clone(),
                        pref_colors: u.pref_colors.clone(),
                    },
                    None => LegacyUser {
                        id: p.game_bot.as_ref().map(|b| b.id).unwrap_or(Uuid::nil()),
                        created_at: p.game_player.created_at,
                        updated_at: p.game_player.updated_at,
                        name: p.name().to_string(),
                        pref_colors: vec![],
                    },
                },
                game_type_user: LegacyGameTypeUser {
                    id: p.game_type_user.id,
                    created_at: p.game_type_user.created_at,
                    updated_at: p.game_type_user.updated_at,
                    game_type_id: p.game_type_user.game_type_id,
                    user_id: p.game_type_user.user_id,
                    rating: p.game_type_user.rating,
                    peak_rating: p.game_type_user.peak_rating,
                },
            })
            .collect()
    }

    fn build_legacy_game_logs(
        logs: &[GameLog],
        markup_players: &[brdgme_markup::Player],
    ) -> Vec<LegacyRenderedGameLog> {
        logs.iter()
            .map(|log| LegacyRenderedGameLog {
                game_log: LegacyGameLog {
                    id: log.id,
                    created_at: log.created_at,
                    updated_at: log.updated_at,
                    game_id: log.game_id,
                    is_public: log.is_public,
                    logged_at: log.logged_at,
                    body: log.body.clone(),
                },
                html: render_markup(&log.body, markup_players),
            })
            .collect()
    }

    #[derive(Clone)]
    pub struct GameBroadcaster {
        conn: redis::aio::MultiplexedConnection,
        client: redis::Client,
    }

    impl GameBroadcaster {
        pub fn new(conn: redis::aio::MultiplexedConnection, client: redis::Client) -> Self {
            Self { conn, client }
        }

        async fn publish(&self, channel: &str, payload: &str) {
            let mut conn = self.conn.clone();
            if let Err(e) = redis::cmd("PUBLISH")
                .arg(channel)
                .arg(payload)
                .query_async::<()>(&mut conn)
                .await
            {
                tracing::error!("Redis PUBLISH failed on {}: {}", channel, e);
            }
        }

        pub async fn broadcast_game_update(
            &self,
            pool: &sqlx::PgPool,
            game_extended: &GameExtended,
            new_logs: &[GameLog],
            public_render: &PubRender,
            player_renders: &[PlayerRender],
        ) {
            let game_id = game_extended.game.id;
            let markup_players = build_markup_players(game_extended);
            let legacy_game_players = build_legacy_game_players(game_extended);
            let legacy_logs = build_legacy_game_logs(new_logs, &markup_players);

            let pub_html = render_markup(&public_render.render, &markup_players);

            let public_response = LegacyShowResponse {
                game: LegacyGame {
                    id: game_extended.game.id,
                    created_at: game_extended.game.created_at,
                    updated_at: game_extended.game.updated_at,
                    game_version_id: game_extended.game.game_version_id,
                    is_finished: game_extended.game.is_finished,
                    finished_at: game_extended.game.finished_at,
                    chat_id: game_extended.game.chat_id,
                    restarted_game_id: game_extended.game.restarted_game_id,
                },
                game_type: LegacyGameType {
                    id: game_extended.game_type.id,
                    created_at: game_extended.game_type.created_at,
                    updated_at: game_extended.game_type.updated_at,
                    name: game_extended.game_type.name.clone(),
                    player_counts: game_extended.game_type.player_counts.clone(),
                },
                game_version: LegacyGameVersion {
                    id: game_extended.game_version.id,
                    created_at: game_extended.game_version.created_at,
                    updated_at: game_extended.game_version.updated_at,
                    game_type_id: game_extended.game_version.game_type_id,
                    name: game_extended.game_version.name.clone(),
                    is_public: game_extended.game_version.is_public,
                    is_deprecated: game_extended.game_version.is_deprecated,
                },
                game_player: None,
                game_players: legacy_game_players.clone(),
                game_logs: legacy_logs
                    .iter()
                    .filter(|l| l.game_log.is_public)
                    .map(|l| LegacyRenderedGameLog {
                        game_log: LegacyGameLog {
                            id: l.game_log.id,
                            created_at: l.game_log.created_at,
                            updated_at: l.game_log.updated_at,
                            game_id: l.game_log.game_id,
                            is_public: l.game_log.is_public,
                            logged_at: l.game_log.logged_at,
                            body: l.game_log.body.clone(),
                        },
                        html: l.html.clone(),
                    })
                    .collect(),
                pub_state: Some(public_render.pub_state.clone()),
                html: pub_html,
                command_spec: None,
                chat: None,
            };

            let pub_payload = match serde_json::to_string(&LegacyGameUpdateMessage {
                game_update: public_response,
            }) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to serialize public GameUpdate: {}", e);
                    return;
                }
            };
            self.publish(&format!("game.{}", game_id), &pub_payload)
                .await;

            // Publish private per-user messages
            for gpe in &game_extended.game_players {
                let player_render = match player_renders.get(gpe.game_player.position as usize) {
                    Some(pr) => pr,
                    None => continue,
                };

                // Use the same filtering as the display path: public logs + logs targeted
                // to this specific player via game_log_targets.
                let player_logs =
                    match crate::db::get_game_logs(pool, game_id, gpe.game_player.id).await {
                        Ok(logs) => logs,
                        Err(e) => {
                            tracing::error!(
                                "Failed to fetch logs for player {}: {}",
                                gpe.game_player.id,
                                e
                            );
                            continue;
                        }
                    };

                let player_legacy_logs: Vec<LegacyRenderedGameLog> = player_logs
                    .iter()
                    .map(|log| LegacyRenderedGameLog {
                        game_log: LegacyGameLog {
                            id: log.id,
                            created_at: log.created_at,
                            updated_at: log.updated_at,
                            game_id: log.game_id,
                            is_public: log.is_public,
                            logged_at: log.logged_at,
                            body: log.body.clone(),
                        },
                        html: render_markup(&log.body, &markup_players),
                    })
                    .collect();

                let player_html = render_markup(&player_render.render, &markup_players);
                let legacy_gp = legacy_game_players
                    .iter()
                    .find(|p| p.game_player.id == gpe.game_player.id)
                    .map(|p| p.game_player.clone());

                let private_response = LegacyShowResponse {
                    game: LegacyGame {
                        id: game_extended.game.id,
                        created_at: game_extended.game.created_at,
                        updated_at: game_extended.game.updated_at,
                        game_version_id: game_extended.game.game_version_id,
                        is_finished: game_extended.game.is_finished,
                        finished_at: game_extended.game.finished_at,
                        chat_id: game_extended.game.chat_id,
                        restarted_game_id: game_extended.game.restarted_game_id,
                    },
                    game_type: LegacyGameType {
                        id: game_extended.game_type.id,
                        created_at: game_extended.game_type.created_at,
                        updated_at: game_extended.game_type.updated_at,
                        name: game_extended.game_type.name.clone(),
                        player_counts: game_extended.game_type.player_counts.clone(),
                    },
                    game_version: LegacyGameVersion {
                        id: game_extended.game_version.id,
                        created_at: game_extended.game_version.created_at,
                        updated_at: game_extended.game_version.updated_at,
                        game_type_id: game_extended.game_version.game_type_id,
                        name: game_extended.game_version.name.clone(),
                        is_public: game_extended.game_version.is_public,
                        is_deprecated: game_extended.game_version.is_deprecated,
                    },
                    game_player: legacy_gp,
                    game_players: legacy_game_players.clone(),
                    game_logs: player_legacy_logs,
                    pub_state: Some(player_render.player_state.clone()),
                    html: player_html.clone(),
                    command_spec: player_render.command_spec.clone(),
                    chat: None,
                };

                let private_payload = match serde_json::to_string(&LegacyGameUpdateMessage {
                    game_update: private_response,
                }) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Failed to serialize private GameUpdate: {}", e);
                        continue;
                    }
                };

                if let Some(ref user) = gpe.user {
                    let user_id = user.id;

                    // Look up auth tokens for this player's user_id
                    let token_ids =
                        sqlx::query("SELECT id FROM user_auth_tokens WHERE user_id = $1")
                            .bind(user_id)
                            .fetch_all(pool)
                            .await;

                    match token_ids {
                        Ok(rows) => {
                            for row in rows {
                                use sqlx::Row;
                                let token_id: Uuid = match row.try_get("id") {
                                    Ok(id) => id,
                                    Err(e) => {
                                        tracing::error!("Failed to get token id: {}", e);
                                        continue;
                                    }
                                };
                                self.publish(&format!("user.{}", token_id), &private_payload)
                                    .await;
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to fetch auth tokens for user {}: {}",
                                user_id,
                                e
                            );
                        }
                    }

                    // Publish Leptos-specific update directly to the user's WS channel.
                    let last_turn_at = gpe.game_player.last_turn_at;
                    let log_entries: Vec<GameLogEntry> = player_logs
                        .iter()
                        .map(|log| GameLogEntry {
                            body_html: render_markup(&log.body, &markup_players),
                            logged_at: log.logged_at,
                            is_new: log.created_at >= last_turn_at,
                        })
                        .collect();

                    let game_view = GameViewData {
                        id: game_extended.game.id,
                        type_name: game_extended.game_type.name.clone(),
                        version_name: game_extended.game_version.name.clone(),
                        html: player_html,
                        is_my_turn: gpe.game_player.is_turn,
                        is_finished: game_extended.game.is_finished,
                        can_undo: gpe.game_player.undo_game_state.is_some(),
                        restarted_game_id: game_extended.game.restarted_game_id,
                        is_2player: game_extended.game_players.len() == 2,
                        players: game_extended
                            .game_players
                            .iter()
                            .map(|p| {
                                use std::str::FromStr;
                                let color = brdgme_color::Color::from_str(&p.game_player.color)
                                    .unwrap_or(brdgme_color::WHITE)
                                    .hex();
                                PlayerViewData {
                                    name: p.name().to_string(),
                                    color,
                                    rating: p.game_type_user.rating,
                                    points: p.game_player.points.unwrap_or(0.0),
                                    is_turn: p.game_player.is_turn,
                                    is_bot: p.game_bot.is_some(),
                                }
                            })
                            .collect(),
                        command_spec: player_render.command_spec.clone(),
                    };

                    let brdgme_msg = WebSocketMessage::BrdgmeUpdate(super::BrdgmeGameUpdate {
                        game_id,
                        game_view,
                        logs: log_entries,
                    });
                    if let Ok(brdgme_payload) = serde_json::to_string(&brdgme_msg) {
                        self.publish(&format!("ws.{}", user_id), &brdgme_payload)
                            .await;
                    }
                }
            }
        }
    }

    pub async fn ws_handler(
        ws: WebSocketUpgrade,
        State(broadcaster): State<GameBroadcaster>,
        session: Session,
    ) -> impl IntoResponse {
        let user_id = get_user_from_session(&session).await.map(|u| u.id);
        ws.on_upgrade(move |socket| handle_socket(socket, broadcaster, user_id))
    }

    async fn handle_socket(socket: WebSocket, broadcaster: GameBroadcaster, user_id: Option<Uuid>) {
        let (mut sender, mut receiver) = socket.split();

        let mut pubsub = match broadcaster.client.get_async_pubsub().await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to get Redis pubsub connection: {}", e);
                return;
            }
        };

        // TODO(Phase 17 NATS): this subscribes to the `game.*` firehose and forwards every
        // game's updates to every connected client. The NATS-based redesign should subscribe
        // per-game/per-user instead, so a client only receives messages relevant to it.
        if let Err(e) = pubsub.psubscribe("game.*").await {
            tracing::error!("Redis PSUBSCRIBE failed: {}", e);
            return;
        }

        if let Some(uid) = user_id
            && let Err(e) = pubsub.subscribe(format!("ws.{}", uid)).await
        {
            tracing::error!("Redis SUBSCRIBE failed for ws.{}: {}", uid, e);
        }

        let mut stream = pubsub.into_on_message();

        // Periodic ping to keep idle connections alive across load-balancer idle timeouts.
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ping_interval.tick().await; // first tick fires immediately, skip it

        loop {
            tokio::select! {
                msg = stream.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => break,
                    };
                    let payload: String = match msg.get_payload() {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    if sender.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
                _ = ping_interval.tick() => {
                    if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
                // Drain inbound messages so pongs and close frames are processed; we don't
                // act on client-sent data here.
                incoming = receiver.next() => {
                    match incoming {
                        Some(Ok(_)) => {}
                        _ => break,
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::db::{self, CreateGameOpts};
        use crate::models::user::User;
        use brdgme_cmd::api::{CliLog, PlayerRender, PubRender};
        use sqlx::PgPool;
        use std::time::Duration;
        use tokio::time::timeout;

        async fn make_broadcaster() -> GameBroadcaster {
            let redis_url =
                std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
            let client = redis::Client::open(redis_url).unwrap();
            let conn = client.get_multiplexed_async_connection().await.unwrap();
            GameBroadcaster::new(conn, client)
        }

        async fn make_user(pool: &PgPool, name: &str) -> User {
            sqlx::query_as!(
                User,
                "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING *",
                Uuid::new_v4(),
                name,
                &Vec::<String>::new()
            )
            .fetch_one(pool)
            .await
            .unwrap()
        }

        async fn make_auth_token(pool: &PgPool, user_id: Uuid) -> Uuid {
            let token_id = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)",
                token_id,
                user_id
            )
            .execute(pool)
            .await
            .unwrap();
            token_id
        }

        /// Two human players, positions 0 and 1, pointed at a dummy (never
        /// dereferenced) game version URI.
        async fn make_two_player_game(pool: &PgPool, p0: &User, p1: &User) -> Uuid {
            let game_type_id = sqlx::query_scalar!(
                "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
                format!("Test Game {}", Uuid::new_v4()),
                &vec![2, 3, 4]
            )
            .fetch_one(pool)
            .await
            .unwrap();
            let game_version_id = sqlx::query_scalar!(
                r#"INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
                   VALUES ($1, $2, $3, true, false) RETURNING id"#,
                game_type_id,
                "1.0.0",
                "http://localhost:0/mock"
            )
            .fetch_one(pool)
            .await
            .unwrap();

            let game = db::create_game_with_users(
                pool,
                CreateGameOpts {
                    game_version_id,
                    whose_turn: &[0],
                    eliminated: &[],
                    placings: &[],
                    points: &[],
                    creator_id: p0.id,
                    opponent_ids: &[p1.id],
                    opponent_emails: &[],
                    bot_slots: &[],
                    chat_id: None,
                    game_state: "initial_state",
                },
            )
            .await
            .unwrap();
            game.id
        }

        fn pub_render() -> PubRender {
            PubRender {
                pub_state: "pub_state_value".to_string(),
                render: "pub render".to_string(),
            }
        }

        fn player_renders_for(count: usize) -> Vec<PlayerRender> {
            (0..count)
                .map(|i| PlayerRender {
                    player_state: format!("p{}_state", i),
                    render: format!("p{}_render", i),
                    command_spec: None,
                })
                .collect()
        }

        async fn recv_payload(pubsub: &mut redis::aio::PubSub) -> serde_json::Value {
            let msg = timeout(Duration::from_secs(5), pubsub.on_message().next())
                .await
                .expect("timed out waiting for redis pub/sub message")
                .expect("pub/sub stream ended unexpectedly");
            let payload: String = msg.get_payload().unwrap();
            serde_json::from_str(&payload).unwrap()
        }

        #[sqlx::test]
        async fn broadcast_publishes_legacy_shaped_public_payload(pool: PgPool) {
            let p0 = make_user(&pool, "p0").await;
            let p1 = make_user(&pool, "p1").await;
            let game_id = make_two_player_game(&pool, &p0, &p1).await;

            let base = time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            );
            db::create_game_logs(
                &pool,
                game_id,
                vec![CliLog {
                    content: "public msg".to_string(),
                    at: base,
                    public: true,
                    to: vec![],
                }],
            )
            .await
            .unwrap();

            let ge = db::find_game_extended(&pool, game_id)
                .await
                .unwrap()
                .unwrap();
            let all_logs = db::get_all_game_logs(&pool, game_id).await.unwrap();

            let broadcaster = make_broadcaster().await;
            let client = redis::Client::open(
                std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            )
            .unwrap();
            let mut pubsub = client.get_async_pubsub().await.unwrap();
            pubsub.subscribe(format!("game.{}", game_id)).await.unwrap();

            broadcaster
                .broadcast_game_update(&pool, &ge, &all_logs, &pub_render(), &player_renders_for(2))
                .await;

            let v = recv_payload(&mut pubsub).await;
            let update = &v["GameUpdate"];

            // Legacy field names/structure that web-legacy's ShowResponse expects.
            assert_eq!(update["game"]["id"].as_str().unwrap(), game_id.to_string());
            assert!(update["game"].get("created_at").is_some());
            assert!(update["game_type"].get("name").is_some());
            assert!(update["game_version"].get("name").is_some());
            // Public payload is not scoped to a viewing player.
            assert!(update["game_player"].is_null());
            assert!(update["command_spec"].is_null());
            assert!(update["chat"].is_null());

            let players = update["game_players"].as_array().unwrap();
            assert_eq!(players.len(), 2);
            for p in players {
                assert!(p.get("game_player").is_some());
                assert!(p.get("user").is_some());
                assert!(p.get("game_type_user").is_some());
            }

            let logs = update["game_logs"].as_array().unwrap();
            assert_eq!(logs.len(), 1);
            assert_eq!(logs[0]["game_log"]["body"].as_str().unwrap(), "public msg");
            assert!(logs[0].get("html").is_some());

            assert_eq!(update["pub_state"].as_str().unwrap(), "pub_state_value");
            assert!(update["html"].as_str().is_some());
        }

        #[sqlx::test]
        async fn broadcast_filters_private_log_to_intended_player_only(pool: PgPool) {
            let p0 = make_user(&pool, "p0").await;
            let p1 = make_user(&pool, "p1").await;
            let game_id = make_two_player_game(&pool, &p0, &p1).await;
            let token0 = make_auth_token(&pool, p0.id).await;
            let token1 = make_auth_token(&pool, p1.id).await;

            // create_game_with_users shuffles slot order before assigning
            // positions, so look up p0's actual position rather than
            // assuming it landed on 0.
            let ge_before = db::find_game_extended(&pool, game_id)
                .await
                .unwrap()
                .unwrap();
            let p0_position = ge_before
                .game_players
                .iter()
                .find(|p| p.user.as_ref().is_some_and(|u| u.id == p0.id))
                .unwrap()
                .game_player
                .position as usize;

            let base = time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            );
            db::create_game_logs(
                &pool,
                game_id,
                vec![
                    CliLog {
                        content: "public msg".to_string(),
                        at: base,
                        public: true,
                        to: vec![],
                    },
                    CliLog {
                        content: "secret to p0".to_string(),
                        at: base + time::Duration::seconds(1),
                        public: false,
                        to: vec![p0_position],
                    },
                ],
            )
            .await
            .unwrap();

            let ge = db::find_game_extended(&pool, game_id)
                .await
                .unwrap()
                .unwrap();
            let all_logs = db::get_all_game_logs(&pool, game_id).await.unwrap();

            let broadcaster = make_broadcaster().await;
            let client = redis::Client::open(
                std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            )
            .unwrap();
            let mut pubsub = client.get_async_pubsub().await.unwrap();
            // The legacy per-user channel ("user.<auth token id>") carries the
            // same filtered logs (from crate::db::get_game_logs) as the
            // Leptos-native "ws.<user id>" channel, just serialized
            // differently - exercising this channel covers the same
            // filtering logic that guards both.
            pubsub.subscribe(format!("user.{}", token0)).await.unwrap();
            pubsub.subscribe(format!("user.{}", token1)).await.unwrap();

            broadcaster
                .broadcast_game_update(&pool, &ge, &all_logs, &pub_render(), &player_renders_for(2))
                .await;

            let mut by_channel = std::collections::HashMap::new();
            for _ in 0..2 {
                let msg = timeout(Duration::from_secs(5), pubsub.on_message().next())
                    .await
                    .expect("timed out waiting for redis pub/sub message")
                    .expect("pub/sub stream ended unexpectedly");
                let channel = msg.get_channel_name().to_string();
                let payload: String = msg.get_payload().unwrap();
                by_channel.insert(channel, payload);
            }

            let p0_payload = &by_channel[&format!("user.{}", token0)];
            let p0_v: serde_json::Value = serde_json::from_str(p0_payload).unwrap();
            let p0_logs = p0_v["GameUpdate"]["game_logs"].as_array().unwrap();
            assert_eq!(
                p0_logs.len(),
                2,
                "the intended player must see both the public and the private log"
            );
            assert!(
                p0_logs
                    .iter()
                    .any(|l| l["game_log"]["body"] == "secret to p0")
            );

            let p1_payload = &by_channel[&format!("user.{}", token1)];
            let p1_v: serde_json::Value = serde_json::from_str(p1_payload).unwrap();
            let p1_logs = p1_v["GameUpdate"]["game_logs"].as_array().unwrap();
            assert_eq!(
                p1_logs.len(),
                1,
                "another player must not receive a log privately targeted at p0"
            );
            assert!(
                p1_logs
                    .iter()
                    .all(|l| l["game_log"]["body"] != "secret to p0"),
                "private log body leaked into another player's payload"
            );
            assert!(
                !p1_payload.contains("secret to p0"),
                "private log content must not appear anywhere in another player's payload"
            );
        }
    }
}
