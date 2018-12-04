use lazy_static::lazy_static;

use std::collections::HashMap;

use crate::board::{Block, Loc};
use crate::casino::Casino;

pub enum Payout {
    Casino(Casino),
    Strip,
}

pub struct Tile {
    pub payout: Payout,
    pub starting_cash: usize,
    pub die: usize,
    pub build_cost: usize,
    pub strip: bool,
}

type TileMap = HashMap<Loc, Tile>;

lazy_static! {
    pub static ref TILES: TileMap = tiles();
}

fn tiles() -> TileMap {
    let mut map: TileMap = HashMap::new();
    map.insert(
        (Block::A, 1).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::A, 2).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::A, 3).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 5,
            die: 5,
            build_cost: 15,
            strip: true,
        },
    );
    map.insert(
        (Block::A, 4).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: false,
        },
    );
    map.insert(
        (Block::A, 5).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::A, 6).into(),
        Tile {
            payout: Payout::Strip,
            starting_cash: 4,
            die: 6,
            build_cost: 20,
            strip: true,
        },
    );

    map.insert(
        (Block::B, 1).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 5,
            die: 5,
            build_cost: 15,
            strip: true,
        },
    );
    map.insert(
        (Block::B, 2).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::B, 3).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::B, 4).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 4,
            die: 6,
            build_cost: 20,
            strip: true,
        },
    );
    map.insert(
        (Block::B, 5).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::B, 6).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: false,
        },
    );

    map.insert(
        (Block::C, 1).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 2).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 3).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 4,
            die: 6,
            build_cost: 20,
            strip: true,
        },
    );
    map.insert(
        (Block::C, 4).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 5).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 9,
            die: 1,
            build_cost: 8,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 6).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: true,
        },
    );
    map.insert(
        (Block::C, 7).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 8).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 9,
            die: 1,
            build_cost: 8,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 9).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: true,
        },
    );
    map.insert(
        (Block::C, 10).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 11).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::C, 12).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 5,
            die: 5,
            build_cost: 15,
            strip: true,
        },
    );

    map.insert(
        (Block::D, 1).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 4,
            die: 6,
            build_cost: 20,
            strip: true,
        },
    );
    map.insert(
        (Block::D, 2).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::D, 3).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: false,
        },
    );
    map.insert(
        (Block::D, 4).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: true,
        },
    );
    map.insert(
        (Block::D, 5).into(),
        Tile {
            payout: Payout::Strip,
            starting_cash: 9,
            die: 1,
            build_cost: 8,
            strip: false,
        },
    );
    map.insert(
        (Block::D, 6).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::D, 7).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 5,
            die: 5,
            build_cost: 15,
            strip: true,
        },
    );
    map.insert(
        (Block::D, 8).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::D, 9).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );

    map.insert(
        (Block::E, 1).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::E, 2).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::E, 3).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 5,
            die: 5,
            build_cost: 15,
            strip: true,
        },
    );
    map.insert(
        (Block::E, 4).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: false,
        },
    );
    map.insert(
        (Block::E, 5).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::E, 6).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 4,
            die: 6,
            build_cost: 20,
            strip: true,
        },
    );

    map.insert(
        (Block::F, 1).into(),
        Tile {
            payout: Payout::Casino(Casino::Albion),
            starting_cash: 4,
            die: 6,
            build_cost: 20,
            strip: true,
        },
    );
    map.insert(
        (Block::F, 2).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map.insert(
        (Block::F, 3).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: false,
        },
    );
    map.insert(
        (Block::F, 4).into(),
        Tile {
            payout: Payout::Casino(Casino::Sphinx),
            starting_cash: 6,
            die: 4,
            build_cost: 12,
            strip: true,
        },
    );
    map.insert(
        (Block::F, 5).into(),
        Tile {
            payout: Payout::Casino(Casino::Pioneer),
            starting_cash: 9,
            die: 1,
            build_cost: 8,
            strip: false,
        },
    );
    map.insert(
        (Block::F, 6).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::F, 7).into(),
        Tile {
            payout: Payout::Casino(Casino::Vega),
            starting_cash: 5,
            die: 5,
            build_cost: 15,
            strip: true,
        },
    );
    map.insert(
        (Block::F, 8).into(),
        Tile {
            payout: Payout::Strip,
            starting_cash: 8,
            die: 2,
            build_cost: 6,
            strip: false,
        },
    );
    map.insert(
        (Block::F, 9).into(),
        Tile {
            payout: Payout::Casino(Casino::Tivoli),
            starting_cash: 7,
            die: 3,
            build_cost: 9,
            strip: false,
        },
    );
    map
}
