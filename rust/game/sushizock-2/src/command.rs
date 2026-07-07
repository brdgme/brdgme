use brdgme_game::command::parser::*;

use crate::Game;
use crate::TileType;

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Roll(Vec<i32>),
    Take(TileType),
    Steal {
        target: usize,
        kind: TileType,
        num: Option<i32>,
    },
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_roll(player) {
            parsers.push(Box::new(roll_parser(self.rolled_dice.len())));
        }
        if self.can_steal(player) {
            parsers.push(Box::new(steal_parser()));
        }
        if self.can_take(player) {
            parsers.push(Box::new(take_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn roll_parser(max: usize) -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc("roll", "roll dice", Token::new("roll")),
            AfterSpace::new(Doc::name_desc(
                "dice",
                "list of dice numbers to roll, separated by spaces",
                Many::bounded_spaced(Int::bounded(1, max as i32), 1, max),
            )),
        ),
        |(_, dice): (String, Vec<i32>)| Command::Roll(dice),
    )
}

pub fn steal_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain4::new(
            Doc::name_desc(
                "steal",
                "steal a tile from another player",
                Token::new("steal"),
            ),
            AfterSpace::new(Doc::name_desc(
                "opponent",
                "the opponent to steal from",
                Player {},
            )),
            AfterSpace::new(Doc::name_desc(
                "color",
                "whether to steal red or blue",
                Enum::partial(vec![TileType::Blue, TileType::Red]),
            )),
            Opt::new(AfterSpace::new(Doc::name_desc(
                "tile",
                "optional if you have 4 chopsticks, which tile to steal in the stack, 1 for top",
                Int::any(),
            ))),
        ),
        |(_, target, kind, num): (String, usize, TileType, Option<i32>)| Command::Steal {
            target,
            kind,
            num,
        },
    )
}

pub fn take_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc("take", "take a red or blue tile", Token::new("take")),
            AfterSpace::new(Doc::name_desc(
                "color",
                "whether to take red or blue",
                Enum::partial(vec![TileType::Blue, TileType::Red]),
            )),
        ),
        |(_, kind): (String, TileType)| Command::Take(kind),
    )
}
