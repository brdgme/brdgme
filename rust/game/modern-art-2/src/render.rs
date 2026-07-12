use brdgme_color::GREEN as MONEY_GREEN;
use brdgme_color::GREY;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::card::{Card, Rank, Suit, suits};
use crate::{PlayerState, PubState};

pub fn money(amount: i32) -> N {
    N::Bold(vec![N::Fg(
        MONEY_GREEN.into(),
        vec![N::text(format!("${}", amount))],
    )])
}

pub fn suit(s: Suit) -> N {
    N::Bold(vec![N::Fg(s.color().into(), vec![N::text(s.name())])])
}

pub fn card_name(c: Card) -> N {
    N::Bold(vec![N::Fg(c.suit.color().into(), vec![N::text(c.name())])])
}

pub fn card_code(c: Card) -> N {
    N::Bold(vec![N::Fg(c.suit.color().into(), vec![N::text(c.code())])])
}

pub fn card_name_code(c: Card) -> N {
    N::Bold(vec![N::Fg(
        c.suit.color().into(),
        vec![N::text(format!("({}) {}", c.code(), c.name()))],
    )])
}

pub fn card_names(cards: &[Card]) -> N {
    let mut out: Vec<N> = vec![];
    for (i, &c) in cards.iter().enumerate() {
        if i > 0 {
            out.push(N::text(" and "));
        }
        out.push(card_name(c));
    }
    N::Group(out)
}

fn suit_value(value_board: &[std::collections::HashMap<Suit, i32>], s: Suit) -> i32 {
    value_board.iter().map(|v| *v.get(&s).unwrap_or(&0)).sum()
}

fn render(
    pub_state: &PubState,
    player: Option<usize>,
    money_amt: Option<i32>,
    hand: Option<&[Card]>,
) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    if pub_state.is_auction {
        out.push(N::Player(pub_state.current_player));
        out.push(N::text(" is auctioning "));
        out.push(card_names(&pub_state.auctioning));
        out.push(N::text("\n\n"));
        if pub_state.auction_type != Some(Rank::Sealed)
            && let Some((bidder, bid)) = pub_state.current_bid
        {
            out.push(N::Bold(vec![N::text("Current bid:")]));
            out.push(N::text(" "));
            out.push(money(bid));
            out.push(N::text(" by "));
            out.push(N::Player(bidder));
            out.push(N::text("\n"));
        }
        out.push(N::text("\n"));
    }

    if let (Some(_p), Some(m), Some(h)) = (player, money_amt, hand) {
        out.push(N::Bold(vec![N::text("Your money:")]));
        out.push(N::text(" "));
        out.push(money(m));
        out.push(N::text("\n\n"));
        out.push(N::Bold(vec![N::text("Your cards:\n")]));
        for &c in h {
            out.push(card_name_code(c));
            out.push(N::text("\n"));
        }
        out.push(N::text("\n"));
    }

    // Players / purchases table.
    let mut rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Players")])]),
        (A::Left, vec![N::Bold(vec![N::text("Purchases")])]),
    ]];
    for op in 0..pub_state.players {
        let cards = pub_state.purchases.get(op).cloned().unwrap_or_default();
        let cards_node = if cards.is_empty() {
            N::Fg(GREY.into(), vec![N::text("None")])
        } else {
            let mut nodes: Vec<N> = vec![];
            for (i, &c) in cards.iter().enumerate() {
                if i > 0 {
                    nodes.push(N::text(" "));
                }
                nodes.push(card_code(c));
            }
            N::Group(nodes)
        };
        rows.push(vec![
            (A::Left, vec![N::Player(op)]),
            (A::Left, vec![cards_node]),
        ]);
    }
    out.push(table_with_gap(&rows, 2));
    out.push(N::text("\n\n"));

    // Artist value board.
    let mut art_rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Artist")])]),
        (A::Left, vec![N::Bold(vec![N::text("R1")])]),
        (A::Left, vec![N::Bold(vec![N::text("R2")])]),
        (A::Left, vec![N::Bold(vec![N::text("R3")])]),
        (A::Left, vec![N::Bold(vec![N::text("R4")])]),
        (A::Left, vec![N::Bold(vec![N::text("Total")])]),
    ]];
    for s in suits() {
        let mut row: Row = vec![(A::Left, vec![suit(s)])];
        for i in 0..4 {
            if pub_state.value_board.len() > i {
                row.push((
                    A::Left,
                    vec![money(*pub_state.value_board[i].get(&s).unwrap_or(&0))],
                ));
            } else {
                row.push((A::Left, vec![N::text(".")]));
            }
        }
        row.push((A::Left, vec![money(suit_value(&pub_state.value_board, s))]));
        art_rows.push(row);
    }
    out.push(table_with_gap(&art_rows, 2));

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(
            &self.public,
            Some(self.player),
            Some(self.money),
            Some(&self.hand),
        )
    }
}
