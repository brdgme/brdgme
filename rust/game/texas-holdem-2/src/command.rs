//! Ported from `brdgme-go/texas_holdem_1/command.go`.

use brdgme_game::command::parser::*;

use crate::Game;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    AllIn,
    Call,
    Check,
    Fold,
    Raise(i32),
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_all_in(player) {
            parsers.push(Box::new(all_in_parser()));
        }
        if self.can_call(player) {
            parsers.push(Box::new(call_parser()));
        }
        if self.can_check(player) {
            parsers.push(Box::new(check_parser()));
        }
        if self.can_fold(player) {
            parsers.push(Box::new(fold_parser()));
        }
        if self.can_raise(player) {
            parsers.push(Box::new(self.raise_parser(player)));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    /// Port of `RaiseParser`.
    ///
    /// Go quirk preserved: the `Int` bound's `min` is `g.LargestRaise`, not
    /// `g.MinRaise()` (`max(MinimumBet, LargestRaise)`) - the raise action's
    /// own validation (`Raise`/`raise` below) uses `MinRaise()` for both the
    /// bound check and the error message, so the parser can accept an amount
    /// the action then rejects. Preserved as-is per the porting correctness
    /// rule; not fixed here.
    fn raise_parser(&self, player: usize) -> impl Parser<T = Command> {
        let behind_current_bet = self.current_bet() - self.bets[player];
        let min = self.largest_raise;
        let max = self.player_money[player] - behind_current_bet;
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "raise",
                    "bet higher than the highest bet by this amount",
                    Token::new("raise"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "amount",
                    "the amount to raise above the highest bet",
                    Int {
                        min: Some(min),
                        max: Some(max),
                    },
                )),
            ),
            |(_, amount)| Command::Raise(amount),
        )
    }
}

fn all_in_parser() -> impl Parser<T = Command> {
    Doc::name_desc(
        "allin",
        "bet all your money and go all in",
        Map::new(Token::new("allin"), |_| Command::AllIn),
    )
}

fn call_parser() -> impl Parser<T = Command> {
    Doc::name_desc(
        "call",
        "increase your bet to match the current bet",
        Map::new(Token::new("call"), |_| Command::Call),
    )
}

fn check_parser() -> impl Parser<T = Command> {
    Doc::name_desc(
        "check",
        "continue without betting more money",
        Map::new(Token::new("check"), |_| Command::Check),
    )
}

fn fold_parser() -> impl Parser<T = Command> {
    Doc::name_desc(
        "fold",
        "forfeit this hand",
        Map::new(Token::new("fold"), |_| Command::Fold),
    )
}
