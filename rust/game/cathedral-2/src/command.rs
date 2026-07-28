//! Port of `command.go`/`play_command.go`'s parser-construction side: single
//! command `play <piece> <loc> [<dir>]`.

use brdgme_game::command::parser::*;

use crate::Game;
use crate::loc::{self, DIR_DOWN, Dir, Loc};
use crate::piece::pieces;

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Play { piece: i32, loc: Loc, dir: Dir },
}

/// A direction choice paired with its string name. Port of the
/// `OrthoDirNames` enum values built in `DirParser` (`command.go`).
#[derive(Debug, Clone, Copy)]
struct DirChoice {
    dir: Dir,
    name: &'static str,
}

impl std::fmt::Display for DirChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Game {
    /// Port of `CommandParser` (`command.go`).
    pub fn command_parser(&self, player: i32) -> Option<Box<dyn Parser<T = Command> + '_>> {
        // Single choke point for both `Gamer::command` and
        // `Gamer::command_spec`, which each build their parser here (c F25).
        if player < 0 || player as usize >= self.players {
            return None;
        }
        if self.can_play(player) {
            Some(Box::new(self.play_parser(player)))
        } else {
            None
        }
    }

    /// Port of `PlayParser` (`command.go`).
    fn play_parser(&self, player: i32) -> impl Parser<T = Command> {
        Map::new(
            Chain4::new(
                Doc::name_desc("play", "play a piece to the board", Token::new("play")),
                AfterSpace::new(Doc::name_desc(
                    "piece",
                    "the piece to play",
                    piece_parser(player),
                )),
                AfterSpace::new(Doc::name_desc(
                    "loc",
                    "the location to play at",
                    loc_parser(),
                )),
                Opt::new(AfterSpace::new(Doc::name_desc(
                    "dir",
                    "the direction to play the piece, or down if not specified",
                    dir_parser(),
                ))),
            ),
            |(_, piece, loc, dir): (String, i32, Loc, Option<Dir>)| Command::Play {
                piece,
                loc,
                dir: dir.unwrap_or(DIR_DOWN),
            },
        )
    }
}

/// Port of `PieceParser` (`command.go`): 1-based `Int{Min:1,
/// Max:len(Pieces[player])}` mapped to a 0-based index by subtracting 1.
fn piece_parser(player: i32) -> impl Parser<T = i32> {
    // `command_parser` rejects out-of-range players before this is built, so
    // the catalogue is always present here; `0` degrades to a parser that
    // matches nothing rather than panicking.
    let max = pieces(player).map_or(0, |p| p.len() as i32);
    Map::new(Int::bounded(1, max), |v: i32| v - 1)
}

/// Port of `LocParser` (`command.go`): an `Enum` over every `AllLocs[i].String()`.
///
/// `Loc`'s `Display` impl forwards verbatim to `to_key()`, and `Enum` only
/// needs `ToString + Clone`, so the locations go in directly - no wrapper
/// struct and no leaked `&'static str` name table (c F22).
fn loc_parser() -> impl Parser<T = Loc> {
    Enum::partial(loc::all_locs())
}

/// Port of `DirParser` (`command.go`): an `Enum` over `OrthoDirNames`.
fn dir_parser() -> impl Parser<T = Dir> {
    let values: Vec<DirChoice> = loc::ORTHO_DIRS
        .iter()
        .map(|&d| DirChoice {
            dir: d,
            name: loc::ortho_dir_name(d),
        })
        .collect();
    Map::new(Enum::partial(values), |c: DirChoice| c.dir)
}

#[cfg(test)]
mod tests {
    use brdgme_game::command::Spec;
    use brdgme_game::command::parser::Parser;

    use super::loc_parser;
    use crate::loc;

    #[test]
    fn loc_parser_spec_is_every_board_location_in_row_major_order() {
        // c F22 lock-in: dropping the leaked `&'static str` name table must
        // not change the accepted grammar or the advertised command spec.
        match loc_parser().to_spec() {
            Spec::Enum { values, exact } => {
                assert!(!exact, "locations are matched by prefix");
                assert_eq!(100, values.len());
                assert_eq!("A1", values[0]);
                assert_eq!("J10", values[99]);
                let expected: Vec<String> = loc::all_locs().iter().map(|l| l.to_key()).collect();
                assert_eq!(expected, values);
            }
            s => panic!("expected an Enum spec, got {:?}", s),
        }
    }

    #[test]
    fn loc_parser_parses_a_full_and_a_partial_location() {
        let names: Vec<String> = vec!["mick".to_string(), "steve".to_string()];
        let out = loc_parser().parse("f6", &names).expect("f6 must parse");
        assert_eq!(loc::Loc::new(5, 5), out.value);
        let out = loc_parser()
            .parse("j10 rest", &names)
            .expect("j10 must parse");
        assert_eq!(loc::Loc::new(9, 9), out.value);
        assert_eq!(" rest", out.remaining);
    }
}
