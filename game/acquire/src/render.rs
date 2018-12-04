use brdgme_game::Renderer;
use brdgme_markup::{row_pad, Align as A, Node as N, Row};
use brdgme_color::*;

use crate::PlayerState;
use crate::PubState;
use crate::board::{self, Board, Loc, Tile};
use crate::corp::{Corp, GAME_END_SIZE, MAJOR_MULT, MINOR_MULT};
use crate::CanEnd;
use crate::CanEndFalse;

use std::iter::repeat;

const TILE_WIDTH: usize = 5;
const TILE_HEIGHT: usize = 2;

static EMPTY_COLOR_EVEN: Color = Color {
    r: 220,
    g: 220,
    b: 220,
};

static EMPTY_COLOR_ODD: Color = Color {
    r: 190,
    g: 190,
    b: 190,
};

static UNINCORPORATED_COLOR: Color = Color {
    r: 100,
    g: 100,
    b: 100,
};

static UNAVAILABLE_LOC_TEXT_COLOR: Color = Color {
    r: 80,
    g: 80,
    b: 80,
};

static AVAILABLE_LOC_BG: Color = Color {
    r: 248,
    g: 187,
    b: 208,
};

fn render(pub_state: &PubState, player: Option<usize>, tiles: &[Loc]) -> Vec<N> {
    vec![
        N::Table(vec![
            vec![(A::Center, vec![pub_state.board.render(tiles)])],
            vec![],
            vec![(A::Center, vec![pub_state.can_end().render_end_text()])],
            vec![(A::Center, vec![pub_state.render_remaining_tiles_text()])],
            vec![],
            vec![(A::Center, vec![pub_state.corp_table()])],
            vec![],
            vec![(A::Center, vec![pub_state.player_table(player)])],
        ]),
    ]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, &[])
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), &self.tiles)
    }
}

static CORP_TABLE_HEADER: &'static [&'static str] =
    &["Corporation", "Size", "Value", "Shares", "Minor", "Major"];

const ROW_PAD: &str = "   ";

impl PubState {
    fn corp_table(&self) -> N {
        let mut rows: Vec<Row> = vec![
            row_pad(
                &CORP_TABLE_HEADER
                    .iter()
                    .map(|h| (A::Left, vec![N::Bold(vec![N::text(*h)])]))
                    .collect::<Row>(),
                ROW_PAD,
            ),
        ];
        rows.extend(
            Corp::iter()
                .map(|c| {
                    let size = self.board.corp_size(c);
                    let value = c.value(size);
                    row_pad(
                        &[
                            (A::Left, vec![c.render()]),
                            (A::Left, vec![N::text(format!("{}", size))]),
                            (A::Left, vec![N::text(format!("${}", value))]),
                            (
                                A::Left,
                                vec![
                                    N::text(format!(
                                        "{} left",
                                        self.shares.get(c).expect("expected corp to have shares")
                                    )),
                                ],
                            ),
                            (A::Left, vec![N::text(format!("${}", value * MINOR_MULT))]),
                            (A::Left, vec![N::text(format!("${}", value * MAJOR_MULT))]),
                        ],
                        ROW_PAD,
                    )
                })
                .collect::<Vec<Row>>(),
        );
        N::Table(rows)
    }

    fn render_remaining_tiles_text(&self) -> N {
        N::Fg(
            GREY.into(),
            vec![
                N::text("Draw tiles remaining: "),
                N::Bold(vec![N::text(format!("{}", self.remaining_tiles))]),
            ],
        )
    }

    fn player_table(&self, player: Option<usize>) -> N {
        let mut rows: Vec<Row> = vec![self.player_header()];
        let num_players = self.players.len();
        for p_offset in 0..num_players {
            let p = player
                .map(|p| (p + p_offset) % num_players)
                .unwrap_or(p_offset);
            rows.push(self.player_row(p));
        }
        N::Table(rows)
    }

    fn player_header(&self) -> Row {
        let mut header_row: Row = vec![
            (A::Left, vec![N::Bold(vec![N::text("Player")])]),
            (A::Left, vec![N::Bold(vec![N::text("Cash")])]),
        ];
        for c in Corp::iter() {
            header_row.push((A::Left, vec![c.render_abbrev()]));
        }
        row_pad(&header_row, ROW_PAD)
    }

    fn player_row(&self, player: usize) -> Row {
        let mut player_row: Row = vec![
            (A::Left, vec![N::Player(player)]),
            (
                A::Left,
                vec![N::text(format!("${}", self.players[player].money))],
            ),
        ];
        for c in Corp::iter() {
            player_row.push((
                A::Left,
                vec![
                    N::text(format!(
                        "{}",
                        self.players[player].shares.get(c).cloned().unwrap_or(0)
                    )),
                ],
            ));
        }
        row_pad(&player_row, ROW_PAD)
    }
}

fn tile_background(c: Color) -> N {
    N::Bg(
        c.into(),
        vec![
            N::text(
                repeat(repeat(" ").take(TILE_WIDTH).collect::<String>())
                    .take(TILE_HEIGHT)
                    .collect::<Vec<String>>()
                    .join("\n"),
            ),
        ],
    )
}

fn empty_color(l: Loc) -> Color {
    if (l.row + l.col) % 2 == 0 {
        EMPTY_COLOR_EVEN
    } else {
        EMPTY_COLOR_ODD
    }
}

fn corp_main_text_thin(c: &Corp, size: usize) -> Vec<N> {
    vec![
        N::Fg(
            c.color().inv().mono().into(),
            vec![
                N::Align(
                    A::Center,
                    TILE_WIDTH,
                    vec![N::text(format!("{}\n${}", c.abbrev(), c.value(size)))],
                ),
            ],
        ),
    ]
}

fn corp_main_text_wide(c: &Corp, size: usize) -> Vec<N> {
    let mut c_name = c.name();
    c_name.truncate(TILE_WIDTH * 2 - 2);
    vec![
        N::Fg(
            c.color().inv().mono().into(),
            vec![
                N::Align(
                    A::Center,
                    TILE_WIDTH * 2,
                    vec![N::text(format!("{}\n${}", c_name, c.value(size)))],
                ),
            ],
        ),
    ]
}

impl Board {
    pub fn render(&self, player_tiles: &[Loc]) -> N {
        let mut layers = vec![];
        // Tile backgrounds and location text.
        for l in Loc::all() {
            let render_x = l.col * TILE_WIDTH;
            let render_y = l.row * TILE_HEIGHT;
            match self.get_tile(&l) {
                Tile::Empty => {
                    layers.push((render_x, render_y, vec![tile_background(empty_color(l))]));
                    layers.push((
                        render_x,
                        render_y,
                        vec![
                            N::Align(
                                A::Center,
                                TILE_WIDTH,
                                vec![
                                    N::Fg(
                                        UNAVAILABLE_LOC_TEXT_COLOR.into(),
                                        vec![N::text(l.name())],
                                    ),
                                ],
                            ),
                        ],
                    ));
                }
                Tile::Unincorporated => {
                    layers.push((
                        render_x,
                        render_y,
                        vec![tile_background(UNINCORPORATED_COLOR)],
                    ));
                }
                Tile::Corp(ref c) => {
                    layers.push((render_x, render_y, vec![tile_background(c.color())]));
                }
                Tile::Discarded => {}
            }
        }
        // Player tiles.
        for t in player_tiles {
            let l = *t;
            let render_x = l.col * TILE_WIDTH;
            let render_y = l.row * TILE_HEIGHT;
            layers.push((
                render_x,
                render_y,
                vec![tile_background(AVAILABLE_LOC_BG)],
            ));
            layers.push((
                render_x,
                render_y,
                vec![
                    N::Align(
                        A::Center,
                        TILE_WIDTH,
                        vec![
                            N::Bold(vec![
                                N::Fg(
                                    AVAILABLE_LOC_BG.inv().mono().into(),
                                    vec![N::text(l.name())],
                                ),
                            ]),
                        ],
                    ),
                ],
            ));
        }
        // Corp text.
        layers.extend(
            Corp::iter()
                .flat_map(|c| {
                    let mut c_text = vec![];
                    // Find the widest lines.
                    // `widths` is a tuple of x, y, width.
                    let widths: Vec<(usize, usize, usize)> = board::rows()
                        .flat_map(|row| {
                            let mut start: Option<usize> = None;
                            board::cols()
                                .filter_map(|col| {
                                    let l = Loc { row: row, col: col };
                                    match self.get_tile(&l) {
                                        Tile::Corp(tc) if tc == *c => {
                                            if start.is_none() {
                                                start = Some(col);
                                            }
                                            if col == board::WIDTH - 1 {
                                                Some(
                                                    (start.unwrap(), row, col - start.unwrap() + 1),
                                                )
                                            } else {
                                                None
                                            }
                                        }
                                        _ => if let Some(s) = start {
                                            start = None;
                                            Some((s, row, col - s))
                                        } else {
                                            None
                                        },
                                    }
                                })
                                .collect::<Vec<(usize, usize, usize)>>()
                        })
                        .collect();
                    if !widths.is_empty() {
                        let (x, y, w) = widths[(widths.len() - 1) / 2];
                        c_text.push((
                            (x + (w - 1) / 2) * TILE_WIDTH,
                            y * TILE_HEIGHT,
                            if w > 1 {
                                corp_main_text_wide(c, self.corp_size(c))
                            } else {
                                corp_main_text_thin(c, self.corp_size(c))
                            },
                        ));
                    }
                    c_text
                })
                .collect::<Vec<(usize, usize, Vec<N>)>>(),
        );
        N::Table(vec![vec![(A::Left, vec![N::Canvas(layers)])]])
    }
}

impl Corp {
    pub fn render_in_color(self, content: Vec<N>) -> N {
        N::Fg(self.color().into(), vec![N::Bold(content)])
    }

    pub fn render_name(self) -> N {
        self.render_in_color(vec![N::text(self.name())])
    }

    pub fn render_abbrev(self) -> N {
        self.render_in_color(vec![N::text(self.abbrev())])
    }
}

impl CanEnd {
    fn render_end_text(&self) -> N {
        N::Fg(
            GREY.into(),
            vec![
                match *self {
                    CanEnd::Triggered => {
                        N::Bold(vec![N::text("The game will end at the end of this turn")])
                    }
                    CanEnd::Finished => N::Bold(vec![N::text("The game has ended")]),
                    CanEnd::True => N::Bold(vec![N::text("The end of the game can be triggered")]),
                    CanEnd::False(ref caf) => caf.render_end_text(),
                },
            ],
        )
    }
}

impl CanEndFalse {
    fn render_end_text(&self) -> N {
        if self.largest == 0 {
            return N::text("No corporations have been founded yet");
        }
        N::Group(vec![
            N::text("Largest corporation is "),
            N::Bold(vec![N::text(format!("{}", self.largest))]),
            N::text(" of "),
            N::Bold(vec![N::text(format!("{}", GAME_END_SIZE))]),
            N::text(", "),
            N::Bold(vec![N::text(format!("{}", self.unsafe_count))]),
            N::text(" unsafe remaining"),
        ])
    }
}
