//! #22b proactive game-email notifications: turn, elimination, and game-finished
//! mails. `notify_game_emails` is wired at the same call sites as
//! `trigger_bot_turns`; it diffs a before/after `GameExtended` and fires the
//! per-recipient senders for each transition. Every send is best-effort and
//! logs-only - nothing here ever fails the game operation that triggered it.
//! SSR-only like the rest of `email`.

/// The single inbound reply domain. Every reply address the app advertises
/// (`g-`/`i-`/`s-`) is on this domain (wfe F14). Deliberately NOT the
/// Message-Id domain (`render.rs`) or the unsubscribe mailto domain, which are
/// different concepts that merely happen to share the string today.
pub const REPLY_DOMAIN: &str = "brdg.me";

/// The per-player game reply address (`g-{token}@brdg.me`) the inbound webhook
/// routes on.
pub fn reply_address(token: &str) -> String {
    format!("g-{token}@{REPLY_DOMAIN}")
}

/// The per-invitee proposal reply address (`i-{token}@brdg.me`).
pub fn invite_reply_address(token: &str) -> String {
    format!("i-{token}@{REPLY_DOMAIN}")
}

/// The per-user settings reply address (`s-{token}@brdg.me`), used for
/// replies that carry no game or invite context.
pub fn settings_reply_address(token: &str) -> String {
    format!("s-{token}@{REPLY_DOMAIN}")
}

pub fn turn_header_text(player_name: &str) -> String {
    format!("It is your turn, {player_name}.")
}

pub fn reminder_header_text(player_name: &str) -> String {
    format!("Still your turn, {player_name}.")
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

enum NotifyKind {
    Turn,
    /// The turn-reminder sweep's nudge for a turn already notified. Shares
    /// `Turn`'s de-threaded per-turn subject so it groups with the mail it
    /// nudges (wfe F39).
    Reminder,
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
    let header = Some(match &kind {
        NotifyKind::Turn => turn_header_text(recipient_player.name()),
        NotifyKind::Reminder => reminder_header_text(recipient_player.name()),
        NotifyKind::Eliminated => eliminated_header_text(recipient_player.name()),
        NotifyKind::Finished => {
            // A last-human-stop game has no result: never derive a winner or
            // rating-delta line from place/points/placing/rating (DRM-03c2a).
            if ge.game.end_reason.as_deref() == Some("last_human_stop") {
                "Game ended early. No game result.".to_string()
            } else {
                let mut placed: Vec<&crate::db::GamePlayerExtended> =
                    ge.game_players.iter().collect();
                placed.sort_by_key(|p| p.game_player.place.unwrap_or(i32::MAX));
                let winners: Vec<(String, Option<i32>)> = placed
                    .iter()
                    .map(|p| (p.name().to_string(), p.game_player.rating_change))
                    .collect();
                finished_header_text(&winners)
            }
        }
    });

    // A reminder carries no digest: the turn email it nudges already showed
    // those lines and the recipient has not moved since (wfe F36).
    let digest = match &kind {
        NotifyKind::Reminder => None,
        _ => digest_since_last_turn(pool, ge, recipient_player).await,
    };

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
        subject: turn_subject_or_fallback(&ge.game_type.name, ge.game.id, log_count),
        header: Some(header),
        digest,
        board,
        you_can,
        browser_url: Some(browser_url(ge.game.id)),
        rules_url: Some(rules_url(ge.game_version.id)),
        footer: Some("Reply to this email to play, or unsubscribe anytime.".to_string()),
    }
}

/// How many logs the game has, or `None` when the count could not be read.
/// Every command appends >=1 log, so this is a monotonic turn counter used both
/// to detect the opening turn and to build the per-turn de-threaded subject. A
/// failed read must NOT collapse to `0`: that gave every affected turn email the
/// identical subject and made it claim to be the game's first message, breaking
/// the de-threading lever documented above (wfe F41).
pub(crate) async fn game_log_count(pool: &sqlx::PgPool, game_id: uuid::Uuid) -> Option<i64> {
    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM game_logs WHERE game_id = $1")
        .bind(game_id)
        .fetch_one(pool)
        .await
    {
        Ok(n) => Some(n),
        Err(e) => {
            tracing::error!("notify: log count failed for game {}: {}", game_id, e);
            None
        }
    }
}

/// The per-turn de-threaded subject, with a timestamped fallback when the turn
/// counter is unavailable. Uniqueness per turn is the one property this subject
/// may never lose (wfe F41).
pub fn turn_subject_or_fallback(
    game_type_name: &str,
    game_id: uuid::Uuid,
    turn: Option<i64>,
) -> String {
    match turn {
        Some(t) => turn_subject(game_type_name, game_id, t),
        None => format!(
            "{game_type_name} {game_id}-t{}",
            time::OffsetDateTime::now_utc().unix_timestamp()
        ),
    }
}

/// What one notification attempt did. `Suppressed` covers every deliberate
/// non-send: opted out, bot slot, no verified address, or active on the web.
/// Every caller is best-effort and drops it.
pub enum SendResult {
    Sent,
    Suppressed,
    Failed,
}

/// Sends one notification for an ALREADY-LOADED game. `log_count` is that
/// game's log count (`None` = it could not be read). Split out of `send_one` so
/// `notify_game_emails` can load the game and the count once for the whole
/// roster instead of once per recipient (wfe F43).
#[allow(clippy::too_many_arguments)]
async fn send_one_loaded(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    ge: &crate::db::GameExtended,
    log_count: Option<i64>,
    game_player_id: uuid::Uuid,
    kind: NotifyKind,
    mode: SendMode,
) -> SendResult {
    let game_id = ge.game.id;

    let recipient_player = match ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == game_player_id)
    {
        Some(p) => p,
        None => {
            tracing::warn!("notify: player {} not in game {}", game_player_id, game_id);
            return SendResult::Failed;
        }
    };

    let recipient = match crate::email::outbound::fetch_email_recipient(pool, game_player_id).await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            tracing::warn!("notify: no recipient row for player {}", game_player_id);
            return SendResult::Failed;
        }
        Err(e) => {
            tracing::error!("notify: failed to load recipient {}: {}", game_player_id, e);
            return SendResult::Failed;
        }
    };

    let should_send = match mode {
        SendMode::Forced => recipient.email.is_some() && !recipient.is_bot,
        SendMode::BypassSuppression => {
            recipient.email.is_some() && !recipient.is_bot && recipient.turn_emails_enabled
        }
        SendMode::Normal => {
            crate::email::outbound::should_email_recipient(&recipient)
                && !crate::email::outbound::suppress_for_web_presence(pool, recipient.user_id).await
        }
    };
    if !should_send {
        return SendResult::Suppressed;
    }

    let email_kind = match &kind {
        NotifyKind::Turn => crate::email::render::EmailKind::Turn,
        NotifyKind::Reminder => crate::email::render::EmailKind::Reminder,
        NotifyKind::Eliminated | NotifyKind::Finished => crate::email::render::EmailKind::GameEvent,
    };

    let token = match crate::email::outbound::ensure_email_token(pool, game_player_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "notify: failed to ensure email token for {}: {}",
                game_player_id,
                e
            );
            return SendResult::Failed;
        }
    };

    let palette = crate::email::render::palette_for_slug(recipient.theme_slug.as_deref());
    let players: Vec<brdgme_markup::Player> = ge
        .game_players
        .iter()
        .map(|p| crate::email::render::player_for_slot(p.name(), &p.game_player.color, palette))
        .collect();

    let is_first_message = log_count == Some(0);
    let (subject, thread_id) = match &kind {
        NotifyKind::Turn | NotifyKind::Reminder => (
            turn_subject_or_fallback(&ge.game_type.name, game_id, log_count),
            None,
        ),
        NotifyKind::Eliminated | NotifyKind::Finished => (
            game_subject(ge, recipient_player),
            Some(format!("game-{game_id}")),
        ),
    };

    let content = build_content(pool, http_client, ge, recipient_player, kind, subject).await;

    let unsub_token: Option<String> = match recipient.user_id {
        Some(uid) => match crate::email::outbound::ensure_unsubscribe_token(pool, uid).await {
            Ok(tok) => Some(tok),
            Err(err) => {
                tracing::warn!(
                    "notify: unsubscribe token fetch failed for {}: {}",
                    uid,
                    err
                );
                None
            }
        },
        None => None,
    };
    let unsubscribe = unsub_token
        .as_ref()
        .map(|tok| crate::email::render::Unsubscribe {
            kind: email_kind,
            token: tok,
        });

    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &players,
        thread_id.as_deref(),
        is_first_message,
        &reply_address(&token),
        unsubscribe,
    );

    // Unreachable: every `should_send` arm above requires `email.is_some()`.
    let to = match recipient.email.clone() {
        Some(e) => e,
        None => return SendResult::Suppressed,
    };
    if crate::email::outbound::try_send_rendered_email(resend, rendered, &to).await {
        SendResult::Sent
    } else {
        SendResult::Failed
    }
}

/// Loads the game and its log count, then sends one notification. Use
/// `send_one_loaded` directly when a caller already holds both (wfe F43).
async fn send_one(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
    kind: NotifyKind,
    mode: SendMode,
) -> SendResult {
    let ge = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            tracing::warn!("notify: game {} not found", game_id);
            return SendResult::Failed;
        }
        Err(e) => {
            tracing::error!("notify: failed to load game {}: {}", game_id, e);
            return SendResult::Failed;
        }
    };
    let log_count = game_log_count(pool, game_id).await;
    send_one_loaded(
        resend,
        pool,
        http_client,
        &ge,
        log_count,
        game_player_id,
        kind,
        mode,
    )
    .await
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
    let _ = send_one(
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
    let _ = send_one(
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

/// Notifies every human player currently on turn that a brand-new game has
/// started. Unlike `notify_game_emails` (which uses `SendMode::Normal`), this
/// bypasses web-presence suppression: the user explicitly created or started
/// the game, so the confirmation must not be swallowed by the hydrated-page
/// presence window (R-20 / F-180). Still respects `turn_emails_enabled` and
/// the bot/address checks via `SendMode::BypassSuppression`.
pub async fn notify_game_started(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
) {
    let after = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("notify: failed to load game {}: {}", game_id, e);
            return;
        }
    };
    let log_count = game_log_count(pool, game_id).await;
    for p in after
        .game_players
        .iter()
        .filter(|p| p.user.is_some() && p.game_bot.is_none())
    {
        if p.game_player.is_turn {
            send_one_loaded(
                resend,
                pool,
                http_client,
                &after,
                log_count,
                p.game_player.id,
                NotifyKind::Turn,
                SendMode::BypassSuppression,
            )
            .await;
        }
    }
}

/// Diffs `before`/`after` game state and fires the appropriate notification for
/// each human player. Mail failures are isolated: every send logs and returns;
/// this never fails the game operation.
///
/// `before` is the pre-command snapshot to diff against; `game::execute_command`
/// returns it. `None` means **brand-new game** - nobody has been notified yet,
/// so every player currently on turn counts as newly on turn. NEVER pass `None`
/// to mean "I could not read the snapshot": that re-notifies every player
/// already on turn (wfe F42).
pub async fn notify_game_emails(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    before: Option<crate::db::GameExtended>,
) {
    let after = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("notify: failed to load game {}: {}", game_id, e);
            return;
        }
    };
    // One load and one count for the whole roster (wfe F43).
    let log_count = game_log_count(pool, game_id).await;

    let was_finished = before.as_ref().map(|b| b.game.is_finished).unwrap_or(false);
    if after.game.is_finished && !was_finished {
        for p in after
            .game_players
            .iter()
            .filter(|p| p.user.is_some() && p.game_bot.is_none())
        {
            send_one_loaded(
                resend,
                pool,
                http_client,
                &after,
                log_count,
                p.game_player.id,
                NotifyKind::Finished,
                SendMode::Normal,
            )
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
            send_one_loaded(
                resend,
                pool,
                http_client,
                &after,
                log_count,
                p.game_player.id,
                NotifyKind::Turn,
                SendMode::Normal,
            )
            .await;
        }
        let was_elim = before_player
            .map(|b| b.game_player.is_eliminated)
            .unwrap_or(false);
        if p.game_player.is_eliminated && !was_elim {
            send_one_loaded(
                resend,
                pool,
                http_client,
                &after,
                log_count,
                p.game_player.id,
                NotifyKind::Eliminated,
                SendMode::Normal,
            )
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
        assert_eq!(invite_reply_address("tok"), "i-tok@brdg.me");
        assert_eq!(settings_reply_address("tok"), "s-tok@brdg.me");
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
    fn turn_subject_fallback_stays_unique_when_the_count_is_unknown() {
        // wfe F41: a failed count must not collapse every turn onto
        // "{type} {id}-0" - that is the de-threading lever at :68-73.
        let id = uuid::Uuid::new_v4();
        assert_eq!(
            turn_subject_or_fallback("Acquire", id, Some(7)),
            turn_subject("Acquire", id, 7)
        );
        let fallback = turn_subject_or_fallback("Acquire", id, None);
        assert_ne!(fallback, turn_subject("Acquire", id, 0));
        assert!(fallback.starts_with(&format!("Acquire {id}-t")));
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

    // DRM-03c2a: a last-human-stop game has no result. The finished mail's
    // header must be the exact no-result line and must not derive any
    // winner/result language from the players' place, points, ranked placing,
    // or rating change.
    #[sqlx::test]
    async fn finished_last_human_stop_header_has_no_game_result(pool: sqlx::PgPool) {
        use axum::{Json, Router, routing::post};
        use brdgme_cmd::api::{PlayerRender as PlayerRenderApi, Request, Response};
        use tokio::net::TcpListener;

        let app = Router::new().route(
            "/",
            post(move |Json(_): Json<Request>| async {
                Json(Response::PlayerRender {
                    render: PlayerRenderApi {
                        player_state: "state".to_string(),
                        render: "mock board".to_string(),
                        command_spec: None,
                    },
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mock_uri = format!("http://{addr}");

        let (game_id, _players) = seed_game_with_emailable_players(&pool, 2).await;
        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        sqlx::query(
            "UPDATE games SET is_finished = true, end_reason = 'last_human_stop' \
             WHERE id = $1",
        )
        .bind(game_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE game_versions SET uri = $1, name = $2 WHERE id = $3")
            .bind(&mock_uri)
            .bind(format!("notify-mock-{}", uuid::Uuid::new_v4().simple()))
            .bind(ge.game.game_version_id)
            .execute(&pool)
            .await
            .unwrap();
        // Distinct result data on every player: the winner/placing header
        // would have surfaced one of these.
        sqlx::query(
            "UPDATE game_players SET \
               place = position, \
               points = (position + 1)::real, \
               ranked_placing = position + 1, \
               rating_change = CASE WHEN position = 0 THEN 16 ELSE -16 END \
             WHERE game_id = $1",
        )
        .bind(game_id)
        .execute(&pool)
        .await
        .unwrap();

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        let content = build_content(
            &pool,
            &reqwest::Client::new(),
            &ge,
            &ge.game_players[0],
            NotifyKind::Finished,
            "subject".to_string(),
        )
        .await;
        let header = content.header.as_deref().expect("finished mail has a header");
        assert_eq!(header, "Game ended early. No game result.");
        assert!(!header.contains("Winners"));
        for p in &ge.game_players {
            assert!(!header.contains(p.name()), "header must not name {}", p.name());
        }
    }

    // DRM-03c2b: a normal game-service finish derives winner/order only from
    // authoritative `game_players.place`, never from `points` or
    // `ranked_placing`, even when those would imply a different result.
    #[sqlx::test]
    async fn finished_game_service_header_uses_authoritative_place(pool: sqlx::PgPool) {
        use axum::{Json, Router, routing::post};
        use brdgme_cmd::api::{PlayerRender as PlayerRenderApi, Request, Response};
        use tokio::net::TcpListener;

        let app = Router::new().route(
            "/",
            post(move |Json(_): Json<Request>| async {
                Json(Response::PlayerRender {
                    render: PlayerRenderApi {
                        player_state: "state".to_string(),
                        render: "mock board".to_string(),
                        command_spec: None,
                    },
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mock_uri = format!("http://{addr}");

        let (game_id, _players) = seed_game_with_emailable_players(&pool, 2).await;
        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        sqlx::query(
            "UPDATE games SET is_finished = true, end_reason = 'game_service' \
             WHERE id = $1",
        )
        .bind(game_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE game_versions SET uri = $1, name = $2 WHERE id = $3")
            .bind(&mock_uri)
            .bind(format!("notify-mock-{}", uuid::Uuid::new_v4().simple()))
            .bind(ge.game.game_version_id)
            .execute(&pool)
            .await
            .unwrap();
        // Authoritative place says position 1 won; points and ranked_placing
        // both say position 0 won. The header must follow `place`.
        sqlx::query(
            "UPDATE game_players SET \
               place = CASE WHEN position = 0 THEN 2 ELSE 1 END, \
               points = CASE WHEN position = 0 THEN 100 ELSE 1 END, \
               ranked_placing = CASE WHEN position = 0 THEN 1 ELSE 2 END, \
               rating_change = CASE WHEN position = 0 THEN 16 ELSE -16 END \
             WHERE game_id = $1",
        )
        .bind(game_id)
        .execute(&pool)
        .await
        .unwrap();

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        let content = build_content(
            &pool,
            &reqwest::Client::new(),
            &ge,
            &ge.game_players[0],
            NotifyKind::Finished,
            "subject".to_string(),
        )
        .await;
        let header = content.header.as_deref().expect("finished mail has a header");
        let p0 = &ge.game_players[0];
        let p1 = &ge.game_players[1];
        assert_eq!(
            header,
            format!(
                "Game over. Winners: {} (-16), {} (+16)",
                p1.name(),
                p0.name()
            ),
            "winner/order must come from authoritative place, not points or ranked_placing"
        );
        assert!(
            !header.starts_with(&format!("Game over. Winners: {}", p0.name())),
            "points/ranked_placing must not promote position 0 over its authoritative place 2"
        );
    }

    // DRM-03c2b: a two-human concession finish keeps the authoritative and
    // competitive 1/2 places even when points or ranked_placing conflict with
    // that order.
    #[sqlx::test]
    async fn finished_two_human_concession_header_uses_authoritative_places(pool: sqlx::PgPool) {
        use axum::{Json, Router, routing::post};
        use brdgme_cmd::api::{PlayerRender as PlayerRenderApi, Request, Response};
        use tokio::net::TcpListener;

        let app = Router::new().route(
            "/",
            post(move |Json(_): Json<Request>| async {
                Json(Response::PlayerRender {
                    render: PlayerRenderApi {
                        player_state: "state".to_string(),
                        render: "mock board".to_string(),
                        command_spec: None,
                    },
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mock_uri = format!("http://{addr}");

        let (game_id, _players) = seed_game_with_emailable_players(&pool, 2).await;
        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        sqlx::query(
            "UPDATE games SET is_finished = true, end_reason = 'concession_forfeit' \
             WHERE id = $1",
        )
        .bind(game_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE game_versions SET uri = $1, name = $2 WHERE id = $3")
            .bind(&mock_uri)
            .bind(format!("notify-mock-{}", uuid::Uuid::new_v4().simple()))
            .bind(ge.game.game_version_id)
            .execute(&pool)
            .await
            .unwrap();
        // Authoritative/competitive places are 1/2 in position order, but
        // points and ranked_placing both name position 1 as winner. The header
        // must retain the 1/2 forfeit order.
        sqlx::query(
            "UPDATE game_players SET \
               place = position + 1, \
               points = CASE WHEN position = 0 THEN 1 ELSE 100 END, \
               ranked_placing = CASE WHEN position = 0 THEN 2 ELSE 1 END, \
               rating_change = CASE WHEN position = 0 THEN 8 ELSE -8 END \
             WHERE game_id = $1",
        )
        .bind(game_id)
        .execute(&pool)
        .await
        .unwrap();

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        let content = build_content(
            &pool,
            &reqwest::Client::new(),
            &ge,
            &ge.game_players[0],
            NotifyKind::Finished,
            "subject".to_string(),
        )
        .await;
        let header = content.header.as_deref().expect("finished mail has a header");
        let p0 = &ge.game_players[0];
        let p1 = &ge.game_players[1];
        assert_eq!(
            header,
            format!(
                "Game over. Winners: {} (+8), {} (-8)",
                p0.name(),
                p1.name()
            ),
            "two-human concession must keep authoritative/competitive 1/2 order"
        );
    }

    #[test]
    fn reminder_header_contains_name() {
        let h = reminder_header_text("Alice");
        assert!(h.contains("Alice"));
        assert!(h.contains("Still your turn"));
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

    /// Builds one game with `n` human players, each with a verified primary
    /// address and `turn_emails_enabled`, returning the game id and each
    /// `(user_id, game_player_id)`.
    async fn seed_game_with_emailable_players(
        pool: &sqlx::PgPool,
        n: usize,
    ) -> (uuid::Uuid, Vec<(uuid::Uuid, uuid::Uuid)>) {
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Notify {}", uuid::Uuid::new_v4()))
        .bind(vec![2, 3, 4])
        .fetch_one(pool)
        .await
        .unwrap();
        let game_version_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://127.0.0.1:1', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let game_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO games (game_version_id, is_finished, game_state)
             VALUES ($1, false, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let colors = ["Green", "Red", "Blue", "Yellow", "Purple"];
        let mut players = Vec::new();
        for (i, color) in colors.iter().take(n).enumerate() {
            let user_id: uuid::Uuid = sqlx::query_scalar(
                "INSERT INTO users (name, pref_colors, turn_emails_enabled)
                 VALUES ($1, $2, true) RETURNING id",
            )
            .bind(format!("u-{}", uuid::Uuid::new_v4()))
            .bind(Vec::<String>::new())
            .fetch_one(pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
                 VALUES ($1, $2, true, NOW())",
            )
            .bind(user_id)
            .bind(format!("u-{}@example.com", uuid::Uuid::new_v4()))
            .execute(pool)
            .await
            .unwrap();
            let gp_id: uuid::Uuid = sqlx::query_scalar(
                "INSERT INTO game_players
                     (game_id, user_id, position, color, has_accepted, is_turn,
                      is_turn_at, last_turn_at, is_eliminated, is_read)
                 VALUES ($1, $2, $3, $4, true, false, NOW(), NOW(), false, false)
                 RETURNING id",
            )
            .bind(game_id)
            .bind(user_id)
            .bind(i as i32)
            .bind(color)
            .fetch_one(pool)
            .await
            .unwrap();
            players.push((user_id, gp_id));
        }
        (game_id, players)
    }

    async fn email_token(pool: &sqlx::PgPool, game_player_id: uuid::Uuid) -> Option<String> {
        sqlx::query_scalar("SELECT email_token FROM game_players WHERE id = $1")
            .bind(game_player_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // Per-recipient web-presence suppression: in the same game, the active
    // player's automated turn email is skipped (no reply token minted) while the
    // inactive player's still goes out.
    #[sqlx::test]
    async fn turn_notification_suppressed_per_recipient_by_presence(pool: sqlx::PgPool) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 2).await;
        let (active_user, active_gp) = players[0];
        let (_inactive_user, inactive_gp) = players[1];
        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(active_user)
            .execute(&pool)
            .await
            .unwrap();

        let http = reqwest::Client::new();
        // The old per-kind wrapper is gone (wfe F43); the tests exercise the
        // real path directly.
        send_one(
            None,
            &pool,
            &http,
            game_id,
            active_gp,
            NotifyKind::Turn,
            SendMode::Normal,
        )
        .await;
        send_one(
            None,
            &pool,
            &http,
            game_id,
            inactive_gp,
            NotifyKind::Turn,
            SendMode::Normal,
        )
        .await;

        assert!(
            email_token(&pool, active_gp).await.is_none(),
            "active player's automated turn email should be suppressed"
        );
        assert!(
            email_token(&pool, inactive_gp).await.is_some(),
            "inactive player's automated turn email should still send"
        );
    }

    // wfe F43 lock-in: one loaded snapshot drives the whole roster, and the
    // diff still only mails the player who is NEWLY on turn.
    #[sqlx::test]
    async fn notify_game_emails_only_mails_the_newly_on_turn_player(pool: sqlx::PgPool) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 2).await;
        let (_u0, gp0) = players[0];
        let (_u1, gp1) = players[1];
        let before = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(gp1)
            .execute(&pool)
            .await
            .unwrap();

        notify_game_emails(None, &pool, &reqwest::Client::new(), game_id, Some(before)).await;

        // A reply token is minted only for a recipient we actually mail.
        assert!(
            email_token(&pool, gp1).await.is_some(),
            "the newly-on-turn player must be mailed"
        );
        assert!(
            email_token(&pool, gp0).await.is_none(),
            "a player whose turn state did not change must not be mailed"
        );
    }

    // wfe F42 companion: `None` means brand-new game, so everyone currently on
    // turn is newly on turn. This is the ONLY intended meaning of `None`.
    #[sqlx::test]
    async fn notify_game_emails_treats_none_as_a_brand_new_game(pool: sqlx::PgPool) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 2).await;
        let (_u0, gp0) = players[0];
        let (_u1, gp1) = players[1];
        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(gp1)
            .execute(&pool)
            .await
            .unwrap();

        notify_game_emails(None, &pool, &reqwest::Client::new(), game_id, None).await;

        assert!(email_token(&pool, gp1).await.is_some());
        assert!(email_token(&pool, gp0).await.is_none());
    }

    // DRM-03c2b: a replacement concession leaves the game unfinished, stamps
    // the conceding human's `conceded` departure metadata, and keeps that human
    // as a participant replaced by a bot. No Finished (game not finished) and
    // no Eliminated (conceded, not eliminated) notification is emitted - there
    // is no replacement-concession notification type.
    #[sqlx::test]
    async fn replacement_concession_emits_no_finished_or_eliminated_notification(
        pool: sqlx::PgPool,
    ) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 2).await;
        let (_u0, gp0) = players[0];
        let (_u1, gp1) = players[1];
        let before = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");

        // Replace the conceding human with a bot while retaining their identity
        // and stamping the approved departure metadata (DRM-03b1a3 shape).
        let bot_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, 'replacement', 'replacement') \
             RETURNING id",
        )
        .bind(game_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_players SET game_bot_id = $1, left_at = NOW(), \
               departure_reason = 'conceded', departure_sequence = 1 \
             WHERE id = $2",
        )
        .bind(bot_id)
        .bind(gp0)
        .execute(&pool)
        .await
        .unwrap();

        notify_game_emails(None, &pool, &reqwest::Client::new(), game_id, Some(before)).await;

        // A reply token is minted only for a recipient we actually mail; neither
        // the bot-replaced conceder nor the still-active human may be mailed.
        assert!(
            email_token(&pool, gp0).await.is_none(),
            "bot-replaced conceder must not get a Finished or Eliminated notification"
        );
        assert!(
            email_token(&pool, gp1).await.is_none(),
            "active player must not get a Finished or Eliminated notification"
        );
    }

    // R-20 / F-180: `notify_game_started` bypasses web-presence suppression so
    // the solo-start confirmation is never swallowed by the hydrated-page
    // presence window, while `notify_game_emails` (Normal mode) still suppresses.
    #[sqlx::test]
    async fn notify_game_started_bypasses_web_presence_suppression(pool: sqlx::PgPool) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 2).await;
        let (active_user, active_gp) = players[0];
        let (_u1, gp1) = players[1];
        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(active_gp)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(active_user)
            .execute(&pool)
            .await
            .unwrap();

        let http = reqwest::Client::new();

        // notify_game_emails (Normal mode) suppresses the active player.
        notify_game_emails(None, &pool, &http, game_id, None).await;
        assert!(
            email_token(&pool, active_gp).await.is_none(),
            "notify_game_emails must suppress a recently-active player"
        );

        // notify_game_started (BypassSuppression) does NOT suppress.
        notify_game_started(None, &pool, &http, game_id).await;
        assert!(
            email_token(&pool, active_gp).await.is_some(),
            "notify_game_started must bypass web-presence suppression"
        );
        assert!(
            email_token(&pool, gp1).await.is_none(),
            "a player not on turn must not be mailed"
        );
    }

    // R-20 / F-179: `notify_game_started` produces exactly one mail per
    // on-turn invitee - no duplicate "game started" burst.
    #[sqlx::test]
    async fn notify_game_started_one_mail_per_on_turn_player(pool: sqlx::PgPool) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 3).await;
        let (_u0, gp0) = players[0];
        let (_u1, gp1) = players[1];
        let (_u2, gp2) = players[2];
        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(gp0)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(gp2)
            .execute(&pool)
            .await
            .unwrap();

        notify_game_started(None, &pool, &reqwest::Client::new(), game_id).await;

        assert!(
            email_token(&pool, gp0).await.is_some(),
            "on-turn player 0 must be mailed exactly once"
        );
        assert!(
            email_token(&pool, gp1).await.is_none(),
            "off-turn player 1 must not be mailed"
        );
        assert!(
            email_token(&pool, gp2).await.is_some(),
            "on-turn player 2 must be mailed exactly once"
        );
    }
}
