use redis::{self, Client};
use serde_json;
use uuid::Uuid;
use failure::{Error, ResultExt};

use brdgme_cmd::cli;
use brdgme_markup as markup;

use std::sync::mpsc::{channel, Receiver, Sender};

use config::CONFIG;
use db::models::*;
use db::query::{CreatedGameLog, PublicGameExtended};
use render;
use controller::game::ShowResponse;

lazy_static! {
    pub static ref CLIENT: Client = connect().unwrap();
}

pub fn connect() -> Result<Client, Error> {
    Ok(Client::open(CONFIG.redis_url.as_ref()).context("unable to open client")?)
}

pub struct Message {
    pub channel: String,
    pub payload: MessageKind,
}

#[derive(Serialize, Clone)]
pub enum MessageKind {
    GameRestarted {
        game_id: Uuid,
        restarted_game_id: Uuid,
    },
    GameUpdate(ShowResponse),
}

pub struct PubQueue {
    rx: Receiver<Message>,
}

impl PubQueue {
    pub fn new() -> (Self, Sender<Message>) {
        let (tx, rx) = channel();
        (PubQueue { rx }, tx)
    }

    pub fn run(&self) -> Result<(), Error> {
        let conn = CLIENT
            .get_connection()
            .context("unable to get Redis connection from client")?;
        loop {
            match self.rx.recv() {
                Ok(message) => {
                    match serde_json::to_string(&message.payload) {
                        Ok(payload) => {
                            let mut pipe = redis::pipe();
                            pipe.cmd("PUBLISH")
                                .arg(message.channel)
                                .arg(payload)
                                .ignore();
                            if let Err(e) = pipe.query::<()>(&conn) {
                                warn!("error publishing message: {}", e);
                            }
                        }
                        Err(e) => warn!("error converting message payload to JSON: {}", e),
                    };
                }
                Err(e) => warn!("error receiving publish message: {}", e),
            }
        }
    }
}

fn created_logs_for_player(
    player_id: Option<Uuid>,
    logs: &[CreatedGameLog],
    players: &[markup::Player],
) -> Result<Vec<RenderedGameLog>, Error> {
    logs.iter()
        .filter(|gl| {
            gl.game_log.is_public
                || player_id
                    .and_then(|p_id| gl.targets.iter().find(|t| t.game_player_id == p_id))
                    .is_some()
        })
        .map(|gl| Ok(gl.game_log.to_owned().into_rendered(players)?))
        .collect()
}

fn game_channel(game_id: &Uuid) -> String {
    format!("game.{}", game_id)
}

fn user_channel(user_auth_token_id: &Uuid) -> String {
    format!("user.{}", user_auth_token_id)
}

pub fn enqueue_game_restarted(
    game_id: &Uuid,
    restarted_game_id: &Uuid,
    user_auth_tokens: &[UserAuthToken],
    pub_queue_tx: &Sender<Message>,
) -> Result<(), Error> {
    let message = MessageKind::GameRestarted {
        game_id: game_id.to_owned(),
        restarted_game_id: restarted_game_id.to_owned(),
    };
    pub_queue_tx
        .send(Message {
            channel: game_channel(game_id),
            payload: message.clone(),
        })
        .context("error enqueuing public game restarted message")?;
    for uat in user_auth_tokens {
        pub_queue_tx
            .send(Message {
                channel: user_channel(&uat.id),
                payload: message.clone(),
            })
            .context("error enqueuing user game restarted message")?;
    }
    Ok(())
}

pub fn enqueue_game_update<'a>(
    game: &'a PublicGameExtended,
    game_logs: &[CreatedGameLog],
    public_render: &cli::PubRender,
    player_renders: &[cli::PlayerRender],
    user_auth_tokens: &[UserAuthToken],
    pub_queue_tx: &Sender<Message>,
) -> Result<(), Error> {
    let markup_players = render::public_game_players_to_markup_players(&game.game_players)?;
    pub_queue_tx
        .send(Message {
            channel: game_channel(&game.game.id),
            payload: MessageKind::GameUpdate(ShowResponse {
                game_player: None,
                game: game.game.to_owned(),
                game_type: game.game_type.to_owned(),
                game_version: game.game_version.to_owned(),
                game_players: game.game_players.to_owned(),
                game_logs: created_logs_for_player(None, game_logs, &markup_players)?,
                state: public_render.pub_state.to_owned(),
                html: render::markup_html(&public_render.render, &markup_players)?,
                command_spec: None,
                chat: game.chat.to_owned(),
            }),
        })
        .context("error enqueuing public game update")?;
    for gp in &game.game_players {
        let player_render = match player_renders.get(gp.game_player.position as usize) {
            Some(pr) => pr,
            None => continue,
        };
        let player_message = ShowResponse {
            game_player: Some(gp.game_player.to_owned()),
            game: game.game.to_owned(),
            game_type: game.game_type.to_owned(),
            game_version: game.game_version.to_owned(),
            game_players: game.game_players.to_owned(),
            game_logs: created_logs_for_player(
                Some(gp.game_player.id),
                game_logs,
                &markup_players,
            )?,
            state: player_render.player_state.to_owned(),
            html: render::markup_html(&player_render.render, &markup_players)?,
            command_spec: player_render.command_spec.to_owned(),
            chat: game.chat.to_owned(),
        };
        for uat in user_auth_tokens {
            if uat.user_id == gp.user.id {
                pub_queue_tx
                    .send(Message {
                        channel: user_channel(&uat.id),
                        payload: MessageKind::GameUpdate(player_message.clone()),
                    })
                    .context("error enqueuing player game update")?;
            }
        }
    }
    Ok(())
}
