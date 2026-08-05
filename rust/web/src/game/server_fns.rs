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
    /// The open restart proposal for this game, if any. While present, a
    /// restart is already in flight; the UI shows a "Restart invite pending"
    /// link to `/invites/{id}` instead of offering a fresh restart.
    pub restart_proposal_id: Option<Uuid>,
    pub is_2player: bool,
    /// Whether the viewer may concede: the viewer is an active human in an
    /// unfinished game with >=2 active humans, and either an eligible
    /// replacement bot exists or the game has exactly 2 total seats for the
    /// platform forfeit. Exactly one active human cannot Concede (#47, dual
    /// result model).
    pub can_concede: bool,
    /// Whether the viewer may end the game: an unfinished game with exactly
    /// one active human (only that active actor) or zero active humans (only
    /// human participants tied in the latest departure event) (#47, dual result
    /// model).
    pub can_end_game: bool,
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
    /// True when a human conceded/was replaced and a bot now plays for them:
    /// both a user and a game bot are present. The card keeps the human's
    /// name/link and adds a `(bot: ...)` suffix (#47).
    pub is_replaced: bool,
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
pub struct RestartPrefill {
    pub game_type_name: String,
    pub version_id: Uuid,
    pub version_name: String,
    pub player_counts: Vec<i32>,
    pub opponents: Vec<PrefillSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefillSlot {
    pub user_id: Option<Uuid>,
    pub name: String,
    pub bot_name: Option<String>,
}

/// Outcome of a restart attempt. `Created` carries the normal proposal/game
/// outcome; `AlreadyRestarted` means a first restart already won the race - the
/// client prefers `game_id` (link to `/games/{id}`) else `proposal_id` (link to
/// `/invites/{id}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestartOutcome {
    Created(crate::proposals::ProposalOutcome),
    AlreadyRestarted {
        proposal_id: Option<Uuid>,
        game_id: Option<Uuid>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLogEntry {
    pub body_html: String,
    pub logged_at: PrimitiveDateTime,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicIndexGame {
    pub game_id: Uuid,
    pub type_name: String,
    pub version_name: String,
    pub html: String,
    pub player_style: String,
    pub player_names: Vec<String>,
    pub logs: Vec<GameLogEntry>,
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

    if player.is_none()
        && !crate::db::is_game_visible_to_user(&pool, game_id, user.id)
            .await
            .map_err(internal("get_game_details: visibility"))?
    {
        return Err(ServerFnError::new("Game not found"));
    }

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
    let nodes = brdgme_markup::from_string(&render_resp.render)
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
        5,
    )
    .await
    .map_err(internal("get_game_details: recent form"))?;

    let other_human_ids: Vec<Uuid> = human_user_ids
        .iter()
        .copied()
        .filter(|uid| *uid != user.id)
        .collect();
    let hide_add_friend = crate::db::should_hide_add_friend_many(&pool, user.id, &other_human_ids)
        .await
        .map_err(internal("get_game_details: friend status"))?;

    let previous_game_id = crate::db::find_predecessor_game_id(&pool, game_id)
        .await
        .map_err(internal("get_game_details: predecessor"))?;

    let restart_proposal_id = crate::db::find_open_restart_proposal(&pool, game_id)
        .await
        .map_err(internal("get_game_details: restart proposal"))?;

    let replacement_available = crate::db::replacement_bot_available(&pool)
        .await
        .map_err(internal("get_game_details: replacement available"))?;

    let can_concede =
        !ge.game.is_finished && concede_eligible(&ge, Some(user.id), replacement_available);

    let can_end_game = !ge.game.is_finished && end_eligible(&ge, Some(user.id));

    Ok(GameViewData {
        id: ge.game.id,
        version_id: ge.game_version.id,
        type_name: ge.game_type.name,
        version_name: ge.game_version.name,
        html,
        is_my_turn: player.map(|p| p.game_player.is_turn).unwrap_or(false),
        is_finished: ge.game.is_finished,
        can_undo: !ge.game.is_finished
            && player
                .and_then(|p| p.game_player.undo_game_state.as_ref())
                .is_some(),
        restarted_game_id: ge.game.restarted_game_id,
        previous_game_id,
        restart_proposal_id,
        is_2player: ge.game_players.len() == 2,
        can_concede,
        can_end_game,
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
                is_replaced: p.user.is_some() && p.game_bot.is_some(),
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

/// Spectator render of one game for the logged-out index, privacy-gated at
/// render time so a game that stops being publicly visible between selection
/// (find_public_index_game_id) and here is refused rather than leaked
/// (Unit B section 2a / D-render-race). position = None => pub_render.
#[cfg(feature = "ssr")]
pub(crate) async fn render_game_public(
    pool: &sqlx::PgPool,
    http: &reqwest::Client,
    game_id: Uuid,
) -> Result<Option<PublicIndexGame>, ServerFnError> {
    use crate::game::client;

    if !crate::db::is_game_publicly_visible(pool, game_id)
        .await
        .map_err(internal("render_game_public: visibility"))?
    {
        return Ok(None);
    }

    let ge = crate::db::find_game_extended(pool, game_id)
        .await
        .map_err(internal("render_game_public: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    let render_resp = client::render(
        http,
        &ge.game_version.uri,
        &ge.game_version.name,
        ge.game.game_state.clone(),
        None,
    )
    .await
    .map_err(internal("render_game_public: render game"))?;

    let nodes = brdgme_markup::from_string(&render_resp.render)
        .map_err(internal("render_game_public: parse markup"))?;
    let semantic_players = ge.semantic_players();
    let html = brdgme_markup::html_class(&brdgme_markup::transform_semantic(
        &nodes,
        &semantic_players,
    ));
    let player_style = ge.player_style();
    let player_names = ge
        .game_players
        .iter()
        .map(|p| p.name().to_string())
        .collect();

    let logs = crate::db::find_recent_game_log_lines(pool, game_id, 3)
        .await
        .map_err(internal("render_game_public: load logs"))?
        .into_iter()
        .map(|log| {
            let nodes = brdgme_markup::from_string(&log.body).unwrap_or_else(|e| {
                tracing::warn!(game_id = %game_id, log_id = %log.id, error = %e, "failed to parse log markup");
                vec![]
            });
            let body_html = brdgme_markup::html_class(&brdgme_markup::transform_semantic(
                &nodes,
                &semantic_players,
            ));
            GameLogEntry {
                body_html,
                logged_at: log.logged_at,
                is_new: false,
            }
        })
        .collect();

    Ok(Some(PublicIndexGame {
        game_id,
        type_name: ge.game_type.name,
        version_name: ge.game_version.name,
        html,
        player_style,
        player_names,
        logs,
    }))
}

#[cfg(feature = "ssr")]
pub(crate) async fn public_index_data(
    pool: &sqlx::PgPool,
    http: &reqwest::Client,
) -> Result<Option<PublicIndexGame>, ServerFnError> {
    let Some(game_id) = crate::db::find_public_index_game_id(pool)
        .await
        .map_err(internal("get_public_index: select game"))?
    else {
        return Ok(None);
    };
    render_game_public(pool, http, game_id).await
}

/// Anonymous (no auth guard): the selected public game's spectator render +
/// title + 3 recent public log lines for the logged-out index, or None when
/// no game qualifies (Unit B R2).
#[server(GetPublicIndex, "/api")]
pub async fn get_public_index() -> Result<Option<PublicIndexGame>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    public_index_data(&pool, &http_client).await
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
        Ok(before) => {
            crate::email::notify::notify_game_emails(
                resend.as_ref(),
                &pool,
                &http_client,
                game_id,
                Some(before),
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
    pub(crate) creator_id: Uuid,
    pub(crate) opponent_ids: &'a [Uuid],
    pub(crate) opponent_emails: &'a [crate::auth::email_addr::CanonicalEmail],
    pub(crate) bot_slots: &'a [BotSlot],
    pub(crate) all_accepted: bool,
}

#[cfg(feature = "ssr")]
pub(crate) struct FetchedGame {
    pub(crate) game_info: brdgme_cmd::api::GameResponse,
    pub(crate) logs: Vec<brdgme_cmd::api::CliLog>,
}

#[cfg(feature = "ssr")]
pub(crate) async fn fetch_game_from_service(
    http_client: &reqwest::Client,
    game_version: &crate::models::game::GameVersion,
    player_count: usize,
) -> Result<FetchedGame, ServerFnError> {
    use crate::game::client;
    use brdgme_cmd::api::{Request, Response};

    let resp = client::request(
        http_client,
        &game_version.uri,
        &game_version.name,
        &Request::New {
            players: player_count,
            seed: None,
        },
    )
    .await
    .map_err(internal("fetch_game_from_service: request new game"))?;

    let (game_info, logs) = match resp {
        Response::New { game, logs, .. } => (game, logs),
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    Ok(FetchedGame { game_info, logs })
}

#[cfg(feature = "ssr")]
pub(crate) async fn insert_game_from_service(
    tx: &mut sqlx::PgConnection,
    game_version_id: Uuid,
    seed: CreateGameSeed<'_>,
    fetched: FetchedGame,
) -> Result<crate::models::game::Game, ServerFnError> {
    use crate::db::CreateGameOpts;

    let status = crate::game::status_fields(fetched.game_info.status);

    let game = crate::db::create_game_with_users_tx(
        &mut *tx,
        CreateGameOpts {
            game_version_id,
            whose_turn: &status.whose_turn,
            eliminated: &status.eliminated,
            placings: &status.placings,
            points: &fetched.game_info.points,
            creator_id: seed.creator_id,
            opponent_ids: seed.opponent_ids,
            opponent_emails: seed.opponent_emails,
            bot_slots: seed.bot_slots,
            chat_id: None,
            game_state: &fetched.game_info.state,
            all_accepted: seed.all_accepted,
        },
    )
    .await
    .map_err(internal("insert_game_from_service: create game"))?;

    crate::db::insert_game_logs_tx(&mut *tx, game.id, fetched.logs)
        .await
        .map_err(internal("insert_game_from_service: create game logs"))?;

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

// Intentionally anonymous: generates a random bot name for the new-game form.
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
            let nodes = brdgme_markup::from_string(&log.body).unwrap_or_else(|e| {
                tracing::warn!(game_id = %game_id, log_id = %log.id, error = %e, "failed to parse log markup");
                vec![]
            });
            let body_html = brdgme_markup::html_class(&brdgme_markup::transform_semantic(
                &nodes,
                &semantic_players,
            ));
            let is_new = log.logged_at >= last_turn_at;
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

    let before = undo_core(&pool, &http_client, game_id, ActingPlayer::User(user.id)).await?;

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

#[cfg(feature = "ssr")]
fn count_active_humans(ge: &crate::db::GameExtended) -> usize {
    ge.game_players
        .iter()
        .filter(|p| p.game_player.user_id.is_some() && p.game_player.left_at.is_none())
        .count()
}

/// Whether `actor_user_id` is an active human in the snapshot: present and not
/// left. Pure-bot seats and departed/replaced humans are never active.
#[cfg(feature = "ssr")]
fn is_active_human(ge: &crate::db::GameExtended, actor_user_id: Uuid) -> bool {
    ge.game_players.iter().any(|p| {
        p.game_player.user_id.is_some_and(|uid| uid == actor_user_id)
            && p.game_player.left_at.is_none()
    })
}

/// Approved Concede actor/threshold rule: the actor is an active human and at
/// least two active humans remain. Exactly one active human cannot Concede.
#[cfg(feature = "ssr")]
fn concede_actor_eligible(ge: &crate::db::GameExtended, actor_user_id: Option<Uuid>) -> bool {
    actor_user_id.is_some_and(|uid| is_active_human(ge, uid)) && count_active_humans(ge) >= 2
}

/// Approved Concede snapshot eligibility: the actor is an active human, at
/// least two active humans remain, and either an eligible replacement bot
/// exists or the game has exactly two total seats for the platform forfeit.
#[cfg(feature = "ssr")]
fn concede_eligible(
    ge: &crate::db::GameExtended,
    actor_user_id: Option<Uuid>,
    replacement_available: bool,
) -> bool {
    concede_actor_eligible(ge, actor_user_id)
        && (replacement_available || ge.game_players.len() == 2)
}

/// Latest human departure-event sequence in the snapshot. Computed from human
/// participant rows only - pure bots are never latest-event authority.
#[cfg(feature = "ssr")]
fn latest_human_departure_sequence(ge: &crate::db::GameExtended) -> Option<i32> {
    ge.game_players
        .iter()
        .filter(|p| p.game_player.user_id.is_some())
        .filter_map(|p| p.game_player.departure_sequence)
        .max()
}

/// Whether `actor_user_id` is a human participant tied in the departure event
/// at `sequence`.
#[cfg(feature = "ssr")]
fn is_human_in_departure_event(
    ge: &crate::db::GameExtended,
    actor_user_id: Uuid,
    sequence: i32,
) -> bool {
    ge.game_players.iter().any(|p| {
        p.game_player.user_id.is_some_and(|uid| uid == actor_user_id)
            && p.game_player.departure_sequence == Some(sequence)
    })
}

/// Approved End snapshot eligibility. Exactly one active human authorizes only
/// that active actor; zero active humans authorizes only human participants
/// tied in the latest human departure event; two or more active humans are
/// rejected. Pure bots are never humans or latest-event authority. None of the
/// authorization inputs consult `place`, `points`, or timestamps.
#[cfg(feature = "ssr")]
fn end_eligible(ge: &crate::db::GameExtended, actor_user_id: Option<Uuid>) -> bool {
    let Some(actor_user_id) = actor_user_id else {
        return false;
    };
    match count_active_humans(ge) {
        n if n >= 2 => false,
        1 => is_active_human(ge, actor_user_id),
        _ => latest_human_departure_sequence(ge)
            .is_some_and(|seq| is_human_in_departure_event(ge, actor_user_id, seq)),
    }
}

#[cfg(feature = "ssr")]
pub(crate) enum ActingPlayer {
    User(Uuid),
    GamePlayer(Uuid),
}

#[cfg(feature = "ssr")]
fn conflict_or_internal(context: &'static str, e: anyhow::Error) -> ServerFnError {
    if e.downcast_ref::<crate::db::GameAlreadyFinished>().is_some() {
        return ServerFnError::new("Game is already finished");
    }
    if e.downcast_ref::<crate::db::StaleStateConflict>().is_some() {
        return ServerFnError::new(
            "The game changed while this was being processed; nothing was changed. Please try again.",
        );
    }
    if e.downcast_ref::<crate::db::NotEnoughActiveHumans>().is_some() {
        return ServerFnError::new(
            "Concede is not available: at least two active humans are required",
        );
    }
    internal(context)(e)
}

#[cfg(feature = "ssr")]
pub(crate) async fn undo_core(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: Uuid,
    actor: ActingPlayer,
) -> Result<crate::db::GameExtended, ServerFnError> {
    use brdgme_cmd::api::{Request, Response};

    let ge = crate::db::find_game_extended(pool, game_id)
        .await
        .map_err(internal("undo_core: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    let player = match &actor {
        ActingPlayer::User(user_id) => ge
            .game_players
            .iter()
            .find(|p| p.user.as_ref().is_some_and(|u| u.id == *user_id))
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?,
        ActingPlayer::GamePlayer(gp_id) => ge
            .game_players
            .iter()
            .find(|p| p.game_player.id == *gp_id)
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?,
    };

    if ge.game.is_finished {
        return Err(ServerFnError::new(
            "This game is finished and can no longer be undone.",
        ));
    }

    let undo_state = player
        .game_player
        .undo_game_state
        .clone()
        .ok_or_else(|| ServerFnError::new("No undo state available"))?;

    let resp = crate::game::client::request(
        http_client,
        &ge.game_version.uri,
        &ge.game_version.name,
        &Request::Status {
            game: undo_state.clone(),
        },
    )
    .await
    .map_err(internal("undo_core: fetch status from game service"))?;

    let game_response = match resp {
        Response::Status { game, .. } => game,
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    let status = crate::game::status_fields(game_response.status);

    crate::db::undo_game(
        pool,
        game_id,
        &undo_state,
        player.game_player.position as usize,
        &status,
        &game_response.points,
        player.game_player.id,
        ge.game.updated_at,
    )
    .await
    .map_err(|e| conflict_or_internal("undo_core: apply undo", e))?;

    Ok(ge)
}

#[cfg(feature = "ssr")]
pub(crate) async fn concede_core(
    pool: &sqlx::PgPool,
    game_id: Uuid,
    actor: ActingPlayer,
) -> Result<crate::db::GameExtended, ServerFnError> {
    let ge = crate::db::find_game_extended(pool, game_id)
        .await
        .map_err(internal("concede_core: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    if ge.game.is_finished {
        return Err(ServerFnError::new("Game is already finished"));
    }

    let player = match &actor {
        ActingPlayer::User(user_id) => ge
            .game_players
            .iter()
            .find(|p| p.user.as_ref().is_some_and(|u| u.id == *user_id))
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?,
        ActingPlayer::GamePlayer(gp_id) => ge
            .game_players
            .iter()
            .find(|p| p.game_player.id == *gp_id)
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?,
    };

    if player.game_player.left_at.is_some() {
        return Err(ServerFnError::new("You have already left this game"));
    }

    // DRM-03b2b: courtesy prechecks derive from the shared snapshot predicates;
    // the locked writers remain authoritative for concurrent changes. First the
    // approved Concede actor/threshold rule - exactly one active human cannot
    // Concede (End replaces it) - guarded before any replacement/forfeit
    // dispatch so web and email cannot replace the last human.
    let actor_user_id = match &actor {
        ActingPlayer::User(user_id) => Some(*user_id),
        ActingPlayer::GamePlayer(_) => player.game_player.user_id,
    };
    if !concede_actor_eligible(&ge, actor_user_id) {
        return Err(ServerFnError::new(
            "Concede is not available: at least two active humans are required",
        ));
    }

    let replacement_available = crate::db::replacement_bot_available(pool)
        .await
        .map_err(internal("concede_core: replacement available"))?;

    if !concede_eligible(&ge, actor_user_id, replacement_available) {
        return Err(ServerFnError::new(
            "Concede is not available: no replacement bot configured",
        ));
    }

    if replacement_available {
        crate::db::concede_game_replace(
            pool,
            game_id,
            player.game_player.id,
            player.name(),
            ge.game.updated_at,
        )
        .await
        .map_err(|e| conflict_or_internal("concede_core: replace", e))?;
    } else {
        crate::db::concede_game(
            pool,
            game_id,
            player.game_player.id,
            player.name(),
            ge.game.updated_at,
        )
        .await
        .map_err(|e| conflict_or_internal("concede_core: concede", e))?;
    }

    Ok(ge)
}

#[cfg(feature = "ssr")]
pub(crate) async fn end_core(
    pool: &sqlx::PgPool,
    game_id: Uuid,
    actor: ActingPlayer,
) -> Result<crate::db::GameExtended, ServerFnError> {
    let ge = crate::db::find_game_extended(pool, game_id)
        .await
        .map_err(internal("end_core: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    if ge.game.is_finished {
        return Err(ServerFnError::new("Game is already finished"));
    }

    let player = match &actor {
        ActingPlayer::User(user_id) => ge
            .game_players
            .iter()
            .find(|p| p.user.as_ref().is_some_and(|u| u.id == *user_id))
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?,
        ActingPlayer::GamePlayer(gp_id) => ge
            .game_players
            .iter()
            .find(|p| p.game_player.id == *gp_id)
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?,
    };

    // DRM-03b2b: courtesy precheck derives from the shared End snapshot
    // predicate (sole active actor, or zero-active latest-departure-tie human);
    // the locked writer remains authoritative for races.
    let actor_user_id = match &actor {
        ActingPlayer::User(user_id) => Some(*user_id),
        ActingPlayer::GamePlayer(_) => player.game_player.user_id,
    };
    if !end_eligible(&ge, actor_user_id) {
        return Err(ServerFnError::new(
            "End game is only available to the last human",
        ));
    }

    // The pre-write snapshot returned to the caller is the same `ge` the
    // writer's stale guard is checked against, so notifications diff the exact
    // state before this lifecycle write.
    crate::db::end_game(pool, game_id, ge.game.updated_at, player.game_player.id)
        .await
        .map_err(|e| conflict_or_internal("end_core: end", e))?;

    Ok(ge)
}

#[server(ConcedeGame, "/api")]
pub async fn concede_game(game_id: Uuid) -> Result<(), ServerFnError> {
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

    let before = concede_core(&pool, game_id, ActingPlayer::User(user.id)).await?;

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

#[server(EndGame, "/api")]
pub async fn end_game(game_id: Uuid) -> Result<(), ServerFnError> {
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

    let before = end_core(&pool, game_id, ActingPlayer::User(user.id)).await?;

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

/// Race-safe restart core shared by the web server fn and the email `restart`
/// command. Serializes concurrent restarts on the old game row (`FOR UPDATE`):
/// the first restart wins, a second (concurrent or later) gets
/// `AlreadyRestarted` linking to the winner's game (solo) or open proposal
/// (humans). An OPEN restart proposal counts as an in-flight restart because the
/// old->new game link is only written when the proposal STARTS. Caller owns
/// post-commit broadcast/notify.
#[cfg(feature = "ssr")]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn restart_core(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    user_id: Uuid,
    old_game_id: Uuid,
    version: &crate::models::game::GameVersion,
    opponent_ids: &[Uuid],
    opponent_emails: &[crate::auth::email_addr::CanonicalEmail],
    bot_slots: &[BotSlot],
) -> Result<RestartOutcome, ServerFnError> {
    let player_count = 1 + opponent_ids.len() + opponent_emails.len() + bot_slots.len();

    let player_counts = crate::db::find_game_type_player_counts(pool, version.id)
        .await
        .map_err(internal("restart_core: find player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;
    if let Some(msg) = roster_error(&player_counts, player_count) {
        return Err(ServerFnError::new(msg));
    }

    let mut bot_slots: Vec<BotSlot> = bot_slots.to_vec();
    let canonical_names = crate::db::validate_bot_slots(pool, &bot_slots)
        .await
        .map_err(internal("restart_core: validate bot slots"))?
        .map_err(ServerFnError::new)?;
    for (slot, canonical) in bot_slots.iter_mut().zip(canonical_names) {
        slot.bot_name = canonical;
    }

    let fetched = if opponent_ids.is_empty() && opponent_emails.is_empty() {
        Some(fetch_game_from_service(http_client, version, player_count).await?)
    } else {
        None
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("restart_core: begin transaction"))?;

    let row: Option<(bool, Option<Uuid>)> =
        sqlx::query_as("SELECT is_finished, restarted_game_id FROM games WHERE id = $1 FOR UPDATE")
            .bind(old_game_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(internal("restart_core: lock game"))?;
    let (is_finished, restarted_game_id) =
        row.ok_or_else(|| ServerFnError::new("Game not found"))?;

    if !crate::db::is_player_in_game(pool, old_game_id, user_id)
        .await
        .map_err(internal("restart_core: check membership"))?
    {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    if !is_finished {
        return Err(ServerFnError::new("Game is not finished"));
    }
    if let Some(new_game_id) = restarted_game_id {
        return Ok(RestartOutcome::AlreadyRestarted {
            proposal_id: None,
            game_id: Some(new_game_id),
        });
    }
    if let Some(pid) = crate::db::find_open_restart_proposal_tx(&mut tx, old_game_id)
        .await
        .map_err(internal("restart_core: find open restart proposal"))?
    {
        return Ok(RestartOutcome::AlreadyRestarted {
            proposal_id: Some(pid),
            game_id: None,
        });
    }

    let violations =
        crate::db::check_invite_policy_tx(&mut tx, user_id, opponent_ids, opponent_emails)
            .await
            .map_err(internal("restart_core: check invite policy"))?;
    if let Some(msg) = violations.into_iter().next() {
        return Err(ServerFnError::new(msg));
    }

    let mut human_invitees: Vec<Uuid> = opponent_ids.to_vec();
    for email in opponent_emails {
        human_invitees
            .push(crate::proposals::find_or_create_user_by_email_tx(&mut tx, email).await?);
    }

    let mut all = vec![user_id];
    all.extend(&human_invitees);
    all.sort();
    let before = all.len();
    all.dedup();
    if all.len() != before {
        return Err(ServerFnError::new(
            "Please ensure each player in the game is unique",
        ));
    }

    if human_invitees.is_empty() {
        let new_game = insert_game_from_service(
            &mut tx,
            version.id,
            CreateGameSeed {
                creator_id: user_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &bot_slots,
                all_accepted: false,
            },
            fetched.expect("fetched when no human invitees"),
        )
        .await?;

        sqlx::query("UPDATE games SET restarted_game_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_game.id)
            .bind(old_game_id)
            .execute(&mut *tx)
            .await
            .map_err(internal("restart_core: link restarted game"))?;

        tx.commit()
            .await
            .map_err(internal("restart_core: commit transaction"))?;

        return Ok(RestartOutcome::Created(crate::proposals::ProposalOutcome {
            proposal_id: None,
            game_id: Some(new_game.id),
        }));
    }

    let proposal_id =
        crate::proposals::insert_proposal(&mut tx, version.id, user_id, Some(old_game_id))
            .await
            .map_err(internal("restart_core: insert proposal"))?;

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
    .map_err(internal("restart_core: insert owner"))?;
    position += 1;

    for uid in &human_invitees {
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
        .map_err(internal("restart_core: insert invitee"))?;
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
        .map_err(internal("restart_core: insert bot"))?;
        position += 1;
    }

    tx.commit()
        .await
        .map_err(internal("restart_core: commit transaction"))?;

    Ok(RestartOutcome::Created(crate::proposals::ProposalOutcome {
        proposal_id: Some(proposal_id),
        game_id: None,
    }))
}

#[server(RestartGameWithRoster, "/api")]
pub async fn restart_game_with_roster(
    game_id: Uuid,
    game_version_id: Uuid,
    opponent_ids: Option<Vec<Uuid>>,
    opponent_emails: Option<Vec<String>>,
    bot_slots: Option<Vec<BotSlot>>,
) -> Result<RestartOutcome, ServerFnError> {
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

    let opponent_ids = opponent_ids.unwrap_or_default();
    let opponent_emails = opponent_emails.unwrap_or_default();
    let opponent_emails: Vec<crate::auth::email_addr::CanonicalEmail> = opponent_emails
        .into_iter()
        .map(|e| crate::auth::email_addr::canonicalize_email(&e))
        .collect();
    if opponent_emails
        .iter()
        .any(|e| e.is_empty() || !e.contains('@'))
    {
        return Err(ServerFnError::new("Invalid email address"));
    }
    let bot_slots = bot_slots.unwrap_or_default();

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(internal("restart_game_with_roster: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;
    if !ge.game.is_finished {
        return Err(ServerFnError::new("Game is not finished"));
    }
    if !ge
        .game_players
        .iter()
        .any(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
    {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    let version = crate::db::find_game_version(&pool, game_version_id)
        .await
        .map_err(internal("restart_game_with_roster: find game version"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;
    if version.game_type_id != ge.game_version.game_type_id {
        return Err(ServerFnError::new(
            "Game version does not match this game's type",
        ));
    }

    let outcome = restart_core(
        &pool,
        &http_client,
        user.id,
        game_id,
        &version,
        &opponent_ids,
        &opponent_emails,
        &bot_slots,
    )
    .await?;

    if let RestartOutcome::Created(ref created) = outcome {
        if let Some(gid) = created.game_id {
            crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, gid).await;
            crate::email::notify::notify_game_emails(
                resend.as_ref(),
                &pool,
                &http_client,
                gid,
                None,
            )
            .await;
        }
        if let Some(pid) = created.proposal_id {
            broadcaster.broadcast_proposal_update(pid).await;
            match crate::proposals::find_proposal_players(&pool, pid).await {
                Ok(players) => {
                    for (user_id, email_token) in players
                        .iter()
                        .filter(|p| p.response == "pending")
                        .filter_map(|p| p.user_id.map(|uid| (uid, p.email_token.clone())))
                    {
                        crate::proposals::mailer().send_invite(pid, user_id, email_token);
                    }
                }
                Err(e) => {
                    tracing::warn!(proposal_id = %pid, error = %e, "failed to fetch proposal players for restart invite emails");
                }
            }
        }
        broadcaster.broadcast_game_update(game_id).await;
    }

    Ok(outcome)
}

#[cfg(feature = "ssr")]
pub(crate) async fn get_restart_prefill_impl(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    game_id: Uuid,
) -> Result<RestartPrefill, ServerFnError> {
    let ge = crate::db::find_game_extended(pool, game_id)
        .await
        .map_err(internal("get_restart_prefill: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    if !ge.game.is_finished {
        return Err(ServerFnError::new("Game is not finished"));
    }
    if !ge
        .game_players
        .iter()
        .any(|p| p.user.as_ref().is_some_and(|u| u.id == user_id))
    {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    let version =
        crate::db::find_latest_non_deprecated_game_version(pool, ge.game_version.game_type_id)
            .await
            .map_err(internal("get_restart_prefill: find latest game version"))?
            .unwrap_or_else(|| ge.game_version.clone());

    let player_counts = crate::db::find_game_type_player_counts(pool, version.id)
        .await
        .map_err(internal("get_restart_prefill: find player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;

    let opponents = ge
        .game_players
        .iter()
        .filter(|p| !p.user.as_ref().is_some_and(|u| u.id == user_id))
        .map(|p| match (&p.user, &p.game_bot) {
            (Some(u), _) => PrefillSlot {
                user_id: Some(u.id),
                name: u.name.clone(),
                bot_name: None,
            },
            (None, Some(b)) => PrefillSlot {
                user_id: None,
                name: b.name.clone(),
                bot_name: Some(b.bot_name.clone()),
            },
            (None, None) => PrefillSlot {
                user_id: None,
                name: p.name().to_string(),
                bot_name: None,
            },
        })
        .collect();

    Ok(RestartPrefill {
        game_type_name: ge.game_type.name,
        version_id: version.id,
        version_name: version.name,
        player_counts,
        opponents,
    })
}

#[server(GetRestartPrefill, "/api")]
pub async fn get_restart_prefill(game_id: Uuid) -> Result<RestartPrefill, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    get_restart_prefill_impl(&pool, user.id, game_id).await
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

    /// Build the finished game's exact old roster (as the email `restart` command
    /// does) and drive `restart_core` with it.
    async fn restart_via_core(
        pool: &PgPool,
        http_client: &reqwest::Client,
        user_id: Uuid,
        game_id: Uuid,
    ) -> Result<RestartOutcome, ServerFnError> {
        let ge = crate::db::find_game_extended(pool, game_id)
            .await
            .unwrap()
            .unwrap();
        let version =
            crate::db::find_latest_non_deprecated_game_version(pool, ge.game_version.game_type_id)
                .await
                .unwrap()
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
        restart_core(
            pool,
            http_client,
            user_id,
            game_id,
            &version,
            &opponent_ids,
            &[],
            &bot_slots,
        )
        .await
    }

    /// Mock game service answering `New` with a valid active game for `players`,
    /// for restart tests that create a game directly (solo bypass).
    async fn spawn_ok_new_game_service() -> String {
        use brdgme_cmd::api::{GameResponse, PubRender, Request, Response};
        crate::game::tests::spawn_mock_game_service(move |req| {
            let players = match req {
                Request::New { players, .. } => players,
                _ => 0,
            };
            Response::New {
                game: GameResponse {
                    state: "mock_state".to_string(),
                    points: vec![0.0; players],
                    status: brdgme_game::Status::Active {
                        whose_turn: vec![0],
                        eliminated: vec![],
                    },
                },
                logs: vec![],
                public_render: PubRender {
                    pub_state: "pub".to_string(),
                    render: "mock render".to_string(),
                },
                player_renders: vec![],
                seed: 0,
            }
        })
        .await
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

    // Pins the sidebar sort order: my-turn games first (longest-waiting
    // first by is_turn_at ASC), then non-my-turn games (most recent own
    // turn first by last_turn_at DESC). Single-player games so whose_turn
    // position 0 is always the creator.
    #[sqlx::test]
    async fn active_games_summary_sorts_my_turn_first_then_by_turn_timestamps(pool: PgPool) {
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

        // (a) not their turn, own turn ended 30 min ago
        let game_a = make_game(&[]).await.unwrap();
        // (b) their turn, waiting 2 hours (longest)
        let game_b = make_game(&[0]).await.unwrap();
        // (c) their turn, waiting 1 hour
        let game_c = make_game(&[0]).await.unwrap();
        // (d) not their turn, own turn ended 3 hours ago
        let game_d = make_game(&[]).await.unwrap();

        sqlx::query(
            "UPDATE game_players SET is_turn_at = timezone('utc', now()) - interval '2 hours' WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game_b.id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET is_turn_at = timezone('utc', now()) - interval '1 hour' WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game_c.id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET last_turn_at = timezone('utc', now()) - interval '30 minutes' WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game_a.id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET last_turn_at = timezone('utc', now()) - interval '3 hours' WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game_d.id)
        .bind(user_id)
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
        assert_eq!(ids, vec![game_b.id, game_c.id, game_a.id, game_d.id]);
        assert!(summaries[0].is_turn);
        assert!(summaries[1].is_turn);
        assert!(!summaries[2].is_turn);
        assert!(!summaries[3].is_turn);
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

        let outcome = restart_via_core(&pool, &http_client, creator_id, game_id)
            .await
            .unwrap();
        let RestartOutcome::Created(outcome) = outcome else {
            panic!("expected Created")
        };

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

        let result = restart_via_core(&pool, &http_client, creator_id, game_id).await;
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

    // A solo-vs-bots restart creates the new game directly (bypassing the
    // proposal flow) and links the old game to it via restarted_game_id.
    #[sqlx::test]
    async fn solo_restart_links_restarted_game_id(pool: PgPool) {
        let uri = spawn_ok_new_game_service().await;
        let (game_id, creator_id) = make_finished_solo_bot_game(&pool, &uri).await;
        let http_client = reqwest::Client::new();

        let outcome = restart_via_core(&pool, &http_client, creator_id, game_id)
            .await
            .unwrap();
        let RestartOutcome::Created(po) = outcome else {
            panic!("expected Created, got {outcome:?}")
        };
        let new_game_id = po.game_id.expect("solo restart creates a game");

        let old_ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(old_ge.game.restarted_game_id, Some(new_game_id));
    }

    // A second solo restart loses the race and reports AlreadyRestarted linking
    // to the game the first restart created.
    #[sqlx::test]
    async fn second_restart_solo_returns_already_restarted_with_game_link(pool: PgPool) {
        let uri = spawn_ok_new_game_service().await;
        let (game_id, creator_id) = make_finished_solo_bot_game(&pool, &uri).await;
        let http_client = reqwest::Client::new();

        let outcome = restart_via_core(&pool, &http_client, creator_id, game_id)
            .await
            .unwrap();
        let RestartOutcome::Created(po) = outcome else {
            panic!("expected Created, got {outcome:?}")
        };
        let new_game_id = po.game_id.expect("solo restart creates a game");

        let outcome = restart_via_core(&pool, &http_client, creator_id, game_id)
            .await
            .unwrap();
        let RestartOutcome::AlreadyRestarted {
            game_id: Some(g),
            proposal_id: None,
        } = outcome
        else {
            panic!("expected AlreadyRestarted with game link, got {outcome:?}")
        };
        assert_eq!(g, new_game_id);
    }

    // A second human-opponent restart loses the race and reports AlreadyRestarted
    // linking to the open proposal the first restart created.
    #[sqlx::test]
    async fn second_restart_human_returns_already_restarted_with_proposal_link(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        let http_client = reqwest::Client::new();

        let outcome = restart_via_core(&pool, &http_client, creator_id, game_id)
            .await
            .unwrap();
        let RestartOutcome::Created(po) = outcome else {
            panic!("expected Created, got {outcome:?}")
        };
        assert!(po.game_id.is_none());
        let pid = po.proposal_id.expect("proposal created");

        let outcome = restart_via_core(&pool, &http_client, creator_id, game_id)
            .await
            .unwrap();
        let RestartOutcome::AlreadyRestarted {
            proposal_id: Some(p),
            game_id: None,
        } = outcome
        else {
            panic!("expected AlreadyRestarted with proposal link, got {outcome:?}")
        };
        assert_eq!(p, pid);
    }

    // An edited roster that drops the human opponent and adds a bot is honoured:
    // the new game has the creator + bot (not the dropped opponent) and the old
    // game links to it.
    #[sqlx::test]
    async fn edited_roster_drop_human_add_bot_is_honoured(pool: PgPool) {
        let uri = spawn_ok_new_game_service().await;
        let (game_id, creator_id) = make_finished_two_player_game(&pool, &uri).await;
        let http_client = reqwest::Client::new();

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        let opponent_id = ge
            .game_players
            .iter()
            .find_map(|p| p.user.as_ref().filter(|u| u.id != creator_id).map(|u| u.id))
            .unwrap();
        let version = ge.game_version.clone();

        let outcome = restart_core(
            &pool,
            &http_client,
            creator_id,
            game_id,
            &version,
            &[],
            &[],
            &[BotSlot {
                name: "Botty".to_string(),
                bot_name: "easy".to_string(),
            }],
        )
        .await
        .unwrap();
        let RestartOutcome::Created(po) = outcome else {
            panic!("expected Created, got {outcome:?}")
        };
        let new_game_id = po.game_id.expect("solo create makes a game");

        let new_ge = crate::db::find_game_extended(&pool, new_game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(new_ge.game_players.len(), 2);
        assert_eq!(
            new_ge
                .game_players
                .iter()
                .filter(|p| p.game_bot.is_some())
                .count(),
            1
        );
        assert!(
            new_ge
                .game_players
                .iter()
                .any(|p| p.user.as_ref().is_some_and(|u| u.id == creator_id))
        );
        assert!(
            !new_ge
                .game_players
                .iter()
                .any(|p| p.user.as_ref().is_some_and(|u| u.id == opponent_id))
        );

        let old_ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(old_ge.game.restarted_game_id, Some(new_game_id));
    }

    // An edited roster whose total player count is not in the game type's
    // player_counts is rejected before any game-service call.
    #[sqlx::test]
    async fn edited_roster_invalid_count_rejected(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        let http_client = reqwest::Client::new();

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        let opponent_id = ge
            .game_players
            .iter()
            .find_map(|p| p.user.as_ref().filter(|u| u.id != creator_id).map(|u| u.id))
            .unwrap();
        let version = ge.game_version.clone();

        let result = restart_core(
            &pool,
            &http_client,
            creator_id,
            game_id,
            &version,
            &[opponent_id],
            &[],
            &[BotSlot {
                name: "Botty".to_string(),
                bot_name: "easy".to_string(),
            }],
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("players"), "unexpected error: {err}");
    }

    // restart_core itself rejects a caller who is not a player of the old game,
    // independent of the caller-side guards.
    #[sqlx::test]
    async fn restart_core_rejects_non_player(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        let stranger = make_user(&pool, "stranger").await;
        let http_client = reqwest::Client::new();

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        let version = ge.game_version.clone();

        let result = restart_core(
            &pool,
            &http_client,
            stranger,
            game_id,
            &version,
            &[creator_id],
            &[],
            &[],
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not a player"), "unexpected error: {err}");
    }

    #[sqlx::test]
    async fn restart_core_rejects_garbage_bot_name(pool: PgPool) {
        let uri = spawn_ok_new_game_service().await;
        let (game_id, creator_id) = make_finished_two_player_game(&pool, &uri).await;
        let http_client = reqwest::Client::new();

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        let version = ge.game_version.clone();

        let result = restart_core(
            &pool,
            &http_client,
            creator_id,
            game_id,
            &version,
            &[],
            &[],
            &[BotSlot {
                name: "Botty".to_string(),
                bot_name: "garbage".to_string(),
            }],
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not a valid bot type"),
            "unexpected error: {err}"
        );
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

    #[sqlx::test]
    async fn restart_prefill_returns_other_human_for_finished_two_player_game(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;

        let prefill = get_restart_prefill_impl(&pool, creator_id, game_id)
            .await
            .unwrap();

        assert_eq!(prefill.game_type_name, "Test Game");
        assert_eq!(prefill.version_name, "v1");
        assert_eq!(prefill.player_counts, vec![2]);
        assert_eq!(prefill.opponents.len(), 1);
        let opp = &prefill.opponents[0];
        assert_eq!(opp.name, "opponent");
        assert!(opp.bot_name.is_none());
        assert_ne!(opp.user_id, Some(creator_id));
        assert!(opp.user_id.is_some());
    }

    #[sqlx::test]
    async fn restart_prefill_returns_bots_for_solo_bot_game(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_solo_bot_game(&pool, "http://127.0.0.1:8100").await;

        let prefill = get_restart_prefill_impl(&pool, creator_id, game_id)
            .await
            .unwrap();

        assert_eq!(prefill.opponents.len(), 1);
        let opp = &prefill.opponents[0];
        assert_eq!(opp.user_id, None);
        assert_eq!(opp.name, "Botty");
        assert_eq!(opp.bot_name, Some("easy".to_string()));
    }

    #[sqlx::test]
    async fn restart_prefill_rejects_non_player(pool: PgPool) {
        let (game_id, _creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        let stranger = make_user(&pool, "stranger").await;

        let result = get_restart_prefill_impl(&pool, stranger, game_id).await;
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn restart_prefill_rejects_unfinished_game(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: creator,
                opponent_ids: &[opponent],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let result = get_restart_prefill_impl(&pool, creator, game.id).await;
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn restart_proposal_id_present_when_open_restart_proposal_exists(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        let game_version_id: Uuid =
            sqlx::query_scalar("SELECT game_version_id FROM games WHERE id = $1")
                .bind(game_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let proposal_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status, restarted_game_id)
             VALUES ($1, $2, 'open', $3) RETURNING id",
        )
        .bind(game_version_id)
        .bind(creator_id)
        .bind(game_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            crate::db::find_open_restart_proposal(&pool, game_id)
                .await
                .unwrap(),
            Some(proposal_id)
        );
    }

    #[sqlx::test]
    async fn restart_proposal_id_none_when_no_open_restart_proposal(pool: PgPool) {
        let (game_id, _creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;

        assert_eq!(
            crate::db::find_open_restart_proposal(&pool, game_id)
                .await
                .unwrap(),
            None
        );
    }

    #[sqlx::test]
    async fn restart_proposal_id_clears_after_cancel(pool: PgPool) {
        let (game_id, creator_id) =
            make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        let game_version_id: Uuid =
            sqlx::query_scalar("SELECT game_version_id FROM games WHERE id = $1")
                .bind(game_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let proposal_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status, restarted_game_id)
             VALUES ($1, $2, 'open', $3) RETURNING id",
        )
        .bind(game_version_id)
        .bind(creator_id)
        .bind(game_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            crate::db::find_open_restart_proposal(&pool, game_id)
                .await
                .unwrap(),
            Some(proposal_id)
        );

        sqlx::query("UPDATE game_proposals SET status = 'cancelled' WHERE id = $1")
            .bind(proposal_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            crate::db::find_open_restart_proposal(&pool, game_id)
                .await
                .unwrap(),
            None
        );
    }

    fn log_time(minutes_ago: i64) -> time::PrimitiveDateTime {
        let t = time::OffsetDateTime::now_utc() - time::Duration::minutes(minutes_ago);
        time::PrimitiveDateTime::new(t.date(), t.time())
    }

    async fn insert_log(
        pool: &PgPool,
        game_id: Uuid,
        body: &str,
        is_public: bool,
        minutes_ago: i64,
    ) {
        sqlx::query(
            "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(game_id)
        .bind(body)
        .bind(is_public)
        .bind(log_time(minutes_ago))
        .execute(pool)
        .await
        .unwrap();
    }

    /// Mock game service answering spectator `PubRender` requests (the public
    /// index renders with position = None). The shared mock in game/mod.rs only
    /// answers what the handler returns, so answer PubRender here.
    async fn spawn_pub_render_service() -> String {
        use brdgme_cmd::api::{PubRender, Request, Response};
        crate::game::tests::spawn_mock_game_service(move |req| match req {
            Request::PubRender { .. } => Response::PubRender {
                render: PubRender {
                    pub_state: "pub".to_string(),
                    render: "mock public render".to_string(),
                },
            },
            _ => Response::SystemError {
                message: "unsupported in mock".to_string(),
            },
        })
        .await
    }

    /// An active (non-finished) two-human game pointed at `uri`; both humans
    /// default to game_visibility = 'public'.
    async fn make_active_public_game(pool: &PgPool, uri: &str) -> Uuid {
        let creator = make_user(pool, "creator").await;
        let opponent = make_user(pool, "opponent").await;
        let game_version_id = make_game_version_at(pool, uri).await;
        let game = crate::db::create_game_with_users(
            pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: creator,
                opponent_ids: &[opponent],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        game.id
    }

    #[sqlx::test]
    async fn public_index_returns_render_logs_and_type_name(pool: PgPool) {
        let uri = spawn_pub_render_service().await;
        let game_id = make_active_public_game(&pool, &uri).await;
        insert_log(&pool, game_id, "first public line", true, 30).await;
        insert_log(&pool, game_id, "second public line", true, 20).await;
        insert_log(&pool, game_id, "third public line", true, 10).await;

        let http = reqwest::Client::new();
        let result = public_index_data(&pool, &http)
            .await
            .unwrap()
            .expect("a game");

        assert_eq!(result.game_id, game_id);
        assert_eq!(result.type_name, "Test Game");
        assert!(
            result.html.contains("mock public render"),
            "html: {}",
            result.html
        );
        assert_eq!(result.player_names.len(), 2);
        assert_eq!(result.logs.len(), 3);
        assert!(result.logs[0].body_html.contains("first public line"));
        assert!(result.logs[2].body_html.contains("third public line"));
        assert!(!result.logs.iter().any(|l| l.is_new));
    }

    #[sqlx::test]
    async fn public_index_none_when_no_qualifying_games(pool: PgPool) {
        let uri = spawn_pub_render_service().await;
        let http = reqwest::Client::new();
        // No games at all.
        assert!(public_index_data(&pool, &http).await.unwrap().is_none());

        // A finished game does not qualify either.
        let game_id = make_active_public_game(&pool, &uri).await;
        sqlx::query("UPDATE games SET is_finished = true WHERE id = $1")
            .bind(game_id)
            .execute(&pool)
            .await
            .unwrap();
        assert!(public_index_data(&pool, &http).await.unwrap().is_none());
    }

    #[sqlx::test]
    async fn render_game_public_refuses_non_public_game(pool: PgPool) {
        let uri = spawn_pub_render_service().await;
        let game_id = make_active_public_game(&pool, &uri).await;
        let http = reqwest::Client::new();

        // Sanity: visible while all players are public.
        assert!(
            render_game_public(&pool, &http, game_id)
                .await
                .unwrap()
                .is_some()
        );

        // A player switching to 'private' after selection must be caught by the
        // render-time re-check (race window, D-render-race).
        let player_user_id: Uuid = sqlx::query_scalar(
            "SELECT user_id FROM game_players WHERE game_id = $1 AND user_id IS NOT NULL LIMIT 1",
        )
        .bind(game_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        crate::db::set_game_visibility(&pool, player_user_id, "private")
            .await
            .unwrap();

        assert!(
            render_game_public(&pool, &http, game_id)
                .await
                .unwrap()
                .is_none()
        );
        // And the full selection path agrees.
        assert!(public_index_data(&pool, &http).await.unwrap().is_none());
    }

    #[sqlx::test]
    async fn public_index_logs_limited_to_three_public_only(pool: PgPool) {
        let uri = spawn_pub_render_service().await;
        let game_id = make_active_public_game(&pool, &uri).await;
        // 5 public lines (oldest -> newest by minutes_ago descending) + 1 private.
        insert_log(&pool, game_id, "public one", true, 50).await;
        insert_log(&pool, game_id, "public two", true, 40).await;
        insert_log(&pool, game_id, "public three", true, 30).await;
        insert_log(&pool, game_id, "public four", true, 20).await;
        insert_log(&pool, game_id, "public five", true, 10).await;
        insert_log(&pool, game_id, "secret private line", false, 5).await;

        let http = reqwest::Client::new();
        let result = public_index_data(&pool, &http)
            .await
            .unwrap()
            .expect("a game");

        // Only the 3 most recent PUBLIC lines, in chronological order.
        assert_eq!(result.logs.len(), 3);
        assert!(result.logs[0].body_html.contains("public three"));
        assert!(result.logs[1].body_html.contains("public four"));
        assert!(result.logs[2].body_html.contains("public five"));
        let joined = result
            .logs
            .iter()
            .map(|l| l.body_html.clone())
            .collect::<String>();
        assert!(!joined.contains("secret private line"));
        assert!(!joined.contains("public one"));
        assert!(!joined.contains("public two"));
    }

    /// wd F21: drive the real `get_game_details` entry point through the
    /// server-fn harness and prove the BATCHED `should_hide_add_friend_many`
    /// add-friend affordance (server_fns.rs:316, :385) for two co-players in one
    /// call: an ACCEPTED friend (affordance hidden) and a PENDING INCOMING
    /// requester (affordance shown - accepting by sending back is the documented
    /// mutual-intent path). The viewer spectates a public game, so the predicate
    /// runs over both human players via the batch query rather than per-row. The
    /// game service is the in-process mock answering PubRender with a canned
    /// render; no real service is contacted.
    #[sqlx::test]
    async fn get_game_details_batch_add_friend_reflects_per_player_state(pool: PgPool) {
        let uri = spawn_pub_render_service().await;

        // Two human co-players; both default to game_visibility = 'public' so the
        // spectating viewer passes the visibility gate.
        let accepted_friend = make_user(&pool, "acceptedfriend").await;
        let pending_in = make_user(&pool, "pendingin").await;
        let game_version_id = make_game_version_at(&pool, &uri).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: accepted_friend,
                opponent_ids: &[pending_in],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        let game_id = game.id;

        let data = crate::test_support::non_admin(&pool, || async {
            let viewer = crate::auth::server::get_current_user()
                .await
                .expect("session query ok")
                .expect("authenticated viewer");

            // viewer <-> accepted_friend: mutual (accepted). pending_in -> viewer:
            // a pending INCOMING request the viewer has not responded to.
            crate::db::test_support::accept_friends(&pool, viewer.id, accepted_friend).await;
            crate::db::send_friend_request(&pool, pending_in, viewer.id)
                .await
                .expect("pending incoming request");

            get_game_details(game_id).await.expect("game details")
        })
        .await;

        let can_add = |uid: Uuid| -> bool {
            data.players
                .iter()
                .find(|p| p.user_id == Some(uid))
                .unwrap_or_else(|| panic!("player {uid} must be in the game view"))
                .can_add_friend
        };

        assert!(
            !can_add(accepted_friend),
            "an accepted friend must have the add-friend affordance hidden"
        );
        assert!(
            can_add(pending_in),
            "a pending INCOMING requester must still show the add-friend affordance"
        );
    }

    #[test]
    fn conflict_or_internal_maps_typed_finished_and_stale_errors() {
        let finished = conflict_or_internal("test", anyhow::anyhow!(crate::db::GameAlreadyFinished));
        match finished {
            leptos::prelude::ServerFnError::ServerError(m) => {
                assert_eq!(m, "Game is already finished");
            }
            _ => panic!("expected ServerError, got {finished:?}"),
        }

        let stale = conflict_or_internal("test", anyhow::anyhow!(crate::db::StaleStateConflict));
        match stale {
            leptos::prelude::ServerFnError::ServerError(m) => {
                assert!(
                    m.contains("nothing was changed"),
                    "unexpected stale message: {m}"
                );
            }
            _ => panic!("expected ServerError, got {stale:?}"),
        }

        let other = conflict_or_internal("test", anyhow::anyhow!("boom"));
        match other {
            leptos::prelude::ServerFnError::ServerError(m) => {
                assert_eq!(m, crate::error::INTERNAL_ERROR_MESSAGE);
            }
            _ => panic!("expected ServerError, got {other:?}"),
        }
    }

    fn dt() -> time::PrimitiveDateTime {
        time::macros::datetime!(2026-01-01 0:00)
    }

    /// A single snapshot seat. `user_id` Some is a human (None a pure bot),
    /// `left` marks an already-left (departed/replaced) human, and
    /// `departure_sequence` ties a human to a departure event. Fixed
    /// `place`/`points`/timestamps keep the pure predicates' inputs focused.
    fn seat(
        user_id: Option<Uuid>,
        left: bool,
        departure_sequence: Option<i32>,
    ) -> crate::db::GamePlayerExtended {
        let game_id = Uuid::new_v4();
        crate::db::GamePlayerExtended {
            game_player: crate::models::game::GamePlayer {
                id: Uuid::new_v4(),
                created_at: dt(),
                updated_at: dt(),
                game_id,
                user_id,
                position: 0,
                color: "red".to_string(),
                has_accepted: true,
                is_turn: false,
                is_turn_at: dt(),
                place: None,
                last_turn_at: dt(),
                is_eliminated: false,
                is_read: true,
                points: Some(1.0),
                undo_game_state: None,
                rating_change: None,
                ranked_placing: None,
                left_at: left.then(dt),
                departure_reason: departure_sequence.map(|_| "conceded".to_string()),
                departure_sequence,
            },
            user: user_id.map(|id| crate::models::user::User {
                id,
                created_at: dt(),
                updated_at: dt(),
                name: "Human".to_string(),
                pref_colors: Vec::new(),
                theme: None,
                is_admin: false,
            }),
            game_bot: user_id.is_none().then(|| crate::models::game::GameBot {
                id: Uuid::new_v4(),
                game_id,
                name: "Bot".to_string(),
                bot_name: "easy".to_string(),
            }),
            game_type_user: crate::models::game::GameTypeUser {
                id: Uuid::new_v4(),
                created_at: dt(),
                updated_at: dt(),
                game_type_id: Uuid::new_v4(),
                user_id: user_id.unwrap_or_default(),
                last_game_finished_at: None,
                rating: 1000,
                peak_rating: 1000,
            },
        }
    }

    fn snapshot(seats: Vec<crate::db::GamePlayerExtended>) -> crate::db::GameExtended {
        crate::db::GameExtended {
            game: crate::models::game::Game {
                id: Uuid::new_v4(),
                created_at: dt(),
                updated_at: dt(),
                game_version_id: Uuid::new_v4(),
                is_finished: false,
                finished_at: None,
                game_state: "state".to_string(),
                chat_id: None,
                restarted_game_id: None,
                end_reason: None,
            },
            game_type: crate::models::game::GameType {
                id: Uuid::new_v4(),
                created_at: dt(),
                updated_at: dt(),
                name: "Game".to_string(),
                player_counts: vec![seats.len() as i32],
                weight: 1.0,
                blurb: String::new(),
            },
            game_version: crate::models::game::GameVersion {
                id: Uuid::new_v4(),
                created_at: dt(),
                updated_at: dt(),
                game_type_id: Uuid::new_v4(),
                name: "v1".to_string(),
                uri: "http://127.0.0.1:8100".to_string(),
                is_public: true,
                is_deprecated: false,
            },
            game_players: seats,
        }
    }

    /// Concede capability matrix: replacement/no-replacement seat cases and the
    /// active-human actor rule.
    #[test]
    fn concede_eligible_covers_replacement_and_forfeit_seat_cases() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let two_humans = snapshot(vec![seat(Some(a), false, None), seat(Some(b), false, None)]);
        assert!(
            concede_eligible(&two_humans, Some(a), false),
            "two total seats forfeit with no replacement bot"
        );
        assert!(concede_eligible(&two_humans, Some(a), true), "replacement path");
        assert!(
            !concede_eligible(&two_humans, Some(c), false),
            "a spectator is not an active human"
        );

        let three_humans = snapshot(vec![
            seat(Some(a), false, None),
            seat(Some(b), false, None),
            seat(Some(c), false, None),
        ]);
        assert!(
            !concede_eligible(&three_humans, Some(a), false),
            "no replacement bot and not exactly two seats"
        );
        assert!(concede_eligible(&three_humans, Some(a), true));

        let with_departed = snapshot(vec![
            seat(Some(a), true, Some(1)),
            seat(Some(b), false, None),
            seat(Some(c), false, None),
        ]);
        assert!(
            !concede_eligible(&with_departed, Some(a), true),
            "a departed human is not an active actor"
        );
        assert!(
            !concede_eligible(&with_departed, Some(b), false),
            "three total seats still need a replacement bot"
        );
        assert!(concede_eligible(&with_departed, Some(b), true));

        assert!(
            !concede_eligible(&three_humans, None, true),
            "a pure-bot seat cannot Concede"
        );
    }

    /// One-active End-versus-Concede precedence: exactly one active human may
    /// End but never Concede, and the sole active human is the only End actor.
    #[test]
    fn one_active_human_prefers_end_over_concede() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let ge = snapshot(vec![
            seat(Some(a), false, None),
            seat(Some(b), true, Some(1)),
            seat(None, false, None),
        ]);

        assert!(
            !concede_eligible(&ge, Some(a), true),
            "exactly one active human cannot Concede even with a replacement bot"
        );
        assert!(end_eligible(&ge, Some(a)), "the sole active human may End");
        assert!(
            !end_eligible(&ge, Some(b)),
            "a departed human is not the active End actor"
        );
    }

    /// Zero-active End: every human tied in the latest departure event is
    /// authorized, while earlier-event and pure-bot actors are not.
    #[test]
    fn zero_active_humans_end_authorizes_latest_departure_tie_only() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let ge = snapshot(vec![
            seat(Some(a), true, Some(1)),
            seat(Some(b), true, Some(2)),
            seat(Some(c), true, Some(2)),
            seat(None, false, None),
        ]);

        assert!(end_eligible(&ge, Some(b)), "tied in the latest event");
        assert!(end_eligible(&ge, Some(c)), "tied in the latest event");
        assert!(
            !end_eligible(&ge, Some(a)),
            "an earlier departure event is not authorized"
        );
        assert!(
            !end_eligible(&ge, None),
            "a pure bot is never latest-event authority"
        );
        assert!(
            !concede_eligible(&ge, Some(b), true),
            "no active humans remain to Concede"
        );
    }

    /// Earlier departed rejection: with one active human, a previously departed
    /// human cannot End.
    #[test]
    fn earlier_departed_human_cannot_end() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let ge = snapshot(vec![seat(Some(a), false, None), seat(Some(b), true, Some(1))]);

        assert!(end_eligible(&ge, Some(a)), "the sole active human may End");
        assert!(
            !end_eligible(&ge, Some(b)),
            "the earlier departed human cannot End"
        );
    }

    /// Pure-bot exclusion: bot seats are never End/Concede actors and never
    /// supply latest-event authority.
    #[test]
    fn pure_bots_are_never_humans_or_latest_event_authority() {
        let a = Uuid::new_v4();
        let all_bots = snapshot(vec![seat(None, false, None), seat(None, false, None)]);
        assert!(!end_eligible(&all_bots, None), "a pure-bot game cannot End");
        assert!(!end_eligible(&all_bots, Some(a)), "there are no humans at all");

        let human_with_bots = snapshot(vec![
            seat(Some(a), false, None),
            seat(None, false, None),
            seat(None, false, None),
        ]);
        assert!(end_eligible(&human_with_bots, Some(a)), "sole active human");
        assert!(
            !end_eligible(&human_with_bots, None),
            "a bot seat cannot End"
        );
        assert!(
            !concede_eligible(&human_with_bots, None, true),
            "a bot seat cannot Concede"
        );
        assert!(
            !concede_eligible(&human_with_bots, Some(a), true),
            "one active human cannot Concede"
        );
    }

    /// Multi-active End rejection: two or more active humans never authorize
    /// End, even for one of the active actors.
    #[test]
    fn two_or_more_active_humans_reject_end() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let two = snapshot(vec![seat(Some(a), false, None), seat(Some(b), false, None)]);
        assert!(!end_eligible(&two, Some(a)), "two active humans");
        assert!(!end_eligible(&two, Some(b)), "two active humans");
        assert!(concede_eligible(&two, Some(a), false), "two-seat forfeit");

        let three = snapshot(vec![
            seat(Some(a), false, None),
            seat(Some(b), false, None),
            seat(Some(c), false, None),
        ]);
        assert!(!end_eligible(&three, Some(a)), "three active humans");
    }

    /// Authorization inputs never consult `place`, `points`, or timestamps:
    /// varying every result/timestamp field leaves identical eligibility.
    #[test]
    fn eligibility_never_consults_place_points_or_timestamps() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut ge = snapshot(vec![seat(Some(a), false, None), seat(Some(b), false, None)]);

        let before = (
            concede_eligible(&ge, Some(a), true),
            end_eligible(&ge, Some(a)),
        );

        for p in &mut ge.game_players {
            p.game_player.place = Some(3);
            p.game_player.ranked_placing = Some(1);
            p.game_player.points = Some(999.0);
            p.game_player.left_at = p
                .game_player
                .left_at
                .map(|_| time::macros::datetime!(2026-06-01 0:00));
            p.game_player.updated_at = time::macros::datetime!(2026-06-01 0:00);
        }

        assert_eq!(
            (
                concede_eligible(&ge, Some(a), true),
                end_eligible(&ge, Some(a))
            ),
            before
        );
    }

    #[sqlx::test]
    async fn end_core_rejects_finished_game(pool: PgPool) {
        let (game_id, creator) = make_finished_two_player_game(&pool, "http://127.0.0.1:8100").await;
        match end_core(&pool, game_id, ActingPlayer::User(creator)).await {
            Err(ServerFnError::ServerError(m)) => {
                assert_eq!(m, "Game is already finished");
            }
            other => panic!("expected ServerError, got {other:?}"),
        }
    }

    #[sqlx::test]
    async fn end_core_rejects_when_two_active_humans(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: creator,
                opponent_ids: &[opponent],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        match end_core(&pool, game.id, ActingPlayer::User(creator)).await {
            Err(ServerFnError::ServerError(m)) => {
                assert_eq!(m, "End game is only available to the last human");
            }
            other => panic!("expected ServerError, got {other:?}"),
        }
        let is_finished: bool =
            sqlx::query_scalar("SELECT is_finished FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!is_finished);
    }

    #[sqlx::test]
    async fn end_core_ends_solo_human_and_returns_pre_write_snapshot(pool: PgPool) {
        let creator = make_user(&pool, "solo").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: creator,
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

        let before = end_core(&pool, game.id, ActingPlayer::User(creator))
            .await
            .expect("last human may stop the game");
        assert!(
            !before.game.is_finished,
            "the returned snapshot must be the pre-write state"
        );

        let is_finished: bool =
            sqlx::query_scalar("SELECT is_finished FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(is_finished, "end_core must delegate to the locked writer");
    }

    #[sqlx::test]
    async fn concede_core_rejects_sole_active_human_before_dispatch(pool: PgPool) {
        // A replacement bot is configured so that, without the two-active-human
        // guard, the sole active human would be replaced. The guard must reject
        // before any replacement/forfeit dispatch.
        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let creator = make_user(&pool, "solo").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: creator,
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

        match concede_core(&pool, game.id, ActingPlayer::User(creator)).await {
            Err(ServerFnError::ServerError(m)) => {
                assert!(
                    m.contains("at least two active humans"),
                    "unexpected concede rejection: {m}"
                );
            }
            other => panic!("expected ServerError, got {other:?}"),
        }

        let (is_finished, game_bot_id): (bool, Option<Uuid>) = sqlx::query_as(
            "SELECT g.is_finished, gp.game_bot_id FROM games g \
             JOIN game_players gp ON gp.game_id = g.id \
             WHERE g.id = $1 AND gp.user_id = $2",
        )
        .bind(game.id)
        .bind(creator)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!is_finished);
        assert!(
            game_bot_id.is_none(),
            "the sole active human must not be replaced"
        );
    }
}
