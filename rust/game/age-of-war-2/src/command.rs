use brdgme_game::command::parser::*;

use crate::Game;
use crate::castle;

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Attack {
        castle: usize,
    },
    /// 1-based, matching the Go EnumFromInts input the player types.
    Line {
        line: i32,
    },
    Roll,
}

/// A castle name paired with its index, for use with the `Enum` parser
/// (which requires `ToString + Clone`). Port of the `brdgme.EnumValue`
/// entries built in `AttackParser` (command.go).
#[derive(Debug, Clone, Copy)]
struct CastleChoice {
    index: usize,
    name: &'static str,
}

impl std::fmt::Display for CastleChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_attack(player) {
            parsers.push(Box::new(self.attack_parser()));
        }
        if self.can_line(player) {
            parsers.push(Box::new(self.line_parser()));
        }
        if self.can_roll(player) {
            parsers.push(Box::new(roll_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    /// Port of AttackParser (command.go).
    fn attack_parser(&self) -> impl Parser<T = Command> {
        let all_castles = castle::castles();
        let remaining_castles: Vec<CastleChoice> = all_castles
            .iter()
            .enumerate()
            .filter(|(k, c)| {
                if self.conquered[*k] && self.castle_owners[*k] == Some(self.current_player) {
                    return false;
                }
                if self.clan_conquered(c.clan).0 {
                    return false;
                }
                true
            })
            .map(|(k, c)| CastleChoice {
                index: k,
                name: c.name,
            })
            .collect();
        Map::new(
            Chain2::new(
                Doc::name_desc("attack", "attack a castle", Token::new("attack")),
                AfterSpace::new(Doc::name_desc(
                    "castle",
                    "the castle to attack",
                    Enum::partial(remaining_castles),
                )),
            ),
            |(_, choice): (String, CastleChoice)| Command::Attack {
                castle: choice.index,
            },
        )
    }

    /// Port of LineParser (command.go).
    fn line_parser(&self) -> impl Parser<T = Command> {
        let currently_attacking = self.currently_attacking.expect("currently attacking");
        let all_castles = castle::castles();
        let castle_lines = all_castles[currently_attacking]
            .calc_lines(self.conquered[currently_attacking])
            .len();
        let remaining_lines: Vec<i32> = (0..castle_lines)
            .filter(|&i| !self.completed_lines.contains(&i))
            .map(|i| (i + 1) as i32)
            .collect();
        Map::new(
            Chain2::new(
                Doc::name_desc("line", "complete a castle line", Token::new("line")),
                AfterSpace::new(Doc::name_desc(
                    "line",
                    "the castle line to complete",
                    Enum::exact(remaining_lines),
                )),
            ),
            |(_, line): (String, i32)| Command::Line { line },
        )
    }
}

/// Port of rollParser (command.go).
fn roll_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "roll",
            "discard one dice and roll the rest",
            Token::new("roll"),
        ),
        |_| Command::Roll,
    )
}
