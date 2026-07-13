use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::{Node as N, comma_list_and};

use crate::{Phase, PlayerState, PubState};

pub fn building(n: i32) -> N {
    N::Fg(
        NamedColor::Green.into(),
        vec![N::Bold(vec![N::text(n.to_string())])],
    )
}

pub fn cheque(n: i32) -> N {
    N::Fg(
        NamedColor::Blue.into(),
        vec![N::Bold(vec![N::text(n.to_string())])],
    )
}

pub fn bold_num(n: i32) -> N {
    N::Bold(vec![N::text(n.to_string())])
}

pub fn cards(deck: &[i32], is_building: bool) -> N {
    let mut nodes: Vec<N> = vec![];
    for (i, c) in deck.iter().enumerate() {
        if i > 0 {
            nodes.push(N::text(" "));
        }
        nodes.push(if is_building {
            building(*c)
        } else {
            cheque(*c)
        });
    }
    N::Group(nodes)
}

fn highest_bid(pub_state: &PubState) -> Option<(usize, i32)> {
    let mut best_p = 0;
    let mut best: i32 = -1;
    for p in 0..pub_state.players {
        if !pub_state.finished_bidding[p] && pub_state.bids[p] > best {
            best_p = p;
            best = pub_state.bids[p];
        }
    }
    if best > 0 { Some((best_p, best)) } else { None }
}

fn rounds_remaining_line(pub_state: &PubState) -> N {
    let (rounds, kind) = match pub_state.phase {
        Phase::Buying => (pub_state.buy_rounds_remaining + 1, "buying"),
        Phase::Selling => (pub_state.sell_rounds_remaining + 1, "selling"),
        Phase::Finished => return N::text(""),
    };
    N::Bold(vec![N::text(format!(
        "\n\n{} {} {} remaining",
        rounds,
        kind,
        if rounds == 1 { "round" } else { "rounds" }
    ))])
}

fn render(pub_state: &PubState, player: Option<usize>, own: Option<&PlayerState>) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    match pub_state.phase {
        Phase::Buying => {
            out.push(N::text("Buildings available: "));
            out.push(cards(&pub_state.open_cards, true));
            out.push(N::text("\n"));
            out.push(N::text("Current bid: "));
            match highest_bid(pub_state) {
                Some((p, amt)) => {
                    out.push(N::Group(vec![bold_num(amt), N::text(" by "), N::Player(p)]))
                }
                None => out.push(N::Fg(NamedColor::Grey.into(), vec![N::text("none")])),
            }
            out.push(N::text("\n"));
            if let Some(p) = player {
                out.push(N::Group(vec![
                    N::text("Your bid: "),
                    bold_num(pub_state.bids[p]),
                    N::text("\n"),
                ]));
            }
            let remaining: Vec<Vec<N>> = (0..pub_state.players)
                .filter(|p| !pub_state.finished_bidding[*p])
                .map(|p| vec![N::Player(p)])
                .collect();
            out.push(N::text("Remaining players: "));
            out.extend(comma_list_and(&remaining));
            out.push(N::text("\n\n"));
        }
        Phase::Selling => {
            out.push(N::text("Cheques available: "));
            out.push(cards(&pub_state.open_cards, false));
            out.push(N::text("\n"));
            if let Some(p) = player
                && pub_state.bids[p] != 0
            {
                out.push(N::Group(vec![
                    N::text("You are playing: "),
                    building(pub_state.bids[p]),
                    N::text("\n"),
                ]));
            }
            out.push(N::text("\n"));
        }
        Phase::Finished => {}
    }

    if let Some(own) = own {
        out.push(N::Group(vec![
            N::text("Your chips: "),
            bold_num(own.chips),
            N::text("\n"),
        ]));
        out.push(N::text("Your buildings: "));
        out.push(cards(&own.hand, true));
        out.push(N::text("\n"));
        out.push(N::text("Your cheques: "));
        out.push(cards(&own.cheques, false));
    }

    if !pub_state.finished {
        out.push(rounds_remaining_line(pub_state));
    }

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(self))
    }
}
