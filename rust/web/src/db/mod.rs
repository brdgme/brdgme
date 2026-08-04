//! Database access layer.
//!
//! # `updated_at` convention
//!
//! `migrations/001_initial_schema.sql:25-32` defines `update_updated_at()` and
//! attaches it as a BEFORE UPDATE trigger (001:392-446) to these 14 tables:
//! `users`, `user_emails`, `user_auth_tokens`, `friends`, `chats`,
//! `chat_users`, `chat_messages`, `game_types`, `game_type_users`,
//! `game_versions`, `games`, `game_players`, `game_logs`, `game_log_targets`.
//! The trigger overwrites `NEW.updated_at` unconditionally, so **never write
//! `updated_at` by hand in an UPDATE against one of those tables** - the
//! assignment is dead SQL (ws F36).
//!
//! Tables added by later migrations have `updated_at` columns but **no
//! trigger**: `bots` and `llm_providers` (013_bot_efficacy.sql:10,20) and
//! `game_proposals` / `game_proposal_players` (015_game_proposals.sql:8,22).
//! Manual `updated_at` maintenance on those tables is REQUIRED - see
//! `delete_game`, which nulls two `game_proposals` FK columns and must keep
//! its manual sets.
//!
//! Three other BEFORE UPDATE triggers are conditional and are NOT substitutes
//! for an explicit write: `update_finished_at` fires only on `is_finished`
//! false -> true (001:448-452), `update_is_turn_at` only on `is_turn`
//! false -> true (001:454-458), and `update_last_turn_at` only on `is_turn`
//! true -> false (001:460-464).
//!
//! # Module map
//!
//! This module is split into focused submodules, each declared privately and
//! glob-re-exported below so callers keep using `crate::db::*` paths:
//!
//! - `common` - row builders (`build_*_from_row`, `build_game_type_user`) and
//!   username/colour helpers (`validate_username`, `normalize_pref_color`,
//!   `choose_colors`, `cap_digest`).
//! - `game_types` - game-version and game-type lookups.
//! - `games` - game reads, extended game/player views, summaries, and logs.
//! - `game_write` - the game write path: create, concede, end, delete, undo,
//!   and the command-success update.
//! - `bots` - bot turn lookups and replacement-bot selection.
//! - `rating` - ELO math and the ranked-placing / rating-change writes.
//! - `users` - user reads, name/theme/pref/email-pref settings, and presence.
//! - `emails` - multiple-emails-per-account (`#22d`) reads and writes.
//! - `social` - friends and blocks (`#30`), opponent and game suggestions.
//! - `visibility` - invite policy and game-visibility predicates.
//! - `discovery` - public-index and friend-recent game discovery.
//! - `proposals` - open restart-proposal lookups.
//!
//! The `#[cfg(all(test, feature = "ssr"))] mod tests` block lives below in
//! `mod.rs`.
//!
//! Every production item is individually `#[cfg(feature = "ssr")]`-gated -
//! there is no module-level gate. The single exception is
//! `validate_username`, which is ungated so the client-side settings form and
//! the server fns share one definition. The other pure predicates
//! (`active_within_window`, `can_remove_email`, `can_switch_to_email`,
//! `is_expired_unverified`, `cap_digest`) are `ssr`-gated even though they are
//! pure; every caller is server-side, so leave them gated.
#[cfg(feature = "ssr")]
use anyhow::{Context, Result};
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;

pub use crate::game::server_fns::BotSlot;

mod bots;
mod common;
mod discovery;
mod emails;
mod game_types;
mod game_write;
mod games;
mod proposals;
mod rating;
mod social;
mod users;
mod visibility;

#[cfg(all(test, feature = "ssr"))]
pub(crate) mod test_support;

pub use bots::*;
pub use common::*;
pub use discovery::*;
pub use emails::*;
pub use game_types::*;
pub use game_write::*;
pub use games::*;
pub use proposals::*;
pub(crate) use rating::*;
pub use social::*;
pub use users::*;
pub use visibility::*;

#[cfg(feature = "ssr")]
pub async fn create_pool() -> Result<PgPool> {
    // F-103: propagate the missing-var error with context instead of panicking.
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

    let pool = PgPool::connect(&database_url).await?;

    Ok(pool)
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    #[sqlx::test]
    async fn migrations_apply_and_pool_connects(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[sqlx::test]
    async fn session_token_validation(pool: PgPool) {
        use crate::auth::session::{invalidate_auth_token, validate_session_token};

        let user = make_user(&pool, "session-user").await;
        let token_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)",
            token_id,
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();

        assert!(validate_session_token(&pool, token_id).await.unwrap());

        invalidate_auth_token(&pool, token_id).await.unwrap();
        assert!(!validate_session_token(&pool, token_id).await.unwrap());

        // Nonexistent token id returns false, not an error.
        assert!(!validate_session_token(&pool, Uuid::new_v4()).await.unwrap());
    }

    /// ws F35 guard: reachability check for exactly the 26 db.rs functions
    /// called by this test, each of which had zero test references at review
    /// time. Re-asserting the cheapest invariant of each keeps the reminder
    /// live so that deleting one of the behavioral tests above still leaves a
    /// failing signal here.
    ///
    /// This test is a *reminder*, not a mechanism.
    #[sqlx::test]
    async fn ws_f35_previously_untested_functions_are_reachable(pool: PgPool) {
        let (game_type_id, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[]).await;
        // make_game_with_players shuffles positions; ensure `a` is on turn.
        sqlx::query("UPDATE game_players SET is_turn = true WHERE game_id = $1 AND user_id = $2")
            .bind(game.id)
            .bind(a.id)
            .execute(&pool)
            .await
            .unwrap();
        let mut conn = pool.acquire().await.unwrap();

        assert_eq!(
            count_incoming_friend_requests(&pool, a.id).await.unwrap(),
            0
        );
        assert!(find_active_turn_games(&pool, a.id, 5).await.unwrap().len() == 1);
        // NOT is_empty(): migrations/013_bot_efficacy.sql:41-44 seeds three
        // enabled bots into every test database.
        assert_eq!(find_enabled_bots(&pool).await.unwrap().len(), 3);
        assert!(find_game(&pool, game.id).await.unwrap().is_some());
        assert!(find_game_version(&pool, gv).await.unwrap().is_some());
        assert!(
            find_game_version_render_meta(&pool, gv)
                .await
                .unwrap()
                .is_some()
        );
        assert!(find_game_version_rules(&pool, gv).await.unwrap().is_some());
        assert!(
            find_latest_non_deprecated_game_version(&pool, game_type_id)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            find_open_restart_proposal(&pool, game.id)
                .await
                .unwrap()
                .is_none()
        );
        let mut tx = pool.begin().await.unwrap();
        assert!(
            find_open_restart_proposal_tx(&mut tx, game.id)
                .await
                .unwrap()
                .is_none()
        );
        tx.rollback().await.unwrap();
        assert_eq!(
            find_user_id_by_name(&pool, "alice").await.unwrap(),
            Some(a.id)
        );
        assert!(validate_username(
            &generate_unique_username(&mut conn).await.unwrap()
        ));
        assert!(
            get_pending_request_source(&pool, Uuid::new_v4(), a.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(get_user(&pool, a.id).await.unwrap().is_some());
        assert!(
            get_user_by_email(&pool, "nobody@example.com")
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(get_user_name(&pool, a.id).await.unwrap(), "alice");
        assert!(get_user_pref_colors(&pool, a.id).await.unwrap().is_empty());
        assert!(!has_block_conn(&mut conn, a.id, b.id).await.unwrap());
        assert!(!is_user_admin(&pool, a.id).await.unwrap());
        mark_game_read(&pool, game.id, a.id).await.unwrap();
        assert!(!replacement_bot_available(&pool).await.unwrap());
        assert!(set_user_name(&pool, a.id, "alice_renamed").await.unwrap());
        set_user_pref_colors(&pool, a.id, &[]).await.unwrap();
        assert!(!should_hide_add_friend(&pool, a.id, b.id).await.unwrap());

        let mut tx = pool.begin().await.unwrap();
        insert_game_logs_tx(&mut tx, game.id, vec![]).await.unwrap();
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn invalidate_all_auth_tokens_removes_every_token(pool: PgPool) {
        use crate::auth::session::validate_session_token;

        let user = make_user(&pool, "multisession").await;

        let token1 = Uuid::new_v4();
        let token2 = Uuid::new_v4();
        sqlx::query("INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)")
            .bind(token1)
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)")
            .bind(token2)
            .bind(user.id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(validate_session_token(&pool, token1).await.unwrap());
        assert!(validate_session_token(&pool, token2).await.unwrap());

        let deleted = invalidate_all_auth_tokens(&pool, user.id).await.unwrap();
        assert_eq!(deleted, 2);

        assert!(!validate_session_token(&pool, token1).await.unwrap());
        assert!(!validate_session_token(&pool, token2).await.unwrap());
    }
}
