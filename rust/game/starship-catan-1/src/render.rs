use std::collections::BTreeMap;

use brdgme_color::NamedColor;
use brdgme_game::Renderer;
use brdgme_markup::ast::Cell;
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};
use serde::{Deserialize, Serialize};

use crate::card::{
    AdventureCard, Module, Resource, SectorCard, adventure_planet_string, render_resource,
};
use crate::{Phase, PlayerBoard};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PubState {
    /// Current game phase (ChooseModule, Produce, ChooseSector, Flight, or TradeAndBuild).
    pub phase: Phase,
    /// Index (0 or 1) of the player whose turn it is.
    pub current_player: usize,
    /// The sector (1-4) currently being explored during flight.
    pub current_sector: i32,
    /// Full board state for both players (resources, modules, colonies, trading posts, defeated pirates, etc.).
    pub player_boards: [PlayerBoard; 2],
    /// Sector cards encountered during the current flight, in order.
    pub flight_cards: Vec<SectorCard>,
    /// Number of trades made at the current flight card's trade stop.
    pub trade_amount: i32,
    /// Number of "take" trades used this TradeAndBuild phase (via Trade module).
    pub player_trade_amount: i32,
    /// The production dice roll for this turn (1-3).
    pub yellow_dice: i32,
    /// Number of flight actions used so far this flight.
    pub flight_actions_used: usize,
    /// True when the current flight card has been fully resolved.
    pub card_finished: bool,
    /// True when the player must choose a module to lose (after losing a pirate fight).
    pub losing_module: bool,
    /// Adventure cards currently available to complete.
    pub current_adventure_cards: Vec<AdventureCard>,
    /// Number of adventure cards remaining in the deck.
    pub adventure_deck_len: usize,
    /// Number of cards remaining in each sector pile (keyed by sector 1-4).
    pub sector_pile_lens: BTreeMap<i32, usize>,
    /// Number of cards in the sector draw pile (used for replacements).
    pub sector_draw_pile_len: usize,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player (0 or 1) this private state belongs to.
    pub player: usize,
    /// Cards the player is peeking at via the Sensor module (private to this player).
    pub peeking: Vec<SectorCard>,
}

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        render(self, None, None)
    }
}

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        render(&self.public, Some(self.player), Some(&self.peeking))
    }
}

fn render_module_level(level: i32) -> Vec<N> {
    match level {
        0 => vec![N::Fg(NamedColor::Grey.into(), vec![N::text("0")])],
        2 => vec![N::Bold(vec![N::text("2")])],
        _ => vec![N::text(level.to_string())],
    }
}

fn resource_row(
    r: Resource,
    viewer: usize,
    opponent: usize,
    boards: &[PlayerBoard; 2],
) -> Vec<Cell> {
    vec![
        (A::Left, render_resource(r)),
        (
            A::Left,
            vec![N::Bold(vec![N::text(boards[viewer].res(r).to_string())])],
        ),
        (
            A::Left,
            vec![N::Fg(
                NamedColor::Grey.into(),
                vec![N::text(boards[opponent].res(r).to_string())],
            )],
        ),
    ]
}

fn double_row(mut left: Vec<Cell>, right: Vec<Cell>) -> Row {
    left.push((A::Left, vec![N::text("")]));
    left.extend(right);
    left
}

fn render(pub_state: &PubState, player: Option<usize>, _peeking: Option<&[SectorCard]>) -> Vec<N> {
    let viewer = player.unwrap_or(0);
    let opponent = (viewer + 1) % 2;
    let boards = &pub_state.player_boards;
    let current = pub_state.current_player;
    let remaining_moves = pub_state.yellow_dice + boards[current].res(Resource::Booster)
        - pub_state.flight_cards.len() as i32;
    let remaining_actions = boards[current].actions() - pub_state.flight_actions_used as i32;
    let remaining_trades = 2 - pub_state.trade_amount;
    let remaining_player_trades =
        boards[current].module(Module::Trade) - pub_state.player_trade_amount;

    let mut out: Vec<N> = vec![];

    // Current turn
    let mut turn_rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Current turn:")])]),
        (A::Left, vec![N::Player(viewer)]),
    ]];
    match pub_state.phase {
        Phase::ChooseSector => {
            if !boards[viewer].last_sectors.is_empty() {
                let sectors = &boards[viewer].last_sectors;
                let mut nodes = vec![N::Bold(vec![N::text(sectors[0].to_string())])];
                for s in &sectors[1..] {
                    nodes.push(N::text(format!(" {}", s)));
                }
                turn_rows.push(vec![
                    (A::Left, vec![N::Bold(vec![N::text("Last sectors")])]),
                    (A::Left, nodes),
                ]);
            }
        }
        Phase::Flight => {
            if let Some(card) = pub_state.flight_cards.last() {
                turn_rows.push(vec![
                    (A::Left, vec![N::Bold(vec![N::text("Current planet:")])]),
                    (A::Left, card.string()),
                ]);
                turn_rows.push(vec![
                    (A::Left, vec![N::Bold(vec![N::text("Current sector:")])]),
                    (A::Left, vec![N::text(pub_state.current_sector.to_string())]),
                ]);
                turn_rows.push(vec![
                    (A::Left, vec![N::Bold(vec![N::text("Moves left:")])]),
                    (A::Left, vec![N::text(remaining_moves.to_string())]),
                ]);
                turn_rows.push(vec![
                    (A::Left, vec![N::Bold(vec![N::text("Actions left:")])]),
                    (A::Left, vec![N::text(remaining_actions.to_string())]),
                ]);
            }
        }
        Phase::TradeAndBuild => {
            turn_rows.push(vec![
                (
                    A::Left,
                    vec![N::Bold(vec![N::text("Post trades remaining:")])],
                ),
                (A::Left, vec![N::text(remaining_trades.to_string())]),
            ]);
            turn_rows.push(vec![
                (
                    A::Left,
                    vec![N::Bold(vec![N::text("Player trades remaining:")])],
                ),
                (A::Left, vec![N::text(remaining_player_trades.to_string())]),
            ]);
        }
        _ => {}
    }
    out.push(table_with_gap(&turn_rows, 2));
    out.push(N::text("\n\n"));

    // Resources
    let bold_name = |p: usize| vec![N::Bold(vec![N::Player(p)])];
    let mut resource_rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Resource")])]),
        (A::Left, bold_name(viewer)),
        (A::Left, bold_name(opponent)),
        (A::Left, vec![N::text(" ")]),
        (A::Left, vec![N::Bold(vec![N::text("Resource")])]),
        (A::Left, bold_name(viewer)),
        (A::Left, bold_name(opponent)),
    ]];
    resource_rows.push(double_row(
        resource_row(Resource::Food, viewer, opponent, boards),
        resource_row(Resource::ColonyShip, viewer, opponent, boards),
    ));
    resource_rows.push(double_row(
        resource_row(Resource::Fuel, viewer, opponent, boards),
        resource_row(Resource::TradeShip, viewer, opponent, boards),
    ));
    resource_rows.push(double_row(
        resource_row(Resource::Carbon, viewer, opponent, boards),
        resource_row(Resource::Booster, viewer, opponent, boards),
    ));
    resource_rows.push(double_row(
        resource_row(Resource::Ore, viewer, opponent, boards),
        resource_row(Resource::Cannon, viewer, opponent, boards),
    ));
    resource_rows.push(resource_row(Resource::Trade, viewer, opponent, boards));
    resource_rows.push(double_row(
        resource_row(Resource::Science, viewer, opponent, boards),
        vec![
            (
                A::Left,
                vec![N::Fg(
                    NamedColor::Red.into(),
                    vec![N::Bold(vec![N::text("medals")])],
                )],
            ),
            (
                A::Left,
                vec![N::Bold(vec![N::text(boards[viewer].medals().to_string())])],
            ),
            (
                A::Left,
                vec![N::text(boards[opponent].medals().to_string())],
            ),
        ],
    ));
    resource_rows.push(double_row(
        vec![
            (A::Left, vec![N::text("")]),
            (A::Left, vec![N::text("")]),
            (A::Left, vec![N::text("")]),
        ],
        vec![
            (
                A::Left,
                vec![N::Fg(
                    NamedColor::Green.into(),
                    vec![N::Bold(vec![N::text("diplomacy")])],
                )],
            ),
            (
                A::Left,
                vec![N::Bold(vec![N::text(
                    boards[viewer].diplomat_points().to_string(),
                )])],
            ),
            (
                A::Left,
                vec![N::text(boards[opponent].diplomat_points().to_string())],
            ),
        ],
    ));
    resource_rows.push(double_row(
        resource_row(Resource::Astro, viewer, opponent, boards),
        vec![
            (
                A::Left,
                vec![N::Fg(
                    NamedColor::Blue.into(),
                    vec![N::Bold(vec![N::text("VP")])],
                )],
            ),
            (
                A::Left,
                vec![N::Bold(vec![N::text(
                    boards[viewer].victory_points().to_string(),
                )])],
            ),
            (
                A::Left,
                vec![N::text(boards[opponent].victory_points().to_string())],
            ),
        ],
    ));
    out.push(table_with_gap(&resource_rows, 2));
    out.push(N::text("\n\n"));

    // Adventure cards
    out.push(N::Bold(vec![N::text("Adventure cards")]));
    out.push(N::text("\n"));
    let mut adventure_rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("#")])]),
        (A::Left, vec![N::Bold(vec![N::text("Planet")])]),
        (A::Left, vec![N::Bold(vec![N::text("Description")])]),
    ]];
    for (i, ac) in pub_state.current_adventure_cards.iter().enumerate() {
        adventure_rows.push(vec![
            (A::Left, vec![N::text((i + 1).to_string())]),
            (A::Left, adventure_planet_string(ac.planet())),
            (
                A::Left,
                vec![N::Fg(NamedColor::Grey.into(), vec![N::text(ac.text())])],
            ),
        ]);
    }
    out.push(table_with_gap(&adventure_rows, 2));
    out.push(N::text("\n\n"));

    // Modules
    let mut module_rows: Vec<Row> = vec![vec![
        (A::Left, vec![N::Bold(vec![N::text("Module")])]),
        (A::Left, vec![N::Player(viewer)]),
        (A::Left, vec![N::Player(opponent)]),
        (A::Left, vec![N::Bold(vec![N::text("Description")])]),
    ]];
    for m in Module::ALL {
        module_rows.push(vec![
            (A::Left, vec![N::text(m.name())]),
            (A::Left, render_module_level(boards[viewer].module(m))),
            (A::Left, render_module_level(boards[opponent].module(m))),
            (
                A::Left,
                vec![N::Fg(NamedColor::Grey.into(), vec![N::text(m.summary())])],
            ),
        ]);
    }
    out.push(table_with_gap(&module_rows, 2));
    out.push(N::text("\n"));
    out.push(N::Bold(vec![N::text("Upgrade cost: L1")]));
    out.push(N::text(" ("));
    out.extend(Module::transaction(1).lose_string());
    out.push(N::text("), "));
    out.push(N::Bold(vec![N::text("L2")]));
    out.push(N::text(" ("));
    out.extend(Module::transaction(2).lose_string());
    out.push(N::text(")"));

    // Cards
    for p in [viewer, opponent] {
        out.push(N::text("\n\n"));
        out.push(N::Player(p));
        out.push(N::text(" "));
        out.push(N::Bold(vec![N::text("cards")]));
        for c in &boards[p].colonies {
            out.push(N::text("\n"));
            out.extend(c.string());
        }
        for c in &boards[p].trading_posts {
            out.push(N::text("\n"));
            out.extend(c.string());
        }
    }

    out
}
