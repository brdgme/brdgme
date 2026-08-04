//! Shared HTTP client for calling game services through the KEDA HTTP
//! interceptor. All in-cluster callers (web, bot, operator) MUST use this
//! crate: the interceptor routes purely on the Host header
//! (`{version_name}.games.internal`), which this client sets on every
//! request. Calling the interceptor without that header returns 404.

/// Typed errors for game-service calls, replacing the crate's previous
/// anyhow strings so callers can branch on kind (ls F32, WP-07). Display
/// messages are self-contained (they embed the underlying cause) because
/// two callers log via `Display` only.
///
/// Response bodies are never retained by these errors: a game-service body
/// can contain every seat's private state, so neither `Display` nor `Debug`
/// may expose it (F-192). Only bounded metadata (status, parse error) and
/// in-band service `message` values are carried.
#[derive(Debug, thiserror::Error)]
pub enum GameClientError {
    /// `version_name` is interpolated into the Host header the KEDA
    /// interceptor routes on; reject non-DNS-label names up front (ls F35).
    #[error(
        "invalid game version name {name:?}: must be a DNS label (ASCII alphanumeric and '-', 1-63 chars, no leading/trailing '-')"
    )]
    InvalidVersionName { name: String },
    #[error("transport error calling game service: {0}")]
    Transport(#[from] reqwest::Error),
    /// The crate-enforced whole-call ceiling fired (ls F31, F-12): attempts,
    /// backoff sleeps, and the response-body read together exceeded
    /// `RetryConfig::request_timeout`. Callers with a tighter
    /// `reqwest::Client` timeout see `Transport` instead.
    #[error("game service request timed out after {after:?}")]
    Timeout { after: std::time::Duration },
    /// The service returned a non-2xx status. The response body is
    /// deliberately not retained or read: it may carry private game state,
    /// so it must never reach logs, Sentry, or a caller through this error.
    #[error("game service returned {status}")]
    HttpStatus { status: reqwest::StatusCode },
    /// The response body failed to parse as a game-service response. The
    /// body is deliberately not retained; `serde_json` parse diagnostics
    /// never embed the input content.
    #[error("error parsing game service response: {source}")]
    ParseResponse {
        #[source]
        source: serde_json::Error,
    },
    /// The service reported an in-band `Response::SystemError`.
    #[error("game service system error: {message}")]
    SystemError { message: String },
    /// The service reported an in-band `Response::UserError` - a game-logic
    /// rejection of the request, surfaced with the service's own message.
    #[error("game service rejected the request: {message}")]
    UserError { message: String },
    #[error("unexpected response to {request} request")]
    UnexpectedResponse { request: &'static str },
    #[error("no player render for position {player}")]
    NoPlayerRender { player: usize },
    #[error("invalid JSON in game state: {0}")]
    StateJson(#[source] serde_json::Error),
    #[error("failed to serialize state as YAML: {0}")]
    StateYaml(#[from] serde_yaml_ng::Error),
}

use brdgme_cmd::api::{PlayerRender, PubRender, Request, Response};
use brdgme_game::command::Spec as CommandSpec;
use std::time::Duration;

/// Bounded retry policy for transient transport failures (connect-refused,
/// timeouts, connections reset mid-request) talking to the game service.
/// Does not retry on any received HTTP response, including non-2xx status -
/// those are game-logic errors, not transport failures.
///
/// `request_timeout` is a crate-enforced ceiling applied with
/// `tokio::time::timeout` across the COMPLETE call: every attempt, every
/// backoff sleep, and the response-body read together (F-12). It is NOT
/// `reqwest`'s per-request timeout: reqwest's would *replace* the caller's
/// client-level timeout (web 10s, bot 60s), whereas a ceiling composes - the
/// tighter of the two always wins. It exists so the guarantee holds even for
/// callers that configure no client timeout at all (the operator, ls F31).
/// It must stay above 60s: the KEDA interceptor holds requests open while a
/// game pod cold-starts, and bot deliberately allows 60s for that. Because
/// the ceiling bounds the whole call, a single hung attempt consumes it;
/// retries only help failures that surface quickly within the budget.
#[derive(Debug, Clone)]
struct RetryConfig {
    base_delay: Duration,
    multiplier: f64,
    cap: Duration,
    max_attempts: u32,
    request_timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_millis(300),
            multiplier: 2.0,
            cap: Duration::from_secs(3),
            max_attempts: 3,
            request_timeout: Duration::from_secs(90),
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

/// The interceptor routes on `Host: {version_name}.games.internal`; a
/// malformed name would otherwise fail deep inside reqwest's header builder
/// with an opaque error (or, with a '.', route to the wrong backend). All
/// legitimate names are k8s object names / game_versions.name values, i.e.
/// DNS labels (ls F35).
fn validate_version_name(name: &str) -> Result<(), GameClientError> {
    let ok = !name.is_empty()
        && name.len() <= 63
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-');
    if ok {
        Ok(())
    } else {
        Err(GameClientError::InvalidVersionName {
            name: name.to_string(),
        })
    }
}

async fn send_with_retry(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    request: &Request,
    config: &RetryConfig,
) -> Result<reqwest::Response, GameClientError> {
    let host = format!("{version_name}.games.internal");
    let mut attempt: u32 = 0;
    loop {
        #[cfg_attr(not(feature = "sentry"), allow(unused_mut))]
        let mut request_builder = client
            .post(uri)
            .header(reqwest::header::HOST, &host)
            .json(request);

        #[cfg(feature = "sentry")]
        {
            let mut trace_headers: Vec<(&str, String)> = Vec::new();
            sentry::configure_scope(|scope| {
                if let Some(span) = scope.get_span() {
                    trace_headers.extend(span.iter_headers());
                }
            });
            for (k, v) in trace_headers {
                request_builder = request_builder.header(k, v);
            }
        }

        match request_builder.send().await {
            Ok(res) => return Ok(res),
            Err(e) => {
                let retryable = e.is_connect() || e.is_timeout() || e.is_request();
                attempt += 1;
                if !retryable || attempt >= config.max_attempts {
                    return Err(e.into());
                }
            }
        }
        let delay = backoff_delay(attempt - 1, config);
        tokio::time::sleep(delay).await;
    }
}

async fn request_with_config(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    request: &Request,
    config: &RetryConfig,
) -> Result<Response, GameClientError> {
    validate_version_name(version_name)?;
    let call = async {
        let res = send_with_retry(client, uri, version_name, request, config).await?;
        let status = res.status();
        if !status.is_success() {
            // Do not read or retain the body: it may carry private state.
            return Err(GameClientError::HttpStatus { status });
        }
        let body = res.text().await.map_err(GameClientError::Transport)?;
        let resp: Response = serde_json::from_str(&body)
            .map_err(|source| GameClientError::ParseResponse { source })?;
        match resp {
            Response::SystemError { message } => Err(GameClientError::SystemError { message }),
            Response::UserError { message } => Err(GameClientError::UserError { message }),
            other => Ok(other),
        }
    };
    tokio::time::timeout(config.request_timeout, call)
        .await
        .map_err(|_| GameClientError::Timeout {
            after: config.request_timeout,
        })?
}

#[tracing::instrument(name = "game_service_request", skip(client, request), fields(game.uri = %uri))]
pub async fn request(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    request: &Request,
) -> Result<Response, GameClientError> {
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
) -> Result<RenderResponse, GameClientError> {
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
) -> Result<RenderResponse, GameClientError> {
    match request(client, uri, version_name, &Request::PubRender { game }).await? {
        Response::PubRender { render } => Ok(render.into()),
        _ => Err(GameClientError::UnexpectedResponse {
            request: "PubRender",
        }),
    }
}

pub async fn player_render(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    game: String,
    player: usize,
) -> Result<RenderResponse, GameClientError> {
    match request(
        client,
        uri,
        version_name,
        &Request::PlayerRender { player, game },
    )
    .await?
    {
        Response::PlayerRender { render } => Ok(render.into()),
        _ => Err(GameClientError::UnexpectedResponse {
            request: "PlayerRender",
        }),
    }
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub pub_state_yaml: String,
    pub player_state_yaml: String,
    pub data_docs: String,
    pub basic_strategy: String,
    pub advanced_strategy: String,
    pub command_spec: Option<CommandSpec>,
    pub rules: String,
}

fn json_to_yaml(json: &str) -> Result<String, GameClientError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(GameClientError::StateJson)?;
    serde_yaml_ng::to_string(&value).map_err(GameClientError::from)
}

/// Fetches everything the bot needs for a turn, using only the redaction
/// boundary render endpoints (`PubRender`/`PlayerRender`) plus the separate
/// rules/strategy/data-docs endpoints. Deliberately never requests
/// `Status`: `Response::Status` carries the full `GameResponse` (including
/// the raw `points` of every seat, bypassing the per-player redaction
/// boundary), so the bot must not ask for it (F-194).
pub async fn fetch_game_data(
    client: &reqwest::Client,
    uri: &str,
    version_name: &str,
    game: String,
    player: usize,
    interface_version: i32,
) -> Result<GameData, GameClientError> {
    let public_render = pub_render(client, uri, version_name, game.clone()).await?;
    let player_render = player_render(client, uri, version_name, game.clone(), player).await?;

    let pub_state_yaml = json_to_yaml(&public_render.state)?;
    let player_state_yaml = json_to_yaml(&player_render.state)?;
    let command_spec = player_render.command_spec.clone();

    let rules_fut = async {
        match request(client, uri, version_name, &Request::Rules).await? {
            Response::Rules { rules } => Ok(rules),
            _ => Err(GameClientError::UnexpectedResponse { request: "Rules" }),
        }
    };

    let (data_docs, basic_strategy, advanced_strategy, rules) = if interface_version >= 2 {
        let dd_fut = async {
            match request(
                client,
                uri,
                version_name,
                &Request::DataDocs { game: game.clone() },
            )
            .await?
            {
                Response::DataDocs { data_docs } => Ok(data_docs),
                _ => Err(GameClientError::UnexpectedResponse {
                    request: "DataDocs",
                }),
            }
        };
        let bs_fut = async {
            match request(
                client,
                uri,
                version_name,
                &Request::BasicStrategy {
                    game: game.clone(),
                    player,
                },
            )
            .await?
            {
                Response::BasicStrategy { strategy } => Ok(strategy),
                _ => Err(GameClientError::UnexpectedResponse {
                    request: "BasicStrategy",
                }),
            }
        };
        let as_fut = async {
            match request(
                client,
                uri,
                version_name,
                &Request::AdvancedStrategy {
                    game: game.clone(),
                    player,
                },
            )
            .await?
            {
                Response::AdvancedStrategy { strategy } => Ok(strategy),
                _ => Err(GameClientError::UnexpectedResponse {
                    request: "AdvancedStrategy",
                }),
            }
        };
        tokio::try_join!(dd_fut, bs_fut, as_fut, rules_fut)?
    } else {
        let placeholder = "Not supported in game interface V1".to_string();
        let rules = rules_fut.await?;
        (placeholder.clone(), placeholder.clone(), placeholder, rules)
    };

    Ok(GameData {
        pub_state_yaml,
        player_state_yaml,
        data_docs,
        basic_strategy,
        advanced_strategy,
        command_spec,
        rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, http::StatusCode, routing::post};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::net::TcpListener;

    fn request_kind(request: &Request) -> &'static str {
        match request {
            Request::Status { .. } => "Status",
            Request::PubRender { .. } => "PubRender",
            Request::PlayerRender { .. } => "PlayerRender",
            Request::Rules => "Rules",
            Request::DataDocs { .. } => "DataDocs",
            Request::BasicStrategy { .. } => "BasicStrategy",
            Request::AdvancedStrategy { .. } => "AdvancedStrategy",
            _ => "Other",
        }
    }

    fn tiny_config() -> RetryConfig {
        RetryConfig {
            base_delay: Duration::from_millis(5),
            multiplier: 2.0,
            cap: Duration::from_millis(20),
            max_attempts: 3,
            request_timeout: Duration::from_secs(90),
        }
    }

    /// F-14 regression: deterministic retry recovery. A raw TCP server drops
    /// the first connection after reading the request (a connection-reset
    /// transport failure) and serves a valid response on the second. No
    /// sleeps or startup races - the server behavior is controlled per
    /// connection, so exactly one retry must reach the recovery response.
    #[tokio::test]
    async fn test_retry_reaches_controlled_recovery() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let accepted2 = accepted.clone();
        tokio::spawn(async move {
            let mut conn = 0;
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                conn += 1;
                accepted2.fetch_add(1, Ordering::SeqCst);
                let mut seen: Vec<u8> = vec![];
                let mut buf = [0u8; 8192];
                loop {
                    let n = socket.read(&mut buf).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    seen.extend_from_slice(&buf[..n]);
                    if seen.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                if conn == 1 {
                    // Reset before any response bytes: retryable.
                    drop(socket);
                    continue;
                }
                let body = serde_json::to_string(&Response::PubRender {
                    render: PubRender {
                        pub_state: "pub".to_string(),
                        render: "render".to_string(),
                    },
                })
                .unwrap();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(resp.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
            }
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
        assert!(
            matches!(resp, Ok(Response::PubRender { .. })),
            "expected retry to reach the recovery response, got {:?}",
            resp
        );
        assert_eq!(
            2,
            accepted.load(Ordering::SeqCst),
            "expected exactly one reset followed by one successful retry"
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
        let err = resp.unwrap_err();
        assert!(
            matches!(err, GameClientError::HttpStatus { status, .. } if status.as_u16() == 500),
            "expected HttpStatus 500, got: {err}"
        );
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
            request_timeout: Duration::from_secs(90),
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
            request_timeout: Duration::from_secs(90),
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
            request_timeout: Duration::from_secs(90),
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

    #[tokio::test]
    async fn test_sends_version_host_header() {
        use axum::http::HeaderMap;
        // Echo the received Host header back in pub_state so the assertion can
        // see exactly what the client sent.
        let app = Router::new().route(
            "/",
            post(
                |headers: HeaderMap, Json(_payload): Json<Request>| async move {
                    let host = headers
                        .get(axum::http::header::HOST)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    Json(Response::PubRender {
                        render: PubRender {
                            pub_state: host,
                            render: String::new(),
                        },
                    })
                },
            ),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let resp = pub_render(&client, &uri, "acquire-1", "g".to_string())
            .await
            .expect("request failed");
        assert_eq!(
            resp.state, "acquire-1.games.internal",
            "client must send Host {{version_name}}.games.internal for KEDA interceptor routing"
        );
    }

    fn mock_game_response(Json(payload): Json<Request>) -> Json<Response> {
        Json(match payload {
            Request::PubRender { .. } => Response::PubRender {
                render: PubRender {
                    pub_state: r#"{"board":"empty","round":1}"#.to_string(),
                    render: "render".to_string(),
                },
            },
            Request::PlayerRender { player, .. } => Response::PlayerRender {
                render: match player {
                    0 => PlayerRender {
                        player_state: r#"{"hand":["A","K"],"score":10}"#.to_string(),
                        render: "p0".to_string(),
                        command_spec: None,
                    },
                    _ => PlayerRender {
                        player_state: r#"{"hand":["Q"],"score":5}"#.to_string(),
                        render: "p1".to_string(),
                        command_spec: None,
                    },
                },
            },
            Request::DataDocs { .. } => Response::DataDocs {
                data_docs: "V2 data docs".to_string(),
            },
            Request::BasicStrategy { .. } => Response::BasicStrategy {
                strategy: "V2 basic strategy".to_string(),
            },
            Request::AdvancedStrategy { .. } => Response::AdvancedStrategy {
                strategy: "V2 advanced strategy".to_string(),
            },
            Request::Rules => Response::Rules {
                rules: "Game rules here".to_string(),
            },
            _ => Response::SystemError {
                message: "unsupported in mock".to_string(),
            },
        })
    }

    fn mock_game_server() -> Router {
        Router::new().route(
            "/",
            post(|req: Json<Request>| async move { mock_game_response(req) }),
        )
    }

    fn delayed_mock_game_server(delay: Duration) -> Router {
        Router::new().route(
            "/",
            post(move |req: Json<Request>| async move {
                tokio::time::sleep(delay).await;
                mock_game_response(req)
            }),
        )
    }

    async fn start_mock_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, mock_game_server()).await.unwrap();
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn test_fetch_game_data_v1_uses_placeholders() {
        let uri = start_mock_server().await;
        let client = reqwest::Client::new();
        let data = fetch_game_data(&client, &uri, "test-v1", "{}".to_string(), 0, 1)
            .await
            .expect("fetch_game_data failed");
        assert_eq!(data.data_docs, "Not supported in game interface V1");
        assert_eq!(data.basic_strategy, "Not supported in game interface V1");
        assert_eq!(data.advanced_strategy, "Not supported in game interface V1");
        assert_eq!(data.rules, "Game rules here");
    }

    #[tokio::test]
    async fn test_fetch_game_data_v2_returns_real_content() {
        let uri = start_mock_server().await;
        let client = reqwest::Client::new();
        let data = fetch_game_data(&client, &uri, "test-v2", "{}".to_string(), 0, 2)
            .await
            .expect("fetch_game_data failed");
        assert_eq!(data.data_docs, "V2 data docs");
        assert_eq!(data.basic_strategy, "V2 basic strategy");
        assert_eq!(data.advanced_strategy, "V2 advanced strategy");
        assert_eq!(data.rules, "Game rules here");
    }

    #[tokio::test]
    async fn test_fetch_game_data_yaml_serialization() {
        let uri = start_mock_server().await;
        let client = reqwest::Client::new();
        let data = fetch_game_data(&client, &uri, "test-v1", "{}".to_string(), 0, 1)
            .await
            .expect("fetch_game_data failed");
        assert!(data.pub_state_yaml.contains("board: empty"));
        assert!(data.pub_state_yaml.contains("round: 1"));
        assert!(data.player_state_yaml.contains("score: 10"));
    }

    #[tokio::test]
    async fn test_system_error_maps_to_typed_variant() {
        let app = Router::new().route(
            "/",
            post(|Json(_): Json<Request>| async move {
                Json(Response::SystemError {
                    message: "state exploded".to_string(),
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let err = pub_render(&client, &uri, "test-game-1", "g".to_string())
            .await
            .unwrap_err();
        match err {
            GameClientError::SystemError { message } => assert_eq!("state exploded", message),
            e => panic!("expected SystemError, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_wrong_variant_maps_to_unexpected_response() {
        let app = Router::new().route(
            "/",
            post(|Json(_): Json<Request>| async move {
                Json(Response::Rules {
                    rules: "not a render".to_string(),
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let err = pub_render(&client, &uri, "test-game-1", "g".to_string())
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                GameClientError::UnexpectedResponse {
                    request: "PubRender"
                }
            ),
            "got {:?}",
            err
        );
    }

    /// F-192 regression: an error raised from a full sensitive response body
    /// must not expose body content through either `Display` or `Debug`.
    /// The mock answers a 500 with a complete game body carrying private
    /// hand state, seat identifiers, a prompt, and a secret; the test formats
    /// the resulting `HttpStatus` error both ways and proves every sentinel
    /// is absent while the HTTP status survives.
    #[tokio::test]
    async fn test_http_status_error_redacts_response_body() {
        const SENTINELS: &[&str] = &[
            "OPPONENT_PRIVATE_STATE_SENTINEL",
            "SEAT_NAME_SENTINEL",
            "PROMPT_SENTINEL",
            "9876543.5",
            "hunter2",
        ];
        let body = r#"{"game":{"state":{"players":[{"name":"SEAT_NAME_SENTINEL","hand":["OPPONENT_PRIVATE_STATE_SENTINEL"]}]},"points":[9876543.5,9876543.5]},"prompt":"PROMPT_SENTINEL","password":"hunter2"}"#;
        let app = Router::new().route(
            "/",
            post(move |_req_body: String| async move { (StatusCode::INTERNAL_SERVER_ERROR, body) }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let err = request(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
        )
        .await
        .unwrap_err();
        match &err {
            GameClientError::HttpStatus { status } => {
                assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, *status)
            }
            e => panic!("expected HttpStatus, got {:?}", e),
        }
        let display = err.to_string();
        let debug = format!("{err:?}");
        for sentinel in SENTINELS {
            assert!(
                !display.contains(sentinel),
                "Display leaked {sentinel:?}: {display}"
            );
            assert!(
                !debug.contains(sentinel),
                "Debug leaked {sentinel:?}: {debug}"
            );
        }
    }

    /// F-192 regression for the parse path: a malformed 200 response whose
    /// body carries sentinels produces a `ParseResponse` error whose
    /// `Display` and `Debug` never expose the body content.
    #[tokio::test]
    async fn test_parse_error_redacts_response_body() {
        const SENTINELS: &[&str] = &["OPPONENT_PRIVATE_STATE_SENTINEL", "9876543.5", "hunter2"];
        let body = r#"{"hand":["OPPONENT_PRIVATE_STATE_SENTINEL"],"points":[9876543.5],"password":"hunter2""#;
        let app = Router::new().route(
            "/",
            post(move |_req_body: String| async move { (StatusCode::OK, body) }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let err = request(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, GameClientError::ParseResponse { .. }),
            "expected ParseResponse, got {:?}",
            err
        );
        let display = err.to_string();
        let debug = format!("{err:?}");
        for sentinel in SENTINELS {
            assert!(
                !display.contains(sentinel),
                "Display leaked {sentinel:?}: {display}"
            );
            assert!(
                !debug.contains(sentinel),
                "Debug leaked {sentinel:?}: {debug}"
            );
        }
    }

    /// F-11 regression: an in-band `Response::UserError` maps to a typed
    /// error that surfaces exactly the service message, with no response
    /// body retained or exposed.
    #[tokio::test]
    async fn test_user_error_maps_to_typed_error_with_message() {
        let app = Router::new().route(
            "/",
            post(|Json(_): Json<Request>| async move {
                Json(Response::UserError {
                    message: "expected buy or done".to_string(),
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let err = request(
            &client,
            &uri,
            "test-game-1",
            &Request::PubRender {
                game: "g".to_string(),
            },
        )
        .await
        .unwrap_err();
        match &err {
            GameClientError::UserError { message } => {
                assert_eq!("expected buy or done", message)
            }
            e => panic!("expected UserError, got {:?}", e),
        }
        assert!(
            err.to_string().contains("expected buy or done"),
            "UserError must surface the service message, got: {err}"
        );
        assert_eq!(
            format!("{err:?}"),
            "UserError { message: \"expected buy or done\" }",
            "Debug must retain exactly the message and no response body"
        );
    }

    /// F-12 regression: the timeout ceiling is per complete call, not per
    /// attempt. The first two connections are reset after the request (fast
    /// retryable failures that consume two backoff sleeps); the third
    /// returns response headers but never the body, so the body read hangs.
    /// The single whole-call ceiling must bound the two retries, both
    /// backoff sleeps, and the hanging body read together: the call must end
    /// at exactly `request_timeout` of elapsed time, never near "ceiling per
    /// attempt plus body-read ceiling" (the pre-fix code spends the two
    /// backoff sleeps AND a second full ceiling on the body read, ending at
    /// ~650-800ms).
    ///
    /// The clock is paused (test-util) and driven forward 1ms at a time, so
    /// the assertion is exact in virtual time and independent of scheduler
    /// wakeup variation (F-12's "does not rely on scheduler timing"). The
    /// loop yields to the reactor on every step, so the loopback connection
    /// resets and the third request complete in real time long before the
    /// 500ms ceiling.
    #[tokio::test(start_paused = true)]
    async fn test_whole_call_timeout_covers_attempts_backoff_and_body_read() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = Arc::new(AtomicUsize::new(0));
        let accepted2 = accepted.clone();
        tokio::spawn(async move {
            let mut conn = 0;
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                conn += 1;
                accepted2.fetch_add(1, Ordering::SeqCst);
                let mut seen: Vec<u8> = vec![];
                let mut buf = [0u8; 8192];
                loop {
                    let n = socket.read(&mut buf).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    seen.extend_from_slice(&buf[..n]);
                    if seen.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                if conn < 3 {
                    drop(socket);
                    continue;
                }
                // Headers promise a body that never arrives.
                let resp = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 100000\r\nconnection: close\r\n\r\n";
                socket.write_all(resp.as_bytes()).await.unwrap();
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });

        let client = reqwest::Client::new();
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            multiplier: 2.0,
            cap: Duration::from_millis(200),
            max_attempts: 3,
            request_timeout: Duration::from_millis(500),
        };
        let uri = format!("http://{}", addr);
        let start = tokio::time::Instant::now();
        let task = tokio::spawn(async move {
            request_with_config(
                &client,
                &uri,
                "test-game-1",
                &Request::PubRender {
                    game: "g".to_string(),
                },
                &config,
            )
            .await
        });
        // Let the spawned call anchor its whole-call ceiling at the paused
        // clock's zero point so the elapsed assertion is exact.
        tokio::task::yield_now().await;
        while !task.is_finished() {
            tokio::time::advance(Duration::from_millis(1)).await;
            // Let a timer-woken task run to completion before re-checking, so
            // the elapsed assertion sees the exact ceiling boundary.
            tokio::task::yield_now().await;
            if start.elapsed() > Duration::from_secs(2) {
                panic!("whole-call timeout did not fire; task still running");
            }
        }
        let resp = task.await.unwrap();
        let elapsed = start.elapsed();
        match resp {
            Err(GameClientError::Timeout { after }) => {
                assert_eq!(Duration::from_millis(500), after)
            }
            r => panic!("expected Timeout, got {:?}", r),
        }
        assert_eq!(
            3,
            accepted.load(Ordering::SeqCst),
            "expected two retries plus the hanging third attempt within the ceiling"
        );
        assert_eq!(
            Duration::from_millis(500),
            elapsed,
            "whole-call ceiling must end at exactly request_timeout, never per-attempt + body-read (pre-fix ends ~650-800ms); elapsed={elapsed:?}"
        );
    }

    #[tokio::test]
    async fn test_retry_on_connection_reset_mid_request() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut seen: Vec<u8> = vec![];
            let mut buf = [0u8; 8192];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                seen.extend_from_slice(&buf[..n]);
                if seen.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = serde_json::to_string(&Response::PubRender {
                render: PubRender {
                    pub_state: "pub".to_string(),
                    render: "render".to_string(),
                },
            })
            .unwrap();
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(resp.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
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
        assert!(
            matches!(resp, Ok(Response::PubRender { .. })),
            "mid-request reset must be retried to success, got {:?}",
            resp
        );
    }

    #[tokio::test]
    async fn test_invalid_version_name_rejected_before_send() {
        let client = reqwest::Client::new();
        for bad in [
            "",
            "has.dot",
            "under_score",
            "-leading",
            "trailing-",
            "has space",
        ] {
            let err = request(&client, "http://127.0.0.1:1", bad, &Request::PlayerCounts)
                .await
                .unwrap_err();
            assert!(
                matches!(err, GameClientError::InvalidVersionName { .. }),
                "{bad:?} should be rejected, got {:?}",
                err
            );
        }
    }

    /// F-194 regression: `fetch_game_data` must request only the redaction
    /// boundary render endpoints (PubRender + PlayerRender), never `Status`.
    /// The mock answers a `Status` request with a distinctive hidden points
    /// value and records every request it receives; the test asserts the
    /// endpoint choices and that the hidden value never reaches any field of
    /// the returned game data.
    #[tokio::test]
    async fn test_fetch_game_data_never_requests_status_and_omits_hidden_points() {
        const HIDDEN_POINTS: f32 = 9876543.0;
        let seen = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let seen2 = Arc::clone(&seen);
        let app = Router::new().route(
            "/",
            post(move |Json(payload): Json<Request>| async move {
                seen2.lock().unwrap().push(request_kind(&payload));
                Json(match payload {
                    Request::Status { .. } => Response::Status {
                        game: brdgme_cmd::api::GameResponse {
                            state: "{}".to_string(),
                            points: vec![HIDDEN_POINTS, HIDDEN_POINTS],
                            status: brdgme_game::Status::Active {
                                whose_turn: vec![0],
                                eliminated: vec![],
                            },
                        },
                        public_render: PubRender {
                            pub_state: "{}".to_string(),
                            render: String::new(),
                        },
                        player_renders: vec![],
                    },
                    Request::PubRender { .. } => Response::PubRender {
                        render: PubRender {
                            pub_state: r#"{"board":"empty"}"#.to_string(),
                            render: String::new(),
                        },
                    },
                    Request::PlayerRender { player, .. } => Response::PlayerRender {
                        render: PlayerRender {
                            player_state: r#"{"hand":["A","K"]}"#.to_string(),
                            render: format!("p{}", player),
                            command_spec: None,
                        },
                    },
                    Request::DataDocs { .. } => Response::DataDocs {
                        data_docs: "docs".to_string(),
                    },
                    Request::BasicStrategy { .. } => Response::BasicStrategy {
                        strategy: "bs".to_string(),
                    },
                    Request::AdvancedStrategy { .. } => Response::AdvancedStrategy {
                        strategy: "as".to_string(),
                    },
                    Request::Rules => Response::Rules {
                        rules: "rules".to_string(),
                    },
                    _ => Response::SystemError {
                        message: "unsupported in mock".to_string(),
                    },
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let data = fetch_game_data(&client, &uri, "test-v2", "{}".to_string(), 0, 2)
            .await
            .expect("fetch_game_data failed");

        let seen = seen.lock().unwrap();
        assert!(
            seen.contains(&"PubRender"),
            "fetch_game_data must request PubRender, saw: {:?}",
            seen
        );
        assert!(
            seen.contains(&"PlayerRender"),
            "fetch_game_data must request PlayerRender, saw: {:?}",
            seen
        );
        assert!(
            !seen.contains(&"Status"),
            "fetch_game_data must never request Status, saw: {:?}",
            seen
        );
        drop(seen);

        let all_content = format!(
            "{}{}{}{}{}{}",
            data.pub_state_yaml,
            data.player_state_yaml,
            data.data_docs,
            data.basic_strategy,
            data.advanced_strategy,
            data.rules
        );
        assert!(
            !all_content.contains(&format!("{HIDDEN_POINTS}")),
            "hidden Status points value must not reach game data: {:?}",
            all_content
        );
    }

    #[tokio::test]
    async fn test_fetch_game_data_v2_parallelizes_followups() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(
                listener,
                delayed_mock_game_server(Duration::from_millis(100)),
            )
            .await
            .unwrap();
        });
        let client = reqwest::Client::new();
        let uri = format!("http://{}", addr);
        let start = std::time::Instant::now();
        let data = fetch_game_data(&client, &uri, "test-v2", "{}".to_string(), 0, 2)
            .await
            .expect("fetch_game_data failed");
        assert_eq!(data.data_docs, "V2 data docs");
        assert!(
            start.elapsed() < Duration::from_millis(400),
            "followup requests appear sequential: {:?}",
            start.elapsed()
        );
    }
}
