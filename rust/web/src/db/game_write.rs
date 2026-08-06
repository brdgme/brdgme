use super::*;
#[cfg(feature = "ssr")]
use crate::game::StatusUpdate;
#[cfg(feature = "ssr")]
use crate::models::user::User;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

#[cfg(feature = "ssr")]
pub struct CreateGameOpts<'a> {
    pub game_version_id: Uuid,
    pub whose_turn: &'a [usize],
    pub eliminated: &'a [usize],
    pub placings: &'a [usize],
    pub points: &'a [f32],
    pub creator_id: Uuid,
    pub opponent_ids: &'a [Uuid],
    pub opponent_emails: &'a [crate::auth::email_addr::CanonicalEmail],
    pub bot_slots: &'a [BotSlot],
    pub chat_id: Option<Uuid>,
    pub game_state: &'a str,
    pub all_accepted: bool,
}

#[cfg(feature = "ssr")]
enum PlayerSlotInternal {
    User(User),
    Bot { name: String, bot_name: String },
}

#[cfg(feature = "ssr")]
pub async fn create_game_with_users(
    pool: &PgPool,
    opts: CreateGameOpts<'_>,
) -> Result<crate::models::game::Game> {
    let mut tx = pool.begin().await?;
    let game = create_game_with_users_tx(&mut tx, opts).await?;
    tx.commit().await?;
    Ok(game)
}

/// Creates a game and its players within an existing transaction, so callers
/// can commit them atomically alongside other writes (e.g. the restarted-game
/// linkage in `restart_game`).
#[cfg(feature = "ssr")]
#[tracing::instrument(skip_all)]
pub async fn create_game_with_users_tx(
    tx: &mut sqlx::PgConnection,
    opts: CreateGameOpts<'_>,
) -> Result<crate::models::game::Game> {
    // 1. Find or create users; collect all slots (users + bots)
    let mut slots: Vec<PlayerSlotInternal> = Vec::new();

    // Creator
    let creator = sqlx::query_as!(
        crate::models::user::User,
        "SELECT id, created_at, updated_at, name, pref_colors, theme, is_admin FROM users WHERE id = $1",
        opts.creator_id
    )
    .fetch_one(&mut *tx)
    .await?;
    slots.push(PlayerSlotInternal::User(creator));

    // Opponent IDs
    for &id in opts.opponent_ids {
        let opponent = sqlx::query_as!(
            crate::models::user::User,
            "SELECT id, created_at, updated_at, name, pref_colors, theme, is_admin FROM users WHERE id = $1",
            id
        )
        .fetch_one(&mut *tx)
        .await?;
        slots.push(PlayerSlotInternal::User(opponent));
    }

    // Opponent Emails
    for email in opts.opponent_emails {
        let user = if let Some(u) = sqlx::query_as!(
            crate::models::user::User,
            r#"SELECT u.id, u.created_at, u.updated_at, u.name, u.pref_colors, u.theme, u.is_admin
               FROM users u JOIN user_emails ue ON u.id = ue.user_id WHERE ue.email = $1"#,
            email.as_str()
        )
        .fetch_optional(&mut *tx)
        .await?
        {
            u
        } else {
            // Create new user for email
            let new_user_id = Uuid::new_v4();
            let username = generate_unique_username(&mut *tx).await?;

            let u = sqlx::query_as!(
                crate::models::user::User,
                "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING id, created_at, updated_at, name, pref_colors, theme, is_admin",
                new_user_id,
                username,
                &Vec::<String>::new()
            )
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
                 VALUES ($1, $2, true, NOW())",
            )
            .bind(new_user_id)
            .bind(email.as_str())
            .execute(&mut *tx)
            .await?;

            u
        };
        slots.push(PlayerSlotInternal::User(user));
    }

    // Bot slots
    for bot in opts.bot_slots {
        slots.push(PlayerSlotInternal::Bot {
            name: bot.name.clone(),
            bot_name: bot.bot_name.clone(),
        });
    }

    // 2. Randomize player order
    {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        slots.shuffle(&mut rng);
    }

    // 3. Assign colors, honouring each user's preferred colors where possible.
    let palette = crate::theme::PLAYER_COLOR_NAMES;
    let prefs: Vec<Vec<String>> = slots
        .iter()
        .map(|slot| match slot {
            PlayerSlotInternal::User(user) => user.pref_colors.clone(),
            PlayerSlotInternal::Bot { .. } => vec![],
        })
        .collect();
    let colors = choose_colors(&prefs, &palette);

    // 4. Create Game
    let is_finished = !opts.placings.is_empty();
    let game = sqlx::query_as::<_, crate::models::game::Game>(
        r#"
        INSERT INTO games (game_version_id, is_finished, game_state, chat_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, created_at, updated_at, game_version_id, is_finished, finished_at, game_state, chat_id, restarted_game_id, end_reason
        "#,
    )
    .bind(opts.game_version_id)
    .bind(is_finished)
    .bind(opts.game_state)
    .bind(opts.chat_id)
    .fetch_one(&mut *tx)
    .await?;

    // 5. Create Players
    let game_type_id = sqlx::query_scalar!(
        "SELECT game_type_id FROM game_versions WHERE id = $1",
        opts.game_version_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Game version not found"))?;

    for (pos, slot) in slots.iter().enumerate() {
        let color = colors
            .get(pos)
            .cloned()
            .unwrap_or_else(|| "Pink".to_string());
        let is_turn = opts.whose_turn.contains(&pos);
        let is_eliminated = opts.eliminated.contains(&pos);
        let place = opts.placings.get(pos).map(|&p| p as i32);

        match slot {
            PlayerSlotInternal::User(user) => {
                sqlx::query!(
                    r#"
                    INSERT INTO game_players (game_id, user_id, position, color, has_accepted, is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place)
                    VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), $7, false, $8)
                    "#,
                    game.id,
                    user.id,
                    pos as i32,
                    color,
                    opts.all_accepted || user.id == opts.creator_id,
                    is_turn,
                    is_eliminated,
                    place
                )
                .execute(&mut *tx)
                .await?;

                sqlx::query!(
                    r#"
                    INSERT INTO game_type_users (game_type_id, user_id)
                    VALUES ($1, $2)
                    ON CONFLICT DO NOTHING
                    "#,
                    game_type_id,
                    user.id
                )
                .execute(&mut *tx)
                .await?;
            }
            PlayerSlotInternal::Bot { name, bot_name } => {
                let bot_id = sqlx::query_scalar!(
                    "INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3) RETURNING id",
                    game.id,
                    name,
                    bot_name
                )
                .fetch_one(&mut *tx)
                .await?;

                sqlx::query!(
                    r#"
                    INSERT INTO game_players (game_id, game_bot_id, position, color, has_accepted, is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place)
                    VALUES ($1, $2, $3, $4, true, $5, NOW(), NOW(), $6, true, $7)
                    "#,
                    game.id,
                    bot_id,
                    pos as i32,
                    color,
                    is_turn,
                    is_eliminated,
                    place
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    Ok(game)
}

/// Inserts a command's logs and their per-player targets inside the caller's
/// transaction.
///
/// Deliberately row-at-a-time: 1 + N + M sequential statements, where N is the
/// number of logs produced by a single game command (single digits in practice)
/// and M their targets. Reviewed as ws F41 and left alone - batching via
/// `UNNEST` would trade three compile-time-checked `query!` macros for
/// hand-verified offline metadata with no measured benefit. Revisit if a game
/// ever emits logs in the hundreds per command, or if this shows up in a real
/// profile.
#[cfg(feature = "ssr")]
pub async fn insert_game_logs_tx(
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    // Get player IDs by position
    let players = sqlx::query!(
        "SELECT id, position FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut pos_to_id = std::collections::HashMap::new();
    for p in players {
        pos_to_id.insert(p.position as usize, p.id);
    }

    for log in logs {
        let log_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO game_logs (id, game_id, body, is_public, logged_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            log_id,
            game_id,
            log.content,
            log.public,
            log.at
        )
        .execute(&mut *tx)
        .await?;

        for &pos in &log.to {
            if let Some(&player_id) = pos_to_id.get(&pos) {
                sqlx::query!(
                    "INSERT INTO game_log_targets (game_log_id, game_player_id) VALUES ($1, $2)",
                    log_id,
                    player_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn create_game_logs(
    pool: &PgPool,
    game_id: Uuid,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    insert_game_logs_tx(&mut tx, game_id, logs).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
pub async fn concede_game(
    pool: &PgPool,
    game_id: Uuid,
    conceding_player_id: Uuid,
    conceding_name: &str,
    expected_updated_at: time::PrimitiveDateTime,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    claim_unfinished_game_tx(&mut tx, game_id, expected_updated_at).await?;

    // DRM-03b1b2: under the game-row lock acquired above and while the game is
    // still unfinished, normalise old-pod departures so the sequence
    // allocation below continues past them.
    normalize_legacy_departures_tx(&mut tx, game_id).await?;

    let already_left: bool = sqlx::query_scalar(
        "SELECT left_at IS NOT NULL FROM game_players WHERE id = $1 AND game_id = $2",
    )
    .bind(conceding_player_id)
    .bind(game_id)
    .fetch_one(&mut *tx)
    .await?;
    if already_left {
        return Err(anyhow::anyhow!("Player has already left this game"));
    }

    // DRM-03b2a1: revalidate the approved two-active-human Concede threshold
    // under the game-row lock and after legacy normalization, before any
    // lifecycle write, so a stale shared precheck cannot slip a
    // sole-active-human concession through.
    let active_humans: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM game_players \
         WHERE game_id = $1 AND user_id IS NOT NULL AND left_at IS NULL",
    )
    .bind(game_id)
    .fetch_one(&mut *tx)
    .await?;
    if active_humans < 2 {
        return Err(NotEnoughActiveHumans.into());
    }

    let update_result = sqlx::query(
        "UPDATE games SET is_finished = true, finished_at = NOW(), \
         end_reason = 'concession_forfeit' \
         WHERE id = $1 AND updated_at = $2 AND is_finished = false",
    )
    .bind(game_id)
    .bind(expected_updated_at)
    .execute(&mut *tx)
    .await?;
    if update_result.rows_affected() == 0 {
        return Err(StaleStateConflict.into());
    }

    let players = sqlx::query!(
        "SELECT id FROM game_players WHERE game_id = $1 ORDER BY position",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    // Assigns place 1 to every non-conceding player and place 2 to the conceder.
    // Only correct for 2-player games; callers must enforce that constraint.
    if players.len() != 2 {
        return Err(anyhow::anyhow!(
            "concede_game requires exactly 2 players, found {}",
            players.len()
        ));
    }

    // DRM-03b1b2: one shared positive sequence for the concession departure.
    // Computed after normalisation and under the game-row lock acquired above,
    // so concurrent events cannot collide.
    let next_departure_sequence: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(departure_sequence), 0) + 1 FROM game_players WHERE game_id = $1",
    )
    .bind(game_id)
    .fetch_one(&mut *tx)
    .await?;

    for p in &players {
        let is_conceder = p.id == conceding_player_id;
        let place: i32 = if is_conceder { 2 } else { 1 };
        // DRM-03b1b2: only the conceder's seat receives departure metadata;
        // the winner keeps `left_at`, `departure_reason`, and
        // `departure_sequence` NULL.
        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = false, place = $1, undo_game_state = NULL,
                   turn_reminder_sent_at = NULL,
                   left_at = CASE WHEN $3 THEN NOW() ELSE left_at END,
                   departure_reason = CASE WHEN $3 THEN 'conceded' ELSE departure_reason END,
                   departure_sequence = CASE WHEN $3 THEN $4 ELSE departure_sequence END
               WHERE id = $2"#,
        )
        .bind(place)
        .bind(p.id)
        .bind(is_conceder)
        .bind(next_departure_sequence)
        .execute(&mut *tx)
        .await?;
    }

    let log_body = format!("{} conceded.", conceding_name);
    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        log_body
    )
    .execute(&mut *tx)
    .await?;

    write_ranked_placings(&mut tx, game_id).await?;
    apply_rating_changes(&mut tx, game_id).await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
pub async fn concede_game_replace(
    pool: &PgPool,
    game_id: Uuid,
    conceding_player_id: Uuid,
    conceding_name: &str,
    expected_updated_at: time::PrimitiveDateTime,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    claim_unfinished_game_tx(&mut tx, game_id, expected_updated_at).await?;

    // DRM-03b1a3: under the claim's game-row lock and while the game is still
    // unfinished, normalise legacy old-pod departures so the sequence
    // allocation below continues past them.
    normalize_legacy_departures_tx(&mut tx, game_id).await?;

    let already_left: bool = sqlx::query_scalar(
        "SELECT left_at IS NOT NULL FROM game_players WHERE id = $1 AND game_id = $2",
    )
    .bind(conceding_player_id)
    .bind(game_id)
    .fetch_one(&mut *tx)
    .await?;
    if already_left {
        return Err(anyhow::anyhow!("Player has already left this game"));
    }

    // DRM-03b2a1: revalidate the approved two-active-human Concede threshold
    // under the game-row lock and after legacy normalization, before bot
    // selection or any lifecycle write, so a stale shared precheck cannot
    // replace the last active human.
    let active_humans: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM game_players \
         WHERE game_id = $1 AND user_id IS NOT NULL AND left_at IS NULL",
    )
    .bind(game_id)
    .fetch_one(&mut *tx)
    .await?;
    if active_humans < 2 {
        return Err(NotEnoughActiveHumans.into());
    }

    let bot = pick_replacement_bot(&mut tx, game_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no replacement bot configured"))?;

    // DRM-03b1a3: one shared positive sequence for the concession departure.
    // Computed after normalisation and bot selection and under the game-row
    // lock, so concurrent events cannot collide.
    let next_departure_sequence: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(departure_sequence), 0) + 1 FROM game_players WHERE game_id = $1",
    )
    .bind(game_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"UPDATE game_players
           SET game_bot_id = $1, left_at = NOW(),
               departure_reason = 'conceded', departure_sequence = $2,
               undo_game_state = NULL, turn_reminder_sent_at = NULL
           WHERE id = $3"#,
    )
    .bind(bot.id)
    .bind(next_departure_sequence)
    .bind(conceding_player_id)
    .execute(&mut *tx)
    .await?;

    let log_body = format!(
        "{} conceded (replaced by bot {}).",
        conceding_name, bot.name
    );
    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        log_body
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE games SET updated_at = updated_at WHERE id = $1")
        .bind(game_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

type ActorRow = (
    Option<Uuid>,
    Option<time::PrimitiveDateTime>,
    Option<i32>,
    i64,
    Option<i32>,
);

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
pub async fn end_game(
    pool: &PgPool,
    game_id: Uuid,
    expected_updated_at: time::PrimitiveDateTime,
    acting_game_player_id: Uuid,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    claim_unfinished_game_tx(&mut tx, game_id, expected_updated_at).await?;

    // DRM-03b1c3: under the claim's game-row lock and while the game is still
    // unfinished, normalise legacy old-pod departures so the finishing report
    // ranks them as departed rather than active.
    normalize_legacy_departures_tx(&mut tx, game_id).await?;

    // DRM-03b1c4: authorize the stop under the same game-row lock and after
    // legacy normalization, using current human participant rows, before any
    // terminal write. Identity is `game_players.id`, never `users.id`. With
    // exactly one active human only that active human is authorized; with zero
    // active humans every human in the latest departure event is. Plain
    // (non-macro) query, not `query!`, because migration-032's
    // `departure_sequence` is not in the committed offline `.sqlx` cache.
    let actor: Option<ActorRow> = sqlx::query_as(
        r#"SELECT gp.user_id, gp.left_at, gp.departure_sequence,
                  (SELECT COUNT(*) FROM game_players
                   WHERE game_id = $1 AND user_id IS NOT NULL AND left_at IS NULL),
                  (SELECT MAX(departure_sequence) FROM game_players
                   WHERE game_id = $1 AND user_id IS NOT NULL)
           FROM game_players gp
           WHERE gp.id = $2 AND gp.game_id = $1"#,
    )
    .bind(game_id)
    .bind(acting_game_player_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((
        actor_user_id,
        actor_left_at,
        actor_departure_sequence,
        active_humans,
        max_departure_sequence,
    )) = actor
    else {
        return Err(anyhow::anyhow!("acting game player is not in this game"));
    };
    if actor_user_id.is_none() {
        return Err(anyhow::anyhow!("acting game player is a bot"));
    }
    if active_humans >= 2 {
        return Err(anyhow::anyhow!(
            "end game requires at most one active human"
        ));
    }
    if active_humans == 1 {
        if actor_left_at.is_some() {
            return Err(anyhow::anyhow!(
                "acting game player is not the last active human"
            ));
        }
    } else {
        let Some(latest_departure_sequence) = max_departure_sequence else {
            return Err(anyhow::anyhow!(
                "no usable departure metadata for this game"
            ));
        };
        if actor_departure_sequence != Some(latest_departure_sequence) {
            return Err(anyhow::anyhow!(
                "acting game player is not in the latest departure event"
            ));
        }
    }

    // Plain (non-macro) query, not `query!`, because migration-032's
    // `end_reason` is not in the committed offline `.sqlx` cache (same
    // convention as `update_game_command_success`).
    let update_result = sqlx::query(
        "UPDATE games SET is_finished = true, finished_at = NOW(), \
         end_reason = 'last_human_stop' \
         WHERE id = $1 AND updated_at = $2 AND is_finished = false",
    )
    .bind(game_id)
    .bind(expected_updated_at)
    .execute(&mut *tx)
    .await?;
    if update_result.rows_affected() == 0 {
        return Err(StaleStateConflict.into());
    }

    // DRM-03b1c3: a last-human stop has no authoritative game places; clear
    // every seat's place, turn, undo, and reminder state so no stale
    // point-sorted place survives. Competitive ranks come solely from
    // `write_ranked_placings` below.
    sqlx::query(
        "UPDATE game_players SET place = NULL, is_turn = false, \
         undo_game_state = NULL, turn_reminder_sent_at = NULL \
         WHERE game_id = $1",
    )
    .bind(game_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        "Game ended.".to_string()
    )
    .execute(&mut *tx)
    .await?;

    write_ranked_placings(&mut tx, game_id).await?;
    apply_rating_changes(&mut tx, game_id).await?;

    tx.commit().await?;
    Ok(())
}

/// #34 admin force delete (spec D3): hard-deletes a game and all dependent
/// rows in one transaction. Any game referencing the deleted one via
/// `restarted_game_id` has that link nulled (making it restartable again), and
/// any proposal referencing it via `started_game_id`/`restarted_game_id` has
/// that link nulled (preserving the proposal history). Ratings are deliberately
/// NOT rewound. Returns false if the game did not exist.
#[cfg(feature = "ssr")]
pub async fn delete_game(pool: &PgPool, game_id: Uuid) -> Result<bool> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET restarted_game_id = NULL WHERE restarted_game_id = $1",
        game_id
    )
    .execute(&mut *tx)
    .await?;
    // game_proposals (migration 015) FK-reference games via started_game_id and
    // restarted_game_id; null both or the game delete violates those FKs.
    // NOTE: game_proposals has NO update_updated_at trigger (see the module
    // header), so the manual `updated_at = NOW()` in the next two statements is
    // REQUIRED - do not sweep it away (ws F36).
    sqlx::query(
        "UPDATE game_proposals SET started_game_id = NULL, updated_at = NOW() WHERE started_game_id = $1",
    )
    .bind(game_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE game_proposals SET restarted_game_id = NULL, updated_at = NOW() WHERE restarted_game_id = $1",
    )
    .bind(game_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM game_log_targets WHERE game_log_id IN (SELECT id FROM game_logs WHERE game_id = $1)",
        game_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM game_logs WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    // game_players before game_bots: game_players.game_bot_id FK.
    sqlx::query!("DELETE FROM game_players WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM game_bots WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query!("DELETE FROM games WHERE id = $1", game_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(feature = "ssr")]
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(game_id = %game_id))]
pub async fn undo_game(
    pool: &PgPool,
    game_id: Uuid,
    undo_state: &str,
    player_position: usize,
    status: &StatusUpdate,
    points: &[f32],
    game_player_id: Uuid,
    expected_updated_at: time::PrimitiveDateTime,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    claim_unfinished_game_tx(&mut tx, game_id, expected_updated_at).await?;

    let undo_check: Option<Option<String>> = sqlx::query_scalar(
        "SELECT undo_game_state FROM game_players WHERE id = $1 AND game_id = $2",
    )
    .bind(game_player_id)
    .bind(game_id)
    .fetch_optional(&mut *tx)
    .await?;
    match undo_check {
        Some(Some(_)) => {}
        _ => return Err(StaleStateConflict.into()),
    }

    let update_result = sqlx::query!(
        "UPDATE games SET game_state = $1, is_finished = $2, finished_at = NULL WHERE id = $3 AND updated_at = $4 AND is_finished = false",
        undo_state,
        status.is_finished,
        game_id,
        expected_updated_at
    )
    .execute(&mut *tx)
    .await?;
    if update_result.rows_affected() == 0 {
        return Err(StaleStateConflict.into());
    }

    let players = sqlx::query!(
        "SELECT id, position FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    for p in players {
        let pos = p.position as usize;
        let is_turn = status.whose_turn.contains(&pos);
        let is_eliminated = status.eliminated.contains(&pos);
        let place: Option<i32> = status.placings.get(pos).map(|&pl| pl as i32);
        let player_points = points.get(pos).copied();

        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = $1, is_eliminated = $2, place = $3, points = $4,
                   undo_game_state = NULL,
                   turn_reminder_sent_at = NULL,
                   left_at = CASE WHEN is_eliminated = false AND $2 = true
                                  THEN NOW()
                                  WHEN is_eliminated = true AND $2 = false
                                  THEN NULL
                                  ELSE left_at END,
                   departure_reason = CASE WHEN is_eliminated = true AND $2 = false
                                           THEN NULL ELSE departure_reason END,
                   departure_sequence = CASE WHEN is_eliminated = true AND $2 = false
                                             THEN NULL ELSE departure_sequence END
               WHERE id = $5"#,
        )
        .bind(is_turn)
        .bind(is_eliminated)
        .bind(place)
        .bind(player_points)
        .bind(p.id)
        .execute(&mut *tx)
        .await?;
    }

    let undo_log_body = format!("{{{{player {}}}}} used an undo", player_position);
    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        undo_log_body,
    )
    .execute(&mut *tx)
    .await?;

    // Rating fields (`game_players.rating_change`/`rating_before`,
    // `game_type_users.rating`) are deliberately NOT touched here: a finished
    // game can no longer be undone (see the claim above), so no rated game ever
    // reaches this function. Rewinding ratings is out of scope by decision
    // (review 2026-07-23, D-3 option A); if undo-after-finish is ever allowed
    // again, the rewind must land in the SAME transaction as this revert.
    tx.commit().await?;
    Ok(())
}

/// Distinguishable error so callers (the `bot.command` consumer) can tell a
/// stale-state conflict apart from other failures and react by re-publishing
/// `bot.turn` rather than giving up.
#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
#[error("Game was updated by another action, please retry")]
pub struct StaleStateConflict;

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
#[error("Game is already finished")]
pub struct GameAlreadyFinished;

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
#[error("Concede is not available: at least two active humans are required")]
pub struct NotEnoughActiveHumans;

#[cfg(feature = "ssr")]
async fn claim_unfinished_game_tx(
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
    expected_updated_at: time::PrimitiveDateTime,
) -> Result<()> {
    let row: Option<(bool, time::PrimitiveDateTime)> =
        sqlx::query_as("SELECT is_finished, updated_at FROM games WHERE id = $1 FOR UPDATE")
            .bind(game_id)
            .fetch_optional(&mut *tx)
            .await?;
    let (is_finished, updated_at) = row.ok_or_else(|| anyhow::anyhow!("Game not found"))?;
    if is_finished {
        return Err(GameAlreadyFinished.into());
    }
    if updated_at != expected_updated_at {
        return Err(StaleStateConflict.into());
    }
    Ok(())
}

/// DRM-03a: old pods write `left_at` without departure metadata during the
/// rollout. Under the authoritative `games` row lock and while the game is
/// still unfinished, stamp such human rows `unknown_legacy` with a per-game
/// dense sequence over `left_at` (equal timestamps tie), offset past any
/// already-assigned sequences so no existing departure is overwritten or
/// collided with. Completed games are never touched. Deterministic and
/// idempotent. Callers must hold the game-row lock before invoking so a
/// finishing report normalises departed old-pod humans as departed before
/// ranking. Shared so the DRM-03b concession/end writers can normalise the
/// same rows under the same lock before they allocate.
#[cfg(feature = "ssr")]
pub(crate) async fn normalize_legacy_departures_tx(
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE game_players gp
        SET departure_reason = 'unknown_legacy',
            departure_sequence = nd.departure_sequence
        FROM (
            SELECT
                id,
                dense_rank() OVER (ORDER BY left_at) + (
                    SELECT COALESCE(MAX(departure_sequence), 0)
                    FROM game_players
                    WHERE game_id = $1
                ) AS departure_sequence
            FROM game_players
            WHERE game_id = $1
              AND user_id IS NOT NULL
              AND left_at IS NOT NULL
              AND departure_sequence IS NULL
        ) nd
        JOIN games g ON g.id = gp.game_id
        WHERE gp.id = nd.id
          AND NOT g.is_finished
        "#,
    )
    .bind(game_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(game_id = %game_id))]
pub async fn update_game_command_success(
    pool: &PgPool,
    game_id: Uuid,
    played_player_id: Uuid,
    prev_game_state: &str,
    new_game_state: &str,
    can_undo: bool,
    status: &StatusUpdate,
    points: &[f32],
    expected_updated_at: time::PrimitiveDateTime,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    let now = {
        let t = time::OffsetDateTime::now_utc();
        time::PrimitiveDateTime::new(t.date(), t.time())
    };
    let finished_at: Option<time::PrimitiveDateTime> =
        if status.is_finished { Some(now) } else { None };

    let mut tx = pool.begin().await?;

    // DRM-03a: acquire the authoritative `games` row lock and validate the
    // optimistic `expected_updated_at` guard BEFORE any normalisation,
    // sequence allocation, or player/game lifecycle write. A row not found
    // here means a stale (or absent) game, matching the legacy 0-row UPDATE
    // behaviour: `StaleStateConflict`, never an unrelated error.
    let locked: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM games WHERE id = $1 AND updated_at = $2 FOR UPDATE")
            .bind(game_id)
            .bind(expected_updated_at)
            .fetch_optional(&mut *tx)
            .await?;
    if locked.is_none() {
        return Err(StaleStateConflict.into());
    }

    // DRM-03a: under the lock above and while the game is still marked
    // unfinished, normalise old-pod departures, so a finishing report ranks
    // them as departed rather than active, and the sequence allocation below
    // continues past them.
    normalize_legacy_departures_tx(&mut tx, game_id).await?;

    // `is_finished` is sticky: a finished game stays finished, matching
    // `COALESCE($3, finished_at)` on the timestamp column. Un-finishing is
    // `undo_game`'s job (it writes is_finished AND finished_at = NULL
    // together); allowing a stray non-finish command to flip the flag here
    // produced `is_finished = false` with a non-NULL `finished_at` (ws F37).
    // `updated_at` is maintained by the update_games_updated_at trigger, so
    // the optimistic-concurrency guard below still sees a changed value.
    // Plain (non-macro) query, not `query!`, because migration-032's
    // `end_reason` is not in the committed offline `.sqlx` cache. A normal
    // service finish records `end_reason = 'game_service'` (DRM-03a); an
    // existing reason is preserved so a retry cannot clobber it.
    let update_result = sqlx::query(
        "UPDATE games SET game_state = $1, is_finished = ($2 OR is_finished), finished_at = COALESCE($3, finished_at), \
         end_reason = CASE WHEN $2 THEN COALESCE(end_reason, 'game_service') ELSE end_reason END \
         WHERE id = $4 AND updated_at = $5",
    )
    .bind(new_game_state)
    .bind(status.is_finished)
    .bind(finished_at)
    .bind(game_id)
    .bind(expected_updated_at)
    .execute(&mut *tx)
    .await?;

    if update_result.rows_affected() == 0 {
        return Err(StaleStateConflict.into());
    }

    // DRM-03a: one shared positive sequence for every human eliminated in
    // this report. Computed after normalisation and under the game-row lock
    // acquired above, so concurrent events cannot collide.
    let next_departure_sequence: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(departure_sequence), 0) + 1 FROM game_players WHERE game_id = $1",
    )
    .bind(game_id)
    .fetch_one(&mut *tx)
    .await?;

    // Plain (non-macro) query, not `query!`; see the `get_user_theme` doc
    // comment above for the same convention.
    let players: Vec<(Uuid, i32, time::PrimitiveDateTime, time::PrimitiveDateTime, Option<String>)> =
        sqlx::query_as(
            "SELECT id, position, is_turn_at, last_turn_at, undo_game_state FROM game_players WHERE game_id = $1",
        )
        .bind(game_id)
        .fetch_all(&mut *tx)
        .await?;

    for (p_id, p_position, p_is_turn_at, p_last_turn_at, p_undo_game_state) in players {
        let pos = p_position as usize;
        let is_turn = status.whose_turn.contains(&pos);
        let place = status.placings.get(pos).map(|&pl| pl as i32);
        let is_eliminated = status.eliminated.contains(&pos);
        let player_points = points.get(pos).copied();
        // `is_turn_at` is LAST TURN ACTIVITY, not "turn started": it is
        // re-stamped on every command by a player who is still on turn, in the
        // same statement that clears `turn_reminder_sent_at` below. That pairing
        // is deliberate - the turn-reminder sweep gates on
        // `turn_reminder_sent_at IS NULL AND is_turn_at < NOW() - threshold`
        // (email/sweep.rs:64-65), so a player mid-multi-action-turn who just
        // acted is not nagged. `find_active_turn_games` orders the switch digest
        // by the same field, i.e. least-recently-active first. The
        // `update_is_turn_at` trigger (migrations/001:454-458) only covers the
        // false -> true transition and is not a substitute for this write
        // (ws F44).
        let is_turn_at = if is_turn { now } else { p_is_turn_at };
        let is_played = p_id == played_player_id;
        let last_turn_at = p_last_turn_at;
        let undo_game_state: Option<&str> = if is_played && can_undo {
            p_undo_game_state.as_deref().or(Some(prev_game_state))
        } else {
            None
        };

        // DRM-03a: only a human seat with no prior departure (`left_at`
        // still NULL) that this active report newly eliminates gets departure
        // metadata; the shared `next_departure_sequence` is bound for every
        // row, but the CASE only fires on that transition, so repeated
        // reports retain existing metadata and pure bots / already-departed
        // seats (e.g. conceded-replaced humans) stay bare.
        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = $1, place = $2,
                   is_eliminated = CASE WHEN $9 THEN is_eliminated ELSE $3 END,
                   points = $4,
                   undo_game_state = $5, last_turn_at = $6, is_turn_at = $7,
                   turn_reminder_sent_at = NULL,
                   left_at = CASE WHEN is_eliminated = false AND $3 = true AND NOT $9
                                  THEN NOW() ELSE left_at END,
                   departure_reason = CASE WHEN user_id IS NOT NULL AND left_at IS NULL
                                               AND is_eliminated = false AND $3 = true AND NOT $9
                                               AND departure_reason IS NULL
                                           THEN 'eliminated' ELSE departure_reason END,
                   departure_sequence = CASE WHEN user_id IS NOT NULL AND left_at IS NULL
                                                 AND is_eliminated = false AND $3 = true AND NOT $9
                                                 AND departure_sequence IS NULL
                                             THEN $10 ELSE departure_sequence END
               WHERE id = $8"#,
        )
        .bind(is_turn)
        .bind(place)
        .bind(is_eliminated)
        .bind(player_points)
        .bind(undo_game_state)
        .bind(last_turn_at)
        .bind(is_turn_at)
        .bind(p_id)
        .bind(status.is_finished)
        .bind(next_departure_sequence)
        .execute(&mut *tx)
        .await?;
    }

    if status.is_finished && !status.placings.is_empty() {
        write_ranked_placings(&mut tx, game_id).await?;
        apply_rating_changes(&mut tx, game_id).await?;
    }

    insert_game_logs_tx(&mut tx, game_id, logs).await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use crate::game::StatusUpdate;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    type ConcedeRow = (
        Option<Uuid>,
        Option<Uuid>,
        Option<time::PrimitiveDateTime>,
        Option<String>,
        Option<i32>,
    );

    #[sqlx::test]
    async fn create_game_with_users_assigns_positions_and_colors(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;

        let game = make_game_with_players(
            &pool,
            game_version_id,
            creator.id,
            &[opponent.id],
            1, // one bot
            &[0],
        )
        .await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge.game_players.len(), 3);

        // Positions are sequential 0..n and colors assigned in the same order.
        let expected_colors = ["Green", "Red", "Blue"];
        for (i, p) in ge.game_players.iter().enumerate() {
            assert_eq!(p.game_player.position, i as i32);
            assert_eq!(p.game_player.color, expected_colors[i]);
        }

        // Creator + opponent rows exist as users; exactly one bot slot.
        let human_ids: Vec<Uuid> = ge
            .game_players
            .iter()
            .filter_map(|p| p.user.as_ref().map(|u| u.id))
            .collect();
        assert!(human_ids.contains(&creator.id));
        assert!(human_ids.contains(&opponent.id));

        let bot_players: Vec<_> = ge
            .game_players
            .iter()
            .filter(|p| p.game_bot.is_some())
            .collect();
        assert_eq!(bot_players.len(), 1);
        let bot_player = bot_players[0];
        assert!(bot_player.game_player.user_id.is_none());
        assert!(bot_player.game_bot.is_some());

        // XOR constraint holds for every player row (checked at DB level too).
        for p in &ge.game_players {
            assert!(p.game_player.user_id.is_some() != p.game_bot.is_some());
        }

        // Underlying game_bots row has game_bot_id set and user_id NULL directly
        // via raw query (belt-and-braces check of the XOR constraint columns).
        let raw = sqlx::query!(
            "SELECT user_id, game_bot_id FROM game_players WHERE game_id = $1 AND user_id IS NULL",
            game.id
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(raw.len(), 1);
        assert!(raw[0].game_bot_id.is_some());

        // Initial is_turn matches whose_turn = [0].
        assert!(ge.game_players[0].game_player.is_turn);
        assert!(!ge.game_players[1].game_player.is_turn);
        assert!(!ge.game_players[2].game_player.is_turn);
    }

    #[sqlx::test]
    async fn update_game_command_success_writes_active_fields(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        let ge_before = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player = ge_before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let played_player_id = played_player.game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "new_state",
            true, // can_undo
            &StatusUpdate {
                is_finished: false,  // -> Active
                whose_turn: vec![1], // whose_turn moves to position 1
                eliminated: vec![0], // position 0 is eliminated
                placings: vec![],
            },
            &[3.5, 1.5],
            ge_before.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.game_state, "new_state");
        assert!(!ge_after.game.is_finished);

        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();

        assert!(!p0.game_player.is_turn);
        assert!(p1.game_player.is_turn);
        // eliminated = [0] must land on position 0's is_eliminated flag only,
        // and must not bleed into place (same-typed placings slice).
        assert!(p0.game_player.is_eliminated);
        assert!(!p1.game_player.is_eliminated);
        assert_eq!(p0.game_player.place, None);
        assert_eq!(p0.game_player.points, Some(3.5));
        assert_eq!(p1.game_player.points, Some(1.5));
        // Only the played player gets undo state stashed.
        assert_eq!(
            p0.game_player.undo_game_state,
            Some("prev_state".to_string())
        );
        assert_eq!(p1.game_player.undo_game_state, None);
        // last_turn_at bumped by the DB trigger on the is_turn true->false
        // transition (p0 leaves turn here), not by the played-player override.
        assert!(p0.game_player.last_turn_at > played_player.game_player.last_turn_at);
        // is_turn_at bumped for whoever's turn it now is (p1).
        assert!(p1.game_player.is_turn_at >= played_player.game_player.is_turn_at);
    }

    #[sqlx::test]
    async fn update_game_command_success_mid_turn_keeps_last_turn_at(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        let ge_before = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player = ge_before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let played_player_id = played_player.game_player.id;
        let last_turn_at_before = played_player.game_player.last_turn_at;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "new_state",
            true, // can_undo
            &StatusUpdate {
                is_finished: false,  // -> Active
                whose_turn: vec![0], // position 0 stays in turn (mid-turn command)
                eliminated: vec![],
                placings: vec![],
            },
            &[3.5, 1.5],
            ge_before.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();

        // No is_turn true->false transition occurred for p0, so the DB
        // trigger does not fire and last_turn_at must be unchanged.
        assert!(p0.game_player.is_turn);
        assert_eq!(p0.game_player.last_turn_at, last_turn_at_before);
    }

    #[sqlx::test]
    async fn update_game_command_success_writes_finished_fields(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
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
                is_finished: true, // -> Finished
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2], // placings by position
            },
            &[10.0, 5.0],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(ge_after.game.is_finished);
        let first_finished_at = ge_after.game.finished_at.expect("finished_at set");

        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert_eq!(p0.game_player.place, Some(1));
        assert_eq!(p1.game_player.place, Some(2));

        // Second command carries is_finished = false. Finish is sticky in both
        // columns: `is_finished` stays true (`($2 OR is_finished)`) and
        // `finished_at` is preserved by the COALESCE. When is_finished = true
        // the call passes Some(now), so a genuine second finish DOES advance
        // finished_at - only `undo_game` un-finishes a game (ws F37).
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "final_state",
            "final_state_2",
            false,
            // is_finished = false -> finished_at param is None
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[10.0, 5.0],
            ge_after.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(
            ge_after_2.game.finished_at,
            Some(first_finished_at),
            "COALESCE preserves finished_at when the new value is NULL"
        );
        assert!(
            ge_after_2.game.is_finished,
            "is_finished must stay true once set; a non-finish command must not \
             produce is_finished = false with a non-NULL finished_at (ws F37)"
        );
    }

    #[sqlx::test]
    async fn update_game_command_success_keeps_first_undo_stash_in_a_run(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;

        // First can_undo=true command by player 0.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_1 = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // Second can_undo=true command by the same player.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_1",
            "state_2",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge_after_1.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert_eq!(
            p0.game_player.undo_game_state,
            Some("state_0".to_string()),
            "the run's undo stash must stay pinned to the first command's prev_game_state"
        );
    }

    #[sqlx::test]
    async fn update_game_command_success_clears_stash_on_non_undoable_command(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;

        // can_undo=true stashes state_0.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_1 = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // Same player plays a can_undo=false command; the stash must clear.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_1",
            "state_2",
            false,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge_after_1.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert_eq!(p0.game_player.undo_game_state, None);
    }

    #[sqlx::test]
    async fn update_game_command_success_clears_stash_when_opponent_plays(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap()
            .game_player
            .id;
        let p1_id = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap()
            .game_player
            .id;

        // Player 0 plays a can_undo=true command, stashing state_0.
        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_1 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_after_1 = ge_after_1
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert_eq!(
            p0_after_1.game_player.undo_game_state,
            Some("state_0".to_string())
        );

        // Opponent (player 1) plays next; player 0's stash must clear since
        // player 0 is not the played player on this command.
        update_game_command_success(
            &pool,
            game.id,
            p1_id,
            "state_1",
            "state_2",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge_after_1.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_after_2 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1_after_2 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert_eq!(
            p0_after_2.game_player.undo_game_state, None,
            "opponent's command must clear player 0's stash"
        );
        assert_eq!(
            p1_after_2.game_player.undo_game_state,
            Some("state_1".to_string())
        );
    }

    #[sqlx::test]
    async fn elimination_sets_left_at_once(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opp.id], 0, &[0]).await;

        let player_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = 1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let left_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(player_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(left_at.is_none());

        let status = StatusUpdate {
            is_finished: false,
            whose_turn: vec![0],
            eliminated: vec![1],
            placings: vec![],
        };
        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            player_id,
            "",
            "",
            false,
            &status,
            &[],
            updated_at,
            vec![],
        )
        .await
        .unwrap();

        let left_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(player_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(left_at.is_some());
        let first_left_at = left_at.unwrap();

        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            player_id,
            "",
            "",
            false,
            &status,
            &[],
            updated_at,
            vec![],
        )
        .await
        .unwrap();
        let left_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(player_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(left_at, first_left_at);
    }

    #[sqlx::test]
    async fn concede_game_replace_swaps_in_bot(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let b = make_user(&pool, "b").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[a.id, b.id], 0, &[0])
                .await;

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let a_pos = position_of(&ge, a.id);
        let conceder: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(a_pos)
                .fetch_one(&pool)
                .await
                .unwrap();

        concede_game_replace(&pool, game.id, conceder, "a", ge.game.updated_at)
            .await
            .unwrap();

        let row: ConcedeRow = sqlx::query_as(
            "SELECT user_id, game_bot_id, left_at, departure_reason, departure_sequence \
             FROM game_players WHERE id = $1",
        )
        .bind(conceder)
        .fetch_one(&pool)
        .await
        .unwrap();
        let replacement_bot_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_bots WHERE game_id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, Some(a.id), "user_id preserved");
        assert_eq!(
            row.1,
            Some(replacement_bot_id),
            "game_bot_id is selected replacement bot"
        );
        assert!(row.2.is_some(), "left_at set");
        assert_eq!(row.3.as_deref(), Some("conceded"), "departure_reason");
        assert_eq!(row.4, Some(1), "departure_sequence");

        let finished: bool = sqlx::query_scalar("SELECT is_finished FROM games WHERE id = $1")
            .bind(game.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(!finished, "game must not be finished");

        let unaffected: Vec<(bool, bool, bool)> = sqlx::query_as(
            "SELECT left_at IS NOT NULL, departure_reason IS NOT NULL, departure_sequence IS NOT NULL \
             FROM game_players WHERE game_id = $1 AND id <> $2",
        )
        .bind(game.id)
        .bind(conceder)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!unaffected.is_empty(), "other players must exist");
        for (left, reason, seq) in unaffected {
            assert!(!left, "unaffected player has no left_at");
            assert!(!reason, "unaffected player has no departure_reason");
            assert!(!seq, "unaffected player has no departure_sequence");
        }
    }

    #[sqlx::test]
    async fn end_game_finishes_and_ranks(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[a.id], 1, &[0]).await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let a_pos = position_of(&ge, a.id);
        let bot_pos = ge
            .game_players
            .iter()
            .find(|p| p.game_player.user_id.is_none())
            .unwrap()
            .game_player
            .position;

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();
        let conceder: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(a_pos)
                .fetch_one(&pool)
                .await
                .unwrap();
        concede_game_replace(&pool, game.id, conceder, "a", ge.game.updated_at)
            .await
            .unwrap();

        sqlx::query("UPDATE game_players SET points = 10 WHERE game_id = $1 AND position = $2")
            .bind(game.id)
            .bind(creator_pos)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE game_players SET points = 5 WHERE game_id = $1 AND position = $2")
            .bind(game.id)
            .bind(bot_pos)
            .execute(&pool)
            .await
            .unwrap();

        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let creator_game_player: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(creator.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        end_game(&pool, game.id, updated_at, creator_game_player)
            .await
            .unwrap();

        let finished: bool = sqlx::query_scalar("SELECT is_finished FROM games WHERE id = $1")
            .bind(game.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(finished);

        let survivor_ranked: Option<i32> = sqlx::query_scalar(
            "SELECT ranked_placing FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(survivor_ranked, Some(1));

        // DRM-03b1c3: a last-human stop records the stop reason, leaves every
        // seat without an authoritative place or turn/undo/reminder state, and
        // still ranks the retained and replaced humans competitively while the
        // pure bot gets no placement.
        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.end_reason.as_deref(), Some("last_human_stop"));
        assert!(
            ge_after.game.finished_at.is_some(),
            "a last-human stop must record finished_at"
        );

        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.place, None,
                "no authoritative place on a last-human stop"
            );
            assert!(!p.game_player.is_turn, "no seat keeps its turn");
            assert_eq!(p.game_player.undo_game_state, None, "undo state cleared");
        }
        let reminded: Vec<i32> = sqlx::query_scalar(
            "SELECT position FROM game_players WHERE game_id = $1 AND turn_reminder_sent_at IS NOT NULL",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(reminded.is_empty(), "no turn reminder survives the stop");

        let by_pos = |pos: i32| {
            ge_after
                .game_players
                .iter()
                .find(|p| p.game_player.position == pos)
                .unwrap()
                .game_player
                .clone()
        };
        assert_eq!(
            by_pos(creator_pos).ranked_placing,
            Some(1),
            "retained human takes the top competitive placement"
        );
        assert_eq!(
            by_pos(a_pos).ranked_placing,
            Some(2),
            "replaced human follows in reverse departure order"
        );
        assert_eq!(
            by_pos(bot_pos).ranked_placing,
            None,
            "pure bot has no competitive placement"
        );
    }

    /// DRM-03b1c2: with exactly one active human, that active human is the
    /// authorized stopper.
    #[sqlx::test]
    async fn end_game_authorizes_sole_active_human(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id], 0, &[0]).await;

        sqlx::query(
            "UPDATE game_players SET left_at = NOW(), departure_reason = 'conceded', \
             departure_sequence = 1 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge.game.is_finished, "fixture must start unfinished");
        assert_eq!(
            ge.game_players
                .iter()
                .filter(|p| p.game_player.user_id.is_some() && p.game_player.left_at.is_none())
                .count(),
            1,
            "fixture must leave exactly one active human"
        );
        let actor: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(creator.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        end_game(&pool, game.id, ge.game.updated_at, actor)
            .await
            .unwrap();

        let after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(
            after.game.is_finished,
            "sole active human must be able to stop"
        );
        assert_eq!(after.game.end_reason.as_deref(), Some("last_human_stop"));
    }

    /// DRM-03b1c2: with exactly one active human, a departed human is rejected
    /// and the rejected call mutates nothing.
    #[sqlx::test]
    async fn end_game_rejects_departed_human_with_one_active_human(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // Seed authoritative places and ratings so the no-mutation assertions
        // prove the rejected call writes nothing.
        for p in &ge.game_players {
            sqlx::query("UPDATE game_players SET place = $1 WHERE id = $2")
                .bind(p.game_player.position + 5)
                .bind(p.game_player.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "UPDATE game_type_users SET rating = 1300, peak_rating = 1400 \
             WHERE user_id IN (SELECT user_id FROM game_players \
             WHERE game_id = $1 AND user_id IS NOT NULL)",
        )
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "UPDATE game_players SET left_at = NOW(), departure_reason = 'conceded', \
             departure_sequence = 1 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .execute(&pool)
        .await
        .unwrap();

        let actor: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(a.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let result = end_game(&pool, game.id, ge.game.updated_at, actor).await;
        assert!(result.is_err(), "departed human must be rejected");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("not the last active human"),
            "unexpected error: {err}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge_after.game.is_finished, "game must stay unfinished");
        assert!(
            ge_after.game.finished_at.is_none(),
            "finished_at must stay unset"
        );
        assert_eq!(ge_after.game.end_reason, None, "end_reason must stay unset");
        assert_eq!(ge_after.game.game_state, "initial_state");
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.place,
                Some(p.game_player.position + 5),
                "authoritative place must stay exactly as seeded"
            );
            assert_eq!(p.game_player.ranked_placing, None, "no ranked placing");
            assert_eq!(p.game_player.rating_change, None, "no rating stamp");
        }
        let ratings: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT gtu.rating, gtu.peak_rating FROM game_type_users gtu \
             JOIN game_players gp ON gp.user_id = gtu.user_id \
             WHERE gp.game_id = $1",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!ratings.is_empty(), "seeded ratings must exist");
        assert!(
            ratings
                .iter()
                .all(|&(rating, peak)| (rating, peak) == (1300, 1400)),
            "game_type_user rating must stay exactly as seeded: {ratings:?}"
        );
        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(
            !logs.iter().any(|l| l.body == "Game ended."),
            "a rejected call must not write the end log"
        );
    }

    /// DRM-03b1c2: with zero active humans, a human in the latest departure
    /// event is the authorized stopper.
    #[sqlx::test]
    async fn end_game_authorizes_latest_departed_human_when_zero_active(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id], 0, &[0]).await;

        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-01 00:00:00', \
             departure_reason = 'conceded', departure_sequence = 1 \
             WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-02 00:00:00', \
             departure_reason = 'conceded', departure_sequence = 2 \
             WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(creator.id)
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge.game.is_finished, "fixture must start unfinished");
        assert_eq!(
            ge.game_players
                .iter()
                .filter(|p| p.game_player.user_id.is_some() && p.game_player.left_at.is_none())
                .count(),
            0,
            "fixture must leave zero active humans"
        );
        let actor: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(creator.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        end_game(&pool, game.id, ge.game.updated_at, actor)
            .await
            .unwrap();

        let after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(
            after.game.is_finished,
            "latest departure event human must be able to stop"
        );
        assert_eq!(after.game.end_reason.as_deref(), Some("last_human_stop"));
    }

    /// DRM-03b1c2: with zero active humans and a tie in the latest departure
    /// event, every tied participant is authorizable. Each success runs on its
    /// own unfinished game.
    #[sqlx::test]
    async fn end_game_authorizes_each_tied_latest_departed_human(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, gv) = make_game_type_and_version(&pool).await;

        let pool_ref = &pool;
        let depart_all_together = |game_id: Uuid| async move {
            sqlx::query(
                "UPDATE game_players SET left_at = '2026-01-01 00:00:00', \
                 departure_reason = 'conceded', departure_sequence = 1 \
                 WHERE game_id = $1 AND user_id IS NOT NULL",
            )
            .bind(game_id)
            .execute(pool_ref)
            .await
            .unwrap();
        };
        let player_id = |game_id: Uuid, user_id: Uuid| async move {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2",
            )
            .bind(game_id)
            .bind(user_id)
            .fetch_one(pool_ref)
            .await
            .unwrap()
        };

        for actor in [creator.id, a.id] {
            let game = make_game_with_players(&pool, gv, creator.id, &[a.id], 0, &[0]).await;
            depart_all_together(game.id).await;
            let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
            assert!(!ge.game.is_finished, "each success must start unfinished");
            end_game(
                &pool,
                game.id,
                ge.game.updated_at,
                player_id(game.id, actor).await,
            )
            .await
            .unwrap();
            let after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
            assert!(
                after.game.is_finished,
                "tied participant {actor} must be authorizable"
            );
            assert_eq!(after.game.end_reason.as_deref(), Some("last_human_stop"));
        }
    }

    /// DRM-03b1c2: with zero active humans, a human from an earlier departure
    /// event is rejected and the rejected call mutates nothing.
    #[sqlx::test]
    async fn end_game_rejects_earlier_departure_event_human(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        for p in &ge.game_players {
            sqlx::query("UPDATE game_players SET place = $1 WHERE id = $2")
                .bind(p.game_player.position + 5)
                .bind(p.game_player.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "UPDATE game_type_users SET rating = 1300, peak_rating = 1400 \
             WHERE user_id IN (SELECT user_id FROM game_players \
             WHERE game_id = $1 AND user_id IS NOT NULL)",
        )
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-01 00:00:00', \
             departure_reason = 'conceded', departure_sequence = 1 \
             WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-02 00:00:00', \
             departure_reason = 'conceded', departure_sequence = 2 \
             WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(creator.id)
        .execute(&pool)
        .await
        .unwrap();

        let actor: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(a.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let result = end_game(&pool, game.id, ge.game.updated_at, actor).await;
        assert!(
            result.is_err(),
            "earlier departure event human must be rejected"
        );
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("latest departure event"),
            "unexpected error: {err}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge_after.game.is_finished, "game must stay unfinished");
        assert!(
            ge_after.game.finished_at.is_none(),
            "finished_at must stay unset"
        );
        assert_eq!(ge_after.game.end_reason, None, "end_reason must stay unset");
        assert_eq!(ge_after.game.game_state, "initial_state");
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.place,
                Some(p.game_player.position + 5),
                "authoritative place must stay exactly as seeded"
            );
            assert_eq!(p.game_player.ranked_placing, None, "no ranked placing");
            assert_eq!(p.game_player.rating_change, None, "no rating stamp");
        }
        let ratings: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT gtu.rating, gtu.peak_rating FROM game_type_users gtu \
             JOIN game_players gp ON gp.user_id = gtu.user_id \
             WHERE gp.game_id = $1",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!ratings.is_empty(), "seeded ratings must exist");
        assert!(
            ratings
                .iter()
                .all(|&(rating, peak)| (rating, peak) == (1300, 1400)),
            "game_type_user rating must stay exactly as seeded: {ratings:?}"
        );
        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(
            !logs.iter().any(|l| l.body == "Game ended."),
            "a rejected call must not write the end log"
        );
    }

    /// DRM-03b1c2: a pure-bot/non-human actor is rejected and the rejected
    /// call mutates nothing.
    #[sqlx::test]
    async fn end_game_rejects_pure_bot_actor(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[], 1, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let bot_actor = ge
            .game_players
            .iter()
            .find(|p| p.game_player.user_id.is_none())
            .unwrap()
            .game_player
            .id;

        for p in &ge.game_players {
            sqlx::query("UPDATE game_players SET place = $1 WHERE id = $2")
                .bind(p.game_player.position + 5)
                .bind(p.game_player.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "UPDATE game_type_users SET rating = 1300, peak_rating = 1400 \
             WHERE user_id IN (SELECT user_id FROM game_players \
             WHERE game_id = $1 AND user_id IS NOT NULL)",
        )
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

        let result = end_game(&pool, game.id, ge.game.updated_at, bot_actor).await;
        assert!(result.is_err(), "pure bot must be rejected");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("a bot"), "unexpected error: {err}");

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge_after.game.is_finished, "game must stay unfinished");
        assert!(
            ge_after.game.finished_at.is_none(),
            "finished_at must stay unset"
        );
        assert_eq!(ge_after.game.end_reason, None, "end_reason must stay unset");
        assert_eq!(ge_after.game.game_state, "initial_state");
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.place,
                Some(p.game_player.position + 5),
                "authoritative place must stay exactly as seeded"
            );
            assert_eq!(p.game_player.ranked_placing, None, "no ranked placing");
            assert_eq!(p.game_player.rating_change, None, "no rating stamp");
        }
        let ratings: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT gtu.rating, gtu.peak_rating FROM game_type_users gtu \
             JOIN game_players gp ON gp.user_id = gtu.user_id \
             WHERE gp.game_id = $1",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!ratings.is_empty(), "seeded ratings must exist");
        assert!(
            ratings
                .iter()
                .all(|&(rating, peak)| (rating, peak) == (1300, 1400)),
            "game_type_user rating must stay exactly as seeded: {ratings:?}"
        );
        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(
            !logs.iter().any(|l| l.body == "Game ended."),
            "a rejected call must not write the end log"
        );
    }

    /// DRM-03b1c2: an attempt with two or more active humans is rejected and
    /// the rejected call mutates nothing.
    #[sqlx::test]
    async fn end_game_rejects_two_or_more_active_humans(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let b = make_user(&pool, "b").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id, b.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        for p in &ge.game_players {
            sqlx::query("UPDATE game_players SET place = $1 WHERE id = $2")
                .bind(p.game_player.position + 5)
                .bind(p.game_player.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "UPDATE game_type_users SET rating = 1300, peak_rating = 1400 \
             WHERE user_id IN (SELECT user_id FROM game_players \
             WHERE game_id = $1 AND user_id IS NOT NULL)",
        )
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

        let actor: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(creator.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let result = end_game(&pool, game.id, ge.game.updated_at, actor).await;
        assert!(
            result.is_err(),
            "two or more active humans must be rejected"
        );
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("at most one active human"),
            "unexpected error: {err}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge_after.game.is_finished, "game must stay unfinished");
        assert!(
            ge_after.game.finished_at.is_none(),
            "finished_at must stay unset"
        );
        assert_eq!(ge_after.game.end_reason, None, "end_reason must stay unset");
        assert_eq!(ge_after.game.game_state, "initial_state");
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.place,
                Some(p.game_player.position + 5),
                "authoritative place must stay exactly as seeded"
            );
            assert_eq!(p.game_player.ranked_placing, None, "no ranked placing");
            assert_eq!(p.game_player.rating_change, None, "no rating stamp");
        }
        let ratings: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT gtu.rating, gtu.peak_rating FROM game_type_users gtu \
             JOIN game_players gp ON gp.user_id = gtu.user_id \
             WHERE gp.game_id = $1",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!ratings.is_empty(), "seeded ratings must exist");
        assert!(
            ratings
                .iter()
                .all(|&(rating, peak)| (rating, peak) == (1300, 1400)),
            "game_type_user rating must stay exactly as seeded: {ratings:?}"
        );
        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(
            !logs.iter().any(|l| l.body == "Game ended."),
            "a rejected call must not write the end log"
        );
    }

    #[sqlx::test]
    async fn undo_game_restores_state_and_clears_undo(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[1])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge.game_players[0].game_player.id;

        // Simulate a played command that stashed undo state for player 0.
        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_before_move",
            "state_after_move",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_before_undo = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        undo_game(
            &pool,
            game.id,
            "state_before_move",
            0,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            p0_id,
            ge_before_undo.game.updated_at,
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.game_state, "state_before_move");
        assert!(!ge_after.game.is_finished);
        assert!(ge_after.game.finished_at.is_none());

        for p in &ge_after.game_players {
            assert!(p.game_player.undo_game_state.is_none());
        }
        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert!(p0.game_player.is_turn);
        assert!(!p1.game_player.is_turn);

        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(logs.iter().any(|l| l.body == "{{player 0}} used an undo"));
    }

    #[sqlx::test]
    async fn concede_game_marks_finished(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
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
        assert!(ge_after.game.is_finished);
        assert!(ge_after.game.finished_at.is_some());
    }

    #[sqlx::test]
    async fn delete_game_removes_all_dependent_rows(pool: PgPool) {
        let user = make_user(&pool, "deleter").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, user.id, &[], 1, &[0]).await;

        // A log targeted at the human player, so game_log_targets is exercised.
        let log_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_logs (game_id, body, is_public, logged_at)
             VALUES ($1, 'hello', false, timezone('utc', now())) RETURNING id",
            game.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let player_id: Uuid = sqlx::query_scalar!(
            "SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2",
            game.id,
            user.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO game_log_targets (game_log_id, game_player_id) VALUES ($1, $2)",
            log_id,
            player_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_game(&pool, game.id).await.unwrap();
        assert!(deleted);

        for (table, count) in [
            ("games", count_rows(&pool, "games").await),
            ("game_players", count_rows(&pool, "game_players").await),
            ("game_bots", count_rows(&pool, "game_bots").await),
            ("game_logs", count_rows(&pool, "game_logs").await),
            (
                "game_log_targets",
                count_rows(&pool, "game_log_targets").await,
            ),
        ] {
            assert_eq!(count, 0, "expected no rows left in {}", table);
        }
        // The user survives the delete.
        assert_eq!(count_rows(&pool, "users").await, 1);
    }

    #[sqlx::test]
    async fn delete_game_nulls_restarted_game_id_references(pool: PgPool) {
        let user = make_user(&pool, "restarter").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let old_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[]).await;
        let new_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[0]).await;
        sqlx::query!(
            "UPDATE games SET restarted_game_id = $1 WHERE id = $2",
            new_game.id,
            old_game.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_game(&pool, new_game.id).await.unwrap();
        assert!(deleted);

        let restarted: Option<Uuid> = sqlx::query_scalar!(
            "SELECT restarted_game_id FROM games WHERE id = $1",
            old_game.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(restarted, None);
    }

    #[sqlx::test]
    async fn delete_game_returns_false_for_missing_game(pool: PgPool) {
        let deleted = delete_game(&pool, Uuid::new_v4()).await.unwrap();
        assert!(!deleted);
    }

    /// ws F36: the manual `updated_at = NOW()` assignments were removed from
    /// every UPDATE against a trigger-maintained table (see the module header).
    /// This pins the trigger actually doing the work, for both `games` and
    /// `game_players`, so a future accidental removal of the trigger - or a
    /// re-added manual set - is caught here.
    #[sqlx::test]
    async fn update_updated_at_trigger_maintains_games_and_game_players(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        // Backdate both rows so any trigger-driven bump is unmistakable.
        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE game_players DISABLE TRIGGER update_game_players_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE games SET updated_at = '2020-01-01 00:00:00' WHERE id = $1")
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE game_players SET updated_at = '2020-01-01 00:00:00' WHERE game_id = $1",
        )
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("ALTER TABLE games ENABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE game_players ENABLE TRIGGER update_game_players_updated_at")
            .execute(&pool)
            .await
            .unwrap();

        // mark_game_read UPDATEs game_players and no longer sets updated_at.
        mark_game_read(&pool, game.id, a.id).await.unwrap();
        let gp_updated: time::PrimitiveDateTime = sqlx::query_scalar(
            "SELECT updated_at FROM game_players WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            gp_updated.year() > 2020,
            "update_game_players_updated_at must bump updated_at without a manual set, got {gp_updated}"
        );

        // end_game UPDATEs games and no longer sets updated_at. Locked
        // authorization makes end_game a one-active-human stop, so depart bob
        // while keeping alice as the sole active human actor.
        sqlx::query(
            "UPDATE game_players SET left_at = NOW(), departure_reason = 'conceded', \
             departure_sequence = 1 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(b.id)
        .execute(&pool)
        .await
        .unwrap();
        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let actor_game_player: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(a.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        end_game(&pool, game.id, updated_at, actor_game_player)
            .await
            .unwrap();
        let g_updated: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            g_updated.year() > 2020,
            "update_games_updated_at must bump updated_at without a manual set, got {g_updated}"
        );
    }

    /// ws F35: `insert_game_logs_tx` had no direct test (only empty-vec calls
    /// via `update_game_command_success`). Covers the log row fields, target
    /// fan-out by position, and that a `to` position with no matching player is
    /// silently dropped.
    #[sqlx::test]
    async fn insert_game_logs_tx_writes_logs_and_targets(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let game = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;

        let at = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::March, 4).unwrap(),
            time::Time::from_hms(5, 6, 7).unwrap(),
        );
        let logs = vec![
            brdgme_cmd::api::CliLog {
                content: "public log".to_string(),
                at,
                public: true,
                to: vec![],
            },
            brdgme_cmd::api::CliLog {
                content: "private to both".to_string(),
                at,
                public: false,
                // position 9 does not exist and must be dropped silently.
                to: vec![0, 1, 9],
            },
        ];

        let mut tx = pool.begin().await.unwrap();
        insert_game_logs_tx(&mut tx, game.id, logs).await.unwrap();
        tx.commit().await.unwrap();

        let rows: Vec<(String, bool, time::PrimitiveDateTime, i64)> = sqlx::query_as(
            "SELECT gl.body, gl.is_public, gl.logged_at,
                    (SELECT COUNT(*) FROM game_log_targets t WHERE t.game_log_id = gl.id)
             FROM game_logs gl WHERE gl.game_id = $1 ORDER BY gl.body",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("private to both".to_string(), false, at, 2));
        assert_eq!(rows[1], ("public log".to_string(), true, at, 0));

        // Targets point at the two real player rows (order-independent since
        // make_game_with_players shuffles positions).
        let mut targeted: Vec<Uuid> = sqlx::query_scalar(
            "SELECT gp.user_id FROM game_log_targets t
             JOIN game_players gp ON gp.id = t.game_player_id
             JOIN game_logs gl ON gl.id = t.game_log_id
             WHERE gl.game_id = $1",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        targeted.sort();
        let mut expected = vec![a.id, b.id];
        expected.sort();
        assert_eq!(targeted, expected);
    }

    #[sqlx::test]
    async fn undo_game_rejects_finished_game(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;

        let mut placings = vec![0usize; 2];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;

        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_before",
            "state_final",
            true,
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

        let ge_finished = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let result = undo_game(
            &pool,
            game.id,
            "state_before",
            0,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            p0_id,
            ge_finished.game.updated_at,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<GameAlreadyFinished>().is_some(),
            "expected GameAlreadyFinished, got: {err:?}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.game_state, "state_final");
        assert!(ge_after.game.is_finished);
        assert!(ge_after.game.finished_at.is_some());
        for p in &ge_after.game_players {
            if p.game_player.user_id.is_some() {
                assert!(p.game_player.place.is_some());
                assert!(p.game_player.rating_change.is_some());
            }
        }

        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(!logs.iter().any(|l| l.body.contains("used an undo")));
    }

    #[sqlx::test]
    async fn undo_game_rejects_stale_updated_at(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge.game_players[0].game_player.id;
        let p1_id = ge.game_players[1].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_p0 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let stale_updated_at = ge_after_p0.game.updated_at;

        update_game_command_success(
            &pool,
            game.id,
            p1_id,
            "state_1",
            "state_2",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge_after_p0.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let result = undo_game(
            &pool,
            game.id,
            "state_0",
            0,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            p0_id,
            stale_updated_at,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<StaleStateConflict>().is_some(),
            "expected StaleStateConflict, got: {err:?}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.game_state, "state_2");
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.undo_game_state.as_deref(),
                Some("state_1"),
                "undo_game_state must be unchanged after rejected undo"
            );
        }
    }

    #[sqlx::test]
    async fn undo_game_rejects_consumed_undo_state(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge.game_players[0].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_move = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        undo_game(
            &pool,
            game.id,
            "state_0",
            0,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            p0_id,
            ge_after_move.game.updated_at,
        )
        .await
        .unwrap();

        let ge_after_undo = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let result = undo_game(
            &pool,
            game.id,
            "state_0",
            0,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            p0_id,
            ge_after_undo.game.updated_at,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<StaleStateConflict>().is_some(),
            "expected StaleStateConflict, got: {err:?}"
        );

        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        let undo_logs: Vec<_> = logs
            .iter()
            .filter(|l| l.body.contains("used an undo"))
            .collect();
        assert_eq!(undo_logs.len(), 1);
    }

    #[sqlx::test]
    async fn concede_game_rejects_finished_game(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;

        let mut placings = vec![0usize; 2];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;

        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_before",
            "state_final",
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

        let ge_finished = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceder = ge_finished
            .game_players
            .iter()
            .find(|p| p.game_player.position == opponent_pos as i32)
            .unwrap();
        let result = concede_game(
            &pool,
            game.id,
            conceder.game_player.id,
            "opponent",
            ge_finished.game.updated_at,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<GameAlreadyFinished>().is_some(),
            "expected GameAlreadyFinished, got: {err:?}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        for p in &ge_after.game_players {
            if p.game_player.user_id.is_some() {
                assert!(p.game_player.place.is_some());
            }
        }
    }

    #[sqlx::test]
    async fn concede_game_rejects_stale_updated_at(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let stale_updated_at = ge.game.updated_at;
        let p0_id = ge.game_players[0].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_1",
            false,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceder = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        let result = concede_game(
            &pool,
            game.id,
            conceder.game_player.id,
            "opponent",
            stale_updated_at,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<StaleStateConflict>().is_some(),
            "expected StaleStateConflict, got: {err:?}"
        );

        let ge_final = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge_final.game.is_finished);
    }

    #[sqlx::test]
    async fn concede_game_replace_rejects_finished_game(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let b = make_user(&pool, "b").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[a.id, b.id], 0, &[0])
                .await;

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge.game_players[0].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_final",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2, 3],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_finished = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let a_pos = position_of(&ge_finished, a.id);
        let conceder: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(a_pos)
                .fetch_one(&pool)
                .await
                .unwrap();

        let result =
            concede_game_replace(&pool, game.id, conceder, "a", ge_finished.game.updated_at).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<GameAlreadyFinished>().is_some(),
            "expected GameAlreadyFinished, got: {err:?}"
        );

        let left_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(conceder)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(left_at.is_none());

        let bot_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_bots WHERE game_id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            bot_count, 0,
            "no game_bots row should exist after rejected concede"
        );
    }

    #[sqlx::test]
    async fn concede_game_requires_two_players(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let b = make_user(&pool, "b").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[a.id, b.id], 0, &[0])
                .await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceding = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();

        let result = concede_game(
            &pool,
            game.id,
            conceding.game_player.id,
            "creator",
            ge.game.updated_at,
        )
        .await;

        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("requires exactly 2 players"),
            "expected player-count error, got: {err_msg}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        for p in &ge_after.game_players {
            assert_eq!(p.game_player.place, None);
        }
        assert!(!ge_after.game.is_finished);
    }

    /// DRM-03b2a1: with exactly one active human, the locked forfeit concession
    /// writer rejects with the typed error and the rejected call mutates
    /// nothing.
    #[sqlx::test]
    async fn concede_game_rejects_sole_active_human_without_mutation(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[a.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // Seed authoritative places and ratings so the no-mutation assertions
        // prove the rejected call writes nothing.
        for p in &ge.game_players {
            sqlx::query("UPDATE game_players SET place = $1 WHERE id = $2")
                .bind(p.game_player.position + 5)
                .bind(p.game_player.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "UPDATE game_type_users SET rating = 1300, peak_rating = 1400 \
             WHERE user_id IN (SELECT user_id FROM game_players \
             WHERE game_id = $1 AND user_id IS NOT NULL)",
        )
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

        // Depart one human so exactly one active human remains.
        sqlx::query(
            "UPDATE game_players SET left_at = NOW(), departure_reason = 'conceded', \
             departure_sequence = 1 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .execute(&pool)
        .await
        .unwrap();

        let conceder: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(creator.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        // Capture the seeded departed human's departure metadata before the
        // rejected call so the assertions below can prove it stays exactly
        // unchanged.
        let (departed_left_at, departed_reason, departed_sequence): (
            Option<time::PrimitiveDateTime>,
            Option<String>,
            Option<i32>,
        ) = sqlx::query_as(
            "SELECT left_at, departure_reason, departure_sequence \
             FROM game_players WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let result = concede_game(&pool, game.id, conceder, "creator", ge.game.updated_at).await;
        assert!(result.is_err(), "sole active human must be rejected");
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<NotEnoughActiveHumans>().is_some(),
            "expected NotEnoughActiveHumans, got: {err:?}"
        );

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge_after.game.is_finished, "game must stay unfinished");
        assert!(
            ge_after.game.finished_at.is_none(),
            "finished_at must stay unset"
        );
        assert_eq!(ge_after.game.end_reason, None, "end_reason must stay unset");
        let conceder_row = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.id == conceder)
            .unwrap();
        assert!(
            conceder_row.game_player.left_at.is_none(),
            "conceder must not depart"
        );
        assert!(
            conceder_row.game_player.departure_reason.is_none(),
            "no departure reason"
        );
        assert!(
            conceder_row.game_player.departure_sequence.is_none(),
            "no departure sequence"
        );
        let departed_row = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.user_id == Some(a.id))
            .unwrap();
        assert_eq!(
            departed_row.game_player.left_at, departed_left_at,
            "seeded departed human's left_at must stay exactly unchanged"
        );
        assert_eq!(
            departed_row.game_player.departure_reason, departed_reason,
            "seeded departed human's departure_reason must stay exactly unchanged"
        );
        assert_eq!(
            departed_row.game_player.departure_sequence, departed_sequence,
            "seeded departed human's departure_sequence must stay exactly unchanged"
        );
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.place,
                Some(p.game_player.position + 5),
                "authoritative place must stay exactly as seeded"
            );
            assert_eq!(p.game_player.ranked_placing, None, "no ranked placing");
            assert_eq!(p.game_player.rating_change, None, "no rating stamp");
        }
        let ratings: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT gtu.rating, gtu.peak_rating FROM game_type_users gtu \
             JOIN game_players gp ON gp.user_id = gtu.user_id \
             WHERE gp.game_id = $1",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!ratings.is_empty(), "seeded ratings must exist");
        assert!(
            ratings
                .iter()
                .all(|&(rating, peak)| (rating, peak) == (1300, 1400)),
            "game_type_user rating must stay exactly as seeded: {ratings:?}"
        );
        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(
            !logs.iter().any(|l| l.body.contains("conceded")),
            "a rejected call must not write a concede log"
        );
    }

    #[sqlx::test]
    async fn concede_game_replace_idempotent(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceder = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let conceder_id = conceder.game_player.id;
        let updated_at = ge.game.updated_at;

        concede_game_replace(&pool, game.id, conceder_id, "creator", updated_at)
            .await
            .unwrap();

        let result = concede_game_replace(&pool, game.id, conceder_id, "creator", updated_at).await;
        assert!(result.is_err());

        let bot_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_bots WHERE game_id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(bot_count, 1);

        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        let public_concede_logs: Vec<_> = logs
            .iter()
            .filter(|l| l.is_public && l.body.contains("conceded"))
            .collect();
        assert_eq!(public_concede_logs.len(), 1);
    }

    #[sqlx::test]
    async fn concede_game_replace_rolls_back_on_failure(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceder = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let conceder_id = conceder.game_player.id;

        sqlx::query("INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, 'Hard', 'Hard')")
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();

        let result =
            concede_game_replace(&pool, game.id, conceder_id, "creator", ge.game.updated_at).await;
        assert!(result.is_err());

        let bot_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_bots WHERE game_id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            bot_count, 1,
            "only the pre-existing row, no orphan from the failed tx"
        );

        let left_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(conceder_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(left_at.is_none(), "game_players UPDATE must be rolled back");
    }

    /// DRM-03b2a1: with exactly one active human, the locked replacement
    /// concession writer rejects with the typed error and the rejected call
    /// mutates nothing, including no bot replacement.
    #[sqlx::test]
    async fn concede_game_replace_rejects_sole_active_human_without_mutation(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[a.id], 0, &[0]).await;

        // A replacement bot is configured so that, without the locked
        // two-active-human guard, the sole active human would be replaced.
        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // Seed authoritative places and ratings so the no-mutation assertions
        // prove the rejected call writes nothing.
        for p in &ge.game_players {
            sqlx::query("UPDATE game_players SET place = $1 WHERE id = $2")
                .bind(p.game_player.position + 5)
                .bind(p.game_player.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "UPDATE game_type_users SET rating = 1300, peak_rating = 1400 \
             WHERE user_id IN (SELECT user_id FROM game_players \
             WHERE game_id = $1 AND user_id IS NOT NULL)",
        )
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

        // Depart one human so exactly one active human remains.
        sqlx::query(
            "UPDATE game_players SET left_at = NOW(), departure_reason = 'conceded', \
             departure_sequence = 1 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .execute(&pool)
        .await
        .unwrap();

        let conceder: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2")
                .bind(game.id)
                .bind(creator.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        // Capture the seeded departed human's departure metadata before the
        // rejected call so the assertions below can prove it stays exactly
        // unchanged.
        let (departed_left_at, departed_reason, departed_sequence): (
            Option<time::PrimitiveDateTime>,
            Option<String>,
            Option<i32>,
        ) = sqlx::query_as(
            "SELECT left_at, departure_reason, departure_sequence \
             FROM game_players WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(a.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let result =
            concede_game_replace(&pool, game.id, conceder, "creator", ge.game.updated_at).await;
        assert!(result.is_err(), "sole active human must be rejected");
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<NotEnoughActiveHumans>().is_some(),
            "expected NotEnoughActiveHumans, got: {err:?}"
        );

        let game_bot_id: Option<Uuid> =
            sqlx::query_scalar("SELECT game_bot_id FROM game_players WHERE id = $1")
                .bind(conceder)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            game_bot_id.is_none(),
            "conceder must not be replaced by a bot"
        );
        let bot_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_bots WHERE game_id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(bot_count, 0, "no game_bots row from the rejected call");

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(!ge_after.game.is_finished, "game must stay unfinished");
        assert!(
            ge_after.game.finished_at.is_none(),
            "finished_at must stay unset"
        );
        assert_eq!(ge_after.game.end_reason, None, "end_reason must stay unset");
        let conceder_row = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.id == conceder)
            .unwrap();
        assert!(
            conceder_row.game_player.left_at.is_none(),
            "conceder must not depart"
        );
        assert!(
            conceder_row.game_player.departure_reason.is_none(),
            "no departure reason"
        );
        assert!(
            conceder_row.game_player.departure_sequence.is_none(),
            "no departure sequence"
        );
        let departed_row = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.user_id == Some(a.id))
            .unwrap();
        assert_eq!(
            departed_row.game_player.left_at, departed_left_at,
            "seeded departed human's left_at must stay exactly unchanged"
        );
        assert_eq!(
            departed_row.game_player.departure_reason, departed_reason,
            "seeded departed human's departure_reason must stay exactly unchanged"
        );
        assert_eq!(
            departed_row.game_player.departure_sequence, departed_sequence,
            "seeded departed human's departure_sequence must stay exactly unchanged"
        );
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.place,
                Some(p.game_player.position + 5),
                "authoritative place must stay exactly as seeded"
            );
            assert_eq!(p.game_player.ranked_placing, None, "no ranked placing");
            assert_eq!(p.game_player.rating_change, None, "no rating stamp");
        }
        let ratings: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT gtu.rating, gtu.peak_rating FROM game_type_users gtu \
             JOIN game_players gp ON gp.user_id = gtu.user_id \
             WHERE gp.game_id = $1",
        )
        .bind(game.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!ratings.is_empty(), "seeded ratings must exist");
        assert!(
            ratings
                .iter()
                .all(|&(rating, peak)| (rating, peak) == (1300, 1400)),
            "game_type_user rating must stay exactly as seeded: {ratings:?}"
        );
        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(
            !logs.iter().any(|l| l.body.contains("conceded")),
            "a rejected call must not write a concede log"
        );
    }

    #[sqlx::test]
    async fn concede_game_replace_assigns_turn_to_bot(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceder = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let conceder_id = conceder.game_player.id;

        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(conceder_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE game_players SET is_turn = false WHERE game_id = $1 AND id != $2")
            .bind(game.id)
            .bind(conceder_id)
            .execute(&pool)
            .await
            .unwrap();

        let ge_before = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        concede_game_replace(
            &pool,
            game.id,
            conceder_id,
            "creator",
            ge_before.game.updated_at,
        )
        .await
        .unwrap();

        let is_turn: bool = sqlx::query_scalar("SELECT is_turn FROM game_players WHERE id = $1")
            .bind(conceder_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(is_turn, "replacement bot must inherit the turn");

        let bot_id: Option<Uuid> =
            sqlx::query_scalar("SELECT game_bot_id FROM game_players WHERE id = $1")
                .bind(conceder_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(bot_id.is_some(), "game_bot_id must be set");
    }

    #[sqlx::test]
    async fn undo_game_clears_left_at_on_unelimination_and_restores_points(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opp.id], 0, &[0]).await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_pos = position_of(&ge, creator.id);
        let p1_pos = position_of(&ge, opp.id);
        let p0_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(p0_pos)
                .fetch_one(&pool)
                .await
                .unwrap();
        let p1_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(p1_pos)
                .fetch_one(&pool)
                .await
                .unwrap();

        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![p0_pos as usize],
                eliminated: vec![p1_pos as usize],
                placings: vec![],
            },
            &[10.0, 3.0],
            updated_at,
            vec![],
        )
        .await
        .unwrap();

        let left_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(left_at.is_some(), "elimination must set left_at");

        let departure_reason: Option<String> =
            sqlx::query_scalar("SELECT departure_reason FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            departure_reason.as_deref(),
            Some("eliminated"),
            "elimination must set departure_reason"
        );

        let departure_sequence: Option<i32> =
            sqlx::query_scalar("SELECT departure_sequence FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            departure_sequence.is_some_and(|s| s > 0),
            "elimination must set a positive departure_sequence"
        );

        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        undo_game(
            &pool,
            game.id,
            "state_0",
            p0_pos as usize,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![p0_pos as usize],
                eliminated: vec![],
                placings: vec![],
            },
            &[10.0, 5.0],
            p0_id,
            updated_at,
        )
        .await
        .unwrap();

        let left_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(left_at.is_none(), "un-elimination must clear left_at");

        let departure_reason: Option<String> =
            sqlx::query_scalar("SELECT departure_reason FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            departure_reason.is_none(),
            "un-elimination must clear departure_reason"
        );

        let departure_sequence: Option<i32> =
            sqlx::query_scalar("SELECT departure_sequence FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            departure_sequence.is_none(),
            "un-elimination must clear departure_sequence"
        );

        let is_eliminated: bool =
            sqlx::query_scalar("SELECT is_eliminated FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!is_eliminated, "un-elimination must clear is_eliminated");

        let points: Option<f32> =
            sqlx::query_scalar("SELECT points FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(points, Some(5.0), "undo must restore points");
    }

    #[sqlx::test]
    async fn concede_game_replace_rejects_already_left_player(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let b = make_user(&pool, "b").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[a.id, b.id], 0, &[0])
                .await;

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let a_pos = position_of(&ge, a.id);
        let a_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(a_pos)
                .fetch_one(&pool)
                .await
                .unwrap();

        sqlx::query("UPDATE game_players SET left_at = NOW(), is_eliminated = true WHERE id = $1")
            .bind(a_id)
            .execute(&pool)
            .await
            .unwrap();

        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let result = concede_game_replace(&pool, game.id, a_id, "a", updated_at).await;
        assert!(result.is_err(), "must reject an already-left player");
        assert!(
            result.unwrap_err().to_string().contains("already left"),
            "error must mention already left"
        );
    }

    #[sqlx::test]
    async fn concede_game_writes_ranked_placing(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opp.id], 0, &[0]).await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let opp_pos = position_of(&ge, opp.id);
        let opp_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(opp_pos)
                .fetch_one(&pool)
                .await
                .unwrap();

        concede_game(&pool, game.id, opp_id, "opp", ge.game.updated_at)
            .await
            .unwrap();

        let winner_ranked: Option<i32> = sqlx::query_scalar(
            "SELECT ranked_placing FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(winner_ranked, Some(1), "winner must have ranked_placing 1");

        let loser_ranked: Option<i32> = sqlx::query_scalar(
            "SELECT ranked_placing FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opp_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(loser_ranked, Some(2), "conceder must have ranked_placing 2");
    }

    #[sqlx::test]
    async fn elimination_guard_preserves_state_on_finish(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opp.id], 0, &[0]).await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_pos = position_of(&ge, creator.id);
        let p1_pos = position_of(&ge, opp.id);
        let p0_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(p0_pos)
                .fetch_one(&pool)
                .await
                .unwrap();
        let p1_id: Uuid =
            sqlx::query_scalar("SELECT id FROM game_players WHERE game_id = $1 AND position = $2")
                .bind(game.id)
                .bind(p1_pos)
                .fetch_one(&pool)
                .await
                .unwrap();

        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_1",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![p1_pos as usize],
                placings: vec![p0_pos as usize, p1_pos as usize],
            },
            &[10.0, 0.0],
            updated_at,
            vec![],
        )
        .await
        .unwrap();

        let is_eliminated: bool =
            sqlx::query_scalar("SELECT is_eliminated FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            !is_eliminated,
            "is_finished=true must preserve existing is_eliminated (false), not write the status value"
        );

        let left_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
                .bind(p1_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            left_at.is_none(),
            "is_finished=true must not set left_at even when status reports elimination"
        );
    }

    /// DRM-03a: old pods write `left_at` without departure metadata during the
    /// rollout. The lifecycle writer stamps such unfinished human rows
    /// `unknown_legacy` with a per-game dense `left_at` sequence (equal
    /// timestamps tie).
    #[sqlx::test]
    async fn update_game_command_success_normalizes_old_pod_departures(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let b = make_user(&pool, "b").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id, b.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let a_pos = position_of(&ge, a.id);
        let b_pos = position_of(&ge, b.id);

        // Old-pod state: left_at set, no departure metadata. creator and a tie.
        for pos in [creator_pos, a_pos] {
            sqlx::query(
                "UPDATE game_players SET left_at = '2026-01-01 00:00:00' \
                 WHERE game_id = $1 AND position = $2",
            )
            .bind(game.id)
            .bind(pos)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-02 00:00:00' \
             WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(b_pos)
        .execute(&pool)
        .await
        .unwrap();

        let played_id = ge.game_players[0].game_player.id;
        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s0",
            "s1",
            false,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![creator_pos as usize],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let dep = |pos: i32| -> (Option<String>, Option<i32>) {
            let p = ge_after
                .game_players
                .iter()
                .find(|p| p.game_player.position == pos)
                .unwrap();
            (
                p.game_player.departure_reason.clone(),
                p.game_player.departure_sequence,
            )
        };
        assert_eq!(
            dep(creator_pos),
            (Some("unknown_legacy".to_string()), Some(1)),
            "earliest tie shares sequence 1"
        );
        assert_eq!(dep(a_pos), (Some("unknown_legacy".to_string()), Some(1)));
        assert_eq!(dep(b_pos), (Some("unknown_legacy".to_string()), Some(2)));
    }

    /// DRM-03a: the normalisation helper never touches completed games, and
    /// new old-pod rows continue the per-game dense numbering past any
    /// already-assigned departure sequences rather than colliding with them.
    #[sqlx::test]
    async fn normalize_legacy_departures_tx_skips_completed_and_continues(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let b = make_user(&pool, "b").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id, b.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);

        // A completed game's departed rows are never written.
        sqlx::query("UPDATE games SET is_finished = true WHERE id = $1")
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-01 00:00:00' \
             WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos)
        .execute(&pool)
        .await
        .unwrap();
        {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("SELECT 1 FROM games WHERE id = $1 FOR UPDATE")
                .bind(game.id)
                .execute(&mut *tx)
                .await
                .unwrap();
            normalize_legacy_departures_tx(&mut tx, game.id)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
        let dep: Option<String> = sqlx::query_scalar(
            "SELECT departure_reason FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(dep.is_none(), "completed game rows must stay untouched");

        // A fresh unfinished game: one departure already carries sequence 1
        // (assigned by the new writer); later old-pod departures must slot in
        // as dense events after it, never reusing its sequence.
        let (_, gv2) = make_game_type_and_version(&pool).await;
        let game2 = make_game_with_players(&pool, gv2, creator.id, &[a.id, b.id], 0, &[0]).await;
        let ge2 = find_game_extended(&pool, game2.id).await.unwrap().unwrap();
        let creator2_pos = position_of(&ge2, creator.id);
        let a2_pos = position_of(&ge2, a.id);
        let b2_pos = position_of(&ge2, b.id);
        sqlx::query(
            "UPDATE game_players SET departure_reason = 'eliminated', departure_sequence = 1, \
             left_at = '2026-01-01 00:00:00' \
             WHERE game_id = $1 AND position = $2",
        )
        .bind(game2.id)
        .bind(a2_pos)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-02 00:00:00' \
             WHERE game_id = $1 AND position = $2",
        )
        .bind(game2.id)
        .bind(creator2_pos)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-03 00:00:00' \
             WHERE game_id = $1 AND position = $2",
        )
        .bind(game2.id)
        .bind(b2_pos)
        .execute(&pool)
        .await
        .unwrap();
        {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("SELECT 1 FROM games WHERE id = $1 FOR UPDATE")
                .bind(game2.id)
                .execute(&mut *tx)
                .await
                .unwrap();
            normalize_legacy_departures_tx(&mut tx, game2.id)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
        let game2_id = game2.id;
        let pool_ref = &pool;
        let seq = move |pos: i32| async move {
            sqlx::query_scalar::<_, i32>(
                "SELECT departure_sequence FROM game_players WHERE game_id = $1 AND position = $2",
            )
            .bind(game2_id)
            .bind(pos)
            .fetch_one(pool_ref)
            .await
            .unwrap()
        };
        assert_eq!(seq(a2_pos).await, 1, "existing sequence must be preserved");
        assert_eq!(
            seq(creator2_pos).await,
            2,
            "dense among old-pod rows, offset past event 1"
        );
        assert_eq!(seq(b2_pos).await, 3);
    }

    /// DRM-03a: an active service update assigns `departure_reason=eliminated`,
    /// `left_at`, and one shared positive sequence only to newly eliminated
    /// human seats; repeated reports retain existing metadata and pure bots
    /// stay bare.
    #[sqlx::test]
    async fn update_game_command_success_assigns_elimination_departure_metadata(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let a = make_user(&pool, "a").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        // Two humans plus one pure bot.
        let game = make_game_with_players(&pool, gv, creator.id, &[a.id], 1, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let a_pos = position_of(&ge, a.id);
        let bot_pos = ge
            .game_players
            .iter()
            .find(|p| p.game_player.user_id.is_none())
            .unwrap()
            .game_player
            .position;
        let played_id = ge.game_players[0].game_player.id;

        // Report 1: creator and a eliminated together -> one shared sequence.
        let status = StatusUpdate {
            is_finished: false,
            whose_turn: vec![],
            eliminated: vec![creator_pos as usize, a_pos as usize],
            placings: vec![],
        };
        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s0",
            "s1",
            false,
            &status,
            &[],
            updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let dep = |pos: i32| -> (Option<String>, Option<i32>) {
            let p = ge_after
                .game_players
                .iter()
                .find(|p| p.game_player.position == pos)
                .unwrap();
            (
                p.game_player.departure_reason.clone(),
                p.game_player.departure_sequence,
            )
        };
        assert_eq!(dep(creator_pos), (Some("eliminated".to_string()), Some(1)));
        assert_eq!(dep(a_pos), (Some("eliminated".to_string()), Some(1)));
        assert_eq!(
            dep(bot_pos),
            (None, None),
            "pure bots get no departure metadata"
        );

        // Report 2: identical report retains the existing metadata.
        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s1",
            "s2",
            false,
            &status,
            &[],
            updated_at,
            vec![],
        )
        .await
        .unwrap();
        let creator_dep_seq: Option<i32> = sqlx::query_scalar(
            "SELECT departure_sequence FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            creator_dep_seq,
            Some(1),
            "repeated reports must retain existing metadata"
        );

        // Report 3: only the bot is newly eliminated -> still no bot
        // metadata, and no new human sequence is allocated.
        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s2",
            "s3",
            false,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![],
                eliminated: vec![bot_pos as usize],
                placings: vec![],
            },
            &[],
            updated_at,
            vec![],
        )
        .await
        .unwrap();
        let bot_dep: Option<String> = sqlx::query_scalar(
            "SELECT departure_reason FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(bot_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            bot_dep.is_none(),
            "a bot-only report must not stamp departure metadata"
        );
    }

    /// DRM-03a: a seat that already left (left_at set, e.g. a conceded-replaced
    /// human during the rollout window) is never "newly eliminated" - a later
    /// report eliminating that position must not stamp it `eliminated`.
    #[sqlx::test]
    async fn update_game_command_success_elimination_skips_already_left_humans(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[opp.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let opp_pos = position_of(&ge, opp.id);
        let played_id = ge.game_players[0].game_player.id;

        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-01 00:00:00' \
             WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opp_pos)
        .execute(&pool)
        .await
        .unwrap();

        let updated_at: time::PrimitiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
                .bind(game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s0",
            "s1",
            false,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![],
                eliminated: vec![opp_pos as usize],
                placings: vec![],
            },
            &[],
            updated_at,
            vec![],
        )
        .await
        .unwrap();

        let opp_dep: Option<String> = sqlx::query_scalar(
            "SELECT departure_reason FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opp_pos)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            opp_dep.is_none(),
            "an already-left seat must not be stamped eliminated"
        );
    }

    /// DRM-03a: a normal service finish writes `end_reason = 'game_service'`,
    /// persists the exact service placings by position, then computes
    /// competitive ranks and applies ELO in the same transaction.
    #[sqlx::test]
    async fn update_game_command_success_service_finish_writes_end_reason_places_and_rating(
        pool: PgPool,
    ) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[opp.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let opp_pos = position_of(&ge, opp.id);
        let played_id = ge.game_players[0].game_player.id;

        let mut placings = vec![0usize; 2];
        placings[creator_pos as usize] = 1;
        placings[opp_pos as usize] = 2;

        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s0",
            "final",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[10.0, 5.0],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.end_reason.as_deref(), Some("game_service"));
        let by_pos = |pos: i32| {
            ge_after
                .game_players
                .iter()
                .find(|p| p.game_player.position == pos)
                .unwrap()
                .game_player
                .clone()
        };
        assert_eq!(by_pos(creator_pos).place, Some(1));
        assert_eq!(by_pos(opp_pos).place, Some(2));
        assert_eq!(by_pos(creator_pos).ranked_placing, Some(1));
        assert_eq!(by_pos(opp_pos).ranked_placing, Some(2));
        assert_eq!(by_pos(creator_pos).rating_change, Some(16));
        assert_eq!(by_pos(opp_pos).rating_change, Some(-16));
    }

    /// DRM-03a: a service finish with no placings records the finish but
    /// remains unranked and unrated - no places are invented from points or
    /// any other source.
    #[sqlx::test]
    async fn update_game_command_success_service_finish_without_placings_stays_unrated(
        pool: PgPool,
    ) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[opp.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_id = ge.game_players[0].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s0",
            "final",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![],
            },
            &[10.0, 5.0],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.end_reason.as_deref(), Some("game_service"));
        for p in &ge_after.game_players {
            assert_eq!(p.game_player.place, None);
            assert_eq!(p.game_player.ranked_placing, None);
            assert_eq!(p.game_player.rating_change, None);
        }
    }

    /// DRM-03a: an old-pod departed human is normalised to `unknown_legacy`
    /// before a finishing report ranks, so it places after the active humans
    /// instead of being treated as an active participant.
    #[sqlx::test]
    async fn update_game_command_success_finish_ranks_old_pod_departed_human_after_active(
        pool: PgPool,
    ) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[opp.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let creator_pos = position_of(&ge, creator.id);
        let opp_pos = position_of(&ge, opp.id);
        let played_id = ge.game_players[0].game_player.id;

        // Old-pod departure: opp left earlier, metadata never written.
        sqlx::query(
            "UPDATE game_players SET left_at = '2026-01-01 00:00:00', is_eliminated = true \
             WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opp_pos)
        .execute(&pool)
        .await
        .unwrap();

        let mut placings = vec![0usize; 2];
        placings[creator_pos as usize] = 1;
        placings[opp_pos as usize] = 2;

        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s0",
            "final",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[10.0, 5.0],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let opp_p = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == opp_pos)
            .unwrap();
        assert_eq!(
            opp_p.game_player.departure_reason.as_deref(),
            Some("unknown_legacy")
        );
        assert_eq!(opp_p.game_player.departure_sequence, Some(1));
        assert_eq!(
            ge_after
                .game_players
                .iter()
                .find(|p| p.game_player.position == creator_pos)
                .unwrap()
                .game_player
                .ranked_placing,
            Some(1),
            "active human ranks ahead of the departed one"
        );
        assert_eq!(opp_p.game_player.ranked_placing, Some(2));
    }

    /// DRM-03a: a terminal service status never infers elimination departures
    /// - the `eliminated` list on a finish report writes no departure metadata.
    #[sqlx::test]
    async fn update_game_command_success_finish_does_not_infer_departures(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opp = make_user(&pool, "opp").await;
        let (_, gv) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, gv, creator.id, &[opp.id], 0, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let opp_pos = position_of(&ge, opp.id);
        let played_id = ge.game_players[0].game_player.id;

        let mut placings = vec![0usize; 2];
        placings[position_of(&ge, creator.id) as usize] = 1;
        placings[opp_pos as usize] = 2;

        update_game_command_success(
            &pool,
            game.id,
            played_id,
            "s0",
            "final",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![opp_pos as usize],
                placings,
            },
            &[10.0, 5.0],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.departure_reason, None,
                "a finish report must not infer departure metadata from its eliminated list"
            );
            assert_eq!(p.game_player.departure_sequence, None);
        }
    }
}
