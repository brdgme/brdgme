//! #22b proactive game-email notifications: turn, elimination, and game-finished
//! mails. `notify_game_emails` is wired at the same call sites as
//! `trigger_bot_turns`; it diffs a before/after `GameExtended` and fires the
//! per-recipient senders for each transition. Every send is best-effort and
//! logs-only - nothing here ever fails the game operation that triggered it.
//! SSR-only like the rest of `email`.

/// The per-player Reply-To address (`g-{token}@brdg.me`) the inbound
/// webhook routes on.
pub fn reply_address(token: &str) -> String {
    format!("g-{token}@brdg.me")
}

pub fn turn_header_text(player_name: &str) -> String {
    format!("It is your turn, {player_name}.")
}

pub fn eliminated_header_text(player_name: &str) -> String {
    format!("{player_name}, you have been eliminated.")
}

/// One player's placing for the game-over header: "Alice (+16)" / "Bob (-16)" /
/// "Carl" (no rating change yet).
pub fn format_player_result(name: &str, rating_change: Option<i32>) -> String {
    match rating_change {
        Some(rc) if rc >= 0 => format!("{name} (+{rc})"),
        Some(rc) => format!("{name} ({rc})"),
        None => name.to_string(),
    }
}

pub fn finished_header_text(winners: &[(String, Option<i32>)]) -> String {
    if winners.is_empty() {
        return "Game over.".to_string();
    }
    let results: Vec<String> = winners
        .iter()
        .map(|(name, rc)| format_player_result(name, *rc))
        .collect();
    format!("Game over. Winners: {}", results.join(", "))
}

pub fn browser_url(game_id: uuid::Uuid) -> String {
    let base = crate::config::public_base_url();
    format!("{base}/games/{game_id}")
}

pub fn rules_url(version_id: uuid::Uuid) -> String {
    let base = crate::config::public_base_url();
    format!("{base}/rules/{version_id}")
}

/// The stable thread subject: "{Game type} with {opponent names}".
pub fn game_subject(
    ge: &crate::db::GameExtended,
    recipient_player: &crate::db::GamePlayerExtended,
) -> String {
    let opponent_names = ge
        .game_players
        .iter()
        .filter(|p| p.game_player.id != recipient_player.game_player.id)
        .map(|p| p.name().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} with {}", ge.game_type.name, opponent_names)
}

/// The per-turn de-threaded subject: "{Game type} {game_id}-{turn}". A unique
/// subject per turn is the reliable de-threading lever (Resend overwrites custom
/// Message-Id); primitives keep it trivially unit-testable.
pub fn turn_subject(game_type_name: &str, game_id: uuid::Uuid, turn: i64) -> String {
    format!("{game_type_name} {game_id}-{turn}")
}

/// Renders the board markup + "You can" command usages for `position`'s view of
/// `ge`, best-effort: a failed game-service render degrades to absent blocks
/// rather than failing the caller.
pub async fn render_board_and_you_can(
    http_client: &reqwest::Client,
    ge: &crate::db::GameExtended,
    position: usize,
) -> (Option<String>, Option<Vec<String>>) {
    let render_resp = crate::game::client::render(
        http_client,
        &ge.game_version.uri,
        &ge.game_version.name,
        ge.game.game_state.clone(),
        Some(position),
    )
    .await;
    match render_resp {
        Ok(resp) => {
            let you_can = resp.command_spec.as_ref().map(|spec| {
                let nodes = brdgme_game::command::doc::render(&spec.doc());
                let s = brdgme_markup::to_string(&nodes);
                s.split('\n')
                    .filter(|l| !l.is_empty())
                    .map(String::from)
                    .collect()
            });
            (Some(resp.render), you_can)
        }
        Err(e) => {
            tracing::error!("Failed to render game {}: {}", ge.game.id, e);
            (None, None)
        }
    }
}

fn now_utc() -> time::PrimitiveDateTime {
    let t = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(t.date(), t.time())
}

enum NotifyKind {
    Turn,
    Eliminated,
    Finished,
}

enum SendMode {
    Normal,
    BypassSuppression,
    Forced,
}

/// The "Since last time" digest lines for one recipient: `get_game_logs` already
/// filters to public + this player's targeted logs, so we keep only those newer
/// than the recipient's `last_turn_at`. Best-effort: `None` on error or when
/// there are no new lines.
async fn digest_since_last_turn(
    pool: &sqlx::PgPool,
    ge: &crate::db::GameExtended,
    recipient_player: &crate::db::GamePlayerExtended,
) -> Option<Vec<String>> {
    match crate::db::get_game_logs(pool, ge.game.id, recipient_player.game_player.id).await {
        Ok(logs) => {
            let lines: Vec<String> = logs
                .into_iter()
                .filter(|l| l.logged_at > recipient_player.game_player.last_turn_at)
                .map(|l| l.body)
                .collect();
            if lines.is_empty() { None } else { Some(lines) }
        }
        Err(e) => {
            tracing::error!("Failed to load game logs for {}: {}", ge.game.id, e);
            None
        }
    }
}

/// Builds the content blocks for one notification, best-effort: a failed render
/// or log load degrades to absent blocks rather than failing the send.
async fn build_content(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    ge: &crate::db::GameExtended,
    recipient_player: &crate::db::GamePlayerExtended,
    kind: NotifyKind,
    subject: String,
) -> crate::email::render::EmailContent {
    let header = Some(match kind {
        NotifyKind::Turn => turn_header_text(recipient_player.name()),
        NotifyKind::Eliminated => eliminated_header_text(recipient_player.name()),
        NotifyKind::Finished => {
            let mut placed: Vec<&crate::db::GamePlayerExtended> = ge.game_players.iter().collect();
            placed.sort_by_key(|p| p.game_player.place.unwrap_or(i32::MAX));
            let winners: Vec<(String, Option<i32>)> = placed
                .iter()
                .map(|p| (p.name().to_string(), p.game_player.rating_change))
                .collect();
            finished_header_text(&winners)
        }
    });

    let digest = digest_since_last_turn(pool, ge, recipient_player).await;

    let (board, you_can) = render_board_and_you_can(
        http_client,
        ge,
        recipient_player.game_player.position as usize,
    )
    .await;

    crate::email::render::EmailContent {
        subject,
        header,
        digest,
        board,
        you_can,
        browser_url: Some(browser_url(ge.game.id)),
        rules_url: Some(rules_url(ge.game_version.id)),
        footer: Some("Reply to this email to play, or unsubscribe anytime.".to_string()),
    }
}

/// Builds the content for an inbound command-failure report: the standard turn
/// email body (current render, "Since last time" logs, command spec, footers)
/// reflecting the game state AFTER the successfully-applied commands, with the
/// caller's failure `header` on top. Uses the per-turn de-threaded subject
/// scheme (`turn_subject`) so clients do not collapse the render into a thread.
pub async fn failure_report_content(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    ge: &crate::db::GameExtended,
    recipient_player: &crate::db::GamePlayerExtended,
    header: String,
) -> crate::email::render::EmailContent {
    let digest = digest_since_last_turn(pool, ge, recipient_player).await;
    let (board, you_can) = render_board_and_you_can(
        http_client,
        ge,
        recipient_player.game_player.position as usize,
    )
    .await;
    let log_count = game_log_count(pool, ge.game.id).await;
    crate::email::render::EmailContent {
        subject: turn_subject(&ge.game_type.name, ge.game.id, log_count),
        header: Some(header),
        digest,
        board,
        you_can,
        browser_url: Some(browser_url(ge.game.id)),
        rules_url: Some(rules_url(ge.game_version.id)),
        footer: Some("Reply to this email to play, or unsubscribe anytime.".to_string()),
    }
}

/// How many logs the game has (plain query; defaults to 0 on error). Every
/// command appends >=1 log, so this is a monotonic turn counter used both to
/// detect the opening turn and to build the per-turn de-threaded subject.
async fn game_log_count(pool: &sqlx::PgPool, game_id: uuid::Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM game_logs WHERE game_id = $1")
        .bind(game_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

async fn send_one(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
    kind: NotifyKind,
    mode: SendMode,
) {
    let ge = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            tracing::warn!("notify: game {} not found", game_id);
            return;
        }
        Err(e) => {
            tracing::error!("notify: failed to load game {}: {}", game_id, e);
            return;
        }
    };

    let recipient_player = match ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == game_player_id)
    {
        Some(p) => p,
        None => {
            tracing::warn!("notify: player {} not in game {}", game_player_id, game_id);
            return;
        }
    };

    let recipient = match crate::email::outbound::fetch_email_recipient(pool, game_player_id).await
    {
        Ok(Some(r)) => r,
        _ => return,
    };

    let should_send = match mode {
        SendMode::Forced => recipient.email.is_some() && !recipient.is_bot,
        SendMode::BypassSuppression => {
            recipient.email.is_some() && !recipient.is_bot && recipient.turn_emails_enabled
        }
        SendMode::Normal => crate::email::outbound::should_email_recipient(
            &recipient,
            now_utc(),
            crate::email::outbound::suppress_window(),
        ),
    };
    if !should_send {
        return;
    }

    let token = match crate::email::outbound::ensure_email_token(pool, game_player_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "notify: failed to ensure email token for {}: {}",
                game_player_id,
                e
            );
            return;
        }
    };

    let palette = crate::email::render::palette_for_slug(recipient.theme_slug.as_deref());
    let players: Vec<brdgme_markup::Player> = ge
        .game_players
        .iter()
        .map(|p| crate::email::render::player_for_slot(p.name(), &p.game_player.color, palette))
        .collect();

    let log_count = game_log_count(pool, game_id).await;
    let is_first_message = log_count == 0;
    let (subject, thread_id) = match &kind {
        NotifyKind::Turn => (
            turn_subject(&ge.game_type.name, ge.game.id, log_count),
            None,
        ),
        NotifyKind::Eliminated | NotifyKind::Finished => (
            game_subject(&ge, recipient_player),
            Some(format!("game-{game_id}")),
        ),
    };

    let content = build_content(pool, http_client, &ge, recipient_player, kind, subject).await;

    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &players,
        thread_id.as_deref(),
        is_first_message,
        &reply_address(&token),
    );

    let to = match recipient.email.clone() {
        Some(e) => e,
        None => return,
    };
    crate::email::outbound::send_rendered_email(resend, rendered, &to).await;
}

pub async fn send_turn_notification(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
) {
    send_one(
        resend,
        pool,
        http_client,
        game_id,
        game_player_id,
        NotifyKind::Turn,
        SendMode::Normal,
    )
    .await;
}

/// Sends a turn notification for one game, bypassing the active-web suppression
/// window. Used by the 22d switch-digest: the user just changed their active
/// address on the web (so they ARE recently active) yet explicitly wants their
/// actionable games re-sent to the new address. Still respects the bot check
/// and the account `turn_emails_enabled` opt-out.
pub async fn send_turn_digest(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
) {
    send_one(
        resend,
        pool,
        http_client,
        game_id,
        game_player_id,
        NotifyKind::Turn,
        SendMode::BypassSuppression,
    )
    .await;
}

/// Sends a turn notification for one game, bypassing BOTH the active-web
/// suppression window AND the account `turn_emails_enabled` opt-out. Used by
/// the email `bump` command: an explicit user pull, so it always re-sends.
/// Still requires a verified primary address and is never sent to a bot.
pub async fn send_turn_digest_forced(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
) {
    send_one(
        resend,
        pool,
        http_client,
        game_id,
        game_player_id,
        NotifyKind::Turn,
        SendMode::Forced,
    )
    .await;
}

pub async fn send_elimination_notification(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
) {
    send_one(
        resend,
        pool,
        http_client,
        game_id,
        game_player_id,
        NotifyKind::Eliminated,
        SendMode::Normal,
    )
    .await;
}

pub async fn send_game_finished_notification(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
) {
    send_one(
        resend,
        pool,
        http_client,
        game_id,
        game_player_id,
        NotifyKind::Finished,
        SendMode::Normal,
    )
    .await;
}

/// Diffs `before`/`after` game state and fires the appropriate notification for
/// each human player. Mail failures are isolated: every send logs and returns;
/// this never fails the game operation.
pub async fn notify_game_emails(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    before: Option<crate::db::GameExtended>,
) {
    let after = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(a)) => a,
        _ => return,
    };

    let was_finished = before.as_ref().map(|b| b.game.is_finished).unwrap_or(false);
    if after.game.is_finished && !was_finished {
        for p in after
            .game_players
            .iter()
            .filter(|p| p.user.is_some() && p.game_bot.is_none())
        {
            send_game_finished_notification(resend, pool, http_client, game_id, p.game_player.id)
                .await;
        }
        return;
    }

    for p in after
        .game_players
        .iter()
        .filter(|p| p.user.is_some() && p.game_bot.is_none())
    {
        let before_player = before.as_ref().and_then(|b| {
            b.game_players
                .iter()
                .find(|bp| bp.game_player.position == p.game_player.position)
        });
        let was_turn = before_player
            .map(|b| b.game_player.is_turn)
            .unwrap_or(false);
        if p.game_player.is_turn && !was_turn {
            send_turn_notification(resend, pool, http_client, game_id, p.game_player.id).await;
        }
        let was_elim = before_player
            .map(|b| b.game_player.is_eliminated)
            .unwrap_or(false);
        if p.game_player.is_eliminated && !was_elim {
            send_elimination_notification(resend, pool, http_client, game_id, p.game_player.id)
                .await;
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn reply_address_formats_token() {
        assert_eq!(reply_address("tok"), "g-tok@brdg.me");
    }

    #[test]
    fn turn_subject_is_name_id_turn_and_unique_per_turn() {
        let id = uuid::Uuid::new_v4();
        assert_eq!(turn_subject("Acquire", id, 12), format!("Acquire {id}-12"));
        assert_ne!(
            turn_subject("Acquire", id, 12),
            turn_subject("Acquire", id, 13)
        );
    }

    #[test]
    fn format_player_result_formats_rating_change() {
        assert_eq!(format_player_result("Alice", Some(16)), "Alice (+16)");
        assert_eq!(format_player_result("Bob", Some(-16)), "Bob (-16)");
        assert_eq!(format_player_result("Carl", None), "Carl");
    }

    #[test]
    fn finished_header_text_lists_winners() {
        let winners = vec![
            ("Alice".to_string(), Some(16)),
            ("Bob".to_string(), Some(-16)),
        ];
        assert_eq!(
            finished_header_text(&winners),
            "Game over. Winners: Alice (+16), Bob (-16)"
        );
        assert_eq!(finished_header_text(&[]), "Game over.");
    }

    #[test]
    fn turn_and_eliminated_headers_contain_name() {
        let turn = turn_header_text("Alice");
        assert!(!turn.is_empty());
        assert!(turn.contains("Alice"));
        let elim = eliminated_header_text("Bob");
        assert!(!elim.is_empty());
        assert!(elim.contains("Bob"));
    }

    #[test]
    fn browser_url_contains_game_path() {
        let id = uuid::Uuid::new_v4();
        let url = browser_url(id);
        assert!(url.ends_with(&format!("/games/{id}")));
    }

    #[test]
    fn rules_url_contains_rules_path() {
        let id = uuid::Uuid::new_v4();
        let url = rules_url(id);
        assert!(url.ends_with(&format!("/rules/{id}")));
    }

    // Runs only where a Postgres is available (CI); expected to fail to connect
    // locally (backlog #40). Missing game -> early return, must not panic.
    #[sqlx::test]
    async fn notify_game_emails_noop_for_missing_game(pool: sqlx::PgPool) {
        notify_game_emails(
            None,
            &pool,
            &reqwest::Client::new(),
            uuid::Uuid::new_v4(),
            None,
        )
        .await;
    }
}
