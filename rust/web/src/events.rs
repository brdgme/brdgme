use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::Stream;
use futures_util::stream::StreamExt;
use sqlx::postgres::PgPool;
use std::collections::HashSet;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::visibility_cache::VisibilityCache;
use crate::websocket::GameBroadcaster;

// Matches the VisibilityCache TTL: a revoked session is dropped from the live
// stream within one TTL, the same staleness bound already accepted for
// visibility changes. Must stay below the 45s bound in the AC1 regression test.
const SESSION_REVALIDATE_PERIOD: Duration = Duration::from_secs(30);

struct SseConnectionGuard;

impl SseConnectionGuard {
    fn new() -> Self {
        axum_prometheus::metrics::gauge!("sse_connections").increment(1.0);
        Self
    }
}

impl Drop for SseConnectionGuard {
    fn drop(&mut self) {
        axum_prometheus::metrics::gauge!("sse_connections").decrement(1.0);
    }
}

// Wraps the response stream so that dropping the SSE body (Axum drops it when
// the client disconnects) fires the per-connection cancellation token. The
// spawned task selects on that token, binding task and NATS-subscription
// lifetime to the connection instead of leaking until the next visible event.
struct SseStream<S> {
    inner: S,
    disconnected: CancellationToken,
}

impl<S: Stream + Unpin> Stream for SseStream<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}

impl<S> Drop for SseStream<S> {
    fn drop(&mut self) {
        self.disconnected.cancel();
    }
}

pub async fn events_handler(
    session: tower_sessions::Session,
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let (viewer, auth_token_id): (Option<Uuid>, Option<Uuid>) =
        match crate::auth::session::get_user_from_session(&session).await {
            Ok(Some(su)) => {
                match crate::auth::session::validate_session_token(&pool, su.auth_token_id).await {
                    Ok(true) => (Some(su.id), Some(su.auth_token_id)),
                    _ => (None, None),
                }
            }
            _ => (None, None),
        };

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let shutdown = broadcaster.shutdown.clone();
    let client = broadcaster.client.clone();
    let disconnected = CancellationToken::new();
    let task_disconnected = disconnected.clone();

    tokio::spawn(async move {
        let _guard = SseConnectionGuard::new();

        let mut game_sub = match client.subscribe("game.>").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("SSE NATS subscribe failed: {}", e);
                return;
            }
        };
        let mut proposal_sub = match client.subscribe("proposal.>").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("SSE NATS subscribe failed: {}", e);
                return;
            }
        };

        let mut cache = VisibilityCache::default();
        let mut revalidate = tokio::time::interval(SESSION_REVALIDATE_PERIOD);
        revalidate.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                msg = game_sub.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => break,
                    };
                    let payload = match std::str::from_utf8(&msg.payload) {
                        Ok(p) => p.to_string(),
                        Err(_) => continue,
                    };
                    let game_id: Option<Uuid> = msg.subject.as_str().strip_prefix("game.").and_then(|s| s.parse().ok());
                    if let Some(game_id) = game_id {
                        let pool = pool.clone();
                        let visible = cache.check_game(game_id, || crate::db::is_game_visible_to_viewer(&pool, game_id, viewer)).await;
                        if visible && tx.send(Ok(Event::default().event("game").data(payload))).is_err() {
                            break;
                        }
                    }
                }
                msg = proposal_sub.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => break,
                    };
                    let payload = match std::str::from_utf8(&msg.payload) {
                        Ok(p) => p.to_string(),
                        Err(_) => continue,
                    };
                    if let Some(viewer_id) = viewer {
                        let proposal_id: Option<Uuid> = msg.subject.as_str().strip_prefix("proposal.").and_then(|s| s.parse().ok());
                        if let Some(proposal_id) = proposal_id {
                            let pool = pool.clone();
                            let visible = cache.check_proposal(proposal_id, || crate::db::is_proposal_visible_to_user(&pool, proposal_id, viewer_id)).await;
                            if visible && tx.send(Ok(Event::default().event("proposal").data(payload))).is_err() {
                                break;
                            }
                        }
                    }
                }
                _ = revalidate.tick(), if auth_token_id.is_some() => {
                    if let Some(token_id) = auth_token_id {
                        match crate::auth::session::validate_session_token(&pool, token_id).await {
                            Ok(true) => {}
                            _ => break,
                        }
                    }
                }
                _ = task_disconnected.cancelled() => {
                    break;
                }
                _ = shutdown.cancelled() => {
                    break;
                }
            }
        }
    });

    Sse::new(SseStream {
        inner: UnboundedReceiverStream::new(rx),
        disconnected,
    })
    .keep_alive(KeepAlive::default())
}

pub async fn events_public_handler(
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    Query(params): Query<Vec<(String, String)>>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, axum::http::StatusCode>
{
    let mut requested_ids: HashSet<Uuid> = HashSet::new();
    for (key, value) in &params {
        if key != "topic" {
            continue;
        }
        let id_str = value
            .strip_prefix("game:")
            .ok_or(axum::http::StatusCode::BAD_REQUEST)?;
        let id: Uuid = id_str
            .parse()
            .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
        requested_ids.insert(id);
    }
    if requested_ids.is_empty() || requested_ids.len() > 16 {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let shutdown = broadcaster.shutdown.clone();
    let client = broadcaster.client.clone();
    let disconnected = CancellationToken::new();
    let task_disconnected = disconnected.clone();

    tokio::spawn(async move {
        let _guard = SseConnectionGuard::new();

        // Subscribe to exactly the requested game.{id} subjects (never game.>)
        // so an anonymous connection only receives what it asked for.
        let mut subs = Vec::with_capacity(requested_ids.len());
        for id in &requested_ids {
            match client.subscribe(format!("game.{id}")).await {
                Ok(s) => subs.push(s),
                Err(e) => {
                    tracing::error!("SSE NATS subscribe failed: {}", e);
                    return;
                }
            }
        }
        let mut game_sub = futures_util::stream::select_all(subs);

        let mut cache = VisibilityCache::default();

        loop {
            tokio::select! {
                msg = game_sub.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => break,
                    };
                    let payload = match std::str::from_utf8(&msg.payload) {
                        Ok(p) => p.to_string(),
                        Err(_) => continue,
                    };
                    let game_id: Option<Uuid> = msg.subject.as_str().strip_prefix("game.").and_then(|s| s.parse().ok());
                    if let Some(game_id) = game_id
                        && requested_ids.contains(&game_id)
                    {
                        let pool = pool.clone();
                        let visible = cache.check_game(game_id, || crate::db::is_game_publicly_visible(&pool, game_id)).await;
                        if visible && tx.send(Ok(Event::default().event("game").data(payload))).is_err() {
                            break;
                        }
                    }
                }
                _ = task_disconnected.cancelled() => {
                    break;
                }
                _ = shutdown.cancelled() => {
                    break;
                }
            }
        }
    });

    Ok(Sse::new(SseStream {
        inner: UnboundedReceiverStream::new(rx),
        disconnected,
    })
    .keep_alive(KeepAlive::default()))
}
