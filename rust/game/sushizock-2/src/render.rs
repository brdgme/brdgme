use brdgme_color::{BLUE, GREY, RED};
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{DieFace, PlayerState, PubState, Tile, TileType};

pub fn tile(t: &Tile) -> N {
    let color = match t.kind {
        TileType::Blue => BLUE,
        TileType::Red => RED,
    };
    N::Fg(color.into(), vec![N::text(t.value.to_string())])
}

pub fn bold_dice(dice: &[DieFace]) -> N {
    let mut nodes: Vec<N> = vec![];
    for (i, d) in dice.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text("  "));
        }
        nodes.push(N::Bold(vec![die_node(*d)]));
    }
    N::Group(nodes)
}

fn die_node(d: DieFace) -> N {
    let (color, ch) = match d {
        DieFace::Sushi => (BLUE, "\u{0398}"),
        DieFace::BlueChopsticks => (BLUE, "X"),
        DieFace::Bones => (RED, "\u{00a5}"),
        DieFace::RedChopsticks => (RED, "X"),
    };
    N::Fg(color.into(), vec![N::text(ch)])
}

pub fn dice_row_bold_then_normal(rolled: &[DieFace], kept: &[DieFace]) -> N {
    let mut nodes: Vec<N> = vec![];
    for (i, d) in rolled.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text("  "));
        }
        nodes.push(N::Bold(vec![die_node(*d)]));
    }
    if !rolled.is_empty() && !kept.is_empty() {
        nodes.push(N::text("  "));
    }
    for (i, d) in kept.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text("  "));
        }
        nodes.push(die_node(*d));
    }
    N::Group(nodes)
}

pub fn bold_tile_list(tiles: &[Tile]) -> N {
    let mut nodes: Vec<N> = vec![];
    for (i, t) in tiles.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(" "));
        }
        nodes.push(N::Bold(vec![tile(t)]));
    }
    N::Group(nodes)
}

fn tile_cells(tiles: &[Tile], highlight_idx: Option<usize>) -> Row {
    tiles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let node = if highlight_idx == Some(i) {
                N::Bold(vec![tile(t)])
            } else {
                tile(t)
            };
            (A::Left, vec![node])
        })
        .collect()
}

fn render(pub_state: &PubState, _player: Option<usize>) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    // Dice section
    out.push(N::Bold(vec![N::text("Dice\n")]));
    if !pub_state.rolled_dice.is_empty() || !pub_state.kept_dice.is_empty() {
        let mut dice_nodes: Vec<N> = vec![];
        for (i, d) in pub_state.rolled_dice.iter().enumerate() {
            if i > 0 {
                dice_nodes.push(N::text("  "));
            }
            dice_nodes.push(N::Bold(vec![die_node(*d)]));
        }
        if !pub_state.rolled_dice.is_empty() && !pub_state.kept_dice.is_empty() {
            dice_nodes.push(N::text("  "));
        }
        for (i, d) in pub_state.kept_dice.iter().enumerate() {
            if i > 0 {
                dice_nodes.push(N::text("  "));
            }
            dice_nodes.push(die_node(*d));
        }
        out.push(N::Group(dice_nodes));
        // Position numbers for rolled dice
        if !pub_state.rolled_dice.is_empty() {
            let mut pos_nodes: Vec<N> = vec![];
            for (i, _) in pub_state.rolled_dice.iter().enumerate() {
                if i > 0 {
                    pos_nodes.push(N::text("  "));
                }
                pos_nodes.push(N::Fg(GREY.into(), vec![N::text((i + 1).to_string())]));
            }
            out.push(N::text("\n"));
            out.push(N::Group(pos_nodes));
        }
        out.push(N::text("\n"));
    }

    // Tiles section
    out.push(N::Bold(vec![N::text("\nTiles\n")]));
    let counts = crate::dice_counts_pub(pub_state);
    let blue_highlight = if counts.sushi > 0 && counts.sushi <= pub_state.blue_tiles.len() {
        Some(counts.sushi - 1)
    } else {
        None
    };
    let red_highlight = if counts.bones > 0 && counts.bones <= pub_state.red_tiles.len() {
        Some(counts.bones - 1)
    } else {
        None
    };
    let tile_rows: Vec<Row> = vec![
        tile_cells(&pub_state.blue_tiles, blue_highlight),
        tile_cells(&pub_state.red_tiles, red_highlight),
    ];
    out.push(table_with_gap(&tile_rows, 1));
    out.push(N::text("\n\n"));

    // Players table
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Player")])]),
        (A::Left, vec![N::Bold(vec![N::text("Blue")])]),
        (A::Left, vec![N::Bold(vec![N::text("Red")])]),
    ]];
    for p in 0..pub_state.players {
        let blue_text: Vec<N> = if !pub_state.player_blue_tiles[p].is_empty() {
            let last = &pub_state.player_blue_tiles[p][pub_state.player_blue_tiles[p].len() - 1];
            vec![
                tile(last),
                N::Fg(
                    GREY.into(),
                    vec![N::text(format!(
                        " ({} tiles)",
                        pub_state.player_blue_tiles[p].len()
                    ))],
                ),
            ]
        } else {
            vec![N::Fg(GREY.into(), vec![N::text("none")])]
        };
        let red_text: Vec<N> = if !pub_state.player_red_tiles[p].is_empty() {
            let last = &pub_state.player_red_tiles[p][pub_state.player_red_tiles[p].len() - 1];
            vec![
                tile(last),
                N::Fg(
                    GREY.into(),
                    vec![N::text(format!(
                        " ({} tiles)",
                        pub_state.player_red_tiles[p].len()
                    ))],
                ),
            ]
        } else {
            vec![N::Fg(GREY.into(), vec![N::text("none")])]
        };
        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (A::Left, blue_text),
            (A::Left, red_text),
        ]);
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
        render(&self.public, Some(self.player))
    }
}
