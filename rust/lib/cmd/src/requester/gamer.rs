use std::fmt::Debug;
use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use brdgme_game::errors::GameError;
use brdgme_game::{CommandResponse, Gamer, Renderer};

use crate::api::{
    CliLog, GameResponse, GameResponseError, PlayerRender, PubRender, Request, Response,
};
use crate::requester::Requester;
use crate::requester::error::RequestError;

pub struct GameRequester<G: Gamer + Debug + Clone + Serialize + DeserializeOwned> {
    gamer: PhantomData<G>,
}

pub fn new<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> GameRequester<G> {
    GameRequester { gamer: PhantomData }
}

impl<G: Gamer + Debug + Clone + Serialize + DeserializeOwned> Requester for GameRequester<G> {
    fn request(&mut self, req: &Request) -> Result<Response, RequestError> {
        match *req {
            Request::New { players, seed } => Ok(handle_new::<G>(players, seed)),
            Request::PlayerCounts => Ok(handle_player_counts::<G>()),
            Request::Status { ref game } => {
                let game = serde_json::from_str(game)?;
                Ok(handle_status::<G>(&game))
            }
            Request::Play {
                player,
                ref command,
                ref names,
                ref game,
            } => {
                let mut game = serde_json::from_str(game)?;
                Ok(handle_play::<G>(player, command, names, &mut game))
            }
            Request::PubRender { ref game } => {
                let game = serde_json::from_str(game)?;
                Ok(handle_pub_render::<G>(&game))
            }
            Request::PlayerRender { player, ref game } => {
                let game = serde_json::from_str(game)?;
                Ok(handle_player_render::<G>(player, &game))
            }
            Request::Rules => Ok(handle_rules::<G>()),
            Request::DataDocs { .. } => Ok(handle_data_docs::<G>()),
            Request::BasicStrategy { .. } => Ok(handle_basic_strategy::<G>()),
            Request::AdvancedStrategy { .. } => Ok(handle_advanced_strategy::<G>()),
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
) -> Result<(PubRender, Vec<PlayerRender>), GameResponseError> {
    let pub_state = game.pub_state();
    let pub_render = PubRender {
        pub_state: serde_json::to_string(&pub_state)?,
        render: brdgme_markup::to_string(&pub_state.render()),
    };
    let mut player_renders: Vec<PlayerRender> = Vec::with_capacity(game.player_count());
    for p in 0..game.player_count() {
        let player_state = game.player_state(p);
        player_renders.push(PlayerRender {
            player_state: serde_json::to_string(&player_state)?,
            render: brdgme_markup::to_string(&player_state.render()),
            command_spec: game.command_spec(p),
        });
    }
    Ok((pub_render, player_renders))
}

fn handle_new<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    players: usize,
    seed: Option<u64>,
) -> Response {
    let seed = seed.unwrap_or_else(rand::random);
    match G::start(players, seed) {
        Ok((game, logs)) => GameResponse::from_gamer(&game)
            .and_then(|gs| {
                let (public_render, player_renders) = renders(&game)?;
                Ok(Response::New {
                    game: gs,
                    logs: CliLog::from_logs(&logs),
                    public_render,
                    player_renders,
                    seed,
                })
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
        .and_then(|gr| {
            let (public_render, player_renders) = renders(game)?;
            Ok(Response::Status {
                game: gr,
                public_render,
                player_renders,
            })
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
            .and_then(|gr| {
                let (public_render, player_renders) = renders(game)?;
                Ok(Response::Play {
                    game: gr,
                    logs: CliLog::from_logs(&logs),
                    can_undo,
                    remaining_input,
                    public_render,
                    player_renders,
                })
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
    match serde_json::to_string(&pub_state) {
        Ok(pub_state_json) => Response::PubRender {
            render: PubRender {
                pub_state: pub_state_json,
                render: brdgme_markup::to_string(&pub_state.render()),
            },
        },
        Err(e) => Response::SystemError {
            message: GameResponseError::from(e).to_string(),
        },
    }
}

fn handle_player_render<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    player: usize,
    game: &G,
) -> Response {
    let player_state = game.player_state(player);
    match serde_json::to_string(&player_state) {
        Ok(player_state_json) => Response::PlayerRender {
            render: PlayerRender {
                player_state: player_state_json,
                render: brdgme_markup::to_string(&player_state.render()),
                command_spec: game.command_spec(player),
            },
        },
        Err(e) => Response::SystemError {
            message: GameResponseError::from(e).to_string(),
        },
    }
}

fn handle_rules<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> Response {
    Response::Rules { rules: G::rules() }
}

fn handle_data_docs<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> Response {
    Response::DataDocs {
        data_docs: G::data_docs(),
    }
}

fn handle_basic_strategy<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> Response {
    Response::BasicStrategy {
        strategy: G::basic_strategy(),
    }
}

fn handle_advanced_strategy<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>() -> Response {
    Response::AdvancedStrategy {
        strategy: G::advanced_strategy(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_game::{BrokenRenderGame, TestGame};

    #[test]
    fn status_render_serialization_failure_returns_system_error() {
        let state = serde_json::to_string(&BrokenRenderGame { players: 2 }).unwrap();
        let mut r = new::<BrokenRenderGame>();
        match r.request(&Request::Status { game: state }).unwrap() {
            Response::SystemError { message } => assert!(
                message.contains("failed to encode game state"),
                "got: {}",
                message
            ),
            resp => panic!("expected SystemError, got {:?}", resp),
        }
    }

    #[test]
    fn pub_render_serialization_failure_returns_system_error() {
        let state = serde_json::to_string(&BrokenRenderGame { players: 2 }).unwrap();
        let mut r = new::<BrokenRenderGame>();
        match r.request(&Request::PubRender { game: state }).unwrap() {
            Response::SystemError { .. } => {}
            resp => panic!("expected SystemError, got {:?}", resp),
        }
    }

    #[test]
    fn player_render_serialization_failure_returns_system_error() {
        let state = serde_json::to_string(&BrokenRenderGame { players: 2 }).unwrap();
        let mut r = new::<BrokenRenderGame>();
        match r.request(&Request::PlayerRender {
            player: 0,
            game: state,
        }) {
            Ok(Response::SystemError { .. }) => {}
            resp => panic!("expected Ok(SystemError), got {:?}", resp),
        }
    }

    #[test]
    fn status_happy_path_unchanged() {
        let state = serde_json::to_string(&TestGame::start(2, 1).unwrap().0).unwrap();
        let mut r = new::<TestGame>();
        match r.request(&Request::Status { game: state }).unwrap() {
            Response::Status { player_renders, .. } => assert_eq!(2, player_renders.len()),
            resp => panic!("expected Status, got {:?}", resp),
        }
    }
}
