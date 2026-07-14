use brdgme_color::{IN_USE_MIXES, IN_USE_SOFTENS, NamedColor};

use crate::ast::TNode;
use crate::semantic::{SemanticCol, SemanticColType};

/// Number of player colour slots (matches `Palette::player_colors`).
const PLAYER_COUNT: usize = 8;

/// Generates the static structural CSS rules for every `mk-fg-*`/`mk-bg-*`
/// class this renderer can emit: named colours, the in-use soften variants,
/// their `c-` (contrast) counterparts, and player 0..7 slots. Rules reference
/// the `--mk-*` custom properties produced by `brdgme_color::palette_css_vars`
/// (players reference `--mk-player-{n}`, which the web layer defines itself
/// on the board container).
pub fn markup_class_css() -> String {
    let mut buf = String::new();
    for named in NamedColor::ALL {
        rule(&mut buf, &named.to_string(), &format!("--mk-{}", named));
        rule(
            &mut buf,
            &format!("c-{}", named),
            &format!("--mk-{}-contrast", named),
        );
    }
    for &(named, pct) in IN_USE_SOFTENS {
        let token = format!("soften-{}-{}", named, pct);
        rule(&mut buf, &token, &format!("--mk-{}", token));
        rule(
            &mut buf,
            &format!("c-{}", token),
            &format!("--mk-{}-contrast", token),
        );
    }
    for &(source, target, pct) in IN_USE_MIXES {
        let token = format!("mix-{}-{}-{}", source, target, pct);
        rule(&mut buf, &token, &format!("--mk-{}", token));
        rule(
            &mut buf,
            &format!("c-{}", token),
            &format!("--mk-{}-contrast", token),
        );
    }
    for n in 0..PLAYER_COUNT {
        let token = format!("player-{}", n);
        rule(&mut buf, &token, &format!("--mk-{}", token));
        rule(
            &mut buf,
            &format!("c-{}", token),
            &format!("--mk-{}-contrast", token),
        );
    }
    buf
}

fn rule(buf: &mut String, token: &str, var: &str) {
    buf.push_str(&format!(".mk-fg-{}{{color:var({})}}\n", token, var));
    buf.push_str(&format!(
        ".mk-bg-{}{{background-color:var({})}}\n",
        token, var
    ));
}

fn escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Builds the CSS class token for a colour, e.g. `red`, `soften-pink-75`,
/// `player-2`. Doesn't include the `c-` contrast prefix or the `mk-fg-`/
/// `mk-bg-` role prefix - callers add those.
fn color_token(color: &SemanticColType) -> String {
    match *color {
        SemanticColType::Named { color, soften } => match soften {
            Some(pct) => format!("soften-{}-{}", color, pct),
            None => color.to_string(),
        },
        SemanticColType::Mix {
            source,
            target,
            pct,
        } => {
            format!("mix-{}-{}-{}", source, target, pct)
        }
        SemanticColType::Player(n) => format!("player-{}", n),
    }
}

fn class_token(c: &SemanticCol) -> String {
    let token = color_token(&c.color);
    if c.contrast {
        format!("c-{}", token)
    } else {
        token
    }
}

fn fg(c: &SemanticCol, content: &str) -> String {
    format!(
        r#"<span class="mk-fg-{}">{}</span>"#,
        class_token(c),
        content
    )
}

fn bg(c: &SemanticCol, content: &str) -> String {
    format!(
        r#"<span class="mk-bg-{}">{}</span>"#,
        class_token(c),
        content
    )
}

fn b(content: &str) -> String {
    format!("<b>{}</b>", content)
}

/// Renders semantically-transformed nodes to HTML using CSS classes
/// (`mk-fg-*` / `mk-bg-*`) instead of inline styles, for use with
/// `palette_css_vars`/`markup_class_css`-generated stylesheets.
pub fn html_class(input: &[TNode<SemanticCol>]) -> String {
    render_nodes(input)
}

fn render_nodes(input: &[TNode<SemanticCol>]) -> String {
    let mut buf = String::new();
    for n in input {
        match *n {
            TNode::Text(ref t) => buf.push_str(&escape(t)),
            TNode::Fg(ref color, ref children) => buf.push_str(&fg(color, &render_nodes(children))),
            TNode::Bg(ref color, ref children) => buf.push_str(&bg(color, &render_nodes(children))),
            TNode::Bold(ref children) => buf.push_str(&b(&render_nodes(children))),
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use brdgme_color::NamedColor;

    use super::*;

    fn fg_named(color: NamedColor, soften: Option<u8>, contrast: bool) -> SemanticCol {
        SemanticCol {
            color: SemanticColType::Named { color, soften },
            contrast,
        }
    }

    #[test]
    fn named_works() {
        assert_eq!(
            html_class(&[TNode::Fg(
                fg_named(NamedColor::Red, None, false),
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-fg-red">x</span>"#
        );
        assert_eq!(
            html_class(&[TNode::Bg(
                fg_named(NamedColor::Blue, None, false),
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-bg-blue">x</span>"#
        );
    }

    #[test]
    fn soften_works() {
        assert_eq!(
            html_class(&[TNode::Bg(
                fg_named(NamedColor::Foreground, Some(86), false),
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-bg-soften-foreground-86">x</span>"#
        );
    }

    #[test]
    fn mix_works() {
        assert_eq!(
            html_class(&[TNode::Bg(
                SemanticCol {
                    color: SemanticColType::Mix {
                        source: NamedColor::Red,
                        target: NamedColor::Blue,
                        pct: 50,
                    },
                    contrast: false,
                },
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-bg-mix-red-blue-50">x</span>"#
        );
    }

    #[test]
    fn player_works() {
        assert_eq!(
            html_class(&[TNode::Fg(
                SemanticCol {
                    color: SemanticColType::Player(2),
                    contrast: false,
                },
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-fg-player-2">x</span>"#
        );
    }

    #[test]
    fn contrast_works() {
        assert_eq!(
            html_class(&[TNode::Fg(
                fg_named(NamedColor::Green, None, true),
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-fg-c-green">x</span>"#
        );
        assert_eq!(
            html_class(&[TNode::Fg(
                SemanticCol {
                    color: SemanticColType::Player(2),
                    contrast: true,
                },
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-fg-c-player-2">x</span>"#
        );
        assert_eq!(
            html_class(&[TNode::Bg(
                fg_named(NamedColor::Pink, Some(75), true),
                vec![TNode::text("x")]
            )]),
            r#"<span class="mk-bg-c-soften-pink-75">x</span>"#
        );
    }

    #[test]
    fn bold_nesting_works() {
        assert_eq!(
            html_class(&[TNode::Bold(vec![TNode::Fg(
                fg_named(NamedColor::Red, None, false),
                vec![TNode::text("x")]
            )])]),
            r#"<b><span class="mk-fg-red">x</span></b>"#
        );
    }

    #[test]
    fn markup_class_css_contains_expected_rules() {
        let css = markup_class_css();
        assert!(css.contains(".mk-fg-red{color:var(--mk-red)}"));
        assert!(css.contains(".mk-bg-red{background-color:var(--mk-red)}"));
        assert!(css.contains(".mk-fg-c-green{color:var(--mk-green-contrast)}"));
        assert!(css.contains(".mk-fg-soften-foreground-90{color:var(--mk-soften-foreground-90)}"));
        assert!(css.contains(".mk-fg-c-soften-pink-80{color:var(--mk-soften-pink-80-contrast)}"));
        assert!(css.contains(".mk-fg-player-0{color:var(--mk-player-0)}"));
        assert!(css.contains(".mk-bg-player-0{background-color:var(--mk-player-0)}"));
        assert!(css.contains(".mk-fg-c-player-0{color:var(--mk-player-0-contrast)}"));
        assert!(css.contains(".mk-fg-player-7{color:var(--mk-player-7)}"));
        assert!(!css.contains("player-8"));
    }

    #[test]
    fn escaping_works() {
        assert_eq!(html_class(&[TNode::text("<a & b>")]), "&lt;a &amp; b&gt;");
    }
}
