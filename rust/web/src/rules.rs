use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::MainLayout;

/// The three rendered HTML sections the rules page shows. `rules` is always
/// present (the column is `NOT NULL DEFAULT ''`); `basic_strategy`/
/// `advanced_strategy` are `None` when the game has no such doc (V1 interface,
/// or a V2 game that didn't author the file) and the section is omitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedDocs {
    pub rules: String,
    pub basic_strategy: Option<String>,
    pub advanced_strategy: Option<String>,
}

/// The `/rules/{version_id}` page: rules + strategy docs for a game version,
/// pre-rendered to HTML by `get_rendered_rules`. Strategy sections are
/// omitted entirely when the game has no such doc.
#[component]
pub fn RulesPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let version_id = move || {
        params
            .get()
            .get("version_id")
            .as_deref()
            .and_then(|id| Uuid::parse_str(id).ok())
    };

    let docs: LocalResource<Result<RenderedDocs, ServerFnError>> =
        LocalResource::new(move || async move {
            match version_id() {
                Some(id) => get_rendered_rules(id).await,
                None => Err(ServerFnError::new("Invalid version ID")),
            }
        });

    view! {
        <MainLayout>
            <div class="rules-page content-page">
                <h1>"Rules"</h1>
                {move || match docs.get() {
                    None => view! { <crate::components::Spinner/> }.into_any(),
                    Some(Err(e)) => view! { <div class="error">"Error: " {e.to_string()}</div> }.into_any(),
                    Some(Ok(docs)) => {
                        let RenderedDocs { rules, basic_strategy, advanced_strategy } = docs;
                        view! {
                            <section class="rules-section">
                                <h2 id="rules">"Rules"</h2>
                                <div class="rules-doc" inner_html=rules></div>
                            </section>
                            {basic_strategy.map(|html| view! {
                                <section class="rules-section">
                                    <h2 id="basic-strategy">"Basic Strategy"</h2>
                                    <div class="rules-doc" inner_html=html></div>
                                </section>
                            })}
                            {advanced_strategy.map(|html| view! {
                                <section class="rules-section">
                                    <h2 id="advanced-strategy">"Advanced Strategy"</h2>
                                    <div class="rules-doc" inner_html=html></div>
                                </section>
                            })}
                        }.into_any()
                    }
                }}
            </div>
        </MainLayout>
    }
}

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("invalid markup in a brdgme render block")]
    Markup,
    #[error("render block references player {index} but only {count} players exist")]
    PlayerOutOfRange { index: usize, count: usize },
}

/// Recursively asserts every `{{player N}}` reference (both `Node::Player` and
/// `ColType::Player` colour refs) is within `0..count`. The markup lib silently
/// tolerates out-of-range indices; the rules renderer must fail loudly instead
/// (an authoring-error signal per docs/authoring/RULES_AUTHORING.md).
#[cfg(feature = "ssr")]
fn validate_player_indices(nodes: &[brdgme_markup::Node], count: usize) -> Result<(), RenderError> {
    use brdgme_markup::Node;
    for n in nodes {
        match n {
            Node::Player(p) => {
                if *p >= count {
                    return Err(RenderError::PlayerOutOfRange { index: *p, count });
                }
            }
            Node::Fg(col, children) | Node::Bg(col, children) => {
                if let brdgme_markup::ast::ColType::Player(p) = col.color
                    && p >= count
                {
                    return Err(RenderError::PlayerOutOfRange { index: p, count });
                }
                validate_player_indices(children, count)?;
            }
            Node::Group(children) | Node::Bold(children) => {
                validate_player_indices(children, count)?;
            }
            Node::Align(_, _, children) | Node::Indent(_, children) => {
                validate_player_indices(children, count)?;
            }
            Node::Table(rows) => {
                for row in rows {
                    for (_align, cells) in row {
                        validate_player_indices(cells, count)?;
                    }
                }
            }
            Node::Canvas(layers) => {
                for (_x, _y, children) in layers {
                    validate_player_indices(children, count)?;
                }
            }
            Node::Text(_) => {}
        }
    }
    Ok(())
}

/// Renders one ```` ```brdgme ```` fence's markup contents through the SAME
/// semantic pipeline the live game render uses (`transform_semantic` +
/// `html_class`), wrapped in a `game-render` board container carrying the
/// shared `player_style` vars so a rules render looks identical to gameplay.
#[cfg(feature = "ssr")]
fn render_fence(
    content: &str,
    players: &[brdgme_markup::SemanticPlayer],
    player_style: &str,
) -> Result<String, RenderError> {
    let (nodes, _) = brdgme_markup::from_string(content).map_err(|_| RenderError::Markup)?;
    validate_player_indices(&nodes, players.len())?;
    let inner = brdgme_markup::html_class(&brdgme_markup::transform_semantic(&nodes, players));
    Ok(format!(
        r#"<div class="game-render" style="{}"><pre>{}</pre></div>"#,
        player_style, inner
    ))
}

/// Renders a non-fence markdown chunk to HTML via pulldown-cmark.
#[cfg(feature = "ssr")]
fn render_markdown(markdown: &str, out: &mut String) {
    let mut opts = pulldown_cmark::Options::empty();
    opts.insert(pulldown_cmark::Options::ENABLE_TABLES);
    opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    opts.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
    let parser = pulldown_cmark::Parser::new_ext(markdown, opts);
    pulldown_cmark::html::push_html(out, parser);
}

/// The shared render fn (handover D3): line-scan `markdown` for ```` ```brdgme
/// ```` fences (always on their own lines per the authoring convention), feed
/// non-fence chunks through pulldown-cmark and fence contents through the
/// semantic markup pipeline, concatenating in original order. Player indices in
/// fences are validated UPFRONT and fail loudly. Reused for rules + basic +
/// advanced strategy.
#[cfg(feature = "ssr")]
pub fn render_doc(
    markdown: &str,
    players: &[brdgme_markup::SemanticPlayer],
    player_style: &str,
) -> Result<String, RenderError> {
    let mut html = String::new();
    let mut prose = String::new();
    let mut in_fence = false;
    let mut fence = String::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if !in_fence && trimmed == "```brdgme" {
            if !prose.is_empty() {
                render_markdown(&prose, &mut html);
                prose.clear();
            }
            in_fence = true;
            fence.clear();
        } else if in_fence && trimmed == "```" {
            html.push_str(&render_fence(&fence, players, player_style)?);
            in_fence = false;
            fence.clear();
        } else if in_fence {
            if !fence.is_empty() {
                fence.push('\n');
            }
            fence.push_str(line);
        } else {
            if !prose.is_empty() {
                prose.push('\n');
            }
            prose.push_str(line);
        }
    }
    if !prose.is_empty() {
        render_markdown(&prose, &mut html);
    }
    Ok(html)
}

/// Builds the synthetic players + container style for a rules render: count =
/// `max(player_counts)` (a SET of supported counts, e.g. `{2,4}` -> 4), palette
/// = `theme::PLAYER_COLOR_NAMES` (the same 8-colour palette real games assign at
/// creation). Colours stay symbolic (semantic path) so they follow the viewer's
/// active theme.
#[cfg(feature = "ssr")]
pub fn synthetic_players(player_counts: &[i32]) -> (Vec<brdgme_markup::SemanticPlayer>, String) {
    let max = player_counts.iter().copied().max().unwrap_or(2) as usize;
    let players: Vec<brdgme_markup::SemanticPlayer> = (0..max)
        .map(|i| brdgme_markup::SemanticPlayer {
            name: format!("Player {}", i + 1),
        })
        .collect();
    let palette_len = crate::theme::PLAYER_COLOR_NAMES.len();
    let slots: Vec<&str> = crate::theme::PLAYER_COLOR_NAMES[..max.min(palette_len)]
        .iter()
        .map(|n| crate::theme::slot_from_color_name(n))
        .collect();
    let player_style = crate::theme::player_style_vars(&slots);
    (players, player_style)
}

/// Strategy is fetched LIVE from the game-service V2 endpoints (there are no
/// strategy DB columns). V1 games (`interface_version < 2`) have no strategy
/// endpoints -> `None`.
#[cfg(feature = "ssr")]
fn strategy_supported(interface_version: i32) -> bool {
    interface_version >= 2
}

/// Empty/whitespace-only strategy docs (a V2 game that didn't author the file)
/// are treated as absent -> `None` so the section is omitted.
#[cfg(feature = "ssr")]
fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

/// Fetches basic + advanced strategy for a game version. Returns
/// `(basic, advanced)`; each is `None` when absent/empty or when the game is V1.
/// The strategy handlers are static `include_str!` content that IGNORE their
/// `game`/`player` request fields (verified: `rust/lib/cmd/src/requester/gamer.rs`
/// matches `BasicStrategy { .. }` and never parses `game`), so an empty `game`
/// string works - no throwaway `Request::New` state is needed.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch_strategy(
    http: &reqwest::Client,
    uri: &str,
    name: &str,
    interface_version: i32,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    use brdgme_cmd::api::{Request, Response};
    if !strategy_supported(interface_version) {
        return Ok((None, None));
    }
    let basic = match crate::game::client::request(
        http,
        uri,
        name,
        &Request::BasicStrategy {
            game: String::new(),
            player: 0,
        },
    )
    .await?
    {
        Response::BasicStrategy { strategy } => strategy,
        _ => {
            return Err(anyhow::anyhow!(
                "unexpected response to BasicStrategy request"
            ));
        }
    };
    let advanced = match crate::game::client::request(
        http,
        uri,
        name,
        &Request::AdvancedStrategy {
            game: String::new(),
            player: 0,
        },
    )
    .await?
    {
        Response::AdvancedStrategy { strategy } => strategy,
        _ => {
            return Err(anyhow::anyhow!(
                "unexpected response to AdvancedStrategy request"
            ));
        }
    };
    Ok((non_empty(basic), non_empty(advanced)))
}

#[server(GetRenderedRules, "/api")]
pub async fn get_rendered_rules(version_id: Uuid) -> Result<RenderedDocs, ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::error::internal;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    let _user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let rules_src = crate::db::find_game_version_rules(&pool, version_id)
        .await
        .map_err(internal("get_rendered_rules: find rules"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;

    let (uri, name, interface_version) =
        crate::db::find_game_version_render_meta(&pool, version_id)
            .await
            .map_err(internal("get_rendered_rules: find render meta"))?
            .ok_or_else(|| ServerFnError::new("Game version not found"))?;

    let player_counts = crate::db::find_game_type_player_counts(&pool, version_id)
        .await
        .map_err(internal("get_rendered_rules: find player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;

    let (players, player_style) = synthetic_players(&player_counts);

    let rules = render_doc(&rules_src, &players, &player_style)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (basic_src, advanced_src) = fetch_strategy(&http_client, &uri, &name, interface_version)
        .await
        .map_err(internal("get_rendered_rules: fetch strategy"))?;

    let basic_strategy = match basic_src {
        Some(src) => Some(
            render_doc(&src, &players, &player_style)
                .map_err(|e| ServerFnError::new(e.to_string()))?,
        ),
        None => None,
    };
    let advanced_strategy = match advanced_src {
        Some(src) => Some(
            render_doc(&src, &players, &player_style)
                .map_err(|e| ServerFnError::new(e.to_string()))?,
        ),
        None => None,
    };

    Ok(RenderedDocs {
        rules,
        basic_strategy,
        advanced_strategy,
    })
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use sqlx::PgPool;

    fn players(n: usize) -> Vec<brdgme_markup::SemanticPlayer> {
        (0..n)
            .map(|i| brdgme_markup::SemanticPlayer {
                name: format!("Player {}", i + 1),
            })
            .collect()
    }

    const STYLE: &str = "--mk-player-0: var(--mk-green);";

    #[test]
    fn prose_only_renders_markdown() {
        let html = render_doc("# Title\n\nSome **bold** text.", &players(2), STYLE).unwrap();
        assert!(html.contains("<h1>Title</h1>"), "got: {html}");
        assert!(html.contains("<strong>bold</strong>"), "got: {html}");
        assert!(!html.contains("game-render"), "no fence -> no board");
    }

    #[test]
    fn gfm_table_renders_html_table() {
        let md = "| Col A | Col B |\n| ----- | ----- |\n| a1 | b1 |";
        let html = render_doc(md, &players(2), STYLE).unwrap();
        assert!(html.contains("<table>"), "got: {html}");
        assert!(html.contains("<th>Col A</th>"), "got: {html}");
        assert!(html.contains("<td>a1</td>"), "got: {html}");
    }

    #[test]
    fn single_fence_intercepted_and_rendered() {
        let md = "Intro text.\n\n```brdgme\n{{player 0}} plays a card\n```\n\nAfter text.";
        let html = render_doc(md, &players(2), STYLE).unwrap();
        assert!(html.contains("<p>Intro text.</p>"), "got: {html}");
        assert!(html.contains("<p>After text.</p>"), "got: {html}");
        assert!(
            html.contains("game-render"),
            "fence wrapped in board: {html}"
        );
        assert!(
            html.contains("mk-fg-player-0"),
            "semantic player class: {html}"
        );
        assert!(
            html.contains("Player 1"),
            "player 0 resolves to Player 1: {html}"
        );
        assert!(
            html.contains(STYLE),
            "board carries the player style: {html}"
        );
    }

    #[test]
    fn multiple_fences_all_rendered() {
        let md = "a\n\n```brdgme\n{{player 0}}\n```\n\nmid\n\n```brdgme\n{{player 1}}\n```\n\nz";
        let html = render_doc(md, &players(2), STYLE).unwrap();
        assert_eq!(html.matches("game-render").count(), 2, "two boards: {html}");
        assert!(html.contains("mk-fg-player-0"), "got: {html}");
        assert!(html.contains("mk-fg-player-1"), "got: {html}");
    }

    #[test]
    fn out_of_range_player_token_errors_loudly() {
        let md = "```brdgme\n{{player 5}}\n```";
        let err = render_doc(md, &players(2), STYLE).unwrap_err();
        assert!(
            matches!(err, RenderError::PlayerOutOfRange { index: 5, count: 2 }),
            "got: {err:?}"
        );
    }

    #[test]
    fn out_of_range_player_color_errors_loudly() {
        let md = "```brdgme\n{{fg player(9)}}x{{/fg}}\n```";
        let err = render_doc(md, &players(2), STYLE).unwrap_err();
        assert!(
            matches!(err, RenderError::PlayerOutOfRange { index: 9, count: 2 }),
            "got: {err:?}"
        );
    }

    #[test]
    fn in_range_player_indices_ok() {
        let md = "```brdgme\n{{player 0}} {{player 1}}\n```";
        assert!(render_doc(md, &players(2), STYLE).is_ok());
    }

    #[test]
    fn synthetic_player_count_is_max_of_player_counts() {
        let (p, style) = synthetic_players(&[2, 4]);
        assert_eq!(p.len(), 4);
        assert_eq!(p[0].name, "Player 1");
        assert_eq!(p[3].name, "Player 4");
        assert!(style.contains("--mk-player-3"), "style: {style}");
        // Palette order: Green, Red, Blue, Orange...
        assert!(
            style.contains("--mk-player-0: var(--mk-green)"),
            "style: {style}"
        );
        assert!(
            style.contains("--mk-player-1: var(--mk-red)"),
            "style: {style}"
        );
    }

    #[test]
    fn synthetic_players_defaults_to_two_when_no_counts() {
        let (p, _) = synthetic_players(&[]);
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn strategy_supported_gates_on_interface_version() {
        assert!(!strategy_supported(1));
        assert!(strategy_supported(2));
        assert!(strategy_supported(3));
    }

    #[test]
    fn non_empty_treats_blank_as_absent() {
        assert_eq!(non_empty(String::new()), None);
        assert_eq!(non_empty("   \n".to_string()), None);
        assert_eq!(non_empty("x".to_string()), Some("x".to_string()));
    }

    // DB integration test (runs in CI where Postgres exists; fails to connect
    // locally - the known #40 condition, not a regression). Uses plain queries
    // for setup to avoid `.sqlx` cache churn.
    #[sqlx::test]
    async fn find_game_version_rules_and_render_meta(pool: PgPool) {
        let gt_id = Uuid::new_v4();
        sqlx::query("INSERT INTO game_types (id, name, player_counts) VALUES ($1, $2, $3)")
            .bind(gt_id)
            .bind("Test Game")
            .bind(vec![2i32, 4])
            .execute(&pool)
            .await
            .unwrap();

        let gv_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO game_versions \
             (id, game_type_id, name, uri, is_public, is_deprecated, rules, interface_version) \
             VALUES ($1, $2, $3, $4, true, false, $5, $6)",
        )
        .bind(gv_id)
        .bind(gt_id)
        .bind("v1")
        .bind("http://127.0.0.1:8100")
        .bind("# Rules\n\n```brdgme\n{{player 0}}\n```")
        .bind(2)
        .execute(&pool)
        .await
        .unwrap();

        let rules = crate::db::find_game_version_rules(&pool, gv_id)
            .await
            .unwrap();
        assert!(
            rules.as_deref().is_some_and(|r| r.contains("# Rules")),
            "rules column round-trips: {rules:?}"
        );

        let meta = crate::db::find_game_version_render_meta(&pool, gv_id)
            .await
            .unwrap();
        assert_eq!(
            meta,
            Some(("http://127.0.0.1:8100".to_string(), "v1".to_string(), 2))
        );

        // A missing version id yields None (not an error).
        assert_eq!(
            crate::db::find_game_version_rules(&pool, Uuid::new_v4())
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            crate::db::find_game_version_render_meta(&pool, Uuid::new_v4())
                .await
                .unwrap(),
            None
        );
    }
}
