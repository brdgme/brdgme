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

/// Splits a `ServerFnError` from a shared helper into this module's
/// user-vs-internal classification (wfe F21, wfe F24).
///
/// `crate::error::internal` is the only producer of
/// `INTERNAL_ERROR_MESSAGE`, and it has already logged the real cause, so that
/// exact message means "infrastructure failure": classify it `Internal` so the
/// caller substitutes the generic apology and inbound.rs logs the correlation.
/// Every other `ServerError` message is a deliberate user-facing string and is
/// returned as its BARE text - `ServerFnError`'s `Display` would otherwise
/// email it wrapped in "error running server function: ...".
fn classify_server_fn_error(
    context: &'static str,
    e: leptos::prelude::ServerFnError,
) -> CommandError {
    use leptos::prelude::ServerFnError;
    match e {
        ServerFnError::ServerError(msg) if msg == crate::error::INTERNAL_ERROR_MESSAGE => {
            CommandError::Internal(anyhow::anyhow!(
                "{context}: internal failure (cause already logged by crate::error::internal)"
            ))
        }
        ServerFnError::ServerError(msg) => CommandError::User(msg),
        other => CommandError::Internal(anyhow::anyhow!("{context}: {other}")),
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpponentToken {
    Bot(String),
    Human(String),
}

pub fn classify_opponent(token: &str, bot_names: &[String]) -> OpponentToken {
    let t = token.trim();
    let lower = t.to_ascii_lowercase();
    if let Some(inner) = lower.strip_prefix("bot:") {
        return OpponentToken::Bot(inner.trim().to_string());
    }
    if bot_names.iter().any(|b| b.to_ascii_lowercase() == lower) {
        OpponentToken::Bot(lower)
    } else {
        OpponentToken::Human(t.to_string())
    }
}

pub fn split_new_args(args: &str, known_keys: &[String]) -> Result<(String, Vec<String>), String> {
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(
            "Usage: new <gametype> <opponent>... (send 'list' to see game types)".to_string(),
        );
    }
    for end in (1..=tokens.len()).rev() {
        let candidate = tokens[..end].join(" ").to_ascii_lowercase();
        if let Some(key) = known_keys.iter().find(|k| **k == candidate) {
            let opponents = tokens[end..].iter().map(|s| s.to_string()).collect();
            return Ok((key.clone(), opponents));
        }
    }
    Err(format!(
        "Unknown game type '{}'. Send 'list' to see available games.",
        tokens[0]
    ))
}

pub struct GameTypeListEntry {
    pub name: String,
    pub player_counts: Vec<i32>,
    pub blurb: String,
}

pub fn format_game_type_list(entries: &[GameTypeListEntry]) -> String {
    if entries.is_empty() {
        return "No game types are available.".to_string();
    }
    let mut out = String::from("Game types you can start:");
    for e in entries {
        let counts = e
            .player_counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("\n{} ({} players)", e.name, counts));
        if !e.blurb.trim().is_empty() {
            out.push_str(&format!("\n  {}", e.blurb.trim()));
        }
    }
    out
}

pub fn check_duplicate_players(ids: &[uuid::Uuid]) -> Result<(), String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let before = sorted.len();
    sorted.dedup();
    if sorted.len() != before {
        return Err("Please ensure each player in the game is unique".to_string());
    }
    Ok(())
}

pub fn resolve_game_type<'a>(
    args: &str,
    types: &'a [(
        crate::models::game::GameType,
        Vec<crate::models::game::GameVersion>,
    )],
) -> Result<(&'a crate::models::game::GameType, Vec<String>), String> {
    let mut keys: Vec<String> = Vec::new();
    for (gt, _) in types {
        for k in [
            gt.name.to_ascii_lowercase(),
            crate::theme::slugify(&gt.name),
        ] {
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
    }
    let (key, opponents) = split_new_args(args, &keys)?;
    let gt = types
        .iter()
        .map(|(gt, _)| gt)
        .find(|gt| gt.name.to_ascii_lowercase() == key || crate::theme::slugify(&gt.name) == key)
        .ok_or_else(|| "Unknown game type. Send 'list' to see available games.".to_string())?;
    Ok((gt, opponents))
}

pub fn help_text() -> String {
    "Commands you can send by email:\n\
     \n\
     Game commands (played on your turn):\n\
     \n\
     Server commands:\n\
     new <gametype> <opponent>... - start a new game (opponents: usernames or bot names like easy/medium/hard)\n\
     list - list the game types you can start\n\
     concede - concede the current game\n\
     end - end the current game (only when you are the last human)\n\
     undo - undo your last move\n\
     bump - re-send all games waiting on your turn to your active address\n\
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
     emails - list your email addresses\n\
     email addresses - add, confirm, switch or remove on the website under Settings\n\
     emails on | emails off - toggle turn-notification emails\n\
     emails invite on | emails invite off - toggle invite-notification emails\n\
     emails reminder on | emails reminder off - toggle reminder-notification emails\n\
     settings - show your current settings"
        .to_string()
}

pub struct SettingsSummary {
    pub name: String,
    pub pref_colors: Vec<String>,
    pub theme: Option<String>,
    pub emails_enabled: bool,
    pub invite_emails_enabled: bool,
    pub reminder_emails_enabled: bool,
}

pub fn format_settings_summary(s: &SettingsSummary) -> String {
    let colors = if s.pref_colors.is_empty() {
        "none set".to_string()
    } else {
        s.pref_colors.join(", ")
    };
    let theme = s.theme.as_deref().unwrap_or("system");
    let emails = if s.emails_enabled { "on" } else { "off" };
    let invite = if s.invite_emails_enabled { "on" } else { "off" };
    let reminder = if s.reminder_emails_enabled {
        "on"
    } else {
        "off"
    };
    format!(
        "Current settings:\n\
         \x20\x20Display name: {name}\n\
         \x20\x20Preferred colours: {colors}\n\
         \x20\x20Theme: {theme}\n\
         \x20\x20Turn-notification emails: {emails}\n\
         \x20\x20Invite-notification emails: {invite}\n\
         \x20\x20Reminder-notification emails: {reminder}",
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
    resend: Option<&resend_rs::Resend>,
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
        "emails" => run_settings_emails(pool, resend, user_id, arg).await,
        "settings" => run_settings_summary(pool, user_id).await,
        _ => unreachable!("settings_verb only yields known verbs"),
    };
    Some(result)
}

pub async fn dispatch_settings_command(
    ctx: &EmailCommandCtx<'_>,
    line: &str,
) -> Option<Result<CommandReply, CommandError>> {
    dispatch_settings_command_for_user(ctx.pool, ctx.resend, ctx.user_id, line).await
}

/// Standalone (no-game) settings path: handles settings commands, `help`, and
/// rejects everything else as unavailable by email without a game.
pub async fn dispatch_settings_standalone(
    pool: &sqlx::PgPool,
    resend: Option<&resend_rs::Resend>,
    user_id: uuid::Uuid,
    line: &str,
) -> Result<CommandReply, CommandError> {
    if let Some(result) = dispatch_settings_command_for_user(pool, resend, user_id, line).await {
        return result;
    }

    let trimmed = line.trim();
    let verb = trimmed.split_once(' ').map(|(v, _)| v).unwrap_or(trimmed);
    if matches!(verb.to_ascii_lowercase().as_str(), "help" | "commands") {
        return Ok(CommandReply::Status(help_text()));
    }
    if verb.eq_ignore_ascii_case("list") {
        return run_list_command(pool).await;
    }

    Err(CommandError::User(
        "That command is not available by email without a game. Available commands: new, bump, list, name, colors, theme, emails on/off, subscribe, unsubscribe, rules, settings, help.".to_string(),
    ))
}

pub struct StandaloneCommandCtx<'a> {
    pub pool: &'a sqlx::PgPool,
    pub http_client: &'a reqwest::Client,
    pub broadcaster: &'a crate::websocket::GameBroadcaster,
    pub jetstream: &'a async_nats::jetstream::Context,
    pub resend: Option<&'a resend_rs::Resend>,
    pub user_id: uuid::Uuid,
}

pub async fn dispatch_standalone_server_command(
    ctx: &StandaloneCommandCtx<'_>,
    line: &str,
) -> Result<CommandReply, CommandError> {
    let trimmed = line.trim();
    let verb = trimmed.split_once(' ').map(|(v, _)| v).unwrap_or(trimmed);
    if verb.eq_ignore_ascii_case("new") {
        let args = trimmed.split_once(' ').map(|(_, a)| a.trim()).unwrap_or("");
        return run_new_command(ctx, args).await;
    }
    if verb.eq_ignore_ascii_case("bump") {
        return run_bump_command(ctx).await;
    }
    if let Some(enabled) = subscribe_toggle(verb) {
        return apply_subscribe_toggle(ctx.pool, ctx.user_id, enabled).await;
    }
    dispatch_settings_standalone(ctx.pool, ctx.resend, ctx.user_id, trimmed).await
}

async fn run_list_command(pool: &sqlx::PgPool) -> Result<CommandReply, CommandError> {
    let types = crate::db::find_available_game_types(pool)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("list: find game types: {e}")))?;
    let entries: Vec<GameTypeListEntry> = types
        .into_iter()
        .map(|(gt, _)| GameTypeListEntry {
            name: gt.name,
            player_counts: gt.player_counts,
            blurb: gt.blurb,
        })
        .collect();
    Ok(CommandReply::Status(format_game_type_list(&entries)))
}

async fn run_new_command(
    ctx: &StandaloneCommandCtx<'_>,
    args: &str,
) -> Result<CommandReply, CommandError> {
    let types = crate::db::find_available_game_types(ctx.pool)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: find game types: {e}")))?;
    let (game_type, opponent_tokens) =
        resolve_game_type(args, &types).map_err(CommandError::User)?;
    let type_name = game_type.name.clone();

    let game_version = crate::db::find_latest_non_deprecated_game_version(ctx.pool, game_type.id)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: find game version: {e}")))?
        .ok_or_else(|| {
            CommandError::User("That game type has no available version.".to_string())
        })?;

    let bot_names = crate::db::find_enabled_bots(ctx.pool)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: find bots: {e}")))?;

    let mut bot_slots: Vec<crate::game::server_fns::BotSlot> = Vec::new();
    let mut bot_display_names: Vec<String> = Vec::new();
    let mut human_ids: Vec<uuid::Uuid> = Vec::new();
    let mut human_names: Vec<String> = Vec::new();
    let mut bot_counter = 0usize;

    for tok in &opponent_tokens {
        match classify_opponent(tok, &bot_names) {
            OpponentToken::Bot(difficulty) => {
                bot_counter += 1;
                let display =
                    petname::petname(1, "-").unwrap_or_else(|| format!("Bot {bot_counter}"));
                bot_display_names.push(format!("{display} (bot: {difficulty})"));
                bot_slots.push(crate::game::server_fns::BotSlot {
                    name: display,
                    bot_name: difficulty,
                });
            }
            OpponentToken::Human(username) => {
                let id = crate::db::find_user_id_by_name(ctx.pool, &username)
                    .await
                    .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: find user: {e}")))?
                    .ok_or_else(|| CommandError::User(format!("No user named '{username}'.")))?;
                if id == ctx.user_id {
                    return Err(CommandError::User(
                        "You are included in the game automatically; do not list yourself as an opponent."
                            .to_string(),
                    ));
                }
                human_ids.push(id);
                human_names.push(username);
            }
        }
    }

    if let Some(msg) = crate::db::validate_bot_slots(ctx.pool, &bot_slots)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: validate bot slots: {e}")))?
    {
        return Err(CommandError::User(msg));
    }

    check_duplicate_players(&human_ids).map_err(CommandError::User)?;

    let player_count = 1 + human_ids.len() + bot_slots.len();
    let player_counts = crate::db::find_game_type_player_counts(ctx.pool, game_version.id)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: find player counts: {e}")))?
        .ok_or_else(|| CommandError::User("Game type not found".to_string()))?;
    if let Some(msg) = crate::game::server_fns::roster_error(&player_counts, player_count) {
        return Err(CommandError::User(msg));
    }

    let fetched = crate::game::server_fns::fetch_game_from_service(
        ctx.http_client,
        &game_version,
        player_count,
    )
    .await
    .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: fetch game: {e}")))?;

    let mut tx = ctx
        .pool
        .begin()
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: begin tx: {e}")))?;
    let game = crate::game::server_fns::insert_game_from_service(
        &mut tx,
        game_version.id,
        crate::game::server_fns::CreateGameSeed {
            creator_id: ctx.user_id,
            opponent_ids: &human_ids,
            opponent_emails: &[],
            bot_slots: &bot_slots,
            all_accepted: false,
        },
        fetched,
    )
    .await
    .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: create game: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("new: commit: {e}")))?;

    crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, game.id).await;
    crate::email::notify::notify_game_emails(ctx.resend, ctx.pool, ctx.http_client, game.id, None)
        .await;

    let roster_parts: Vec<String> = human_names
        .iter()
        .cloned()
        .chain(bot_display_names.iter().cloned())
        .collect();
    let roster = roster_parts.join(", ");
    let link = crate::email::notify::browser_url(game.id);
    let msg = if roster.is_empty() {
        format!("Game created: {type_name} (solo). Play: {link}")
    } else {
        format!("Game created: {type_name} with {roster}. Play: {link}")
    };
    Ok(CommandReply::Status(msg))
}

async fn run_bump_command(ctx: &StandaloneCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    bump_reply(ctx.pool, ctx.http_client, ctx.resend, ctx.user_id).await
}

async fn bump_reply(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    resend: Option<&resend_rs::Resend>,
    user_id: uuid::Uuid,
) -> Result<CommandReply, CommandError> {
    let games = crate::db::find_active_turn_games(pool, user_id, crate::db::SWITCH_DIGEST_CAP + 1)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("bump: find turn games: {e}")))?;
    let capped_out = games.len() > crate::db::SWITCH_DIGEST_CAP;
    let capped = crate::db::cap_digest(games, crate::db::SWITCH_DIGEST_CAP);
    let n = capped.len();
    for (game_id, game_player_id) in capped {
        crate::email::notify::send_turn_digest_forced(
            resend,
            pool,
            http_client,
            game_id,
            game_player_id,
        )
        .await;
    }
    let mut msg = match n {
        0 => "No games are waiting on your turn.".to_string(),
        1 => "Re-sent 1 game to your active address.".to_string(),
        n => format!("Re-sent {n} games to your active address."),
    };
    if capped_out {
        msg.push_str(&format!(
            " More games are waiting; this reply is capped at {}. Open the site to see them all.",
            crate::db::SWITCH_DIGEST_CAP
        ));
    }
    Ok(CommandReply::Status(msg))
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
    _resend: Option<&resend_rs::Resend>,
    user_id: uuid::Uuid,
    arg: Option<&str>,
) -> Result<CommandReply, CommandError> {
    let Some(arg) = arg else {
        return run_emails_list(pool, user_id).await;
    };
    let (sub, rest) = arg
        .split_once(' ')
        .map(|(s, r)| (s, Some(r.trim())))
        .unwrap_or((arg, None));
    match sub.to_ascii_lowercase().as_str() {
        "on" => run_emails_toggle(pool, user_id, true).await,
        "off" => run_emails_toggle(pool, user_id, false).await,
        "invite" => {
            let sub_arg = rest.unwrap_or("");
            match sub_arg.to_ascii_lowercase().as_str() {
                "on" => run_emails_invite_toggle(pool, user_id, true).await,
                "off" => run_emails_invite_toggle(pool, user_id, false).await,
                _ => Err(CommandError::User(
                    "Usage: emails invite on | emails invite off".to_string(),
                )),
            }
        }
        "reminder" => {
            let sub_arg = rest.unwrap_or("");
            match sub_arg.to_ascii_lowercase().as_str() {
                "on" => run_emails_reminder_toggle(pool, user_id, true).await,
                "off" => run_emails_reminder_toggle(pool, user_id, false).await,
                _ => Err(CommandError::User(
                    "Usage: emails reminder on | emails reminder off".to_string(),
                )),
            }
        }
        _ => Err(CommandError::User(
            "Usage: emails | emails on | emails off | emails invite on | emails invite off | emails reminder on | emails reminder off. Email addresses are managed on the website under Settings.".to_string(),
        )),
    }
}

async fn run_emails_list(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<CommandReply, CommandError> {
    let rows = crate::db::list_user_emails(pool, user_id)
        .await
        .map_err(CommandError::Internal)?;
    if rows.is_empty() {
        return Ok(CommandReply::Status(
            "No email addresses on your account.".to_string(),
        ));
    }
    let mut lines = Vec::with_capacity(rows.len());
    for row in &rows {
        let marker = if row.is_primary { "*" } else { " " };
        let status = if row.is_primary {
            "active"
        } else if row.verified_at.is_some() {
            "verified"
        } else {
            "unverified"
        };
        lines.push(format!("{marker} {} ({status})", row.email));
    }
    Ok(CommandReply::Status(lines.join("\n")))
}

async fn run_emails_toggle(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    enabled: bool,
) -> Result<CommandReply, CommandError> {
    crate::db::set_user_turn_emails_enabled(pool, user_id, enabled)
        .await
        .map_err(CommandError::Internal)?;
    let msg = if enabled {
        "Turn-notification emails are now on."
    } else {
        "Turn-notification emails are now off."
    };
    Ok(CommandReply::Status(msg.to_string()))
}

async fn run_emails_invite_toggle(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    enabled: bool,
) -> Result<CommandReply, CommandError> {
    crate::db::set_user_invite_emails_enabled(pool, user_id, enabled)
        .await
        .map_err(CommandError::Internal)?;
    let msg = if enabled {
        "Invite-notification emails are now on."
    } else {
        "Invite-notification emails are now off."
    };
    Ok(CommandReply::Status(msg.to_string()))
}

async fn run_emails_reminder_toggle(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    enabled: bool,
) -> Result<CommandReply, CommandError> {
    crate::db::set_user_reminder_emails_enabled(pool, user_id, enabled)
        .await
        .map_err(CommandError::Internal)?;
    let msg = if enabled {
        "Reminder-notification emails are now on."
    } else {
        "Reminder-notification emails are now off."
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
    let (emails_enabled, invite_emails_enabled, reminder_emails_enabled) =
        crate::db::get_user_email_prefs(pool, user_id)
            .await
            .map_err(CommandError::Internal)?;

    Ok(CommandReply::Status(format_settings_summary(
        &SettingsSummary {
            name,
            pref_colors,
            theme,
            emails_enabled,
            invite_emails_enabled,
            reminder_emails_enabled,
        },
    )))
}

async fn run_concede(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let before = crate::game::server_fns::concede_core(
        ctx.pool,
        ctx.game_id,
        crate::game::server_fns::ActingPlayer::GamePlayer(ctx.game_player_id),
    )
    .await
    .map_err(|e| classify_server_fn_error("concede", e))?;

    crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, ctx.game_id).await;
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

async fn run_end(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let ge = crate::db::find_game_extended(ctx.pool, ctx.game_id)
        .await?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    if ge.game.is_finished {
        return Err(CommandError::User("Game is already finished".to_string()));
    }

    let is_player = ge
        .game_players
        .iter()
        .any(|p| p.game_player.id == ctx.game_player_id);
    if !is_player {
        return Err(CommandError::User(
            "You are not a player in this game".to_string(),
        ));
    }

    let active_humans = ge
        .game_players
        .iter()
        .filter(|p| p.game_player.user_id.is_some() && p.game_player.left_at.is_none())
        .count();
    if active_humans > 1 {
        return Err(CommandError::User(
            "End game is only available to the last human".to_string(),
        ));
    }

    let before = ge.clone();
    crate::db::end_game(ctx.pool, ctx.game_id)
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

    Ok(CommandReply::Status("Game ended.".to_string()))
}

async fn run_undo(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let before = crate::game::server_fns::undo_core(
        ctx.pool,
        ctx.http_client,
        ctx.game_id,
        crate::game::server_fns::ActingPlayer::GamePlayer(ctx.game_player_id),
    )
    .await
    .map_err(|e| classify_server_fn_error("undo", e))?;

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
    use crate::game::server_fns::{BotSlot, RestartOutcome, restart_core};

    let ge = crate::db::find_game_extended(ctx.pool, ctx.game_id)
        .await
        .map_err(|e| CommandError::Internal(anyhow::anyhow!("restart: find game: {e}")))?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    if !ge.game.is_finished {
        return Err(CommandError::User("Game is not finished".to_string()));
    }
    if !ge
        .game_players
        .iter()
        .any(|p| p.user.as_ref().is_some_and(|u| u.id == ctx.user_id))
    {
        return Err(CommandError::User(
            "You are not a player in this game".to_string(),
        ));
    }

    let version =
        crate::db::find_latest_non_deprecated_game_version(ctx.pool, ge.game_version.game_type_id)
            .await
            .map_err(|e| {
                CommandError::Internal(anyhow::anyhow!("restart: find latest game version: {e}"))
            })?
            .unwrap_or_else(|| ge.game_version.clone());

    let opponent_ids: Vec<uuid::Uuid> = ge
        .game_players
        .iter()
        .filter_map(|p| {
            p.user
                .as_ref()
                .filter(|u| u.id != ctx.user_id)
                .map(|u| u.id)
        })
        .collect();
    let bot_slots: Vec<BotSlot> = ge
        .game_players
        .iter()
        .filter_map(|p| {
            p.game_bot.as_ref().map(|b| BotSlot {
                name: b.name.clone(),
                bot_name: b.bot_name.clone(),
            })
        })
        .collect();

    let outcome = restart_core(
        ctx.pool,
        ctx.http_client,
        ctx.user_id,
        ctx.game_id,
        &version,
        &opponent_ids,
        &[],
        &bot_slots,
    )
    .await
    .map_err(|e| classify_server_fn_error("restart", e))?;

    match outcome {
        RestartOutcome::Created(created) => {
            if let Some(gid) = created.game_id {
                crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, gid)
                    .await;
                crate::email::notify::notify_game_emails(
                    ctx.resend,
                    ctx.pool,
                    ctx.http_client,
                    gid,
                    None,
                )
                .await;
                ctx.broadcaster.broadcast_game_update(ctx.game_id).await;
                return Ok(CommandReply::Status("Game restarted.".to_string()));
            }
            if let Some(pid) = created.proposal_id {
                ctx.broadcaster.broadcast_proposal_update(pid).await;
            }
            Ok(CommandReply::Status(
                "Restart proposed; invitees must confirm before the game starts.".to_string(),
            ))
        }
        RestartOutcome::AlreadyRestarted { .. } => Err(CommandError::User(
            "Game has already been restarted".to_string(),
        )),
    }
}

async fn run_rules(
    ctx: &EmailCommandCtx<'_>,
    filter: RulesFilter,
) -> Result<CommandReply, CommandError> {
    let version_id = crate::db::find_game_version_id_for_game(ctx.pool, ctx.game_id)
        .await
        .map_err(CommandError::Internal)?
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

async fn apply_subscribe_toggle(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    enabled: bool,
) -> Result<CommandReply, CommandError> {
    crate::db::set_user_turn_emails_enabled(pool, user_id, enabled)
        .await
        .map_err(CommandError::Internal)?;
    let msg = if enabled {
        "Subscribed. Turn-notification emails are now on."
    } else {
        "Unsubscribed. Turn-notification emails are now off."
    };
    Ok(CommandReply::Status(msg.to_string()))
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
        "end" => return run_end(ctx).await,
        "undo" => return run_undo(ctx).await,
        "restart" => return run_restart(ctx).await,
        "rules" => return run_rules(ctx, parse_rules_arg(arg)).await,
        "help" | "commands" => return Ok(CommandReply::Status(help_text())),
        "new" => {
            let sctx = StandaloneCommandCtx {
                pool: ctx.pool,
                http_client: ctx.http_client,
                broadcaster: ctx.broadcaster,
                jetstream: ctx.jetstream,
                resend: ctx.resend,
                user_id: ctx.user_id,
            };
            return run_new_command(&sctx, arg.unwrap_or("")).await;
        }
        "bump" => {
            let sctx = StandaloneCommandCtx {
                pool: ctx.pool,
                http_client: ctx.http_client,
                broadcaster: ctx.broadcaster,
                jetstream: ctx.jetstream,
                resend: ctx.resend,
                user_id: ctx.user_id,
            };
            return run_bump_command(&sctx).await;
        }
        "list" => return run_list_command(ctx.pool).await,
        _ => {}
    }

    if let Some(enabled) = subscribe_toggle(&verb_lower) {
        return apply_subscribe_toggle(ctx.pool, ctx.user_id, enabled).await;
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
        Ok(_) => Ok(CommandReply::GameMove),
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

    #[test]
    fn classify_server_fn_error_redacted_internal_is_internal() {
        let e = leptos::prelude::ServerFnError::new(crate::error::INTERNAL_ERROR_MESSAGE);
        match classify_server_fn_error("ctx", e) {
            CommandError::Internal(_) => {}
            CommandError::User(m) => panic!("expected Internal, got User({m})"),
        }
    }

    #[test]
    fn classify_server_fn_error_user_message_is_bare() {
        let e = leptos::prelude::ServerFnError::new("Game is not finished");
        match classify_server_fn_error("ctx", e) {
            CommandError::User(m) => assert_eq!(m, "Game is not finished"),
            CommandError::Internal(e) => panic!("expected User, got Internal({e})"),
        }
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

        crate::db::set_user_turn_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!enabled);

        crate::db::set_user_turn_emails_enabled(&pool, user_id, true)
            .await
            .unwrap();
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(enabled);
    }

    #[sqlx::test]
    async fn standalone_unsubscribe_and_subscribe_toggle_turn_emails(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "sub-toggle-user").await;
        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let ctx = StandaloneCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            user_id,
        };

        match dispatch_standalone_server_command(&ctx, "unsubscribe").await {
            Ok(CommandReply::Status(_)) => {}
            Ok(_) => panic!("expected Status reply for unsubscribe"),
            Err(e) => panic!("expected Ok for unsubscribe, got {e}"),
        }
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!enabled);

        match dispatch_standalone_server_command(&ctx, "subscribe").await {
            Ok(CommandReply::Status(_)) => {}
            Ok(_) => panic!("expected Status reply for subscribe"),
            Err(e) => panic!("expected Ok for subscribe, got {e}"),
        }
        let enabled: bool =
            sqlx::query_scalar("SELECT turn_emails_enabled FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(enabled);
    }

    #[sqlx::test]
    async fn standalone_rejects_unsupported_verb_with_updated_message(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "reject-verb-user").await;
        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let ctx = StandaloneCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            user_id,
        };
        match dispatch_standalone_server_command(&ctx, "frobnicate").await {
            Err(CommandError::User(msg)) => {
                assert!(msg.contains("bump"), "rejection should mention bump: {msg}");
                assert!(
                    msg.contains("unsubscribe"),
                    "rejection should mention unsubscribe: {msg}"
                );
            }
            Err(CommandError::Internal(e)) => panic!("expected User error, got Internal: {e}"),
            Ok(_) => panic!("expected User error, got Ok"),
        }
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
            invite_emails_enabled: true,
            reminder_emails_enabled: false,
        };
        let text = format_settings_summary(&summary);
        assert!(text.contains("Current settings:"));
        assert!(text.contains("Display name: alice"));
        assert!(text.contains("Preferred colours: Green, Red, Blue"));
        assert!(text.contains("Theme: dracula"));
        assert!(text.contains("Turn-notification emails: on"));
        assert!(text.contains("Invite-notification emails: on"));
        assert!(text.contains("Reminder-notification emails: off"));
    }

    #[test]
    fn format_settings_summary_defaults() {
        let summary = SettingsSummary {
            name: "bob".to_string(),
            pref_colors: vec![],
            theme: None,
            emails_enabled: false,
            invite_emails_enabled: false,
            reminder_emails_enabled: false,
        };
        let text = format_settings_summary(&summary);
        assert!(text.contains("Preferred colours: none set"));
        assert!(text.contains("Theme: system"));
        assert!(text.contains("Turn-notification emails: off"));
        assert!(text.contains("Invite-notification emails: off"));
        assert!(text.contains("Reminder-notification emails: off"));
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

        let ok = dispatch_settings_command_for_user(&pool, None, u1, "name alice")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(ok), "Display name set to alice.");

        let taken = expect_user_err(
            dispatch_settings_command_for_user(&pool, None, u2, "name alice").await,
        );
        assert_eq!(taken, "That name is taken");

        let invalid = expect_user_err(
            dispatch_settings_command_for_user(&pool, None, u1, "name has space").await,
        );
        assert!(invalid.contains("1-16 characters"));
    }

    #[sqlx::test]
    async fn settings_colors_valid_and_invalid(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;

        let ok = dispatch_settings_command_for_user(&pool, None, user_id, "colors green,red,blue")
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
            dispatch_settings_command_for_user(&pool, None, user_id, "colors green,notacolor,blue")
                .await,
        );
        assert!(invalid.contains("3 distinct colours"));
    }

    #[sqlx::test]
    async fn settings_theme_valid_invalid_and_system(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;

        let ok = dispatch_settings_command_for_user(&pool, None, user_id, "theme dracula")
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

        let invalid = expect_user_err(
            dispatch_settings_command_for_user(&pool, None, user_id, "theme nope").await,
        );
        assert!(invalid.contains("Unknown theme"));

        let system = dispatch_settings_command_for_user(&pool, None, user_id, "theme system")
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

        dispatch_settings_command_for_user(&pool, None, user_id, "emails off")
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

        dispatch_settings_command_for_user(&pool, None, user_id, "emails on")
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

    #[test]
    fn classify_opponent_detects_bots() {
        let bn = vec!["easy".to_string(), "medium".to_string(), "hard".to_string()];
        assert_eq!(
            classify_opponent("easy", &bn),
            OpponentToken::Bot("easy".to_string())
        );
        assert_eq!(
            classify_opponent("HARD", &bn),
            OpponentToken::Bot("hard".to_string())
        );
        assert_eq!(
            classify_opponent("bot:medium", &bn),
            OpponentToken::Bot("medium".to_string())
        );
        assert_eq!(
            classify_opponent("BOT:easy", &bn),
            OpponentToken::Bot("easy".to_string())
        );
        assert_eq!(
            classify_opponent("alice", &bn),
            OpponentToken::Human("alice".to_string())
        );
    }

    #[test]
    fn split_new_args_single_word_type() {
        let known = vec!["chess".to_string()];
        assert_eq!(
            split_new_args("chess alice easy", &known),
            Ok((
                "chess".to_string(),
                vec!["alice".to_string(), "easy".to_string()]
            ))
        );
        assert_eq!(
            split_new_args("CHESS alice", &known),
            Ok(("chess".to_string(), vec!["alice".to_string()]))
        );
    }

    #[test]
    fn split_new_args_multi_word_type() {
        let known = vec!["nine mens morris".to_string(), "chess".to_string()];
        assert_eq!(
            split_new_args("nine mens morris alice", &known),
            Ok(("nine mens morris".to_string(), vec!["alice".to_string()]))
        );
    }

    #[test]
    fn split_new_args_unknown_type_errors() {
        let known = vec!["chess".to_string()];
        let err = split_new_args("nope alice", &known).unwrap_err();
        assert!(err.contains("Unknown game type"));
    }

    #[test]
    fn split_new_args_empty_errors() {
        let known = vec!["chess".to_string()];
        let err = split_new_args("", &known).unwrap_err();
        assert!(err.contains("Usage"));
    }

    #[test]
    fn format_game_type_list_renders() {
        let entries = vec![
            GameTypeListEntry {
                name: "Chess".to_string(),
                player_counts: vec![2],
                blurb: "Classic strategy game.".to_string(),
            },
            GameTypeListEntry {
                name: "Nine Mens Morris".to_string(),
                player_counts: vec![2],
                blurb: String::new(),
            },
        ];
        let out = format_game_type_list(&entries);
        assert!(out.contains("Game types you can start:"));
        assert!(out.contains("Chess"));
        assert!(out.contains("Nine Mens Morris"));
        assert!(out.contains("(2 players)"));
        assert!(out.contains("Classic strategy game."));

        let empty = format_game_type_list(&[]);
        assert!(empty.contains("No game types"));
    }

    #[test]
    fn check_duplicate_players_detects_dupes() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        assert!(check_duplicate_players(&[a, b]).is_ok());

        let err = check_duplicate_players(&[a, a]).unwrap_err();
        assert!(err.contains("unique"));
    }

    fn game_type(name: &str) -> crate::models::game::GameType {
        let ts = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        );
        crate::models::game::GameType {
            id: uuid::Uuid::new_v4(),
            created_at: ts,
            updated_at: ts,
            name: name.to_string(),
            player_counts: vec![2],
            weight: 1.0,
            blurb: String::new(),
        }
    }

    #[test]
    fn resolve_game_type_matches_name_and_slug() {
        let chess = game_type("Chess");
        let morris = game_type("Nine Mens Morris");
        let types = vec![(chess.clone(), vec![]), (morris.clone(), vec![])];

        let (gt, opponents) = resolve_game_type("chess alice", &types).unwrap();
        assert_eq!(gt.id, chess.id);
        assert_eq!(opponents, vec!["alice".to_string()]);

        let (gt, opponents) = resolve_game_type("NINE MENS MORRIS bob", &types).unwrap();
        assert_eq!(gt.id, morris.id);
        assert_eq!(opponents, vec!["bob".to_string()]);

        let slug = crate::theme::slugify("Chess");
        let (gt, _) = resolve_game_type(&format!("{slug} alice"), &types).unwrap();
        assert_eq!(gt.id, chess.id);

        assert!(resolve_game_type("nope alice", &types).is_err());
    }

    #[test]
    fn help_text_lists_new_and_list() {
        let text = help_text();
        assert!(text.contains("new <gametype>"));
        assert!(text.contains("list"));
    }

    #[test]
    fn emails_subcommands_in_help_text() {
        let text = help_text();
        assert!(text.contains("emails on"));
        assert!(text.contains("emails off"));
    }

    #[test]
    fn settings_verb_matches_emails_subcommands() {
        assert_eq!(settings_verb("emails add foo@bar.com"), Some("emails"));
        assert_eq!(settings_verb("emails confirm 123456"), Some("emails"));
        assert_eq!(settings_verb("emails active foo@bar.com"), Some("emails"));
        assert_eq!(settings_verb("emails use foo@bar.com"), Some("emails"));
        assert_eq!(settings_verb("emails remove foo@bar.com"), Some("emails"));
        assert_eq!(settings_verb("emails"), Some("emails"));
        assert_eq!(settings_verb("EMAILS ADD foo@bar.com"), Some("emails"));
    }

    #[sqlx::test]
    async fn emails_list_shows_addresses(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())")
            .bind(user_id)
            .bind("primary@example.com")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, false, NOW())")
            .bind(user_id)
            .bind("work@example.com")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind("pending@example.com")
            .execute(&pool)
            .await
            .unwrap();

        let reply = dispatch_settings_command_for_user(&pool, None, user_id, "emails")
            .await
            .unwrap()
            .unwrap();
        let text = status_msg(reply);
        assert!(text.contains("* primary@example.com (active)"));
        assert!(text.contains("work@example.com (verified)"));
        assert!(text.contains("pending@example.com (unverified)"));
    }

    #[sqlx::test]
    async fn emails_list_empty(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;
        let reply = dispatch_settings_command_for_user(&pool, None, user_id, "emails")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(reply), "No email addresses on your account.");
    }

    #[sqlx::test]
    async fn emails_on_off_still_works(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;

        let reply = dispatch_settings_command_for_user(&pool, None, user_id, "emails off")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(reply), "Turn-notification emails are now off.");

        let reply = dispatch_settings_command_for_user(&pool, None, user_id, "emails on")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status_msg(reply), "Turn-notification emails are now on.");
    }

    #[sqlx::test]
    async fn emails_unknown_subcommand_errors(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;
        let err = expect_user_err(
            dispatch_settings_command_for_user(&pool, None, user_id, "emails bogus").await,
        );
        assert!(err.contains("Usage:"));
    }

    #[sqlx::test]
    async fn run_settings_emails_rejects_removed_subcommands(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;
        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_emails WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        for arg in [
            "add x@y.com",
            "confirm 123456",
            "active x@y.com",
            "use x@y.com",
            "remove x@y.com",
        ] {
            match run_settings_emails(&pool, None, user_id, Some(arg)).await {
                Err(CommandError::User(msg)) => {
                    assert!(msg.to_lowercase().contains("website"));
                }
                Err(CommandError::Internal(e)) => {
                    panic!("expected User error for {arg}, got Internal: {e}")
                }
                Ok(_) => panic!("expected User error for {arg}, got Ok"),
            }
        }
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_emails WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn help_text_omits_address_management() {
        let t = help_text();
        assert!(!t.contains("emails add"));
        assert!(!t.contains("emails confirm"));
        assert!(!t.contains("emails active"));
        assert!(!t.contains("emails remove"));
        assert!(t.contains("emails on"));
    }

    #[sqlx::test]
    async fn retained_settings_verbs_still_work(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "test-player").await;
        for line in [
            "name Bob",
            "theme system",
            "colors green,red,blue",
            "emails on",
            "emails off",
            "emails invite on",
            "emails reminder off",
            "emails",
            "settings",
        ] {
            match dispatch_settings_standalone(&pool, None, user_id, line).await {
                Ok(_) => {}
                Err(e) => panic!("expected Ok for {line}, got {e}"),
            }
        }
    }

    #[sqlx::test]
    async fn game_path_cannot_manage_addresses(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "game-path-player").await;
        let opp = seed_user(&pool, "game-path-opp").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[opp],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "initial_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        let (game_player_id, position): (uuid::Uuid, i32) = sqlx::query_as(
            "SELECT id, position FROM game_players WHERE game_id = $1 AND user_id = $2",
        )
        .bind(game.id)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_emails WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let ctx = EmailCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            game_id: game.id,
            game_player_id,
            user_id,
            position: position as usize,
        };
        match dispatch_email_command(&ctx, "emails add attacker@evil.com").await {
            Err(CommandError::User(msg)) => {
                assert!(msg.to_lowercase().contains("website"));
            }
            Err(CommandError::Internal(e)) => panic!("expected User error, got Internal: {e}"),
            Ok(_) => panic!("expected User error, got Ok"),
        }
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_emails WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(before, after);
    }

    async fn make_game_version(pool: &sqlx::PgPool) -> uuid::Uuid {
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Test Game {}", uuid::Uuid::new_v4()))
        .bind(vec![2, 3, 4])
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) VALUES ($1, $2, $3, true, false) RETURNING id",
        )
        .bind(game_type_id)
        .bind("test-v1")
        .bind("http://127.0.0.1:1")
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn make_standalone_ctx_deps() -> (
        crate::websocket::GameBroadcaster,
        async_nats::jetstream::Context,
    ) {
        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let client = async_nats::connect(&nats_url).await.unwrap();
        let js = async_nats::jetstream::new(client.clone());
        (crate::websocket::GameBroadcaster::new(client), js)
    }

    #[sqlx::test]
    async fn bump_verb_is_case_insensitive(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "bump-user").await;
        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let ctx = StandaloneCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            user_id,
        };
        for line in ["bump", "Bump", "BUMP"] {
            match dispatch_standalone_server_command(&ctx, line).await {
                Ok(CommandReply::Status(msg)) => {
                    assert_eq!(msg, "No games are waiting on your turn.");
                }
                Ok(_) => panic!("expected Status reply for {line}"),
                Err(CommandError::User(m)) => {
                    panic!("expected Status reply for {line}, got user error: {m}")
                }
                Err(CommandError::Internal(e)) => {
                    panic!("expected Status reply for {line}, got internal error: {e}")
                }
            }
        }
    }

    #[sqlx::test]
    async fn new_rejects_naming_yourself_as_an_opponent(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "self-namer").await;
        let type_name = format!("selftest{}", uuid::Uuid::new_v4().simple());
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(&type_name)
        .bind(vec![2, 3, 4])
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) VALUES ($1, $2, $3, true, false)",
        )
        .bind(game_type_id)
        .bind("test-v1")
        .bind("http://127.0.0.1:1")
        .execute(&pool)
        .await
        .unwrap();

        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let sctx = StandaloneCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            user_id,
        };

        match run_new_command(&sctx, &format!("{type_name} self-namer")).await {
            Err(CommandError::User(msg)) => assert!(
                msg.contains("included in the game automatically"),
                "unexpected user error: {msg}"
            ),
            Err(CommandError::Internal(e)) => panic!("expected User error, got Internal: {e}"),
            Ok(_) => panic!("expected User error, got Ok"),
        }
    }

    #[sqlx::test]
    async fn bump_resends_only_my_turn_games(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "bump-player").await;
        let opp1 = seed_user(&pool, "bump-opp-1").await;
        let opp2 = seed_user(&pool, "bump-opp-2").await;
        let opp3 = seed_user(&pool, "bump-opp-3").await;
        let game_version_id = make_game_version(&pool).await;

        for opp in [opp1, opp2] {
            let game = crate::db::create_game_with_users(
                &pool,
                crate::db::CreateGameOpts {
                    game_version_id,
                    whose_turn: &[],
                    eliminated: &[],
                    placings: &[],
                    points: &[],
                    creator_id: user_id,
                    opponent_ids: &[opp],
                    opponent_emails: &[],
                    bot_slots: &[],
                    chat_id: None,
                    game_state: "initial_state",
                    all_accepted: false,
                },
            )
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE game_players SET is_turn = (user_id IS NOT NULL) WHERE game_id = $1",
                game.id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[opp3],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "initial_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE game_players SET is_turn = (game_bot_id IS NOT NULL) WHERE game_id = $1",
            game.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let reply = bump_reply(&pool, &reqwest::Client::new(), None, user_id)
            .await
            .unwrap();
        assert_eq!(status_msg(reply), "Re-sent 2 games to your active address.");
    }

    // The `bump` command is a direct response to an inbound email: it must
    // re-send even while the user is active on the web (Forced mode bypasses
    // presence suppression that would hold back automated turn emails).
    #[sqlx::test]
    async fn bump_sends_regardless_of_web_presence(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "bump-active").await;
        let opp = seed_user(&pool, "bump-active-opp").await;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind(format!("bump-active-{}@example.com", uuid::Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();

        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[opp],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "initial_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        sqlx::query("UPDATE game_players SET is_turn = (user_id = $1) WHERE game_id = $2")
            .bind(user_id)
            .bind(game.id)
            .execute(&pool)
            .await
            .unwrap();

        let reply = bump_reply(&pool, &reqwest::Client::new(), None, user_id)
            .await
            .unwrap();
        assert_eq!(status_msg(reply), "Re-sent 1 game to your active address.");
    }

    #[sqlx::test]
    async fn new_command_rejects_invalid_bot_type(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "bot-reject-user").await;
        let type_name = format!("botreject{}", uuid::Uuid::new_v4().simple());
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(&type_name)
        .bind(vec![2, 3, 4])
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) VALUES ($1, $2, $3, true, false)",
        )
        .bind(game_type_id)
        .bind("test-v1")
        .bind("http://127.0.0.1:1")
        .execute(&pool)
        .await
        .unwrap();

        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let sctx = StandaloneCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            user_id,
        };

        match run_new_command(&sctx, &format!("{type_name} bot:garbage")).await {
            Err(CommandError::User(msg)) => {
                assert!(
                    msg.contains("garbage"),
                    "error should name the invalid bot: {msg}"
                );
                assert!(msg.contains("easy"), "error should list valid bots: {msg}");
            }
            Err(CommandError::Internal(e)) => {
                panic!("expected User error, got Internal: {e}")
            }
            Ok(_) => panic!("expected User error, got Ok"),
        }

        let game_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM games")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(game_count, 0, "no game should have been created");
    }

    #[sqlx::test]
    async fn new_command_accepts_valid_bot_type(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "bot-accept-user").await;
        let type_name = format!("botaccept{}", uuid::Uuid::new_v4().simple());
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(&type_name)
        .bind(vec![2, 3, 4])
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) VALUES ($1, $2, $3, true, false)",
        )
        .bind(game_type_id)
        .bind("test-v1")
        .bind("http://127.0.0.1:1")
        .execute(&pool)
        .await
        .unwrap();

        let (broadcaster, jetstream) = make_standalone_ctx_deps().await;
        let http_client = reqwest::Client::new();
        let sctx = StandaloneCommandCtx {
            pool: &pool,
            http_client: &http_client,
            broadcaster: &broadcaster,
            jetstream: &jetstream,
            resend: None,
            user_id,
        };

        match run_new_command(&sctx, &format!("{type_name} bot:easy")).await {
            Err(CommandError::User(msg)) => {
                panic!("validation should pass for enabled bot, got User error: {msg}")
            }
            Err(CommandError::Internal(_)) => {}
            Ok(_) => {}
        }
    }
}
