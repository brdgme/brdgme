use super::*;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %id))]
pub async fn find_game(pool: &PgPool, id: Uuid) -> Result<Option<crate::models::game::Game>> {
    sqlx::query_as!(
        crate::models::game::Game,
        r#"
        SELECT id, created_at, updated_at, game_version_id, is_finished, finished_at, game_state, chat_id, restarted_game_id
        FROM games
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GamePlayerExtended {
    pub game_player: crate::models::game::GamePlayer,
    pub user: Option<crate::models::user::User>,
    pub game_bot: Option<crate::models::game::GameBot>,
    pub game_type_user: crate::models::game::GameTypeUser,
}

#[cfg(feature = "ssr")]
impl GamePlayerExtended {
    pub fn name(&self) -> &str {
        if let Some(u) = &self.user {
            &u.name
        } else if let Some(b) = &self.game_bot {
            &b.name
        } else {
            "Bot"
        }
    }

    /// This game player's `--mk-{slot}` colour slot token (e.g. "green") -
    /// the web layer's colour representation; never resolve this to a
    /// concrete hex value for display, that bakes in one theme.
    pub fn slot(&self) -> &'static str {
        crate::theme::slot_from_color_name(&self.game_player.color)
    }
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameExtended {
    pub game: crate::models::game::Game,
    pub game_type: crate::models::game::GameType,
    pub game_version: crate::models::game::GameVersion,
    pub game_players: Vec<GamePlayerExtended>,
}

#[cfg(feature = "ssr")]
impl GameExtended {
    /// Names-only semantic players for `transform_semantic` - colour stays
    /// symbolic (`SemanticColType::Player(n)`) and is resolved client-side by
    /// the `--mk-player-{n}` vars this game's `player_style_vars` container
    /// sets, not baked into the HTML here.
    pub fn semantic_players(&self) -> Vec<brdgme_markup::SemanticPlayer> {
        self.game_players
            .iter()
            .map(|p| brdgme_markup::SemanticPlayer {
                name: p.name().to_string(),
            })
            .collect()
    }

    /// The `--mk-player-{n}` container style for this game's board/log HTML.
    pub fn player_style(&self) -> String {
        let slots: Vec<&str> = self.game_players.iter().map(|p| p.slot()).collect();
        crate::theme::player_style_vars(&slots)
    }
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %id))]
pub async fn find_game_extended(pool: &PgPool, id: Uuid) -> Result<Option<GameExtended>> {
    let game = find_game(pool, id).await?;
    let game = match game {
        Some(g) => g,
        None => return Ok(None),
    };

    let game_version = find_game_version(pool, game.game_version_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Game version not found"))?;

    let game_type = sqlx::query_as!(
        crate::models::game::GameType,
        "SELECT id, created_at, updated_at, name, player_counts, weight, blurb FROM game_types WHERE id = $1",
        game_version.game_type_id
    )
    .fetch_one(pool)
    .await?;

    let players_raw = sqlx::query!(
        r#"
        SELECT
            gp.id as gp_id, gp.created_at as gp_created_at, gp.updated_at as gp_updated_at,
            gp.game_id as gp_game_id, gp.user_id as gp_user_id, gp.position as gp_position,
            gp.color as gp_color, gp.has_accepted as gp_has_accepted, gp.is_turn as gp_is_turn,
            gp.is_turn_at as gp_is_turn_at, gp.place as gp_place,
            gp.last_turn_at as gp_last_turn_at, gp.is_eliminated as gp_is_eliminated,
            gp.is_read as gp_is_read, gp.points as gp_points,
            gp.undo_game_state as gp_undo_game_state, gp.rating_change as gp_rating_change,
            gp.ranked_placing as gp_ranked_placing, gp.left_at as gp_left_at,
            u.id as "u_id?", u.created_at as "u_created_at?", u.updated_at as "u_updated_at?",
            u.name as "u_name?", u.pref_colors as "u_pref_colors?",
            gtu.id as "gtu_id?", gtu.created_at as "gtu_created_at?", gtu.updated_at as "gtu_updated_at?",
            gtu.game_type_id as "gtu_game_type_id?", gtu.user_id as "gtu_user_id?",
            gtu.last_game_finished_at as "gtu_last_game_finished_at?", gtu.rating as "gtu_rating?",
            gtu.peak_rating as "gtu_peak_rating?",
            gb.id as "gb_id?", gb.game_id as "gb_game_id?", gb.name as "gb_name?",
            gb.bot_name as "gb_bot_name?"
        FROM game_players gp
        LEFT JOIN users u ON gp.user_id = u.id
        LEFT JOIN game_type_users gtu ON gtu.user_id = u.id AND gtu.game_type_id = $2
        LEFT JOIN game_bots gb ON gp.game_bot_id = gb.id
        WHERE gp.game_id = $1
        ORDER BY gp.position
        "#,
        id,
        game_version.game_type_id
    )
    .fetch_all(pool)
    .await?;

    let mut game_players = Vec::new();
    for p in players_raw {
        let gtu = build_game_type_user(
            p.gtu_id,
            p.gtu_created_at,
            p.gtu_updated_at,
            p.gtu_game_type_id,
            p.gtu_user_id,
            p.gtu_last_game_finished_at,
            p.gtu_rating,
            p.gtu_peak_rating,
            p.u_id,
            game_version.game_type_id,
            p.gp_created_at,
        );
        let user = build_user_from_row(
            p.u_id,
            p.u_created_at,
            p.u_updated_at,
            p.u_name,
            p.u_pref_colors,
        )?;
        let game_bot = build_game_bot_from_row(p.gb_id, p.gb_game_id, p.gb_name, p.gb_bot_name)?;

        game_players.push(GamePlayerExtended {
            game_player: build_game_player_from_row(
                p.gp_id,
                p.gp_created_at,
                p.gp_updated_at,
                p.gp_game_id,
                p.gp_user_id,
                p.gp_position,
                p.gp_color,
                p.gp_has_accepted,
                p.gp_is_turn,
                p.gp_is_turn_at,
                p.gp_place,
                p.gp_last_turn_at,
                p.gp_is_eliminated,
                p.gp_is_read,
                p.gp_points,
                p.gp_undo_game_state,
                p.gp_rating_change,
                p.gp_ranked_placing,
                p.gp_left_at,
            ),
            user,
            game_bot,
            game_type_user: gtu,
        });
    }

    Ok(Some(GameExtended {
        game,
        game_type,
        game_version,
        game_players,
    }))
}

#[cfg(feature = "ssr")]
pub async fn is_player_in_game(pool: &PgPool, game_id: Uuid, user_id: Uuid) -> Result<bool> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM game_players WHERE game_id = $1 AND user_id = $2) AS "exists!""#,
        game_id,
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

/// Skinny projection for the sidebar: one row per (game, opponent), already
/// sorted my-turn-first then most recently updated. Opponent rows are LEFT
/// JOINed so games with no opponents still appear; exclusion of the
/// requesting user's own seat is by player-row id, not user id.
#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(user_id = %user_id))]
pub async fn find_active_game_summaries(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<crate::game::server_fns::GameSummary>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            g.id as game_id,
            gv.name as version_name,
            gt.name as type_name,
            me.is_turn as my_is_turn,
            me.is_turn_at as my_is_turn_at,
            opp.id as "opp_id?",
            COALESCE(u.name, gb.name, 'Bot') as "opp_name!",
            opp.color as "opp_color?"
        FROM games g
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
        LEFT JOIN game_players opp ON opp.game_id = g.id AND opp.id <> me.id
        LEFT JOIN users u ON u.id = opp.user_id
        LEFT JOIN game_bots gb ON gb.id = opp.game_bot_id
        WHERE g.is_finished = false
        ORDER BY
            me.is_turn DESC,
            CASE WHEN me.is_turn THEN me.is_turn_at END ASC,
            CASE WHEN NOT me.is_turn THEN me.last_turn_at END DESC,
            g.id, opp.position
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let mut summaries: Vec<crate::game::server_fns::GameSummary> = Vec::new();
    for row in rows {
        if summaries.last().map(|s| s.id) != Some(row.game_id) {
            summaries.push(crate::game::server_fns::GameSummary {
                id: row.game_id,
                name: row.version_name,
                type_name: row.type_name,
                opponents: Vec::new(),
                is_turn: row.my_is_turn,
                is_turn_at: row.my_is_turn_at,
            });
        }
        if row.opp_id.is_some() {
            let color = crate::theme::slot_from_color_name(row.opp_color.as_deref().unwrap_or(""))
                .to_string();
            let summary = summaries.last_mut().ok_or_else(|| {
                anyhow::anyhow!("opponent row for game {} has no summary", row.game_id)
            })?;
            summary
                .opponents
                .push(crate::game::server_fns::OpponentSummary {
                    name: row.opp_name,
                    color,
                });
        }
    }

    Ok(summaries)
}

#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct PendingGameRow {
    proposal_id: Uuid,
    type_name: String,
    owner_user_id: Uuid,
    my_response: String,
    player_user_id: Option<Uuid>,
    player_name: String,
    player_response: String,
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(user_id = %user_id))]
pub async fn find_pending_game_summaries(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<crate::game::server_fns::PendingGameSummary>> {
    let rows = sqlx::query_as::<_, PendingGameRow>(
        "SELECT gp.id AS proposal_id, gt.name AS type_name, gp.owner_user_id, \
                me.response AS my_response, pp.user_id AS player_user_id, \
                COALESCE(u.name, pp.bot_name, 'Bot') AS player_name, \
                pp.response AS player_response \
         FROM game_proposals gp \
         JOIN game_versions gv ON gv.id = gp.game_version_id \
         JOIN game_types gt ON gt.id = gv.game_type_id \
         JOIN game_proposal_players me ON me.proposal_id = gp.id AND me.user_id = $1 \
         JOIN game_proposal_players pp ON pp.proposal_id = gp.id \
         LEFT JOIN users u ON u.id = pp.user_id \
         WHERE gp.status = 'open' \
         ORDER BY gp.created_at DESC, gp.id, pp.position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<crate::game::server_fns::PendingGameSummary> = Vec::new();
    for row in rows {
        let is_owner = row.owner_user_id == user_id;
        if out.last().map(|s| s.id) != Some(row.proposal_id) {
            out.push(crate::game::server_fns::PendingGameSummary {
                id: row.proposal_id,
                type_name: row.type_name,
                players: Vec::new(),
                is_owner,
                is_invitee_needing_accept: !is_owner && row.my_response == "pending",
                is_ready_to_start: true,
            });
        }
        let summary = out.last_mut().ok_or_else(|| {
            anyhow::anyhow!(
                "pending player row for proposal {} has no summary",
                row.proposal_id
            )
        })?;
        if row.player_user_id.is_some() && row.player_response != "accepted" {
            summary.is_ready_to_start = false;
        }
        if row.player_user_id != Some(user_id) {
            summary.players.push(row.player_name);
        }
    }
    Ok(out)
}

#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct FinishedGameRow {
    game_id: Uuid,
    type_name: String,
    opp_name: Option<String>,
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(user_id = %user_id))]
pub async fn find_finished_game_summaries(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<crate::game::server_fns::FinishedGameSummary>> {
    let rows = sqlx::query_as::<_, FinishedGameRow>(
        "SELECT g.id AS game_id, gt.name AS type_name, \
                COALESCE(u.name, gb.name, 'Bot') AS opp_name \
         FROM games g \
         JOIN game_versions gv ON gv.id = g.game_version_id \
         JOIN game_types gt ON gt.id = gv.game_type_id \
         JOIN game_players me ON me.game_id = g.id AND me.user_id = $1 \
         LEFT JOIN game_players opp ON opp.game_id = g.id AND opp.id <> me.id \
         LEFT JOIN users u ON u.id = opp.user_id \
         LEFT JOIN game_bots gb ON gb.id = opp.game_bot_id \
         WHERE g.is_finished = true \
           AND g.id IN ( \
               SELECT g2.id FROM games g2 \
               JOIN game_players me2 ON me2.game_id = g2.id AND me2.user_id = $1 \
               WHERE g2.is_finished = true \
               ORDER BY g2.finished_at DESC, g2.id \
               LIMIT 3 \
           ) \
         ORDER BY g.finished_at DESC, g.id, opp.position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<crate::game::server_fns::FinishedGameSummary> = Vec::new();
    for row in rows {
        if out.last().map(|s| s.id) != Some(row.game_id) {
            out.push(crate::game::server_fns::FinishedGameSummary {
                id: row.game_id,
                type_name: row.type_name,
                players: Vec::new(),
            });
        }
        if let Some(name) = row.opp_name {
            let summary = out.last_mut().ok_or_else(|| {
                anyhow::anyhow!(
                    "finished opponent row for game {} has no summary",
                    row.game_id
                )
            })?;
            summary.players.push(name);
        }
    }
    Ok(out)
}

/// The predecessor of a game is the older game that was restarted into it:
/// `games.restarted_game_id` points old->new, so the new game's predecessor is
/// the row whose `restarted_game_id` equals this game's id. At most one old
/// game points at a given new game.
#[cfg(feature = "ssr")]
pub async fn find_predecessor_game_id(pool: &PgPool, game_id: Uuid) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM games WHERE restarted_game_id = $1")
        .bind(game_id)
        .fetch_optional(pool)
        .await?;
    Ok(id)
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id, user_id = %user_id))]
pub async fn mark_game_read(pool: &PgPool, game_id: Uuid, user_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE game_players SET is_read = true WHERE game_id = $1 AND user_id = $2",
        game_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn get_all_game_logs(
    pool: &PgPool,
    game_id: Uuid,
) -> Result<Vec<crate::models::game::GameLog>> {
    sqlx::query_as!(
        crate::models::game::GameLog,
        r#"
        SELECT id, created_at, updated_at, game_id, body, is_public, logged_at
        FROM game_logs
        WHERE game_id = $1
        ORDER BY logged_at ASC
        "#,
        game_id,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn get_game_logs(
    pool: &PgPool,
    game_id: Uuid,
    game_player_id: Uuid,
) -> Result<Vec<crate::models::game::GameLog>> {
    sqlx::query_as!(
        crate::models::game::GameLog,
        r#"
        SELECT id, created_at, updated_at, game_id, body, is_public, logged_at
        FROM game_logs
        WHERE game_id = $1
          AND (is_public = true OR id IN (
              SELECT game_log_id FROM game_log_targets WHERE game_player_id = $2
          ))
        ORDER BY logged_at ASC
        "#,
        game_id,
        game_player_id,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// The most recent `limit` public log lines for a game, returned in
/// chronological order (oldest first). For the logged-out index's "3 recent
/// log lines" - spectators only ever see `is_public = true` lines (the
/// player-scoped `get_game_logs` also surfaces targeted private lines, which
/// must NOT appear on the public index).
#[cfg(feature = "ssr")]
pub async fn find_recent_game_log_lines(
    pool: &PgPool,
    game_id: Uuid,
    limit: i64,
) -> Result<Vec<crate::models::game::GameLog>> {
    let mut logs: Vec<crate::models::game::GameLog> = sqlx::query_as(
        "SELECT id, created_at, updated_at, game_id, body, is_public, logged_at
         FROM game_logs
         WHERE game_id = $1 AND is_public = true
         ORDER BY logged_at DESC
         LIMIT $2",
    )
    .bind(game_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    logs.reverse();
    Ok(logs)
}

/// Games where the user currently holds the turn in an unfinished game, oldest
/// turn first, capped at `cap` (the 22d switch-digest targets). Returns
/// `(game_id, game_player_id)` pairs.
#[cfg(feature = "ssr")]
pub async fn find_active_turn_games(
    pool: &PgPool,
    user_id: Uuid,
    cap: usize,
) -> Result<Vec<(Uuid, Uuid)>> {
    let cap = cap as i64;
    Ok(sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT gp.game_id, gp.id
         FROM game_players gp
         JOIN games g ON gp.game_id = g.id
         WHERE gp.user_id = $1 AND gp.is_turn = true AND g.is_finished = false
         ORDER BY gp.is_turn_at ASC NULLS LAST
         LIMIT $2",
    )
    .bind(user_id)
    .bind(cap)
    .fetch_all(pool)
    .await?)
}

/// The game version a game was created from. `Ok(None)` when the game does not
/// exist.
#[cfg(feature = "ssr")]
pub async fn find_game_version_id_for_game(pool: &PgPool, game_id: Uuid) -> Result<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT game_version_id FROM games WHERE id = $1")
            .bind(game_id)
            .fetch_optional(pool)
            .await?,
    )
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    #[sqlx::test]
    async fn find_recent_game_log_lines_returns_last_n_in_order(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let alice = make_user(&pool, "alice").await;
        let game = make_game_with_players(&pool, gv, alice.id, &[], 0, &[0]).await;

        for (i, body) in ["line1", "line2", "line3", "line4"].iter().enumerate() {
            let minutes = 4 - i as i32;
            sqlx::query(
                "INSERT INTO game_logs (game_id, body, is_public, logged_at) \
                 VALUES ($1, $2, true, NOW() - ($3 || ' minutes')::interval)",
            )
            .bind(game.id)
            .bind(*body)
            .bind(minutes.to_string())
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO game_logs (game_id, body, is_public, logged_at) \
             VALUES ($1, $2, false, NOW())",
        )
        .bind(game.id)
        .bind("secret")
        .execute(&pool)
        .await
        .unwrap();

        let logs = find_recent_game_log_lines(&pool, game.id, 3).await.unwrap();
        let bodies: Vec<&str> = logs.iter().map(|l| l.body.as_str()).collect();
        assert_eq!(logs.len(), 3);
        assert_eq!(bodies, ["line2", "line3", "line4"]);
    }

    #[sqlx::test]
    async fn active_summaries_exclude_finished_and_pending(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let opp = make_user(&pool, "opp").await;
        let (_, version) = make_game_type_and_version(&pool).await;

        // In-progress game: should appear.
        make_game_with_players(&pool, version, me.id, &[opp.id], 0, &[0]).await;

        // Finished game: excluded.
        let finished = make_game_with_players(&pool, version, me.id, &[opp.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET is_finished = TRUE, finished_at = timezone('utc', now()) WHERE id = $1")
            .bind(finished.id)
            .execute(&pool)
            .await
            .unwrap();

        // Open proposal owned by me: excluded (no games row yet).
        let proposal = make_proposal(&pool, version, me.id).await;
        add_proposal_player(&pool, proposal, 0, Some(me.id), None, "accepted").await;

        let rows = find_active_game_summaries(&pool, me.id).await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[sqlx::test]
    async fn pending_summaries_roles_and_ready_to_start(pool: PgPool) {
        let owner = make_user(&pool, "owner").await;
        let invitee_a = make_user(&pool, "invitee_a").await;
        let invitee_b = make_user(&pool, "invitee_b").await;
        let (_, version) = make_game_type_and_version(&pool).await;

        let proposal = make_proposal(&pool, version, owner.id).await;
        add_proposal_player(&pool, proposal, 0, Some(owner.id), None, "accepted").await;
        add_proposal_player(&pool, proposal, 1, Some(invitee_a.id), None, "pending").await;
        add_proposal_player(&pool, proposal, 2, Some(invitee_b.id), None, "accepted").await;

        // Owner view.
        let owner_rows = find_pending_game_summaries(&pool, owner.id).await.unwrap();
        assert_eq!(owner_rows.len(), 1);
        let o = &owner_rows[0];
        assert!(o.is_owner);
        assert!(!o.is_invitee_needing_accept);
        assert!(!o.is_ready_to_start, "invitee_a still pending");
        let mut o_players = o.players.clone();
        o_players.sort();
        assert_eq!(
            o_players,
            vec!["invitee_a".to_string(), "invitee_b".to_string()]
        );

        // Invitee A view.
        let a_rows = find_pending_game_summaries(&pool, invitee_a.id)
            .await
            .unwrap();
        assert_eq!(a_rows.len(), 1);
        let a = &a_rows[0];
        assert!(!a.is_owner);
        assert!(a.is_invitee_needing_accept);
        let mut a_players = a.players.clone();
        a_players.sort();
        assert_eq!(
            a_players,
            vec!["invitee_b".to_string(), "owner".to_string()]
        );

        // Once invitee_a accepts, the owner can start.
        sqlx::query("UPDATE game_proposal_players SET response='accepted' WHERE proposal_id=$1 AND user_id=$2")
            .bind(proposal)
            .bind(invitee_a.id)
            .execute(&pool)
            .await
            .unwrap();
        let owner_rows = find_pending_game_summaries(&pool, owner.id).await.unwrap();
        assert!(owner_rows[0].is_ready_to_start);

        // A bot never blocks ready_to_start.
        let bot_proposal = make_proposal(&pool, version, owner.id).await;
        add_proposal_player(&pool, bot_proposal, 0, Some(owner.id), None, "accepted").await;
        add_proposal_player(&pool, bot_proposal, 1, Some(invitee_a.id), None, "accepted").await;
        add_proposal_player(&pool, bot_proposal, 2, None, Some("Botty"), "accepted").await;
        let bot_rows = find_pending_game_summaries(&pool, owner.id).await.unwrap();
        let bot_summary = bot_rows.iter().find(|s| s.id == bot_proposal).unwrap();
        assert!(bot_summary.is_ready_to_start);
    }

    #[sqlx::test]
    async fn finished_summaries_returns_three_most_recent_in_order(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let opp = make_user(&pool, "opp").await;
        let (_, version) = make_game_type_and_version(&pool).await;

        let g1 = make_game_with_players(&pool, version, me.id, &[opp.id], 0, &[0]).await;
        let g2 = make_game_with_players(&pool, version, me.id, &[opp.id], 0, &[0]).await;
        let g3 = make_game_with_players(&pool, version, me.id, &[opp.id], 0, &[0]).await;
        let g4 = make_game_with_players(&pool, version, me.id, &[opp.id], 0, &[0]).await;

        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_finished_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();

        for (game, days) in [(&g1, 4), (&g2, 3), (&g3, 2), (&g4, 1)] {
            sqlx::query(
                "UPDATE games SET is_finished = TRUE, finished_at = timezone('utc', now()) - ($2 || ' days')::interval WHERE id = $1",
            )
            .bind(game.id)
            .bind(days.to_string())
            .execute(&pool)
            .await
            .unwrap();
        }

        let rows = find_finished_game_summaries(&pool, me.id).await.unwrap();
        let ids: Vec<Uuid> = rows.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![g4.id, g3.id, g2.id]);
    }

    #[sqlx::test]
    async fn find_game_extended_round_trips_mixed_players(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;

        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge.game.id, game.id);
        assert_eq!(ge.game_type.id, game_type_id);
        assert_eq!(ge.game_version.id, game_version_id);
        assert_eq!(ge.game_players.len(), 2);

        let human = ge.game_players.iter().find(|p| p.user.is_some()).unwrap();
        assert_eq!(human.user.as_ref().unwrap().id, creator.id);
        assert!(human.game_bot.is_none());

        let bot = ge
            .game_players
            .iter()
            .find(|p| p.game_bot.is_some())
            .unwrap();
        assert!(bot.user.is_none());

        // create_game_with_users itself inserts a game_type_users row (DB
        // column default rating 1200), so it's present here.
        assert_eq!(human.game_type_user.rating, 1200);
        assert_eq!(human.game_type_user.peak_rating, 1200);
        assert_eq!(human.game_type_user.user_id, creator.id);

        // Nonexistent game id returns Ok(None), not a panic.
        let missing = find_game_extended(&pool, Uuid::new_v4()).await.unwrap();
        assert!(missing.is_none());
    }

    #[sqlx::test]
    async fn is_player_in_game_checks_membership(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let outsider = make_user(&pool, "outsider").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;

        assert!(is_player_in_game(&pool, game.id, creator.id).await.unwrap());
        assert!(
            !is_player_in_game(&pool, game.id, outsider.id)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn find_game_extended_missing_game_type_user_defaults_to_1200(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;

        // Explicitly insert a game_type_users row for a *different* game type to
        // make sure the LEFT JOIN filter (game_type_id match) is respected, and
        // that a genuinely missing row still defaults correctly.
        let (_other_game_type_id, _) = make_game_type_and_version(&pool).await;

        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 0, &[0]).await;

        // create_game_with_users auto-creates a game_type_users row; delete it
        // to exercise the genuinely-missing-row default path in
        // build_game_type_user (rating/peak_rating default to 1200, matching
        // the DB column default).
        sqlx::query!(
            "DELETE FROM game_type_users WHERE user_id = $1 AND game_type_id = $2",
            creator.id,
            game_type_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let human = &ge.game_players[0];
        assert_eq!(human.game_type_user.rating, 1200);
        assert_eq!(human.game_type_user.peak_rating, 1200);
        assert_eq!(
            human.game_type_user.id,
            Uuid::nil(),
            "synthetic default rating row must be marked by a nil id (ws F43)"
        );
        assert_eq!(human.game_type_user.game_type_id, game_type_id);
    }

    #[sqlx::test]
    async fn find_active_game_summaries_groups_and_filters(pool: PgPool) {
        let user = make_user(&pool, "user").await;
        let other = make_user(&pool, "other").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;

        // Game 1: user is player 0 (their turn).
        let game1 =
            make_game_with_players(&pool, game_version_id, user.id, &[other.id], 0, &[0]).await;
        // Game 2: user is player 1 (opponent's turn, not user's).
        let game2 =
            make_game_with_players(&pool, game_version_id, other.id, &[user.id], 0, &[0]).await;
        // Game 3: user in a finished game - must be excluded.
        let game3 = create_game_with_users(
            &pool,
            CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[0, 1],
                points: &[1.0, 0.0],
                creator_id: user.id,
                opponent_ids: &[other.id],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "finished_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let summaries = find_active_game_summaries(&pool, user.id).await.unwrap();
        let game_ids: Vec<Uuid> = summaries.iter().map(|s| s.id).collect();

        assert!(game_ids.contains(&game1.id));
        assert!(game_ids.contains(&game2.id));
        assert!(
            !game_ids.contains(&game3.id),
            "finished games must be excluded"
        );
        assert_eq!(summaries.len(), 2, "no duplicate/mis-grouped rows");

        for s in &summaries {
            // The other human is the only opponent; the user never appears.
            assert_eq!(s.opponents.len(), 1);
            assert_eq!(s.opponents[0].name, "other");
        }

        // A user in no games gets an empty vec, not an error.
        let lonely = make_user(&pool, "lonely").await;
        let none = find_active_game_summaries(&pool, lonely.id).await.unwrap();
        assert!(none.is_empty());
    }

    #[sqlx::test]
    async fn game_logs_public_and_private_visibility_and_order(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();

        let base = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        );

        let logs = vec![
            brdgme_cmd::api::CliLog {
                content: "first public".to_string(),
                at: base,
                public: true,
                to: vec![],
            },
            brdgme_cmd::api::CliLog {
                content: "private to p0".to_string(),
                at: base + time::Duration::seconds(1),
                public: false,
                to: vec![0],
            },
            brdgme_cmd::api::CliLog {
                content: "second public".to_string(),
                at: base + time::Duration::seconds(2),
                public: true,
                to: vec![],
            },
        ];

        create_game_logs(&pool, game.id, logs).await.unwrap();

        let all_logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert_eq!(all_logs.len(), 3);
        // Ordered by logged_at ascending.
        assert_eq!(all_logs[0].body, "first public");
        assert_eq!(all_logs[1].body, "private to p0");
        assert_eq!(all_logs[2].body, "second public");

        let p0_logs = get_game_logs(&pool, game.id, p0.game_player.id)
            .await
            .unwrap();
        assert_eq!(p0_logs.len(), 3);

        let p1_logs = get_game_logs(&pool, game.id, p1.game_player.id)
            .await
            .unwrap();
        assert_eq!(
            p1_logs.len(),
            2,
            "p1 must not see the private log targeted at p0"
        );
        assert!(p1_logs.iter().all(|l| l.body != "private to p0"));
    }

    #[sqlx::test]
    async fn find_predecessor_game_id_returns_game_pointing_at_target(pool: PgPool) {
        let user = make_user(&pool, "restarter").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let old_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[]).await;
        let new_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[0]).await;

        assert_eq!(
            find_predecessor_game_id(&pool, new_game.id).await.unwrap(),
            None
        );

        sqlx::query!(
            "UPDATE games SET restarted_game_id = $1 WHERE id = $2",
            new_game.id,
            old_game.id
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            find_predecessor_game_id(&pool, new_game.id).await.unwrap(),
            Some(old_game.id)
        );
        assert_eq!(
            find_predecessor_game_id(&pool, old_game.id).await.unwrap(),
            None
        );
        assert_eq!(
            find_predecessor_game_id(&pool, Uuid::new_v4())
                .await
                .unwrap(),
            None
        );
    }

    #[sqlx::test]
    async fn finished_game_exposes_per_player_placings(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = create_game_with_users(
            &pool,
            CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[2, 1],
                points: &[],
                creator_id: creator.id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Bot 0".to_string(),
                    bot_name: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "initial_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(ge.game.is_finished);
        let by_pos: std::collections::HashMap<i32, Option<i32>> = ge
            .game_players
            .iter()
            .map(|p| (p.game_player.position, p.game_player.place))
            .collect();
        assert_eq!(by_pos[&0], Some(2));
        assert_eq!(by_pos[&1], Some(1));
    }

    /// ws F35: `find_active_turn_games` feeds the 22d switch digest and had no
    /// test. Covers oldest-turn-first ordering, the cap, and the three
    /// exclusions (not my turn, finished game, other user).
    ///
    /// Note: `game_players.is_turn_at` is `timestamp without time zone NOT
    /// NULL` (migrations/001_initial_schema.sql:193), so the query's `NULLS
    /// LAST` clause (db.rs:3112) is vestigial and cannot be exercised.
    /// Backdating `is_turn_at` alone does not disturb `is_turn`, so the
    /// `update_is_turn_at` trigger (001:454-458) does not fire and undo the
    /// fixture.
    #[sqlx::test]
    async fn find_active_turn_games_orders_oldest_turn_first_and_caps(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let me = make_user(&pool, "me").await;
        let other = make_user(&pool, "other").await;

        // Three games where it is my turn, with distinct is_turn_at values.
        // NOTE: make_game_with_players shuffles positions, so whose_turn is
        // unreliable; explicitly set is_turn for `me` after creation. The
        // update_is_turn_at trigger overwrites is_turn_at on false->true, so
        // backdate in a SECOND statement (trigger does not re-fire).
        let mut ids = Vec::new();
        for (i, day) in ["2026-01-03", "2026-01-01", "2026-01-02"]
            .iter()
            .enumerate()
        {
            let g = make_game_with_players(&pool, gv, me.id, &[other.id], 0, &[]).await;
            sqlx::query(
                "UPDATE game_players SET is_turn = true WHERE game_id = $1 AND user_id = $2",
            )
            .bind(g.id)
            .bind(me.id)
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "UPDATE game_players SET is_turn_at = $1::timestamp WHERE game_id = $2 AND user_id = $3",
            )
            .bind(format!("{day} 00:00:00"))
            .bind(g.id)
            .bind(me.id)
            .execute(&pool)
            .await
            .unwrap();
            ids.push((i, g.id, *day));
        }
        // A game where it is NOT my turn.
        let not_my_turn = make_game_with_players(&pool, gv, me.id, &[other.id], 0, &[]).await;
        sqlx::query("UPDATE game_players SET is_turn = false WHERE game_id = $1 AND user_id = $2")
            .bind(not_my_turn.id)
            .bind(me.id)
            .execute(&pool)
            .await
            .unwrap();
        // A finished game where it IS my turn.
        let finished = make_game_with_players(&pool, gv, me.id, &[other.id], 0, &[]).await;
        sqlx::query("UPDATE game_players SET is_turn = true WHERE game_id = $1 AND user_id = $2")
            .bind(finished.id)
            .bind(me.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE games SET is_finished = true WHERE id = $1")
            .bind(finished.id)
            .execute(&pool)
            .await
            .unwrap();

        let rows = find_active_turn_games(&pool, me.id, 10).await.unwrap();
        let got: Vec<Uuid> = rows.iter().map(|(g, _)| *g).collect();
        let by_day = |d: &str| ids.iter().find(|(_, _, day)| *day == d).unwrap().1;
        assert_eq!(
            got,
            vec![
                by_day("2026-01-01"),
                by_day("2026-01-02"),
                by_day("2026-01-03")
            ],
            "must be ordered by is_turn_at ascending"
        );
        assert!(
            !got.contains(&not_my_turn.id),
            "must exclude games where it is not my turn"
        );
        assert!(!got.contains(&finished.id), "must exclude finished games");

        // The returned game_player_id must be MY player row, not the opponent's.
        let (_, gp_id) = rows[0];
        let owner: Uuid = sqlx::query_scalar("SELECT user_id FROM game_players WHERE id = $1")
            .bind(gp_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(owner, me.id);

        // Cap.
        let capped = find_active_turn_games(&pool, me.id, 2).await.unwrap();
        assert_eq!(capped.len(), 2);
        assert_eq!(capped[0].0, by_day("2026-01-01"));

        // Another user sees none of my turns.
        assert!(
            find_active_turn_games(&pool, other.id, 10)
                .await
                .unwrap()
                .is_empty()
        );
    }

    /// ws F35: `mark_game_read` had no test. Only the calling user's player row
    /// may be marked, and only in the named game.
    #[sqlx::test]
    async fn mark_game_read_marks_only_the_caller_in_that_game(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let g1 = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;
        let g2 = make_game_with_players(&pool, gv, a.id, &[b.id], 0, &[0]).await;
        sqlx::query("UPDATE game_players SET is_read = false")
            .execute(&pool)
            .await
            .unwrap();

        mark_game_read(&pool, g1.id, a.id).await.unwrap();

        let read: Vec<(Uuid, Uuid, bool)> = sqlx::query_as(
            "SELECT game_id, user_id, is_read FROM game_players ORDER BY game_id, position",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        for (game_id, user_id, is_read) in read {
            let expected = game_id == g1.id && user_id == a.id;
            assert_eq!(
                is_read, expected,
                "is_read wrong for game {game_id} user {user_id} (g1={}, g2={})",
                g1.id, g2.id
            );
        }

        // Marking a game the user is not in is a no-op, not an error.
        let stranger = make_user(&pool, "stranger").await;
        mark_game_read(&pool, g1.id, stranger.id).await.unwrap();
    }

    #[sqlx::test]
    async fn find_game_version_id_for_game_returns_none_for_unknown_game(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        assert_eq!(
            find_game_version_id_for_game(&pool, Uuid::new_v4())
                .await
                .unwrap(),
            None
        );
        Ok(())
    }
}
