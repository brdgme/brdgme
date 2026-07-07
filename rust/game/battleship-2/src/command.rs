use brdgme_game::command::parser::*;

use crate::{Direction, Game, Loc, Ship, all_locations};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Place {
        ship: Ship,
        loc: Loc,
        dir: Direction,
    },
    Shoot {
        loc: Loc,
    },
}

pub fn shoot_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc("shoot", "shoot at a location", Token::new("shoot")),
            AfterSpace::new(Doc::name_desc(
                "location",
                "the location to shoot at",
                Enum::exact(all_locations()),
            )),
        ),
        |(_, loc)| Command::Shoot { loc },
    )
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_place(player) {
            parsers.push(Box::new(self.place_parser(player)));
        }
        if self.can_shoot(player) {
            parsers.push(Box::new(shoot_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    pub fn place_parser(&self, player: usize) -> impl Parser<T = Command> {
        let ships: Vec<Ship> = self.left_to_place.get(player).cloned().unwrap_or_default();
        Map::new(
            Chain4::new(
                Doc::name_desc("place", "place a ship", Token::new("place")),
                AfterSpace::new(Doc::name_desc(
                    "ship",
                    "the ship to place",
                    Enum::partial(ships),
                )),
                AfterSpace::new(Doc::name_desc(
                    "location",
                    "the location to place the ship",
                    Enum::exact(all_locations()),
                )),
                AfterSpace::new(Doc::name_desc(
                    "direction",
                    "the direction to place the ship",
                    Enum::partial(Direction::all()),
                )),
            ),
            |(_, ship, loc, dir)| Command::Place { ship, loc, dir },
        )
    }
}
