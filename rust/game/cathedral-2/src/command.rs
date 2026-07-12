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

/// A location choice paired with its string name, for use with the `Enum`
/// parser. Port of the `AllLocs[i].String()` enum values built in
/// `LocParser` (`command.go`).
#[derive(Debug, Clone, Copy)]
struct LocChoice {
    loc: Loc,
    name: &'static str,
}

// Leaked once per process; the location set is fixed (100 entries), so this
// is a bounded, one-time allocation rather than a per-parse leak.
fn loc_name(loc: Loc) -> &'static str {
    Box::leak(loc.to_key().into_boxed_str())
}

impl std::fmt::Display for LocChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
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
    let max = pieces(player).len() as i32;
    Map::new(Int::bounded(1, max), |v: i32| v - 1)
}

/// Port of `LocParser` (`command.go`): an `Enum` over every `AllLocs[i].String()`.
fn loc_parser() -> impl Parser<T = Loc> {
    let values: Vec<LocChoice> = loc::all_locs()
        .into_iter()
        .map(|l| LocChoice {
            loc: l,
            name: loc_name(l),
        })
        .collect();
    Map::new(Enum::partial(values), |c: LocChoice| c.loc)
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
