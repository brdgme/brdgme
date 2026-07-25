use brdgme_game::command::parser::*;

use crate::Game;
use crate::card::Card;

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Play { card: Card },
    Discard { card: Card },
    Done,
}

struct CardParser;

impl Parser for CardParser {
    type T = Card;

    fn parse<'a>(
        &self,
        input: &'a str,
        _names: &[String],
    ) -> Result<Output<'a, Card>, brdgme_game::errors::GameError> {
        let chars: Vec<char> = input.chars().collect();
        if chars.len() < 2 {
            return Err(brdgme_game::errors::GameError::Parse {
                message: Some("the card must be a letter followed by a number, eg. r6".to_string()),
                expected: self.expected(_names),
                offset: 0,
            });
        }
        // Byte length of the first two chars: `2` as a byte index cuts
        // multi-byte input (e.g. "r€") mid-char and panics.
        let split = chars[0].len_utf8() + chars[1].len_utf8();
        match Card::parse(&input[..split]) {
            Some(card) => Ok(Output {
                value: card,
                consumed: &input[..split],
                remaining: &input[split..],
            }),
            None => Err(brdgme_game::errors::GameError::Parse {
                message: Some("the card must be a letter followed by a number, eg. r6".to_string()),
                expected: self.expected(_names),
                offset: 0,
            }),
        }
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        vec!["a card like r6 or b4".to_string()]
    }

    fn to_spec(&self) -> brdgme_game::command::Spec {
        brdgme_game::command::Spec::Token("card".to_string())
    }
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.finished || self.current_player != player {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];

        if self.can_play(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "play",
                        "play a card to your palette, eg. play b4",
                        Token::new("play"),
                    ),
                    AfterSpace::new(CardParser),
                ),
                |(_, card): (String, Card)| Command::Play { card },
            )));
        }

        if self.can_discard(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "discard",
                        "discard a card and set the new rule, eg. discard b4",
                        Token::new("discard"),
                    ),
                    AfterSpace::new(CardParser),
                ),
                |(_, card): (String, Card)| Command::Discard { card },
            )));
        }

        if self.can_done(player) {
            parsers.push(Box::new(Map::new(
                Doc::name_desc(
                    "done",
                    "finish your turn, you will be eliminated if you haven't played or discarded a card or if you aren't the leader",
                    Token::new("done"),
                ),
                |_| Command::Done,
            )));
        }

        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Suit;

    #[test]
    fn card_parser_handles_multibyte_input() {
        let parser = CardParser;
        // '€' is 3 bytes: the old byte-index-2 slice panicked mid-char.
        parser
            .parse("r€", &[])
            .expect_err("expected 'r€' to produce an error");
        parser
            .parse("€5", &[])
            .expect_err("expected '€5' to produce an error");
        parser
            .parse("r\u{a0}", &[])
            .expect_err("expected 'r' + NBSP to produce an error");
        // ASCII behavior unchanged.
        let out = parser.parse("r6x", &[]).expect("expected 'r6x' to parse");
        assert_eq!(
            out.value,
            Card {
                suit: Suit::Red,
                rank: 6
            }
        );
        assert_eq!(out.consumed, "r6");
        assert_eq!(out.remaining, "x");
        parser
            .parse("r", &[])
            .expect_err("expected single char to produce an error");
    }
}
