//! Port of `tile.go`.

use serde::{Deserialize, Serialize};

/// Port of the `NoPlayer` const.
pub const NO_PLAYER: i32 = -1;
/// Port of the `PlayerCathedral` const: the neutral "player" identity used
/// only for the Cathedral piece's tiles.
pub const PLAYER_CATHEDRAL: i32 = 2;

/// Port of `PlayerType` (`piece.go`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerType {
    pub player: i32,
    pub typ: i32,
}

/// Port of `Tile` (`tile.go`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub player: i32,
    pub typ: i32,
    pub owner: i32,
    pub text: String,
}

/// Port of `EmptyTile`.
pub fn empty_tile() -> Tile {
    Tile {
        player: NO_PLAYER,
        typ: 0,
        owner: NO_PLAYER,
        text: String::new(),
    }
}

impl Default for Tile {
    fn default() -> Self {
        empty_tile()
    }
}

impl Tile {
    pub fn player_type(&self) -> PlayerType {
        PlayerType {
            player: self.player,
            typ: self.typ,
        }
    }
}
