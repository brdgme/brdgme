//! RFC 8058 one-click unsubscribe endpoints (WP-58). `POST` is the one-click
//! target advertised by the `List-Unsubscribe-Post: List-Unsubscribe=One-Click`
//! header; `GET` renders a confirmation page with a POST form because mail
//! clients and link scanners prefetch GET URLs and must not mutate state.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct UnsubscribeQuery {
    t: String,
}

/// `POST /api/unsubscribe/{kind}` - the RFC 8058 one-click target. Returns the
/// same 200 body whether the token matched a user or not, so a token's validity
/// is never leaked to a caller probing tokens.
pub async fn unsubscribe_post(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Query(params): Query<UnsubscribeQuery>,
) -> impl IntoResponse {
    let Some(kind) = crate::email::render::EmailKind::from_slug(&kind) else {
        return (
            StatusCode::BAD_REQUEST,
            "Unknown subscription type.".to_string(),
        );
    };
    match crate::db::disable_email_pref_by_unsubscribe_token(&state.pool, &params.t, kind).await {
        Ok(_) => (
            StatusCode::OK,
            "Unsubscribed. You will no longer receive these emails.".to_string(),
        ),
        Err(err) => {
            tracing::warn!("unsubscribe: failed to disable email pref: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Temporary error; please retry.".to_string(),
            )
        }
    }
}

/// `GET /api/unsubscribe/{kind}` - renders a confirmation page with a POST form.
/// Must never mutate: scanners and mail clients prefetch GET URLs.
pub async fn unsubscribe_get(
    Path(kind): Path<String>,
    Query(params): Query<UnsubscribeQuery>,
) -> Response {
    let Some(kind) = crate::email::render::EmailKind::from_slug(&kind) else {
        return (StatusCode::BAD_REQUEST, "Unknown subscription type.").into_response();
    };
    let slug = crate::email::render::escape_html_attr(kind.slug());
    let token = crate::email::render::escape_html_attr(&params.t);
    let label = kind.link_label();
    let html = format!(
        "<html><body style=\"font-family:sans-serif;padding:24px;\"><p>{label}</p><form method=\"post\" action=\"/api/unsubscribe/{slug}?t={token}\"><button type=\"submit\">Unsubscribe</button></form></body></html>"
    );
    (StatusCode::OK, Html(html)).into_response()
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use sqlx::PgPool;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn seed_user(pool: &PgPool) -> Uuid {
        sqlx::query_scalar("INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id")
            .bind(format!("u-{}", Uuid::new_v4()))
            .bind(Vec::<String>::new())
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn seed_with_token(pool: &PgPool, token: &str) -> Uuid {
        let user_id = seed_user(pool).await;
        sqlx::query(
            "UPDATE users SET unsubscribe_token = $1, turn_emails_enabled = true, invite_emails_enabled = true, reminder_emails_enabled = true WHERE id = $2",
        )
        .bind(token)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        user_id
    }

    async fn make_state(pool: PgPool) -> AppState {
        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let client = async_nats::connect(&nats_url).await.unwrap();
        let jetstream = async_nats::jetstream::new(client.clone());
        AppState {
            leptos_options: leptos::config::LeptosOptions::builder()
                .output_name("web")
                .build(),
            pool,
            broadcaster: crate::websocket::GameBroadcaster::new(client),
            http_client: reqwest::Client::new(),
            resend: None,
            jetstream,
        }
    }

    async fn app(pool: &PgPool) -> Router {
        Router::new()
            .route(
                "/api/unsubscribe/{kind}",
                get(unsubscribe_get).post(unsubscribe_post),
            )
            .with_state(make_state(pool.clone()).await)
    }

    async fn prefs(pool: &PgPool, user_id: Uuid) -> (bool, bool, bool) {
        crate::db::get_user_email_prefs(pool, user_id)
            .await
            .unwrap()
    }

    async fn post_unsubscribe(app: &Router, kind: &str, token: &str) -> StatusCode {
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/unsubscribe/{kind}?t={token}"))
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(req).await.unwrap().status()
    }

    async fn get_unsubscribe(app: &Router, kind: &str, token: &str) -> StatusCode {
        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/unsubscribe/{kind}?t={token}"))
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(req).await.unwrap().status()
    }

    #[sqlx::test]
    async fn post_disables_only_intended_pref_for_each_slug(pool: PgPool) {
        let app = app(&pool).await;
        for (slug, token) in [
            ("turn", "tok-turn"),
            ("game", "tok-game"),
            ("reminder", "tok-reminder"),
            ("invite", "tok-invite"),
        ] {
            let user_id = seed_with_token(&pool, token).await;
            assert_eq!(post_unsubscribe(&app, slug, token).await, StatusCode::OK);
            let (turn, invite, reminder) = prefs(&pool, user_id).await;
            let expected = match slug {
                "turn" | "game" => (false, true, true),
                "reminder" => (true, true, false),
                "invite" => (true, false, true),
                _ => unreachable!(),
            };
            assert_eq!(
                (turn, invite, reminder),
                expected,
                "slug {slug} must disable only its own live preference"
            );
        }
    }

    #[sqlx::test]
    async fn post_is_idempotent(pool: PgPool) {
        let app = app(&pool).await;
        let user_id = seed_with_token(&pool, "tok-twice").await;

        assert_eq!(
            post_unsubscribe(&app, "reminder", "tok-twice").await,
            StatusCode::OK
        );
        assert_eq!(
            post_unsubscribe(&app, "reminder", "tok-twice").await,
            StatusCode::OK
        );

        let (_, _, reminder) = prefs(&pool, user_id).await;
        assert!(!reminder);
    }

    #[sqlx::test]
    async fn post_with_unknown_token_changes_nothing(pool: PgPool) {
        let app = app(&pool).await;
        let user_id = seed_with_token(&pool, "tok-real").await;

        assert_eq!(
            post_unsubscribe(&app, "reminder", "tok-does-not-exist").await,
            StatusCode::OK
        );

        assert_eq!(prefs(&pool, user_id).await, (true, true, true));
    }

    #[sqlx::test]
    async fn get_mutates_nothing(pool: PgPool) {
        let app = app(&pool).await;
        let user_id = seed_with_token(&pool, "tok-get").await;

        assert_eq!(
            get_unsubscribe(&app, "reminder", "tok-get").await,
            StatusCode::OK
        );

        assert_eq!(prefs(&pool, user_id).await, (true, true, true));
    }

    #[sqlx::test]
    async fn unknown_kind_is_400(pool: PgPool) {
        let app = app(&pool).await;

        assert_eq!(
            post_unsubscribe(&app, "bogus", "tok").await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            get_unsubscribe(&app, "bogus", "tok").await,
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn get_reflects_slug_and_token_escaped() {
        let resp = unsubscribe_get(
            Path("reminder".to_string()),
            Query(UnsubscribeQuery {
                t: "a&b\"c<d'e".to_string(),
            }),
        );
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(
            html.contains("/api/unsubscribe/reminder?t=a&amp;b&quot;c&lt;d&#39;e"),
            "reflected action href must be escaped: {html}"
        );
        assert!(
            !html.contains("t=a&b\"c<d'e"),
            "raw token must not be reflected: {html}"
        );
    }
}
