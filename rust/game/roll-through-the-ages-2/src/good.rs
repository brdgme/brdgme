//! Port of `brdgme-go/roll_through_the_ages_1/good.go`.

use brdgme_color::{self as color, Color};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Good {
    Wood,
    Stone,
    Pottery,
    Cloth,
    Spearhead,
}

pub const GOODS: [Good; 5] = [
    Good::Wood,
    Good::Stone,
    Good::Pottery,
    Good::Cloth,
    Good::Spearhead,
];

/// Port of `GoodsReversed`.
pub fn goods_reversed() -> [Good; 5] {
    let mut rev = GOODS;
    rev.reverse();
    rev
}

impl Good {
    /// 0-based index, matching Go's `Good` iota value.
    pub fn index(self) -> usize {
        match self {
            Good::Wood => 0,
            Good::Stone => 1,
            Good::Pottery => 2,
            Good::Cloth => 3,
            Good::Spearhead => 4,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Good::Wood => "wood",
            Good::Stone => "stone",
            Good::Pottery => "pottery",
            Good::Cloth => "cloth",
            Good::Spearhead => "spearhead",
        }
    }

    pub fn colour(self) -> Color {
        match self {
            Good::Wood => color::PURPLE,
            Good::Stone => color::GREY,
            Good::Pottery => color::RED,
            Good::Cloth => color::BLUE,
            Good::Spearhead => color::YELLOW,
        }
    }
}

/// Port of `GoodMaximum`.
pub fn good_maximum(good: Good) -> i32 {
    8 - good.index() as i32
}

/// Port of `GoodValue`.
pub fn good_value(good: Good, n: i32) -> i32 {
    (n * (n + 1) / 2) * (good.index() as i32 + 1)
}

#[cfg(test)]
mod test {
    use super::*;

    // Port of TestGoodMaximum (good_test.go).
    #[test]
    fn test_good_maximum() {
        assert_eq!(8, good_maximum(Good::Wood));
        assert_eq!(7, good_maximum(Good::Stone));
        assert_eq!(6, good_maximum(Good::Pottery));
        assert_eq!(5, good_maximum(Good::Cloth));
        assert_eq!(4, good_maximum(Good::Spearhead));
    }

    // Port of TestGoodValue (good_test.go).
    #[test]
    fn test_good_value() {
        assert_eq!(1, good_value(Good::Wood, 1));
        assert_eq!(10, good_value(Good::Wood, 4));
        assert_eq!(36, good_value(Good::Wood, 8));

        assert_eq!(2, good_value(Good::Stone, 1));
        assert_eq!(12, good_value(Good::Stone, 3));
        assert_eq!(56, good_value(Good::Stone, 7));

        assert_eq!(3, good_value(Good::Pottery, 1));
        assert_eq!(18, good_value(Good::Pottery, 3));
        assert_eq!(63, good_value(Good::Pottery, 6));

        assert_eq!(4, good_value(Good::Cloth, 1));
        assert_eq!(24, good_value(Good::Cloth, 3));
        assert_eq!(60, good_value(Good::Cloth, 5));

        assert_eq!(5, good_value(Good::Spearhead, 1));
        assert_eq!(30, good_value(Good::Spearhead, 3));
        assert_eq!(50, good_value(Good::Spearhead, 4));
    }
}
