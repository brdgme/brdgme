#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

/// The open restart proposal for a game, if any: a proposal carrying
/// `restarted_game_id = old_game_id` still in the `open` state. An open restart
/// proposal is an in-flight restart that blocks a second one - the old->new game
/// link is only written when the proposal STARTS, so checking
/// `games.restarted_game_id` alone would miss this case. Earliest first for a
/// deterministic winner. Plain query to avoid `.sqlx` churn.
#[cfg(feature = "ssr")]
pub async fn find_open_restart_proposal_tx(
    tx: &mut sqlx::PgConnection,
    old_game_id: Uuid,
) -> Result<Option<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM game_proposals WHERE restarted_game_id = $1 AND status = 'open' ORDER BY created_at LIMIT 1",
    )
    .bind(old_game_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn find_open_restart_proposal(pool: &PgPool, old_game_id: Uuid) -> Result<Option<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM game_proposals WHERE restarted_game_id = $1 AND status = 'open' ORDER BY created_at LIMIT 1",
    )
    .bind(old_game_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn is_proposal_visible_to_user(
    pool: &PgPool,
    proposal_id: Uuid,
    viewer_id: Uuid,
) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_proposal_players WHERE proposal_id = $1 AND user_id = $2)",
    )
    .bind(proposal_id)
    .bind(viewer_id)
    .fetch_one(pool)
    .await?)
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    /// ws F35: neither restart-proposal lookup had a test. Only `open`
    /// proposals count, the earliest wins, and the `_tx` variant must agree
    /// with the pool variant.
    #[sqlx::test]
    async fn find_open_restart_proposal_finds_earliest_open_only(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let owner = make_user(&pool, "owner").await;
        let other = make_user(&pool, "other").await;
        let old_game = make_game_with_players(&pool, gv, owner.id, &[other.id], 0, &[0]).await;

        // Helper as a plain async fn call, not a closure: no borrow-checker
        // gymnastics, and `game_proposals` only needs these five columns
        // (migrations/015_game_proposals.sql:5-15 - `status` is CHECKed against
        // 'open'/'started'/'cancelled', `created_at` is a bare `timestamp`).
        async fn insert_proposal(
            pool: &PgPool,
            gv: Uuid,
            owner_id: Uuid,
            old_id: Uuid,
            status: &str,
            created: &str,
        ) -> Uuid {
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO game_proposals
                   (game_version_id, owner_user_id, restarted_game_id, status, created_at)
                 VALUES ($1, $2, $3, $4, $5::timestamp) RETURNING id",
            )
            .bind(gv)
            .bind(owner_id)
            .bind(old_id)
            .bind(status)
            .bind(created)
            .fetch_one(pool)
            .await
            .unwrap()
        }

        // No proposal yet.
        assert!(
            find_open_restart_proposal(&pool, old_game.id)
                .await
                .unwrap()
                .is_none()
        );

        let cancelled = insert_proposal(
            &pool,
            gv,
            owner.id,
            old_game.id,
            "cancelled",
            "2026-01-01 00:00:00",
        )
        .await;
        assert!(
            find_open_restart_proposal(&pool, old_game.id)
                .await
                .unwrap()
                .is_none(),
            "a cancelled proposal must not count"
        );

        let later_open = insert_proposal(
            &pool,
            gv,
            owner.id,
            old_game.id,
            "open",
            "2026-01-03 00:00:00",
        )
        .await;
        let earlier_open = insert_proposal(
            &pool,
            gv,
            owner.id,
            old_game.id,
            "open",
            "2026-01-02 00:00:00",
        )
        .await;
        assert_eq!(
            find_open_restart_proposal(&pool, old_game.id)
                .await
                .unwrap(),
            Some(earlier_open),
            "earliest open proposal wins for a deterministic winner"
        );
        assert_ne!(earlier_open, later_open);
        assert_ne!(earlier_open, cancelled);

        // The _tx variant must agree.
        let mut tx = pool.begin().await.unwrap();
        assert_eq!(
            find_open_restart_proposal_tx(&mut tx, old_game.id)
                .await
                .unwrap(),
            Some(earlier_open)
        );
        tx.rollback().await.unwrap();

        // An unrelated game has none.
        let unrelated = make_game_with_players(&pool, gv, owner.id, &[other.id], 0, &[0]).await;
        assert!(
            find_open_restart_proposal(&pool, unrelated.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    async fn is_proposal_visible_to_user_participant_is_visible(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let owner = make_user(&pool, "owner").await;
        let player = make_user(&pool, "player").await;
        let proposal = make_proposal(&pool, gv, owner.id).await;
        add_proposal_player(&pool, proposal, 0, Some(owner.id), None, "accepted").await;
        add_proposal_player(&pool, proposal, 1, Some(player.id), None, "pending").await;

        assert!(
            is_proposal_visible_to_user(&pool, proposal, owner.id)
                .await
                .unwrap()
        );
        assert!(
            is_proposal_visible_to_user(&pool, proposal, player.id)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn is_proposal_visible_to_user_non_participant_is_not_visible(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let owner = make_user(&pool, "owner").await;
        let stranger = make_user(&pool, "stranger").await;
        let proposal = make_proposal(&pool, gv, owner.id).await;
        add_proposal_player(&pool, proposal, 0, Some(owner.id), None, "accepted").await;

        assert!(
            !is_proposal_visible_to_user(&pool, proposal, stranger.id)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn is_proposal_visible_to_user_nonexistent_proposal_is_not_visible(pool: PgPool) {
        let owner = make_user(&pool, "owner").await;
        assert!(
            !is_proposal_visible_to_user(&pool, Uuid::new_v4(), owner.id)
                .await
                .unwrap()
        );
    }
}
