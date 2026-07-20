use std::collections::HashMap;

use brdgme_cost::Cost;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Good {
    Coin,
    Wood,
    Stone,
    Ore,
    Clay,
    Papyrus,
    Textile,
    Glass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CardKind {
    Raw,
    Manufactured,
    Civilian,
    Scientific,
    Commercial,
    Military,
    Guild,
    Wonder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Field {
    Mathematics,
    Engineering,
    Theology,
}

pub const DIR_LEFT: i32 = -1;
pub const DIR_DOWN: i32 = 0;
pub const DIR_RIGHT: i32 = 1;
pub const DIR_ALL: &[i32] = &[DIR_LEFT, DIR_DOWN, DIR_RIGHT];
pub const DIR_NEIGHBOURS: &[i32] = &[DIR_LEFT, DIR_RIGHT];
pub const DIR_SELF: &[i32] = &[DIR_DOWN];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BonusTarget {
    Kind(CardKind),
    DefeatTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MultiResource {
    AttackStrength,
    VP,
    Coin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardEffect {
    Good {
        goods: Vec<Cost<Good>>,
    },
    VP {
        vp: i32,
    },
    Military {
        strength: i32,
    },
    Science {
        fields: Vec<Field>,
    },
    Bonus {
        target_kinds: Vec<BonusTarget>,
        directions: Vec<i32>,
        vp: i32,
        coins: i32,
    },
    Trade {
        directions: Vec<i32>,
        goods: Vec<Good>,
    },
    Tavern,
    Multi {
        resources: Cost<MultiResource>,
    },
    FreeBuild {
        has_built: bool,
    },
    DrawDiscard {
        vp: i32,
    },
    MimicGuild,
    PlayFinalCard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub name: String,
    pub kind: CardKind,
    pub cost: Cost<Good>,
    pub free_with: Vec<String>,
    pub makes_free: Vec<String>,
    pub effect: CardEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct City {
    pub name: String,
    pub initial_resource: Good,
    pub wonder_stages: Vec<String>,
}

pub fn raw_goods() -> Vec<Good> {
    vec![Good::Wood, Good::Stone, Good::Ore, Good::Clay]
}

pub fn manufactured_goods() -> Vec<Good> {
    vec![Good::Papyrus, Good::Textile, Good::Glass]
}

pub fn all_fields() -> Vec<Field> {
    vec![Field::Mathematics, Field::Engineering, Field::Theology]
}

fn cost(entries: &[(Good, i32)]) -> Cost<Good> {
    Cost(entries.iter().cloned().collect())
}

fn multi_cost(entries: &[(MultiResource, i32)]) -> Cost<MultiResource> {
    Cost(entries.iter().cloned().collect())
}

fn card(
    name: &str,
    kind: CardKind,
    c: Cost<Good>,
    free_with: &[&str],
    makes_free: &[&str],
    effect: CardEffect,
) -> Card {
    Card {
        name: name.to_string(),
        kind,
        cost: c,
        free_with: free_with.iter().map(|s| s.to_string()).collect(),
        makes_free: makes_free.iter().map(|s| s.to_string()).collect(),
        effect,
    }
}

pub fn card_db() -> HashMap<String, Card> {
    let cards = vec![
        // Age 1 Raw
        card(
            "Lumber Yard",
            CardKind::Raw,
            cost(&[]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Wood, 1)])],
            },
        ),
        card(
            "Stone Pit",
            CardKind::Raw,
            cost(&[]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Stone, 1)])],
            },
        ),
        card(
            "Clay Pool",
            CardKind::Raw,
            cost(&[]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Clay, 1)])],
            },
        ),
        card(
            "Ore Vein",
            CardKind::Raw,
            cost(&[]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Ore, 1)])],
            },
        ),
        card(
            "Tree Farm",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Wood, 1)]), cost(&[(Good::Clay, 1)])],
            },
        ),
        card(
            "Excavation",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Stone, 1)]), cost(&[(Good::Clay, 1)])],
            },
        ),
        card(
            "Clay Pit",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Clay, 1)]), cost(&[(Good::Ore, 1)])],
            },
        ),
        card(
            "Timber Yard",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Stone, 1)]), cost(&[(Good::Wood, 1)])],
            },
        ),
        card(
            "Forest Cave",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Wood, 1)]), cost(&[(Good::Ore, 1)])],
            },
        ),
        card(
            "Mine",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Stone, 1)]), cost(&[(Good::Ore, 1)])],
            },
        ),
        // Age 1 Manufactured
        card(
            "Loom",
            CardKind::Manufactured,
            cost(&[]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Textile, 1)])],
            },
        ),
        card(
            "Glassworks",
            CardKind::Manufactured,
            cost(&[]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Glass, 1)])],
            },
        ),
        card(
            "Press",
            CardKind::Manufactured,
            cost(&[]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Papyrus, 1)])],
            },
        ),
        // Age 1 Civilian
        card(
            "Pawnshop",
            CardKind::Civilian,
            cost(&[]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Baths",
            CardKind::Civilian,
            cost(&[(Good::Stone, 1)]),
            &[],
            &["Aqueduct"],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Altar",
            CardKind::Civilian,
            cost(&[]),
            &[],
            &["Temple"],
            CardEffect::VP { vp: 2 },
        ),
        card(
            "Theater",
            CardKind::Civilian,
            cost(&[]),
            &[],
            &["Statue"],
            CardEffect::VP { vp: 2 },
        ),
        // Age 1 Commercial
        card(
            "Tavern",
            CardKind::Commercial,
            cost(&[]),
            &[],
            &[],
            CardEffect::Tavern,
        ),
        card(
            "East Trading Post",
            CardKind::Commercial,
            cost(&[]),
            &[],
            &["Forum"],
            CardEffect::Trade {
                directions: vec![DIR_RIGHT],
                goods: raw_goods(),
            },
        ),
        card(
            "West Trading Post",
            CardKind::Commercial,
            cost(&[]),
            &[],
            &["Forum"],
            CardEffect::Trade {
                directions: vec![DIR_LEFT],
                goods: raw_goods(),
            },
        ),
        card(
            "Marketplace",
            CardKind::Commercial,
            cost(&[]),
            &[],
            &["Caravansery"],
            CardEffect::Trade {
                directions: vec![DIR_LEFT, DIR_RIGHT],
                goods: manufactured_goods(),
            },
        ),
        // Age 1 Military
        card(
            "Stockade",
            CardKind::Military,
            cost(&[(Good::Wood, 1)]),
            &[],
            &[],
            CardEffect::Military { strength: 1 },
        ),
        card(
            "Barracks",
            CardKind::Military,
            cost(&[(Good::Ore, 1)]),
            &[],
            &[],
            CardEffect::Military { strength: 1 },
        ),
        card(
            "Guard Tower",
            CardKind::Military,
            cost(&[(Good::Clay, 1)]),
            &[],
            &[],
            CardEffect::Military { strength: 1 },
        ),
        // Age 1 Science
        card(
            "Apothecary",
            CardKind::Scientific,
            cost(&[(Good::Textile, 1)]),
            &[],
            &["Stables", "Dispensary"],
            CardEffect::Science {
                fields: vec![Field::Mathematics],
            },
        ),
        card(
            "Workshop",
            CardKind::Scientific,
            cost(&[(Good::Glass, 1)]),
            &[],
            &["Laboratory", "Archery Range"],
            CardEffect::Science {
                fields: vec![Field::Engineering],
            },
        ),
        card(
            "Scriptorium",
            CardKind::Scientific,
            cost(&[(Good::Papyrus, 1)]),
            &[],
            &["Courthouse", "Library"],
            CardEffect::Science {
                fields: vec![Field::Theology],
            },
        ),
        // Age 2 Raw
        card(
            "Sawmill",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Wood, 2)])],
            },
        ),
        card(
            "Quarry",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Stone, 2)])],
            },
        ),
        card(
            "Brickyard",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Clay, 2)])],
            },
        ),
        card(
            "Foundry",
            CardKind::Raw,
            cost(&[(Good::Coin, 1)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![cost(&[(Good::Ore, 2)])],
            },
        ),
        // Age 2 Civilian
        card(
            "Aqueduct",
            CardKind::Civilian,
            cost(&[(Good::Stone, 3)]),
            &["Baths"],
            &[],
            CardEffect::VP { vp: 5 },
        ),
        card(
            "Temple",
            CardKind::Civilian,
            cost(&[(Good::Wood, 1), (Good::Clay, 1), (Good::Glass, 1)]),
            &["Altar"],
            &["Pantheon"],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Statue",
            CardKind::Civilian,
            cost(&[(Good::Ore, 2), (Good::Wood, 1)]),
            &["Theater"],
            &["Gardens"],
            CardEffect::VP { vp: 4 },
        ),
        card(
            "Courthouse",
            CardKind::Civilian,
            cost(&[(Good::Clay, 2), (Good::Textile, 1)]),
            &["Scriptorium"],
            &[],
            CardEffect::VP { vp: 4 },
        ),
        // Age 2 Commercial
        card(
            "Forum",
            CardKind::Commercial,
            cost(&[(Good::Clay, 2)]),
            &["East Trading Post", "West Trading Post"],
            &["Haven"],
            CardEffect::Good {
                goods: vec![
                    cost(&[(Good::Papyrus, 1)]),
                    cost(&[(Good::Textile, 1)]),
                    cost(&[(Good::Glass, 1)]),
                ],
            },
        ),
        card(
            "Caravansery",
            CardKind::Commercial,
            cost(&[(Good::Wood, 2)]),
            &["Marketplace"],
            &["Lighthouse"],
            CardEffect::Good {
                goods: vec![
                    cost(&[(Good::Wood, 1)]),
                    cost(&[(Good::Stone, 1)]),
                    cost(&[(Good::Ore, 1)]),
                    cost(&[(Good::Clay, 1)]),
                ],
            },
        ),
        card(
            "Vineyard",
            CardKind::Commercial,
            cost(&[]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Raw)],
                directions: DIR_ALL.to_vec(),
                vp: 0,
                coins: 1,
            },
        ),
        card(
            "Bazar",
            CardKind::Commercial,
            cost(&[]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Manufactured)],
                directions: DIR_ALL.to_vec(),
                vp: 0,
                coins: 2,
            },
        ),
        // Age 2 Military
        card(
            "Walls",
            CardKind::Military,
            cost(&[(Good::Stone, 3)]),
            &[],
            &["Fortifications"],
            CardEffect::Military { strength: 2 },
        ),
        card(
            "Training Ground",
            CardKind::Military,
            cost(&[(Good::Ore, 2), (Good::Wood, 1)]),
            &[],
            &["Circus"],
            CardEffect::Military { strength: 2 },
        ),
        card(
            "Stables",
            CardKind::Military,
            cost(&[(Good::Clay, 1), (Good::Wood, 1), (Good::Ore, 1)]),
            &["Apothecary"],
            &[],
            CardEffect::Military { strength: 2 },
        ),
        card(
            "Archery Range",
            CardKind::Military,
            cost(&[(Good::Wood, 2), (Good::Ore, 1)]),
            &["Workshop"],
            &[],
            CardEffect::Military { strength: 2 },
        ),
        // Age 2 Science
        card(
            "Dispensary",
            CardKind::Scientific,
            cost(&[(Good::Ore, 2), (Good::Glass, 1)]),
            &["Apothecary"],
            &["Lodge", "Arena"],
            CardEffect::Science {
                fields: vec![Field::Mathematics],
            },
        ),
        card(
            "Laboratory",
            CardKind::Scientific,
            cost(&[(Good::Clay, 2), (Good::Papyrus, 1)]),
            &["Workshop"],
            &["Observatory", "Siege Workshop"],
            CardEffect::Science {
                fields: vec![Field::Engineering],
            },
        ),
        card(
            "Library",
            CardKind::Scientific,
            cost(&[(Good::Stone, 2), (Good::Textile, 1)]),
            &["Scriptorium"],
            &["Senate", "University"],
            CardEffect::Science {
                fields: vec![Field::Theology],
            },
        ),
        card(
            "School",
            CardKind::Scientific,
            cost(&[(Good::Wood, 1), (Good::Papyrus, 1)]),
            &[],
            &["Academy", "Study"],
            CardEffect::Science {
                fields: vec![Field::Theology],
            },
        ),
        // Age 3 Civilian
        card(
            "Pantheon",
            CardKind::Civilian,
            cost(&[
                (Good::Clay, 2),
                (Good::Ore, 1),
                (Good::Glass, 1),
                (Good::Papyrus, 1),
                (Good::Textile, 1),
            ]),
            &["Temple"],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        card(
            "Gardens",
            CardKind::Civilian,
            cost(&[(Good::Clay, 2), (Good::Wood, 1)]),
            &["Statue"],
            &[],
            CardEffect::VP { vp: 5 },
        ),
        card(
            "Town Hall",
            CardKind::Civilian,
            cost(&[(Good::Stone, 2), (Good::Ore, 1), (Good::Glass, 1)]),
            &[],
            &[],
            CardEffect::VP { vp: 6 },
        ),
        card(
            "Palace",
            CardKind::Civilian,
            cost(&[
                (Good::Stone, 1),
                (Good::Ore, 1),
                (Good::Wood, 1),
                (Good::Clay, 1),
                (Good::Glass, 1),
                (Good::Papyrus, 1),
                (Good::Textile, 1),
            ]),
            &[],
            &[],
            CardEffect::VP { vp: 8 },
        ),
        card(
            "Senate",
            CardKind::Civilian,
            cost(&[(Good::Wood, 2), (Good::Stone, 1), (Good::Ore, 1)]),
            &["Library"],
            &[],
            CardEffect::VP { vp: 6 },
        ),
        // Age 3 Commercial
        card(
            "Haven",
            CardKind::Commercial,
            cost(&[(Good::Wood, 1), (Good::Ore, 1), (Good::Textile, 1)]),
            &["Forum"],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Raw)],
                directions: DIR_SELF.to_vec(),
                vp: 1,
                coins: 1,
            },
        ),
        card(
            "Lighthouse",
            CardKind::Commercial,
            cost(&[(Good::Stone, 1), (Good::Glass, 1)]),
            &["Caravansery"],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Commercial)],
                directions: DIR_SELF.to_vec(),
                vp: 1,
                coins: 1,
            },
        ),
        card(
            "Chamber of Commerce",
            CardKind::Commercial,
            cost(&[(Good::Clay, 2), (Good::Papyrus, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Manufactured)],
                directions: DIR_SELF.to_vec(),
                vp: 2,
                coins: 2,
            },
        ),
        card(
            "Arena",
            CardKind::Commercial,
            cost(&[(Good::Stone, 2), (Good::Ore, 1)]),
            &["Dispensary"],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Wonder)],
                directions: DIR_SELF.to_vec(),
                vp: 1,
                coins: 3,
            },
        ),
        // Age 3 Military
        card(
            "Fortifications",
            CardKind::Military,
            cost(&[(Good::Ore, 3), (Good::Stone, 1)]),
            &["Walls"],
            &[],
            CardEffect::Military { strength: 3 },
        ),
        card(
            "Circus",
            CardKind::Military,
            cost(&[(Good::Stone, 3), (Good::Ore, 1)]),
            &["Training Ground"],
            &[],
            CardEffect::Military { strength: 3 },
        ),
        card(
            "Arsenal",
            CardKind::Military,
            cost(&[(Good::Wood, 2), (Good::Ore, 1), (Good::Textile, 1)]),
            &[],
            &[],
            CardEffect::Military { strength: 3 },
        ),
        card(
            "Siege Workshop",
            CardKind::Military,
            cost(&[(Good::Clay, 3), (Good::Wood, 1)]),
            &["Laboratory"],
            &[],
            CardEffect::Military { strength: 3 },
        ),
        // Age 3 Science
        card(
            "Lodge",
            CardKind::Scientific,
            cost(&[(Good::Clay, 2), (Good::Papyrus, 1), (Good::Textile, 1)]),
            &["Dispensary"],
            &[],
            CardEffect::Science {
                fields: vec![Field::Mathematics],
            },
        ),
        card(
            "Observatory",
            CardKind::Scientific,
            cost(&[(Good::Ore, 2), (Good::Glass, 1), (Good::Textile, 1)]),
            &["Laboratory"],
            &[],
            CardEffect::Science {
                fields: vec![Field::Engineering],
            },
        ),
        card(
            "University",
            CardKind::Scientific,
            cost(&[(Good::Wood, 2), (Good::Papyrus, 1), (Good::Glass, 1)]),
            &["Library"],
            &[],
            CardEffect::Science {
                fields: vec![Field::Theology],
            },
        ),
        card(
            "Academy",
            CardKind::Scientific,
            cost(&[(Good::Stone, 3), (Good::Glass, 1)]),
            &["School"],
            &[],
            CardEffect::Science {
                fields: vec![Field::Mathematics],
            },
        ),
        card(
            "Study",
            CardKind::Scientific,
            cost(&[(Good::Wood, 1), (Good::Papyrus, 1), (Good::Textile, 1)]),
            &["School"],
            &[],
            CardEffect::Science {
                fields: vec![Field::Engineering],
            },
        ),
        // Guilds
        card(
            "Workers Guild",
            CardKind::Guild,
            cost(&[
                (Good::Ore, 2),
                (Good::Clay, 1),
                (Good::Stone, 1),
                (Good::Wood, 1),
            ]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Raw)],
                directions: DIR_NEIGHBOURS.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        card(
            "Craftsmens Guild",
            CardKind::Guild,
            cost(&[(Good::Ore, 2), (Good::Stone, 2)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Manufactured)],
                directions: DIR_NEIGHBOURS.to_vec(),
                vp: 2,
                coins: 0,
            },
        ),
        card(
            "Traders Guild",
            CardKind::Guild,
            cost(&[(Good::Glass, 1), (Good::Textile, 1), (Good::Papyrus, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Commercial)],
                directions: DIR_NEIGHBOURS.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        card(
            "Philosophers Guild",
            CardKind::Guild,
            cost(&[(Good::Clay, 3), (Good::Papyrus, 1), (Good::Textile, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Scientific)],
                directions: DIR_NEIGHBOURS.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        card(
            "Spies Guild",
            CardKind::Guild,
            cost(&[(Good::Clay, 3), (Good::Glass, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Military)],
                directions: DIR_NEIGHBOURS.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        card(
            "Strategists Guild",
            CardKind::Guild,
            cost(&[(Good::Ore, 2), (Good::Stone, 1), (Good::Textile, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::DefeatTokens],
                directions: DIR_NEIGHBOURS.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        card(
            "Shipowners Guild",
            CardKind::Guild,
            cost(&[(Good::Wood, 3), (Good::Glass, 1), (Good::Papyrus, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![
                    BonusTarget::Kind(CardKind::Raw),
                    BonusTarget::Kind(CardKind::Manufactured),
                    BonusTarget::Kind(CardKind::Guild),
                ],
                directions: DIR_SELF.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        card(
            "Scientists Guild",
            CardKind::Guild,
            cost(&[(Good::Wood, 2), (Good::Ore, 2), (Good::Papyrus, 1)]),
            &[],
            &[],
            CardEffect::Science {
                fields: all_fields(),
            },
        ),
        card(
            "Magistrates Guild",
            CardKind::Guild,
            cost(&[(Good::Wood, 3), (Good::Stone, 1), (Good::Textile, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Civilian)],
                directions: DIR_NEIGHBOURS.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        card(
            "Builders Guild",
            CardKind::Guild,
            cost(&[(Good::Stone, 2), (Good::Clay, 2), (Good::Glass, 1)]),
            &[],
            &[],
            CardEffect::Bonus {
                target_kinds: vec![BonusTarget::Kind(CardKind::Wonder)],
                directions: DIR_ALL.to_vec(),
                vp: 1,
                coins: 0,
            },
        ),
        // Rhodes A Wonder Stages
        card(
            "Rhodes A Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Rhodes A Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Clay, 3)]),
            &[],
            &[],
            CardEffect::Military { strength: 2 },
        ),
        card(
            "Rhodes A Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Ore, 4)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Rhodes B Wonder Stages
        card(
            "Rhodes B Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Stone, 3)]),
            &[],
            &[],
            CardEffect::Multi {
                resources: multi_cost(&[
                    (MultiResource::AttackStrength, 1),
                    (MultiResource::VP, 3),
                    (MultiResource::Coin, 3),
                ]),
            },
        ),
        card(
            "Rhodes B Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Ore, 4)]),
            &[],
            &[],
            CardEffect::Multi {
                resources: multi_cost(&[
                    (MultiResource::AttackStrength, 1),
                    (MultiResource::VP, 4),
                    (MultiResource::Coin, 4),
                ]),
            },
        ),
        // Alexandria A Wonder Stages
        card(
            "Alexandria A Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Stone, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Alexandria A Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Ore, 2)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![
                    cost(&[(Good::Clay, 1)]),
                    cost(&[(Good::Ore, 1)]),
                    cost(&[(Good::Wood, 1)]),
                    cost(&[(Good::Stone, 1)]),
                ],
            },
        ),
        card(
            "Alexandria A Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Glass, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Alexandria B Wonder Stages
        card(
            "Alexandria B Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Clay, 2)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![
                    cost(&[(Good::Clay, 1)]),
                    cost(&[(Good::Ore, 1)]),
                    cost(&[(Good::Wood, 1)]),
                    cost(&[(Good::Stone, 1)]),
                ],
            },
        ),
        card(
            "Alexandria B Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::Good {
                goods: vec![
                    cost(&[(Good::Glass, 1)]),
                    cost(&[(Good::Textile, 1)]),
                    cost(&[(Good::Papyrus, 1)]),
                ],
            },
        ),
        card(
            "Alexandria B Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Stone, 3)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Ephesus A Wonder Stages
        card(
            "Ephesus A Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Stone, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Ephesus A Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::Multi {
                resources: multi_cost(&[(MultiResource::Coin, 9)]),
            },
        ),
        card(
            "Ephesus A Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Papyrus, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Ephesus B Wonder Stages
        card(
            "Ephesus B Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Stone, 2)]),
            &[],
            &[],
            CardEffect::Multi {
                resources: multi_cost(&[(MultiResource::VP, 2), (MultiResource::Coin, 4)]),
            },
        ),
        card(
            "Ephesus B Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::Multi {
                resources: multi_cost(&[(MultiResource::VP, 3), (MultiResource::Coin, 4)]),
            },
        ),
        card(
            "Ephesus B Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Papyrus, 1), (Good::Textile, 1), (Good::Glass, 1)]),
            &[],
            &[],
            CardEffect::Multi {
                resources: multi_cost(&[(MultiResource::VP, 5), (MultiResource::Coin, 4)]),
            },
        ),
        // Babylon A Wonder Stages
        card(
            "Babylon A Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Clay, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Babylon A Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Wood, 3)]),
            &[],
            &[],
            CardEffect::Science {
                fields: all_fields(),
            },
        ),
        card(
            "Babylon A Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Clay, 4)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Babylon B Wonder Stages
        card(
            "Babylon B Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Textile, 1), (Good::Clay, 1)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Babylon B Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Glass, 1), (Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::PlayFinalCard,
        ),
        card(
            "Babylon B Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Papyrus, 1), (Good::Clay, 3)]),
            &[],
            &[],
            CardEffect::Science {
                fields: all_fields(),
            },
        ),
        // Olympia A Wonder Stages
        card(
            "Olympia A Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Olympia A Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Stone, 2)]),
            &[],
            &[],
            CardEffect::FreeBuild { has_built: false },
        ),
        card(
            "Olympia A Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Ore, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Olympia B Wonder Stages
        card(
            "Olympia B Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::Trade {
                directions: vec![DIR_LEFT, DIR_RIGHT],
                goods: raw_goods(),
            },
        ),
        card(
            "Olympia B Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Stone, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 5 },
        ),
        card(
            "Olympia B Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Textile, 1), (Good::Ore, 2)]),
            &[],
            &[],
            CardEffect::MimicGuild,
        ),
        // Halicarnassus A Wonder Stages
        card(
            "Halicarnassus A Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Clay, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Halicarnassus A Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Ore, 3)]),
            &[],
            &[],
            CardEffect::DrawDiscard { vp: 0 },
        ),
        card(
            "Halicarnassus A Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Textile, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Halicarnassus B Wonder Stages
        card(
            "Halicarnassus B Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Ore, 2)]),
            &[],
            &[],
            CardEffect::DrawDiscard { vp: 2 },
        ),
        card(
            "Halicarnassus B Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Clay, 3)]),
            &[],
            &[],
            CardEffect::DrawDiscard { vp: 1 },
        ),
        card(
            "Halicarnassus B Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Glass, 1), (Good::Papyrus, 1), (Good::Textile, 1)]),
            &[],
            &[],
            CardEffect::DrawDiscard { vp: 0 },
        ),
        // Giza A Wonder Stages
        card(
            "Giza A Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Stone, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Giza A Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Wood, 3)]),
            &[],
            &[],
            CardEffect::VP { vp: 5 },
        ),
        card(
            "Giza A Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Stone, 4)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
        // Giza B Wonder Stages
        card(
            "Giza B Wonder Stage 1",
            CardKind::Wonder,
            cost(&[(Good::Wood, 2)]),
            &[],
            &[],
            CardEffect::VP { vp: 3 },
        ),
        card(
            "Giza B Wonder Stage 2",
            CardKind::Wonder,
            cost(&[(Good::Stone, 3)]),
            &[],
            &[],
            CardEffect::VP { vp: 5 },
        ),
        card(
            "Giza B Wonder Stage 3",
            CardKind::Wonder,
            cost(&[(Good::Clay, 3)]),
            &[],
            &[],
            CardEffect::VP { vp: 5 },
        ),
        card(
            "Giza B Wonder Stage 4",
            CardKind::Wonder,
            cost(&[(Good::Papyrus, 1), (Good::Stone, 4)]),
            &[],
            &[],
            CardEffect::VP { vp: 7 },
        ),
    ];

    cards.into_iter().map(|c| (c.name.clone(), c)).collect()
}

pub fn cities() -> Vec<City> {
    vec![
        City {
            name: "Rhodes A".to_string(),
            initial_resource: Good::Ore,
            wonder_stages: vec![
                "Rhodes A Wonder Stage 1".to_string(),
                "Rhodes A Wonder Stage 2".to_string(),
                "Rhodes A Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Rhodes B".to_string(),
            initial_resource: Good::Ore,
            wonder_stages: vec![
                "Rhodes B Wonder Stage 1".to_string(),
                "Rhodes B Wonder Stage 2".to_string(),
            ],
        },
        City {
            name: "Alexandria A".to_string(),
            initial_resource: Good::Glass,
            wonder_stages: vec![
                "Alexandria A Wonder Stage 1".to_string(),
                "Alexandria A Wonder Stage 2".to_string(),
                "Alexandria A Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Alexandria B".to_string(),
            initial_resource: Good::Glass,
            wonder_stages: vec![
                "Alexandria B Wonder Stage 1".to_string(),
                "Alexandria B Wonder Stage 2".to_string(),
                "Alexandria B Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Ephesus A".to_string(),
            initial_resource: Good::Papyrus,
            wonder_stages: vec![
                "Ephesus A Wonder Stage 1".to_string(),
                "Ephesus A Wonder Stage 2".to_string(),
                "Ephesus A Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Ephesus B".to_string(),
            initial_resource: Good::Papyrus,
            wonder_stages: vec![
                "Ephesus B Wonder Stage 1".to_string(),
                "Ephesus B Wonder Stage 2".to_string(),
                "Ephesus B Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Babylon A".to_string(),
            initial_resource: Good::Clay,
            wonder_stages: vec![
                "Babylon A Wonder Stage 1".to_string(),
                "Babylon A Wonder Stage 2".to_string(),
                "Babylon A Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Babylon B".to_string(),
            initial_resource: Good::Clay,
            wonder_stages: vec![
                "Babylon B Wonder Stage 1".to_string(),
                "Babylon B Wonder Stage 2".to_string(),
                "Babylon B Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Olympia A".to_string(),
            initial_resource: Good::Wood,
            wonder_stages: vec![
                "Olympia A Wonder Stage 1".to_string(),
                "Olympia A Wonder Stage 2".to_string(),
                "Olympia A Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Olympia B".to_string(),
            initial_resource: Good::Wood,
            wonder_stages: vec![
                "Olympia B Wonder Stage 1".to_string(),
                "Olympia B Wonder Stage 2".to_string(),
                "Olympia B Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Halicarnassus A".to_string(),
            initial_resource: Good::Textile,
            wonder_stages: vec![
                "Halicarnassus A Wonder Stage 1".to_string(),
                "Halicarnassus A Wonder Stage 2".to_string(),
                "Halicarnassus A Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Halicarnassus B".to_string(),
            initial_resource: Good::Textile,
            wonder_stages: vec![
                "Halicarnassus B Wonder Stage 1".to_string(),
                "Halicarnassus B Wonder Stage 2".to_string(),
                "Halicarnassus B Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Giza A".to_string(),
            initial_resource: Good::Stone,
            wonder_stages: vec![
                "Giza A Wonder Stage 1".to_string(),
                "Giza A Wonder Stage 2".to_string(),
                "Giza A Wonder Stage 3".to_string(),
            ],
        },
        City {
            name: "Giza B".to_string(),
            initial_resource: Good::Stone,
            wonder_stages: vec![
                "Giza B Wonder Stage 1".to_string(),
                "Giza B Wonder Stage 2".to_string(),
                "Giza B Wonder Stage 3".to_string(),
                "Giza B Wonder Stage 4".to_string(),
            ],
        },
    ]
}

struct CardForPlayers {
    name: &'static str,
    players: &'static [usize],
}

const AGE1_CARDS: &[CardForPlayers] = &[
    CardForPlayers {
        name: "Lumber Yard",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Stone Pit",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Clay Pool",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Ore Vein",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Tree Farm",
        players: &[6],
    },
    CardForPlayers {
        name: "Excavation",
        players: &[4],
    },
    CardForPlayers {
        name: "Clay Pit",
        players: &[3],
    },
    CardForPlayers {
        name: "Timber Yard",
        players: &[3],
    },
    CardForPlayers {
        name: "Forest Cave",
        players: &[5],
    },
    CardForPlayers {
        name: "Mine",
        players: &[6],
    },
    CardForPlayers {
        name: "Loom",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Glassworks",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Press",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Pawnshop",
        players: &[4, 7],
    },
    CardForPlayers {
        name: "Baths",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Altar",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Theater",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Tavern",
        players: &[4, 5, 7],
    },
    CardForPlayers {
        name: "East Trading Post",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "West Trading Post",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Marketplace",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Stockade",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Barracks",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Guard Tower",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Apothecary",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Workshop",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Scriptorium",
        players: &[3, 4],
    },
];

const AGE2_CARDS: &[CardForPlayers] = &[
    CardForPlayers {
        name: "Sawmill",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Quarry",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Brickyard",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Foundry",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Loom",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Glassworks",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Press",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Aqueduct",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Temple",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Statue",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Courthouse",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Forum",
        players: &[3, 6, 7],
    },
    CardForPlayers {
        name: "Caravansery",
        players: &[3, 5, 6],
    },
    CardForPlayers {
        name: "Vineyard",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Bazar",
        players: &[4, 7],
    },
    CardForPlayers {
        name: "Walls",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Training Ground",
        players: &[4, 6, 7],
    },
    CardForPlayers {
        name: "Stables",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Archery Range",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Dispensary",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Laboratory",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Library",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "School",
        players: &[3, 7],
    },
];

const AGE3_CARDS: &[CardForPlayers] = &[
    CardForPlayers {
        name: "Pantheon",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Gardens",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Town Hall",
        players: &[3, 5, 6],
    },
    CardForPlayers {
        name: "Palace",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Senate",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Haven",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Lighthouse",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Chamber of Commerce",
        players: &[4, 6],
    },
    CardForPlayers {
        name: "Arena",
        players: &[3, 5, 7],
    },
    CardForPlayers {
        name: "Fortifications",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Circus",
        players: &[4, 5, 6],
    },
    CardForPlayers {
        name: "Arsenal",
        players: &[3, 4, 7],
    },
    CardForPlayers {
        name: "Siege Workshop",
        players: &[3, 5],
    },
    CardForPlayers {
        name: "Lodge",
        players: &[3, 6],
    },
    CardForPlayers {
        name: "Observatory",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "University",
        players: &[3, 4],
    },
    CardForPlayers {
        name: "Academy",
        players: &[3, 7],
    },
    CardForPlayers {
        name: "Study",
        players: &[3, 5],
    },
];

const GUILD_CARDS: &[&str] = &[
    "Workers Guild",
    "Craftsmens Guild",
    "Traders Guild",
    "Philosophers Guild",
    "Spies Guild",
    "Strategists Guild",
    "Shipowners Guild",
    "Scientists Guild",
    "Magistrates Guild",
    "Builders Guild",
];

fn deck_for_players(
    db: &HashMap<String, Card>,
    cards: &[CardForPlayers],
    players: usize,
) -> Vec<Card> {
    let mut deck = Vec::new();
    for c in cards {
        for &p in c.players {
            if p <= players {
                deck.push(db[c.name].clone());
            }
        }
    }
    deck
}

pub fn deck_age1(players: usize) -> Vec<Card> {
    let db = card_db();
    deck_for_players(&db, AGE1_CARDS, players)
}

pub fn deck_age2(players: usize) -> Vec<Card> {
    let db = card_db();
    deck_for_players(&db, AGE2_CARDS, players)
}

pub fn deck_age3(players: usize, rng: &mut impl rand::Rng) -> Vec<Card> {
    let db = card_db();
    let mut deck = deck_for_players(&db, AGE3_CARDS, players);
    let mut guilds = deck_guild();
    use rand::seq::SliceRandom;
    guilds.shuffle(rng);
    guilds.truncate(players + 2);
    deck.extend(guilds);
    deck
}

pub fn deck_guild() -> Vec<Card> {
    let db = card_db();
    GUILD_CARDS.iter().map(|name| db[*name].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use brdgme_game::rng::GameRng;

    #[test]
    fn deck_age1_sizes() {
        for p in 3..=7 {
            assert_eq!(deck_age1(p).len(), p * 7, "age 1 with {p} players");
        }
    }

    #[test]
    fn deck_age2_sizes() {
        for p in 3..=7 {
            assert_eq!(deck_age2(p).len(), p * 7, "age 2 with {p} players");
        }
    }

    #[test]
    fn deck_age3_sizes() {
        let mut rng = GameRng::seed_from_u64(42);
        for p in 3..=7 {
            assert_eq!(
                deck_age3(p, &mut rng).len(),
                p * 7,
                "age 3 with {p} players"
            );
        }
    }
}
