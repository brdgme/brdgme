use std::collections::HashMap;

use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::card::*;
use crate::{DIRK, PlayerState, PubBoard, PubState};

fn currency_color(c: Currency) -> NamedColor {
    match c {
        Currency::Blue => NamedColor::Blue,
        Currency::Green => NamedColor::Green,
        Currency::Red => NamedColor::Red,
        Currency::Yellow => NamedColor::Yellow,
    }
}

fn tile_type_color(t: TileType) -> NamedColor {
    match t {
        TileType::Empty => NamedColor::Grey,
        TileType::Fountain => NamedColor::Grey,
        TileType::Pavillion => NamedColor::Cyan,
        TileType::Seraglio => NamedColor::Red,
        TileType::Arcades => NamedColor::Blue,
        TileType::Chambers => NamedColor::Yellow,
        TileType::Garden => NamedColor::Green,
        TileType::Tower => NamedColor::Purple,
    }
}

fn render_card(card: Card) -> N {
    N::Fg(
        currency_color(card.currency).into(),
        vec![N::Bold(vec![N::text(format!("{}", card))])],
    )
}

fn render_cards(cards: &[Card]) -> Vec<N> {
    let mut sorted = cards.to_vec();
    sorted.sort_by(|a, b| {
        let cur_a = Currency::ALL
            .iter()
            .position(|&c| c == a.currency)
            .unwrap_or(0);
        let cur_b = Currency::ALL
            .iter()
            .position(|&c| c == b.currency)
            .unwrap_or(0);
        cur_a.cmp(&cur_b).then(a.value.cmp(&b.value))
    });
    let mut nodes = vec![];
    for (i, c) in sorted.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text("  "));
        }
        nodes.push(render_card(*c));
    }
    nodes
}

fn render_tile_abbr(t: TileType) -> N {
    N::Fg(
        tile_type_color(t).into(),
        vec![N::Bold(vec![N::text(t.abbr().trim().to_string())])],
    )
}

fn tile_counts(grid: &Grid) -> HashMap<TileType, i32> {
    let mut counts = HashMap::new();
    for t in grid.values() {
        if t.tile_type != TileType::Empty {
            *counts.entry(t.tile_type).or_insert(0) += 1;
        }
    }
    counts
}

fn render_grid(grid: &Grid) -> N {
    if grid.is_empty() {
        return N::text("(empty)");
    }
    let (min, max) = grid_bounds(grid);
    let mut rows: Vec<Row> = vec![];

    let mut header: Row = vec![(A::Center, vec![N::text("")])];
    for x in min.x..=max.x {
        let col_letter = ((x - min.x) as u8 + b'a') as char;
        header.push((A::Center, vec![N::text(format!("{}", col_letter))]));
    }
    rows.push(header);

    for y in min.y..=max.y {
        let mut row: Row = vec![(A::Right, vec![N::text(format!("{}", y - min.y + 1))])];
        for x in min.x..=max.x {
            let tile = grid_tile_at(grid, Vect { x, y });
            if tile.tile_type == TileType::Empty {
                row.push((A::Center, vec![N::text(" . ".to_string())]));
            } else {
                row.push((A::Center, vec![render_tile_abbr(tile.tile_type)]));
            }
        }
        rows.push(row);
    }
    table_with_gap(&rows, 0)
}

fn render_tiles_for_purchase(tiles: &[Tile]) -> N {
    let rows: Vec<Row> = Currency::ALL
        .iter()
        .enumerate()
        .filter_map(|(i, &cur)| {
            let tile = tiles.get(i)?;
            if tile.tile_type == TileType::Empty {
                return None;
            }
            Some(vec![
                (A::Left, vec![render_tile_abbr(tile.tile_type)]),
                (
                    A::Left,
                    vec![
                        N::text("cost "),
                        N::Fg(
                            currency_color(cur).into(),
                            vec![N::Bold(vec![N::text(format!("{}", tile.cost))])],
                        ),
                    ],
                ),
            ])
        })
        .collect();
    if rows.is_empty() {
        N::text("No tiles available")
    } else {
        table_with_gap(&rows, 2)
    }
}

fn render_money(cards: &[Card]) -> Vec<N> {
    if cards.is_empty() {
        return vec![N::text("No money available")];
    }
    render_cards(cards)
}

fn render_player_summary(state: &PubState) -> N {
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Player")])]),
        (A::Center, vec![N::Bold(vec![N::text("Pav")])]),
        (A::Center, vec![N::Bold(vec![N::text("Ser")])]),
        (A::Center, vec![N::Bold(vec![N::text("Arc")])]),
        (A::Center, vec![N::Bold(vec![N::text("Cha")])]),
        (A::Center, vec![N::Bold(vec![N::text("Gar")])]),
        (A::Center, vec![N::Bold(vec![N::text("Tow")])]),
        (A::Center, vec![N::Bold(vec![N::text("Wall")])]),
        (A::Center, vec![N::Bold(vec![N::text("Cards")])]),
        (A::Center, vec![N::Bold(vec![N::text("Pts")])]),
    ]];

    for p in 0..state.all_players {
        let board = &state.boards[p];
        let counts = tile_counts(&board.grid);
        let is_dirk = state.human_players == 2 && p == DIRK;

        let wall_str = if is_dirk {
            "N/A".to_string()
        } else {
            format!("{}", grid_longest_ext_wall(&board.grid))
        };
        let cards_str = if is_dirk {
            "N/A".to_string()
        } else {
            format!("{}", board.card_count)
        };

        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (
                A::Center,
                vec![N::text(format!(
                    "{}",
                    counts.get(&TileType::Pavillion).unwrap_or(&0)
                ))],
            ),
            (
                A::Center,
                vec![N::text(format!(
                    "{}",
                    counts.get(&TileType::Seraglio).unwrap_or(&0)
                ))],
            ),
            (
                A::Center,
                vec![N::text(format!(
                    "{}",
                    counts.get(&TileType::Arcades).unwrap_or(&0)
                ))],
            ),
            (
                A::Center,
                vec![N::text(format!(
                    "{}",
                    counts.get(&TileType::Chambers).unwrap_or(&0)
                ))],
            ),
            (
                A::Center,
                vec![N::text(format!(
                    "{}",
                    counts.get(&TileType::Garden).unwrap_or(&0)
                ))],
            ),
            (
                A::Center,
                vec![N::text(format!(
                    "{}",
                    counts.get(&TileType::Tower).unwrap_or(&0)
                ))],
            ),
            (A::Center, vec![N::text(wall_str)]),
            (A::Center, vec![N::text(cards_str)]),
            (
                A::Center,
                vec![N::Bold(vec![N::text(format!("{}", board.points))])],
            ),
        ]);
    }
    table_with_gap(&rows, 2)
}

fn render_place_tiles(tiles: &[Tile]) -> Vec<N> {
    let non_empty = not_empty(tiles);
    if non_empty.is_empty() {
        return vec![];
    }
    let mut nodes = vec![N::Bold(vec![N::text("Tiles to place: ")])];
    for (i, t) in non_empty.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text("  "));
        }
        nodes.push(N::text(format!("{}:", i)));
        nodes.push(render_tile_abbr(t.tile_type));
    }
    nodes
}

fn render_reserve(tiles: &[Tile]) -> Vec<N> {
    let non_empty = not_empty(tiles);
    if non_empty.is_empty() {
        return vec![];
    }
    let mut nodes = vec![N::Bold(vec![N::text("Reserved: ")])];
    for (i, t) in non_empty.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text("  "));
        }
        nodes.push(N::text(format!("{}:", i)));
        nodes.push(render_tile_abbr(t.tile_type));
    }
    nodes
}

fn render_game(state: &PubState, viewer: Option<usize>, hand: Option<&[Card]>) -> Vec<N> {
    let mut rows: Vec<Row> = vec![];

    rows.push(vec![(
        A::Center,
        vec![
            N::Bold(vec![N::text("Round ")]),
            N::text(format!("{}", state.round)),
            N::text("  "),
            N::Bold(vec![N::text("Tiles in bag: ")]),
            N::text(format!("{}", state.tile_bag_len)),
        ],
    )]);
    rows.push(vec![]);

    rows.push(vec![(
        A::Center,
        vec![N::Bold(vec![N::text("Tiles for purchase")])],
    )]);
    rows.push(vec![(
        A::Center,
        vec![render_tiles_for_purchase(&state.tiles)],
    )]);
    rows.push(vec![]);

    rows.push(vec![(
        A::Center,
        vec![N::Bold(vec![N::text("Money available")])],
    )]);
    rows.push(vec![(A::Center, render_money(&state.cards))]);
    rows.push(vec![]);

    rows.push(vec![(A::Center, vec![N::Bold(vec![N::text("Players")])])]);
    rows.push(vec![(A::Center, vec![render_player_summary(state)])]);
    rows.push(vec![]);

    if let Some(hand) = hand {
        rows.push(vec![(A::Center, vec![N::Bold(vec![N::text("Your hand")])])]);
        rows.push(vec![(A::Center, render_cards(hand))]);
        rows.push(vec![]);
    }

    if let Some(p) = viewer {
        let board = &state.boards[p];
        let place_nodes = render_place_tiles(&board.place);
        if !place_nodes.is_empty() {
            rows.push(vec![(A::Center, place_nodes)]);
            rows.push(vec![]);
        }
        let reserve_nodes = render_reserve(&board.reserve);
        if !reserve_nodes.is_empty() {
            rows.push(vec![(A::Center, reserve_nodes)]);
            rows.push(vec![]);
        }
    }

    let start = viewer.unwrap_or(0);
    for i in 0..state.all_players {
        let p = (start + i) % state.all_players;
        let board: &PubBoard = &state.boards[p];
        rows.push(vec![(
            A::Center,
            vec![N::Bold(vec![N::Player(p)]), N::text(" grid")],
        )]);
        rows.push(vec![(A::Center, vec![render_grid(&board.grid)])]);
        rows.push(vec![]);
    }

    vec![N::Table(rows)]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render_game(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render_game(&self.public, Some(self.player), Some(&self.hand))
    }
}
