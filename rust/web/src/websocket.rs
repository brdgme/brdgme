use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::game::server_fns::{GameViewData, GameLogEntry};

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
        extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
        response::IntoResponse,
    };
    use brdgme_cmd::api::{PlayerRender, PubRender};
    use futures_util::{sink::SinkExt, stream::StreamExt};
    use serde::Serialize;
    use time::PrimitiveDateTime;
    use tower_sessions::Session;

    use crate::auth::session::get_user_from_session;
    use crate::db::GameExtended;
    use crate::game::server_fns::{GameViewData, PlayerViewData, GameLogEntry};
    use crate::models::game::GameLog;

    // Legacy-compatible serialization structs matching the format the React frontend expects.
    // Field names and structure mirror the legacy rust/api ShowResponse.

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
        user_id: Uuid,
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
        game_extended.game_players.iter().map(|p| {
            use std::str::FromStr;
            brdgme_markup::Player {
                name: p.user.name.clone(),
                color: brdgme_color::Color::from_str(&p.game_player.color)
                    .unwrap_or(brdgme_color::WHITE),
            }
        }).collect()
    }

    fn render_markup(markup_str: &str, players: &[brdgme_markup::Player]) -> String {
        let (nodes, _) = brdgme_markup::from_string(markup_str).unwrap_or_else(|_| (vec![], ""));
        brdgme_markup::html(&brdgme_markup::transform(&nodes, players))
    }

    fn build_legacy_game_players(game_extended: &GameExtended) -> Vec<LegacyGamePlayerEntry> {
        game_extended.game_players.iter().map(|p| LegacyGamePlayerEntry {
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
            user: LegacyUser {
                id: p.user.id,
                created_at: p.user.created_at,
                updated_at: p.user.updated_at,
                name: p.user.name.clone(),
                pref_colors: p.user.pref_colors.clone(),
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
        }).collect()
    }

    fn build_legacy_game_logs(
        logs: &[GameLog],
        markup_players: &[brdgme_markup::Player],
    ) -> Vec<LegacyRenderedGameLog> {
        logs.iter().map(|log| LegacyRenderedGameLog {
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
        }).collect()
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
                game_logs: legacy_logs.iter().filter(|l| l.game_log.is_public).map(|l| LegacyRenderedGameLog {
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
                }).collect(),
                pub_state: Some(public_render.pub_state.clone()),
                html: pub_html,
                command_spec: None,
                chat: None,
            };

            let pub_payload = match serde_json::to_string(&LegacyGameUpdateMessage { game_update: public_response }) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to serialize public GameUpdate: {}", e);
                    return;
                }
            };
            self.publish(&format!("game.{}", game_id), &pub_payload).await;

            // Publish private per-user messages
            for gpe in &game_extended.game_players {
                let player_render = match player_renders.get(gpe.game_player.position as usize) {
                    Some(pr) => pr,
                    None => continue,
                };

                // Use the same filtering as the display path: public logs + logs targeted
                // to this specific player via game_log_targets.
                let player_logs = match crate::db::get_game_logs(pool, game_id, gpe.game_player.id).await {
                    Ok(logs) => logs,
                    Err(e) => {
                        tracing::error!("Failed to fetch logs for player {}: {}", gpe.game_player.id, e);
                        continue;
                    }
                };

                let player_legacy_logs: Vec<LegacyRenderedGameLog> = player_logs.iter().map(|log| LegacyRenderedGameLog {
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
                }).collect();

                let player_html = render_markup(&player_render.render, &markup_players);
                let legacy_gp = legacy_game_players.iter()
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

                let private_payload = match serde_json::to_string(&LegacyGameUpdateMessage { game_update: private_response }) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Failed to serialize private GameUpdate: {}", e);
                        continue;
                    }
                };

                // Look up auth tokens for this player's user_id
                let user_id = gpe.user.id;
                let token_ids = sqlx::query(
                    "SELECT id FROM user_auth_tokens WHERE user_id = $1"
                )
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
                            self.publish(&format!("user.{}", token_id), &private_payload).await;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch auth tokens for user {}: {}", user_id, e);
                    }
                }

                // Publish Leptos-specific update directly to the user's WS channel.
                let last_turn_at = gpe.game_player.last_turn_at;
                let log_entries: Vec<GameLogEntry> = player_logs.iter().map(|log| {
                    GameLogEntry {
                        body_html: render_markup(&log.body, &markup_players),
                        logged_at: log.logged_at,
                        is_new: log.created_at >= last_turn_at,
                    }
                }).collect();

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
                    players: game_extended.game_players.iter().map(|p| {
                        use std::str::FromStr;
                        let color = brdgme_color::Color::from_str(&p.game_player.color)
                            .unwrap_or(brdgme_color::WHITE)
                            .hex();
                        PlayerViewData {
                            name: p.user.name.clone(),
                            color,
                            rating: p.game_type_user.rating,
                            points: p.game_player.points.unwrap_or(0.0),
                            is_turn: p.game_player.is_turn,
                        }
                    }).collect(),
                    command_spec: player_render.command_spec.clone(),
                };

                let brdgme_msg = WebSocketMessage::BrdgmeUpdate(super::BrdgmeGameUpdate {
                    game_id,
                    game_view,
                    logs: log_entries,
                });
                if let Ok(brdgme_payload) = serde_json::to_string(&brdgme_msg) {
                    self.publish(&format!("ws.{}", user_id), &brdgme_payload).await;
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
        let (mut sender, _receiver) = socket.split();

        let mut pubsub = match broadcaster.client.get_async_pubsub().await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to get Redis pubsub connection: {}", e);
                return;
            }
        };

        if let Err(e) = pubsub.psubscribe("game.*").await {
            tracing::error!("Redis PSUBSCRIBE failed: {}", e);
            return;
        }

        if let Some(uid) = user_id {
            if let Err(e) = pubsub.subscribe(format!("ws.{}", uid)).await {
                tracing::error!("Redis SUBSCRIBE failed for ws.{}: {}", uid, e);
            }
        }

        let mut stream = pubsub.into_on_message();

        while let Some(msg) = stream.next().await {
            let payload: String = match msg.get_payload() {
                Ok(p) => p,
                Err(_) => continue,
            };
            if sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    }
}
