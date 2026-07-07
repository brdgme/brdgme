use std::collections::HashSet;

use brdgme_color::GREY;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row};

use crate::{Card, PlayerState, PubState};

pub fn card(c: Card) -> N {
    if c == Card::Played {
        N::Fg(GREY.into(), vec![N::text("played")])
    } else {
        N::Fg(c.color().into(), vec![N::Bold(vec![N::text(c.name())])])
    }
}

pub fn cards_list(cards: &[Card]) -> N {
    let mut nodes: Vec<N> = vec![];
    for (i, c) in cards.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(", "));
        }
        nodes.push(card(*c));
    }
    N::Group(nodes)
}

pub fn comma_list_nodes(nodes: Vec<N>) -> N {
    let mut out: Vec<N> = vec![];
    for (i, n) in nodes.into_iter().enumerate() {
        if i > 0 {
            out.push(N::text(", "));
        }
        out.push(n);
    }
    N::Group(out)
}

fn render_name(player: usize, players: usize) -> N {
    if player > players - 1 {
        N::Fg(GREY.into(), vec![N::Bold(vec![N::text("<dummy>")])])
    } else {
        N::Player(player)
    }
}

fn card_column(c: Card) -> usize {
    match c {
        Card::Tempura => 0,
        Card::Sashimi => 1,
        Card::Dumpling => 2,
        Card::MakiRoll3 | Card::MakiRoll2 | Card::MakiRoll1 => 3,
        Card::SalmonNigiri | Card::SquidNigiri | Card::EggNigiri | Card::Wasabi => 4,
        Card::Pudding => 5,
        Card::Chopsticks => 6,
        Card::Played => 0,
    }
}

const NUM_COLUMNS: usize = 7;

fn cards_cells(cards: &[Card]) -> Vec<Row> {
    let mut columns: Vec<Vec<Vec<N>>> = vec![vec![]; NUM_COLUMNS];
    let mut unused_wasabi = 0;
    for &c in cards {
        let col = card_column(c);
        match c {
            Card::Wasabi => {
                columns[col].push(vec![card(c)]);
                unused_wasabi += 1;
            }
            Card::SalmonNigiri | Card::SquidNigiri | Card::EggNigiri if unused_wasabi > 0 => {
                let idx = columns[col].len() - unused_wasabi;
                columns[col][idx] = vec![card(c), N::text(" + "), card(Card::Wasabi)];
                unused_wasabi -= 1;
            }
            Card::SalmonNigiri | Card::SquidNigiri | Card::EggNigiri => {
                columns[col].push(vec![card(c)]);
            }
            _ => {
                columns[col].push(vec![card(c)]);
            }
        }
    }
    let non_empty: Vec<usize> = (0..NUM_COLUMNS)
        .filter(|&i| !columns[i].is_empty())
        .collect();
    let max_len = non_empty
        .iter()
        .map(|&i| columns[i].len())
        .max()
        .unwrap_or(0);
    let mut rows: Vec<Row> = vec![];
    for y in 0..max_len {
        let mut row: Row = vec![];
        for &col in &non_empty {
            if y < columns[col].len() {
                row.push((A::Left, columns[col][y].clone()));
            } else {
                row.push((A::Left, vec![]));
            }
        }
        rows.push(row);
    }
    rows
}

fn hand_table(hand: &[Card]) -> Vec<Row> {
    let mut explained: HashSet<Card> = HashSet::new();
    let mut rows: Vec<Row> = vec![];
    for (i, &c) in hand.iter().enumerate() {
        let mut row: Row = vec![
            (
                A::Left,
                vec![N::Fg(GREY.into(), vec![N::text(format!("({})", i + 1))])],
            ),
            (A::Left, vec![card(c)]),
        ];
        if !explained.contains(&c) && !c.explanation().is_empty() {
            row.push((
                A::Left,
                vec![N::Fg(
                    GREY.into(),
                    vec![N::text(format!("  {}", c.explanation()))],
                )],
            ));
            explained.insert(c);
            if c == Card::MakiRoll1 || c == Card::MakiRoll2 || c == Card::MakiRoll3 {
                explained.insert(Card::MakiRoll1);
                explained.insert(Card::MakiRoll2);
                explained.insert(Card::MakiRoll3);
            }
        }
        rows.push(row);
    }
    rows
}

fn render(pub_state: &PubState, own: Option<&PlayerState>) -> Vec<N> {
    let mut out: Vec<N> = vec![
        N::text("It is round "),
        N::Bold(vec![N::text(pub_state.round.to_string())]),
        N::text(" of "),
        N::Bold(vec![N::text("3")]),
        N::text("\n\n"),
    ];

    if let Some(own) = own {
        let p = own.player;
        out.push(N::Bold(vec![N::text("Hand:\n\n")]));
        out.push(N::Table(hand_table(&own.hand)));
        out.push(N::text("\n\n"));

        let mut playing_output = false;
        if let Some(ref playing) = own.playing {
            out.push(N::Group(vec![
                N::text("Playing: "),
                cards_list(playing),
                N::text("\n"),
            ]));
            playing_output = true;
        }
        if p == pub_state.controller
            && pub_state.players == 2
            && let Some(ref dummy_playing) = own.dummy_playing
        {
            out.push(N::Group(vec![
                N::text("Dummy:   "),
                cards_list(dummy_playing),
                N::text("\n"),
            ]));
            playing_output = true;
        }
        if playing_output {
            out.push(N::text("\n"));
        }
    }

    let p_count = pub_state.all_players;
    let dir: i32 = if pub_state.round % 2 == 1 { -1 } else { 1 };
    let p_num = own.map(|o| o.player as i32).unwrap_or(-1);
    for i in 0..p_count {
        let mut p = p_num + i as i32 * dir;
        if p < 0 {
            p += p_count as i32;
        }
        p %= p_count as i32;
        let p_usize = p as usize;
        let heading = if p_num >= 0 && i == 1 {
            "You are passing cards to "
        } else {
            ""
        };
        out.push(N::text(heading));
        out.push(render_name(p_usize, pub_state.players));
        out.push(N::text(" ("));
        out.push(N::Bold(vec![N::text(
            pub_state.player_points[p_usize].to_string(),
        )]));
        out.push(N::text("):\n"));
        out.push(N::Table(cards_cells(&pub_state.played[p_usize])));
        out.push(N::text("\n\n"));
    }

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self))
    }
}
