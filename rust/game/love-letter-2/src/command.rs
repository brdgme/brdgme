use brdgme_game::command::parser::*;

use crate::Game;
use crate::card::{Card, princess_to_guard};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Princess,
    Countess,
    King(usize),
    Prince(usize),
    Handmaid,
    Baron(usize),
    Priest(usize),
    Guard(usize, Card),
}

impl Game {
    /// Mirrors the Go `CommandParser`: only available on the current
    /// player's turn, `OneOf` of the card parsers in the same order as Go
    /// (princess, countess, king, prince, handmaid, baron, priest, guard),
    /// each only present if the player currently holds that card.
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.current_player != player {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        let hand = self.hands.get(player).cloned().unwrap_or_default();
        if hand.contains(&Card::Princess) {
            parsers.push(Box::new(princess_parser()));
        }
        if hand.contains(&Card::Countess) {
            parsers.push(Box::new(countess_parser()));
        }
        if hand.contains(&Card::King) {
            parsers.push(Box::new(king_parser()));
        }
        if hand.contains(&Card::Prince) {
            parsers.push(Box::new(prince_parser()));
        }
        if hand.contains(&Card::Handmaid) {
            parsers.push(Box::new(handmaid_parser()));
        }
        if hand.contains(&Card::Baron) {
            parsers.push(Box::new(baron_parser()));
        }
        if hand.contains(&Card::Priest) {
            parsers.push(Box::new(priest_parser()));
        }
        if hand.contains(&Card::Guard) {
            parsers.push(Box::new(guard_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn princess_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "princess",
            "play the Princess card (you will be eliminated)",
            Token::new("princess"),
        ),
        |_| Command::Princess,
    )
}

pub fn countess_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "countess",
            "play the Countess card, which you must do if you also have the King or Prince",
            Token::new("countess"),
        ),
        |_| Command::Countess,
    )
}

pub fn king_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "king",
                "play the King card to trade your hand with another player",
                Token::new("king"),
            ),
            AfterSpace::new(Doc::name_desc(
                "player",
                "the player to trade hands with",
                Player {},
            )),
        ),
        |(_, target)| Command::King(target),
    )
}

pub fn prince_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "prince",
                "play the Prince card to make a player discard their hand, including yourself",
                Token::new("prince"),
            ),
            AfterSpace::new(Doc::name_desc(
                "player",
                "the player to discard their hand, including yourself",
                Player {},
            )),
        ),
        |(_, target)| Command::Prince(target),
    )
}

pub fn handmaid_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "handmaid",
            "play the Handmaid card, which protects you from being targeted until your next turn",
            Token::new("handmaid"),
        ),
        |_| Command::Handmaid,
    )
}

pub fn baron_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "baron",
                "play the Baron card to compare your hand to another player, lowest hand is eliminated",
                Token::new("baron"),
            ),
            AfterSpace::new(Doc::name_desc(
                "player",
                "the player to compare your hand to",
                Player {},
            )),
        ),
        |(_, target)| Command::Baron(target),
    )
}

pub fn priest_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            // NB: the Go source's Doc description says "Baron" here, not
            // "Priest" - this is a known typo in `love_letter_1/command.go`
            // (`PriestParser.Parser.Desc`), preserved verbatim per the
            // porting correctness rule.
            Doc::name_desc(
                "priest",
                "play the Baron card to peek at another player's hand",
                Token::new("priest"),
            ),
            AfterSpace::new(Doc::name_desc("player", "the player to peek", Player {})),
        ),
        |(_, target)| Command::Priest(target),
    )
}

pub fn card_parser() -> impl Parser<T = Card> {
    Enum::exact(princess_to_guard())
}

pub fn guard_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain3::new(
            Doc::name_desc(
                "guard",
                "play the Guard card to guess the card of another player, eliminating them if correct",
                Token::new("guard"),
            ),
            AfterSpace::new(Doc::name_desc("player", "the player to target", Player {})),
            AfterSpace::new(Doc::name_desc(
                "card",
                "the card you think they are",
                card_parser(),
            )),
        ),
        |(_, target, card)| Command::Guard(target, card),
    )
}
