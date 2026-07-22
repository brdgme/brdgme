//! Shared themed email renderer (#22b Decision 3): one fn producing
//! `{ subject, text, html, headers }` from generic content blocks + the
//! recipient's palette. Markup pipeline (concrete inline colours) for the HTML
//! part wrapped in mrml MJML chrome; `brdgme_markup::plain` (unthemed terminal
//! render) for the text part.

use std::collections::BTreeMap;

use brdgme_color::{NamedColor, Palette};
use brdgme_markup::Player;

/// Generic content blocks for one outbound email. Every text block is raw
/// `brdgme_markup` text. All blocks are optional so the same renderer serves
/// turn notifications, reminders, command responses, invites, and rules dumps.
pub struct EmailContent {
    /// Stable across the thread, e.g. "{Game type} with {opponent names}".
    pub subject: String,
    /// Status/header line (whose turn / result + placings + rating changes).
    pub header: Option<String>,
    /// "Since last time:" digest log lines.
    pub digest: Option<Vec<String>>,
    /// The board render (raw markup).
    pub board: Option<String>,
    /// "You can:" legal command usages.
    pub you_can: Option<Vec<String>>,
    /// "continue playing in your browser" link (plain URL).
    pub browser_url: Option<String>,
    /// "View rules" link (plain URL to /rules/{version_id}).
    pub rules_url: Option<String>,
    /// Footer / unsubscribe text.
    pub footer: Option<String>,
}

/// A fully-rendered email ready for the Resend `text` + `html` fields plus the
/// threading/unsubscribe headers and the reply address.
pub struct RenderedEmail {
    pub subject: String,
    pub text: String,
    pub html: String,
    pub headers: BTreeMap<String, String>,
    // Sent via Resend's dedicated reply_to field, not a header: Resend ignores a
    // custom Reply-To header, so the reply address must ride the API field.
    pub reply_to: String,
}

/// Resolves a stored `users.theme` slug to a concrete palette. NULL/system/
/// unknown -> brdgme light (email cannot see `prefers-color-scheme`; light is
/// the safe default and matches the web `:root` default).
pub fn palette_for_slug(slug: Option<&str>) -> &'static Palette {
    use crate::theme::slugify;
    slug.and_then(|want| {
        brdgme_color::themes()
            .iter()
            .find(|(name, _, _)| slugify(name) == want)
            .map(|(_, _, p)| *p)
    })
    .unwrap_or(&brdgme_color::LIGHT)
}

/// Builds a markup `Player` with a CONCRETE colour resolved from `palette` for
/// a `game_players.color` slot name (e.g. "Green" -> the palette's green).
pub fn player_for_slot(name: &str, color_name: &str, palette: &Palette) -> Player {
    let token = crate::theme::slot_from_color_name(color_name);
    let named: NamedColor = token.parse().unwrap_or(NamedColor::Grey);
    Player {
        name: name.to_string(),
        color: palette.color(named),
    }
}

fn render_block(markup: &str, players: &[Player], palette: &Palette) -> (String, String) {
    let (nodes, _) = brdgme_markup::from_string(markup).unwrap_or_default();
    let tnodes = brdgme_markup::transform_with_palette(&nodes, players, palette);
    (brdgme_markup::html(&tnodes), brdgme_markup::plain(&tnodes))
}

fn fallback_html(bg: &str, fg: &str, body: &str) -> String {
    format!(
        "<html><body style=\"background-color:{bg};\"><pre style=\"background-color:{bg};color:{fg};font-family:'DejaVu Sans Mono','Lucida Console',monospace;line-height:1.2em;white-space:pre;\">{body}</pre></body></html>",
    )
}

/// Renders one outbound game email. `palette`/`players` are the recipient's
/// resolved theme palette and the game's players with concrete colours (build
/// via `palette_for_slug` + `player_for_slot`). `thread_id` is the optional
/// Message-Id local part (e.g. "game-{id}", "proposal-{id}"); when `Some`,
/// `is_first_message` selects Message-Id vs In-Reply-To+References, and when
/// `None` no threading headers are emitted (de-threaded turn mail);
/// `reply_address` is the full reply address (e.g. "g-{token}@brdg.me"), carried
/// on the `reply_to` field.
pub fn render_game_email(
    content: &EmailContent,
    palette: &Palette,
    players: &[Player],
    thread_id: Option<&str>,
    is_first_message: bool,
    reply_address: &str,
) -> RenderedEmail {
    let bg = palette.background.hex();
    let fg = palette.foreground.hex();
    let muted = palette.grey.hex();
    let accent = palette.color(NamedColor::Blue).hex();

    let header = content
        .header
        .as_ref()
        .map(|m| render_block(m, players, palette));
    let board = content
        .board
        .as_ref()
        .map(|m| render_block(m, players, palette));
    let footer = content
        .footer
        .as_ref()
        .map(|m| render_block(m, players, palette));
    let digest: Option<Vec<(String, String)>> = content.digest.as_ref().map(|ls| {
        ls.iter()
            .map(|m| render_block(m, players, palette))
            .collect()
    });
    let you_can: Option<Vec<(String, String)>> = content.you_can.as_ref().map(|ls| {
        ls.iter()
            .map(|m| render_block(m, players, palette))
            .collect()
    });

    let mut body = String::new();
    if let Some((h, _)) = &header {
        body.push_str(h);
        body.push_str("\n\n");
    }
    if let Some(ls) = &digest {
        body.push_str(&format!(
            "<span style=\"color:{muted};\">Since last time:</span>\n"
        ));
        for (h, _) in ls {
            body.push_str(h);
            body.push('\n');
        }
        body.push('\n');
    }
    if let Some((b, _)) = &board {
        body.push_str(b);
        body.push_str("\n\n");
    }
    if let Some(ls) = &you_can {
        body.push_str(&format!("<span style=\"color:{muted};\">You can:</span>\n"));
        for (h, _) in ls {
            body.push_str(h);
            body.push('\n');
        }
        body.push('\n');
    }
    if let Some(url) = &content.browser_url {
        body.push_str(&format!(
            "<a href=\"{url}\" style=\"color:{accent};\">Play in your browser</a>\n\n"
        ));
    }
    if let Some(url) = &content.rules_url {
        body.push_str(&format!(
            "<a href=\"{url}\" style=\"color:{muted};font-size:12px;\">View rules</a>\n\n"
        ));
    }
    if let Some((f, _)) = &footer {
        body.push_str(&format!("<span style=\"color:{muted};\">{f}</span>"));
    }

    let body = body.replace('\n', "<br>");

    // The board lives in a single `<pre>`. It must be wrapped in a real
    // `<tr><td>` cell: a bare `<pre>` as a direct child of `<tbody>` (what
    // `<mj-raw>` otherwise emits) is invalid HTML, so mail clients
    // foster-parent it into the column's `font-size:0px` wrapper div and every
    // glyph collapses to 0px - only elements carrying their own font-size (the
    // rules link) stay visible. The explicit `font-size` on the cell and the
    // `<pre>` is defence in depth against that 0px inheritance.
    let mjml = format!(
        r#"<mjml><mj-body background-color="{bg}"><mj-section><mj-column><mj-raw><tr><td style="padding:0;font-size:13px;"><pre style="background-color:{bg};color:{fg};font-family:'DejaVu Sans Mono','Lucida Console',monospace;font-size:13px;line-height:1.2em;white-space:pre;padding:16px;margin:0;">{body}</pre></td></tr></mj-raw></mj-column></mj-section></mj-body></mjml>"#,
    );

    let html = mrml::parse(&mjml)
        .ok()
        .and_then(|p| {
            p.element
                .render(&mrml::prelude::render::RenderOptions::default())
                .ok()
        })
        .unwrap_or_else(|| fallback_html(&bg, &fg, &body));

    let mut text = String::new();
    if let Some((_, p)) = &header {
        text.push_str(p);
        text.push_str("\n\n");
    }
    if let Some(ls) = &digest {
        text.push_str("Since last time:\n");
        for (_, p) in ls {
            text.push_str(p);
            text.push('\n');
        }
        text.push('\n');
    }
    if let Some((_, p)) = &board {
        text.push_str(p);
        text.push_str("\n\n");
    }
    if let Some(ls) = &you_can {
        text.push_str("You can:\n");
        for (_, p) in ls {
            text.push_str(p);
            text.push('\n');
        }
        text.push('\n');
    }
    if let Some(url) = &content.browser_url {
        text.push_str(&format!("Play in your browser: {url}\n\n"));
    }
    if let Some(url) = &content.rules_url {
        text.push_str(&format!("View rules: {url}\n\n"));
    }
    if let Some((_, p)) = &footer {
        text.push_str(p);
    }

    let mut headers = BTreeMap::new();
    if let Some(thread_id) = thread_id {
        let msg_id = format!("<{thread_id}@brdg.me>");
        if is_first_message {
            headers.insert("Message-Id".to_string(), msg_id);
        } else {
            headers.insert("In-Reply-To".to_string(), msg_id.clone());
            headers.insert("References".to_string(), msg_id);
        }
    }
    headers.insert(
        "List-Unsubscribe".to_string(),
        "<mailto:unsubscribe@brdg.me?subject=unsubscribe>".to_string(),
    );
    headers.insert(
        "List-Unsubscribe-Post".to_string(),
        "List-Unsubscribe=One-Click".to_string(),
    );

    RenderedEmail {
        subject: content.subject.clone(),
        text,
        html,
        headers,
        reply_to: reply_address.to_string(),
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use brdgme_color::{DARK, DRACULA, LIGHT};

    fn two_players(palette: &Palette) -> Vec<Player> {
        vec![
            player_for_slot("Alice", "Green", palette),
            player_for_slot("Bob", "Red", palette),
        ]
    }

    fn minimal_content() -> EmailContent {
        EmailContent {
            subject: "S".into(),
            header: None,
            digest: None,
            board: None,
            you_can: None,
            browser_url: None,
            rules_url: None,
            footer: None,
        }
    }

    fn full_content() -> EmailContent {
        EmailContent {
            subject: "Chess with Bob".into(),
            header: Some("It is your turn.".into()),
            digest: Some(vec!["Bob moved e2-e4".into()]),
            board: Some("{{fg green}}board-here{{/fg}}".into()),
            you_can: Some(vec!["move ## ##".into()]),
            browser_url: Some("https://brdg.me/games/abc".into()),
            rules_url: Some("https://brdg.me/rules/abc".into()),
            footer: Some("Reply to play. Unsubscribe anytime.".into()),
        }
    }

    #[test]
    fn palette_for_slug_resolves_named_theme() {
        assert_eq!(palette_for_slug(Some("dracula")), &DRACULA);
        assert_eq!(palette_for_slug(Some("brdgme-dark")), &DARK);
    }

    #[test]
    fn palette_for_slug_defaults_to_brdgme_light() {
        assert_eq!(palette_for_slug(None), &LIGHT);
        assert_eq!(palette_for_slug(Some("system")), &LIGHT);
        assert_eq!(palette_for_slug(Some("no-such-theme")), &LIGHT);
    }

    #[test]
    fn player_for_slot_resolves_concrete_colour() {
        assert_eq!(player_for_slot("A", "Green", &DRACULA).color, DRACULA.green);
        assert_eq!(
            player_for_slot("A", "Amber", &DRACULA).color,
            DRACULA.orange
        );
    }

    #[test]
    fn render_game_email_html_contains_board_and_theme_colours() {
        let email = render_game_email(
            &full_content(),
            &DRACULA,
            &two_players(&DRACULA),
            Some("game-abc"),
            true,
            "g-tok@brdg.me",
        );
        assert!(!email.html.is_empty());
        assert!(email.html.contains("board-here"));
        assert!(email.html.contains(&DRACULA.green.hex()));
        assert!(!email.html.contains("var(--mk-"));
        assert!(email.html.contains("It is your turn."));
        assert!(email.html.contains("Since last time:"));
        assert!(email.html.contains("Bob moved e2-e4"));
        assert!(email.html.contains("You can:"));
        assert!(email.html.contains("move ## ##"));
        assert!(email.html.contains("https://brdg.me/games/abc"));
        assert!(email.html.contains("View rules"));
        assert!(email.html.contains("Reply to play"));
    }

    #[test]
    fn render_game_email_html_pre_is_valid_table_content_with_font_size() {
        let email = render_game_email(
            &full_content(),
            &DRACULA,
            &two_players(&DRACULA),
            Some("game-abc"),
            true,
            "g-tok@brdg.me",
        );
        // A bare `<pre>` directly inside `<tbody>` is invalid HTML: mail
        // clients foster-parent it into the column's `font-size:0px` wrapper
        // div, collapsing every glyph to 0px (only elements with their own
        // font-size, e.g. the rules link, stay visible). The board must sit in
        // a real table cell and carry its own font-size.
        assert!(
            !email.html.contains("<tbody><pre"),
            "<pre> must not be a bare child of <tbody>: {}",
            email.html
        );
        let pre_start = email.html.find("<pre").expect("html has a <pre>");
        let pre_tag_end = email.html[pre_start..].find('>').expect("<pre> closes");
        let pre_tag = &email.html[pre_start..pre_start + pre_tag_end];
        assert!(
            pre_tag.contains("font-size"),
            "<pre> must declare an explicit font-size so it does not inherit the column's 0px: {pre_tag}"
        );
    }

    #[test]
    fn render_game_email_html_uses_br_between_blocks() {
        let email = render_game_email(
            &full_content(),
            &DRACULA,
            &two_players(&DRACULA),
            Some("game-abc"),
            true,
            "g-tok@brdg.me",
        );
        assert!(
            email.html.contains("It is your turn.<br /><br />"),
            "header must be followed by <br /><br />: {}",
            email.html
        );
        assert!(
            email.html.contains("Since last time:</span><br />"),
            "digest label must be followed by <br />: {}",
            email.html
        );
        assert!(
            email.html.contains("Bob moved e2-e4<br />"),
            "digest line must end with <br />: {}",
            email.html
        );
        assert!(
            email.html.contains("You can:</span><br />"),
            "you_can label must be followed by <br />: {}",
            email.html
        );
    }

    #[test]
    fn render_game_email_omits_absent_blocks() {
        let content = EmailContent {
            subject: "S".into(),
            header: None,
            digest: None,
            board: Some("{{player 0}} plays".into()),
            you_can: None,
            browser_url: None,
            rules_url: None,
            footer: None,
        };
        let email = render_game_email(
            &content,
            &LIGHT,
            &two_players(&LIGHT),
            Some("game-abc"),
            true,
            "g-tok@brdg.me",
        );
        assert!(!email.html.contains("Since last time:"));
        assert!(!email.html.contains("You can:"));
        assert!(!email.html.contains("Play in your browser"));
        assert!(!email.html.contains("View rules"));
        assert!(email.html.contains("Alice"));
        assert!(email.html.contains("plays"));
    }

    #[test]
    fn render_game_email_text_is_plain_unthemed() {
        let email = render_game_email(
            &full_content(),
            &DRACULA,
            &two_players(&DRACULA),
            Some("game-abc"),
            true,
            "g-tok@brdg.me",
        );
        assert!(!email.text.contains(&DRACULA.green.hex()));
        assert!(!email.text.contains("<span"));
        assert!(email.text.contains("board-here"));
        assert!(email.text.contains("It is your turn."));
        assert!(email.text.contains("Since last time:"));
        assert!(email.text.contains("You can:"));
        assert!(email.text.contains("https://brdg.me/games/abc"));
        assert!(email.text.contains("View rules: https://brdg.me/rules/abc"));
    }

    #[test]
    fn headers_first_message_sets_message_id() {
        let email = render_game_email(
            &minimal_content(),
            &LIGHT,
            &[],
            Some("game-abc"),
            true,
            "g-tok@brdg.me",
        );
        assert_eq!(
            email.headers.get("Message-Id").map(String::as_str),
            Some("<game-abc@brdg.me>")
        );
        assert_eq!(email.headers.get("In-Reply-To"), None);
        assert_eq!(email.headers.get("References"), None);
        assert_eq!(email.reply_to, "g-tok@brdg.me");
        assert_eq!(
            email.headers.get("List-Unsubscribe").map(String::as_str),
            Some("<mailto:unsubscribe@brdg.me?subject=unsubscribe>")
        );
        assert_eq!(
            email
                .headers
                .get("List-Unsubscribe-Post")
                .map(String::as_str),
            Some("List-Unsubscribe=One-Click")
        );
    }

    #[test]
    fn headers_later_message_sets_in_reply_to_and_references() {
        let email = render_game_email(
            &minimal_content(),
            &LIGHT,
            &[],
            Some("game-abc"),
            false,
            "g-tok@brdg.me",
        );
        assert_eq!(email.headers.get("Message-Id"), None);
        assert_eq!(
            email.headers.get("In-Reply-To").map(String::as_str),
            Some("<game-abc@brdg.me>")
        );
        assert_eq!(
            email.headers.get("References").map(String::as_str),
            Some("<game-abc@brdg.me>")
        );
    }

    #[test]
    fn headers_parameterised_for_invite_and_settings() {
        let invite = render_game_email(
            &minimal_content(),
            &LIGHT,
            &[],
            Some("proposal-123"),
            true,
            "i-tok@brdg.me",
        );
        assert_eq!(
            invite.headers.get("Message-Id").map(String::as_str),
            Some("<proposal-123@brdg.me>")
        );
        assert_eq!(invite.reply_to, "i-tok@brdg.me");

        let settings = render_game_email(
            &minimal_content(),
            &LIGHT,
            &[],
            Some("settings-u1"),
            false,
            "s-tok@brdg.me",
        );
        assert_eq!(
            settings.headers.get("In-Reply-To").map(String::as_str),
            Some("<settings-u1@brdg.me>")
        );
        assert_eq!(settings.reply_to, "s-tok@brdg.me");
    }

    #[test]
    fn reply_to_field_carries_reply_address() {
        let email = render_game_email(
            &minimal_content(),
            &LIGHT,
            &[],
            Some("game-abc"),
            true,
            "g-tok@brdg.me",
        );
        assert_eq!(email.reply_to, "g-tok@brdg.me");
    }

    #[test]
    fn headers_none_thread_id_sets_no_threading_headers() {
        let email = render_game_email(&minimal_content(), &LIGHT, &[], None, true, "g-tok@brdg.me");
        assert_eq!(email.headers.get("Message-Id"), None);
        assert_eq!(email.headers.get("In-Reply-To"), None);
        assert_eq!(email.headers.get("References"), None);
        assert_eq!(email.reply_to, "g-tok@brdg.me");
        assert_eq!(
            email.headers.get("List-Unsubscribe").map(String::as_str),
            Some("<mailto:unsubscribe@brdg.me?subject=unsubscribe>")
        );
    }
}
