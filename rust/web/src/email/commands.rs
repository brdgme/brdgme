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
     Settings commands:\n\
     name <display name> - set your display name\n\
     colors <c1,c2,c3> - set your 3 preferred colours (alias: colours)\n\
     theme <name> - set your theme (or 'theme system' for the default)\n\
     emails on | emails off - toggle turn-notification emails\n\
     settings - show your current settings"
        .to_string()
}

pub struct SettingsSummary {
    pub name: String,
    pub pref_colors: Vec<String>,
    pub theme: Option<String>,
    pub emails_enabled: bool,
}

pub fn format_settings_summary(s: &SettingsSummary) -> String {
    let colors = if s.pref_colors.is_empty() {
        "none set".to_string()
    } else {
        s.pref_colors.join(", ")
    };
    let theme = s.theme.as_deref().unwrap_or("system");
    let emails = if s.emails_enabled { "on" } else { "off" };
    format!(
        "Current settings:\n\
         \x20\x20Display name: {name}\n\
         \x20\x20Preferred colours: {colors}\n\
         \x20\x20Theme: {theme}\n\
         \x20\x20Turn-notification emails: {emails}",
        name = s.name,
    )
}

/// Maps a line to its canonical settings verb, case-insensitively, or `None`
/// when the line is not a settings command. `colours` normalises to `colors`.
fn settings_verb(line: &str) -> Option<&'static str> {
    let trimmed = line.trim();
    let verb = trimmed.split_once(' ').map(|(v, _)| v).unwrap_or(trimmed);
    match verb.to_ascii_lowercase().as_str() {
        "name" => Some("name"),
        "colors" | "colours" => Some("colors"),
        "theme" => Some("theme"),
        "emails" => Some("emails"),
        "settings" => Some("settings"),
        _ => None,
    }
}

/// Settings commands (name/colours/theme/emails/settings) that need only a
/// pool + user, no game context. Returns `Some(result)` when the line matches
/// a settings command; `None` falls through to the game-command path.
pub async fn dispatch_settings_command_for_user(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    line: &str,
) -> Option<Result<CommandReply, CommandError>> {
    let verb = settings_verb(line)?;
    let trimmed = line.trim();
    let arg = trimmed
        .split_once(' ')
        .map(|(_, a)| a.trim())
        .filter(|a| !a.is_empty());

    let result = match verb {
        "name" => run_settings_name(pool, user_id, arg).await,
        "colors" => run_settings_colors(pool, user_id, arg).await,
        "theme" => run_settings_theme(pool, user_id, arg).await,
        "emails" => run_settings_emails(pool, user_id, arg).await,
        "settings" => run_settings_summary(pool, user_id).await,
        _ => unreachable!("settings_verb only yields known verbs"),
    };
    Some(result)
}

pub async fn dispatch_settings_command(
    ctx: &EmailCommandCtx<'_>,
    line: &str,
) -> Option<Result<CommandReply, CommandError>> {
    dispatch_settings_command_for_user(ctx.pool, ctx.user_id, line).await
}

/// Standalone (no-game) settings path: handles settings commands, `help`, and
/// rejects everything else as unavailable by email without a game.
pub async fn dispatch_settings_standalone(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    line: &str,
) -> Result<CommandReply, CommandError> {
    if let Some(result) = dispatch_settings_command_for_user(pool, user_id, line).await {
        return result;
    }

    let trimmed = line.trim();
    let verb = trimmed.split_once(' ').map(|(v, _)| v).unwrap_or(trimmed);
    if matches!(verb.to_ascii_lowercase().as_str(), "help" | "commands") {
        return Ok(CommandReply::Status(help_text()));
    }

    Err(CommandError::User(
        "That command is not available by email without a game. Available settings commands: name, colors, theme, emails on/off, settings, help.".to_string(),
    ))
}

async fn run_settings_name(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    arg: Option<&str>,
) -> Result<CommandReply, CommandError> {
    let name = arg.ok_or_else(|| CommandError::User("Usage: name <display name>".to_string()))?;
    if !crate::db::validate_username(name) {
        return Err(CommandError::User(
            "1-16 characters: letters, numbers, - and _. Must be unique.".to_string(),
        ));
    }
    match crate::db::set_user_name(pool, user_id, name)
        .await
        .map_err(CommandError::Internal)?
    {
        true => Ok(CommandReply::Status(format!("Display name set to {name}."))),
        false => Err(CommandError::User("That name is taken".to_string())),
    }
}

async fn run_settings_colors(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    arg: Option<&str>,
) -> Result<CommandReply, CommandError> {
    let arg = arg.ok_or_else(|| CommandError::User("Usage: colors <c1,c2,c3>".to_string()))?;
    let colors: Vec<String> = arg
        .split([',', ' '])
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(crate::db::normalize_pref_color)
        .collect();
    if !crate::auth::server::validate_pref_colors(&colors) {
        return Err(CommandError::User(
            "Preferred colours must be 3 distinct colours from: Green, Red, Blue, Orange, Purple, Brown, Cyan, Pink.".to_string(),
        ));
    }
    crate::db::set_user_pref_colors(pool, user_id, &colors)
        .await
        .map_err(CommandError::Internal)?;
    Ok(CommandReply::Status(format!(
        "Preferred colours set to {}.",
        colors.join(", ")
    )))
}

async fn run_settings_theme(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    arg: Option<&str>,
) -> Result<CommandReply, CommandError> {
    let arg = arg.ok_or_else(|| CommandError::User("Usage: theme <name>".to_string()))?;
    let slug = arg.trim().to_ascii_lowercase();
    if slug == "system" || slug == "none" {
        crate::db::set_user_theme(pool, user_id, None)
            .await
            .map_err(CommandError::Internal)?;
        return Ok(CommandReply::Status("Theme set to system.".to_string()));
    }
    if !crate::theme::is_known_slug(&slug) {
        return Err(CommandError::User(
            "Unknown theme. Send 'theme system' or one of the theme slugs (e.g. dracula, brdgme-dark).".to_string(),
        ));
    }
    crate::db::set_user_theme(pool, user_id, Some(&slug))
        .await
        .map_err(CommandError::Internal)?;
    Ok(CommandReply::Status(format!("Theme set to {slug}.")))
}

async fn run_settings_emails(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    arg: Option<&str>,
) -> Result<CommandReply, CommandError> {
    let enabled = match arg.map(|a| a.to_ascii_lowercase()) {
        Some(ref a) if a == "on" => true,
        Some(ref a) if a == "off" => false,
        _ => {
            return Err(CommandError::User(
                "Usage: emails on | emails off".to_string(),
            ));
        }
    };
    set_turn_emails_enabled(pool, user_id, enabled)
        .await
        .map_err(CommandError::Internal)?;
    let msg = if enabled {
        "Turn-notification emails are now on."
    } else {
        "Turn-notification emails are now off."
    };
    Ok(CommandReply::Status(msg.to_string()))
}

async fn run_settings_summary(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<CommandReply, CommandError> {
    let name = crate::db::get_user_name(pool, user_id)
        .await
        .map_err(CommandError::Internal)?;
    let pref_colors = crate::db::get_user_pref_colors(pool, user_id)
        .await
        .map_err(CommandError::Internal)?;
    let theme = crate::db::get_user_theme(pool, user_id)
        .await
        .map_err(CommandError::Internal)?;
    let emails_enabled: bool =
        sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(|e| CommandError::Internal(anyhow::anyhow!("settings: fetch emails: {e}")))?;

    Ok(CommandReply::Status(format_settings_summary(
        &SettingsSummary {
            name,
            pref_colors,
            theme,
            emails_enabled,
        },
    )))
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

    if let Some(result) = dispatch_settings_command(ctx, trimmed).await {
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

    #[test]
    fn settings_verb_is_case_insensitive_and_aliases() {
        assert_eq!(settings_verb("NAME foo"), Some("name"));
        assert_eq!(settings_verb("Name foo"), Some("name"));
        assert_eq!(settings_verb("name foo"), Some("name"));
        assert_eq!(settings_verb("colors green,red,blue"), Some("colors"));
        assert_eq!(settings_verb("colours green,red,blue"), Some("colors"));
        assert_eq!(settings_verb("COLOURS green red blue"), Some("colors"));
        assert_eq!(settings_verb("theme system"), Some("theme"));
        assert_eq!(settings_verb("theme dracula"), Some("theme"));
        assert_eq!(settings_verb("emails on"), Some("emails"));
        assert_eq!(settings_verb("settings"), Some("settings"));
        assert_eq!(settings_verb("concede"), None);
        assert_eq!(settings_verb("play e4"), None);
    }

    #[test]
    fn format_settings_summary_renders_current_settings() {
        let summary = SettingsSummary {
            name: "alice".to_string(),
            pref_colors: vec!["Green".to_string(), "Red".to_string(), "Blue".to_string()],
            theme: Some("dracula".to_string()),
            emails_enabled: true,
        };
        let text = format_settings_summary(&summary);
        assert!(text.contains("Current settings:"));
        assert!(text.contains("Display name: alice"));
        assert!(text.contains("Preferred colours: Green, Red, Blue"));
        assert!(text.contains("Theme: dracula"));
        assert!(text.contains("Turn-notification emails: on"));
    }

    #[test]
    fn format_settings_summary_defaults() {
        let summary = SettingsSummary {
            name: "bob".to_string(),
            pref_colors: vec![],
            theme: None,
            emails_enabled: false,
        };
        let text = format_settings_summary(&summary);
        assert!(text.contains("Preferred colours: none set"));
        assert!(text.contains("Theme: system"));
        assert!(text.contains("Turn-notification emails: off"));
    }

    async fn seed_user(pool: &sqlx::PgPool, name: &str) -> uuid::Uuid {
        sqlx::query_scalar("INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id")
            .bind(name)
            .bind(Vec::<String>::new())
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn status_msg(reply: CommandReply) -> String {
        match reply {
            CommandReply::Status(s) => s,
            _ => panic!("expected Status reply"),
        }
    }

    fn expect_user_err(result: Option<Result<CommandReply, CommandError>>) -> String {
        match result.expect("expected a settings result") {
            Err(CommandError::User(s)) => s,
            Err(CommandError::Internal(e)) => panic!("expected User error, got Internal: {e}"),
            Ok(_) => panic!("expected User error, got Ok"),
        }
    }

    #[sqlx::test]
    async fn settings_name_valid_and_duplicate(pool: sqlx::PgPool) {
        let u1 = seed_user(&pool, "test-player").await;
        let u2 = seed_user(&pool, "other-player").await;

        let ok = dispatch_settings_command_for_user(&pool, u1, "name alice")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(ok), "Display name set to alice.");

        let taken =
            expect_user_err(dispatch_settings_command_for_user(&pool, u2, "name alice").await);
        assert_eq!(taken, "That name is taken");

        let invalid =
            expect_user_err(dispatch_settings_command_for_user(&pool, u1, "name has space").await);
        assert!(invalid.contains("1-16 characters"));
    }

    #[sqlx::test]
    async fn settings_colors_valid_and_invalid(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;

        let ok = dispatch_settings_command_for_user(&pool, user_id, "colors green,red,blue")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(ok), "Preferred colours set to Green, Red, Blue.");

        let stored: Vec<String> = sqlx::query_scalar("SELECT pref_colors FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, vec!["Green", "Red", "Blue"]);

        let invalid = expect_user_err(
            dispatch_settings_command_for_user(&pool, user_id, "colors green,notacolor,blue").await,
        );
        assert!(invalid.contains("3 distinct colours"));
    }

    #[sqlx::test]
    async fn settings_theme_valid_invalid_and_system(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;

        let ok = dispatch_settings_command_for_user(&pool, user_id, "theme dracula")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(ok), "Theme set to dracula.");
        let theme: Option<String> = sqlx::query_scalar("SELECT theme FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(theme.as_deref(), Some("dracula"));

        let invalid =
            expect_user_err(dispatch_settings_command_for_user(&pool, user_id, "theme nope").await);
        assert!(invalid.contains("Unknown theme"));

        let system = dispatch_settings_command_for_user(&pool, user_id, "theme system")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(system), "Theme set to system.");
        let theme: Option<String> = sqlx::query_scalar("SELECT theme FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(theme.is_none());
    }

    #[sqlx::test]
    async fn settings_emails_on_off_toggle(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;

        dispatch_settings_command_for_user(&pool, user_id, "emails off")
            .await
            .unwrap()
            .unwrap();
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!enabled);

        dispatch_settings_command_for_user(&pool, user_id, "emails on")
            .await
            .unwrap()
            .unwrap();
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(enabled);
    }
}
