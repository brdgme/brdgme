pub struct EmailCommandCtx<'a> {
    pub pool: &'a sqlx::PgPool,
    pub http_client: &'a reqwest::Client,
    pub broadcaster: &'a crate::websocket::GameBroadcaster,
    pub jetstream: &'a async_nats::jetstream::Context,
    pub resend: Option<&'a resend_rs::Resend>,
    pub game_id: uuid::Uuid,
    pub game_player_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub position: usize,
}

pub enum CommandReply {
    GameMove,
    Status(String),
    FullContent { html: String, text: String },
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    User(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesFilter {
    All,
    Basic,
    Advanced,
}

pub fn parse_rules_arg(arg: Option<&str>) -> RulesFilter {
    match arg.map(|s| s.trim().to_ascii_lowercase()) {
        Some(ref s) if s == "basic" => RulesFilter::Basic,
        Some(ref s) if s == "advanced" => RulesFilter::Advanced,
        _ => RulesFilter::All,
    }
}

pub fn subscribe_toggle(verb: &str) -> Option<bool> {
    match verb.to_ascii_lowercase().as_str() {
        "subscribe" => Some(true),
        "unsubscribe" => Some(false),
        _ => None,
    }
}

pub fn help_text() -> String {
    "Commands you can send by email:\n\
     \n\
     Game commands (played on your turn):\n\
     \n\
     Server commands:\n\
     concede - concede the current game\n\
     undo - undo your last move\n\
     restart - restart a finished game\n\
     rules [basic|advanced] - email the game rules and strategy\n\
     subscribe - turn on turn-notification emails (account-wide)\n\
     unsubscribe - turn off turn-notification emails (account-wide)\n\
     help - show this message\n\
     \n\
     Settings commands (name, colors, theme, emails) are also available."
        .to_string()
}

/// Extension point for unit c2 (settings commands: name/colours/theme/emails).
/// Returns `Some(result)` when the line matches a settings command; `None`
/// falls through to the game-command path.
pub fn dispatch_settings_command(
    _ctx: &EmailCommandCtx<'_>,
    _line: &str,
) -> Option<Result<CommandReply, CommandError>> {
    None
}

async fn set_turn_emails_enabled(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    enabled: bool,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE users SET turn_emails_enabled = $1, updated_at = NOW() WHERE id = $2")
        .bind(enabled)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn run_concede(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let ge = crate::db::find_game_extended(ctx.pool, ctx.game_id)
        .await?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    if ge.game.is_finished {
        return Err(CommandError::User("Game is already finished".to_string()));
    }
    if ge.game_players.len() != 2 {
        return Err(CommandError::User(
            "Concede is only available in 2-player games".to_string(),
        ));
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == ctx.game_player_id)
        .ok_or_else(|| CommandError::User("You are not a player in this game".to_string()))?;

    let before = ge.clone();
    crate::db::concede_game(ctx.pool, ctx.game_id, ctx.game_player_id, player.name())
        .await
        .map_err(CommandError::Internal)?;

    ctx.broadcaster.broadcast_game_update(ctx.game_id).await;
    crate::email::notify::notify_game_emails(
        ctx.resend,
        ctx.pool,
        ctx.http_client,
        ctx.game_id,
        Some(before),
    )
    .await;

    Ok(CommandReply::Status("You conceded.".to_string()))
}

async fn run_undo(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    use brdgme_cmd::api::{Request, Response};

    let ge = crate::db::find_game_extended(ctx.pool, ctx.game_id)
        .await?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    let player = ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == ctx.game_player_id)
        .ok_or_else(|| CommandError::User("You are not a player in this game".to_string()))?;

    let undo_state = player
        .game_player
        .undo_game_state
        .clone()
        .ok_or_else(|| CommandError::User("No undo state available".to_string()))?;

    let before = ge.clone();

    let resp = crate::game::client::request(
        ctx.http_client,
        &ge.game_version.uri,
        &ge.game_version.name,
        &Request::Status {
            game: undo_state.clone(),
        },
    )
    .await
    .map_err(|e| CommandError::Internal(anyhow::anyhow!("undo: fetch status: {e}")))?;

    let game_response = match resp {
        Response::Status { game, .. } => game,
        _ => {
            return Err(CommandError::Internal(anyhow::anyhow!(
                "undo: unexpected response from game service"
            )));
        }
    };

    let status = crate::game::status_fields(game_response.status);
    crate::db::undo_game(ctx.pool, ctx.game_id, &undo_state, ctx.position, &status)
        .await
        .map_err(CommandError::Internal)?;

    crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, ctx.game_id).await;
    crate::email::notify::notify_game_emails(
        ctx.resend,
        ctx.pool,
        ctx.http_client,
        ctx.game_id,
        Some(before),
    )
    .await;

    Ok(CommandReply::Status("Undo applied.".to_string()))
}

async fn run_restart(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let new_game_id = crate::game::server_fns::restart_game_impl(
        ctx.pool,
        ctx.http_client,
        ctx.user_id,
        ctx.game_id,
    )
    .await
    .map_err(|e| CommandError::User(e.to_string()))?;

    crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, new_game_id).await;
    crate::email::notify::notify_game_emails(
        ctx.resend,
        ctx.pool,
        ctx.http_client,
        new_game_id,
        None,
    )
    .await;
    ctx.broadcaster.broadcast_game_update(ctx.game_id).await;

    Ok(CommandReply::Status("Game restarted.".to_string()))
}

async fn run_rules(
    ctx: &EmailCommandCtx<'_>,
    filter: RulesFilter,
) -> Result<CommandReply, CommandError> {
    let version_id: uuid::Uuid =
        sqlx::query_scalar("SELECT game_version_id FROM games WHERE id = $1")
            .bind(ctx.game_id)
            .fetch_optional(ctx.pool)
            .await
            .map_err(|e| CommandError::Internal(anyhow::anyhow!("rules: fetch game version: {e}")))?
            .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    let rules_src = crate::db::find_game_version_rules(ctx.pool, version_id)
        .await?
        .ok_or_else(|| CommandError::User("Game version not found".to_string()))?;

    let (uri, name, interface_version) =
        crate::db::find_game_version_render_meta(ctx.pool, version_id)
            .await?
            .ok_or_else(|| CommandError::User("Game version not found".to_string()))?;

    let player_counts = crate::db::find_game_type_player_counts(ctx.pool, version_id)
        .await?
        .ok_or_else(|| CommandError::User("Game type not found".to_string()))?;

    let (players, player_style) = crate::rules::synthetic_players(&player_counts);
    let rules_html = crate::rules::render_doc(&rules_src, &players, &player_style)
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("rules render: {e}")))?;

    let (basic_src, advanced_src) =
        crate::rules::fetch_strategy(ctx.http_client, &uri, &name, interface_version)
            .await
            .map_err(|e| CommandError::Internal(anyhow::anyhow!("rules strategy: {e}")))?;

    let mut html = format!("<h2>Rules</h2>{rules_html}");
    let mut text = format!("Rules\n=====\n\n{rules_src}");

    if (filter == RulesFilter::All || filter == RulesFilter::Basic)
        && let Some(src) = &basic_src
    {
        let h = crate::rules::render_doc(src, &players, &player_style)
            .map_err(|e| CommandError::Internal(anyhow::anyhow!("basic strategy render: {e}")))?;
        html.push_str(&format!("<h2>Basic Strategy</h2>{h}"));
        text.push_str(&format!("\n\nBasic Strategy\n==============\n\n{src}"));
    }
    if (filter == RulesFilter::All || filter == RulesFilter::Advanced)
        && let Some(src) = &advanced_src
    {
        let h = crate::rules::render_doc(src, &players, &player_style).map_err(|e| {
            CommandError::Internal(anyhow::anyhow!("advanced strategy render: {e}"))
        })?;
        html.push_str(&format!("<h2>Advanced Strategy</h2>{h}"));
        text.push_str(&format!(
            "\n\nAdvanced Strategy\n=================\n\n{src}"
        ));
    }

    Ok(CommandReply::FullContent { html, text })
}

pub async fn dispatch_email_command(
    ctx: &EmailCommandCtx<'_>,
    line: &str,
) -> Result<CommandReply, CommandError> {
    let trimmed = line.trim();
    let (verb, arg) = match trimmed.split_once(' ') {
        Some((v, a)) => (v, Some(a.trim())),
        None => (trimmed, None),
    };
    let verb_lower = verb.to_ascii_lowercase();

    match verb_lower.as_str() {
        "concede" => return run_concede(ctx).await,
        "undo" => return run_undo(ctx).await,
        "restart" => return run_restart(ctx).await,
        "rules" => return run_rules(ctx, parse_rules_arg(arg)).await,
        "help" | "commands" => return Ok(CommandReply::Status(help_text())),
        _ => {}
    }

    if let Some(enabled) = subscribe_toggle(&verb_lower) {
        set_turn_emails_enabled(ctx.pool, ctx.user_id, enabled)
            .await
            .map_err(CommandError::Internal)?;
        let msg = if enabled {
            "Subscribed. Turn-notification emails are now on."
        } else {
            "Unsubscribed. Turn-notification emails are now off."
        };
        return Ok(CommandReply::Status(msg.to_string()));
    }

    if let Some(result) = dispatch_settings_command(ctx, trimmed) {
        return result;
    }

    match crate::game::execute_command(
        ctx.pool,
        ctx.http_client,
        ctx.broadcaster,
        ctx.jetstream,
        ctx.game_id,
        ctx.position,
        trimmed.to_string(),
    )
    .await
    {
        Ok(()) => Ok(CommandReply::GameMove),
        Err(crate::game::ExecuteCommandError::UserError(msg)) => Err(CommandError::User(msg)),
        Err(crate::game::ExecuteCommandError::Conflict) => Err(CommandError::User(
            "Your move could not be applied because the game changed; please try again."
                .to_string(),
        )),
        Err(crate::game::ExecuteCommandError::Other(e)) => Err(CommandError::Internal(e)),
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn verb_matching_is_case_insensitive() {
        assert_eq!(subscribe_toggle("SUBSCRIBE"), Some(true));
        assert_eq!(subscribe_toggle("Subscribe"), Some(true));
        assert_eq!(subscribe_toggle("subscribe"), Some(true));
        assert_eq!(subscribe_toggle("UNSUBSCRIBE"), Some(false));
        assert_eq!(subscribe_toggle("Unsubscribe"), Some(false));
        assert_eq!(subscribe_toggle("unsubscribe"), Some(false));
        assert_eq!(subscribe_toggle("unknown"), None);
    }

    #[test]
    fn parse_rules_arg_variants() {
        assert_eq!(parse_rules_arg(None), RulesFilter::All);
        assert_eq!(parse_rules_arg(Some("basic")), RulesFilter::Basic);
        assert_eq!(parse_rules_arg(Some("ADVANCED")), RulesFilter::Advanced);
        assert_eq!(parse_rules_arg(Some("  basic  ")), RulesFilter::Basic);
        assert_eq!(parse_rules_arg(Some("garbage")), RulesFilter::All);
    }

    #[test]
    fn help_text_names_server_commands() {
        let text = help_text();
        assert!(text.contains("concede"));
        assert!(text.contains("undo"));
        assert!(text.contains("restart"));
        assert!(text.contains("subscribe"));
        assert!(text.contains("unsubscribe"));
        assert!(text.contains("rules"));
        assert!(text.contains("help"));
        assert!(text.contains("Settings commands"));
    }

    #[test]
    fn help_text_is_nonempty() {
        assert!(!help_text().is_empty());
    }

    #[sqlx::test]
    async fn subscribe_unsubscribe_toggles_turn_emails(pool: sqlx::PgPool) {
        let user_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("test-player")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        set_turn_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!enabled);

        set_turn_emails_enabled(&pool, user_id, true).await.unwrap();
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(enabled);
    }
}
