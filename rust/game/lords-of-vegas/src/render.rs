use std::iter;

use brdgme_color::*;
use brdgme_game::Renderer;
use brdgme_markup::ast::{Col, Row};
use brdgme_markup::{Align as A, Node as N};

use crate::board::{Block, Board, BoardTile, Loc, TileOwner, BLOCKS};
use crate::card::casino_card_count;
use crate::casino::CASINOS;
use crate::tile::TILES;
use crate::PlayerState;
use crate::PubState;
use crate::CASINO_CARDS;
use crate::CASINO_TILES;
use crate::PLAYER_DICE;
use crate::PLAYER_OWNER_TOKENS;
use crate::POINT_STOPS;

const TILE_WIDTH: usize = 9;
const TILE_HEIGHT: usize = 4;
const INLAY_WIDTH: usize = 5;
const INLAY_HEIGHT: usize = 2;
const INLAY_TOP: usize = 1;
const INLAY_LEFT: usize = 2;
const ALLEY_FULL_HEIGHT: usize = 3;
const STRIP_FULL_WIDTH: usize = 9;

static UNBUILT_TILE_BG: Color = Color {
    r: 200,
    g: 200,
    b: 200,
};

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        vec![self.render_with_perspective(None)]
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        vec![self.pub_state.render_with_perspective(Some(self.player))]
    }
}

impl PubState {
    pub fn render_with_perspective(&self, perspective: Option<usize>) -> N {
        N::Table(vec![
            vec![(
                A::Center,
                vec![N::Table(vec![vec![(A::Left, vec![self.board.render()])]])],
            )],
            vec![],
            vec![(
                A::Center,
                vec![self.render_player_table(perspective.unwrap_or(0))],
            )],
            vec![],
            vec![(A::Center, vec![self.render_casino_table()])],
        ])
    }

    pub fn render_player_table(&self, perspective: usize) -> N {
        let mut rows: Vec<Row> = vec![];
        rows.push(vec![
            (A::Right, vec![N::Bold(vec![N::text("Player")])]),
            (A::Left, vec![N::text("  ")]),
            (A::Center, vec![N::Bold(vec![N::text("Cash")])]),
            (A::Left, vec![N::text("  ")]),
            (A::Center, vec![N::Bold(vec![N::text("Dice")])]),
            (A::Left, vec![N::text("  ")]),
            (A::Center, vec![N::Bold(vec![N::text("Tokens")])]),
            (A::Left, vec![N::text("  ")]),
            (A::Center, vec![N::Bold(vec![N::text("Points")])]),
        ]);
        let p_len = self.players.len();
        for i in 0..p_len {
            let p = (perspective + i) % p_len;
            let used = self.board.used_resources(p);
            rows.push(vec![
                (A::Right, vec![N::Player(p)]),
                (A::Left, vec![]),
                (A::Center, vec![render_cash(self.players[p].cash)]),
                (A::Left, vec![]),
                (
                    A::Center,
                    vec![N::text(format!("{}", PLAYER_DICE - used.dice))],
                ),
                (A::Left, vec![]),
                (
                    A::Center,
                    vec![N::text(format!("{}", PLAYER_OWNER_TOKENS - used.tokens))],
                ),
                (A::Left, vec![]),
                (
                    A::Center,
                    vec![N::text(format!("{}", POINT_STOPS[self.players[p].points]))],
                ),
            ]);
        }
        N::Table(rows)
    }

    pub fn render_casino_table(&self) -> N {
        let mut casino_names: Row = vec![(A::Right, vec![N::Bold(vec![N::text("Casino")])])];
        let mut remaining_cards: Row = vec![(A::Right, vec![N::Bold(vec![N::text("Cards left")])])];
        let mut remaining_tiles: Row = vec![(A::Right, vec![N::Bold(vec![N::text("Tiles left")])])];
        for casino in CASINOS {
            casino_names.push((A::Left, vec![N::text("  ")]));
            casino_names.push((A::Center, vec![casino.render()]));
            remaining_cards.push((A::Left, vec![]));
            remaining_cards.push((
                A::Center,
                vec![N::text(format!(
                    "{}",
                    CASINO_CARDS - casino_card_count(&self.played, *casino)
                ))],
            ));
            remaining_tiles.push((A::Left, vec![]));
            remaining_tiles.push((
                A::Center,
                vec![N::text(format!(
                    "{}",
                    CASINO_TILES - self.board.casino_tile_count(*casino)
                ))],
            ));
        }
        N::Table(vec![casino_names, remaining_cards, remaining_tiles])
    }
}

fn block_offset(block: Block) -> (usize, usize) {
    (
        match block {
            Block::A | Block::C | Block::E => 0,
            Block::B | Block::D | Block::F => TILE_WIDTH * 3 + STRIP_FULL_WIDTH,
        },
        match block {
            Block::A | Block::B => 0,
            Block::C | Block::D => TILE_HEIGHT * 2 + ALLEY_FULL_HEIGHT,
            Block::E => TILE_HEIGHT * 6 + ALLEY_FULL_HEIGHT * 2,
            Block::F => TILE_HEIGHT * 5 + ALLEY_FULL_HEIGHT * 2,
        },
    )
}

impl Board {
    fn render(&self) -> N {
        let mut layers = vec![];
        for block in BLOCKS {
            let (x, y) = block_offset(*block);
            layers.push((x, y, vec![self.render_block(*block)]));
        }
        N::Canvas(layers)
    }

    fn render_block(&self, block: Block) -> N {
        let mut layers = vec![];
        for lot in 1..=block.max_lot() {
            let loc = Loc { block, lot };
            let x = (lot - 1) % 3;
            let y = (lot - 1) / 3;
            layers.push((
                x * TILE_WIDTH,
                y * TILE_HEIGHT,
                vec![self.get(&loc).render(&loc)],
            ));
        }
        N::Canvas(layers)
    }
}

impl BoardTile {
    fn render(&self, loc: &Loc) -> N {
        let bot_text = format!("{}{:2}", loc.block, loc.lot);
        let player_color: Col = match *self {
            BoardTile::Owned { player }
            | BoardTile::Built {
                owner: Some(TileOwner { player, .. }),
                ..
            } => player.into(),
            _ => WHITE.into(),
        };
        let player_color_fg = player_color.inv().mono();
        let middle_text = match *self {
            BoardTile::Built {
                owner: Some(TileOwner { die, .. }),
                ..
            } => vec![N::Bg(
                player_color,
                vec![N::Fg(
                    player_color_fg,
                    vec![N::Bold(vec![N::text(format!(" {} ", die))])],
                )],
            )],
            _ => vec![
                N::Bg(
                    player_color,
                    vec![N::Fg(
                        player_color_fg,
                        vec![N::text(format!("${:2}", TILES[loc].build_cost))],
                    )],
                ),
                N::text(format!("\n({})", TILES[loc].die)),
            ],
        };

        let border_bg = match *self {
            BoardTile::Built { casino, .. } => *casino.color(),
            _ => UNBUILT_TILE_BG,
        };
        let inlay_bg = WHITE;
        let border_fg = border_bg.inv().mono();
        let inlay_fg = inlay_bg.inv().mono();

        N::Canvas(vec![
            // Tile background
            (
                0,
                0,
                vec![N::Bg(
                    border_bg.into(),
                    vec![N::text(rect(TILE_WIDTH, TILE_HEIGHT))],
                )],
            ),
            // Inlay background
            (
                INLAY_LEFT,
                INLAY_TOP,
                vec![N::Bg(
                    inlay_bg.into(),
                    vec![N::text(rect(INLAY_WIDTH, INLAY_HEIGHT))],
                )],
            ),
            // Middle text
            (
                INLAY_LEFT,
                INLAY_TOP,
                vec![N::Align(
                    A::Center,
                    INLAY_WIDTH,
                    vec![N::Fg(inlay_fg.into(), middle_text)],
                )],
            ),
            // Bot text
            (
                0,
                TILE_HEIGHT - 1,
                vec![N::Align(
                    A::Center,
                    TILE_WIDTH,
                    vec![N::Fg(
                        border_fg.into(),
                        vec![N::Bold(vec![N::text(bot_text)])],
                    )],
                )],
            ),
        ])
    }
}

fn rect(w: usize, h: usize) -> String {
    let line: String = iter::repeat(" ").take(w).collect();
    let mut r = line.clone();
    for _ in 0..h - 1 {
        r.push('\n');
        r.push_str(&line);
    }
    r
}

pub fn render_cash(amount: usize) -> N {
    N::Bold(vec![N::Fg(
        GREEN.into(),
        vec![N::text(format!("${}", amount))],
    )])
}
