use super::*;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

/// Plain query, matching get_user_theme - invite_policy is deliberately NOT
/// a field on models::user::User.
#[cfg(feature = "ssr")]
pub async fn get_invite_policy(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT invite_policy FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

#[cfg(feature = "ssr")]
pub async fn set_invite_policy(pool: &PgPool, user_id: Uuid, policy: &str) -> Result<()> {
    sqlx::query("UPDATE users SET invite_policy = $1 WHERE id = $2")
        .bind(policy)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Plain query, matching get_invite_policy - game_visibility is deliberately
/// NOT a field on models::user::User (Unit B / R4).
#[cfg(feature = "ssr")]
pub async fn get_game_visibility(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT game_visibility FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

#[cfg(feature = "ssr")]
pub async fn set_game_visibility(pool: &PgPool, user_id: Uuid, visibility: &str) -> Result<()> {
    sqlx::query("UPDATE users SET game_visibility = $1 WHERE id = $2")
        .bind(visibility)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Bulk lookup of (user_id, game_visibility) for the given users, for
/// game-visibility checks across many players at once.
#[cfg(feature = "ssr")]
pub async fn find_game_visibility_for_users_tx(
    tx: &mut sqlx::PgConnection,
    user_ids: &[Uuid],
) -> Result<Vec<(Uuid, String)>> {
    Ok(
        sqlx::query_as("SELECT id, game_visibility FROM users WHERE id = ANY($1)")
            .bind(user_ids)
            .fetch_all(tx)
            .await?,
    )
}

/// A game is publicly visible (eligible for the logged-out index) iff EVERY
/// human player has game_visibility = 'public'. Bots (user_id IS NULL) are
/// dropped by the JOIN and never block. Used by both the selection query and
/// the render fn so they cannot drift (Unit B section 2c).
#[cfg(feature = "ssr")]
pub async fn is_game_publicly_visible(pool: &PgPool, game_id: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT NOT EXISTS(
            SELECT 1 FROM game_players gp
            JOIN users u ON u.id = gp.user_id
            WHERE gp.game_id = $1 AND u.game_visibility <> 'public')",
    )
    .bind(game_id)
    .fetch_one(pool)
    .await?)
}

/// A game is visible to `viewer_id` iff the viewer is one of its players, OR
/// every human player is either 'public' or ('friends' AND friends with the
/// viewer). A 'private' player blocks all non-self viewing. Bots never block.
/// **This predicate is duplicated once**, inlined into
/// `friend_recent_visible_game` to avoid a per-candidate round trip (ws F40).
/// The two are kept in step by
/// `friend_recent_visible_game_matches_is_game_visible_to_user`; if you change
/// the rule here, change it there and that test will tell you if you missed a
/// case.
#[cfg(feature = "ssr")]
pub async fn is_game_visible_to_user(
    pool: &PgPool,
    game_id: Uuid,
    viewer_id: Uuid,
) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_players WHERE game_id = $1 AND user_id = $2)
           OR NOT EXISTS(
             SELECT 1 FROM game_players gp
             JOIN users u ON u.id = gp.user_id
             WHERE gp.game_id = $1
               AND NOT (
                 u.game_visibility = 'public'
                 OR (u.game_visibility = 'friends' AND EXISTS(
                   SELECT 1 FROM friends f WHERE f.has_accepted = TRUE
                     AND ((f.source_user_id = $2 AND f.target_user_id = u.id)
                       OR (f.target_user_id = $2 AND f.source_user_id = u.id))
                 ))
               ))",
    )
    .bind(game_id)
    .bind(viewer_id)
    .fetch_one(pool)
    .await?)
}

/// Thin dispatcher for WP-42's per-socket filter: `None` viewer delegates to
/// `is_game_publicly_visible`, `Some(v)` to `is_game_visible_to_user`.
#[cfg(feature = "ssr")]
pub async fn is_game_visible_to_viewer(
    pool: &PgPool,
    game_id: Uuid,
    viewer: Option<Uuid>,
) -> Result<bool> {
    match viewer {
        None => is_game_publicly_visible(pool, game_id).await,
        Some(v) => is_game_visible_to_user(pool, game_id, v).await,
    }
}

/// The subset of `user_ids` whose identity may be shown to `viewer`.
/// Cross-references the canonical per-player clause in `is_game_visible_to_user`:
/// 'public' passes for everyone; 'friends' passes only for accepted friends
/// (either direction); 'private' passes only for self. A NULL viewer leaves
/// only 'public' passing. One query, no N+1.
#[cfg(feature = "ssr")]
pub async fn visible_user_ids(
    pool: &PgPool,
    user_ids: &[Uuid],
    viewer: Option<Uuid>,
) -> Result<std::collections::HashSet<Uuid>> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT u.id FROM users u WHERE u.id = ANY($1)
           AND (
             u.game_visibility = 'public'
             OR ($2::uuid IS NOT NULL AND u.id = $2)
             OR (u.game_visibility = 'friends' AND $2::uuid IS NOT NULL AND EXISTS(
               SELECT 1 FROM friends f WHERE f.has_accepted = TRUE
                 AND ((f.source_user_id = $2 AND f.target_user_id = u.id)
                   OR (f.target_user_id = $2 AND f.source_user_id = u.id))
             ))
           )",
    )
    .bind(user_ids)
    .bind(viewer)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// D4 + D7 enforcement choke point. Call after the roster is known but
/// before players are attached: create_new_game and restart_game today,
/// #24's create_proposal and any future matchmaking tomorrow.
///
/// Emails resolving to no account pass (the account is created at game
/// creation with default 'open' and can have no blocks). Block-by-target
/// uses wording identical to policy 'none' so a blocked creator cannot
/// distinguish the two (D7 detectability).
#[cfg(feature = "ssr")]
pub async fn check_invite_policy_tx(
    tx: &mut sqlx::PgConnection,
    creator_id: Uuid,
    opponent_ids: &[Uuid],
    opponent_emails: &[String],
) -> Result<Vec<String>> {
    let mut targets: Vec<Uuid> = opponent_ids.to_vec();
    for email in opponent_emails {
        let existing: Option<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM user_emails WHERE email = $1")
                .bind(email)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some(id) = existing {
            targets.push(id);
        }
    }
    targets.sort();
    targets.dedup();

    let mut violations = Vec::new();
    for target in targets {
        if target == creator_id {
            continue;
        }
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT name, invite_policy FROM users WHERE id = $1")
                .bind(target)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((name, policy)) = row else {
            violations.push("Player not found".to_string());
            continue;
        };
        if has_block_conn(&mut *tx, target, creator_id).await? {
            violations.push(format!("{name} is not accepting game invitations"));
            continue;
        }
        if has_block_conn(&mut *tx, creator_id, target).await? {
            violations.push(format!("You have blocked {name}"));
            continue;
        }
        if policy == "none" {
            violations.push(format!("{name} is not accepting game invitations"));
        } else if policy == "friends" && !are_friends_conn(&mut *tx, creator_id, target).await? {
            violations.push(format!("{name} only accepts games from friends"));
        } // 'open' passes
    }
    Ok(violations)
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::postgres::PgPool;

    #[sqlx::test]
    async fn invite_policy_default_open_allows_everyone(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        assert_eq!(get_invite_policy(&pool, b.id).await.unwrap(), "open");
        assert!(check_roster(&pool, a.id, &[b.id], &[]).await.is_empty());
    }

    #[sqlx::test]
    async fn invite_policy_none_blocks_with_generic_message(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        set_invite_policy(&pool, b.id, "none").await.unwrap();
        assert_eq!(
            check_roster(&pool, a.id, &[b.id], &[]).await,
            vec!["bob is not accepting game invitations".to_string()]
        );
    }

    #[sqlx::test]
    async fn invite_policy_friends_requires_accepted_friendship(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        set_invite_policy(&pool, b.id, "friends").await.unwrap();
        assert_eq!(
            check_roster(&pool, a.id, &[b.id], &[]).await,
            vec!["bob only accepts games from friends".to_string()]
        );
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        // pending is not enough
        assert!(!check_roster(&pool, a.id, &[b.id], &[]).await.is_empty());
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        assert!(check_roster(&pool, a.id, &[b.id], &[]).await.is_empty());
    }

    #[sqlx::test]
    async fn policy_check_covers_email_of_existing_user(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)")
            .bind(b.id)
            .bind("bob@example.com")
            .execute(&pool)
            .await
            .unwrap();
        set_invite_policy(&pool, b.id, "none").await.unwrap();
        assert_eq!(
            check_roster(&pool, a.id, &[], &["bob@example.com".to_string()]).await,
            vec!["bob is not accepting game invitations".to_string()]
        );
        // unknown email = account created later with default 'open': passes
        assert!(
            check_roster(&pool, a.id, &[], &["new@example.com".to_string()])
                .await
                .is_empty()
        );
    }

    #[sqlx::test]
    async fn blocks_stop_game_inclusion_both_ways(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        block_user(&pool, b.id, a.id).await.unwrap();
        // b blocked a: a's attempt fails with wording identical to policy
        // 'none' (deniability, D7)
        assert_eq!(
            check_roster(&pool, a.id, &[b.id], &[]).await,
            vec!["bob is not accepting game invitations".to_string()]
        );
        // and b cannot rope a into a game either, with an honest message
        assert_eq!(
            check_roster(&pool, b.id, &[a.id], &[]).await,
            vec!["You have blocked alice".to_string()]
        );
    }

    #[sqlx::test]
    async fn game_visibility_defaults_to_public(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        assert_eq!(get_game_visibility(&pool, a.id).await.unwrap(), "public");
    }

    #[sqlx::test]
    async fn set_game_visibility_round_trips_each_value(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        for v in ["friends", "private", "public"] {
            set_game_visibility(&pool, a.id, v).await.unwrap();
            assert_eq!(get_game_visibility(&pool, a.id).await.unwrap(), v);
        }
    }

    #[sqlx::test]
    async fn find_game_visibility_for_users_tx_bulk_lookup(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        set_game_visibility(&pool, b.id, "private").await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        let mut rows = find_game_visibility_for_users_tx(&mut tx, &[a.id, b.id])
            .await
            .unwrap();
        tx.commit().await.unwrap();
        rows.sort_by_key(|(id, _)| *id);
        let mut expected = vec![(a.id, "public".to_string()), (b.id, "private".to_string())];
        expected.sort_by_key(|(id, _)| *id);
        assert_eq!(rows, expected);
    }

    #[sqlx::test]
    async fn is_game_publicly_visible_requires_all_public(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        // both default 'public'
        assert!(is_game_publicly_visible(&pool, game.id).await.unwrap());

        set_game_visibility(&pool, b.id, "friends").await.unwrap();
        assert!(!is_game_publicly_visible(&pool, game.id).await.unwrap());

        set_game_visibility(&pool, b.id, "public").await.unwrap();
        set_game_visibility(&pool, a.id, "private").await.unwrap();
        assert!(!is_game_publicly_visible(&pool, game.id).await.unwrap());
    }

    #[sqlx::test]
    async fn is_game_publicly_visible_ignores_bots(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        // one human (public) + one bot: bots never block
        let game = make_game_with_players(&pool, gv, a.id, &[], 1, &[0]).await;
        assert!(is_game_publicly_visible(&pool, game.id).await.unwrap());
    }

    #[sqlx::test]
    async fn is_game_visible_to_user_friends_tier(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let friend = make_user(&pool, "cara").await;
        let stranger = make_user(&pool, "dan").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        set_game_visibility(&pool, a.id, "friends").await.unwrap();
        // b is public; a is 'friends'
        accept_friends(&pool, a.id, friend.id).await;

        // a player in the game always sees it
        assert!(is_game_visible_to_user(&pool, game.id, b.id).await.unwrap());
        // a friend of the 'friends' player sees it
        assert!(
            is_game_visible_to_user(&pool, game.id, friend.id)
                .await
                .unwrap()
        );
        // a non-friend does not
        assert!(
            !is_game_visible_to_user(&pool, game.id, stranger.id)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn is_game_visible_to_user_private_blocks_non_self(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let friend = make_user(&pool, "cara").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        set_game_visibility(&pool, a.id, "private").await.unwrap();
        accept_friends(&pool, a.id, friend.id).await;

        // the private player (a self player) and the other player still see it
        assert!(is_game_visible_to_user(&pool, game.id, a.id).await.unwrap());
        assert!(is_game_visible_to_user(&pool, game.id, b.id).await.unwrap());
        // even a friend of the private player does not
        assert!(
            !is_game_visible_to_user(&pool, game.id, friend.id)
                .await
                .unwrap()
        );
    }

    /// ws F51(2): with TWO 'friends'-tier players, a viewer who is a friend of
    /// only one of them must NOT see the game - the rule is "no player fails
    /// the check", not "some player passes it".
    #[sqlx::test]
    async fn is_game_visible_to_user_friends_tier_requires_every_friends_player(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let half_friend = make_user(&pool, "cara").await;
        let both_friend = make_user(&pool, "dana").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        set_game_visibility(&pool, a.id, "friends").await.unwrap();
        set_game_visibility(&pool, b.id, "friends").await.unwrap();
        // cara is friends with `a` only; dana is friends with both.
        accept_friends(&pool, a.id, half_friend.id).await;
        accept_friends(&pool, a.id, both_friend.id).await;
        accept_friends(&pool, b.id, both_friend.id).await;

        assert!(
            !is_game_visible_to_user(&pool, game.id, half_friend.id)
                .await
                .unwrap(),
            "a viewer friends with only one of two 'friends' players must NOT see the game"
        );
        assert!(
            is_game_visible_to_user(&pool, game.id, both_friend.id)
                .await
                .unwrap(),
            "a viewer friends with every 'friends' player must see the game"
        );
    }

    #[sqlx::test]
    async fn visible_user_ids_matrix(pool: PgPool) {
        let pub_user = make_user(&pool, "pub_user").await;
        let friends_user = make_user(&pool, "friends_user").await;
        let priv_user = make_user(&pool, "priv_user").await;
        let friend_of = make_user(&pool, "friend_of").await;
        let stranger = make_user(&pool, "stranger").await;

        set_game_visibility(&pool, pub_user.id, "public")
            .await
            .unwrap();
        set_game_visibility(&pool, friends_user.id, "friends")
            .await
            .unwrap();
        set_game_visibility(&pool, priv_user.id, "private")
            .await
            .unwrap();
        accept_friends(&pool, friends_user.id, friend_of.id).await;

        let all = [pub_user.id, friends_user.id, priv_user.id];

        // Anonymous viewer: only public passes
        let vis = visible_user_ids(&pool, &all, None).await.unwrap();
        assert!(vis.contains(&pub_user.id));
        assert!(!vis.contains(&friends_user.id));
        assert!(!vis.contains(&priv_user.id));

        // Stranger viewer: public passes, friends/private do not
        let vis = visible_user_ids(&pool, &all, Some(stranger.id))
            .await
            .unwrap();
        assert!(vis.contains(&pub_user.id));
        assert!(!vis.contains(&friends_user.id));
        assert!(!vis.contains(&priv_user.id));

        // Friend of friends_user: public + friends_user pass
        let vis = visible_user_ids(&pool, &all, Some(friend_of.id))
            .await
            .unwrap();
        assert!(vis.contains(&pub_user.id));
        assert!(vis.contains(&friends_user.id));
        assert!(!vis.contains(&priv_user.id));

        // Self: private user can see themselves
        let vis = visible_user_ids(&pool, &all, Some(priv_user.id))
            .await
            .unwrap();
        assert!(vis.contains(&pub_user.id));
        assert!(!vis.contains(&friends_user.id));
        assert!(vis.contains(&priv_user.id));
    }

    #[sqlx::test]
    async fn visible_user_ids_drift_guard_matches_is_game_visible_to_user(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let viewer = make_user(&pool, "viewer").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        set_game_visibility(&pool, a.id, "friends").await.unwrap();
        set_game_visibility(&pool, b.id, "public").await.unwrap();

        // viewer is NOT a player. The game is visible iff every human player
        // is in visible_user_ids.
        let game_visible = is_game_visible_to_user(&pool, game.id, viewer.id)
            .await
            .unwrap();
        let vis = visible_user_ids(&pool, &[a.id, b.id], Some(viewer.id))
            .await
            .unwrap();
        let all_visible = vis.contains(&a.id) && vis.contains(&b.id);
        assert_eq!(
            game_visible, all_visible,
            "drift: is_game_visible_to_user says {game_visible} but visible_user_ids says {all_visible}"
        );

        // Now make viewer a friend of alice - both should agree the game is visible
        accept_friends(&pool, a.id, viewer.id).await;
        let game_visible = is_game_visible_to_user(&pool, game.id, viewer.id)
            .await
            .unwrap();
        let vis = visible_user_ids(&pool, &[a.id, b.id], Some(viewer.id))
            .await
            .unwrap();
        let all_visible = vis.contains(&a.id) && vis.contains(&b.id);
        assert_eq!(
            game_visible, all_visible,
            "drift after friendship: is_game_visible_to_user says {game_visible} but visible_user_ids says {all_visible}"
        );
        assert!(game_visible);
    }
}
