use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status, placings_log};
use brdgme_markup::Node as N;

use crate::card::{Card, GEMS, Noble, Resource, level_1_cards, level_2_cards, level_3_cards};
use crate::command::{Command, ParsedLoc};
use crate::cost::Cost;
use crate::player_board::PlayerBoard;

pub mod card;
pub mod command;
pub mod cost;
pub mod player_board;
pub mod render;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 4;
/// Ported from `game.go`'s `MaxGold`.
const MAX_GOLD: i32 = 5;
/// Ported from `game.go`'s `MaxTokens`.
const MAX_TOKENS: i32 = 10;

/// Ported from `game.go`'s `Phase` (`PhaseMain`/`PhaseVisit`/`PhaseDiscard`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    Main,
    Visit,
    Discard,
}

/// Ported from `game.go`'s `Game`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    /// `Decks[3][]Card` - remaining draw pile per level (0 = level 1).
    pub decks: Vec<Vec<Card>>,
    /// `Board[3][]Card` - face-up cards per level.
    pub board: Vec<Vec<Card>>,
    pub nobles: Vec<Noble>,
    pub tokens: Cost,
    pub player_boards: Vec<PlayerBoard>,
    pub current_player: usize,
    pub phase: Phase,
    pub end_triggered: bool,
    pub ended: bool,
    pub rng: GameRng,
}

/// Per-player public info shown by `render.go`'s Player table plus
/// board/reserve counts - never carries a non-viewing-player's reserve card
/// contents (the game's one piece of hidden information).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PubPlayer {
    /// Permanent gem bonuses from development cards this player owns, one per owned card of that gem. Discounts card costs and attracts nobles.
    pub bonuses: Cost,
    /// Gem and Gold tokens this player is currently holding.
    pub tokens: Cost,
    /// Nobles that have visited this player, each worth 3 prestige.
    pub nobles: Vec<Noble>,
    /// Number of development cards this player has bought.
    pub card_count: usize,
    /// Number of cards this player has reserved. The reserved cards themselves are hidden.
    pub reserve_count: usize,
    /// This player's current prestige score.
    pub prestige: i32,
}

/// Ported from what `render.go`'s `PubRender` (`pNum == -1`) actually shows:
/// deck contents/sizes are never rendered so are omitted; reserve card
/// contents are never shown for any player in the pub view.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players in the game, 2 through 4.
    pub players: usize,
    /// Face-up development cards available to buy or reserve, indexed by level (0 = level 1, 1 = level 2, 2 = level 3). Each level holds up to 4 cards; a slot disappears once its deck is empty.
    pub board: Vec<Vec<Card>>,
    /// Nobles currently available to visit, each worth 3 prestige and requiring a cost paid in permanent card bonuses.
    pub nobles: Vec<Noble>,
    /// The bank's remaining supply of each gem and Gold token.
    pub tokens: Cost,
    /// Public info for each player, indexed by player number (0-based).
    pub player_boards: Vec<PubPlayer>,
    /// Index (0-based) of the player whose turn it is.
    pub current_player: usize,
    /// Current turn phase: Main, Visit, or Discard.
    pub phase: Phase,
    /// True once the game has ended.
    pub finished: bool,
}

/// `{ public: PubState, player: usize, reserve: Vec<Card> }` per the port
/// plan - the viewing player's own full reserve, the only hidden info.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player (0-based) this private state belongs to.
    pub player: usize,
    /// This player's own reserved cards, up to 3. The only hidden information in the game; other players see only the count.
    pub reserve: Vec<Card>,
}

/// Ported from `game.go`'s `MaxGems`.
fn max_gems(players: usize) -> i32 {
    match players {
        2 => 4,
        3 => 5,
        _ => 7,
    }
}

/// Ported from `render.go`'s `ResourceColours`.
pub(crate) fn resource_color(r: Resource) -> brdgme_color::NamedColor {
    match r {
        Resource::Diamond => brdgme_color::NamedColor::Grey,
        Resource::Sapphire => brdgme_color::NamedColor::Blue,
        Resource::Emerald => brdgme_color::NamedColor::Green,
        Resource::Ruby => brdgme_color::NamedColor::Red,
        Resource::Onyx => brdgme_color::NamedColor::Foreground,
        Resource::Gold => brdgme_color::NamedColor::Yellow,
        Resource::Prestige => brdgme_color::NamedColor::Purple,
    }
}

fn resource_node(r: Resource) -> N {
    N::Fg(resource_color(r).into(), vec![N::text(r.name())])
}

fn bold_resource_node(r: Resource) -> N {
    N::Bold(vec![resource_node(r)])
}

/// Port of `RenderCard` (`render.go`), simplified to the card identity
/// (resource + prestige) used in logs; full table rendering is Task 3.
fn card_node(c: &Card) -> N {
    let mut parts = vec![resource_node(c.resource)];
    if c.prestige > 0 {
        parts.push(N::text(format!(" {}", c.prestige)));
    }
    N::Group(parts)
}

/// Port of `RenderNoble` (`render.go`), simplified as above.
fn noble_node(n: &Noble) -> N {
    N::Bold(vec![N::Fg(
        resource_color(Resource::Prestige).into(),
        vec![N::text(format!("{}", n.prestige))],
    )])
}

/// Joins nodes for each item with ", " and " and " before the last, mirroring
/// `brdgme.CommaList`.
fn comma_list_nodes(items: Vec<N>) -> Vec<N> {
    let len = items.len();
    let mut parts = vec![];
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            parts.push(N::text(if i == len - 1 { " and " } else { ", " }));
        }
        parts.push(item);
    }
    parts
}

impl Game {
    pub fn max_gems(&self) -> i32 {
        max_gems(self.players)
    }

    /// Ported from `game.go`'s `CheckEndTriggered`.
    fn check_end_triggered(&mut self) -> Vec<Log> {
        if self.end_triggered {
            return vec![];
        }
        for p in 0..self.players {
            if self.player_boards[p].prestige() >= 15 {
                self.end_triggered = true;
                return vec![Log::public(vec![N::Bold(vec![N::text(
                    "The end of the game has been triggered",
                )])])];
            }
        }
        vec![]
    }

    /// Ported from `game.go`'s `Placings`.
    fn placings(&self) -> Vec<usize> {
        gen_placings(
            &(0..self.players)
                .map(|p| {
                    vec![
                        self.player_boards[p].prestige(),
                        self.player_boards[p].cards.len() as i32,
                    ]
                })
                .collect::<Vec<Vec<i32>>>(),
        )
    }

    fn main_phase(&mut self) {
        self.phase = Phase::Main;
    }

    /// Ported from `game.go`'s `DiscardPhase`.
    fn discard_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Discard;
        if self.player_boards[self.current_player].tokens.sum() <= MAX_TOKENS {
            return self.next_phase();
        }
        vec![]
    }

    /// Ported from `game.go`'s `VisitPhase`.
    fn visit_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Visit;
        let pb_bonuses = self.player_boards[self.current_player].bonuses();
        let can_visit: Vec<usize> = (0..self.nobles.len())
            .filter(|&i| cost::can_afford(&pb_bonuses, &self.nobles[i].cost))
            .collect();
        match can_visit.len() {
            0 => self.next_phase(),
            1 => self
                .visit(self.current_player, can_visit[0])
                .expect("invariant: auto-visit must always succeed"),
            _ => vec![],
        }
    }

    /// Ported from `game.go`'s `NextPlayer`.
    fn next_player(&mut self) -> Vec<Log> {
        let logs = self.check_end_triggered();
        self.current_player = (self.current_player + 1) % self.players;
        if self.end_triggered && self.current_player == 0 {
            self.ended = true;
        } else {
            self.main_phase();
        }
        logs
    }

    /// Ported from `game.go`'s `NextPhase`.
    fn next_phase(&mut self) -> Vec<Log> {
        match self.phase {
            Phase::Main => self.visit_phase(),
            Phase::Visit => self.discard_phase(),
            Phase::Discard => self.next_player(),
        }
    }

    /// Ported from `game.go`'s `Pay`. Panics (invariant) if the player can't
    /// afford - callers must validate `can_afford` first, matching Go's
    /// defence-in-depth `CanAfford` check inside `Pay` (there, the returned
    /// error is discarded by every caller since it can never actually
    /// trigger given the prior check).
    fn pay(&mut self, player: usize, amount: &Cost) {
        assert!(
            self.player_boards[player].can_afford(amount),
            "invariant: pay called without a prior can_afford check"
        );
        let offset = self.player_boards[player].bonuses().sub(amount);
        for gem in GEMS {
            let off = offset.get(gem);
            if off < 0 {
                let new_pb_gem = self.player_boards[player].tokens.get(gem) + off;
                self.player_boards[player].tokens.set(gem, new_pb_gem);
                self.tokens.set(gem, self.tokens.get(gem) - off);
                if new_pb_gem < 0 {
                    let gold = self.player_boards[player].tokens.get(Resource::Gold) + new_pb_gem;
                    self.player_boards[player].tokens.set(Resource::Gold, gold);
                    self.tokens.set(gem, self.tokens.get(gem) + new_pb_gem);
                    self.tokens
                        .set(Resource::Gold, self.tokens.get(Resource::Gold) - new_pb_gem);
                    self.player_boards[player].tokens.set(gem, 0);
                }
            }
        }
    }

    /// Ported from `take_command.go`'s `CanTake`.
    pub fn can_take(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Main
    }

    /// Ported from `take_command.go`'s `Take`.
    pub fn take(&mut self, player: usize, tokens: &[Resource]) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if !self.can_take(player) {
            return Err(GameError::invalid_input("unable to take right now"));
        }
        let mut logs = vec![];
        match tokens.len() {
            2 => {
                if tokens[0] != tokens[1] {
                    return Err(GameError::invalid_input(
                        "must take the same type of tokens when taking two",
                    ));
                }
                if self.tokens.get(tokens[0]) < 4 {
                    return Err(GameError::invalid_input(
                        "can only take two when there are four or more remaining",
                    ));
                }
                logs.push(Log::public(vec![
                    N::Player(player),
                    N::text(" took "),
                    N::Bold(vec![N::text("2 "), resource_node(tokens[0])]),
                ]));
            }
            3 => {
                for i in 0..3 {
                    if tokens[i] == tokens[(i + 1) % 3] {
                        return Err(GameError::invalid_input(
                            "must take different tokens when taking three",
                        ));
                    }
                    if self.tokens.get(tokens[i]) == 0 {
                        return Err(GameError::invalid_input(
                            "there aren't enough tokens remaning to take that",
                        ));
                    }
                }
                let mut parts = vec![N::Player(player), N::text(" took ")];
                parts.extend(comma_list_nodes(
                    tokens.iter().map(|&t| bold_resource_node(t)).collect(),
                ));
                logs.push(Log::public(parts));
            }
            _ => {
                return Err(GameError::invalid_input(
                    "can only take two or three tokens",
                ));
            }
        }
        let amount = Cost::from_resources(tokens);
        self.player_boards[player].tokens = self.player_boards[player].tokens.add(&amount);
        self.tokens = self.tokens.sub(&amount);
        logs.extend(self.next_phase());
        Ok(logs)
    }

    /// Ported from `buy_command.go`'s `CanBuy`.
    pub fn can_buy(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Main
    }

    /// Ported from `buy_command.go`'s `Buy`.
    pub fn buy(&mut self, player: usize, loc: ParsedLoc) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if !self.can_buy(player) {
            return Err(GameError::invalid_input("unable to buy right now"));
        }
        let ParsedLoc { row, col } = loc;
        let mut logs = vec![];
        match row {
            0..=2 => {
                if col >= self.board[row].len() {
                    return Err(GameError::invalid_input("that is not a valid card"));
                }
                let c = self.board[row][col].clone();
                if !self.player_boards[player].can_afford(&c.cost) {
                    return Err(GameError::invalid_input("you can't afford that card"));
                }
                self.pay(player, &c.cost);
                self.player_boards[player].cards.push(c.clone());
                if !self.decks[row].is_empty() {
                    self.board[row][col] = self.decks[row].remove(0);
                } else {
                    self.board[row].remove(col);
                }
                logs.push(Log::public(vec![
                    N::Player(player),
                    N::text(" bought "),
                    card_node(&c),
                    N::text(" from the board"),
                ]));
            }
            3 => {
                if col >= self.player_boards[player].reserve.len() {
                    return Err(GameError::invalid_input("that is not a valid reserve card"));
                }
                let c = self.player_boards[player].reserve[col].clone();
                if !self.player_boards[player].can_afford(&c.cost) {
                    return Err(GameError::invalid_input("you can't afford that card"));
                }
                self.pay(player, &c.cost);
                self.player_boards[player].cards.push(c.clone());
                self.player_boards[player].reserve.remove(col);
                logs.push(Log::public(vec![
                    N::Player(player),
                    N::text(" bought "),
                    card_node(&c),
                    N::text(" from their reserve"),
                ]));
            }
            _ => return Err(GameError::invalid_input("that is not a valid row")),
        }
        logs.extend(self.next_phase());
        Ok(logs)
    }

    /// Ported from `reserve_command.go`'s `CanReserve`.
    pub fn can_reserve(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::Main
            && self.player_boards[player].reserve.len() < 3
    }

    /// Ported from `reserve_command.go`'s `Reserve`.
    pub fn reserve(&mut self, player: usize, loc: ParsedLoc) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if !self.can_reserve(player) {
            return Err(GameError::invalid_input("unable to reserve right now"));
        }
        let ParsedLoc { row, col } = loc;
        if row > 2 {
            return Err(GameError::invalid_input("that is not a valid row"));
        }
        if col >= self.board[row].len() {
            return Err(GameError::invalid_input("that is not a valid card"));
        }
        let c = self.board[row][col].clone();
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" reserved "),
            card_node(&c),
        ])];
        self.player_boards[player].reserve.push(c);
        if self.tokens.get(Resource::Gold) > 0 {
            let pb_gold = self.player_boards[player].tokens.get(Resource::Gold) + 1;
            self.player_boards[player]
                .tokens
                .set(Resource::Gold, pb_gold);
            self.tokens
                .set(Resource::Gold, self.tokens.get(Resource::Gold) - 1);
        }
        if !self.decks[row].is_empty() {
            self.board[row][col] = self.decks[row].remove(0);
        } else {
            self.board[row].remove(col);
        }
        logs.extend(self.next_phase());
        Ok(logs)
    }

    /// Ported from `discard_command.go`'s `CanDiscard`.
    pub fn can_discard(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Discard
    }

    /// Ported from `discard_command.go`'s `Discard`.
    pub fn discard(&mut self, player: usize, tokens: &[Resource]) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if !self.can_discard(player) {
            return Err(GameError::invalid_input("unable to discard right now"));
        }
        if tokens.is_empty() {
            return Err(GameError::invalid_input(
                "please specify at least one token",
            ));
        }
        let t_cost = Cost::from_resources(tokens);
        if !self.player_boards[player].tokens.can_afford(&t_cost) {
            return Err(GameError::invalid_input("you don't have that many tokens"));
        }
        self.player_boards[player].tokens = self.player_boards[player].tokens.sub(&t_cost);
        self.tokens = self.tokens.add(&t_cost);

        let mut parts = vec![N::Player(player), N::text(" discarded ")];
        parts.extend(comma_list_nodes(
            tokens.iter().map(|&t| bold_resource_node(t)).collect(),
        ));
        let mut logs = vec![Log::public(parts)];

        if self.player_boards[player].tokens.sum() <= MAX_TOKENS {
            logs.extend(self.next_phase());
        }
        Ok(logs)
    }

    /// Ported from `visit_command.go`'s `CanVisit`.
    pub fn can_visit(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Visit
    }

    /// Ported from `visit_command.go`'s `Visit`. Deliberately does not
    /// re-check affordability (quirk 1) - any valid noble index may be
    /// visited.
    pub fn visit(&mut self, player: usize, noble: usize) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if !self.can_visit(player) {
            return Err(GameError::invalid_input("unable to visit right now"));
        }
        if noble >= self.nobles.len() {
            return Err(GameError::invalid_input("that is not a valid noble number"));
        }
        let n = self.nobles[noble].clone();
        self.player_boards[player].nobles.push(n.clone());
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" was visited by "),
            noble_node(&n),
        ])];
        self.nobles.remove(noble);
        logs.extend(self.next_phase());
        Ok(logs)
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players) {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }
        let mut rng = GameRng::seed_from_u64(seed);

        let mut decks = vec![];
        let mut board = vec![];
        for mut cards in [level_1_cards(), level_2_cards(), level_3_cards()] {
            cards.shuffle(&mut rng);
            let rest = cards.split_off(4);
            board.push(cards);
            decks.push(rest);
        }

        let mut nobles = card::noble_cards();
        nobles.shuffle(&mut rng);
        nobles.truncate(players + 1);

        let mut tokens = Cost::new();
        tokens.set(Resource::Gold, MAX_GOLD);
        let gems = max_gems(players);
        for r in GEMS {
            tokens.set(r, gems);
        }

        let g = Game {
            players,
            decks,
            board,
            nobles,
            tokens,
            player_boards: vec![PlayerBoard::new(); players],
            current_player: 0,
            phase: Phase::Main,
            end_triggered: false,
            ended: false,
            rng,
        };
        Ok((g, vec![]))
    }

    fn status(&self) -> Status {
        if self.ended {
            Status::Finished {
                placings: self.placings(),
                stats: vec![Default::default(); self.players],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            board: self.board.clone(),
            nobles: self.nobles.clone(),
            tokens: self.tokens.clone(),
            player_boards: self
                .player_boards
                .iter()
                .map(|pb| PubPlayer {
                    bonuses: pb.bonuses(),
                    tokens: pb.tokens.clone(),
                    nobles: pb.nobles.clone(),
                    card_count: pb.cards.len(),
                    reserve_count: pb.reserve.len(),
                    prestige: pb.prestige(),
                })
                .collect(),
            current_player: self.current_player,
            phase: self.phase,
            finished: self.ended,
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            reserve: self.player_boards[player].reserve.clone(),
        }
    }

    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let output = match self.command_parser(player) {
            Some(cp) => cp,
            None => {
                return Err(GameError::invalid_input(
                    "not expecting any commands at the moment",
                ));
            }
        }
        .parse(input, players);
        match output {
            Ok(ParseOutput {
                remaining,
                value: Command::Buy(loc),
                ..
            }) => {
                let mut logs = self.buy(player, loc)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].prestige()))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Discard(tokens),
                ..
            }) => {
                let mut logs = self.discard(player, &tokens)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].prestige()))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Reserve(loc),
                ..
            }) => {
                let mut logs = self.reserve(player, loc)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].prestige()))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Take(tokens),
                ..
            }) => {
                let mut logs = self.take(player, &tokens)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].prestige()))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Visit(noble),
                ..
            }) => {
                let mut logs = self.visit(player, noble)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].prestige()))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Err(e) => Err(e),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..self.players)
            .map(|p| self.player_boards[p].prestige() as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..=MAX_PLAYERS).collect()
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn rules() -> String {
        include_str!("../RULES.md").to_string()
    }

    fn data_docs() -> String {
        include_str!("../DATA_DOCS.md").to_string()
    }

    fn basic_strategy() -> String {
        include_str!("../BASIC_STRATEGY.md").to_string()
    }

    fn advanced_strategy() -> String {
        include_str!("../ADVANCED_STRATEGY.md").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("player{}", i)).collect()
    }

    fn card_with_cost(resource: Resource, cost: &[(Resource, i32)]) -> Card {
        Card {
            resource,
            prestige: 0,
            cost: Cost(cost.iter().cloned().collect()),
        }
    }

    #[test]
    fn test_start_rejects_invalid_player_counts() {
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(5, 1).is_err());
        for n in 2..=4 {
            assert!(Game::start(n, 1).is_ok());
        }
    }

    #[test]
    fn test_start_board_and_decks() {
        let (g, _) = Game::start(3, 1).unwrap();
        assert_eq!(4, g.board[0].len());
        assert_eq!(4, g.board[1].len());
        assert_eq!(4, g.board[2].len());
        assert_eq!(36, g.decks[0].len());
        assert_eq!(26, g.decks[1].len());
        assert_eq!(16, g.decks[2].len());
        assert_eq!(4, g.nobles.len()); // players + 1
        assert_eq!(Phase::Main, g.phase);
        assert_eq!(0, g.current_player);
        for pb in &g.player_boards {
            assert_eq!(0, pb.cards.len());
            assert_eq!(0, pb.reserve.len());
        }
    }

    #[test]
    fn test_start_token_pools() {
        for (players, expected_gems) in [(2, 4), (3, 5), (4, 7)] {
            let (g, _) = Game::start(players, 1).unwrap();
            for r in GEMS {
                assert_eq!(expected_gems, g.tokens.get(r));
            }
            assert_eq!(MAX_GOLD, g.tokens.get(Resource::Gold));
            assert_eq!(players + 1, g.nobles.len());
        }
    }

    #[test]
    fn test_take_two_same() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let gem = GEMS[0];
        let bank_before = g.tokens.get(gem);
        assert!(bank_before >= 4);
        g.take(0, &[gem, gem]).unwrap();
        assert_eq!(2, g.player_boards[0].tokens.get(gem));
        assert_eq!(bank_before - 2, g.tokens.get(gem));
    }

    #[test]
    fn test_take_two_same_requires_bank_of_four() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let gem = GEMS[0];
        g.tokens.set(gem, 3);
        assert!(g.take(0, &[gem, gem]).is_err());
        g.tokens.set(gem, 2);
        assert!(g.take(0, &[gem, gem]).is_err());
    }

    #[test]
    fn test_take_two_different_rejected() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        assert!(g.take(0, &[GEMS[0], GEMS[1]]).is_err());
    }

    #[test]
    fn test_take_three_distinct() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.take(0, &[GEMS[0], GEMS[1], GEMS[2]]).unwrap();
        assert_eq!(1, g.player_boards[0].tokens.get(GEMS[0]));
        assert_eq!(1, g.player_boards[0].tokens.get(GEMS[1]));
        assert_eq!(1, g.player_boards[0].tokens.get(GEMS[2]));
    }

    #[test]
    fn test_take_three_with_repeat_rejected() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        assert!(g.take(0, &[GEMS[0], GEMS[1], GEMS[0]]).is_err());
    }

    #[test]
    fn test_take_invalid_counts_rejected() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        assert!(g.take(0, &[]).is_err());
        assert!(g.take(0, &[GEMS[0]]).is_err());
        assert!(g.take(0, &[GEMS[0], GEMS[1], GEMS[2], GEMS[3]]).is_err());
        assert!(
            g.take(0, &[GEMS[0], GEMS[1], GEMS[2], GEMS[3], GEMS[4]])
                .is_err()
        );
    }

    #[test]
    fn test_take_gem_with_zero_one_two_three_in_bank() {
        let (g, _) = Game::start(3, 1).unwrap();
        for n in 0..=3 {
            let mut g = g.clone();
            g.tokens.set(GEMS[2], n);
            let res = g.take(0, &[GEMS[0], GEMS[1], GEMS[2]]);
            if n == 0 {
                assert!(res.is_err());
            } else {
                assert!(res.is_ok());
            }
        }
    }

    #[test]
    fn test_take_gold_rejected_unparseable() {
        let mut g = Game::start(3, 1).unwrap().0;
        assert!(g.command(0, "take Gold", &players(3)).is_err());
    }

    #[test]
    fn test_take_command_spec_autocomplete() {
        // Regression: the trailing (still-being-typed) token must filter the
        // autocomplete suggestions, not be consumed as a completed token.
        let (g, _) = Game::start(3, 1).unwrap();
        let spec = g
            .command_spec(0)
            .expect("player 0 should have a command spec in a fresh game");
        let vals = |input: &str| -> Vec<String> {
            spec.suggest(input, &[])
                .iter()
                .map(|s| s.value.clone())
                .collect()
        };
        assert_eq!(vals("take dia"), vec!["Diamond"]);
        assert_eq!(vals("take dia sap em"), vec!["Emerald"]);
        assert!(vals("take dia sap emsa").is_empty());
    }

    #[test]
    fn test_take_parser_regression_cases() {
        let p = players(3);
        {
            let mut g = Game::start(3, 1).unwrap().0;
            g.command(0, "take Diamond Diamond", &p).unwrap();
        }
        {
            let mut g = Game::start(3, 1).unwrap().0;
            assert!(g.command(0, "take Diamond Sapphire", &p).is_err());
        }
        {
            let mut g = Game::start(3, 1).unwrap().0;
            g.command(0, "take Diamond Sapphire Ruby", &p).unwrap();
        }
    }

    #[test]
    fn test_buy_paying_with_only_bonuses() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        // Give player 0 enough bonuses to cover a cheap card entirely.
        g.player_boards[0].cards = vec![
            card_with_cost(Resource::Diamond, &[]),
            card_with_cost(Resource::Sapphire, &[]),
        ];
        g.board[0][0] = card_with_cost(
            Resource::Ruby,
            &[(Resource::Diamond, 1), (Resource::Sapphire, 1)],
        );
        let bank_before = g.tokens.clone();
        g.buy(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert_eq!(3, g.player_boards[0].cards.len());
        assert_eq!(0, g.player_boards[0].tokens.sum());
        assert_eq!(bank_before, g.tokens);
    }

    #[test]
    fn test_buy_paying_with_only_tokens() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].tokens.set(Resource::Diamond, 3);
        g.board[0][0] = card_with_cost(Resource::Ruby, &[(Resource::Diamond, 2)]);
        let bank_before = g.tokens.get(Resource::Diamond);
        g.buy(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert_eq!(1, g.player_boards[0].tokens.get(Resource::Diamond));
        assert_eq!(bank_before + 2, g.tokens.get(Resource::Diamond));
    }

    #[test]
    fn test_buy_gold_fallback_arithmetic() {
        // Player has 1 diamond bonus, 0 plain diamond tokens, 2 gold; card
        // costs 3 diamond.
        // offset = bonuses(1) - cost(3) = -2 for diamond.
        // pb.tokens[diamond] += -2 => -2, bank.tokens[diamond] -= -2 => +2.
        // pb.tokens[diamond] < 0, so gold fallback:
        //   pb.tokens[gold] += pb.tokens[diamond] (-2) => gold 0
        //   bank.tokens[diamond] += pb.tokens[diamond] (-2) => back to original
        //   bank.tokens[gold] -= pb.tokens[diamond] (-2) => bank gold +2
        //   pb.tokens[diamond] = 0
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].cards = vec![card_with_cost(Resource::Diamond, &[])];
        g.player_boards[0].tokens.set(Resource::Gold, 2);
        g.board[0][0] = card_with_cost(Resource::Ruby, &[(Resource::Diamond, 3)]);
        let bank_diamond_before = g.tokens.get(Resource::Diamond);
        let bank_gold_before = g.tokens.get(Resource::Gold);
        g.buy(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert_eq!(0, g.player_boards[0].tokens.get(Resource::Diamond));
        assert_eq!(0, g.player_boards[0].tokens.get(Resource::Gold));
        assert_eq!(bank_diamond_before, g.tokens.get(Resource::Diamond));
        assert_eq!(bank_gold_before + 2, g.tokens.get(Resource::Gold));
    }

    #[test]
    fn test_buy_from_board_with_deck_refill() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board[0][0] = card_with_cost(Resource::Ruby, &[]);
        assert!(!g.decks[0].is_empty());
        let deck_len_before = g.decks[0].len();
        g.buy(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert_eq!(4, g.board[0].len());
        assert_eq!(deck_len_before - 1, g.decks[0].len());
    }

    #[test]
    fn test_buy_from_board_without_deck_refill_shifts_letters() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.decks[0] = vec![];
        g.board[0] = vec![
            card_with_cost(Resource::Ruby, &[]),
            card_with_cost(Resource::Onyx, &[]),
        ];
        g.buy(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert_eq!(1, g.board[0].len());
        assert_eq!(Resource::Onyx, g.board[0][0].resource);
    }

    #[test]
    fn test_buy_from_own_reserve() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].reserve = vec![card_with_cost(Resource::Ruby, &[])];
        g.buy(0, ParsedLoc { row: 3, col: 0 }).unwrap();
        assert_eq!(0, g.player_boards[0].reserve.len());
        assert_eq!(1, g.player_boards[0].cards.len());
    }

    #[test]
    fn test_buy_unaffordable_card_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board[0][0] = card_with_cost(Resource::Ruby, &[(Resource::Diamond, 20)]);
        assert!(g.buy(0, ParsedLoc { row: 0, col: 0 }).is_err());
    }

    #[test]
    fn test_buy_invalid_loc_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        assert!(g.buy(0, ParsedLoc { row: 0, col: 99 }).is_err());
        assert!(g.buy(0, ParsedLoc { row: 5, col: 0 }).is_err());
    }

    #[test]
    fn test_reserve_grants_gold_and_fills_slot() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let gold_before = g.tokens.get(Resource::Gold);
        let logs = g.reserve(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert_eq!(1, g.player_boards[0].reserve.len());
        assert_eq!(1, g.player_boards[0].tokens.get(Resource::Gold));
        assert_eq!(gold_before - 1, g.tokens.get(Resource::Gold));
        assert!(!logs.is_empty());
    }

    #[test]
    fn test_reserve_no_gold_when_bank_out() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.tokens.set(Resource::Gold, 0);
        g.reserve(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert_eq!(0, g.player_boards[0].tokens.get(Resource::Gold));
        assert_eq!(0, g.tokens.get(Resource::Gold));
    }

    #[test]
    fn test_reserve_at_three_already_rejected() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].reserve = vec![
            card_with_cost(Resource::Ruby, &[]),
            card_with_cost(Resource::Ruby, &[]),
            card_with_cost(Resource::Ruby, &[]),
        ];
        assert!(!g.can_reserve(0));
        assert!(g.reserve(0, ParsedLoc { row: 0, col: 0 }).is_err());
    }

    #[test]
    fn test_reserve_own_reserve_slot_rejected() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].reserve = vec![card_with_cost(Resource::Ruby, &[])];
        // Row 3 targets the reserve; `Reserve` rejects it (only board rows
        // 0-2 valid) - the loc parser never offers row 3 as a `reserve`
        // target either, but assert the action-level guard directly.
        assert!(g.reserve(0, ParsedLoc { row: 3, col: 0 }).is_err());
    }

    #[test]
    fn test_reserve_logs_full_card_detail_publicly() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let logs = g.reserve(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        assert!(logs[0].public);
        assert!(!logs[0].content.is_empty());
    }

    #[test]
    fn test_reserve_visibility_not_leaked_to_others() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.reserve(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        let other_state = g.player_state(1);
        assert_eq!(0, other_state.reserve.len());
        assert_eq!(1, other_state.public.player_boards[0].reserve_count);
        let owner_state = g.player_state(0);
        assert_eq!(1, owner_state.reserve.len());
    }

    #[test]
    fn test_discard_down_to_ten_resumes_main_for_next_player() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.current_player = 0;
        g.phase = Phase::Discard;
        g.player_boards[0].tokens.set(GEMS[0], MAX_TOKENS + 2);
        g.discard(0, &[GEMS[0], GEMS[0]]).unwrap();
        assert_eq!(Phase::Main, g.phase);
        assert_eq!(1, g.current_player);
    }

    #[test]
    fn test_discard_insufficient_leaves_discard_phase() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.current_player = 0;
        g.phase = Phase::Discard;
        g.player_boards[0].tokens.set(GEMS[0], MAX_TOKENS + 2);
        g.discard(0, &[GEMS[0]]).unwrap();
        assert_eq!(Phase::Discard, g.phase);
        assert!(g.command(0, "take Diamond Diamond", &players(2)).is_err());
    }

    #[test]
    fn test_discard_tokens_not_held_rejected() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Discard;
        assert!(g.discard(0, &[GEMS[0]]).is_err());
    }

    #[test]
    fn test_discard_gold_allowed() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Discard;
        g.player_boards[0].tokens.set(Resource::Gold, 1);
        g.discard(0, &[Resource::Gold]).unwrap();
        assert_eq!(0, g.player_boards[0].tokens.get(Resource::Gold));
    }

    #[test]
    fn test_visit_zero_affordable_auto_skips() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Main;
        // No bonuses at all, no noble is affordable (nobles cost at least 3
        // of something).
        g.visit_phase();
        assert_eq!(Phase::Main, g.phase);
    }

    #[test]
    fn test_visit_one_affordable_auto_visits() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.nobles = vec![Noble {
            prestige: 3,
            cost: Cost(std::collections::HashMap::from([(Resource::Diamond, 1)])),
        }];
        g.player_boards[0].cards = vec![card_with_cost(Resource::Diamond, &[])];
        g.phase = Phase::Main;
        let logs = g.visit_phase();
        assert_eq!(1, g.player_boards[0].nobles.len());
        assert!(!logs.is_empty());
    }

    #[test]
    fn test_visit_two_or_more_offers_choice_including_unaffordable() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.nobles = vec![
            Noble {
                prestige: 3,
                cost: Cost(std::collections::HashMap::from([(Resource::Diamond, 1)])),
            },
            Noble {
                prestige: 3,
                cost: Cost(std::collections::HashMap::from([(Resource::Sapphire, 1)])),
            },
        ];
        g.player_boards[0].cards = vec![
            card_with_cost(Resource::Diamond, &[]),
            card_with_cost(Resource::Sapphire, &[]),
        ];
        g.phase = Phase::Main;
        g.visit_phase();
        assert_eq!(Phase::Visit, g.phase);
        assert!(g.can_visit(0));

        // Quirk 1: visiting an unaffordable noble by index still succeeds.
        g.nobles.push(Noble {
            prestige: 3,
            cost: Cost(std::collections::HashMap::from([(Resource::Onyx, 10)])),
        });
        g.visit(0, 2).unwrap();
        assert_eq!(1, g.player_boards[0].nobles.len());
    }

    #[test]
    fn test_end_trigger_and_final_round() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_boards[0].cards = vec![Card {
            resource: Resource::Diamond,
            prestige: 15,
            cost: Cost::new(),
        }];
        g.phase = Phase::Discard;
        g.current_player = 0;
        let logs = g.next_player();
        assert!(g.end_triggered);
        assert!(!g.ended);
        assert_eq!(1, g.current_player);
        assert!(!logs.is_empty());

        // Continues until it wraps back to player 0.
        g.current_player = 2;
        g.next_player();
        assert!(g.ended);
    }

    #[test]
    fn test_end_trigger_only_fires_once() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_boards[0].cards = vec![Card {
            resource: Resource::Diamond,
            prestige: 15,
            cost: Cost::new(),
        }];
        g.next_player();
        assert!(g.end_triggered);
        g.player_boards[1].cards = vec![Card {
            resource: Resource::Diamond,
            prestige: 16,
            cost: Cost::new(),
        }];
        let logs = g.next_player();
        assert!(logs.is_empty());
    }

    #[test]
    fn test_placings_tie_broken_by_card_count() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].cards = vec![
            card_with_cost(Resource::Diamond, &[]),
            card_with_cost(Resource::Diamond, &[]),
        ];
        g.player_boards[0].cards[0].prestige = 5;
        g.player_boards[0].cards[1].prestige = 0;
        g.player_boards[1].cards = vec![card_with_cost(Resource::Diamond, &[])];
        g.player_boards[1].cards[0].prestige = 5;
        let placings = g.placings();
        assert_eq!(1, placings[0]);
        assert_eq!(2, placings[1]);
    }

    #[test]
    fn test_placings_true_tie_standard_competition() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        for p in 0..3 {
            g.player_boards[p].cards = vec![card_with_cost(Resource::Diamond, &[])];
            g.player_boards[p].cards[0].prestige = 5;
        }
        // Two tied at the top, one behind: standard-competition should give
        // [1, 1, 3], not compact-ordinal [1, 1, 2].
        g.player_boards[2].cards[0].prestige = 0;
        let placings = g.placings();
        assert_eq!(1, placings[0]);
        assert_eq!(1, placings[1]);
        assert_eq!(3, placings[2]);
    }

    #[test]
    fn test_pub_state_reserve_counts_no_content() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.reserve(0, ParsedLoc { row: 0, col: 0 }).unwrap();
        let pub_state = g.pub_state();
        assert_eq!(1, pub_state.player_boards[0].reserve_count);
        let json = serde_json::to_string(&pub_state).unwrap();
        assert!(!json.contains("\"reserve\""));
    }

    #[test]
    fn test_command_after_finished_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.ended = true;
        assert!(g.command(0, "take Diamond Diamond", &players(2)).is_err());
    }

    #[test]
    fn test_command_wrong_player_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        assert!(g.command(1, "take Diamond Diamond", &players(2)).is_err());
        assert!(g.command_spec(1).is_none());
        assert!(g.command_spec(0).is_some());
    }

    #[test]
    fn test_parser_regression_cases() {
        let p = players(2);
        {
            let mut g = Game::start(2, 1).unwrap().0;
            g.board[0][0] = card_with_cost(Resource::Ruby, &[]);
            g.command(0, "buy A1", &p).unwrap();
        }
        {
            let mut g = Game::start(2, 1).unwrap().0;
            g.player_boards[0].reserve = vec![card_with_cost(Resource::Ruby, &[])];
            g.command(0, "buy A4", &p).unwrap();
        }
        {
            let mut g = Game::start(2, 1).unwrap().0;
            g.command(0, "reserve B2", &p).unwrap();
        }
        {
            let mut g = Game::start(2, 1).unwrap().0;
            g.phase = Phase::Discard;
            g.player_boards[0].tokens.set(Resource::Gold, 1);
            g.command(0, "discard Gold", &p).unwrap();
        }
        {
            let mut g = Game::start(2, 1).unwrap().0;
            g.nobles = vec![
                Noble {
                    prestige: 3,
                    cost: Cost::new(),
                },
                Noble {
                    prestige: 3,
                    cost: Cost::new(),
                },
            ];
            g.phase = Phase::Visit;
            g.command(0, "visit 1", &p).unwrap();
        }
    }
}
