//! Port of `brdgme-go/roll_through_the_ages_1/render.go`.
//!
//! No hidden information in this game: Go's `PubRender()` is literally
//! `PlayerRender(CurrentPlayer)` - `PubState`'s render below does exactly
//! that, not a neutral/different layout.
//!
//! Every `render.Table(cells, 0, 2)` call in the Go source uses
//! `colSpacing=2`; all six tables here use `table_with_gap(&rows, 2)` to
//! insert the equivalent spacer cells (see `docs/porting/RENDER_PARITY.md`).

use brdgme_color::NamedColor;
use brdgme_game::game::Renderer;
use brdgme_markup::ast::Cell;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::development::DEVELOPMENTS;
use crate::dice::Die;
use crate::good::{Good, goods_reversed};
use crate::monument::MONUMENTS;
use crate::player_board::{BASE_CITY_SIZE, CITY_LEVELS, MAX_CITY_PROGRESS};
use crate::{Game, Phase, PlayerState, PubState};

/// Port of `RenderX`.
fn render_x(player: usize, strong: bool) -> N {
    let x = if strong { "X" } else { "x" };
    let coloured = N::Fg(player.into(), vec![N::text(x)]);
    if strong {
        N::Bold(vec![coloured])
    } else {
        coloured
    }
}

/// Port of `render.BoldIf`.
fn bold_if(node: N, when: bool) -> N {
    if when { N::Bold(vec![node]) } else { node }
}

/// Port of `render.Fgp` for arbitrary text.
fn fgp(player: usize, content: impl Into<String>) -> N {
    N::Fg(player.into(), vec![N::text(content.into())])
}

/// Port of `RenderDice`: colours each occurrence of a known letter within
/// the die's face string (`DiceValueColours`), grouping consecutive
/// same-coloured characters into a single run for a smaller node tree
/// (equivalent output to Go's per-letter `strings.Replace`).
fn render_dice(die: Die) -> Vec<N> {
    let s = die.face_string();
    let mut nodes = vec![];
    let mut run = String::new();
    let mut run_colour = None;
    for c in s.chars() {
        let colour = crate::dice::dice_value_colour(c);
        if colour != run_colour {
            if !run.is_empty() {
                nodes.push(match run_colour {
                    Some(col) => N::Fg(col.into(), vec![N::text(run.clone())]),
                    None => N::text(run.clone()),
                });
            }
            run.clear();
            run_colour = colour;
        }
        run.push(c);
    }
    if !run.is_empty() {
        nodes.push(match run_colour {
            Some(col) => N::Fg(col.into(), vec![N::text(run)]),
            None => N::text(run),
        });
    }
    nodes
}

/// Port of `RenderGoodName`.
fn render_good_name(good: Good) -> N {
    N::Bold(vec![N::Fg(
        good.colour().into(),
        vec![N::text(good.name())],
    )])
}

/// Port of `strings.Title` as applied to lowercase, space-separated names
/// (development names are single lowercase words; monument names are
/// already Title Case, so this is idempotent for them).
fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn cell(content: Vec<N>) -> Cell {
    (A::Left, content)
}

fn cell_align(content: Vec<N>, align: A) -> Cell {
    (align, content)
}

/// Port of `Game.PlayerRender`.
fn render(game: &Game, player: usize) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    // Dice
    let mut dice_row: Row = vec![];
    let mut number_row: Row = vec![];
    for (i, &d) in game.rolled_dice.iter().enumerate() {
        dice_row.push(cell(vec![N::Bold(render_dice(d))]));
        let dice_string = d.face_string();
        number_row.push(cell(vec![
            N::text(" ".repeat(dice_string.len() / 2)),
            N::Fg(NamedColor::Grey.into(), vec![N::text((i + 1).to_string())]),
        ]));
    }
    for &d in game.kept_dice.iter() {
        dice_row.push(cell(render_dice(d)));
    }
    out.push(N::Bold(vec![N::text("Dice")]));
    out.push(N::text(" "));
    out.push(N::Fg(
        NamedColor::Grey.into(),
        vec![N::text("(F: food, W: worker, G: good, C: coin, X: skull)")],
    ));
    out.push(N::text("\n"));
    out.push(table_with_gap(&[dice_row, number_row], 2));
    out.push(N::text("\n\n"));

    // Remaining turns
    if game.final_round {
        out.push(N::Bold(vec![N::text("This is the final round")]));
        out.push(N::text("\n\n"));
    }

    // Turn resources
    match game.phase {
        Phase::Build | Phase::Buy => {
            let cells: Vec<Row> = vec![
                vec![cell(vec![N::Bold(vec![N::text("Turn supplies")])])],
                vec![
                    cell(vec![N::Bold(vec![N::text("Workers:")])]),
                    cell(vec![N::text(game.remaining_workers.to_string())]),
                ],
                vec![
                    cell(vec![N::Bold(vec![N::text("Coins:")])]),
                    cell(vec![N::text(format!(
                        "{} ({} including goods)",
                        game.remaining_coins,
                        game.remaining_coins + game.boards[game.current_player].goods_value(),
                    ))]),
                ],
            ];
            out.push(table_with_gap(&cells, 2));
            out.push(N::text("\n\n"));
        }
        Phase::Trade => {
            let cells: Vec<Row> = vec![
                vec![cell(vec![N::Bold(vec![N::text("Turn supplies")])])],
                vec![
                    cell(vec![N::Bold(vec![N::text("Ships:")])]),
                    cell(vec![N::text(game.remaining_ships.to_string())]),
                ],
            ];
            out.push(table_with_gap(&cells, 2));
            out.push(N::text("\n\n"));
        }
        _ => {}
    }

    // Cities
    out.push(N::Bold(vec![N::text("Cities")]));
    out.push(N::text(" "));
    out.push(N::Fg(
        NamedColor::Grey.into(),
        vec![N::text("(number of dice and food used per turn)")],
    ));
    out.push(N::text("\n"));
    let mut city_header: Vec<N> = vec![N::Bold(vec![N::text(BASE_CITY_SIZE.to_string())])];
    let mut last = 0;
    for (i, &n) in CITY_LEVELS.iter().enumerate() {
        city_header.push(N::text(" ".repeat(((n - last - 1) * 2 + 1) as usize)));
        city_header.push(N::Bold(vec![N::text(
            (BASE_CITY_SIZE + i as i32 + 1).to_string(),
        )]));
        last = n;
    }
    let mut cities_rows: Vec<Row> = vec![vec![
        cell(vec![N::Bold(vec![N::text("Player")])]),
        cell(city_header),
    ]];
    for p in 0..game.players {
        let remaining = MAX_CITY_PROGRESS - game.boards[p].city_progress;
        let marker = render_x(p, p == player);
        let mut progress_nodes: Vec<N> = vec![];
        for _ in 0..(game.boards[p].city_progress + 1) {
            progress_nodes.push(marker.clone());
            progress_nodes.push(N::text(" "));
        }
        for _ in 0..remaining {
            progress_nodes.push(N::Fg(NamedColor::Grey.into(), vec![N::text(".")]));
            progress_nodes.push(N::text(" "));
        }
        let mut row: Row = vec![cell(vec![N::Player(p)]), cell(progress_nodes)];
        if remaining > 0 {
            let left = bold_if(
                N::Fg(
                    NamedColor::Grey.into(),
                    vec![N::text(format!("({} left)", remaining))],
                ),
                p == player,
            );
            row.push(cell(vec![left]));
        }
        cities_rows.push(row);
    }
    out.push(table_with_gap(&cities_rows, 2));
    out.push(N::text("\n\n"));

    // Developments
    let mut dev_header: Row = vec![cell(vec![N::Bold(vec![N::text("Development")])])];
    for p in 0..game.players {
        dev_header.push(cell(vec![N::Player(p)]));
    }
    dev_header.push(cell(vec![N::Bold(vec![N::text("Cost")])]));
    dev_header.push(cell(vec![N::Bold(vec![N::text("Pts")])]));
    dev_header.push(cell(vec![N::Bold(vec![N::text("Effect")])]));
    let mut dev_rows: Vec<Row> = vec![dev_header];
    for &d in DEVELOPMENTS.iter() {
        let dv = d.value();
        let mut row: Row = vec![cell(vec![N::text(title_case(dv.name))])];
        for p in 0..game.players {
            let node = if game.boards[p].developments.contains(&d) {
                render_x(p, player == p)
            } else {
                N::Fg(NamedColor::Grey.into(), vec![N::text(".")])
            };
            row.push(cell_align(vec![node], A::Center));
        }
        row.push(cell(vec![N::text(format!(" {}", dv.cost))]));
        row.push(cell(vec![N::text(format!(" {}", dv.points))]));
        row.push(cell(vec![N::Fg(
            NamedColor::Grey.into(),
            vec![N::text(dv.effect)],
        )]));
        dev_rows.push(row);
    }
    out.push(table_with_gap(&dev_rows, 2));
    out.push(N::text("\n\n"));

    // Monuments
    let mut mon_header: Row = vec![cell(vec![N::Bold(vec![N::text("Monument")])])];
    for p in 0..game.players {
        mon_header.push(cell(vec![N::Player(p)]));
    }
    mon_header.push(cell(vec![N::Bold(vec![N::text("Size")])]));
    mon_header.push(cell(vec![N::Bold(vec![N::text("Pts")])]));
    mon_header.push(cell(vec![N::Bold(vec![N::text("Effect")])]));
    let mut mon_rows: Vec<Row> = vec![mon_header];
    for &m in MONUMENTS.iter() {
        let mv = m.value();
        let mut row: Row = vec![cell(vec![N::text(title_case(mv.name))])];
        for p in 0..game.players {
            let progress = game.boards[p].monuments.get(&m).copied().unwrap_or(0);
            let node = if progress == 0 {
                N::Fg(NamedColor::Grey.into(), vec![N::text(".")])
            } else if progress == mv.size {
                render_x(p, game.boards[p].monument_built_first.contains(&m))
            } else {
                fgp(p, progress.to_string())
            };
            row.push(cell_align(vec![node], A::Center));
        }
        row.push(cell(vec![N::text(format!(" {}", mv.size))]));
        row.push(cell(vec![
            N::Bold(vec![N::text(mv.points.to_string())]),
            N::text(format!("/{}", mv.subsequent_points)),
        ]));
        row.push(cell(vec![N::Fg(
            NamedColor::Grey.into(),
            vec![N::text(mv.effect)],
        )]));
        mon_rows.push(row);
    }
    out.push(table_with_gap(&mon_rows, 2));
    out.push(N::text("\n\n"));

    // Resources
    let mut res_header: Row = vec![cell(vec![N::Bold(vec![N::text("Resource")])])];
    for p in 0..game.players {
        res_header.push(cell(vec![N::Player(p)]));
    }
    let mut res_rows: Vec<Row> = vec![res_header];
    for good in goods_reversed().iter() {
        let mut row: Row = vec![cell(vec![render_good_name(*good)])];
        for p in 0..game.players {
            let num = game.boards[p].goods.get(good).copied().unwrap_or(0);
            let node = if num > 0 {
                bold_if(
                    fgp(
                        p,
                        format!("{} ({})", num, crate::good::good_value(*good, num)),
                    ),
                    p == player,
                )
            } else {
                N::Fg(NamedColor::Grey.into(), vec![N::text(".")])
            };
            row.push(cell_align(vec![node], A::Center));
        }
        res_rows.push(row);
    }
    let mut total_row: Row = vec![cell(vec![N::Bold(vec![N::text("total")])])];
    for p in 0..game.players {
        let node = bold_if(
            fgp(
                p,
                format!(
                    "{} ({})",
                    game.boards[p].goods_num(),
                    game.boards[p].goods_value()
                ),
            ),
            p == player,
        );
        total_row.push(cell_align(vec![node], A::Center));
    }
    res_rows.push(total_row);
    res_rows.push(vec![]);

    let mut food_row: Row = vec![cell(vec![N::Bold(vec![N::Fg(
        NamedColor::Green.into(),
        vec![N::text("food")],
    )])])];
    for p in 0..game.players {
        let node = bold_if(fgp(p, game.boards[p].food.to_string()), p == player);
        food_row.push(cell_align(vec![node], A::Center));
    }
    res_rows.push(food_row);

    let mut ship_row: Row = vec![cell(vec![N::Bold(vec![N::Fg(
        NamedColor::Blue.into(),
        vec![N::text("ship")],
    )])])];
    for p in 0..game.players {
        let node = bold_if(fgp(p, game.boards[p].ships.to_string()), p == player);
        ship_row.push(cell_align(vec![node], A::Center));
    }
    res_rows.push(ship_row);

    let mut disaster_row: Row = vec![cell(vec![N::Bold(vec![N::Fg(
        NamedColor::Red.into(),
        vec![N::text("disaster")],
    )])])];
    for p in 0..game.players {
        let node = bold_if(fgp(p, game.boards[p].disasters.to_string()), p == player);
        disaster_row.push(cell_align(vec![node], A::Center));
    }
    res_rows.push(disaster_row);

    let mut score_row: Row = vec![cell(vec![N::Bold(vec![N::text("score")])])];
    for p in 0..game.players {
        let node = bold_if(fgp(p, game.boards[p].score().to_string()), p == player);
        score_row.push(cell_align(vec![node], A::Center));
    }
    res_rows.push(score_row);

    out.push(table_with_gap(&res_rows, 2));

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(&self.game, self.game.current_player)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.game, self.player)
    }
}
