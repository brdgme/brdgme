use std::cmp;

use crate::{next_player, PlayerState, PubState, MAX_PLAYERS, ROUNDS, START_ROUND};
use crate::card::{by_expedition, expeditions, Card};

use brdgme_color::GREY;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row};
use brdgme_markup::ast::Cell;

use std::iter::repeat;

const EXP_SPACER: &'static str = "  ";
const TABLEAU_HEADER_SPACER: &'static str = "   ";
const OPPONENT_SPACER: &'static str = "       ";
const SCORE_SPACER: &'static str = "  ";
const EMPTY_CARD_PILE: &'static str = "--";

fn render(pub_state: &PubState, player: Option<usize>, hand: Option<&[Card]>) -> Vec<N> {
    let mut layout: Vec<Row> = vec![];
    if !pub_state.is_finished {
        layout.extend(vec![
            vec![
                (
                    A::Center,
                    vec![
                        N::text("Round "),
                        N::Bold(vec![N::text(format!("{}", pub_state.round))]),
                        N::text(" of "),
                        N::Bold(vec![N::text(format!("{}", super::ROUNDS))]),
                    ],
                ),
            ],
            vec![],
        ]);
    }
    layout.extend(
        pub_state
            .render_tableau(player)
            .into_iter()
            .map(|n| vec![(A::Center, vec![n])])
            .collect::<Vec<Row>>(),
    );
    if let Some(h) = hand {
        layout.append(&mut vec![
            vec![],
            vec![
                (
                    A::Center,
                    vec![N::Fg(GREY.into(), vec![N::text("Your hand")])],
                ),
            ],
            vec![(A::Center, render_hand(h))],
        ]);
    }
    // Scores
    let persp = match player {
        Some(p) if p < pub_state.players => p,
        _ => 0,
    };
    let mut scores: Vec<Row> = vec![];
    let mut header: Row = vec![(A::Left, vec![])];
    for r in START_ROUND..(START_ROUND + ROUNDS) {
        header.extend(vec![
            (A::Left, vec![N::text(SCORE_SPACER)]),
            (
                A::Center,
                vec![N::Fg(GREY.into(), vec![N::text(format!("R{}", r))])],
            ),
        ]);
    }
    header.extend(vec![
        (A::Left, vec![N::text("  ")]),
        (A::Center, vec![N::Fg(GREY.into(), vec![N::text("Tot")])]),
    ]);
    scores.push(header);
    for p_offset in 0..pub_state.players {
        let p = (persp + p_offset) % pub_state.players;
        let mut score_row: Row = vec![(A::Right, vec![N::Player(p)])];
        for r in 0..ROUNDS {
            score_row.extend(vec![
                (A::Left, vec![]),
                (
                    A::Center,
                    vec![
                        N::text(
                            pub_state
                                .scores
                                .get(p)
                                .and_then(|s| s.get(r))
                                .map(|rs| format!("{}", rs))
                                .unwrap_or_else(|| "".to_string()),
                        ),
                    ],
                ),
            ]);
        }
        score_row.extend(vec![
            (A::Left, vec![]),
            (
                A::Center,
                vec![N::text(format!("{}", pub_state.player_score(p)))],
            ),
        ]);
        scores.push(score_row);
    }
    layout.append(&mut vec![
        vec![],
        vec![
            (A::Center, vec![N::Fg(GREY.into(), vec![N::text("Scores")])]),
        ],
        vec![(A::Center, vec![N::Table(scores)])],
    ]);
    vec![N::Table(layout)]
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(&self.hand))
    }
}

impl PubState {
    fn render_tableau(&self, player: Option<usize>) -> Vec<N> {
        let p = player.unwrap_or(0) % MAX_PLAYERS;
        let mut layout: Vec<N> = vec![];
        let mut rows: Vec<Row> = vec![];

        // Top half
        match self.players {
            2 => {
                // Two players, we just put the top in the main table.
                let mut top = match self.expeditions.get(next_player(p, self.players)) {
                    Some(e) => render_tableau_cards(e, &N::Player(next_player(p, self.players))),
                    None => vec![],
                };
                top.reverse();
                rows.append(&mut top);
            }
            3 => {
                // Three players, we don't align the opponents with the discards, they are their own
                // section in the layout side by side.
                let mut opponent_tableaus: Vec<Vec<Row>> = vec![];
                let mut tallest: usize = 0;
                // Get the tableau rows for each opponent.
                for opp_offset in 1..self.players {
                    let opp = (p + opp_offset) % self.players;
                    let mut opp_tableau = match self.expeditions.get(opp) {
                        Some(e) => render_tableau_cards(e, &N::Player(opp)),
                        None => vec![],
                    };
                    let height = opp_tableau.len();
                    if height > tallest {
                        tallest = height;
                    }
                    opp_tableau.reverse();
                    opponent_tableaus.push(opp_tableau);
                }
                // Adjust all to the same height bottom aligned and render each as a table cell.
                layout.push(N::Table(vec![
                    opponent_tableaus
                        .into_iter()
                        .enumerate()
                        .flat_map(|(i, mut opp_tableau)| {
                            let mut cells: Vec<Cell> = vec![];
                            if i > 0 {
                                cells.push((A::Left, vec![N::text(OPPONENT_SPACER)]))
                            }
                            let height = opp_tableau.len();
                            if height < tallest {
                                let mut new_tableau: Vec<Row> =
                                    repeat(vec![]).take(tallest - height).collect();
                                new_tableau.extend(opp_tableau);
                                opp_tableau = new_tableau;
                            }
                            cells.push((A::Left, vec![N::Table(opp_tableau)]));
                            cells
                        })
                        .collect(),
                ]))
            }
            _ => unreachable!(),
        }

        // Blank row
        rows.push(vec![]);
        if self.players > 2 {
            // Some extra space for 3 players as it gets a bit busy.
            rows.push(vec![]);
        }

        // Discards
        let mut discards: Row = vec![
            (A::Right, vec![N::Fg(GREY.into(), vec![N::text("Discard")])]),
        ];
        for (i, &e) in expeditions().iter().enumerate() {
            // Column spacing
            discards.push((
                A::Left,
                vec![
                    N::text(if i == 0 {
                        TABLEAU_HEADER_SPACER
                    } else {
                        EXP_SPACER
                    }),
                ],
            ));

            discards.push((
                A::Center,
                vec![
                    if let Some(v) = self.discards.get(&e) {
                        card(&(e, *v).into())
                    } else {
                        N::Fg(e.color().into(), vec![N::text(EMPTY_CARD_PILE)])
                    },
                ],
            ));
        }
        discards.push((
            A::Left,
            vec![
                N::Fg(
                    GREY.into(),
                    vec![
                        N::text("   "),
                        N::Bold(vec![N::text(format!("{}", self.deck_remaining))]),
                        N::text(" left"),
                    ],
                ),
            ],
        ));

        rows.push(discards);

        // Blank row
        rows.push(vec![]);
        if self.players > 2 {
            // Some extra space for 3 players as it gets a bit busy.
            rows.push(vec![]);
        }

        // Bottom half
        if let Some(e) = self.expeditions.get(p) {
            rows.append(&mut render_tableau_cards(e, &N::Player(p)));
        }
        layout.push(N::Table(rows));
        layout
    }

    pub fn player_score(&self, player: usize) -> isize {
        match self.scores.get(player) {
            Some(s) => s.iter().sum(),
            None => 0,
        }
    }
}

fn render_tableau_cards(cards: &[Card], header: &N) -> Vec<Row> {
    let mut rows: Vec<Row> = vec![];
    let by_exp = by_expedition(cards);
    let mut largest: usize = 1;
    for e in expeditions() {
        largest = cmp::max(largest, by_exp.get(&e).unwrap_or(&vec![]).len());
    }
    for row_i in 0..largest {
        let mut row: Row = vec![
            if row_i == 0 {
                (A::Right, vec![header.to_owned()])
            } else {
                (A::Left, vec![])
            },
        ];
        for (i, &e) in expeditions().iter().enumerate() {
            // Column spacing
            row.push((
                A::Left,
                vec![
                    N::text(if i == 0 {
                        TABLEAU_HEADER_SPACER
                    } else {
                        EXP_SPACER
                    }),
                ],
            ));
            match by_exp.get(&e).unwrap_or(&vec![]).get(row_i) {
                Some(c) => row.push((A::Center, vec![card(c)])),
                None => row.push((
                    A::Left,
                    vec![
                        N::Fg(
                            e.color().into(),
                            vec![
                                N::text(if row_i == 0 {
                                    EMPTY_CARD_PILE
                                } else {
                                    EXP_SPACER
                                }),
                            ],
                        ),
                    ],
                )),
            }
        }
        rows.push(row);
    }
    rows
}

fn render_hand(cards: &[Card]) -> Vec<N> {
    let mut output: Vec<N> = vec![];
    let mut sorted = cards.to_owned();
    sorted.sort();
    for c in sorted {
        if !output.is_empty() {
            output.push(N::text(" "));
        }
        output.push(card(&c));
    }
    output
}

pub fn card(c: &Card) -> N {
    N::Bold(vec![
        N::Fg(c.expedition.color().into(), vec![N::text(c.to_string())]),
    ])
}

pub fn comma_cards(cards: &[Card]) -> Vec<N> {
    let mut output: Vec<N> = vec![];
    for c in cards {
        if !output.is_empty() {
            output.push(N::text(", "));
        }
        output.push(card(c));
    }
    output
}
