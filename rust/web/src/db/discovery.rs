use super::*;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

/// Picks the game to show on the logged-out index: an active (non-finished)
/// game whose human players are ALL game_visibility = 'public', ranked by the
/// count of recently-active human players, then most-recent update. Bots
/// (user_id IS NULL) are dropped by the JOIN and never affect visibility or
/// the active count. Shares the all-public predicate with
/// `is_game_publicly_visible` so selection and render cannot drift (Unit B 2c).
#[cfg(feature = "ssr")]
pub async fn find_public_index_game_id(pool: &PgPool) -> Result<Option<Uuid>> {
    let window =
        time::Duration::try_from(RECENTLY_ACTIVE_WINDOW).unwrap_or(time::Duration::minutes(10));
    let cutoff = time::OffsetDateTime::now_utc() - window;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT g.id
         FROM games g
         WHERE g.is_finished = false
           AND NOT EXISTS (
             SELECT 1 FROM game_players gp
             JOIN users u ON u.id = gp.user_id
             WHERE gp.game_id = g.id
               AND u.game_visibility <> 'public')
         ORDER BY (
           SELECT COUNT(*)
           FROM game_players gp2
           JOIN users u2 ON u2.id = gp2.user_id
           WHERE gp2.game_id = g.id
             AND u2.last_active_at > $1
         ) DESC, g.updated_at DESC, g.id
         LIMIT 1",
    )
    .bind(cutoff)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// The friend's most recently updated game that `viewer_id` may see, scanning
/// only the `scan_limit` most recent (so an old visible game does not surface
/// when everything recent is hidden - the pre-inlining behaviour, preserved).
///
/// The visibility rule is `crate::db::is_game_visible_to_user`'s predicate
/// (defined in `db/visibility.rs`), inlined so this is one query instead of one
/// query per candidate (ws F40). The derived table applies the scan window
/// first and the predicate is projected as `visible`, which keeps the window
/// semantics identical. Keep in step with `crate::db::is_game_visible_to_user`;
/// the `friend_recent_visible_game_matches_is_game_visible_to_user` test
/// asserts the two agree.
#[cfg(feature = "ssr")]
pub async fn friend_recent_visible_game(
    pool: &PgPool,
    friend_user_id: Uuid,
    viewer_id: Uuid,
    scan_limit: i64,
) -> Result<Option<(Uuid, String, time::PrimitiveDateTime)>> {
    Ok(sqlx::query_as(
        "SELECT c.id, c.name, c.updated_at
         FROM (
           SELECT g.id, gt.name, g.updated_at,
                  (EXISTS(SELECT 1 FROM game_players v
                          WHERE v.game_id = g.id AND v.user_id = $3)
                   OR NOT EXISTS(
                     SELECT 1 FROM game_players gp2
                     JOIN users u ON u.id = gp2.user_id
                     WHERE gp2.game_id = g.id
                       AND NOT (
                         u.game_visibility = 'public'
                         OR (u.game_visibility = 'friends' AND EXISTS(
                           SELECT 1 FROM friends f WHERE f.has_accepted = TRUE
                             AND ((f.source_user_id = $3 AND f.target_user_id = u.id)
                               OR (f.target_user_id = $3 AND f.source_user_id = u.id))
                         ))
                       ))) AS visible
           FROM game_players gp
           JOIN games g ON g.id = gp.game_id
           JOIN game_versions gv ON gv.id = g.game_version_id
           JOIN game_types gt ON gt.id = gv.game_type_id
           WHERE gp.user_id = $1
           ORDER BY g.updated_at DESC, g.id
           LIMIT $2
         ) c
         WHERE c.visible
         ORDER BY c.updated_at DESC, c.id
         LIMIT 1",
    )
    .bind(friend_user_id)
    .bind(scan_limit)
    .bind(viewer_id)
    .fetch_optional(pool)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn recent_games_for_index(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<(Uuid, String, bool, bool, time::PrimitiveDateTime)>> {
    Ok(sqlx::query_as(
        "SELECT g.id, gt.name, g.is_finished, me.is_turn, g.updated_at
         FROM games g
         JOIN game_versions gv ON gv.id = g.game_version_id
         JOIN game_types gt ON gt.id = gv.game_type_id
         JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
         ORDER BY g.updated_at DESC, g.id
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
    use sqlx::postgres::PgPool;

    #[sqlx::test]
    async fn find_public_index_game_id_picks_most_active_players(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let cara = make_user(&pool, "cara").await;
        let dan = make_user(&pool, "dan").await;

        let game_a = make_game_with_players(&pool, gv, alice.id, &[bob.id], 0, &[0]).await;
        set_recently_active(&pool, alice.id).await;
        set_recently_active(&pool, bob.id).await;

        let _game_b = make_game_with_players(&pool, gv, cara.id, &[dan.id], 0, &[0]).await;
        set_recently_active(&pool, cara.id).await;
        set_stale(&pool, dan.id).await;

        assert_eq!(
            find_public_index_game_id(&pool).await.unwrap(),
            Some(game_a.id)
        );
    }

    #[sqlx::test]
    async fn find_public_index_game_id_excludes_non_public_players(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let cara = make_user(&pool, "cara").await;

        let _game_a = make_game_with_players(&pool, gv, alice.id, &[bob.id], 0, &[0]).await;
        set_recently_active(&pool, alice.id).await;
        set_recently_active(&pool, bob.id).await;
        set_game_visibility(&pool, bob.id, "friends").await.unwrap();

        let game_b = make_game_with_players(&pool, gv, cara.id, &[], 0, &[0]).await;
        set_recently_active(&pool, cara.id).await;

        assert_eq!(
            find_public_index_game_id(&pool).await.unwrap(),
            Some(game_b.id)
        );
    }

    #[sqlx::test]
    async fn find_public_index_game_id_excludes_private_player(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;

        let _game = make_game_with_players(&pool, gv, alice.id, &[bob.id], 0, &[0]).await;
        set_recently_active(&pool, alice.id).await;
        set_recently_active(&pool, bob.id).await;
        set_game_visibility(&pool, bob.id, "private").await.unwrap();

        assert_eq!(find_public_index_game_id(&pool).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn find_public_index_game_id_excludes_finished_games(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;

        let game = make_game_with_players(&pool, gv, alice.id, &[bob.id], 0, &[0]).await;
        set_recently_active(&pool, alice.id).await;
        set_recently_active(&pool, bob.id).await;
        finish_game(&pool, game.id).await;

        assert_eq!(find_public_index_game_id(&pool).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn find_public_index_game_id_none_when_no_games(pool: PgPool) {
        assert_eq!(find_public_index_game_id(&pool).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn find_public_index_game_id_tiebreaks_by_updated_at(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;

        let game_a = make_game_with_players(&pool, gv, alice.id, &[], 0, &[0]).await;
        let game_b = make_game_with_players(&pool, gv, bob.id, &[], 0, &[0]).await;

        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE games SET updated_at = NOW() - INTERVAL '1 hour' WHERE id = $1")
            .bind(game_a.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE games SET updated_at = NOW() - INTERVAL '2 hours' WHERE id = $1")
            .bind(game_b.id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            find_public_index_game_id(&pool).await.unwrap(),
            Some(game_a.id)
        );
    }

    #[sqlx::test]
    async fn friend_recent_visible_game_returns_most_recent(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let viewer = make_user(&pool, "viewer").await;
        let friend = make_user(&pool, "friend").await;
        let other = make_user(&pool, "other").await;
        accept_friends(&pool, viewer.id, friend.id).await;

        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();

        let older = make_game_with_players(&pool, gv, friend.id, &[other.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET updated_at = '2026-01-01 00:00:00' WHERE id = $1")
            .bind(older.id)
            .execute(&pool)
            .await
            .unwrap();

        let newer = make_game_with_players(&pool, gv, friend.id, &[other.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET updated_at = '2026-01-02 00:00:00' WHERE id = $1")
            .bind(newer.id)
            .execute(&pool)
            .await
            .unwrap();

        let result = friend_recent_visible_game(&pool, friend.id, viewer.id, 10)
            .await
            .unwrap();
        assert!(result.is_some());
        let (game_id, _, _) = result.unwrap();
        assert_eq!(game_id, newer.id);
    }

    #[sqlx::test]
    async fn friend_recent_visible_game_skips_private_player(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let viewer = make_user(&pool, "viewer").await;
        let friend = make_user(&pool, "friend").await;
        let private_player = make_user(&pool, "privatep").await;
        let other = make_user(&pool, "other").await;
        accept_friends(&pool, viewer.id, friend.id).await;
        set_game_visibility(&pool, private_player.id, "private")
            .await
            .unwrap();

        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();

        let older = make_game_with_players(&pool, gv, friend.id, &[other.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET updated_at = '2026-01-01 00:00:00' WHERE id = $1")
            .bind(older.id)
            .execute(&pool)
            .await
            .unwrap();

        let newer =
            make_game_with_players(&pool, gv, friend.id, &[private_player.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET updated_at = '2026-01-02 00:00:00' WHERE id = $1")
            .bind(newer.id)
            .execute(&pool)
            .await
            .unwrap();

        let result = friend_recent_visible_game(&pool, friend.id, viewer.id, 10)
            .await
            .unwrap();
        assert!(result.is_some());
        let (game_id, _, _) = result.unwrap();
        assert_eq!(
            game_id, older.id,
            "should skip the newer game with a private player"
        );
    }

    #[sqlx::test]
    async fn friend_recent_visible_game_returns_none_when_no_games(pool: PgPool) {
        let viewer = make_user(&pool, "viewer").await;
        let friend = make_user(&pool, "friend").await;
        accept_friends(&pool, viewer.id, friend.id).await;

        let result = friend_recent_visible_game(&pool, friend.id, viewer.id, 10)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test]
    async fn recent_games_for_index_returns_last_10_most_recent_first(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let user = make_user(&pool, "alice").await;
        let opp = make_user(&pool, "bob").await;

        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();

        let mut game_ids = Vec::new();
        for i in 0..12 {
            let g = make_game_with_players(&pool, gv, user.id, &[opp.id], 0, &[0]).await;
            let ts = format!("2026-01-{:02} 00:00:00", i + 1);
            sqlx::query("UPDATE games SET updated_at = $1::timestamp WHERE id = $2")
                .bind(ts)
                .bind(g.id)
                .execute(&pool)
                .await
                .unwrap();
            game_ids.push(g.id);
        }

        let rows = recent_games_for_index(&pool, user.id, 10).await.unwrap();
        assert_eq!(rows.len(), 10);
        assert_eq!(rows[0].0, game_ids[11], "most recent first");
        assert_eq!(rows[9].0, game_ids[2]);
    }

    #[sqlx::test]
    async fn recent_games_for_index_has_correct_turn_and_finished_flags(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let user = make_user(&pool, "alice").await;
        let opp = make_user(&pool, "bob").await;

        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();

        let my_turn = make_game_with_players(&pool, gv, user.id, &[opp.id], 0, &[0, 1]).await;
        sqlx::query("UPDATE games SET updated_at = '2026-01-03 00:00:00' WHERE id = $1")
            .bind(my_turn.id)
            .execute(&pool)
            .await
            .unwrap();

        let not_my_turn = make_game_with_players(&pool, gv, user.id, &[opp.id], 0, &[]).await;
        sqlx::query("UPDATE games SET updated_at = '2026-01-02 00:00:00' WHERE id = $1")
            .bind(not_my_turn.id)
            .execute(&pool)
            .await
            .unwrap();

        let finished = make_game_with_players(&pool, gv, user.id, &[opp.id], 0, &[0, 1]).await;
        sqlx::query(
            "UPDATE games SET is_finished = true, finished_at = NOW(), updated_at = '2026-01-01 00:00:00' WHERE id = $1",
        )
        .bind(finished.id)
        .execute(&pool)
        .await
        .unwrap();

        let rows = recent_games_for_index(&pool, user.id, 10).await.unwrap();
        assert_eq!(rows.len(), 3);

        let r0 = &rows[0];
        assert_eq!(r0.0, my_turn.id);
        assert!(!r0.2, "not finished");
        assert!(r0.3, "is my turn");

        let r1 = &rows[1];
        assert_eq!(r1.0, not_my_turn.id);
        assert!(!r1.2, "not finished");
        assert!(!r1.3, "not my turn");

        let r2 = &rows[2];
        assert_eq!(r2.0, finished.id);
        assert!(r2.2, "is finished");
    }

    /// ws F40: the inlined predicate must agree with `is_game_visible_to_user`
    /// case for case. Four visibility tiers x one game each; for each case,
    /// "the function returned this game" must equal "the shared predicate says
    /// this game is visible".
    ///
    /// Each case gets its OWN `friend` user (and therefore its own one-game
    /// scan universe), so no rows have to be deleted between cases and the
    /// `scan_limit` window is unambiguous.
    #[sqlx::test]
    async fn friend_recent_visible_game_matches_is_game_visible_to_user(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let viewer = make_user(&pool, "viewer").await;

        // Four co-players, one per case: public (the default), 'friends' and a
        // friend of the viewer, 'friends' but NOT a friend of the viewer, and
        // 'private'.
        let co_public = make_user(&pool, "copublic").await;
        let co_friends_yes = make_user(&pool, "cofriendsyes").await;
        let co_friends_no = make_user(&pool, "cofriendsno").await;
        let co_private = make_user(&pool, "coprivate").await;
        set_game_visibility(&pool, co_friends_yes.id, "friends")
            .await
            .unwrap();
        set_game_visibility(&pool, co_friends_no.id, "friends")
            .await
            .unwrap();
        set_game_visibility(&pool, co_private.id, "private")
            .await
            .unwrap();
        accept_friends(&pool, viewer.id, co_friends_yes.id).await;

        for (case, co, expected_visible) in [
            ("public", co_public.id, true),
            ("friends_yes", co_friends_yes.id, true),
            ("friends_no", co_friends_no.id, false),
            ("private", co_private.id, false),
        ] {
            // A fresh friend per case. The friend stays at the 'public'
            // default so only `co` can hide the game.
            let friend = make_user(&pool, &format!("friend_{case}")).await;
            accept_friends(&pool, viewer.id, friend.id).await;

            let game = make_game_with_players(&pool, gv, friend.id, &[co], 0, &[0]).await;
            let via_predicate = is_game_visible_to_user(&pool, game.id, viewer.id)
                .await
                .unwrap();
            let via_inlined = friend_recent_visible_game(&pool, friend.id, viewer.id, 10)
                .await
                .unwrap()
                .map(|(id, _, _)| id)
                == Some(game.id);
            assert_eq!(
                via_predicate, expected_visible,
                "is_game_visible_to_user disagreed with the expected case {case}"
            );
            assert_eq!(
                via_inlined, via_predicate,
                "inlined predicate disagreed with is_game_visible_to_user for case {case}"
            );
        }
    }
}
