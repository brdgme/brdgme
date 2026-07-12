use brdgme_color::{BLUE, Color, GREEN, GREY, PURPLE, RED};
use brdgme_markup::Node as N;
use serde::{Deserialize, Serialize};

/// Port of brdgme-go/age_of_war_1/dice.go dice face constants.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum Die {
    Inf1,
    Inf2,
    Inf3,
    Archery,
    Cavalry,
    Daimyo,
}

pub const ALL_DICE: [Die; 6] = [
    Die::Inf1,
    Die::Inf2,
    Die::Inf3,
    Die::Archery,
    Die::Cavalry,
    Die::Daimyo,
];

impl Die {
    /// Port of DiceInfantry - Some(n) if this die counts as n infantry.
    pub fn infantry(self) -> Option<u32> {
        match self {
            Die::Inf1 => Some(1),
            Die::Inf2 => Some(2),
            Die::Inf3 => Some(3),
            _ => None,
        }
    }

    /// Port of DiceStrings.
    pub fn label(self) -> &'static str {
        match self {
            Die::Inf1 => "1 inf",
            Die::Inf2 => "2 inf",
            Die::Inf3 => "3 inf",
            Die::Archery => "arch",
            Die::Cavalry => "cav",
            Die::Daimyo => "dai",
        }
    }

    /// Port of DiceColours (InfantryColour = render.Blue for the inf faces).
    pub fn colour(self) -> Color {
        match self {
            Die::Inf1 | Die::Inf2 | Die::Inf3 => BLUE,
            Die::Archery => PURPLE,
            Die::Cavalry => GREEN,
            Die::Daimyo => RED,
        }
    }

    /// Port of RenderDie.
    pub fn render(self) -> N {
        N::Bold(vec![N::Fg(
            self.colour().into(),
            vec![N::text(self.label())],
        )])
    }
}

/// Port of the Clan* iota constants in castles.go.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum Clan {
    Oda,
    Tokugawa,
    Uesugi,
    Mori,
    Chosokabe,
    Shimazu,
}

pub const ALL_CLANS: [Clan; 6] = [
    Clan::Oda,
    Clan::Tokugawa,
    Clan::Uesugi,
    Clan::Mori,
    Clan::Chosokabe,
    Clan::Shimazu,
];

impl Clan {
    /// Port of ClanSetPoints.
    pub fn set_points(self) -> u32 {
        match self {
            Clan::Oda => 10,
            Clan::Tokugawa => 8,
            Clan::Uesugi => 8,
            Clan::Mori => 5,
            Clan::Chosokabe => 4,
            Clan::Shimazu => 3,
        }
    }

    /// Port of ClanNames.
    pub fn name(self) -> &'static str {
        match self {
            Clan::Oda => "Oda",
            Clan::Tokugawa => "Tokugawa",
            Clan::Uesugi => "Uesugi",
            Clan::Mori => "Mori",
            Clan::Chosokabe => "Chosokabe",
            Clan::Shimazu => "Shimazu",
        }
    }

    /// Port of ClanColours.
    pub fn colour(self) -> Color {
        match self {
            Clan::Oda => brdgme_color::YELLOW,
            Clan::Tokugawa => GREY,
            Clan::Uesugi => PURPLE,
            Clan::Mori => RED,
            Clan::Chosokabe => brdgme_color::BLACK,
            Clan::Shimazu => GREEN,
        }
    }

    /// Port of RenderClan.
    pub fn render(self) -> N {
        N::Bold(vec![N::Fg(
            self.colour().into(),
            vec![N::text(self.name())],
        )])
    }
}

/// Port of the Line struct in castles.go.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Line {
    pub infantry: u32,
    pub symbols: Vec<Die>,
}

impl Line {
    /// Port of Line.MinDice.
    pub fn min_dice(&self) -> usize {
        self.symbols.len() + (self.infantry as usize).div_ceil(3)
    }

    /// Port of Line.CanAfford. Returns (can_afford, dice_used).
    pub fn can_afford(&self, with: &[Die]) -> (bool, usize) {
        let mut symbols: Vec<Die> = vec![];
        let mut inf: Vec<u32> = vec![];
        for &w in with {
            if let Some(i) = w.infantry() {
                inf.push(i);
            } else {
                symbols.push(w);
            }
        }
        // Sort descending, matching Go's sort.Reverse(sort.IntSlice(inf)).
        inf.sort_unstable_by(|a, b| b.cmp(a));

        let mut can = int_slice_sub(&symbols, &self.symbols);
        let mut using = self.symbols.len();

        let mut rem_inf = self.infantry as i64;
        for i in inf {
            if rem_inf <= 0 {
                break;
            }
            rem_inf -= i as i64;
            using += 1;
        }
        if rem_inf > 0 {
            can = false;
        }

        (can, using)
    }

    /// Port of Line.RenderRow.
    pub fn render_row(&self) -> Vec<N> {
        let mut row: Vec<N> = vec![];
        for &s in &self.symbols {
            row.push(s.render());
        }
        if self.infantry > 0 {
            row.push(render_inf(self.infantry));
        }
        row
    }
}

/// Port of RenderInf.
pub fn render_inf(n: u32) -> N {
    N::Bold(vec![N::Fg(
        BLUE.into(),
        vec![N::text(format!("{} inf", n))],
    )])
}

/// Port of brdgme.IntSliceSub - true iff every element of `sub` is present
/// (as a multiset) in `ints`.
fn int_slice_sub(ints: &[Die], sub: &[Die]) -> bool {
    let mut sub_counts: Vec<Die> = sub.to_vec();
    for i in ints {
        if let Some(pos) = sub_counts.iter().position(|s| s == i) {
            sub_counts.remove(pos);
        }
    }
    sub_counts.is_empty()
}

/// Port of the Castle struct in castles.go.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Castle {
    pub clan: Clan,
    pub name: &'static str,
    pub points: u32,
    /// Lines from top to bottom on the card, not including the special
    /// Daimyo line added when stealing.
    pub lines: Vec<Line>,
}

impl Castle {
    /// Port of Castle.MinDice.
    pub fn min_dice(&self) -> usize {
        self.lines.iter().map(Line::min_dice).sum()
    }

    /// Port of Castle.CalcLines.
    pub fn calc_lines(&self, stealing: bool) -> Vec<Line> {
        let mut lines = self.lines.clone();
        if stealing {
            lines.push(Line {
                infantry: 0,
                symbols: vec![Die::Daimyo],
            });
        }
        lines
    }

    /// Port of Castle.RenderName.
    pub fn render_name(&self) -> N {
        N::Bold(vec![N::Fg(
            self.clan.colour().into(),
            vec![N::text(self.name)],
        )])
    }
}

fn line(infantry: u32, symbols: Vec<Die>) -> Line {
    Line { infantry, symbols }
}

fn no_symbols(infantry: u32) -> Line {
    line(infantry, vec![])
}

/// Port of the Castles table in castles.go. Order matters - it groups
/// castles by clan and drives board layout/iteration order.
pub fn castles() -> Vec<Castle> {
    vec![
        // Clan Oda
        Castle {
            clan: Clan::Oda,
            name: "Azuchi",
            points: 3,
            lines: vec![
                line(0, vec![Die::Archery]),
                line(0, vec![Die::Cavalry, Die::Cavalry]),
                no_symbols(5),
            ],
        },
        Castle {
            clan: Clan::Oda,
            name: "Matsumoto",
            points: 2,
            lines: vec![
                line(0, vec![Die::Archery]),
                line(0, vec![Die::Archery]),
                no_symbols(7),
            ],
        },
        Castle {
            clan: Clan::Oda,
            name: "Odani",
            points: 1,
            lines: vec![no_symbols(10)],
        },
        Castle {
            clan: Clan::Oda,
            name: "Gifu",
            points: 1,
            lines: vec![
                line(0, vec![Die::Daimyo]),
                line(0, vec![Die::Archery]),
                line(0, vec![Die::Cavalry]),
            ],
        },
        // Clan Tokugawa
        Castle {
            clan: Clan::Tokugawa,
            name: "Edo",
            points: 3,
            lines: vec![
                line(0, vec![Die::Archery, Die::Cavalry]),
                line(0, vec![Die::Archery, Die::Cavalry]),
                no_symbols(3),
            ],
        },
        Castle {
            clan: Clan::Tokugawa,
            name: "Kiyosu",
            points: 2,
            lines: vec![
                line(0, vec![Die::Daimyo]),
                line(0, vec![Die::Archery]),
                line(0, vec![Die::Cavalry]),
                no_symbols(3),
            ],
        },
        Castle {
            clan: Clan::Tokugawa,
            name: "Inuyama",
            points: 1,
            lines: vec![
                line(0, vec![Die::Daimyo]),
                line(0, vec![Die::Archery, Die::Archery]),
            ],
        },
        // Clan Uesugi
        Castle {
            clan: Clan::Uesugi,
            name: "Kasugayama",
            points: 4,
            lines: vec![
                line(0, vec![Die::Archery, Die::Archery]),
                line(0, vec![Die::Cavalry, Die::Cavalry]),
            ],
        },
        Castle {
            clan: Clan::Uesugi,
            name: "Kitanosho",
            points: 3,
            lines: vec![
                line(0, vec![Die::Daimyo]),
                line(0, vec![Die::Archery, Die::Cavalry]),
                no_symbols(6),
            ],
        },
        // Clan Mori
        Castle {
            clan: Clan::Mori,
            name: "Gassantoda",
            points: 2,
            lines: vec![line(0, vec![Die::Daimyo]), no_symbols(8)],
        },
        Castle {
            clan: Clan::Mori,
            name: "Takahashi",
            points: 2,
            lines: vec![
                line(0, vec![Die::Cavalry, Die::Cavalry]),
                no_symbols(5),
                no_symbols(2),
            ],
        },
        // Clan Chosokabe
        Castle {
            clan: Clan::Chosokabe,
            name: "Matsuyama",
            points: 2,
            lines: vec![line(0, vec![Die::Daimyo]), no_symbols(4), no_symbols(4)],
        },
        Castle {
            clan: Clan::Chosokabe,
            name: "Marugame",
            points: 1,
            lines: vec![
                line(0, vec![Die::Daimyo, Die::Daimyo]),
                line(0, vec![Die::Cavalry]),
            ],
        },
        // Clan Shimazu
        Castle {
            clan: Clan::Shimazu,
            name: "Kumamoto",
            points: 3,
            lines: vec![
                line(0, vec![Die::Daimyo, Die::Daimyo]),
                line(0, vec![Die::Cavalry]),
                line(0, vec![Die::Archery]),
                no_symbols(4),
            ],
        },
    ]
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fourteen_castles() {
        assert_eq!(14, castles().len());
    }

    #[test]
    fn line_min_dice() {
        // {Infantry: 5} => 0 symbols + ceil(5/3) = 0 + 2 = 2
        assert_eq!(2, no_symbols(5).min_dice());
        // {Symbols: [Archery]} => 1 symbol + 0 = 1
        assert_eq!(1, line(0, vec![Die::Archery]).min_dice());
        // {Infantry: 3} => 0 + ceil(3/3) = 1
        assert_eq!(1, no_symbols(3).min_dice());
    }

    #[test]
    fn line_can_afford_symbols() {
        let l = line(0, vec![Die::Archery, Die::Cavalry]);
        assert_eq!(
            (true, 2),
            l.can_afford(&[Die::Archery, Die::Cavalry, Die::Inf1])
        );
        assert_eq!((false, 2), l.can_afford(&[Die::Archery, Die::Archery]));
    }

    #[test]
    fn line_can_afford_infantry() {
        let l = no_symbols(5);
        // 3 + 2 = 5, uses 2 dice
        assert_eq!((true, 2), l.can_afford(&[Die::Inf3, Die::Inf2]));
        // 3 + 1 = 4, not enough
        assert_eq!((false, 2), l.can_afford(&[Die::Inf3, Die::Inf1]));
        // Descending consumption: uses just the 3 first (not enough alone),
        // then the 2, reaching exactly 5 with 2 dice, ignoring the leftover 1.
        assert_eq!((true, 2), l.can_afford(&[Die::Inf1, Die::Inf3, Die::Inf2]));
    }

    #[test]
    fn castle_calc_lines_stealing() {
        let c = &castles()[0];
        assert_eq!(3, c.lines.len());
        assert_eq!(3, c.calc_lines(false).len());
        assert_eq!(4, c.calc_lines(true).len());
        assert_eq!(
            vec![Die::Daimyo],
            c.calc_lines(true).last().unwrap().symbols
        );
    }
}
