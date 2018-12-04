use failure::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

use std::fmt::Debug;
use std::marker::PhantomData;

use brdgme_game::errors::GameError;
use brdgme_game::{CommandResponse, Gamer, Renderer};
use brdgme_markup;

use crate::api::{CliLog, GameResponse, PlayerRender, PubRender, Request, Response};
use crate::requester::Requester;

pub struct GameRequester<G: Gamer + Debug + Clone + Serialize + DeserializeOwned> {
    gamer: PhantomData<G>,
}

pub fn new<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> GameRequester<G> {
    GameRequester { gamer: PhantomData }
}

impl<G: Gamer + Debug + Clone + Serialize + DeserializeOwned> Requester for GameRequester<G> {
    fn request(&mut self, req: &Request) -> Result<Response, Error> {
        match *req {
            Request::New { players } => Ok(handle_new::<G>(players)),
            Request::PlayerCounts => Ok(handle_player_counts::<G>()),
            Request::Status { ref game } => {
                let game = serde_json::from_str(&game).unwrap();
                Ok(handle_status::<G>(&game))
            }
            Request::Play {
                player,
                ref command,
                ref names,
                ref game,
            } => {
                let mut game = serde_json::from_str(&game).unwrap();
                Ok(handle_play::<G>(player, &command, &names, &mut game))
            }
            Request::PubRender { ref game } => {
                let game = serde_json::from_str(&game).unwrap();
                Ok(handle_pub_render::<G>(&game))
            }
            Request::PlayerRender { player, ref game } => {
                let game = serde_json::from_str(&game).unwrap();
                Ok(handle_player_render::<G>(player, &game))
            }
        }
    }
}

fn handle_player_counts<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> Response {
    Response::PlayerCounts {
        player_counts: G::player_counts(),
    }
}

pub fn renders<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    game: &G,
) -> (PubRender, Vec<PlayerRender>) {
    let pub_state = game.pub_state();
    let pub_render = PubRender {
        pub_state: serde_json::to_string(&pub_state).unwrap(),
        render: brdgme_markup::to_string(&pub_state.render()),
    };
    let player_renders: Vec<PlayerRender> = (0..game.player_count())
        .map(|p| {
            let player_state = game.player_state(p);
            PlayerRender {
                player_state: serde_json::to_string(&player_state).unwrap(),
                render: brdgme_markup::to_string(&player_state.render()),
                command_spec: game.command_spec(p),
            }
        })
        .collect();
    (pub_render, player_renders)
}

fn handle_new<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(players: usize) -> Response {
    match G::new(players) {
        Ok((game, logs)) => GameResponse::from_gamer(&game)
            .map(|gs| {
                let (public_render, player_renders) = renders(&game);
                Response::New {
                    game: gs,
                    logs: CliLog::from_logs(&logs),
                    public_render,
                    player_renders,
                }
            })
            .unwrap_or_else(|e| Response::SystemError {
                message: e.to_string(),
            }),
        Err(GameError::Internal { message }) => Response::SystemError { message },
        Err(e) => Response::UserError {
            message: e.to_string(),
        },
    }
}

fn handle_status<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(game: &G) -> Response {
    GameResponse::from_gamer(game)
        .map(|gr| {
            let (public_render, player_renders) = renders(game);
            Response::Status {
                game: gr,
                public_render,
                player_renders,
            }
        })
        .unwrap_or_else(|e| Response::SystemError {
            message: e.to_string(),
        })
}

fn handle_play<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    player: usize,
    command: &str,
    names: &[String],
    game: &mut G,
) -> Response {
    match game.command(player, command, names) {
        Ok(CommandResponse {
            logs,
            can_undo,
            remaining_input,
        }) => GameResponse::from_gamer(game)
            .map(|gr| {
                let (public_render, player_renders) = renders(game);
                Response::Play {
                    game: gr,
                    logs: CliLog::from_logs(&logs),
                    can_undo,
                    remaining_input,
                    public_render,
                    player_renders,
                }
            })
            .unwrap_or_else(|e| Response::SystemError {
                message: e.to_string(),
            }),
        Err(GameError::Internal { message }) => Response::SystemError { message },
        Err(e) => Response::UserError {
            message: e.to_string(),
        },
    }
}

fn handle_pub_render<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    game: &G,
) -> Response {
    let pub_state = game.pub_state();
    Response::PubRender {
        render: PubRender {
            pub_state: serde_json::to_string(&pub_state).unwrap(),
            render: brdgme_markup::to_string(&pub_state.render()),
        },
    }
}

fn handle_player_render<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    player: usize,
    game: &G,
) -> Response {
    let player_state = game.player_state(player);
    Response::PlayerRender {
        render: PlayerRender {
            player_state: serde_json::to_string(&player_state).unwrap(),
            render: brdgme_markup::to_string(&player_state.render()),
            command_spec: game.command_spec(player),
        },
    }
}
