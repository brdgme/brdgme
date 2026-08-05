#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use uuid::Uuid;

const ELO_K: f32 = 32.0;

#[cfg(feature = "ssr")]
fn elo_transformed_rating(rating: i32) -> f32 {
    10f32.powf(rating as f32 / 400.0)
}

#[cfg(feature = "ssr")]
fn elo_expected_score(a_rating: i32, b_rating: i32) -> f32 {
    let a_trans = elo_transformed_rating(a_rating);
    let b_trans = elo_transformed_rating(b_rating);
    a_trans / (a_trans + b_trans)
}

#[cfg(feature = "ssr")]
fn elo_rating_change(a_rating: i32, b_rating: i32, a_score: f32) -> i32 {
    let a_expected = elo_expected_score(a_rating, b_rating);
    (ELO_K * (a_score - a_expected)).round() as i32
}

/// Computes and persists `ranked_placing` for every human player in a game
/// that just finished. Active humans keep their game placing order; departed
/// humans (ordered by `departure_sequence`) take the remaining placings.
/// Pure bots are omitted. Must run in the same transaction as the placings
/// write and before `apply_rating_changes`.
#[cfg(feature = "ssr")]
pub(crate) async fn write_ranked_placings(
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
) -> Result<()> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        user_id: Option<Uuid>,
        departure_sequence: Option<i32>,
        place: Option<i32>,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, user_id, departure_sequence, place FROM game_players WHERE game_id = $1",
    )
    .bind(game_id)
    .fetch_all(&mut *tx)
    .await?;

    let inputs: Vec<crate::game::placing::PlacingInput> = rows
        .iter()
        .map(|r| crate::game::placing::PlacingInput {
            game_player_id: r.id,
            is_pure_bot: r.user_id.is_none(),
            departure_sequence: r.departure_sequence,
            game_placing: r.place,
        })
        .collect();

    let ranked = crate::game::placing::compute_ranked_placings(&inputs);
    for (id, placing) in ranked {
        sqlx::query("UPDATE game_players SET ranked_placing = $1 WHERE id = $2")
            .bind(placing)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    Ok(())
}

/// Applies ELO rating changes for a game that just transitioned to finished
/// with placings. Must be called within the same transaction as the
/// placings write. No-op if the idempotency guard trips (any player already
/// has a rating_change). Bot players are excluded from the calculation;
/// only human players are rated against each other.
#[cfg(feature = "ssr")]
pub(crate) async fn apply_rating_changes(tx: &mut sqlx::PgConnection, game_id: Uuid) -> Result<()> {
    struct PlayerRow {
        id: Uuid,
        position: i32,
        user_id: Option<Uuid>,
        place: Option<i32>,
        ranked_placing: Option<i32>,
        rating_change: Option<i32>,
    }

    let players = sqlx::query_as!(
        PlayerRow,
        "SELECT id, position, user_id, place, ranked_placing, rating_change FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    if players.iter().any(|p| p.rating_change.is_some()) {
        // Idempotency guard: this game has already been rated.
        return Ok(());
    }
    if players.iter().all(|p| p.place.is_none()) {
        return Ok(());
    }

    let game_type_id = sqlx::query_scalar!(
        r#"
        SELECT gv.game_type_id
        FROM games g
        JOIN game_versions gv ON gv.id = g.game_version_id
        WHERE g.id = $1
        "#,
        game_id
    )
    .fetch_one(&mut *tx)
    .await?;

    struct RatedPlayer {
        position: i32,
        user_id: Uuid,
        rating: i32,
    }

    let mut rated_players = Vec::with_capacity(players.len());
    for p in &players {
        if p.user_id.is_none() {
            continue;
        }
        let user_id = p.user_id.ok_or_else(|| {
            anyhow::anyhow!("game_player {}: user_id missing for human player", p.id)
        })?;

        sqlx::query!(
            "INSERT INTO game_type_users (game_type_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            game_type_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        let rating = sqlx::query_scalar!(
            "SELECT rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2",
            game_type_id,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        rated_players.push(RatedPlayer {
            position: p.position,
            user_id,
            rating,
        });
    }

    let rating_befores: std::collections::HashMap<i32, i32> = rated_players
        .iter()
        .map(|p| (p.position, p.rating))
        .collect();

    if rated_players.len() < 2 {
        return Ok(());
    }

    let places: std::collections::HashMap<i32, i32> = players
        .iter()
        .map(|p| (p.position, p.ranked_placing.or(p.place).unwrap_or(i32::MAX)))
        .collect();

    let mut rating_changes: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
    // Each unordered pair exactly once: index `i` against the tail slice.
    // (Was `.take(len - 1).enumerate()` + `.skip(a_index + 1)`; the `take` was
    // redundant because the last index yields an empty tail - ws F50.)
    for (i, a) in rated_players.iter().enumerate() {
        for b in &rated_players[i + 1..] {
            let a_place = places.get(&a.position).copied().unwrap_or(i32::MAX);
            let b_place = places.get(&b.position).copied().unwrap_or(i32::MAX);
            let a_score: f32 = match a_place.cmp(&b_place) {
                std::cmp::Ordering::Less => 1.0,
                std::cmp::Ordering::Equal => 0.5,
                std::cmp::Ordering::Greater => 0.0,
            };
            let change = elo_rating_change(a.rating, b.rating, a_score);
            *rating_changes.entry(a.position).or_insert(0) += change;
            *rating_changes.entry(b.position).or_insert(0) -= change;
        }
    }

    for p in &rated_players {
        let change = rating_changes.get(&p.position).copied().unwrap_or(0);
        if change == 0 {
            continue;
        }
        sqlx::query!(
            r#"
            UPDATE game_type_users
            SET rating = rating + $1, peak_rating = GREATEST(peak_rating, rating + $1)
            WHERE game_type_id = $2 AND user_id = $3
            "#,
            change,
            game_type_id,
            p.user_id
        )
        .execute(&mut *tx)
        .await?;
    }

    for p in &players {
        let Some(&change) = rating_changes.get(&p.position) else {
            continue;
        };
        let rating_before = rating_befores.get(&p.position).copied();
        sqlx::query("UPDATE game_players SET rating_change = $1, rating_before = $2 WHERE id = $3")
            .bind(change)
            .bind(rating_before)
            .bind(p.id)
            .execute(&mut *tx)
            .await?;
    }

    Ok(())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use crate::db::*;
    use crate::game::StatusUpdate;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    #[sqlx::test]
    async fn ratings_use_ranked_placing_and_skip_pure_bots(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        // 2 humans + 1 bot. Positions are shuffled by create_game_with_users, so
        // look them up explicitly rather than assuming 0/1/2.
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opp.id], 1, &[0]).await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let opp_pos = position_of(&ge, opp.id);
        let bot_pos = ge
            .game_players
            .iter()
            .find(|p| p.game_player.user_id.is_none())
            .unwrap()
            .game_player
            .position;
        let game_bot_id: Uuid = sqlx::query_scalar(
            "SELECT game_bot_id FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(bot_pos)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Replaced human (opp): both user_id and game_bot_id set. Best game
        // placing (1) but worst ranked placing (2).
        sqlx::query(
            "UPDATE game_players SET place = $1, ranked_placing = $2, left_at = NOW(), game_bot_id = $3 WHERE game_id = $4 AND position = $5",
        )
        .bind(1i32)
        .bind(2i32)
        .bind(game_bot_id)
        .bind(game.id)
        .bind(opp_pos)
        .execute(&pool)
        .await
        .unwrap();
        // Survivor (creator): game placing 2, ranked placing 1.
        sqlx::query(
            "UPDATE game_players SET place = $1, ranked_placing = $2 WHERE game_id = $3 AND position = $4",
        )
        .bind(2i32)
        .bind(1i32)
        .bind(game.id)
        .bind(creator_pos)
        .execute(&pool)
        .await
        .unwrap();
        // Pure bot: game placing 3, no ranked placing.
        sqlx::query("UPDATE game_players SET place = 3 WHERE game_id = $1 AND position = $2")
            .bind(game.id)
            .bind(bot_pos)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE games SET is_finished = true, finished_at = NOW() WHERE id = $1")
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        apply_rating_changes(&mut tx, game.id).await.unwrap();
        tx.commit().await.unwrap();

        // The replaced human (opp) must be rated (has user_id) despite game_bot_id.
        let rated: (Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT rating_change, ranked_placing FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opp_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            rated.0.is_some(),
            "replaced human must receive a rating change"
        );
        // The pure bot must NOT be rated.
        let bot_rated: Option<i32> = sqlx::query_scalar(
            "SELECT rating_change FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(bot_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(bot_rated.is_none(), "pure bot must not be rated");
    }

    #[test]
    fn elo_rating_change_works() {
        assert_eq!(elo_rating_change(1184, 1200, 0.0), -15i32);
        assert_eq!(elo_rating_change(2400, 2000, 0.0), -29i32);
        assert_eq!(elo_rating_change(2400, 2000, 1.0), 3i32);
        assert_eq!(elo_rating_change(2400, 2000, 0.5), -13i32);
    }

    #[test]
    fn elo_rating_change_three_player_pairwise_sums_to_zero() {
        // Simulates the pairwise accumulation done in apply_rating_changes for
        // a 3-player game with placings [1, 2, 3] (position 0 wins, 1 second,
        // 2 last) and equal starting ratings.
        let ratings = [1200, 1200, 1200];
        let places = [1, 2, 3];
        let mut changes = [0i32; 3];
        for a in 0..ratings.len() - 1 {
            for b in (a + 1)..ratings.len() {
                let a_score: f32 = match places[a].cmp(&places[b]) {
                    std::cmp::Ordering::Less => 1.0,
                    std::cmp::Ordering::Equal => 0.5,
                    std::cmp::Ordering::Greater => 0.0,
                };
                let change = elo_rating_change(ratings[a], ratings[b], a_score);
                changes[a] += change;
                changes[b] -= change;
            }
        }
        // Zero-sum: total rating points gained equals total lost.
        assert_eq!(changes.iter().sum::<i32>(), 0);
        // Winner gains, last place loses.
        assert!(changes[0] > 0);
        assert!(changes[2] < 0);
        assert_eq!(changes, [32, 0, -32]);
    }

    async fn find_rating_change(pool: &PgPool, game_id: Uuid, position: i32) -> Option<i32> {
        sqlx::query_scalar!(
            "SELECT rating_change FROM game_players WHERE game_id = $1 AND position = $2",
            game_id,
            position
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn game_type_rating(pool: &PgPool, game_type_id: Uuid, user_id: Uuid) -> (i32, i32) {
        let row = sqlx::query!(
            "SELECT rating, peak_rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2",
            game_type_id,
            user_id
        )
        .fetch_one(pool)
        .await
        .unwrap();
        (row.rating, row.peak_rating)
    }

    #[sqlx::test]
    async fn finishing_a_two_player_game_rates_both_players(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;

        // creator places 1st (winner), opponent 2nd (loser), by position.
        let mut placings = vec![0usize; 2];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        // Both players started at the DB default rating (1200): winner (place
        // 1) gains, loser (place 2) loses the same amount.
        let winner_change = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let loser_change = find_rating_change(&pool, game.id, opponent_pos as i32).await;
        assert_eq!(winner_change, Some(16));
        assert_eq!(loser_change, Some(-16));

        let (winner_rating, winner_peak) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (loser_rating, loser_peak) = game_type_rating(&pool, game_type_id, opponent.id).await;
        assert_eq!(winner_rating, 1216);
        assert_eq!(winner_peak, 1216);
        assert_eq!(loser_rating, 1184);
        assert_eq!(loser_peak, 1200);
    }

    #[sqlx::test]
    async fn finishing_a_three_player_game_rates_all_pairs(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let p1 = make_user(&pool, "p1").await;
        let p2 = make_user(&pool, "p2").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[p1.id, p2.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let p1_pos = position_of(&ge, p1.id) as usize;
        let p2_pos = position_of(&ge, p2.id) as usize;

        // creator 1st, p1 2nd, p2 3rd, by position.
        let mut placings = vec![0usize; 3];
        placings[creator_pos] = 1;
        placings[p1_pos] = 2;
        placings[p2_pos] = 3;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let c_creator = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let c_p1 = find_rating_change(&pool, game.id, p1_pos as i32).await;
        let c_p2 = find_rating_change(&pool, game.id, p2_pos as i32).await;
        assert_eq!(c_creator, Some(32));
        // A net-zero change is now written as Some(0) so the idempotency
        // guard is armed even for exact ties (WP-40 Task 5).
        assert_eq!(c_p1, Some(0));
        assert_eq!(c_p2, Some(-32));
        // Zero-sum across all pairs.
        assert_eq!(
            c_creator.unwrap_or(0) + c_p1.unwrap_or(0) + c_p2.unwrap_or(0),
            0
        );
    }

    #[sqlx::test]
    async fn second_finish_does_not_re_rate(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let (rating_after_first, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let ge_after_first = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // A second "finish" write (e.g. a retry) must not re-rate the game -
        // the idempotency guard trips because rating_change is already set.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "final_state",
            "final_state_2",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2],
            },
            &[],
            ge_after_first.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let (rating_after_second, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        assert_eq!(rating_after_first, rating_after_second);
    }

    #[sqlx::test]
    async fn two_player_game_with_bot_is_not_rated(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge
            .game_players
            .iter()
            .find(|p| p.user.is_some())
            .unwrap()
            .game_player
            .id;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.rating_change, None,
                "with only one human player, no pairwise rating is possible"
            );
        }
    }

    #[sqlx::test]
    async fn three_player_game_with_bot_rates_humans_only(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 1, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;
        let bot_pos = ge
            .game_players
            .iter()
            .find(|p| p.user.is_none())
            .unwrap()
            .game_player
            .position as usize;

        let mut placings = vec![0usize; 3];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;
        placings[bot_pos] = 3;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let creator_change = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let opponent_change = find_rating_change(&pool, game.id, opponent_pos as i32).await;
        let bot_change = find_rating_change(&pool, game.id, bot_pos as i32).await;

        assert!(creator_change.is_some());
        assert!(opponent_change.is_some());
        assert_eq!(bot_change, None);
        assert_eq!(creator_change.unwrap() + opponent_change.unwrap(), 0);

        let (creator_rating, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (opponent_rating, _) = game_type_rating(&pool, game_type_id, opponent.id).await;
        assert_eq!(creator_rating, 1200 + creator_change.unwrap());
        assert_eq!(opponent_rating, 1200 + opponent_change.unwrap());
    }

    #[sqlx::test]
    async fn game_type_users_row_created_on_first_rated_game(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        // Explicitly delete the game_type_users rows that create_game_with_users
        // auto-created, so the finish path must INSERT them itself.
        sqlx::query!(
            "DELETE FROM game_type_users WHERE game_type_id = $1",
            game_type_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;

        let mut placings = vec![0usize; 2];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let (winner_rating, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (loser_rating, _) = game_type_rating(&pool, game_type_id, opponent.id).await;
        // DB column default rating is 1200, so the newly-created rows started
        // there before the change was applied.
        assert_eq!(winner_rating, 1216);
        assert_eq!(loser_rating, 1184);
    }

    #[sqlx::test]
    async fn concede_game_assigns_places_and_rates(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceding = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let conceding_id = conceding.game_player.id;

        concede_game(&pool, game.id, conceding_id, "creator", ge.game.updated_at)
            .await
            .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceder = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.id == conceding_id)
            .unwrap();
        let non_conceder = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.id != conceding_id)
            .unwrap();
        assert_eq!(conceder.game_player.place, Some(2));
        assert_eq!(non_conceder.game_player.place, Some(1));
        assert_eq!(conceder.game_player.rating_change, Some(-16));
        assert_eq!(non_conceder.game_player.rating_change, Some(16));

        let (non_conceder_rating, _) =
            game_type_rating(&pool, game_type_id, non_conceder.user.as_ref().unwrap().id).await;
        assert_eq!(non_conceder_rating, 1216);
    }

    #[sqlx::test]
    async fn finishing_a_game_captures_rating_before(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 1, &[0])
                .await;

        sqlx::query(
            "UPDATE game_type_users SET rating = 1300 WHERE game_type_id = $1 AND user_id = $2",
        )
        .bind(game_type_id)
        .bind(creator.id)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "UPDATE game_type_users SET rating = 1100 WHERE game_type_id = $1 AND user_id = $2",
        )
        .bind(game_type_id)
        .bind(opponent.id)
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;
        let bot_pos = ge
            .game_players
            .iter()
            .find(|p| p.user.is_none())
            .unwrap()
            .game_player
            .position as usize;

        let mut placings = vec![0usize; 3];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;
        placings[bot_pos] = 3;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let creator_rb: Option<i32> = sqlx::query_scalar(
            "SELECT rating_before FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(creator_rb, Some(1300));

        let opponent_rb: Option<i32> = sqlx::query_scalar(
            "SELECT rating_before FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opponent_pos as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(opponent_rb, Some(1100));

        let bot_rb: Option<i32> = sqlx::query_scalar(
            "SELECT rating_before FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(bot_pos as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(bot_rb, None);

        let creator_change = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let opponent_change = find_rating_change(&pool, game.id, opponent_pos as i32).await;

        let (creator_rating_after, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (opponent_rating_after, _) = game_type_rating(&pool, game_type_id, opponent.id).await;

        assert_eq!(
            creator_rb.unwrap() + creator_change.unwrap(),
            creator_rating_after
        );
        assert_eq!(
            opponent_rb.unwrap() + opponent_change.unwrap(),
            opponent_rating_after
        );
    }

    #[sqlx::test]
    async fn apply_rating_changes_writes_zero_change(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let opponent_pos = position_of(&ge, opponent.id);

        sqlx::query(
            "UPDATE game_players SET place = 1, ranked_placing = 1 WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET place = 1, ranked_placing = 1 WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opponent_pos)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE games SET is_finished = true, finished_at = NOW() WHERE id = $1")
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        apply_rating_changes(&mut tx, game.id).await.unwrap();
        tx.commit().await.unwrap();

        let c_change = find_rating_change(&pool, game.id, creator_pos).await;
        let o_change = find_rating_change(&pool, game.id, opponent_pos).await;
        assert_eq!(c_change, Some(0));
        assert_eq!(o_change, Some(0));

        let c_rb: Option<i32> = sqlx::query_scalar(
            "SELECT rating_before FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(c_rb, Some(1200));

        let (creator_rating, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        assert_eq!(creator_rating, 1200);

        let mut tx = pool.begin().await.unwrap();
        apply_rating_changes(&mut tx, game.id).await.unwrap();
        tx.commit().await.unwrap();

        let (creator_rating_after, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        assert_eq!(creator_rating_after, 1200);
    }
}
