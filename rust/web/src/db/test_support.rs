use super::*;
use crate::models::user::User;
use sqlx::postgres::PgPool;
use uuid::Uuid;

pub(crate) async fn make_user(pool: &PgPool, name: &str) -> User {
    sqlx::query_as!(
        User,
        "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING id, created_at, updated_at, name, pref_colors, theme, is_admin",
        Uuid::new_v4(),
        name,
        &Vec::<String>::new()
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Creates a game type + a public, non-deprecated game version pointing at a
/// dummy URI. None of the db.rs functions under test call out to the game
/// service over HTTP, so the URI is never dereferenced.
pub(crate) async fn make_game_type_and_version(pool: &PgPool) -> (Uuid, Uuid) {
    let game_type_id = sqlx::query_scalar!(
        "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        format!("Test Game {}", Uuid::new_v4()),
        &vec![2, 3, 4]
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let game_version_id = sqlx::query_scalar!(
        r#"INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
           VALUES ($1, $2, $3, true, false) RETURNING id"#,
        game_type_id,
        "test-v1",
        "http://localhost:0/mock"
    )
    .fetch_one(pool)
    .await
    .unwrap();

    (game_type_id, game_version_id)
}

/// Creates a fixture game with `human_users.len()` human players followed by
/// `bot_count` bot players (positions assigned in that order), using
/// `create_game_with_users` so the function under test in point 1 doubles as
/// the fixture builder for the other tests.
pub(crate) async fn make_game_with_players(
    pool: &PgPool,
    game_version_id: Uuid,
    creator_id: Uuid,
    opponent_ids: &[Uuid],
    bot_count: usize,
    whose_turn: &[usize],
) -> crate::models::game::Game {
    let bot_slots: Vec<BotSlot> = (0..bot_count)
        .map(|i| BotSlot {
            name: format!("Bot {}", i),
            bot_name: "easy".to_string(),
        })
        .collect();

    create_game_with_users(
        pool,
        CreateGameOpts {
            game_version_id,
            whose_turn,
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id,
            opponent_ids,
            opponent_emails: &[],
            bot_slots: &bot_slots,
            chat_id: None,
            game_state: "initial_state",
            all_accepted: false,
        },
    )
    .await
    .unwrap()
}

pub(crate) async fn make_proposal(pool: &PgPool, game_version_id: Uuid, owner_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO game_proposals (game_version_id, owner_user_id, status) VALUES ($1,$2,'open') RETURNING id",
    )
    .bind(game_version_id)
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

pub(crate) async fn add_proposal_player(
    pool: &PgPool,
    proposal_id: Uuid,
    position: i32,
    user_id: Option<Uuid>,
    bot_name: Option<&str>,
    response: &str,
) {
    sqlx::query(
        "INSERT INTO game_proposal_players (proposal_id, position, user_id, bot_name, response) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(proposal_id)
    .bind(position)
    .bind(user_id)
    .bind(bot_name)
    .bind(response)
    .execute(pool)
    .await
    .unwrap();
}

pub(crate) async fn finish_game(pool: &PgPool, game_id: Uuid) {
    sqlx::query("UPDATE games SET is_finished = true WHERE id = $1")
        .bind(game_id)
        .execute(pool)
        .await
        .unwrap();
}

pub(crate) async fn set_recently_active(pool: &PgPool, user_id: Uuid) {
    sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
}

pub(crate) async fn set_stale(pool: &PgPool, user_id: Uuid) {
    sqlx::query("UPDATE users SET last_active_at = NOW() - INTERVAL '1 hour' WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
}

/// Test-only row counter. **The `format!`-built SQL is safe ONLY because
/// every caller passes a hard-coded table-name literal.** Do not copy this
/// pattern outside `mod tests`, and never pass a runtime value for `table`
/// (ws F51(3)).
pub(crate) async fn count_rows(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
        .fetch_one(pool)
        .await
        .unwrap()
}

/// `create_game_with_users` shuffles slot order before assigning
/// positions, so a user's position within a game is not predictable from
/// call order. Look it up explicitly rather than assuming position 0/1/2.
pub(crate) fn position_of(ge: &GameExtended, user_id: Uuid) -> i32 {
    ge.game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user_id))
        .unwrap()
        .game_player
        .position
}

pub(crate) async fn check_roster(
    pool: &PgPool,
    creator: Uuid,
    ids: &[Uuid],
    emails: &[String],
) -> Vec<String> {
    let mut tx = pool.begin().await.unwrap();
    let v = check_invite_policy_tx(&mut tx, creator, ids, emails)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    v
}

pub(crate) async fn accept_friends(pool: &PgPool, a: Uuid, b: Uuid) {
    send_friend_request(pool, a, b).await.unwrap();
    send_friend_request(pool, b, a).await.unwrap();
}
