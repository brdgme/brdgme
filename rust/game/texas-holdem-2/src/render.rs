//! Ported from the rendering half of `brdgme-go/texas_holdem_1/texas_holdem.go`
//! (`PubRender`/`PlayerRender`/`RenderCash`/`RenderCards`).
//!
//! Go also defines `RenderCashFixedWidth`, but nothing in `texas_holdem_1`
//! ever calls it (`PlayerRender` uses plain `RenderCash` in the players
//! table) - it is dead code in the source. It is not ported here; ported
//! `Node::Table`/`table_with_gap` output does not need manual fixed-width
//! padding since the table renderer aligns columns itself.

use brdgme_color::GREEN as MONEY_GREEN;
use brdgme_color::GREY;
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::card::Card;
use crate::{PlayerState, PubState};

/// Port of `RenderCash`.
pub fn cash(amount: i32) -> N {
    N::Bold(vec![N::Fg(
        MONEY_GREEN.into(),
        vec![N::text(format!("${}", amount))],
    )])
}

/// Port of `RenderCards`.
pub fn cards(deck: &[Card]) -> Vec<N> {
    let mut out: Vec<N> = vec![];
    for (i, c) in deck.iter().enumerate() {
        if i > 0 {
            out.push(N::text(" "));
        }
        out.push(N::Bold(vec![c.render_standard_52_fixed_width()]));
    }
    out
}

fn render(pub_state: &PubState, player: Option<usize>, hand: Option<&[Card]>) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    out.push(N::Bold(vec![N::text("Community cards:  ")]));
    out.extend(cards(&pub_state.community_cards));
    out.push(N::text("\n"));
    out.push(N::Bold(vec![N::text("Current pot:      ")]));
    out.push(cash(pub_state.pot));
    out.push(N::text("\n\n"));

    if let (Some(p), Some(h)) = (player, hand) {
        out.push(N::Bold(vec![N::text("Your cards:  ")]));
        out.extend(cards(h));
        out.push(N::text("\n"));
        out.push(N::Bold(vec![N::text("Your cash:   ")]));
        out.push(cash(pub_state.player_money[p]));
        out.push(N::text("\n\n"));
    }

    let header: Row = vec![
        (A::Left, vec![N::Bold(vec![N::text("Players")])]),
        (A::Left, vec![N::Bold(vec![N::text("Cash")])]),
        (A::Left, vec![N::Bold(vec![N::text("Bet")])]),
    ];
    let mut table_rows: Vec<Row> = vec![header];
    for table_player_num in 0..pub_state.players {
        let mut name: Vec<N> = vec![N::Player(table_player_num)];
        if table_player_num == pub_state.current_dealer {
            name.push(N::text(" (D)"));
        }
        let mut row: Row = vec![(A::Left, name)];
        if pub_state.player_money[table_player_num] == 0 && pub_state.bets[table_player_num] == 0 {
            row.push((A::Left, vec![N::Fg(GREY.into(), vec![N::text("Out")])]));
        } else {
            row.push((
                A::Left,
                vec![cash(pub_state.player_money[table_player_num])],
            ));
            row.push((A::Left, vec![cash(pub_state.bets[table_player_num])]));
            let extra: Vec<N> = if pub_state.folded_players[table_player_num] {
                vec![N::Fg(GREY.into(), vec![N::text("Folded")])]
            } else {
                vec![]
            };
            row.push((A::Left, extra));
        }
        table_rows.push(row);
    }
    out.push(table_with_gap(&table_rows, 2));
    out
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
