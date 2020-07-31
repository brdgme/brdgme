use anyhow::{anyhow, Context, Result};
use hyper::net::HttpsConnector;
use hyper::{self, Client as HttpClient};
use hyper_rustls::TlsClient;
use serde_json;

use brdgme_game::command::Spec as CommandSpec;

pub fn request(uri: &str, request: &brdgme_cmd::api::Request) -> Result<brdgme_cmd::api::Response> {
    let connector = HttpsConnector::new(TlsClient::new());
    let https = HttpClient::with_connector(connector);
    let res = https
        .post(uri)
        .body(&serde_json::to_string(request).context("error converting request to JSON")?)
        .send()
        .context("error getting new game state")?;
    if res.status != hyper::Ok {
        return Err(anyhow!("game request failed"));
    }
    match serde_json::from_reader::<_, brdgme_cmd::api::Response>(res)
        .context("error parsing JSON response")?
    {
        //cli::Response::UserError { message } => Err(ErrorKind::UserError(message).into()),
        brdgme_cmd::api::Response::SystemError { message } => Err(anyhow!("{}", message)),
        default => Ok(default),
    }
}

#[derive(Debug, Clone)]
pub struct RenderResponse {
    pub render: String,
    pub state: String,
    pub command_spec: Option<CommandSpec>,
}

impl From<brdgme_cmd::api::PubRender> for RenderResponse {
    fn from(render: brdgme_cmd::api::PubRender) -> Self {
        Self {
            render: render.render,
            state: render.pub_state,
            command_spec: None,
        }
    }
}

impl From<brdgme_cmd::api::PlayerRender> for RenderResponse {
    fn from(render: brdgme_cmd::api::PlayerRender) -> Self {
        Self {
            render: render.render,
            state: render.player_state,
            command_spec: render.command_spec,
        }
    }
}

pub fn render(uri: &str, game: String, player: Option<usize>) -> Result<RenderResponse> {
    match player {
        Some(p) => player_render(uri, game, p),
        None => pub_render(uri, game),
    }
}

pub fn pub_render(uri: &str, game: String) -> Result<RenderResponse> {
    request(uri, &brdgme_cmd::api::Request::PubRender { game }).and_then(|resp| match resp {
        brdgme_cmd::api::Response::PubRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    })
}

pub fn player_render(uri: &str, game: String, player: usize) -> Result<RenderResponse> {
    request(
        uri,
        &brdgme_cmd::api::Request::PlayerRender { player, game },
    )
    .and_then(|resp| match resp {
        brdgme_cmd::api::Response::PlayerRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    })
}
