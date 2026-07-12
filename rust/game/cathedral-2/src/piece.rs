//! Port of `piece.go`.

use crate::loc::Loc;
use crate::tile::{PLAYER_CATHEDRAL, PlayerType};

/// Port of `Piece` (`piece.go`). `directional` is carried but, per the Go
/// source, is not read anywhere outside this struct's definition - it is
/// dead data preserved for fidelity (rotation/placement is not gated by it;
/// see `CanPlayPiece`/`Play` in `play_command.go`, which rotate by `dir`
/// regardless of this flag).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub player_type: PlayerType,
    pub positions: Vec<Loc>,
    pub directional: bool,
}

impl Piece {
    /// Port of `Piece.Bounds`.
    pub fn bounds(&self) -> (Loc, Loc) {
        let mut lower = Loc::new(0, 0);
        let mut upper = Loc::new(0, 0);
        for (i, &l) in self.positions.iter().enumerate() {
            if i == 0 || l.x < lower.x {
                lower.x = l.x;
            }
            if i == 0 || l.y < lower.y {
                lower.y = l.y;
            }
            if i == 0 || l.x > upper.x {
                upper.x = l.x;
            }
            if i == 0 || l.y > upper.y {
                upper.y = l.y;
            }
        }
        (lower, upper)
    }

    /// Port of `Piece.Width`.
    pub fn width(&self) -> i32 {
        let (lower, upper) = self.bounds();
        upper.sub(lower).x + 1
    }
}

fn p(player: i32, typ: i32, positions: &[(i32, i32)], directional: bool) -> Piece {
    Piece {
        player_type: PlayerType { player, typ },
        positions: positions.iter().map(|&(x, y)| Loc::new(x, y)).collect(),
        directional,
    }
}

/// Port of `Pieces[0]` (`piece.go`): player 0's 14 pieces, indices `0..13`.
fn player_0_pieces() -> Vec<Piece> {
    vec![
        p(0, 1, &[(0, 0), (0, 1), (-1, 1), (0, 2), (1, 2)], true),
        p(0, 2, &[(0, 0), (0, 1), (1, 1), (1, 2), (2, 2)], true),
        p(0, 3, &[(0, 0), (0, 1), (-1, 1), (1, 1), (0, 2)], false),
        p(0, 4, &[(0, 0), (1, 0), (0, 1), (0, 2), (1, 2)], true),
        p(0, 5, &[(0, 0), (0, 1), (1, 1), (0, 2)], true),
        p(0, 6, &[(0, 0), (0, 1), (1, 1), (1, 2)], true),
        p(0, 7, &[(0, 0), (1, 0), (0, 1), (1, 1)], false),
        p(0, 8, &[(0, 0), (0, 1), (0, 2)], true),
        p(0, 9, &[(0, 0), (0, 1), (1, 1)], true),
        p(0, 10, &[(0, 0), (0, 1), (1, 1)], true),
        p(0, 11, &[(0, 0), (0, 1)], true),
        p(0, 12, &[(0, 0), (0, 1)], true),
        p(0, 13, &[(0, 0)], false),
        p(0, 14, &[(0, 0)], false),
    ]
}

/// Port of `Pieces[1]` (`piece.go`): player 1's 15 pieces, index 0 is the
/// Cathedral (`PlayerType{PlayerCathedral, 1}`).
fn player_1_pieces() -> Vec<Piece> {
    vec![
        p(
            PLAYER_CATHEDRAL,
            1,
            &[(0, 0), (0, 1), (0, 2), (-1, 2), (1, 2), (0, 3)],
            true,
        ),
        p(1, 2, &[(0, 0), (0, 1), (1, 1), (0, 2), (-1, 2)], true),
        p(1, 3, &[(0, 0), (0, 1), (1, 1), (1, 2), (2, 2)], true),
        p(1, 4, &[(0, 0), (0, 1), (-1, 1), (1, 1), (0, 2)], false),
        p(1, 5, &[(0, 0), (1, 0), (0, 1), (0, 2), (1, 2)], true),
        p(1, 6, &[(0, 0), (0, 1), (1, 1), (0, 2)], true),
        p(1, 7, &[(0, 0), (0, 1), (-1, 1), (-1, 2)], true),
        p(1, 8, &[(0, 0), (1, 0), (0, 1), (1, 1)], false),
        p(1, 9, &[(0, 0), (0, 1), (0, 2)], true),
        p(1, 10, &[(0, 0), (0, 1), (1, 1)], true),
        p(1, 11, &[(0, 0), (0, 1), (1, 1)], true),
        p(1, 12, &[(0, 0), (0, 1)], true),
        p(1, 13, &[(0, 0), (0, 1)], true),
        p(1, 14, &[(0, 0)], false),
        p(1, 15, &[(0, 0)], false),
    ]
}

/// Port of the `Pieces` package var (`piece.go`), the full piece catalogue
/// keyed by player index (0 or 1).
pub fn pieces(player: i32) -> Vec<Piece> {
    match player {
        0 => player_0_pieces(),
        1 => player_1_pieces(),
        _ => panic!("invalid player: {}", player),
    }
}
