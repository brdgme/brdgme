#[cfg(feature = "ssr")]
use crate::error::internal;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSlot {
    pub name: String,
    pub bot_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentSummary {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: Uuid,
    pub name: String,
    pub type_name: String,
    pub opponents: Vec<OpponentSummary>,
    pub is_turn: bool,
    /// When `is_turn` last changed (trigger-maintained) - the "Next game"
    /// button targets the game waiting on the player the longest.
    pub is_turn_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingGameSummary {
    pub id: Uuid,
    pub type_name: String,
    pub players: Vec<String>,
    pub is_owner: bool,
    pub is_invitee_needing_accept: bool,
    pub is_ready_to_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedGameSummary {
    pub id: Uuid,
    pub type_name: String,
    pub players: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarGames {
    pub active: Vec<GameSummary>,
    pub pending: Vec<PendingGameSummary>,
    pub finished: Vec<FinishedGameSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameViewData {
    pub id: Uuid,
    pub version_id: Uuid,
    pub type_name: String,
    pub version_name: String,
    pub html: String,
    pub is_my_turn: bool,
    pub is_finished: bool,
    pub can_undo: bool,
    pub restarted_game_id: Option<Uuid>,
    /// The game this one was restarted from (reverse of `restarted_game_id`).
    pub previous_game_id: Option<Uuid>,
    pub is_2player: bool,
    pub players: Vec<PlayerViewData>,
    pub command_spec: Option<brdgme_game::command::Spec>,
    /// `--mk-player-{n}`/`--mk-player-{n}-contrast` var declarations for this
    /// game's players, in position order. Apply as an inline `style` on any
    /// container whose `html` (board or log) content uses the markup
    /// `mk-fg-player-{n}`/`mk-bg-player-{n}` classes.
    pub player_style: String,
    /// Whether the current viewer is an admin - gates admin-only actions
    /// like "Bump bot to play".
    pub viewer_is_admin: bool,
    /// None when the viewer is anonymous (public spectator perspective).
    pub viewer_user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerViewData {
    pub name: String,
    /// The player's `--mk-{slot}` colour slot token (e.g. "green") - never a
    /// resolved hex value, so display always follows the active theme.
    pub color: String,
    pub rating: i32,
    /// ELO change applied when the game finished; `None` until then (and
    /// always `None` for unrated/bot games).
    pub rating_change: Option<i32>,
    pub points: f32,
    /// 1-based placing for finished games (standard-competition ties); `None`
    /// otherwise.
    pub place: Option<i32>,
    pub is_turn: bool,
    pub is_bot: bool,
    /// Bot name (e.g. "medium"); `None` for humans. Drives the
    /// `(bot: bot_name)` suffix in the game-page player card.
    pub bot_name: Option<String>,
    /// None for bots. Drives the game-page add-friend affordance (#30 D3).
    pub user_id: Option<Uuid>,
    /// False when already friends or viewer has an outgoing request; hides
    /// the "Add friend" link in the game sidebar.
    pub can_add_friend: bool,
    /// Recent form (this game's game type only), oldest-to-newest. Empty
    /// for bots or players with no qualifying finished games (#29).
    pub form: Vec<crate::stats::FormResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameVersionInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTypeInfo {
    pub id: Uuid,
    pub name: String,
    pub player_counts: Vec<i32>,
    /// Complexity, 0.0 (light) to 5.0 (heavy), from game_types.weight.
    pub weight: f32,
    /// 1-2 sentence description; empty string renders nothing.
    pub blurb: String,
    pub versions: Vec<GameVersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLogEntry {
    pub body_html: String,
    pub logged_at: PrimitiveDateTime,
    pub is_new: bool,
}

/// Builds the active-game summaries for `user`, or an empty list if there is
/// no logged-in user - anonymous visitors hit pages that render
/// `SidebarMenu` (e.g. the homepage), and "not logged in" is a normal state
/// there, not an error.
#[cfg(feature = "ssr")]
async fn active_games_summary(
    user: Option<crate::auth::AuthUser>,
    pool: &sqlx::PgPool,
) -> Result<Vec<GameSummary>, ServerFnError> {
    let Some(user) = user else {
        return Ok(Vec::new());
    };

    crate::db::find_active_game_summaries(pool, user.id)
        .await
        .map_err(internal("get_active_games: find active games"))
}

#[server(GetSidebarGames, "/api")]
pub async fn get_sidebar_games() -> Result<SidebarGames, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user().await?;
    let Some(user) = user else {
        return Ok(SidebarGames {
            active: Vec::new(),
            pending: Vec::new(),
            finished: Vec::new(),
        });
    };

    let uid = user.id;
    let active = active_games_summary(Some(user), &pool).await?;
    let pending = crate::db::find_pending_game_summaries(&pool, uid)
        .await
        .map_err(internal("get_sidebar_games: pending"))?;
    let finished = crate::db::find_finished_game_summaries(&pool, uid)
        .await
        .map_err(internal("get_sidebar_games: finished"))?;
    Ok(SidebarGames {
        active,
        pending,
        finished,
    })
}

#[server(GetGameDetails, "/api")]
pub async fn get_game_details(game_id: Uuid) -> Result<GameViewData, ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::game::client;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(internal("get_game_details: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id));

    let render_resp = client::render(
        &http_client,
        &ge.game_version.uri,
        &ge.game_version.name,
        ge.game.game_state.clone(),
        player.map(|p| p.game_player.position as usize),
    )
    .await
    .map_err(internal("get_game_details: render game"))?;

    // Convert markup to HTML, semantically: colours stay symbolic (CSS
    // classes referencing `--mk-*` vars) rather than baked-in hex, so the
    // rendered board follows the viewer's active theme.
    let (nodes, _) = brdgme_markup::from_string(&render_resp.render)
        .map_err(internal("get_game_details: parse markup"))?;

    let html = brdgme_markup::html_class(&brdgme_markup::transform_semantic(
        &nodes,
        &ge.semantic_players(),
    ));
    let player_style = ge.player_style();

    let viewer_is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("get_game_details: check admin"))?;

    let human_user_ids: Vec<Uuid> = ge
        .game_players
        .iter()
        .filter_map(|p| p.user.as_ref().map(|u| u.id))
        .collect();
    let form_by_user = crate::stats::recent_form_for_game_type(
        &pool,
        &human_user_ids,
        ge.game_version.game_type_id,
        10,
    )
    .await
    .map_err(internal("get_game_details: recent form"))?;

    let mut hide_add_friend = std::collections::HashSet::new();
    for uid in &human_user_ids {
        if *uid != user.id
            && crate::db::should_hide_add_friend(&pool, user.id, *uid)
                .await
                .map_err(internal("get_game_details: friend status"))?
        {
            hide_add_friend.insert(*uid);
        }
    }

    let previous_game_id = crate::db::find_predecessor_game_id(&pool, game_id)
        .await
        .map_err(internal("get_game_details: predecessor"))?;

    Ok(GameViewData {
        id: ge.game.id,
        version_id: ge.game_version.id,
        type_name: ge.game_type.name,
        version_name: ge.game_version.name,
        html,
        is_my_turn: player.map(|p| p.game_player.is_turn).unwrap_or(false),
        is_finished: ge.game.is_finished,
        can_undo: player
            .and_then(|p| p.game_player.undo_game_state.as_ref())
            .is_some(),
        restarted_game_id: ge.game.restarted_game_id,
        previous_game_id,
        is_2player: ge.game_players.len() == 2,
        players: ge
            .game_players
            .iter()
            .map(|p| PlayerViewData {
                name: p.name().to_string(),
                color: p.slot().to_string(),
                rating: p.game_type_user.rating,
                rating_change: p.game_player.rating_change,
                points: p.game_player.points.unwrap_or(0.0),
                place: p.game_player.place,
                is_turn: p.game_player.is_turn,
                is_bot: p.game_bot.is_some(),
                bot_name: p.game_bot.as_ref().map(|b| b.bot_name.clone()),
                user_id: p.user.as_ref().map(|u| u.id),
                can_add_friend: p
                    .user
                    .as_ref()
                    .is_some_and(|u| !hide_add_friend.contains(&u.id)),
                form: p
                    .user
                    .as_ref()
                    .and_then(|u| form_by_user.get(&u.id).cloned())
                    .unwrap_or_default(),
            })
            .collect(),
        command_spec: render_resp.command_spec,
        player_style,
        viewer_is_admin,
        viewer_user_id: Some(user.id),
    })
}

/// Ok(None) = success. Ok(Some(message)) = the game rejected the command -
/// expected user-input feedback rendered inline by the command input (same
/// pattern as set_username), NOT a transport/server error.
#[server(SubmitCommand, "/api")]
pub async fn submit_command(
    game_id: Uuid,
    command: String,
) -> Result<Option<String>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let resend = expect_context::<Option<resend_rs::Resend>>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let position: i32 = sqlx::query_scalar!(
        "SELECT position FROM game_players WHERE game_id = $1 AND user_id = $2",
        game_id,
        user.id
    )
    .fetch_optional(&pool)
    .await
    .map_err(internal("submit_command: find player position"))?
    .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    let before = crate::db::find_game_extended(&pool, game_id)
        .await
        .ok()
        .flatten();

    match super::execute_command(
        &pool,
        &http_client,
        &broadcaster,
        &jetstream,
        game_id,
        position as usize,
        command,
    )
    .await
    {
        Ok(()) => {
            crate::email::notify::notify_game_emails(
                resend.as_ref(),
                &pool,
                &http_client,
                game_id,
                before,
            )
            .await;
            Ok(None)
        }
        Err(crate::game::ExecuteCommandError::UserError(msg)) => Ok(Some(msg)),
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}

#[server(GetAvailableGameTypes, "/api")]
pub async fn get_available_game_types() -> Result<Vec<GameTypeInfo>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let _ = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let game_types = crate::db::find_available_game_types(&pool)
        .await
        .map_err(internal("get_available_game_types: find game types"))?;

    Ok(game_types
        .into_iter()
        .map(|(gt, versions)| GameTypeInfo {
            id: gt.id,
            name: gt.name,
            player_counts: gt.player_counts,
            weight: gt.weight,
            blurb: gt.blurb,
            versions: versions
                .into_iter()
                .map(|gv| GameVersionInfo {
                    id: gv.id,
                    name: gv.name,
                })
                .collect(),
        })
        .collect())
}

#[cfg(feature = "ssr")]
pub(crate) struct CreateGameSeed<'a> {
    pub(crate) player_count: usize,
    pub(crate) creator_id: Uuid,
    pub(crate) opponent_ids: &'a [Uuid],
    pub(crate) opponent_emails: &'a [String],
    pub(crate) bot_slots: &'a [BotSlot],
    pub(crate) all_accepted: bool,
}

/// Requests a fresh game from the game service and creates it (game row,
/// players, logs) within the caller's transaction. Deliberately neither
/// begins/commits the transaction nor broadcasts: `restart_game` must keep
/// the new game atomic with its `restarted_game_id` write, so callers own
/// the commit and the post-commit notifications.
#[cfg(feature = "ssr")]
pub(crate) async fn create_game_from_service(
    tx: &mut sqlx::PgConnection,
    http_client: &reqwest::Client,
    game_version: &crate::models::game::GameVersion,
    seed: CreateGameSeed<'_>,
) -> Result<crate::models::game::Game, ServerFnError> {
    use crate::db::CreateGameOpts;
    use crate::game::client;
    use brdgme_cmd::api::{Request, Response};

    let resp = client::request(
        http_client,
        &game_version.uri,
        &game_version.name,
        &Request::New {
            players: seed.player_count,
            seed: None,
        },
    )
    .await
    .map_err(internal("create_game_from_service: request new game"))?;

    let (game_info, logs) = match resp {
        Response::New { game, logs, .. } => (game, logs),
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    let status = crate::game::status_fields(game_info.status);

    let game = crate::db::create_game_with_users_tx(
        &mut *tx,
        CreateGameOpts {
            game_version_id: game_version.id,
            whose_turn: &status.whose_turn,
            eliminated: &status.eliminated,
            placings: &status.placings,
            points: &game_info.points,
            creator_id: seed.creator_id,
            opponent_ids: seed.opponent_ids,
            opponent_emails: seed.opponent_emails,
            bot_slots: seed.bot_slots,
            chat_id: None,
            game_state: &game_info.state,
            all_accepted: seed.all_accepted,
        },
    )
    .await
    .map_err(internal("create_game_from_service: create game"))?;

    crate::db::insert_game_logs_tx(&mut *tx, game.id, logs)
        .await
        .map_err(internal("create_game_from_service: create game logs"))?;

    Ok(game)
}

#[cfg(feature = "ssr")]
pub(crate) fn roster_error(player_counts: &[i32], player_count: usize) -> Option<String> {
    if player_counts.contains(&(player_count as i32)) {
        return None;
    }
    let counts = player_counts
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "This game supports {counts} players, but the request has {player_count} (including you)"
    ))
}

#[server(GenerateBotName, "/api")]
pub async fn generate_bot_name() -> Result<String, ServerFnError> {
    Ok(petname::petname(1, "-").unwrap_or_else(|| "Bot".to_string()))
}

#[server(GetAvailableBots, "/api")]
pub async fn get_available_bots() -> Result<Vec<String>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let _ = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let bots = crate::db::find_enabled_bots(&pool)
        .await
        .map_err(internal("get_available_bots: find enabled bots"))?;

    Ok(bots)
}

#[server(GetGameLogs, "/api")]
pub async fn get_game_logs(game_id: Uuid) -> Result<Vec<GameLogEntry>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(internal("get_game_logs: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    let last_turn_at = player.game_player.last_turn_at;
    let game_player_id = player.game_player.id;

    let logs = crate::db::get_game_logs(&pool, game_id, game_player_id)
        .await
        .map_err(internal("get_game_logs: load logs"))?;

    let semantic_players = ge.semantic_players();

    let entries = logs
        .into_iter()
        .map(|log| {
            let (nodes, _) = brdgme_markup::from_string(&log.body).unwrap_or_else(|_| (vec![], ""));
            let body_html = brdgme_markup::html_class(&brdgme_markup::transform_semantic(
                &nodes,
                &semantic_players,
            ));
            let is_new = log.created_at >= last_turn_at;
            GameLogEntry {
                body_html,
                logged_at: log.logged_at,
                is_new,
            }
        })
        .collect();

    Ok(entries)
}

#[server(MarkRead, "/api")]
pub async fn mark_read(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::db::mark_game_read(&pool, game_id, user.id)
        .await
        .map_err(internal("mark_read: mark game read"))
}

#[server(UndoGame, "/api")]
pub async fn undo_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::game::client;
    use crate::websocket::GameBroadcaster;
    use brdgme_cmd::api::{Request, Response};
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let resend = expect_context::<Option<resend_rs::Resend>>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(internal("undo_game: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;
    let before = ge.clone();

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    let undo_state = player
        .game_player
        .undo_game_state
        .clone()
        .ok_or_else(|| ServerFnError::new("No undo state available"))?;

    let resp = client::request(
        &http_client,
        &ge.game_version.uri,
        &ge.game_version.name,
        &Request::Status {
            game: undo_state.clone(),
        },
    )
    .await
    .map_err(internal("undo_game: fetch status from game service"))?;

    let game_response = match resp {
        Response::Status { game, .. } => game,
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    let status = crate::game::status_fields(game_response.status);

    crate::db::undo_game(
        &pool,
        game_id,
        &undo_state,
        player.game_player.position as usize,
        &status,
    )
    .await
    .map_err(internal("undo_game: apply undo"))?;

    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game_id).await;

    crate::email::notify::notify_game_emails(
        resend.as_ref(),
        &pool,
        &http_client,
        game_id,
        Some(before),
    )
    .await;
    Ok(())
}

#[server(ConcedeGame, "/api")]
pub async fn concede_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let resend = expect_context::<Option<resend_rs::Resend>>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(internal("concede_game: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;
    let before = ge.clone();

    if ge.game.is_finished {
        return Err(ServerFnError::new("Game is already finished"));
    }
    if ge.game_players.len() != 2 {
        return Err(ServerFnError::new(
            "Concede is only available in 2-player games",
        ));
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    crate::db::concede_game(&pool, game_id, player.game_player.id, player.name())
        .await
        .map_err(internal("concede_game: concede"))?;

    broadcaster.broadcast_game_update(game_id).await;

    crate::email::notify::notify_game_emails(
        resend.as_ref(),
        &pool,
        &http_client,
        game_id,
        Some(before),
    )
    .await;
    Ok(())
}

/// The restart flow minus the leptos context plumbing and post-commit
/// broadcasts, so tests can drive it against a mock game service. A restart
/// opens a proposal carrying the old roster (owner and bots accepted, humans
/// pending); a solo-vs-bots restart bypasses the proposal and creates the game
/// directly, atomically linking `restarted_game_id`.
#[cfg(feature = "ssr")]
pub(crate) async fn restart_game_impl(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    user_id: Uuid,
    game_id: Uuid,
) -> Result<crate::proposals::ProposalOutcome, ServerFnError> {
    let ge = crate::db::find_game_extended(pool, game_id)
        .await
        .map_err(internal("restart_game: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    if !ge.game.is_finished {
        return Err(ServerFnError::new("Game is not finished"));
    }
    if ge.game.restarted_game_id.is_some() {
        return Err(ServerFnError::new("Game has already been restarted"));
    }
    if !ge
        .game_players
        .iter()
        .any(|p| p.user.as_ref().is_some_and(|u| u.id == user_id))
    {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    // If a newer, non-deprecated version of this game type exists, restart
    // onto that version rather than the (possibly deprecated) version the
    // finished game was played on. Falls back to the original version if
    // none is found.
    let restart_game_version =
        crate::db::find_latest_non_deprecated_game_version(pool, ge.game_version.game_type_id)
            .await
            .map_err(internal("restart_game: find latest game version"))?
            .unwrap_or_else(|| ge.game_version.clone());

    let opponent_ids: Vec<Uuid> = ge
        .game_players
        .iter()
        .filter_map(|p| p.user.as_ref().filter(|u| u.id != user_id).map(|u| u.id))
        .collect();
    let bot_slots: Vec<BotSlot> = ge
        .game_players
        .iter()
        .filter_map(|p| {
            p.game_bot.as_ref().map(|b| BotSlot {
                name: b.name.clone(),
                bot_name: b.bot_name.clone(),
            })
        })
        .collect();
    let player_count = ge.game_players.len();

    let player_counts = crate::db::find_game_type_player_counts(pool, restart_game_version.id)
        .await
        .map_err(internal("restart_game: find player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;
    if let Some(msg) = roster_error(&player_counts, player_count) {
        return Err(ServerFnError::new(msg));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("restart_game: begin transaction"))?;

    // #30: a restart re-attaches every human player, so the same policy /
    // block rules apply as for a fresh game.
    let violations = crate::db::check_invite_policy_tx(&mut tx, user_id, &opponent_ids, &[])
        .await
        .map_err(internal("restart_game: check invite policy"))?;
    if let Some(msg) = violations.into_iter().next() {
        return Err(ServerFnError::new(msg));
    }

    if opponent_ids.is_empty() {
        let new_game = create_game_from_service(
            &mut tx,
            http_client,
            &restart_game_version,
            CreateGameSeed {
                player_count,
                creator_id: user_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &bot_slots,
                all_accepted: false,
            },
        )
        .await?;

        sqlx::query!(
            "UPDATE games SET restarted_game_id = $1, updated_at = NOW() WHERE id = $2",
            new_game.id,
            game_id
        )
        .execute(&mut *tx)
        .await
        .map_err(internal("restart_game: link restarted game"))?;

        tx.commit()
            .await
            .map_err(internal("restart_game: commit transaction"))?;

        return Ok(crate::proposals::ProposalOutcome {
            proposal_id: None,
            game_id: Some(new_game.id),
        });
    }

    let proposal_id =
        crate::proposals::insert_proposal(&mut tx, restart_game_version.id, user_id, Some(game_id))
            .await
            .map_err(internal("restart_game: insert proposal"))?;

    let mut position = 0;
    crate::proposals::insert_proposal_player(
        &mut tx,
        proposal_id,
        position,
        Some(user_id),
        None,
        None,
        "accepted",
        None,
    )
    .await
    .map_err(internal("restart_game: insert owner"))?;
    position += 1;

    for uid in &opponent_ids {
        let token = Uuid::new_v4().simple().to_string();
        crate::proposals::insert_proposal_player(
            &mut tx,
            proposal_id,
            position,
            Some(*uid),
            None,
            None,
            "pending",
            Some(token),
        )
        .await
        .map_err(internal("restart_game: insert invitee"))?;
        position += 1;
    }

    for bot in &bot_slots {
        crate::proposals::insert_proposal_player(
            &mut tx,
            proposal_id,
            position,
            None,
            Some(bot.name.clone()),
            Some(bot.bot_name.clone()),
            "accepted",
            None,
        )
        .await
        .map_err(internal("restart_game: insert bot"))?;
        position += 1;
    }

    tx.commit()
        .await
        .map_err(internal("restart_game: commit transaction"))?;

    Ok(crate::proposals::ProposalOutcome {
        proposal_id: Some(proposal_id),
        game_id: None,
    })
}

#[server(RestartGame, "/api")]
pub async fn restart_game(
    game_id: Uuid,
) -> Result<crate::proposals::ProposalOutcome, ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::proposals::InviteMailer;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let resend = expect_context::<Option<resend_rs::Resend>>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let outcome = restart_game_impl(&pool, &http_client, user.id, game_id).await?;

    if let Some(gid) = outcome.game_id {
        crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, gid).await;
        crate::email::notify::notify_game_emails(resend.as_ref(), &pool, &http_client, gid, None)
            .await;
    }

    if let Some(pid) = outcome.proposal_id {
        broadcaster.broadcast_proposal_update(pid).await;
        if let Ok(players) = crate::proposals::find_proposal_players(&pool, pid).await {
            for p in players
                .iter()
                .filter(|p| p.user_id.is_some() && p.response == "pending")
            {
                crate::proposals::mailer().send_invite(
                    pid,
                    p.user_id.unwrap(),
                    p.email_token.clone(),
                );
            }
        }
    }

    // Refresh the old game's view (restarted link / proposal banner).
    broadcaster.broadcast_game_update(game_id).await;

    Ok(outcome)
}

#[server(BumpBotTurns, "/api")]
pub async fn bump_bot_turns(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // Only players in the game can bump bots.
    let is_player = crate::db::is_player_in_game(&pool, game_id, user.id)
        .await
        .map_err(internal("bump_bot_turns: check player"))?;
    if !is_player {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    let is_admin = crate::db::is_user_admin(&pool, user.id)
        .await
        .map_err(internal("bump_bot_turns: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    crate::game::trigger_bot_turns(&pool, &jetstream, game_id).await;
    Ok(())
}

/// Admin-only hard delete, minus leptos context plumbing so tests can drive
/// it. Admins need not be players in the game.
#[cfg(feature = "ssr")]
async fn force_delete_game_impl(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    game_id: Uuid,
) -> Result<(), ServerFnError> {
    let is_admin = crate::db::is_user_admin(pool, user_id)
        .await
        .map_err(internal("force_delete_game: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    let deleted = crate::db::delete_game(pool, game_id)
        .await
        .map_err(internal("force_delete_game: delete game"))?;
    if !deleted {
        return Err(ServerFnError::new("Game not found"));
    }
    Ok(())
}

#[server(ForceDeleteGame, "/api")]
pub async fn force_delete_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    force_delete_game_impl(&pool, user.id, game_id).await?;

    // Spec D3: broadcast the usual game-update signal so open clients
    // refresh (their refetch will surface "Game not found"). No bot trigger.
    broadcaster.broadcast_game_update(game_id).await;
    Ok(())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn make_user(pool: &PgPool, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3)",
            id,
            name,
            &Vec::<String>::new()
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    async fn make_game_version(pool: &PgPool) -> Uuid {
        make_game_version_at(pool, "http://127.0.0.1:8100").await
    }

    async fn make_game_version_at(pool: &PgPool, uri: &str) -> Uuid {
        let game_type_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO game_types (id, name, player_counts) VALUES ($1, $2, $3)",
            game_type_id,
            "Test Game",
            &vec![2i32]
        )
        .execute(pool)
        .await
        .unwrap();
        let game_version_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO game_versions (id, game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, $2, $3, $4, true, false)",
            game_version_id,
            game_type_id,
            "v1",
            uri
        )
        .execute(pool)
        .await
        .unwrap();
        game_version_id
    }

    /// A finished two-player game (placings set, `restarted_game_id` NULL)
    /// whose game version points at `uri`. Returns `(game_id, creator_id)`.
    async fn make_finished_two_player_game(pool: &PgPool, uri: &str) -> (Uuid, Uuid) {
        let creator = make_user(pool, "creator").await;
        let opponent = make_user(pool, "opponent").await;
        let game_version_id = make_game_version_at(pool, uri).await;
        let game = crate::db::create_game_with_users(
            pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[1, 2],
                points: &[1.0, 0.0],
                creator_id: creator,
                opponent_ids: &[opponent],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "final_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        (game.id, creator)
    }

    /// A finished solo-vs-bots game (1 human + 1 bot) whose game version points
    /// at `uri`. Returns `(game_id, creator_id)`.
    async fn make_finished_solo_bot_game(pool: &PgPool, uri: &str) -> (Uuid, Uuid) {
        let creator = make_user(pool, "creator").await;
        let game_version_id = make_game_version_at(pool, uri).await;
        let game = crate::db::create_game_with_users(
            pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[1, 2],
                points: &[1.0, 0.0],
                creator_id: creator,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Botty".to_string(),
                    bot_name: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "final_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        (game.id, creator)
    }

    // Anonymous visitors hit pages that render SidebarMenu (e.g. the
    // homepage) before logging in; that must not surface as a 500.
    #[sqlx::test]
    async fn active_games_summary_returns_empty_for_anonymous_user(pool: PgPool) {
        let summaries = active_games_summary(None, &pool).await.unwrap();
        assert!(summaries.is_empty());
    }

    // Regression test for a hard-load of a bot game's page: the LEFT JOINed
    // bot player (NULL user_id) must not trip the summary query/mapping.
    #[sqlx::test]
    async fn active_games_summary_includes_bot_opponent(pool: PgPool) {
        let user_id = make_user(&pool, "human").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Botty".to_string(),
                    bot_name: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let user = crate::auth::AuthUser {
            id: user_id,
            name: "human".to_string(),
            email: "human@example.com".to_string(),
        };
        let summaries = active_games_summary(Some(user), &pool).await.unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, game.id);
        assert_eq!(summaries[0].opponents.len(), 1);
        assert_eq!(summaries[0].opponents[0].name, "Botty");
    }

    // Pins the sidebar sort order: my-turn games first, then most recently
    // updated. Single-player games so whose_turn position 0 is always the
    // creator (player order is shuffled in multi-slot games).
    #[sqlx::test]
    async fn active_games_summary_sorts_my_turn_first_then_updated_at_desc(pool: PgPool) {
        let user_id = make_user(&pool, "human").await;
        let game_version_id = make_game_version(&pool).await;

        let make_game = |whose_turn: &'static [usize]| {
            crate::db::create_game_with_users(
                &pool,
                crate::db::CreateGameOpts {
                    game_version_id,
                    whose_turn,
                    eliminated: &[],
                    placings: &[],
                    points: &[],
                    creator_id: user_id,
                    opponent_ids: &[],
                    opponent_emails: &[],
                    bot_slots: &[],
                    chat_id: None,
                    game_state: "state",
                    all_accepted: false,
                },
            )
        };

        // (a) not their turn, updated recently
        let game_a = make_game(&[]).await.unwrap();
        // (b) their turn, updated long ago
        let game_b = make_game(&[0]).await.unwrap();
        // (c) their turn, updated recently (creation timestamp left as-is)
        let game_c = make_game(&[0]).await.unwrap();

        // The update_games_updated_at trigger overwrites updated_at on every
        // UPDATE; disable it so the backdated values stick.
        sqlx::raw_sql("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE games SET updated_at = timezone('utc', now()) - interval '1 hour' WHERE id = $1",
            game_a.id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE games SET updated_at = timezone('utc', now()) - interval '10 days' WHERE id = $1",
            game_b.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let user = crate::auth::AuthUser {
            id: user_id,
            name: "human".to_string(),
            email: "human@example.com".to_string(),
        };
        let summaries = active_games_summary(Some(user), &pool).await.unwrap();

        let ids: Vec<Uuid> = summaries.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![game_c.id, game_b.id, game_a.id]);
        assert!(summaries[0].is_turn);
        assert!(summaries[1].is_turn);
        assert!(!summaries[2].is_turn);
    }

    // The requesting user must never be listed among their own opponents;
    // every other human and bot must be, with the bot named from
    // game_bots.name.
    #[sqlx::test]
    async fn active_games_summary_excludes_self_from_opponents(pool: PgPool) {
        let user_id = make_user(&pool, "alice").await;
        let opponent_id = make_user(&pool, "bob").await;
        let game_version_id = make_game_version(&pool).await;
        crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[opponent_id],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Botty".to_string(),
                    bot_name: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let user = crate::auth::AuthUser {
            id: user_id,
            name: "alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        let summaries = active_games_summary(Some(user), &pool).await.unwrap();

        assert_eq!(summaries.len(), 1);
        let mut opponent_names: Vec<&str> = summaries[0]
            .opponents
            .iter()
            .map(|o| o.name.as_str())
            .collect();
        opponent_names.sort();
        assert_eq!(opponent_names, vec!["Botty", "bob"]);
    }

    // Restarting a finished two-player (human) game opens a proposal carrying
    // the old roster; the old game stays finished and unlinked until the
    // proposal starts, and no new game row is created yet.
    #[sqlx::test]
    async fn restart_game_with_human_opponent_creates_a_proposal(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        let http_client = reqwest::Client::new();

        let outcome = restart_game_impl(&pool, &http_client, creator_id, game_id)
            .await
            .unwrap();

        assert!(outcome.game_id.is_none());
        let proposal_id = outcome.proposal_id.expect("proposal created");

        let proposal = crate::proposals::find_proposal(&pool, proposal_id)
            .await
            .unwrap()
            .expect("proposal row exists");
        assert_eq!(proposal.status, "open");
        assert_eq!(proposal.restarted_game_id, Some(game_id));
        assert_eq!(proposal.owner_user_id, creator_id);

        let players = crate::proposals::find_proposal_players(&pool, proposal_id)
            .await
            .unwrap();
        assert_eq!(players.len(), 2);
        let owner = players
            .iter()
            .find(|p| p.user_id == Some(creator_id))
            .expect("owner row");
        assert_eq!(owner.response, "accepted");
        let pending: Vec<_> = players
            .iter()
            .filter(|p| p.user_id.is_some() && p.user_id != Some(creator_id))
            .collect();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].response, "pending");

        let old_ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(old_ge.game.restarted_game_id, None);

        let games_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(games_count, 1);
    }

    // A failed game service call on the solo-vs-bots bypass must leave no
    // orphan game row and keep the old game restartable (restarted_game_id
    // NULL).
    #[sqlx::test]
    async fn restart_game_failed_service_call_leaves_no_new_game(pool: PgPool) {
        use brdgme_cmd::api::Response;

        let uri = crate::game::tests::spawn_mock_game_service(|_req| Response::UserError {
            message: "nope".to_string(),
        })
        .await;
        let (game_id, creator_id) = make_finished_solo_bot_game(&pool, &uri).await;
        let http_client = reqwest::Client::new();

        let result = restart_game_impl(&pool, &http_client, creator_id, game_id).await;
        assert!(result.is_err());

        let games_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(games_count, 1);

        let old_ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(old_ge.game.restarted_game_id, None);
    }

    #[sqlx::test]
    async fn force_delete_game_rejects_non_admin(pool: PgPool) {
        let user_id = make_user(&pool, "notadmin").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let result = force_delete_game_impl(&pool, user_id, game.id).await;
        assert!(result.is_err());
        // Game must still exist.
        assert!(
            crate::db::find_game(&pool, game.id)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[sqlx::test]
    async fn force_delete_game_deletes_for_admin(pool: PgPool) {
        let admin_id = make_user(&pool, "admin").await;
        sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin_id)
            .execute(&pool)
            .await
            .unwrap();
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: admin_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        force_delete_game_impl(&pool, admin_id, game.id)
            .await
            .unwrap();
        assert!(
            crate::db::find_game(&pool, game.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    // Regression: force-deleting a game that a proposal references via
    // started_game_id (or restarted_game_id) used to fail the
    // game_proposals FK and abort the delete. The links must be nulled so
    // the delete succeeds and the proposal history survives.
    #[sqlx::test]
    async fn force_delete_game_deletes_game_with_proposal_references(pool: PgPool) {
        let admin_id = make_user(&pool, "admin3").await;
        sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin_id)
            .execute(&pool)
            .await
            .unwrap();
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: admin_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let started_proposal_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status, started_game_id)
             VALUES ($1, $2, 'started', $3) RETURNING id",
        )
        .bind(game_version_id)
        .bind(admin_id)
        .bind(game.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_proposal_players (proposal_id, position, user_id, response)
             VALUES ($1, 0, $2, 'accepted')",
        )
        .bind(started_proposal_id)
        .bind(admin_id)
        .execute(&pool)
        .await
        .unwrap();
        let restart_proposal_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status, restarted_game_id)
             VALUES ($1, $2, 'open', $3) RETURNING id",
        )
        .bind(game_version_id)
        .bind(admin_id)
        .bind(game.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        force_delete_game_impl(&pool, admin_id, game.id)
            .await
            .unwrap();

        assert!(
            crate::db::find_game(&pool, game.id)
                .await
                .unwrap()
                .is_none()
        );

        let started_ref: Option<Uuid> =
            sqlx::query_scalar("SELECT started_game_id FROM game_proposals WHERE id = $1")
                .bind(started_proposal_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(started_ref, None);
        let restarted_ref: Option<Uuid> =
            sqlx::query_scalar("SELECT restarted_game_id FROM game_proposals WHERE id = $1")
                .bind(restart_proposal_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(restarted_ref, None);

        let player_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_proposal_players WHERE proposal_id = $1")
                .bind(started_proposal_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(player_count, 1);
    }

    #[sqlx::test]
    async fn force_delete_game_missing_game_errors(pool: PgPool) {
        let admin_id = make_user(&pool, "admin2").await;
        sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin_id)
            .execute(&pool)
            .await
            .unwrap();
        let result = force_delete_game_impl(&pool, admin_id, Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[test]
    fn roster_error_accepts_supported_counts() {
        assert_eq!(roster_error(&[2, 3, 4], 2), None);
        assert_eq!(roster_error(&[2, 3, 4], 3), None);
        assert_eq!(roster_error(&[2, 3, 4], 4), None);
    }

    #[test]
    fn roster_error_rejects_unsupported_counts() {
        let err = roster_error(&[2, 3, 4], 5).expect("5 players rejected");
        assert!(err.contains("2, 3, 4"), "message lists counts: {err}");
        assert!(err.contains('5'), "message names the bad count: {err}");
        // Non-contiguous counts: the gap is rejected.
        assert!(roster_error(&[2, 4], 3).is_some());
        // Solo (no opponents) rejected when unsupported.
        assert!(roster_error(&[2, 3, 4], 1).is_some());
    }
}
