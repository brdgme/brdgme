use brdgme_game::Gamer;
use brdgme_game::command::parser::*;

use crate::Game;
use crate::{MAX_BID_VALUE, MIN_BID_QUANTITY, MIN_BID_VALUE, START_DICE_COUNT};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Bid { quantity: i32, value: i32 },
    Call,
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_bid(player) {
            parsers.push(Box::new(bid_parser(self.players)));
        }
        if self.can_call(player) {
            parsers.push(Box::new(call_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn bid_parser(players: usize) -> impl Parser<T = Command> {
    Map::new(
        Chain3::new(
            Doc::name_desc(
                "bid",
                "bid the number of dice under all players' cups",
                Token::new("bid"),
            ),
            AfterSpace::new(Doc::name_desc(
                "quantity",
                "the quantity of dice to bid",
                Int {
                    min: Some(MIN_BID_QUANTITY),
                    max: Some((players * START_DICE_COUNT) as i32),
                },
            )),
            AfterSpace::new(Doc::name_desc(
                "value",
                "the face value of dice to bid, including wild dice (1)",
                Int {
                    min: Some(MIN_BID_VALUE),
                    max: Some(MAX_BID_VALUE),
                },
            )),
        ),
        |(_, quantity, value)| Command::Bid { quantity, value },
    )
}

pub fn call_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc("call", "call that the bid is too high", Token::new("call")),
        |_| Command::Call,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use brdgme_game::command::Spec as CommandSpec;

    fn quantity_max(spec: &CommandSpec) -> Option<Option<i32>> {
        match spec {
            CommandSpec::Doc { name, spec, .. } if name == "quantity" => match spec.as_ref() {
                CommandSpec::Int { max, .. } => Some(*max),
                _ => None,
            },
            CommandSpec::Chain(specs) | CommandSpec::OneOf(specs) => {
                specs.iter().find_map(quantity_max)
            }
            _ => None,
        }
    }

    #[test]
    fn bid_parser_quantity_max_is_total_dice() {
        let players = 3;
        let cap = (players * START_DICE_COUNT) as i32;
        let parser = bid_parser(players);
        assert_eq!(quantity_max(&parser.to_spec()), Some(Some(cap)));

        let at_cap = format!("bid {} 6", cap);
        let out = parser
            .parse(&at_cap, &[])
            .expect("a bid at the cap must parse");
        assert_eq!(
            out.value,
            Command::Bid {
                quantity: cap,
                value: 6
            }
        );

        let over_cap = format!("bid {} 6", cap + 1);
        parser
            .parse(&over_cap, &[])
            .expect_err("a bid above the cap must be rejected");
    }
}
