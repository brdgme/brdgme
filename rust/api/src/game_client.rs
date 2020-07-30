use anyhow::{anyhow, Context, Result};
use bytes::Buf;
use hyper_tls::HttpsConnector;

use brdgme_cmd::api::{PlayerRender, PubRender, Request, Response};
use brdgme_game::command::Spec as CommandSpec;

pub async fn request(uri: &str, request: &Request) -> Result<Response> {
    let connector = HttpsConnector::new();
    let https = hyper::Client::builder().build(connector);
    let req = hyper::Request::builder()
        .method("POST")
        .uri(uri)
        .body(hyper::Body::from(
            serde_json::to_string(request).context("error converting request to JSON")?,
        ))?;
    let mut res = https.request(req).await?;
    if res.status() != hyper::StatusCode::OK {
        return Err(anyhow!("game request failed"));
    }
    let body_buf = hyper::body::aggregate(res.body_mut()).await?;
    match serde_json::from_slice::<Response>(body_buf.bytes())
        .context("error parsing JSON response")?
    {
        //Response::UserError { message } => Err(ErrorKind::UserError(message).into()),
        Response::SystemError { message } => Err(anyhow!("{}", message)),
        default => Ok(default),
    }
}

#[derive(Debug, Clone)]
pub struct RenderResponse {
    pub render: String,
    pub state: String,
    pub command_spec: Option<CommandSpec>,
}

impl From<PubRender> for RenderResponse {
    fn from(render: PubRender) -> Self {
        Self {
            render: render.render,
            state: render.pub_state,
            command_spec: None,
        }
    }
}

impl From<PlayerRender> for RenderResponse {
    fn from(render: PlayerRender) -> Self {
        Self {
            render: render.render,
            state: render.player_state,
            command_spec: render.command_spec,
        }
    }
}

pub async fn render(uri: &str, game: String, player: Option<usize>) -> Result<RenderResponse> {
    match player {
        Some(p) => player_render(uri, game, p).await,
        None => pub_render(uri, game).await,
    }
}

pub async fn pub_render(uri: &str, game: String) -> Result<RenderResponse> {
    request(uri, &Request::PubRender { game })
        .await
        .and_then(|resp| match resp {
            Response::PubRender { render } => Ok(render.into()),
            _ => Err(anyhow!("invalid response type")),
        })
}

pub async fn player_render(uri: &str, game: String, player: usize) -> Result<RenderResponse> {
    request(uri, &Request::PlayerRender { player, game })
        .await
        .and_then(|resp| match resp {
            Response::PlayerRender { render } => Ok(render.into()),
            _ => Err(anyhow!("invalid response type")),
        })
}
