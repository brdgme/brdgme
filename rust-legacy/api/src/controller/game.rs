use rocket::State;
use rocket_contrib::Json;
use uuid::Uuid;
use diesel::Connection;
use diesel::pg::PgConnection;
use chrono::Utc;
use failure::{Error, ResultExt};

use brdgme_cmd::cli;
use brdgme_game::{Stat, Status};
use brdgme_game::command::Spec as CommandSpec;
use brdgme_markup as markup;

use std::collections::HashMap;
use std::borrow::Cow;
use std::sync::Mutex;
use std::sync::mpsc::Sender;

use db::{models, query};
use db::CONN;
use game_client;
use render;
use controller::{UuidParam, CORS};
use websocket;
use errors::ControllerError;

#[derive(Deserialize)]
pub struct CreateRequest {
    game_version_id: Uuid,
    opponent_ids: Option<Vec<Uuid>>,
    opponent_emails: Option<Vec<String>>,
}

#[post("/", data = "<data>")]
pub fn create(
    data: Json<CreateRequest>,
    user: models::User,
    pub_queue_tx: State<Mutex<Sender<websocket::Message>>>,
) -> Result<CORS<Json<ShowResponse>>, ControllerError> {
    let user_id = user.id;
    let data = data.into_inner();
    let conn = &*CONN.w.get().context("unable to get connection")?;

    let (created_game, created_logs, public_render, player_renders, user_ids) =
        conn.transaction::<_, Error, _>(move || {
            let opponent_ids = data.opponent_ids.unwrap_or_else(|| vec![]);
            let opponent_emails = data.opponent_emails.unwrap_or_else(|| vec![]);
            let player_count: usize = 1 + opponent_ids.len() + opponent_emails.len();
            let game_version = query::find_game_version(&data.game_version_id, conn)
                .context("error finding game version")?
                .ok_or_else::<Error, _>(|| format_err!("could not find game version"))?;

            let resp = game_client::request(
                &game_version.uri,
                &cli::Request::New {
                    players: player_count,
                },
            )?;
            let (game_info, logs, public_render, player_renders) = match resp {
                cli::Response::New {
                    game,
                    logs,
                    public_render,
                    player_renders,
                } => (game, logs, public_render, player_renders),
                _ => bail!("expected cli::Response::New"),
            };
            let status = game_status_values(&game_info.status);
            let created_game = query::create_game_with_users(
                &query::CreateGameOpts {
                    new_game: &models::NewGame {
                        game_version_id: data.game_version_id,
                        is_finished: status.is_finished,
                        game_state: &game_info.state,
                    },
                    whose_turn: &status.whose_turn,
                    eliminated: &status.eliminated,
                    placings: &status.placings,
                    points: &game_info.points,
                    creator_id: &user_id,
                    opponent_ids: &opponent_ids,
                    opponent_emails: &opponent_emails,
                    chat_id: None,
                },
                conn,
            ).context("unable to create game")?;
            let created_logs = query::create_game_logs_from_cli(&created_game.game.id, logs, conn)
                .context("unable to create game logs")?;
            let mut user_ids = opponent_ids.clone();
            user_ids.push(user_id);
            Ok((
                created_game,
                created_logs,
                public_render,
                player_renders,
                user_ids,
            ))
        }).context("error committing transaction")?;
    let game_extended = query::find_game_extended(&created_game.game.id, conn)
        .context("unable to get extended game")?;
    let player = created_game.players.iter().find(|p| p.user_id == user_id);
    websocket::enqueue_game_update(
        &game_extended.clone().into_public(),
        &created_logs,
        &public_render,
        &player_renders,
        &query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
        &pub_queue_tx
            .inner()
            .lock()
            .map_err::<Error, _>(|e| format_err!("unable to get lock on pub_queue_tx: {}", e))?
            .clone(),
    )?;
    Ok(CORS(Json(game_extended_to_show_response(
        player,
        &game_extended,
        player
            .and_then(|p| player_renders.get(p.position as usize))
            .map(|render| render.to_owned().into())
            .as_ref(),
        conn,
    )?)))
}

struct StatusValues {
    is_finished: bool,
    whose_turn: Vec<usize>,
    eliminated: Vec<usize>,
    placings: Vec<usize>,
    stats: Vec<HashMap<String, Stat>>,
}
fn game_status_values(status: &Status) -> StatusValues {
    let (is_finished, whose_turn, eliminated, placings, stats) = match *status {
        Status::Active {
            ref whose_turn,
            ref eliminated,
        } => (
            false,
            whose_turn.clone(),
            eliminated.clone(),
            vec![],
            vec![],
        ),
        Status::Finished {
            ref placings,
            ref stats,
        } => (true, vec![], vec![], placings.clone(), stats.clone()),
    };
    StatusValues {
        is_finished,
        whose_turn,
        eliminated,
        placings,
        stats,
    }
}

#[derive(Serialize, Clone)]
pub struct ShowResponse {
    pub game: models::PublicGame,
    pub state: String,
    pub game_version: models::PublicGameVersion,
    pub game_type: models::PublicGameType,
    pub game_player: Option<models::PublicGamePlayer>,
    pub game_players: Vec<models::PublicGamePlayerTypeUser>,
    pub html: String,
    pub game_logs: Vec<models::RenderedGameLog>,
    pub command_spec: Option<CommandSpec>,
    pub chat: Option<query::chat::PublicChatExtended>,
}

#[get("/<id>")]
pub fn show(
    id: UuidParam,
    user: Option<models::User>,
) -> Result<CORS<Json<ShowResponse>>, ControllerError> {
    let id = id.into_uuid();
    let conn = &*CONN.r.get().context("error getting connection")?;

    let game_extended = query::find_game_extended(&id, conn)?;
    let game_player: Option<&models::GamePlayer> = user.and_then(|u| {
        game_extended
            .game_players
            .iter()
            .find(|&gptu| u.id == gptu.user.id)
            .map(|gptu| &gptu.game_player)
    });
    Ok(CORS(Json(game_extended_to_show_response(
        game_player,
        &game_extended,
        None,
        conn,
    )?)))
}

fn game_extended_to_show_response(
    game_player: Option<&models::GamePlayer>,
    game_extended: &query::GameExtended,
    render: Option<&game_client::RenderResponse>,
    conn: &PgConnection,
) -> Result<ShowResponse, ControllerError> {
    let public = game_extended.clone().into_public();
    let render: Cow<game_client::RenderResponse> = match render {
        Some(r) => Cow::Borrowed(r),
        None => Cow::Owned(game_client::render(
            &game_extended.game_version.uri,
            game_extended.game.game_state.to_owned(),
            game_player.map(|gp| gp.position as usize),
        )?),
    };

    let (nodes, _) = markup::from_string(&render.render).context("error parsing render markup")?;

    let markup_players = render::game_players_to_markup_players(&game_extended.game_players)?;
    let game_logs = match game_player {
        Some(gp) => query::find_game_logs_for_player(&gp.id, conn),
        None => query::find_public_game_logs_for_game(&game_extended.game.id, conn),
    }?;

    Ok(ShowResponse {
        game_player: game_player.map(|gp| gp.to_owned().into_public()),
        game: public.game,
        state: render.state.to_owned(),
        game_version: public.game_version,
        game_type: public.game_type,
        game_players: public.game_players,
        html: markup::html(&markup::transform(&nodes, &markup_players)),
        game_logs: game_logs
            .into_iter()
            .map(|gl| gl.into_rendered(&markup_players))
            .collect::<Result<Vec<models::RenderedGameLog>, Error>>()?,
        command_spec: render.command_spec.to_owned(),
        chat: public.chat,
    })
}

#[derive(Deserialize)]
pub struct CommandRequest {
    command: String,
}

#[post("/<id>/command", data = "<data>")]
pub fn command(
    id: UuidParam,
    user: models::User,
    pub_queue_tx: State<Mutex<Sender<websocket::Message>>>,
    data: Json<CommandRequest>,
) -> Result<CORS<Json<ShowResponse>>, ControllerError> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().context("unable to get connection")?;
    conn.transaction::<_, ControllerError, _>(|| {
        let (game, game_version) = query::find_game_with_version(&id, conn)
            .context("error finding game")?
            .ok_or_else::<ControllerError, _>(|| {
                ControllerError::bad_request("game does not exist")
            })?;
        if game.is_finished {
            return Err(ControllerError::bad_request("game is already finished"));
        }

        let players: Vec<(models::GamePlayer, models::User)> =
            query::find_game_players_with_user_by_game(&id, conn)
                .context("error finding game players")?;
        let player: &models::GamePlayer = &players
            .iter()
            .find(|&&(ref p, _)| p.user_id == user.id)
            .ok_or_else::<Error, _>(|| format_err!("you are not a player in this game"))?
            .0;
        let position = player.position;

        let names = players
            .iter()
            .map(|&(_, ref user)| user.name.clone())
            .collect::<Vec<String>>();

        let (game_response, logs, can_undo, remaining_command, public_render, player_renders) =
            match game_client::request(
                &game_version.uri,
                &cli::Request::Play {
                    player: position as usize,
                    game: game.game_state.clone(),
                    command: data.command.to_owned(),
                    names: names,
                },
            )? {
                cli::Response::Play {
                    game,
                    logs,
                    can_undo,
                    remaining_input,
                    public_render,
                    player_renders,
                } => (
                    game,
                    logs,
                    can_undo,
                    remaining_input,
                    public_render,
                    player_renders,
                ),
                cli::Response::UserError { message } => {
                    return Err(ControllerError::bad_request(message))
                }
                _ => Err(format_err!("invalid response type"))?,
            };
        if !remaining_command.trim().is_empty() {
            return Err(ControllerError::bad_request(format!(
                "unexpected '{}'",
                remaining_command
            )));
        }
        let status = game_status_values(&game_response.status);

        let updated = query::update_game_command_success(
            &id,
            &player.id,
            &models::NewGame {
                game_version_id: game.game_version_id,
                is_finished: status.is_finished,
                game_state: &game_response.state,
            },
            if can_undo {
                Some(&game.game_state)
            } else {
                None
            },
            &status.whose_turn,
            &status.eliminated,
            &status.placings,
            &game_response.points,
            conn,
        ).context("error updating game")?;

        let created_logs = query::create_game_logs_from_cli(&id, logs, conn)
            .context("unable to create game logs")?;
        let game_extended =
            query::find_game_extended(&id, conn).context("unable to get extended game")?;
        let user_ids: Vec<Uuid> = game_extended
            .game_players
            .iter()
            .map(|gptu| gptu.user.id)
            .collect();
        websocket::enqueue_game_update(
            &game_extended.clone().into_public(),
            &created_logs,
            &public_render,
            &player_renders,
            &query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
            &pub_queue_tx
                .inner()
                .lock()
                .map_err::<Error, _>(|e| format_err!("unable to get lock on pub_queue_tx: {}", e))?
                .clone(),
        )?;
        let gp = game_extended
            .game_players
            .iter()
            .find(|gptu| gptu.game_player.id == player.id)
            .map(|gptu| &gptu.game_player);
        Ok(CORS(Json(game_extended_to_show_response(
            gp,
            &game_extended,
            gp.and_then(|gp| player_renders.get(gp.position as usize))
                .map(|render| render.clone().into())
                .as_ref(),
            conn,
        )?)))
    })
}

#[post("/<id>/undo")]
pub fn undo(
    id: UuidParam,
    user: models::User,
    pub_queue_tx: State<Mutex<Sender<websocket::Message>>>,
) -> Result<CORS<Json<ShowResponse>>, ControllerError> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().context("unable to get connection")?;

    conn.transaction::<_, ControllerError, _>(|| {
        let (game, game_version) = query::find_game_with_version(&id, conn)
            .context("error finding game")?
            .ok_or_else::<ControllerError, _>(|| {
                ControllerError::bad_request("game does not exist")
            })?;
        if game.is_finished {
            return Err(ControllerError::bad_request("game is already finished"));
        }

        let player = query::find_game_player_by_user_and_game(&user.id, &id, conn)
            .context("error finding game player")?
            .ok_or_else::<ControllerError, _>(|| {
                ControllerError::bad_request("you aren't a player in this game")
            })?;

        let undo_state = player
            .undo_game_state
            .clone()
            .ok_or_else::<ControllerError, _>(|| {
                ControllerError::bad_request("you can't undo at the moment")
            })?;

        let (game_response, public_render, player_renders) = match game_client::request(
            &game_version.uri,
            &cli::Request::Status {
                game: undo_state.clone(),
            },
        )? {
            cli::Response::Status {
                game,
                public_render,
                player_renders,
            } => (game, public_render, player_renders),
            _ => Err(format_err!("invalid response type"))?,
        };
        let status = game_status_values(&game_response.status);
        let updated = query::update_game_command_success(
            &id,
            &player.id,
            &models::NewGame {
                game_version_id: game.game_version_id,
                is_finished: status.is_finished,
                game_state: &game_response.state,
            },
            None,
            &status.whose_turn,
            &status.eliminated,
            &status.placings,
            &game_response.points,
            conn,
        ).context("error updating game")?;
        query::player_cannot_undo_set_undo_game_state(&id, conn)
            .context("unable to clear undo_game_state for all players")?;
        let created_log = query::create_game_log(
            &models::NewGameLog {
                game_id: id,
                body: &markup::to_string(&[
                    markup::Node::Player(player.position as usize),
                    markup::Node::text(" used an undo"),
                ]),
                is_public: true,
                logged_at: Utc::now().naive_utc(),
            },
            &[],
            conn,
        ).context("unable to create undo game log")?;
        let game_extended =
            query::find_game_extended(&id, conn).context("unable to get extended game")?;
        let user_ids: Vec<Uuid> = game_extended
            .game_players
            .iter()
            .map(|gptu| gptu.user.id)
            .collect();
        websocket::enqueue_game_update(
            &game_extended.clone().into_public(),
            &[created_log],
            &public_render,
            &player_renders,
            &query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
            &pub_queue_tx
                .inner()
                .lock()
                .map_err::<Error, _>(|e| format_err!("unable to get lock on pub_queue_tx: {}", e))?
                .clone(),
        )?;
        let gp = game_extended
            .game_players
            .iter()
            .find(|gptu| gptu.game_player.id == player.id)
            .map(|gptu| &gptu.game_player);
        Ok(CORS(Json(game_extended_to_show_response(
            gp,
            &game_extended,
            gp.and_then(|gp| player_renders.get(gp.position as usize))
                .map(|render| render.clone().into())
                .as_ref(),
            conn,
        )?)))
    })
}

#[post("/<id>/mark_read")]
pub fn mark_read(
    id: UuidParam,
    user: models::User,
) -> Result<CORS<Json<Option<models::PublicGamePlayer>>>, ControllerError> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().context("unable to get connection")?;

    conn.transaction::<_, ControllerError, _>(|| {
        Ok(CORS(Json(
            query::mark_game_read(&id, &user.id, conn)
                .context("error marking game read")?
                .map(|gp| gp.into_public()),
        )))
    })
}

#[post("/<id>/concede")]
pub fn concede(
    id: UuidParam,
    user: models::User,
    pub_queue_tx: State<Mutex<Sender<websocket::Message>>>,
) -> Result<CORS<Json<ShowResponse>>, ControllerError> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().context("unable to get connection")?;

    Ok(conn.transaction::<_, ControllerError, _>(|| {
        let (game, game_version) = query::find_game_with_version(&id, conn)
            .context("error finding game")?
            .ok_or_else::<ControllerError, _>(|| {
                ControllerError::bad_request("game does not exist")
            })?;
        if game.is_finished {
            return Err(ControllerError::bad_request("game is already finished"));
        }

        let player_count = query::find_player_count_by_game(&id, conn)
            .context("error finding player count for game")?;
        if player_count > 2 {
            return Err(ControllerError::bad_request(
                "cannot concede games with more than two players",
            ));
        }

        let player = query::find_game_player_by_user_and_game(&user.id, &id, conn)
            .context("error finding game player")?
            .ok_or_else::<ControllerError, _>(|| {
                ControllerError::bad_request("you aren't a player in this game")
            })?;

        let updated = query::concede_game(&id, &player.id, conn).context("error conceding game")?;

        let (public_render, player_renders) = match game_client::request(
            &game_version.uri,
            &cli::Request::Status {
                game: game.game_state.clone(),
            },
        )? {
            cli::Response::Status {
                public_render,
                player_renders,
                ..
            } => (public_render, player_renders),
            _ => return Err(format_err!("invalid response type").into()),
        };
        let created_log = query::create_game_log(
            &models::NewGameLog {
                game_id: id,
                body: &markup::to_string(&[
                    markup::Node::Player(player.position as usize),
                    markup::Node::text(" conceded"),
                ]),
                is_public: true,
                logged_at: Utc::now().naive_utc(),
            },
            &[],
            conn,
        ).context("unable to create concede game log")?;
        let game_extended =
            query::find_game_extended(&id, conn).context("unable to get extended game")?;
        let user_ids: Vec<Uuid> = game_extended
            .game_players
            .iter()
            .map(|gptu| gptu.user.id)
            .collect();
        websocket::enqueue_game_update(
            &game_extended.clone().into_public(),
            &[created_log],
            &public_render,
            &player_renders,
            &query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
            &pub_queue_tx
                .inner()
                .lock()
                .map_err::<Error, _>(|e| format_err!("unable to get lock on pub_queue_tx: {}", e))?
                .clone(),
        )?;
        let gp = game_extended
            .game_players
            .iter()
            .find(|gptu| gptu.game_player.id == player.id)
            .map(|gptu| &gptu.game_player);
        Ok(CORS(Json(game_extended_to_show_response(
            gp,
            &game_extended,
            gp.and_then(|gp| player_renders.get(gp.position as usize))
                .map(|render| render.clone().into())
                .as_ref(),
            conn,
        )?)))
    })?)
}

#[post("/<id>/restart")]
pub fn restart(
    id: UuidParam,
    user: models::User,
    pub_queue_tx: State<Mutex<Sender<websocket::Message>>>,
) -> Result<CORS<Json<ShowResponse>>, ControllerError> {
    let id = id.into_uuid();
    let user_id = user.id;
    let conn = &*CONN.w.get().context("unable to get connection")?;

    let (created_game, created_logs, public_render, player_renders, user_ids) =
        conn.transaction::<_, Error, _>(move || {
            let game_extended =
                query::find_game_extended(&id, conn).context("could not find game to restart")?;
            let game_player = game_extended
                .game_players
                .iter()
                .find(|gptu| gptu.user.id == user.id)
                .ok_or_else(|| format_err!("you are not a player in this game"))?;
            if game_extended.game.restarted_game_id.is_some() {
                bail!("game has already been restarted");
            }
            let opponent_ids: Vec<Uuid> = game_extended
                .game_players
                .iter()
                .filter_map(|gptu| {
                    if gptu.user.id == user.id {
                        None
                    } else {
                        Some(gptu.user.id)
                    }
                })
                .collect();
            let player_count: usize = 1 + opponent_ids.len();

            let resp = game_client::request(
                &game_extended.game_version.uri,
                &cli::Request::New {
                    players: player_count,
                },
            )?;
            let (game_info, logs, public_render, player_renders) = match resp {
                cli::Response::New {
                    game,
                    logs,
                    public_render,
                    player_renders,
                } => (game, logs, public_render, player_renders),
                _ => bail!("expected cli::Response::New"),
            };
            let status = game_status_values(&game_info.status);
            let created_game = query::create_game_with_users(
                &query::CreateGameOpts {
                    new_game: &models::NewGame {
                        game_version_id: game_extended.game_version.id,
                        is_finished: status.is_finished,
                        game_state: &game_info.state,
                    },
                    whose_turn: &status.whose_turn,
                    eliminated: &status.eliminated,
                    placings: &status.placings,
                    points: &game_info.points,
                    creator_id: &user_id,
                    opponent_ids: &opponent_ids,
                    opponent_emails: &[],
                    chat_id: None,
                },
                conn,
            ).context("unable to create game")?;
            let created_logs = query::create_game_logs_from_cli(&created_game.game.id, logs, conn)
                .context("unable to create game logs")?;
            query::game::update_restarted_game_id(
                &game_extended.game.id,
                &created_game.game.id,
                conn,
            )?;
            let mut user_ids = opponent_ids.clone();
            user_ids.push(user_id);
            Ok((
                created_game,
                created_logs,
                public_render,
                player_renders,
                user_ids,
            ))
        }).context("error committing transaction")?;
    let game_extended = query::find_game_extended(&created_game.game.id, conn)
        .context("unable to get extended game")?;
    let player = created_game.players.iter().find(|p| p.user_id == user_id);
    let tx = pub_queue_tx
        .inner()
        .lock()
        .map_err::<Error, _>(|e| format_err!("unable to get lock on pub_queue_tx: {}", e))?
        .clone();
    let tokens = query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?;
    websocket::enqueue_game_update(
        &game_extended.clone().into_public(),
        &created_logs,
        &public_render,
        &player_renders,
        &tokens,
        &tx,
    )?;
    websocket::enqueue_game_restarted(&id, &game_extended.game.id, &tokens, &tx)?;
    Ok(CORS(Json(game_extended_to_show_response(
        player,
        &game_extended,
        player
            .and_then(|p| player_renders.get(p.position as usize))
            .map(|render| render.clone().into())
            .as_ref(),
        conn,
    )?)))
}
