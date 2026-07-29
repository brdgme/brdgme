use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::card::Geisha;
use crate::{Pending, Phase, PlayerState, PubState};

const COL_SPACING: usize = 2;

pub fn geisha_node(g: Geisha) -> N {
    N::Bold(vec![N::Fg(
        g.color().into(),
        vec![N::text(g.name().to_string())],
    )])
}

pub fn comma_geisha(cards: &[Geisha]) -> Vec<N> {
    let mut out: Vec<N> = vec![];
    for (i, g) in cards.iter().enumerate() {
        if i > 0 {
            out.push(N::text(", "));
        }
        out.push(geisha_node(*g));
    }
    out
}

fn grey(text: &str) -> N {
    N::Fg(NamedColor::Grey.into(), vec![N::text(text.to_string())])
}

fn marker_nodes(marker: Option<usize>) -> Vec<N> {
    match marker {
        Some(0) => vec![N::Bold(vec![N::text("< ")]), N::Player(0)],
        Some(1) => vec![N::Player(1), N::Bold(vec![N::text(" >")])],
        _ => vec![grey("-")],
    }
}

fn actions_node(used: [bool; 4]) -> N {
    let labels = ["S", "T", "G", "C"];
    let mut nodes: Vec<N> = vec![];
    for (k, label) in labels.iter().enumerate() {
        if k > 0 {
            nodes.push(N::text(" "));
        }
        let letter = N::text(label.to_string());
        nodes.push(if used[k] {
            grey(label)
        } else {
            N::Bold(vec![N::Fg(NamedColor::Green.into(), vec![letter])])
        });
    }
    N::Group(nodes)
}

fn board_table(pub_state: &PubState) -> Vec<Row> {
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Geisha")])]),
        (A::Center, vec![N::Bold(vec![N::text("Charm")])]),
        (A::Center, vec![N::Player(0)]),
        (A::Center, vec![N::Bold(vec![N::text("Marker")])]),
        (A::Center, vec![N::Player(1)]),
    ]];
    for g in Geisha::ALL {
        let i = g.index();
        let faceup = pub_state.faceup.get(i).copied().unwrap_or([0, 0]);
        let marker = pub_state.marker.get(i).copied().unwrap_or(None);
        rows.push(vec![
            (A::Left, vec![geisha_node(g)]),
            (A::Center, vec![N::text(g.charm().to_string())]),
            (
                A::Center,
                vec![N::Bold(vec![N::text(faceup[0].to_string())])],
            ),
            (A::Center, marker_nodes(marker)),
            (
                A::Center,
                vec![N::Bold(vec![N::text(faceup[1].to_string())])],
            ),
        ]);
    }
    rows
}

fn summary_table(pub_state: &PubState) -> Vec<Row> {
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Player")])]),
        (A::Center, vec![N::Bold(vec![N::text("Geisha")])]),
        (A::Center, vec![N::Bold(vec![N::text("Charm")])]),
        (A::Center, vec![N::Bold(vec![N::text("Hand")])]),
        (A::Center, vec![N::Bold(vec![N::text("Secret")])]),
        (A::Center, vec![N::Bold(vec![N::text("Traded")])]),
        (A::Left, vec![N::Bold(vec![N::text("Actions")])]),
    ]];
    for p in 0..pub_state.players {
        let used = pub_state.used.get(p).copied().unwrap_or([true; 4]);
        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (
                A::Center,
                vec![N::Bold(vec![N::text(
                    pub_state
                        .geisha_counts
                        .get(p)
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                )])],
            ),
            (
                A::Center,
                vec![N::Bold(vec![N::text(
                    pub_state.charms.get(p).copied().unwrap_or(0).to_string(),
                )])],
            ),
            (
                A::Center,
                vec![N::text(
                    pub_state
                        .hand_counts
                        .get(p)
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                )],
            ),
            (
                A::Center,
                vec![if pub_state.has_secret.get(p).copied().unwrap_or(false) {
                    N::text("yes")
                } else {
                    grey("-")
                }],
            ),
            (
                A::Center,
                vec![N::text(
                    pub_state
                        .traded_counts
                        .get(p)
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                )],
            ),
            (A::Left, vec![actions_node(used)]),
        ]);
    }
    rows
}

fn pending_nodes(pub_state: &PubState) -> Option<N> {
    let pending = pub_state.pending.as_ref()?;
    let line = match pending {
        Pending::Gift { actor, cards } => {
            let mut line = vec![
                N::Bold(vec![N::text("Gift: ")]),
                N::Player(*actor),
                N::text(" offered "),
            ];
            line.extend(comma_geisha(cards));
            line.push(N::text(" - "));
            line.push(N::Player(1 - *actor));
            line.push(N::Bold(vec![N::text(" chooses one")]));
            line
        }
        Pending::Competition { actor, sets } => {
            let mut line = vec![
                N::Bold(vec![N::text("Competition: ")]),
                N::Player(*actor),
                N::text(" offered set 1 { "),
            ];
            line.extend(comma_geisha(&sets[0]));
            line.push(N::text(" } and set 2 { "));
            line.extend(comma_geisha(&sets[1]));
            line.push(N::text(" } - "));
            line.push(N::Player(1 - *actor));
            line.push(N::Bold(vec![N::text(" chooses a set")]));
            line
        }
    };
    Some(N::Group(line))
}

fn render(
    pub_state: &PubState,
    player: Option<usize>,
    hand: Option<&[Geisha]>,
    secret: Option<Option<Geisha>>,
    traded: Option<&[Geisha]>,
) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    if pub_state.finished {
        if let Some(w) = pub_state.winner {
            out.push(N::Bold(vec![
                N::text("Game over - "),
                N::Player(w),
                N::text(" wins!"),
            ]));
        }
    } else {
        match pub_state.phase {
            Phase::ChooseAction => out.push(N::Bold(vec![
                N::Player(pub_state.current),
                N::text("'s turn - choose an action (secret, trade, gift or compete)"),
            ])),
            Phase::OpponentChoose => out.push(N::Bold(vec![
                N::Player(1 - pub_state.current),
                N::text(" chooses from the pending cards"),
            ])),
            Phase::Finished => {}
        }
    }
    out.push(N::text(format!(
        "Round {}  -  {} cards left in the deck\n",
        pub_state.round, pub_state.deck_remaining
    )));

    if let Some(pending) = pending_nodes(pub_state) {
        out.push(pending);
        out.push(N::text("\n"));
    }

    out.push(table_with_gap(&board_table(pub_state), COL_SPACING));
    out.push(N::text("\n"));
    out.push(table_with_gap(&summary_table(pub_state), COL_SPACING));

    if player.is_some() {
        out.push(N::text("\n"));
        if let Some(h) = hand {
            out.push(N::Bold(vec![N::text("Your hand: ")]));
            if h.is_empty() {
                out.push(grey("empty"));
            } else {
                out.push(N::Group(comma_geisha(h)));
            }
            out.push(N::text("\n"));
        }
        if let Some(sec) = secret {
            out.push(N::Bold(vec![N::text("Your secret: ")]));
            match sec {
                Some(g) => out.push(geisha_node(g)),
                None => out.push(grey("none")),
            }
            out.push(N::text("\n"));
        }
        if let Some(tr) = traded {
            out.push(N::Bold(vec![N::text("Your trade-off discard: ")]));
            if tr.is_empty() {
                out.push(grey("none"));
            } else {
                out.push(N::Group(comma_geisha(tr)));
            }
            out.push(N::text("\n"));
        }
    }

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(
            &self.public,
            Some(self.player),
            Some(&self.hand),
            Some(self.secret),
            Some(&self.traded),
        )
    }
}
