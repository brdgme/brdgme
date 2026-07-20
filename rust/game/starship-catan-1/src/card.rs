use std::fmt;

use brdgme_color::NamedColor;
use brdgme_game::rng::GameRng;
use brdgme_markup::Node as N;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Resource {
    Any,
    Food,
    Fuel,
    Carbon,
    Ore,
    Science,
    Trade,
    Astro,
    ColonyShip,
    TradeShip,
    Booster,
    Cannon,
}

impl Resource {
    pub const GOODS: [Resource; 5] = [
        Resource::Food,
        Resource::Fuel,
        Resource::Carbon,
        Resource::Ore,
        Resource::Trade,
    ];

    pub const BUILDABLES: [Resource; 4] = [
        Resource::TradeShip,
        Resource::ColonyShip,
        Resource::Booster,
        Resource::Cannon,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Resource::Any => "any resource",
            Resource::Food => "food",
            Resource::Fuel => "fuel",
            Resource::Carbon => "carbon",
            Resource::Ore => "ore",
            Resource::Science => "science",
            Resource::Trade => "trade",
            Resource::Astro => "astro",
            Resource::ColonyShip => "colony ship",
            Resource::TradeShip => "trade ship",
            Resource::Booster => "booster",
            Resource::Cannon => "cannon",
        }
    }

    pub fn color(self) -> NamedColor {
        match self {
            Resource::Any => NamedColor::Green,
            Resource::Food => NamedColor::Red,
            Resource::Fuel => NamedColor::Grey,
            Resource::Carbon => NamedColor::Cyan,
            Resource::Ore => NamedColor::Foreground,
            Resource::Science => NamedColor::Purple,
            Resource::Trade => NamedColor::Yellow,
            Resource::Astro => NamedColor::Green,
            Resource::ColonyShip => NamedColor::Cyan,
            Resource::TradeShip => NamedColor::Yellow,
            Resource::Booster => NamedColor::Red,
            Resource::Cannon => NamedColor::Blue,
        }
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Module {
    Logistics,
    Command,
    Sensor,
    Trade,
    Science,
    Production,
}

impl Module {
    pub const ALL: [Module; 6] = [
        Module::Logistics,
        Module::Command,
        Module::Sensor,
        Module::Trade,
        Module::Science,
        Module::Production,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Module::Logistics => "logistics",
            Module::Command => "command",
            Module::Sensor => "sensor",
            Module::Trade => "trade",
            Module::Science => "science",
            Module::Production => "production",
        }
    }

    pub fn summary(self) -> String {
        match self {
            Module::Logistics => "store extra goods (2, 3, 4)".to_string(),
            Module::Command => "take extra actions (2, 3, 4)".to_string(),
            Module::Sensor => "peek at sector cards (0, 2, 3)".to_string(),
            Module::Trade => "buy resources from opponent for $2 (0, 1, 2)".to_string(),
            Module::Science => "produce science (0, 1, 2)".to_string(),
            Module::Production => "produce trade (0, 1, 2)".to_string(),
        }
    }

    pub fn description(self, player: usize, level: i32) -> String {
        match self {
            Module::Logistics => {
                format!("Store up to {} resources in each resource bay", 2 + level)
            }
            Module::Command => {
                format!("Take up to {} actions during your flight phase", 2 + level)
            }
            Module::Sensor => format!(
                "Look at the first {} sector cards of a flight, put each card on the bottom or top of the stack in any order",
                1 + level
            ),
            Module::Trade => format!(
                "Buy {} resource(s) of your choice from your opponent for 2 Astro each",
                level
            ),
            Module::Science => format!(
                "Produce a science point on a roll of a {}",
                join_dice(&Module::science_module_dice(level, player))
            ),
            Module::Production => format!(
                "Produce a trade good on a roll of a {}",
                join_dice(&Module::trade_module_dice(level, player))
            ),
        }
    }

    pub fn science_module_dice(level: i32, player: usize) -> Vec<i32> {
        match level {
            0 => vec![],
            1 => vec![3 - player as i32],
            _ => vec![2, 3],
        }
    }

    pub fn trade_module_dice(level: i32, player: usize) -> Vec<i32> {
        match level {
            0 => vec![],
            1 => vec![2 + player as i32],
            _ => vec![2, 3],
        }
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

fn join_dice(dice: &[i32]) -> String {
    dice.iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(" or ")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TradeDir {
    Both,
    Buy,
    Sell,
}

impl TradeDir {
    pub fn sign(self) -> i32 {
        match self {
            TradeDir::Both => 0,
            TradeDir::Buy => 1,
            TradeDir::Sell => -1,
        }
    }

    pub fn string(self) -> &'static str {
        match self {
            TradeDir::Both => "buy/sell",
            TradeDir::Buy => "buy",
            TradeDir::Sell => "sell",
        }
    }
}

pub fn amount_trade_dir(amount: i32) -> TradeDir {
    if amount == 0 {
        TradeDir::Both
    } else if amount > 0 {
        TradeDir::Buy
    } else {
        TradeDir::Sell
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AdventurePlanet {
    Hades,
    Pallas,
    Picasso,
    Poseidon,
}

impl AdventurePlanet {
    pub fn name(self) -> &'static str {
        match self {
            AdventurePlanet::Hades => "Hades",
            AdventurePlanet::Pallas => "Pallas",
            AdventurePlanet::Picasso => "Picasso",
            AdventurePlanet::Poseidon => "Poseidon",
        }
    }

    pub fn color(self) -> NamedColor {
        match self {
            AdventurePlanet::Hades => NamedColor::Red,
            AdventurePlanet::Pallas => NamedColor::Yellow,
            AdventurePlanet::Picasso => NamedColor::Purple,
            AdventurePlanet::Poseidon => NamedColor::Cyan,
        }
    }
}

pub fn adventure_planet_string(planet: AdventurePlanet) -> Vec<N> {
    vec![N::Fg(
        planet.color().into(),
        vec![N::Bold(vec![N::text(planet.name())])],
    )]
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SectorCard {
    Colony {
        name: String,
        resource: Resource,
        dice: i32,
        start_card: bool,
    },
    Trade {
        name: String,
        resources: Vec<Resource>,
        price: i32,
        maximum: i32,
        direction: TradeDir,
        trading_post: bool,
    },
    Pirate {
        strength: i32,
        ransom: i32,
        destroy_cannon: bool,
        destroy_module: bool,
    },
    Median,
    Empty,
    AdventurePlanet {
        planet: AdventurePlanet,
    },
}

impl SectorCard {
    pub fn string(&self) -> Vec<N> {
        match self {
            SectorCard::Colony {
                name,
                resource,
                dice,
                ..
            } => {
                let mut n = vec![
                    N::Fg(
                        NamedColor::Green.into(),
                        vec![N::Bold(vec![N::text(name.clone())])],
                    ),
                    N::text(" (colony planet, roll "),
                    N::Bold(vec![N::text(dice.to_string())]),
                    N::text(" for "),
                ];
                n.extend(render_resource(*resource));
                n.push(N::text(")"));
                n
            }
            SectorCard::Trade {
                name,
                resources,
                price,
                maximum,
                direction,
                trading_post,
            } => {
                let mut part: Vec<N> = vec![N::text(direction.string())];
                if *maximum > 0 {
                    part.push(N::text(" up to "));
                    part.push(N::Bold(vec![N::text(maximum.to_string())]));
                }
                part.push(N::text(" "));
                part.push(N::Bold(render_resources(resources)));
                part.push(N::text(" for "));
                part.extend(render_money(*price));
                part.push(N::text(" each"));
                if *trading_post {
                    part.push(N::text(", trading post"));
                }
                let mut n = vec![
                    N::Fg(
                        NamedColor::Yellow.into(),
                        vec![N::Bold(vec![N::text(name.clone())])],
                    ),
                    N::text(" ("),
                ];
                n.extend(part);
                n.push(N::text(")"));
                n
            }
            SectorCard::Pirate { ransom, .. } => {
                let mut n = vec![
                    N::Fg(
                        NamedColor::Grey.into(),
                        vec![N::Bold(vec![N::text("pirate ship")])],
                    ),
                    N::text(", asking a ransom of "),
                ];
                n.extend(render_money(*ransom));
                n
            }
            SectorCard::Median => vec![
                N::Fg(
                    NamedColor::Red.into(),
                    vec![N::Bold(vec![N::text("Median")])],
                ),
                N::text(" (2 diplomat points)"),
            ],
            SectorCard::Empty => vec![
                N::Bold(vec![N::Fg(
                    NamedColor::Grey.into(),
                    vec![N::text("Lost Planet")],
                )]),
                N::text(" (empty space)"),
            ],
            SectorCard::AdventurePlanet { planet } => adventure_planet_string(*planet),
        }
    }

    pub fn full_string(&self) -> Vec<N> {
        match self {
            SectorCard::Pirate {
                strength,
                destroy_cannon,
                destroy_module,
                ..
            } => {
                let mut n = self.string();
                n.push(N::text(" (strength "));
                n.push(N::Bold(vec![N::text(strength.to_string())]));
                if *destroy_cannon {
                    n.push(N::text(", destroys cannon"));
                }
                if *destroy_module {
                    n.push(N::text(", destroys module"));
                }
                n.push(N::text(")"));
                n
            }
            _ => self.string(),
        }
    }

    pub fn victory_points(&self) -> i32 {
        match self {
            SectorCard::Colony { .. } => 1,
            _ => 0,
        }
    }

    pub fn medals(&self) -> i32 {
        match self {
            SectorCard::Pirate { .. } => 1,
            _ => 0,
        }
    }

    pub fn diplomat_points(&self) -> i32 {
        match self {
            SectorCard::Trade { trading_post, .. } => {
                if *trading_post {
                    1
                } else {
                    0
                }
            }
            SectorCard::Median => 2,
            _ => 0,
        }
    }

    pub fn can_found_trading_post(&self) -> bool {
        match self {
            SectorCard::Trade { trading_post, .. } => *trading_post,
            SectorCard::Median => true,
            _ => false,
        }
    }

    pub fn requires_action(&self) -> bool {
        matches!(self, SectorCard::Pirate { .. })
    }
}

pub fn render_money(amount: i32) -> Vec<N> {
    vec![N::Bold(vec![N::Fg(
        NamedColor::Green.into(),
        vec![N::text(format!("${}", amount))],
    )])]
}

pub fn render_resource(resource: Resource) -> Vec<N> {
    vec![N::Bold(vec![N::Fg(
        resource.color().into(),
        vec![N::text(resource.name())],
    )])]
}

pub fn render_resource_amount(resource: Resource, amount: i32) -> Vec<N> {
    if resource == Resource::Astro {
        render_money(amount)
    } else {
        let mut n = vec![N::text(format!("{} ", amount))];
        n.extend(render_resource(resource));
        n
    }
}

pub fn render_resources(resources: &[Resource]) -> Vec<N> {
    let mut n = vec![];
    for (i, r) in resources.iter().enumerate() {
        if i > 0 {
            n.push(N::text(", "));
        }
        n.extend(render_resource(*r));
    }
    n
}

fn colony(name: &str, resource: Resource, dice: i32) -> SectorCard {
    SectorCard::Colony {
        name: name.to_string(),
        resource,
        dice,
        start_card: false,
    }
}

fn trade(name: &str, resources: Vec<Resource>, price: i32) -> SectorCard {
    SectorCard::Trade {
        name: name.to_string(),
        resources,
        price,
        maximum: 0,
        direction: TradeDir::Both,
        trading_post: false,
    }
}

fn trading_post(
    name: &str,
    resources: Vec<Resource>,
    price: i32,
    direction: TradeDir,
    maximum: i32,
    trading_post: bool,
) -> SectorCard {
    SectorCard::Trade {
        name: name.to_string(),
        resources,
        price,
        maximum,
        direction,
        trading_post,
    }
}

fn pirate(strength: i32, ransom: i32, destroy_cannon: bool, destroy_module: bool) -> SectorCard {
    SectorCard::Pirate {
        strength,
        ransom,
        destroy_cannon,
        destroy_module,
    }
}

pub fn starting_cards() -> Vec<SectorCard> {
    vec![
        colony("Alioth VIII", Resource::Carbon, 1),
        colony("Megrez VII", Resource::Fuel, 1),
    ]
}

pub fn sector_base_cards() -> Vec<SectorCard> {
    let goods = Resource::GOODS.to_vec();
    vec![
        SectorCard::AdventurePlanet {
            planet: AdventurePlanet::Hades,
        },
        SectorCard::AdventurePlanet {
            planet: AdventurePlanet::Pallas,
        },
        SectorCard::AdventurePlanet {
            planet: AdventurePlanet::Picasso,
        },
        SectorCard::AdventurePlanet {
            planet: AdventurePlanet::Poseidon,
        },
        colony("Dubhe IV", Resource::Carbon, 2),
        colony("Phekda VI", Resource::Food, 1),
        colony("Merak V", Resource::Food, 3),
        colony("Alkor III", Resource::Fuel, 3),
        colony("Bellatrix I", Resource::Ore, 1),
        colony("Heka II", Resource::Ore, 2),
        pirate(2, 3, false, false),
        pirate(3, 3, false, false),
        trading_post(
            "Alnitak IX",
            vec![Resource::Trade],
            3,
            TradeDir::Both,
            0,
            true,
        ),
        trading_post(
            "Beteigeuze VI",
            vec![Resource::Carbon],
            3,
            TradeDir::Both,
            0,
            true,
        ),
        trading_post("Aigel X", vec![Resource::Ore], 3, TradeDir::Both, 0, true),
        trading_post(
            "Mintaka II",
            vec![Resource::Fuel],
            3,
            TradeDir::Both,
            0,
            true,
        ),
        trading_post("Saiph VI", vec![Resource::Food], 3, TradeDir::Both, 0, true),
        trade("Corendium VII", vec![Resource::Carbon], 1),
        trade("Tostoku I", vec![Resource::Carbon], 2),
        trade("Marsitis VI", vec![Resource::Carbon], 4),
        trade("Quartzee X", vec![Resource::Carbon], 5),
        trade("Planctoinis VII", vec![Resource::Food], 1),
        trade("Sputsallia IV", vec![Resource::Food], 2),
        trade("Pobeckifiked VI", vec![Resource::Food], 4),
        trade("Califasperum V", vec![Resource::Food], 5),
        trade("Litigus IX", vec![Resource::Fuel], 1),
        trade("Gonsarium II", vec![Resource::Fuel], 2),
        trade("Brocollar II", vec![Resource::Fuel], 4),
        trade("Phlatiarum V", vec![Resource::Fuel], 5),
        trade("Ireoni VII", vec![Resource::Ore], 1),
        trade("Cupperius IV", vec![Resource::Ore], 2),
        trade("Leedsi X", vec![Resource::Ore], 4),
        trade("Bazaltide IV", vec![Resource::Ore], 5),
        trade("Martkwal VIII", vec![Resource::Trade], 1),
        trade("Beowulf's Bane", vec![Resource::Trade], 2),
        trade("Parapeckis VII", vec![Resource::Trade], 4),
        trade("Martiin - Tempest II", vec![Resource::Trade], 5),
        trade("Kopernikus II", vec![Resource::Science], 3),
        trading_post(
            "Diplomat Outpost",
            goods.clone(),
            3,
            TradeDir::Buy,
            1,
            false,
        ),
        trading_post("Diplomat Outpost", goods, 3, TradeDir::Buy, 1, false),
    ]
}

pub fn sector1_cards() -> Vec<SectorCard> {
    let goods = Resource::GOODS.to_vec();
    vec![
        SectorCard::Empty,
        SectorCard::Empty,
        trading_post(
            "Green Folk Outpost",
            vec![Resource::Science],
            4,
            TradeDir::Sell,
            1,
            true,
        ),
        trading_post("Diplomat Outpost", goods, 3, TradeDir::Buy, 1, true),
        pirate(2, 3, false, false),
        pirate(3, 3, false, false),
        pirate(4, 3, false, false),
    ]
}

pub fn sector2_cards() -> Vec<SectorCard> {
    vec![
        SectorCard::Empty,
        SectorCard::Empty,
        colony("Benet-Nash IX", Resource::Carbon, 3),
        colony("Mizar X", Resource::Food, 2),
        trading_post(
            "Scientist Outpost",
            vec![Resource::Science],
            2,
            TradeDir::Buy,
            1,
            true,
        ),
        pirate(4, 5, true, false),
        pirate(5, 5, false, true),
    ]
}

pub fn sector3_cards() -> Vec<SectorCard> {
    let goods = Resource::GOODS.to_vec();
    vec![
        SectorCard::Empty,
        SectorCard::Empty,
        colony("Enif I", Resource::Fuel, 2),
        colony("Theta Pegasi II", Resource::Ore, 3),
        trading_post("Merchant Outpost", goods, 3, TradeDir::Sell, 2, false),
        pirate(5, 5, false, true),
        pirate(6, 5, false, true),
    ]
}

pub fn sector4_cards() -> Vec<SectorCard> {
    vec![
        SectorCard::Empty,
        SectorCard::Empty,
        SectorCard::Empty,
        SectorCard::Empty,
        SectorCard::Median,
        pirate(6, 5, false, true),
        pirate(7, 5, false, true),
    ]
}

pub fn shuffled_sector_cards(rng: &mut GameRng) -> Vec<SectorCard> {
    let mut deck = sector4_cards();
    deck.shuffle(rng);
    let mut s3 = sector3_cards();
    s3.shuffle(rng);
    deck.extend(s3);
    let mut s2 = sector2_cards();
    s2.shuffle(rng);
    deck.extend(s2);
    let mut s1 = sector1_cards();
    s1.shuffle(rng);
    deck.extend(s1);
    let mut base = sector_base_cards();
    base.shuffle(rng);
    deck.extend(base);
    deck
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AdventureCard {
    EnvironmentalCrisis,
    DiplomaticGift,
    MerchantGift,
    Famine,
    WholesaleOrder1,
    PirateNest,
    CouncilMeeting,
    Epidemic,
    Emergency,
    Reconstruction,
    Monument,
    WholesaleOrder2,
}

impl AdventureCard {
    pub fn planet(self) -> AdventurePlanet {
        match self {
            AdventureCard::EnvironmentalCrisis => AdventurePlanet::Poseidon,
            AdventureCard::DiplomaticGift => AdventurePlanet::Picasso,
            AdventureCard::MerchantGift => AdventurePlanet::Pallas,
            AdventureCard::Famine => AdventurePlanet::Picasso,
            AdventureCard::WholesaleOrder1 => AdventurePlanet::Pallas,
            AdventureCard::PirateNest => AdventurePlanet::Hades,
            AdventureCard::CouncilMeeting => AdventurePlanet::Poseidon,
            AdventureCard::Epidemic => AdventurePlanet::Hades,
            AdventureCard::Emergency => AdventurePlanet::Picasso,
            AdventureCard::Reconstruction => AdventurePlanet::Hades,
            AdventureCard::Monument => AdventurePlanet::Pallas,
            AdventureCard::WholesaleOrder2 => AdventurePlanet::Poseidon,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            AdventureCard::EnvironmentalCrisis => {
                "In Poseidon there are environmental problems.  Donate 1 science point and gain 3 astro and 1 resource of your choice."
            }
            AdventureCard::DiplomaticGift => {
                "Greetings, Catanian!  A diplomatic gift is waiting on the planet of Picasso for you.  Gain 1 resource of your choice."
            }
            AdventureCard::MerchantGift => {
                "Greetings, Catanian!  A merchant gift is waiting on the planet of Pallas for you.  Gain 1 resource of your choice."
            }
            AdventureCard::Famine => {
                "Famine on Picasso!  Donate 1 food and gain a medal and 1 resource of your choice."
            }
            AdventureCard::WholesaleOrder1 => {
                "Pallas urgently requires merchandise.  Donate 1 trade good and gain a medal and 1 resource of your choice."
            }
            AdventureCard::PirateNest => {
                "Pirates have taken root in Hades.  Reach Hades with 4 boosters and gain a medal and 1 resource of your choice."
            }
            AdventureCard::CouncilMeeting => {
                "The Galactic Council urgently requires 6 Astro to organise the meeting of the council.  Donate 6 Astro and gain a medal and 2 resources of your choice."
            }
            AdventureCard::Epidemic => {
                "A mystery plague has broken out on Hades.  Donate 2 science points and gain a victory point."
            }
            AdventureCard::Emergency => {
                "A spaceship near Picasso is in a gravitational trap.  Whoever reaches picasso with 4 boosters can set them free and gain a medal and 1 resource of your choice."
            }
            AdventureCard::Reconstruction => {
                "We have freed Hades from pirates and the population urgently requires reconstruction aid.  Donate 10 Astro and gain 2 medals."
            }
            AdventureCard::Monument => {
                "The Pallas population wants to build a monument for the merchants.  Donate 2 ore and 1 carbon and gain a victory point."
            }
            AdventureCard::WholesaleOrder2 => {
                "This time Poseidon urgently requires merchandise.  Donate 2 trade goods and gain a medal and 2 resources of your choice."
            }
        }
    }

    pub fn medals(self) -> i32 {
        match self {
            AdventureCard::Famine => 1,
            AdventureCard::WholesaleOrder1 => 1,
            AdventureCard::PirateNest => 1,
            AdventureCard::CouncilMeeting => 1,
            AdventureCard::Emergency => 1,
            AdventureCard::Reconstruction => 2,
            AdventureCard::WholesaleOrder2 => 1,
            _ => 0,
        }
    }

    pub fn victory_points(self) -> i32 {
        match self {
            AdventureCard::Epidemic => 1,
            AdventureCard::Monument => 1,
            _ => 0,
        }
    }
}

pub fn adventure1_cards() -> Vec<AdventureCard> {
    vec![
        AdventureCard::EnvironmentalCrisis,
        AdventureCard::DiplomaticGift,
        AdventureCard::MerchantGift,
    ]
}

pub fn adventure2_cards() -> Vec<AdventureCard> {
    vec![
        AdventureCard::Famine,
        AdventureCard::WholesaleOrder1,
        AdventureCard::PirateNest,
    ]
}

pub fn adventure3_cards() -> Vec<AdventureCard> {
    vec![
        AdventureCard::CouncilMeeting,
        AdventureCard::Epidemic,
        AdventureCard::Emergency,
    ]
}

pub fn adventure4_cards() -> Vec<AdventureCard> {
    vec![
        AdventureCard::Reconstruction,
        AdventureCard::Monument,
        AdventureCard::WholesaleOrder2,
    ]
}

pub fn shuffled_adventure_cards(rng: &mut GameRng) -> Vec<AdventureCard> {
    let mut deck = adventure4_cards();
    deck.shuffle(rng);
    let mut a3 = adventure3_cards();
    a3.shuffle(rng);
    deck.extend(a3);
    let mut a2 = adventure2_cards();
    a2.shuffle(rng);
    deck.extend(a2);
    let mut a1 = adventure1_cards();
    a1.shuffle(rng);
    deck.extend(a1);
    deck
}

pub fn current_adventure_cards(adventure_cards: &[AdventureCard]) -> Vec<AdventureCard> {
    let n = adventure_cards.len().min(3);
    adventure_cards[adventure_cards.len() - n..].to_vec()
}
