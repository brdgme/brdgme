#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

#[cfg(feature = "ssr")]
#[derive(Debug, sqlx::FromRow)]
struct FriendRow {
    id: Uuid,
    source_user_id: Uuid,
    has_accepted: Option<bool>,
}

/// D1 lifecycle. Creates a pending request, treating a reverse pending row
/// as mutual intent (auto-accept), a reverse declined row as the decliner
/// changing their mind (flip to accepted), and everything else as a silent
/// no-op. If the target has blocked the source, this is a silent no-op too
/// (D7): the requester must not be able to distinguish any of these.
///
/// Self-requests are a silent no-op (the `friends_check` CHECK constraint
/// stays as the backstop, and `friends.rs`' server fn rejects them with a real
/// user error before we get here) - ws F48.
///
/// The whole read-then-insert runs under a transaction-scoped advisory lock on
/// the ORDERED pair, so two opposite-direction requests serialize and the
/// second one takes the mutual-intent auto-accept branch instead of colliding
/// with the `friends_pair_key` expression index (010_friends.sql:7-9) and
/// returning a raw 23505 - ws F39.
#[cfg(feature = "ssr")]
pub async fn send_friend_request(pool: &PgPool, source: Uuid, target: Uuid) -> Result<()> {
    if source == target {
        // Silent no-op, matching this function's other silent paths (ws F48).
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    // Serialize both directions of this unordered pair for the duration of the
    // transaction, so the read-then-insert below cannot race the opposite
    // direction into a raw `friends_pair_key` 23505 (ws F39). Same key from
    // either direction, and it is the only lock taken, so no deadlock is
    // possible. Released on commit or rollback.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(
           hashtext(LEAST($1::uuid, $2::uuid)::text),
           hashtext(GREATEST($1::uuid, $2::uuid)::text))",
    )
    .bind(source)
    .bind(target)
    .execute(&mut *tx)
    .await?;
    let target_blocked_source: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2)",
    )
    .bind(target)
    .bind(source)
    .fetch_one(&mut *tx)
    .await?;
    if target_blocked_source {
        return Ok(()); // tx dropped -> rollback; nothing written
    }
    let row: Option<FriendRow> = sqlx::query_as(
        "SELECT id, source_user_id, has_accepted FROM friends
         WHERE (source_user_id = $1 AND target_user_id = $2)
            OR (source_user_id = $2 AND target_user_id = $1)",
    )
    .bind(source)
    .bind(target)
    .fetch_optional(&mut *tx)
    .await?;
    match row {
        None => {
            sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
                .bind(source)
                .bind(target)
                .execute(&mut *tx)
                .await?;
        }
        // I already have an outgoing row (pending, declined, or accepted):
        // silent no-op in every case.
        Some(r) if r.source_user_id == source => {}
        // Reverse row: they asked me (pending -> mutual intent) or they asked
        // me and I declined (my own request now = both sides opted in).
        Some(r) => {
            if r.has_accepted != Some(true) {
                sqlx::query("UPDATE friends SET has_accepted = TRUE WHERE id = $1")
                    .bind(r.id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Returns false when no pending request with this id targets `responder`
/// (already responded, wrong user, or unknown id).
#[cfg(feature = "ssr")]
pub async fn respond_to_friend_request(
    pool: &PgPool,
    request_id: Uuid,
    responder: Uuid,
    accept: bool,
) -> Result<bool> {
    let res = sqlx::query(
        "UPDATE friends SET has_accepted = $1
         WHERE id = $2 AND target_user_id = $3 AND has_accepted IS NULL",
    )
    .bind(accept)
    .bind(request_id)
    .bind(responder)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// The requester behind a pending incoming request - used by the
/// decline-and-block path (D7), which needs the source id to block.
#[cfg(feature = "ssr")]
pub async fn get_pending_request_source(
    pool: &PgPool,
    request_id: Uuid,
    responder: Uuid,
) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT source_user_id FROM friends
         WHERE id = $1 AND target_user_id = $2 AND has_accepted IS NULL",
    )
    .bind(request_id)
    .bind(responder)
    .fetch_optional(pool)
    .await?)
}

/// Deletes only ACCEPTED rows: a requester must not be able to delete the
/// declined row that shields the decliner from re-request spam.
#[cfg(feature = "ssr")]
pub async fn unfriend(pool: &PgPool, a: Uuid, b: Uuid) -> Result<()> {
    sqlx::query(
        "DELETE FROM friends WHERE has_accepted = TRUE
         AND ((source_user_id = $1 AND target_user_id = $2)
           OR (source_user_id = $2 AND target_user_id = $1))",
    )
    .bind(a)
    .bind(b)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn are_friends_conn(conn: &mut sqlx::PgConnection, a: Uuid, b: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM friends WHERE has_accepted = TRUE
         AND ((source_user_id = $1 AND target_user_id = $2)
           OR (source_user_id = $2 AND target_user_id = $1)))",
    )
    .bind(a)
    .bind(b)
    .fetch_one(conn)
    .await?)
}

/// True when the "Add friend" affordance should be hidden: already friends
/// (either direction) or viewer already has an outgoing row (pending/declined).
#[cfg(feature = "ssr")]
pub async fn should_hide_add_friend(pool: &PgPool, viewer: Uuid, target: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM friends
           WHERE (source_user_id = $1 AND target_user_id = $2)
              OR (has_accepted = TRUE AND source_user_id = $2 AND target_user_id = $1))",
    )
    .bind(viewer)
    .bind(target)
    .fetch_one(pool)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn should_hide_add_friend_many(
    pool: &PgPool,
    viewer: Uuid,
    targets: &[Uuid],
) -> Result<std::collections::HashSet<Uuid>> {
    if targets.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT target_user_id FROM friends
         WHERE source_user_id = $1 AND target_user_id = ANY($2)
         UNION
         SELECT DISTINCT source_user_id FROM friends
         WHERE target_user_id = $1 AND has_accepted = TRUE AND source_user_id = ANY($2)",
    )
    .bind(viewer)
    .bind(targets)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

#[cfg(feature = "ssr")]
pub async fn list_friends(pool: &PgPool, user_id: Uuid) -> Result<Vec<(Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT u.id, u.name FROM friends f
         JOIN users u ON u.id = CASE WHEN f.source_user_id = $1
                                     THEN f.target_user_id ELSE f.source_user_id END
         WHERE f.has_accepted = TRUE
           AND (f.source_user_id = $1 OR f.target_user_id = $1)
         ORDER BY lower(u.name)",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

/// (request_id, requester_user_id, requester_name), oldest first.
#[cfg(feature = "ssr")]
pub async fn list_incoming_friend_requests(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT f.id, u.id, u.name FROM friends f
         JOIN users u ON u.id = f.source_user_id
         WHERE f.target_user_id = $1 AND f.has_accepted IS NULL
         ORDER BY f.created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn count_incoming_friend_requests(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM friends WHERE target_user_id = $1 AND has_accepted IS NULL",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?)
}

/// Outgoing requests shown as "pending". DELIBERATELY includes declined
/// rows (has_accepted = FALSE): the requester must not be able to
/// distinguish pending from declined (D1 silent shield).
#[cfg(feature = "ssr")]
pub async fn list_outgoing_friend_requests(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT u.id, u.name FROM friends f
         JOIN users u ON u.id = f.target_user_id
         WHERE f.source_user_id = $1
           AND (f.has_accepted IS NULL OR f.has_accepted = FALSE)
         ORDER BY f.created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

/// D7. Idempotent. Severs any friends row for the pair (accepted, pending,
/// or declined, either direction) atomically with the block insert.
#[cfg(feature = "ssr")]
pub async fn block_user(pool: &PgPool, blocker: Uuid, blocked: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO blocks (blocker_user_id, blocked_user_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(blocker)
    .bind(blocked)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM friends
         WHERE (source_user_id = $1 AND target_user_id = $2)
            OR (source_user_id = $2 AND target_user_id = $1)",
    )
    .bind(blocker)
    .bind(blocked)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Deletes the block only. Does not restore any friendship; a fresh friend
/// request afterwards is allowed (D7).
#[cfg(feature = "ssr")]
pub async fn unblock_user(pool: &PgPool, blocker: Uuid, blocked: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2")
        .bind(blocker)
        .bind(blocked)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn has_block(pool: &PgPool, blocker: Uuid, blocked: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2)",
    )
    .bind(blocker)
    .bind(blocked)
    .fetch_one(pool)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn has_block_conn(
    conn: &mut sqlx::PgConnection,
    blocker: Uuid,
    blocked: Uuid,
) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2)",
    )
    .bind(blocker)
    .bind(blocked)
    .fetch_one(conn)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn list_blocked(pool: &PgPool, blocker: Uuid) -> Result<Vec<(Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT u.id, u.name FROM blocks b
         JOIN users u ON u.id = b.blocked_user_id
         WHERE b.blocker_user_id = $1
         ORDER BY b.created_at DESC",
    )
    .bind(blocker)
    .fetch_all(pool)
    .await?)
}

/// D6: friends tier (most recently played with first - resolved decision
/// 2026-07-18 - then alphabetical), then distinct human co-players from the
/// caller's last 20 games. Excludes self and any block in either direction.
#[cfg(feature = "ssr")]
pub async fn opponent_suggestions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, String, bool)>> {
    let friends: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT u.id, u.name FROM friends f
         JOIN users u ON u.id = CASE WHEN f.source_user_id = $1
                                     THEN f.target_user_id ELSE f.source_user_id END
         WHERE f.has_accepted = TRUE
           AND (f.source_user_id = $1 OR f.target_user_id = $1)
         ORDER BY (SELECT max(g.updated_at) FROM games g
                   JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
                   JOIN game_players them ON them.game_id = g.id AND them.user_id = u.id)
                  DESC NULLS LAST,
                  lower(u.name)",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let recent: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT u.id, u.name FROM
           (SELECT g.id AS game_id, g.updated_at FROM games g
            JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
            ORDER BY g.updated_at DESC LIMIT 20) recent
         JOIN game_players op ON op.game_id = recent.game_id AND op.user_id <> $1
         JOIN users u ON u.id = op.user_id
         WHERE NOT EXISTS (SELECT 1 FROM blocks b
                           WHERE (b.blocker_user_id = $1 AND b.blocked_user_id = u.id)
                              OR (b.blocker_user_id = u.id AND b.blocked_user_id = $1))
         GROUP BY u.id, u.name
         ORDER BY max(recent.updated_at) DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<(Uuid, String, bool)> = friends
        .into_iter()
        .map(|(id, name)| (id, name, true))
        .collect();
    for (id, name) in recent {
        if !out.iter().any(|(fid, _, _)| *fid == id) {
            out.push((id, name, false));
        }
    }
    Ok(out)
}

/// D5: in-progress games containing >= 1 accepted friend where the caller
/// is NOT a player (spectating links). Human player names only - bots live
/// in game_bots and are omitted from this lightweight feed.
#[cfg(feature = "ssr")]
pub async fn friends_active_games(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<(Uuid, String, Vec<String>)>> {
    Ok(sqlx::query_as(
        "SELECT g.id, gt.name, array_agg(u.name ORDER BY gp.position)
         FROM games g
         JOIN game_versions gv ON gv.id = g.game_version_id
         JOIN game_types gt ON gt.id = gv.game_type_id
         JOIN game_players gp ON gp.game_id = g.id
         JOIN users u ON u.id = gp.user_id
         WHERE g.is_finished = FALSE
           AND NOT EXISTS (SELECT 1 FROM game_players me
                           WHERE me.game_id = g.id AND me.user_id = $1)
           AND EXISTS (
               SELECT 1 FROM game_players fgp
               JOIN friends f ON f.has_accepted = TRUE
                    AND ((f.source_user_id = $1 AND f.target_user_id = fgp.user_id)
                      OR (f.target_user_id = $1 AND f.source_user_id = fgp.user_id))
               WHERE fgp.game_id = g.id)
         GROUP BY g.id, gt.name, g.updated_at
         ORDER BY g.updated_at DESC
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

/// D5: last `limit` finished games involving >= 1 friend (the caller's own
/// finished games qualify too). Names ordered by place (NULLS LAST), places
/// COALESCEd to 0 for "not placed".
#[cfg(feature = "ssr")]
pub async fn friends_recent_results(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<
    Vec<(
        Uuid,
        String,
        Option<time::PrimitiveDateTime>,
        Vec<String>,
        Vec<i32>,
    )>,
> {
    Ok(sqlx::query_as(
        "SELECT g.id, gt.name, g.finished_at,
                array_agg(u.name ORDER BY gp.place ASC NULLS LAST, gp.position),
                array_agg(COALESCE(gp.place, 0) ORDER BY gp.place ASC NULLS LAST, gp.position)
         FROM games g
         JOIN game_versions gv ON gv.id = g.game_version_id
         JOIN game_types gt ON gt.id = gv.game_type_id
         JOIN game_players gp ON gp.game_id = g.id
         JOIN users u ON u.id = gp.user_id
         WHERE g.is_finished = TRUE
           AND EXISTS (
               SELECT 1 FROM game_players fgp
               JOIN friends f ON f.has_accepted = TRUE
                    AND ((f.source_user_id = $1 AND f.target_user_id = fgp.user_id)
                      OR (f.target_user_id = $1 AND f.source_user_id = fgp.user_id))
               WHERE fgp.game_id = g.id)
         GROUP BY g.id, gt.name, g.finished_at
         ORDER BY g.finished_at DESC NULLS LAST
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use crate::db::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    async fn friend_row_state(pool: &PgPool, a: Uuid, b: Uuid) -> Option<(Uuid, Option<bool>)> {
        sqlx::query_as::<_, (Uuid, Option<bool>)>(
            "SELECT source_user_id, has_accepted FROM friends
             WHERE (source_user_id = $1 AND target_user_id = $2)
                OR (source_user_id = $2 AND target_user_id = $1)",
        )
        .bind(a)
        .bind(b)
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn friend_request_creates_pending_row(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, None))
        );
    }

    #[sqlx::test]
    async fn reverse_pending_request_auto_accepts(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
        let mut conn = pool.acquire().await.unwrap();
        assert!(are_friends_conn(&mut conn, a.id, b.id).await.unwrap());
    }

    #[sqlx::test]
    async fn accept_and_decline_update_pending_row(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        // wrong responder: the requester cannot accept their own request
        assert!(
            !respond_to_friend_request(&pool, req_id, a.id, true)
                .await
                .unwrap()
        );
        assert!(
            respond_to_friend_request(&pool, req_id, b.id, true)
                .await
                .unwrap()
        );
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
        // already-responded request is no longer pending
        assert!(
            !respond_to_friend_request(&pool, req_id, b.id, false)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn rerequest_after_decline_is_silent_noop(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        assert!(
            respond_to_friend_request(&pool, req_id, b.id, false)
                .await
                .unwrap()
        );
        // silent shield: re-request succeeds but the row stays declined
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(false)))
        );
        // and the requester still sees it as an outgoing "pending" request
        let outgoing = list_outgoing_friend_requests(&pool, a.id).await.unwrap();
        assert_eq!(outgoing, vec![(b.id, "bob".to_string())]);
    }

    #[sqlx::test]
    async fn decliner_own_request_flips_to_accepted(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        respond_to_friend_request(&pool, req_id, b.id, false)
            .await
            .unwrap();
        // b changed their mind: both sides have now expressed intent
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
    }

    #[sqlx::test]
    async fn pair_unique_index_rejects_reverse_duplicate(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
            .bind(a.id)
            .bind(b.id)
            .execute(&pool)
            .await
            .unwrap();
        let err =
            sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
                .bind(b.id)
                .bind(a.id)
                .execute(&pool)
                .await;
        assert!(
            err.is_err(),
            "pair-unique index must reject B->A when A->B exists"
        );
    }

    /// ws F48: a self-request is a silent application-level no-op. The
    /// `friends_check` CHECK constraint (migrations/001:114) remains the
    /// backstop and is asserted directly below, so the guard cannot be
    /// mistaken for the DB's protection going away.
    #[sqlx::test]
    async fn self_request_is_silent_no_op(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        send_friend_request(&pool, a.id, a.id)
            .await
            .expect("self-request must be a silent Ok, not an error");
        assert_eq!(
            count_rows(&pool, "friends").await,
            0,
            "self-request must not write a friends row"
        );
        // The DB CHECK still rejects a self row inserted directly.
        let direct =
            sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
                .bind(a.id)
                .bind(a.id)
                .execute(&pool)
                .await;
        assert!(
            direct.is_err(),
            "friends_check must still reject a self row (ws F48 backstop)"
        );
    }

    #[sqlx::test]
    async fn unfriend_deletes_accepted_but_not_declined(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        respond_to_friend_request(&pool, req_id, b.id, false)
            .await
            .unwrap();
        // declined row survives unfriend (anti-harassment shield stays)
        unfriend(&pool, a.id, b.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_some());
        // flip to accepted, then unfriend from the other side deletes it
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        unfriend(&pool, b.id, a.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_none());
        // clean slate: fresh request allowed
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, None))
        );
    }

    #[sqlx::test]
    async fn friend_lists_and_name_lookup(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let c = make_user(&pool, "carol").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        send_friend_request(&pool, c.id, a.id).await.unwrap(); // incoming pending for a
        assert_eq!(
            list_friends(&pool, a.id).await.unwrap(),
            vec![(b.id, "bob".to_string())]
        );
        let incoming = list_incoming_friend_requests(&pool, a.id).await.unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(
            (incoming[0].1, incoming[0].2.clone()),
            (c.id, "carol".to_string())
        );
        assert_eq!(
            list_outgoing_friend_requests(&pool, c.id).await.unwrap(),
            vec![(a.id, "alice".to_string())]
        );
        assert_eq!(
            get_user_by_name(&pool, "ALICE").await.unwrap(),
            Some((a.id, "alice".to_string()))
        );
        assert_eq!(get_user_by_name(&pool, "nobody").await.unwrap(), None);
    }

    #[sqlx::test]
    async fn block_severs_friendship_and_pending(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        block_user(&pool, b.id, a.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_none());
        assert!(has_block(&pool, b.id, a.id).await.unwrap());
        assert!(!has_block(&pool, a.id, b.id).await.unwrap()); // directed
        assert_eq!(
            list_blocked(&pool, b.id).await.unwrap(),
            vec![(a.id, "alice".to_string())]
        );
        // idempotent
        block_user(&pool, b.id, a.id).await.unwrap();
    }

    #[sqlx::test]
    async fn blocked_requester_is_silently_ignored(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        block_user(&pool, b.id, a.id).await.unwrap();
        // a's request "succeeds" but writes nothing (silent shield)
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_none());
        assert!(
            list_incoming_friend_requests(&pool, b.id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[sqlx::test]
    async fn unblock_allows_fresh_request_but_restores_nothing(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        block_user(&pool, b.id, a.id).await.unwrap();
        unblock_user(&pool, b.id, a.id).await.unwrap();
        assert!(!has_block(&pool, b.id, a.id).await.unwrap());
        let mut conn = pool.acquire().await.unwrap();
        assert!(!are_friends_conn(&mut conn, a.id, b.id).await.unwrap());
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, None))
        );
    }

    #[sqlx::test]
    async fn suggestions_friends_first_then_recent_coplayers(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let friend_old = make_user(&pool, "zed").await; // friend, played long ago
        let friend_new = make_user(&pool, "amy").await; // friend, played recently
        let stranger = make_user(&pool, "stranger").await; // co-player, not friend
        for f in [friend_old.id, friend_new.id] {
            send_friend_request(&pool, me.id, f).await.unwrap();
            send_friend_request(&pool, f, me.id).await.unwrap();
        }
        let (_, version) = make_game_type_and_version(&pool).await;
        let g1 = make_game_with_players(&pool, version, me.id, &[friend_old.id], 0, &[0]).await;
        let g2 = make_game_with_players(&pool, version, me.id, &[friend_new.id], 0, &[0]).await;
        let g3 = make_game_with_players(&pool, version, me.id, &[stranger.id], 0, &[0]).await;
        // force distinct recency: g1 oldest, g3 newest
        for (i, gid) in [g1.id, g2.id, g3.id].iter().enumerate() {
            sqlx::query(
                "UPDATE games SET updated_at = NOW() - make_interval(days => $1) WHERE id = $2",
            )
            .bind(3 - i as i32)
            .bind(gid)
            .execute(&pool)
            .await
            .unwrap();
        }
        let s = opponent_suggestions(&pool, me.id).await.unwrap();
        assert_eq!(
            s,
            vec![
                (friend_new.id, "amy".to_string(), true), // friends by recency
                (friend_old.id, "zed".to_string(), true),
                (stranger.id, "stranger".to_string(), false), // then co-players
            ]
        );
    }

    #[sqlx::test]
    async fn suggestions_exclude_blocked_and_self(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let blocked_by_me = make_user(&pool, "villain").await;
        let blocked_me = make_user(&pool, "hermit").await;
        let (_, version) = make_game_type_and_version(&pool).await;
        make_game_with_players(
            &pool,
            version,
            me.id,
            &[blocked_by_me.id, blocked_me.id],
            0,
            &[0],
        )
        .await;
        block_user(&pool, me.id, blocked_by_me.id).await.unwrap();
        block_user(&pool, blocked_me.id, me.id).await.unwrap();
        assert!(opponent_suggestions(&pool, me.id).await.unwrap().is_empty());
    }

    #[sqlx::test]
    async fn friends_active_games_excludes_own_and_nonfriend_games(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let friend = make_user(&pool, "friend").await;
        let other = make_user(&pool, "other").await;
        let bystander = make_user(&pool, "bystander").await;
        send_friend_request(&pool, me.id, friend.id).await.unwrap();
        send_friend_request(&pool, friend.id, me.id).await.unwrap();
        let (_, version) = make_game_type_and_version(&pool).await;
        // friend's game without me: should appear
        let g = make_game_with_players(&pool, version, friend.id, &[other.id], 0, &[0]).await;
        // my own game with the friend: excluded (I am in it)
        make_game_with_players(&pool, version, friend.id, &[me.id], 0, &[0]).await;
        // game with no friends in it: excluded
        make_game_with_players(&pool, version, other.id, &[bystander.id], 0, &[0]).await;
        let rows = friends_active_games(&pool, me.id, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, g.id);
        let mut names = rows[0].2.clone();
        names.sort();
        assert_eq!(names, vec!["friend".to_string(), "other".to_string()]);
    }

    #[sqlx::test]
    async fn friends_recent_results_return_places(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let friend = make_user(&pool, "friend").await;
        let other = make_user(&pool, "other").await;
        send_friend_request(&pool, me.id, friend.id).await.unwrap();
        send_friend_request(&pool, friend.id, me.id).await.unwrap();
        let (_, version) = make_game_type_and_version(&pool).await;
        let g = make_game_with_players(&pool, version, friend.id, &[other.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET is_finished = TRUE, finished_at = timezone('utc', now()) WHERE id = $1")
            .bind(g.id).execute(&pool).await.unwrap();
        sqlx::query("UPDATE game_players SET place = 1 WHERE game_id = $1 AND user_id = $2")
            .bind(g.id)
            .bind(friend.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE game_players SET place = 2 WHERE game_id = $1 AND user_id = $2")
            .bind(g.id)
            .bind(other.id)
            .execute(&pool)
            .await
            .unwrap();
        let rows = friends_recent_results(&pool, me.id, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        let (game_id, _type_name, finished_at, names, places) = rows[0].clone();
        assert_eq!(game_id, g.id);
        assert!(finished_at.is_some());
        assert_eq!(names, vec!["friend".to_string(), "other".to_string()]);
        assert_eq!(places, vec![1, 2]);
    }

    /// ws F39: two opposite-direction requests must end in the mutual-intent
    /// accepted state regardless of interleaving. A single-connection test
    /// cannot force the true concurrent interleaving, so this asserts the
    /// serialized outcome plus that the advisory lock is re-entrant for the
    /// same pair within one session (taking it twice must not deadlock or
    /// change the result), which is what the pooled server does across
    /// sequential requests.
    #[sqlx::test]
    async fn opposite_direction_requests_auto_accept_under_pair_lock(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;

        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true))),
            "B->A after A->B must auto-accept the single pair row"
        );
        assert_eq!(
            count_rows(&pool, "friends").await,
            1,
            "the pair-unique index must still leave exactly one row"
        );

        // Re-sending in either direction stays a no-op and never errors.
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(count_rows(&pool, "friends").await, 1);
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
    }

    /// ws F35: `should_hide_add_friend` had no test. The button hides when the
    /// viewer already has an outgoing row of ANY state (pending, declined,
    /// accepted - the D1 shield) and when an ACCEPTED reverse row exists, but
    /// NOT for a merely pending incoming request.
    #[sqlx::test]
    async fn should_hide_add_friend_covers_every_row_state(pool: PgPool) {
        let viewer = make_user(&pool, "viewer").await;
        let stranger = make_user(&pool, "stranger").await;
        let pending_out = make_user(&pool, "pendingout").await;
        let pending_in = make_user(&pool, "pendingin").await;
        let declined_out = make_user(&pool, "declinedout").await;
        let accepted = make_user(&pool, "accepted").await;

        send_friend_request(&pool, viewer.id, pending_out.id)
            .await
            .unwrap();
        send_friend_request(&pool, pending_in.id, viewer.id)
            .await
            .unwrap();
        send_friend_request(&pool, viewer.id, declined_out.id)
            .await
            .unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, declined_out.id)
            .await
            .unwrap()[0];
        respond_to_friend_request(&pool, req_id, declined_out.id, false)
            .await
            .unwrap();
        accept_friends(&pool, viewer.id, accepted.id).await;

        assert!(
            !should_hide_add_friend(&pool, viewer.id, stranger.id)
                .await
                .unwrap()
        );
        assert!(
            should_hide_add_friend(&pool, viewer.id, pending_out.id)
                .await
                .unwrap()
        );
        assert!(
            should_hide_add_friend(&pool, viewer.id, declined_out.id)
                .await
                .unwrap()
        );
        assert!(
            should_hide_add_friend(&pool, viewer.id, accepted.id)
                .await
                .unwrap()
        );
        assert!(
            !should_hide_add_friend(&pool, viewer.id, pending_in.id)
                .await
                .unwrap(),
            "a pending INCOMING request must not hide the button - accepting it \
             by sending back is the documented mutual-intent path"
        );
    }

    /// wd F21: `should_hide_add_friend_many` (the batched predicate behind
    /// `get_game_details`' add-friend affordance) must agree with the singular
    /// `should_hide_add_friend` for every friend-row state, in one batched
    /// call. Mirrors the singular test above by batch-equivalence: stranger
    /// (no row), pending outgoing, declined outgoing and accepted all hide;
    /// a pending INCOMING request does not. Also covers the empty-targets
    /// early return.
    #[sqlx::test]
    async fn should_hide_add_friend_many_matches_singular_per_row_state(pool: PgPool) {
        let viewer = make_user(&pool, "viewer").await;
        let stranger = make_user(&pool, "stranger").await;
        let pending_out = make_user(&pool, "pendingout").await;
        let pending_in = make_user(&pool, "pendingin").await;
        let declined_out = make_user(&pool, "declinedout").await;
        let accepted = make_user(&pool, "accepted").await;

        send_friend_request(&pool, viewer.id, pending_out.id)
            .await
            .unwrap();
        send_friend_request(&pool, pending_in.id, viewer.id)
            .await
            .unwrap();
        send_friend_request(&pool, viewer.id, declined_out.id)
            .await
            .unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, declined_out.id)
            .await
            .unwrap()[0];
        respond_to_friend_request(&pool, req_id, declined_out.id, false)
            .await
            .unwrap();
        accept_friends(&pool, viewer.id, accepted.id).await;

        let targets = [
            stranger.id,
            pending_out.id,
            pending_in.id,
            declined_out.id,
            accepted.id,
        ];
        let hidden = should_hide_add_friend_many(&pool, viewer.id, &targets)
            .await
            .unwrap();

        for t in targets {
            assert_eq!(
                hidden.contains(&t),
                should_hide_add_friend(&pool, viewer.id, t).await.unwrap(),
                "batch and singular predicates disagree for target {t}"
            );
        }
        assert!(!hidden.contains(&stranger.id));
        assert!(hidden.contains(&pending_out.id));
        assert!(hidden.contains(&declined_out.id));
        assert!(hidden.contains(&accepted.id));
        assert!(
            !hidden.contains(&pending_in.id),
            "a pending INCOMING request must not hide the button"
        );

        let empty = should_hide_add_friend_many(&pool, viewer.id, &[])
            .await
            .unwrap();
        assert!(empty.is_empty(), "empty targets must short-circuit to empty");
    }

    /// ws F35: three untested friend/block helpers, batched.
    #[sqlx::test]
    async fn friend_request_helpers(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let x = make_user(&pool, "requesterx").await;
        let y = make_user(&pool, "requestery").await;

        assert_eq!(
            count_incoming_friend_requests(&pool, me.id).await.unwrap(),
            0
        );
        send_friend_request(&pool, x.id, me.id).await.unwrap();
        send_friend_request(&pool, y.id, me.id).await.unwrap();
        assert_eq!(
            count_incoming_friend_requests(&pool, me.id).await.unwrap(),
            2
        );

        let incoming = list_incoming_friend_requests(&pool, me.id).await.unwrap();
        let (req_id, _, _) = incoming[0];
        let source = get_pending_request_source(&pool, req_id, me.id)
            .await
            .unwrap();
        assert!(source == Some(x.id) || source == Some(y.id));
        assert_eq!(
            get_pending_request_source(&pool, req_id, x.id)
                .await
                .unwrap(),
            None,
            "only the TARGET of the request may resolve its source"
        );
        assert_eq!(
            get_pending_request_source(&pool, Uuid::new_v4(), me.id)
                .await
                .unwrap(),
            None
        );

        // Once responded, it is no longer pending and drops out of both.
        respond_to_friend_request(&pool, req_id, me.id, true)
            .await
            .unwrap();
        assert_eq!(
            count_incoming_friend_requests(&pool, me.id).await.unwrap(),
            1
        );
        assert_eq!(
            get_pending_request_source(&pool, req_id, me.id)
                .await
                .unwrap(),
            None
        );

        // has_block_conn must agree with has_block, and is directional.
        block_user(&pool, me.id, x.id).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        assert!(has_block_conn(&mut conn, me.id, x.id).await.unwrap());
        assert!(!has_block_conn(&mut conn, x.id, me.id).await.unwrap());
        assert_eq!(
            has_block_conn(&mut conn, me.id, x.id).await.unwrap(),
            has_block(&pool, me.id, x.id).await.unwrap()
        );
    }
}
