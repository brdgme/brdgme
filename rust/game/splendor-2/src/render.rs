//! Ported from `brdgme-go/splendor_1/render.go`. Go's `render.Table(rows,
//! rowSpacing, colSpacing)` inserts literal blank spacer cells between every
//! column - Rust has no such parameter on `Node::Table`, so
//! `brdgme_markup::table_with_gap` is used instead (it inserts the same kind
//! of blank spacer cells). `rowSpacing` is always 0 in the Go source so no
//! row spacers are needed anywhere in this file.

use brdgme_color::{CYAN, GREY};
use brdgme_game::Renderer;
use brdgme_markup::ast::Cell;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::card::{Card, GEMS, RESOURCES, Resource};
use crate::cost::{self, Cost};
use crate::{PlayerState, PubState};

/// Ported from `render.go`'s `ResourceAbbr`.
fn resource_abbr(r: Resource) -> &'static str {
    match r {
        Resource::Diamond => "Diam",
        Resource::Sapphire => "Saph",
        Resource::Emerald => "Emer",
        Resource::Ruby => "Ruby",
        Resource::Onyx => "Onyx",
        Resource::Gold => "Gold",
        Resource::Prestige => "VP",
    }
}

/// Port of `RenderResourceColour` (`render.go`).
fn render_resource_colour(text: impl Into<String>, r: Resource) -> N {
    N::Fg((&crate::resource_color(r)).into(), vec![N::text(text)])
}

fn grey(nodes: Vec<N>) -> N {
    N::Fg((&GREY).into(), nodes)
}

fn cel(align: A, nodes: Vec<N>) -> Cell {
    (align, nodes)
}

fn blank_cell() -> Cell {
    (A::Left, vec![])
}

/// Port of `RenderAmount` (`render.go`): each non-zero resource (in fixed
/// `Resources` order) as a bold coloured number, joined with a grey "-".
fn render_amount(a: &Cost) -> Vec<N> {
    let mut parts: Vec<Vec<N>> = vec![];
    for &r in RESOURCES.iter() {
        let n = a.get(r);
        if n > 0 {
            parts.push(vec![N::Bold(vec![render_resource_colour(
                n.to_string(),
                r,
            )])]);
        }
    }
    let mut out: Vec<N> = vec![];
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            out.push(grey(vec![N::text("-")]));
        }
        out.extend(part);
    }
    out
}

/// Port of `RenderCardBonusVP` (`render.go`).
fn render_card_bonus_vp(c: &Card) -> N {
    let mut parts = vec![render_resource_colour(
        resource_abbr(c.resource),
        c.resource,
    )];
    if c.prestige > 0 {
        parts.push(N::text(" "));
        parts.push(render_resource_colour(
            c.prestige.to_string(),
            Resource::Prestige,
        ));
    }
    N::Bold(parts)
}

/// One board/reserve card cell pair (upper: afford-marker + bonus/VP, lower:
/// cost). `afford` is `(bonuses, buying_power)` for the viewing player, only
/// present in player views (`pNum >= 0` in Go).
fn card_cells(c: &Card, afford: Option<(&Cost, &Cost)>) -> (Cell, Cell) {
    let mut upper: Vec<N> = vec![];
    if let Some((bonuses, buying_power)) = afford {
        if cost::can_afford(bonuses, &c.cost) {
            upper.push(N::Bold(vec![N::Fg(
                (&brdgme_color::GREEN).into(),
                vec![N::text("X ")],
            )]));
        } else if cost::can_afford(buying_power, &c.cost) {
            upper.push(N::Bold(vec![N::Fg(
                (&brdgme_color::YELLOW).into(),
                vec![N::text("X ")],
            )]));
        }
    }
    upper.push(render_card_bonus_vp(c));
    (
        cel(A::Center, upper),
        cel(A::Center, render_amount(&c.cost)),
    )
}

/// Port of `PlayerRender` (`render.go`), shared by `PubState` (`player =
/// None`, matching Go's `pNum == -1`) and `PlayerState` (`player = Some((idx,
/// reserve))`, the viewing player's own reserve cards - the only hidden
/// information in this game).
fn render(pub_state: &PubState, player: Option<(usize, &[Card])>) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    let bonuses_tokens = player.map(|(p, _)| {
        let pb = &pub_state.player_boards[p];
        (pb.bonuses.clone(), pb.tokens.clone())
    });
    let afford = bonuses_tokens
        .as_ref()
        .map(|(bonuses, tokens)| (bonuses, bonuses.add(tokens)));

    // Nobles.
    let mut noble_header: Row = vec![blank_cell()];
    let mut noble_row: Row = vec![cel(
        A::Left,
        vec![grey(vec![
            N::text("Nobles ("),
            N::Bold(vec![render_resource_colour("3", Resource::Prestige)]),
            N::text(" each)"),
        ])],
    )];
    for (i, n) in pub_state.nobles.iter().enumerate() {
        noble_header.push(cel(
            A::Center,
            vec![grey(vec![N::text((i + 1).to_string())])],
        ));
        noble_row.push(cel(A::Left, render_amount(&n.cost)));
    }
    out.push(table_with_gap(&[noble_header, noble_row], 2));
    out.push(N::text("\n\n"));

    // Board.
    let mut longest_row = pub_state.board.iter().map(|r| r.len()).max().unwrap_or(0);
    if let Some((_, reserve)) = player {
        longest_row = longest_row.max(reserve.len());
    }
    let mut header: Row = vec![blank_cell()];
    for i in 0..longest_row {
        header.push(cel(
            A::Center,
            vec![N::Bold(vec![grey(vec![N::text(
                ((b'A' + i as u8) as char).to_string(),
            )])])],
        ));
    }
    let mut rows: Vec<Row> = vec![header];
    for (l, r) in pub_state.board.iter().enumerate() {
        let mut upper: Row = vec![cel(
            A::Left,
            vec![grey(vec![
                N::text("Level "),
                N::Bold(vec![N::text((l + 1).to_string())]),
            ])],
        )];
        let mut lower: Row = vec![blank_cell()];
        for c in r {
            let (u, lo) = card_cells(c, afford.as_ref().map(|(b, bp)| (*b, bp)));
            upper.push(u);
            lower.push(lo);
        }
        rows.push(upper);
        rows.push(lower);
        rows.push(vec![]);
    }
    let mut upper: Row = vec![cel(
        A::Left,
        vec![grey(vec![N::text("Level "), N::Bold(vec![N::text("4")])])],
    )];
    let mut lower: Row = vec![cel(A::Left, vec![grey(vec![N::text("Reserved")])])];
    if let Some((_, reserve)) = player {
        for c in reserve {
            let (u, lo) = card_cells(c, afford.as_ref().map(|(b, bp)| (*b, bp)));
            upper.push(u);
            lower.push(lo);
        }
    }
    rows.push(upper);
    rows.push(lower);
    out.push(table_with_gap(&rows, 3));
    out.push(N::text("\n\n\n"));

    // Tokens.
    let mut table_header: Row = vec![blank_cell()];
    let mut your_token_row: Row = vec![cel(A::Left, vec![N::Bold(vec![N::text("You have")])])];
    let mut your_token_desc_row: Row = vec![cel(
        A::Left,
        vec![N::Bold(vec![grey(vec![N::text("(card+token)")])])],
    )];
    let mut avail_token_row: Row = vec![cel(A::Left, vec![N::Bold(vec![N::text("Tokens left")])])];
    for &gem in GEMS.iter().chain([Resource::Gold].iter()) {
        table_header.push(cel(
            A::Center,
            vec![N::Bold(vec![render_resource_colour(
                resource_abbr(gem),
                gem,
            )])],
        ));
        if let Some((bonuses, tokens)) = &bonuses_tokens {
            your_token_row.push(cel(
                A::Center,
                vec![N::Bold(vec![N::text(
                    (bonuses.get(gem) + tokens.get(gem)).to_string(),
                )])],
            ));
            let desc_cell = if gem != Resource::Gold {
                vec![grey(vec![N::text(format!(
                    "({}+{})",
                    bonuses.get(gem),
                    tokens.get(gem)
                ))])]
            } else {
                vec![]
            };
            your_token_desc_row.push(cel(A::Center, desc_cell));
        }
        avail_token_row.push(cel(
            A::Center,
            vec![N::text(pub_state.tokens.get(gem).to_string())],
        ));
    }
    let mut rows: Vec<Row> = vec![table_header];
    if bonuses_tokens.is_some() {
        rows.push(your_token_row);
        rows.push(your_token_desc_row);
    }
    rows.push(avail_token_row);
    out.push(table_with_gap(&rows, 3));
    out.push(N::text("\n\n\n"));

    // Player table.
    let mut header: Row = vec![blank_cell()];
    for &gem in GEMS.iter() {
        header.push(cel(
            A::Center,
            vec![N::Bold(vec![render_resource_colour(
                resource_abbr(gem),
                gem,
            )])],
        ));
    }
    header.push(cel(
        A::Center,
        vec![N::Bold(vec![render_resource_colour(
            resource_abbr(Resource::Gold),
            Resource::Gold,
        )])],
    ));
    header.push(cel(A::Center, vec![N::Bold(vec![N::text("Tok")])]));
    header.push(cel(
        A::Center,
        vec![N::Bold(vec![N::Fg((&CYAN).into(), vec![N::text("Res")])])],
    ));
    header.push(cel(
        A::Center,
        vec![N::Bold(vec![render_resource_colour(
            resource_abbr(Resource::Prestige),
            Resource::Prestige,
        )])],
    ));
    header.push(cel(A::Center, vec![N::Bold(vec![N::text("Dev")])]));
    let mut rows: Vec<Row> = vec![header];
    for p in 0..pub_state.players {
        let bold = player.map(|(pi, _)| pi == p).unwrap_or(false);
        let pb = &pub_state.player_boards[p];
        let mut row: Row = vec![cel(A::Left, vec![N::Player(p)])];
        for &gem in GEMS.iter() {
            let text = N::text(format!("{}+{}", pb.bonuses.get(gem), pb.tokens.get(gem)));
            row.push(cel(
                A::Center,
                if bold {
                    vec![N::Bold(vec![text])]
                } else {
                    vec![text]
                },
            ));
        }
        let cells = [
            pb.tokens.get(Resource::Gold).to_string(),
            pb.tokens.sum().to_string(),
            pb.reserve_count.to_string(),
            pb.prestige.to_string(),
            pb.card_count.to_string(),
        ];
        for text in cells {
            row.push(cel(
                A::Center,
                if bold {
                    vec![N::Bold(vec![N::text(text)])]
                } else {
                    vec![N::text(text)]
                },
            ));
        }
        rows.push(row);
    }
    out.push(table_with_gap(&rows, 2));

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some((self.player, &self.reserve)))
    }
}
