use anyhow::{Context, Result, anyhow};
use brdgme_cmd::api::{PlayerRender, PubRender, Request, Response};
use brdgme_game::command::Spec as CommandSpec;
use std::time::Duration;

/// Bounded retry policy for transient transport failures (connect-refused,
/// timeouts) talking to the game service. Does not retry on any received
/// HTTP response, including non-2xx status - those are game-logic errors,
/// not transport failures.
#[derive(Debug, Clone)]
struct RetryConfig {
    base_delay: Duration,
    multiplier: f64,
    cap: Duration,
    max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_millis(300),
            multiplier: 2.0,
            cap: Duration::from_secs(3),
            max_attempts: 3,
        }
    }
}

/// Pure function: attempt index (0-based, i.e. the attempt that just failed)
/// -> backoff duration before the next attempt. Uses "equal jitter": half of
/// the exponential delay is fixed, half is random, so the delay always lies
/// within [exp/2, exp] (capped at `config.cap`).
fn backoff_delay(attempt: u32, config: &RetryConfig) -> Duration {
    let exp_ms = config.base_delay.as_millis() as f64 * config.multiplier.powi(attempt as i32);
    let capped_ms = exp_ms.min(config.cap.as_millis() as f64);
    let half_ms = capped_ms / 2.0;
    let jitter_ms = half_ms * rand::random::<f64>();
    Duration::from_millis((half_ms + jitter_ms) as u64)
}

async fn send_with_retry(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    request: &Request,
    config: &RetryConfig,
) -> reqwest::Result<reqwest::Response> {
    let host = format!("{version_name}.games.internal");
    let mut attempt: u32 = 0;
    loop {
        match client
            .post(uri)
            .header(reqwest::header::HOST, &host)
            .json(request)
            .send()
            .await
        {
            Ok(res) => return Ok(res),
            Err(e) => {
                attempt += 1;
                let retryable = e.is_connect() || e.is_timeout();
                if !retryable || attempt >= config.max_attempts {
                    return Err(e);
                }
                let delay = backoff_delay(attempt - 1, config);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

async fn request_with_config(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    request: &Request,
    config: &RetryConfig,
) -> Result<Response> {
    let res = send_with_retry(client, uri, version_name, request, config).await?;
    let body = res.text().await.context("error reading response body")?;
    let resp: Response =
        serde_json::from_str(&body).with_context(|| format!("error parsing response: {}", body))?;
    match resp {
        Response::SystemError { message } => Err(anyhow!("{}", message)),
        other => Ok(other),
    }
}

#[tracing::instrument(name = "game_service_request", skip(client, request), fields(game.uri = %uri))]
pub async fn request(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    request: &Request,
) -> Result<Response> {
    request_with_config(client, uri, version_name, request, &RetryConfig::default()).await
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
    version_name: &str,
    game: String,
    player: Option<usize>,
) -> Result<RenderResponse> {
    match player {
        Some(p) => player_render(client, uri, version_name, game, p).await,
        None => pub_render(client, uri, version_name, game).await,
    }
}

pub async fn pub_render(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    game: String,
) -> Result<RenderResponse> {
    match request(client, uri, version_name, &Request::PubRender { game }).await? {
        Response::PubRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    }
}

pub async fn player_render(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    game: String,
    player: usize,
) -> Result<RenderResponse> {
    match request(
        client,
        uri,
        version_name,
        &Request::PlayerRender { player, game },
    )
    .await?
    {
        Response::PlayerRender { render } => Ok(render.into()),
        _ => Err(anyhow!("invalid response type")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, http::StatusCode, routing::post};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::net::TcpListener;

    fn tiny_config() -> RetryConfig {
        RetryConfig {
            base_delay: Duration::from_millis(5),
            multiplier: 2.0,
            cap: Duration::from_millis(20),
            max_attempts: 3,
        }
    }

    #[tokio::test]
    async fn test_retry_on_connect_refused_then_success() {
        // Reserve a free port, then drop the listener so the port refuses
        // connections (nothing is listening).
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        // Bring up a real server on the same port shortly after, before the
        // retry loop's backoff elapses, so the first attempt(s) hit
        // connection-refused and a later attempt succeeds.
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(15)).await;
            let app = Router::new().route(
                "/",
                post(|Json(payload): Json<Request>| async move {
                    match payload {
                        Request::PubRender { .. } => Json(Response::PubRender {
                            render: PubRender {
                                pub_state: "pub".to_string(),
                                render: "render".to_string(),
                            },
                        }),
                        _ => Json(Response::SystemError {
                            message: "unsupported in mock".to_string(),
                        }),
                    }
                }),
            );
            let listener = TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        let config = RetryConfig {
            base_delay: Duration::from_millis(40),
            multiplier: 2.0,
            cap: Duration::from_millis(200),
            max_attempts: 3,
        };
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let start = std::time::Instant::now();
        let resp = request_with_config(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
            &config,
        )
        .await;
        assert!(resp.is_ok(), "expected eventual success, got {:?}", resp);
        // Guaranteed minimum backoff before the retry is half of base_delay
        // (40ms) = 20ms; use a slightly looser bound to avoid flakiness.
        assert!(
            start.elapsed() >= Duration::from_millis(15),
            "expected at least one backoff sleep before success, elapsed={:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn test_no_retry_on_http_error_response() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter2 = counter.clone();
        let app = Router::new().route(
            "/",
            post(move |_body: String| {
                let counter = counter2.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    (StatusCode::INTERNAL_SERVER_ERROR, "boom")
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let resp = request_with_config(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
            &tiny_config(),
        )
        .await;
        assert!(resp.is_err(), "expected error, got {:?}", resp);
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "non-2xx game-logic response must not be retried"
        );
    }

    #[tokio::test]
    async fn test_bounded_max_attempts_on_permanent_failure() {
        // A listener that accepts TCP connections but never writes a
        // response, so every attempt times out at the client's short
        // per-request timeout. Counts how many attempts were actually made.
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        std_listener.set_nonblocking(true).unwrap();
        let addr = std_listener.local_addr().unwrap();
        let listener = TcpListener::from_std(std_listener).unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter2 = counter.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((socket, _)) = listener.accept().await {
                    counter2.fetch_add(1, Ordering::SeqCst);
                    // Hold the connection open without responding, well
                    // beyond the test's lifetime.
                    tokio::spawn(async move {
                        let _socket = socket;
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    });
                }
            }
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(30))
            .build()
            .unwrap();
        let uri = format!("http://{}", addr);
        let resp = request_with_config(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
            &tiny_config(),
        )
        .await;
        assert!(resp.is_err(), "expected permanent failure, got {:?}", resp);
        assert_eq!(
            counter.load(Ordering::SeqCst),
            tiny_config().max_attempts as usize,
            "expected exactly max_attempts connection attempts"
        );
    }

    #[test]
    fn test_backoff_delay_grows_with_attempt() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            multiplier: 2.0,
            cap: Duration::from_secs(10),
            max_attempts: 5,
        };
        let d0 = backoff_delay(0, &config);
        let d1 = backoff_delay(1, &config);
        let d2 = backoff_delay(2, &config);
        assert!(d0 < d1, "d0={:?} should be < d1={:?}", d0, d1);
        assert!(d1 < d2, "d1={:?} should be < d2={:?}", d1, d2);
    }

    #[test]
    fn test_backoff_delay_respects_cap() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            multiplier: 2.0,
            cap: Duration::from_millis(500),
            max_attempts: 10,
        };
        // attempt 10 would be 100 * 2^10 ms without a cap - far beyond `cap`.
        let d = backoff_delay(10, &config);
        assert!(
            d <= config.cap,
            "delay {:?} exceeded cap {:?}",
            d,
            config.cap
        );
        assert!(
            d >= config.cap / 2,
            "delay {:?} should be at least half the cap once capped",
            d
        );
    }

    #[test]
    fn test_backoff_delay_jitter_varies_within_band() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(200),
            multiplier: 2.0,
            cap: Duration::from_secs(10),
            max_attempts: 5,
        };
        let samples: Vec<Duration> = (0..20).map(|_| backoff_delay(1, &config)).collect();
        // attempt 1: exp = 400ms, band is [200ms, 400ms]
        for d in &samples {
            assert!(
                *d >= Duration::from_millis(200) && *d <= Duration::from_millis(400),
                "sample {:?} outside expected jitter band",
                d
            );
        }
        assert!(
            samples.windows(2).any(|w| w[0] != w[1]),
            "expected jitter to produce varying delays across samples"
        );
    }

    #[tokio::test]
    async fn test_game_client_contract() {
        // 1. Setup Mock Server
        let app = Router::new().route(
            "/",
            post(|Json(payload): Json<Request>| async move {
                match payload {
                    Request::New { players, .. } => {
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
                            seed: 0,
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
        let req = Request::New {
            players: 2,
            seed: None,
        };
        let client = reqwest::Client::new();
        let resp = request(&client, &uri, "test-game-1", &req)
            .await
            .expect("request failed");

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
