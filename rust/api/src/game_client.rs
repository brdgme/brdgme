use anyhow::{anyhow, Context, Result};

use brdgme_game::command::Spec as CommandSpec;

pub async fn request(
    uri: &str,
    request: &brdgme_cmd::api::Request,
) -> Result<brdgme_cmd::api::Response> {
    let client = reqwest::Client::new();
    let res = client.post(uri).json(&request).send().await?;
    match res.json().await.context("error parsing JSON response")? {
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

pub async fn render(uri: &str, game: String, player: Option<usize>) -> Result<RenderResponse> {
    match player {
        Some(p) => player_render(uri, game, p).await,
        None => pub_render(uri, game).await,
    }
}

pub async fn pub_render(uri: &str, game: String) -> Result<RenderResponse> {
    match request(uri, &brdgme_cmd::api::Request::PubRender { game }).await? {
        brdgme_cmd::api::Response::PubRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    }
}

pub async fn player_render(uri: &str, game: String, player: usize) -> Result<RenderResponse> {
    match request(
        uri,
        &brdgme_cmd::api::Request::PlayerRender { player, game },
    )
    .await?
    {
        brdgme_cmd::api::Response::PlayerRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    }
}
