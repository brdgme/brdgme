use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct InternalCommandRequest {
    pub player_position: usize,
    pub command: String,
}

pub async fn internal_play_command(
    State(pool): State<PgPool>,
    State(broadcaster): State<crate::websocket::GameBroadcaster>,
    State(http_client): State<reqwest::Client>,
    Path(id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<InternalCommandRequest>,
) -> impl IntoResponse {
    let expected_key = match std::env::var("INTERNAL_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_API_KEY not configured",
            )
                .into_response()
        }
    };
    let provided_key = match headers.get("X-Internal-Key").and_then(|v| v.to_str().ok()) {
        Some(k) => k.to_string(),
        None => return (StatusCode::UNAUTHORIZED, "Missing X-Internal-Key header").into_response(),
    };
    if provided_key != expected_key {
        return (StatusCode::UNAUTHORIZED, "Invalid internal key").into_response();
    }

    match super::execute_command(
        &pool,
        &http_client,
        &broadcaster,
        id,
        payload.player_position,
        payload.command,
    )
    .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub fn api_routes() -> axum::Router<crate::state::AppState> {
    axum::Router::new().route(
        "/internal/game/{id}/command",
        axum::routing::post(internal_play_command),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use crate::websocket::GameBroadcaster;
    use axum::body::Body;
    use axum::http::{header, Request};
    use tower::ServiceExt;

    async fn make_broadcaster() -> GameBroadcaster {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let client = redis::Client::open(redis_url).unwrap();
        let conn = client.get_multiplexed_async_connection().await.unwrap();
        GameBroadcaster::new(conn, client)
    }

    async fn test_app(pool: PgPool) -> axum::Router {
        let leptos_options = leptos::config::get_configuration(None)
            .unwrap()
            .leptos_options;
        let session_layer = crate::auth::session::create_session_layer(&pool).await;
        let state = AppState {
            leptos_options,
            pool: pool.clone(),
            broadcaster: make_broadcaster().await,
            http_client: reqwest::Client::new(),
            resend: None,
            login_rate_limiter: crate::auth::rate_limit::build_login_rate_limiter(),
            confirm_rate_limiter: crate::auth::rate_limit::build_confirm_rate_limiter(),
        };
        axum::Router::new()
            .nest("/api", api_routes())
            .layer(session_layer)
            .with_state(state)
    }

    #[sqlx::test]
    async fn internal_command_requires_matching_key(pool: PgPool) {
        std::env::set_var("INTERNAL_API_KEY", "secret-key");
        let app = test_app(pool).await;
        let id = Uuid::new_v4();
        let body = serde_json::json!({ "player_position": 0, "command": "abc" }).to_string();

        // Correct key: reaches execute_command and fails only because the
        // game doesn't exist (proves auth passed and the request was
        // executed, not rejected).
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/internal/game/{}/command", id))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("X-Internal-Key", "secret-key")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Wrong key.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/internal/game/{}/command", id))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("X-Internal-Key", "wrong-key")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // Missing key.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/internal/game/{}/command", id))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // INTERNAL_API_KEY unset: rejects even the "right" key from before.
        std::env::remove_var("INTERNAL_API_KEY");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/internal/game/{}/command", id))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("X-Internal-Key", "secret-key")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
