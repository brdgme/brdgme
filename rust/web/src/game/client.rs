use anyhow::{Context, Result, anyhow};
use brdgme_cmd::api::{PlayerRender, PubRender, Request, Response};
use brdgme_game::command::Spec as CommandSpec;

#[tracing::instrument(name = "game_service_request", skip(client, request), fields(game.uri = %uri))]
pub async fn request(client: &reqwest::Client, uri: &str, request: &Request) -> Result<Response> {
    let res = client.post(uri).json(&request).send().await?;
    let body = res.text().await.context("error reading response body")?;
    let resp: Response =
        serde_json::from_str(&body).with_context(|| format!("error parsing response: {}", body))?;
    match resp {
        Response::SystemError { message } => Err(anyhow!("{}", message)),
        other => Ok(other),
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

pub async fn render(
    client: &reqwest::Client,
    uri: &str,
    game: String,
    player: Option<usize>,
) -> Result<RenderResponse> {
    match player {
        Some(p) => player_render(client, uri, game, p).await,
        None => pub_render(client, uri, game).await,
    }
}

pub async fn pub_render(
    client: &reqwest::Client,
    uri: &str,
    game: String,
) -> Result<RenderResponse> {
    match request(client, uri, &Request::PubRender { game }).await? {
        Response::PubRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    }
}

pub async fn player_render(
    client: &reqwest::Client,
    uri: &str,
    game: String,
    player: usize,
) -> Result<RenderResponse> {
    match request(client, uri, &Request::PlayerRender { player, game }).await? {
        Response::PlayerRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, routing::post};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_game_client_contract() {
        // 1. Setup Mock Server
        let app = Router::new().route(
            "/",
            post(|Json(payload): Json<Request>| async move {
                match payload {
                    Request::New { players } => {
                        // Mock response for New Game
                        Json(Response::New {
                            game: brdgme_cmd::api::GameResponse {
                                state: format!("mock_state_{}", players),
                                points: vec![0.0; players],
                                status: brdgme_game::Status::Active {
                                    whose_turn: vec![0],
                                    eliminated: vec![],
                                },
                            },
                            logs: vec![],
                            public_render: PubRender {
                                pub_state: "pub".to_string(),
                                render: "render".to_string(),
                            },
                            player_renders: vec![],
                        })
                    }
                    _ => Json(Response::SystemError {
                        message: "unsupported in mock".to_string(),
                    }),
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // 2. Execute Client Request
        let uri = format!("http://{}", addr);
        let req = Request::New { players: 2 };
        let client = reqwest::Client::new();
        let resp = request(&client, &uri, &req).await.expect("request failed");

        // 3. Verify Response
        match resp {
            Response::New { game, .. } => {
                assert_eq!(game.state, "mock_state_2");
                assert_eq!(game.points.len(), 2);
            }
            _ => panic!("expected Response::New"),
        }
    }
}
