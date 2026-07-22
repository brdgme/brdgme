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

fn corner_char(grid: &Grid, x: i32, y: i32) -> char {
    let cur = grid_tile_at(grid, Vect { x, y });
    let up = grid_tile_at(grid, Vect { x, y: y - 1 });
    let left = grid_tile_at(grid, Vect { x: x - 1, y });
    let up_left = grid_tile_at(grid, Vect { x: x - 1, y: y - 1 });

    let any_present = [&cur, &up, &left, &up_left]
        .iter()
        .any(|t| t.tile_type != TileType::Empty);
    if !any_present {
        return '▒';
    }

    let bit_up = up.has_wall(Dir::Left) || up_left.has_wall(Dir::Right);
    let bit_down = cur.has_wall(Dir::Left) || left.has_wall(Dir::Right);
    let bit_left = up_left.has_wall(Dir::Down) || left.has_wall(Dir::Up);
    let bit_right = up.has_wall(Dir::Down) || cur.has_wall(Dir::Up);

    match (bit_up, bit_down, bit_left, bit_right) {
        (true, true, true, true) => '╬',
        (true, true, true, false) => '╣',
        (true, true, false, true) => '╠',
        (true, false, true, true) => '╩',
        (false, true, true, true) => '╦',
        (true, false, true, false) => '╝',
        (true, false, false, true) => '╚',
        (false, true, true, false) => '╗',
        (false, true, false, true) => '╔',
        (false, false, true, true) => '═',
        (false, false, true, false) => '═',
        (false, false, false, true) => '═',
        (true, true, false, false) => '║',
        (true, false, false, false) => '║',
        (false, true, false, false) => '║',
        (false, false, false, false) => ' ',
    }
}

fn upper_str(grid: &Grid, x: i32, y: i32) -> String {
    let cur = grid_tile_at(grid, Vect { x, y });
    let above = grid_tile_at(grid, Vect { x, y: y - 1 });
    if cur.has_wall(Dir::Up) || above.has_wall(Dir::Down) {
        "═══".to_string()
    } else if cur.tile_type == TileType::Empty && above.tile_type == TileType::Empty {
        "▒▒▒".to_string()
    } else {
        "   ".to_string()
    }
}

fn left_char(grid: &Grid, x: i32, y: i32) -> char {
    let cur = grid_tile_at(grid, Vect { x, y });
    let left = grid_tile_at(grid, Vect { x: x - 1, y });
    if cur.has_wall(Dir::Left) || left.has_wall(Dir::Right) {
        '║'
    } else if cur.tile_type == TileType::Empty && left.tile_type == TileType::Empty {
        '▒'
    } else {
        ' '
    }
}

fn centre_str(grid: &Grid, x: i32, y: i32) -> String {
    let tile = grid_tile_at(grid, Vect { x, y });
    if tile.tile_type == TileType::Empty {
        "▒▒▒".to_string()
    } else {
        tile.tile_type.abbr().to_string()
    }
}

fn render_grid(grid: &Grid) -> N {
    if grid.is_empty() {
        return N::text("(empty)");
    }
    let (min, max) = grid_bounds(grid);
    let x_start = min.x - 1;
    let x_end = max.x + 1;
    let y_start = min.y - 1;
    let y_end = max.y + 1;

    let mut lines: Vec<String> = vec![];

    let mut header = "    ".to_string();
    for x in x_start..=x_end {
        let col_letter = ((x - x_start) as u8 + b'a') as char;
        header.push_str(&format!(" {}  ", col_letter));
    }
    lines.push(header);

    for y in y_start..=y_end {
        let label = format!("{:>2}", y - min.y + 2);
        let mut line1 = format!("{}  ", label);
        let mut line2 = format!("{}  ", label);
        for x in x_start..=x_end {
            line1.push(corner_char(grid, x, y));
            line1.push_str(&upper_str(grid, x, y));
            line2.push(left_char(grid, x, y));
            line2.push_str(&centre_str(grid, x, y));
        }
        lines.push(line1);
        lines.push(line2);
    }

    N::text(lines.join("\n"))
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

fn render_tile_block(t: &Tile) -> (String, String, String) {
    let up = if t.has_wall(Dir::Up) {
        "═══"
    } else {
        "   "
    };
    let left = if t.has_wall(Dir::Left) { '║' } else { ' ' };
    let right = if t.has_wall(Dir::Right) { '║' } else { ' ' };
    let down = if t.has_wall(Dir::Down) {
        "═══"
    } else {
        "   "
    };
    let top_left = match (t.has_wall(Dir::Up), t.has_wall(Dir::Left)) {
        (true, true) => '╔',
        (true, false) => '═',
        (false, true) => '║',
        (false, false) => ' ',
    };
    let top_right = match (t.has_wall(Dir::Up), t.has_wall(Dir::Right)) {
        (true, true) => '╗',
        (true, false) => '═',
        (false, true) => '║',
        (false, false) => ' ',
    };
    let bot_left = match (t.has_wall(Dir::Down), t.has_wall(Dir::Left)) {
        (true, true) => '╚',
        (true, false) => '═',
        (false, true) => '║',
        (false, false) => ' ',
    };
    let bot_right = match (t.has_wall(Dir::Down), t.has_wall(Dir::Right)) {
        (true, true) => '╝',
        (true, false) => '═',
        (false, true) => '║',
        (false, false) => ' ',
    };
    (
        format!("{}{}{}", top_left, up, top_right),
        format!("{}{}{}", left, t.tile_type.abbr(), right),
        format!("{}{}{}", bot_left, down, bot_right),
    )
}

fn render_tile_set(label: &str, tiles: &[Tile]) -> Vec<N> {
    let non_empty = not_empty(tiles);
    if non_empty.is_empty() {
        return vec![];
    }
    let mut tops = vec![];
    let mut mids = vec![];
    let mut bots = vec![];
    let mut indices = vec![];
    for (i, t) in non_empty.iter().enumerate() {
        let (top, mid, bot) = render_tile_block(t);
        tops.push(top);
        mids.push(mid);
        bots.push(bot);
        indices.push(format!(" {}  ", i + 1));
    }
    let sep = "  ";
    let text = format!(
        "{}\n{}\n{}\n{}\n{}",
        label,
        tops.join(sep),
        mids.join(sep),
        bots.join(sep),
        indices.join(sep),
    );
    vec![N::text(text)]
}

fn render_place_tiles(tiles: &[Tile]) -> Vec<N> {
    render_tile_set("Tiles to place:", tiles)
}

fn render_reserve(tiles: &[Tile]) -> Vec<N> {
    render_tile_set("Reserved:", tiles)
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
