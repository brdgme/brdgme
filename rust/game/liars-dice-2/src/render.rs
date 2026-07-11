use brdgme_color::{BLACK, CYAN, GREY, RED};
use brdgme_game::Renderer;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use crate::{PlayerState, PubState};

fn render(pub_state: &PubState, _player: Option<usize>, hand: Option<&[u8]>) -> Vec<N> {
    let mut out: Vec<N> = vec![];

    let bid = if pub_state.bid_quantity == 0 {
        N::Fg(GREY.into(), vec![N::text("first bid")])
    } else {
        render_bid(pub_state.bid_quantity, pub_state.bid_value)
    };
    out.push(N::Group(vec![N::text("Current bid: "), bid, N::text("\n")]));

    if let Some(h) = hand
        && !h.is_empty()
    {
        let mut dice_nodes: Vec<N> = vec![];
        for (i, d) in h.iter().enumerate() {
            if i > 0 {
                dice_nodes.push(N::text(" "));
            }
            dice_nodes.push(render_die(*d as i32));
        }
        out.push(N::Group(vec![
            N::text("Your dice: "),
            N::Bold(dice_nodes),
            N::text("\n\n"),
        ]));
    }

    let mut rows: Vec<Row> = vec![];
    rows.push(vec![
        (A::Left, vec![N::Bold(vec![N::text("Player")])]),
        (A::Left, vec![N::Bold(vec![N::text("Remaining dice")])]),
    ]);
    for p in 0..pub_state.players {
        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (
                A::Left,
                vec![N::text(format!(
                    "{}",
                    pub_state.remaining_dice.get(p).copied().unwrap_or(0)
                ))],
            ),
        ]);
    }
    out.push(table_with_gap(&rows, 1));

    out
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(&self.dice))
    }
}

pub fn render_bid(quantity: i32, value: i32) -> N {
    let suffix = if quantity > 1 { "s" } else { "" };
    N::Group(vec![
        N::text(format!("{} ", number_str(quantity))),
        render_die(value),
        N::text(suffix.to_string()),
    ])
}

// Port of NumberStr from brdgme-go/brdgme/strings.go, covering the range of
// bid quantities possible in this game (0..999). Falls back to digits for
// anything outside that range, and for negative numbers.
fn number_str(n: i32) -> String {
    if !(0..1000).contains(&n) {
        return n.to_string();
    }
    if n == 0 {
        return "zero".to_string();
    }
    let mut parts: Vec<&str> = vec![];
    let mut n = n;
    if n >= 100 {
        let h = n / 100;
        n -= h * 100;
        parts.push(ones_str(h));
        parts.push("hundred");
        if n > 0 {
            parts.push("and");
        }
    }
    if n >= 20 {
        let t = n / 10;
        n -= t * 10;
        parts.push(match t {
            2 => "twenty",
            3 => "thirty",
            4 => "fourty",
            5 => "fifty",
            6 => "sixty",
            7 => "seventy",
            8 => "eighty",
            9 => "ninety",
            _ => unreachable!(),
        });
    }
    if n > 0 {
        parts.push(match n {
            1 => "one",
            2 => "two",
            3 => "three",
            4 => "four",
            5 => "five",
            6 => "six",
            7 => "seven",
            8 => "eight",
            9 => "nine",
            10 => "ten",
            11 => "eleven",
            12 => "twelve",
            13 => "thirteen",
            14 => "fourteen",
            15 => "fifteen",
            16 => "sixteen",
            17 => "seventeen",
            18 => "eighteen",
            19 => "nineteen",
            _ => unreachable!(),
        });
    }
    parts.join(" ")
}

fn ones_str(n: i32) -> &'static str {
    match n {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        _ => unreachable!(),
    }
}

pub fn render_die(value: i32) -> N {
    let color = if value == 1 { CYAN } else { BLACK };
    N::Bold(vec![N::Fg(color.into(), vec![N::text(value.to_string())])])
}

pub fn reveal_table(player_dice: &[Vec<u8>], active: &[usize], bid_value: i32) -> N {
    let mut rows: Vec<Row> = vec![];
    for &p in active {
        let mut dice_nodes: Vec<N> = vec![];
        for (i, d) in player_dice[p].iter().enumerate() {
            if i > 0 {
                dice_nodes.push(N::text(" "));
            }
            if *d as i32 == bid_value || *d == 1 {
                dice_nodes.push(N::Fg(RED.into(), vec![N::text(d.to_string())]));
            } else {
                dice_nodes.push(N::text(d.to_string()));
            }
        }
        rows.push(vec![
            (A::Left, vec![N::Player(p)]),
            (A::Left, vec![N::Bold(dice_nodes)]),
        ]);
    }
    table_with_gap(&rows, 1)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_number_str() {
        assert_eq!("two", number_str(2));
        assert_eq!("twenty five", number_str(25));
        assert_eq!("one hundred and five", number_str(105));
        assert_eq!("1000", number_str(1000));
    }
}
