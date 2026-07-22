use std::fmt;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status, placings_log};
use brdgme_markup::Node as N;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub mod command;
mod render;

use command::Command;

pub const BOARD_SIZE: usize = 3;
pub const NUM_PLAYERS: usize = 2;
pub type Board = [[Cell; BOARD_SIZE]; BOARD_SIZE];

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    #[default]
    Empty,
    X,
    O,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loc {
    pub row: usize,
    pub col: usize,
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let index = self.row * BOARD_SIZE + self.col;
        let letter = char::from(b'a' + index as u8);
        write!(f, "{letter}")
    }
}

pub fn all_locations() -> Vec<Loc> {
    (0..BOARD_SIZE)
        .flat_map(|row| (0..BOARD_SIZE).map(move |col| Loc { row, col }))
        .collect()
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,
    pub start_player: usize,
    pub board: Board,
    pub rng: GameRng,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players (always 2).
    pub players: usize,
    /// Index (0 or 1) of the player whose turn it is.
    pub current_player: usize,
    /// Index (0 or 1) of the player who goes first (plays as X).
    pub start_player: usize,
    /// The 3x3 game board. Each cell is Empty, X, or O.
    pub board: Board,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player (0 or 1) this private state belongs to.
    pub player: usize,
}

impl Game {
    pub fn next_player(&mut self) {
        self.current_player = (self.current_player + 1) % NUM_PLAYERS;
    }

    pub fn can_play(&self, player: usize) -> bool {
        !self.is_finished() && self.current_player == player
    }

    pub fn play(&mut self, player: usize, loc: Loc) -> Result<Vec<Log>, GameError> {
        if self.is_finished() {
            return Err(GameError::Finished);
        }
        if self.current_player != player {
            return Err(GameError::NotYourTurn);
        }
        if loc.row >= BOARD_SIZE || loc.col >= BOARD_SIZE {
            return Err(GameError::invalid_input("location is outside the board"));
        }
        if self.board[loc.row][loc.col] != Cell::Empty {
            return Err(GameError::invalid_input("cell is not empty"));
        }

        self.board[loc.row][loc.col] = if player == self.start_player {
            Cell::X
        } else {
            Cell::O
        };
        let mark = if player == self.start_player {
            "X"
        } else {
            "O"
        };
        self.next_player();
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" played "),
            N::Bold(vec![N::text(mark.to_string())]),
            N::text(" at "),
            N::Bold(vec![N::text(loc.to_string())]),
        ])])
    }

    pub fn winner(&self) -> Option<usize> {
        let line_winner = (0..BOARD_SIZE)
            .find_map(|row| Self::matching_line(self.board[row]))
            .or_else(|| {
                (0..BOARD_SIZE).find_map(|col| {
                    Self::matching_line([
                        self.board[0][col],
                        self.board[1][col],
                        self.board[2][col],
                    ])
                })
            })
            .or_else(|| Self::matching_line([self.board[0][0], self.board[1][1], self.board[2][2]]))
            .or_else(|| {
                Self::matching_line([self.board[0][2], self.board[1][1], self.board[2][0]])
            });

        line_winner.map(|cell| match cell {
            Cell::X => self.start_player,
            Cell::O => (self.start_player + 1) % NUM_PLAYERS,
            Cell::Empty => self.start_player,
        })
    }

    fn matching_line(line: [Cell; BOARD_SIZE]) -> Option<Cell> {
        (line[0] != Cell::Empty && line.iter().all(|cell| *cell == line[0])).then_some(line[0])
    }

    pub fn is_finished(&self) -> bool {
        self.winner().is_some() || self.board.iter().flatten().all(|cell| *cell != Cell::Empty)
    }

    fn placings(&self) -> Vec<usize> {
        let winner = self.winner();
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|player| vec![i32::from(winner == Some(player))])
            .collect();
        gen_placings(&metrics)
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if players != NUM_PLAYERS {
            return Err(GameError::PlayerCount {
                min: NUM_PLAYERS,
                max: NUM_PLAYERS,
                given: players,
            });
        }

        let mut rng = GameRng::seed_from_u64(seed);
        let start_player = rng.random_range(0..NUM_PLAYERS);
        Ok((
            Game {
                players,
                current_player: start_player,
                start_player,
                board: [[Cell::Empty; BOARD_SIZE]; BOARD_SIZE],
                rng,
            },
            vec![],
        ))
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            current_player: self.current_player,
            start_player: self.start_player,
            board: self.board,
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
        }
    }

    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let output = match self.command_parser(player) {
            Some(parser) => parser,
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
                value: Command::Play(loc),
                ..
            }) => {
                let mut logs = self.play(player, loc)?;
                if self.is_finished() {
                    logs.push(placings_log(&self.placings(), None));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: true,
                    remaining_input: remaining.to_string(),
                })
            }
            Err(error) => Err(GameError::invalid_input(error.to_string())),
        }
    }

    fn status(&self) -> Status {
        if self.is_finished() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: vec![],
            }
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|parser| parser.to_spec())
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        vec![NUM_PLAYERS]
    }

    fn points(&self) -> Vec<f32> {
        let winner = self.winner();
        (0..self.players)
            .map(|player| f32::from(winner == Some(player)))
            .collect()
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
    use std::collections::BTreeSet;

    use brdgme_game::rng::GameRng;
    use brdgme_game::{Gamer, Renderer, Status};

    use super::*;

    fn players() -> Vec<String> {
        vec!["Mick".to_string(), "Steve".to_string()]
    }

    fn loc(letter: char) -> Loc {
        all_locations()
            .into_iter()
            .find(|loc| loc.to_string() == letter.to_string())
            .expect("location should exist")
    }

    fn game_with_starter(starter: usize) -> Game {
        Game {
            players: NUM_PLAYERS,
            current_player: starter,
            start_player: starter,
            board: [[Cell::Empty; BOARD_SIZE]; BOARD_SIZE],
            rng: GameRng::seed_from_u64(1),
        }
    }

    #[test]
    fn test_new() {
        assert!(Game::start(NUM_PLAYERS, 1).is_ok());
    }

    #[test]
    fn test_new_errors_with_incorrect_players() {
        assert!(Game::start(1, 1).is_err());
    }

    #[test]
    fn test_render_for_player() {
        let game = game_with_starter(0);

        assert_eq!(game.pub_state().render(), game.player_state(0).render());
        assert!(!game.player_state(0).render().is_empty());
    }

    #[test]
    fn test_player_action() {
        let mut game = game_with_starter(0);

        game.command(0, "play a", &players()).unwrap();
        assert_eq!(game.board[0][0], Cell::X);
        assert!(game.command(1, "play moog", &players()).is_err());
    }

    #[test]
    fn test_next_player() {
        let mut game = game_with_starter(0);

        game.next_player();
        assert_eq!(game.current_player, 1);
        game.next_player();
        assert_eq!(game.current_player, 0);
    }

    #[test]
    fn test_play_same_cell() {
        let mut game = game_with_starter(0);

        game.play(0, loc('a')).unwrap();
        assert!(game.play(1, loc('a')).is_err());
    }

    #[test]
    fn test_mark_cell_for_player() {
        let mut game = game_with_starter(0);

        game.play(0, Loc { row: 1, col: 1 }).unwrap();
        assert_eq!(game.board[1][1], Cell::X);
        game.play(1, Loc { row: 2, col: 1 }).unwrap();
        assert_eq!(game.board[2][1], Cell::O);
        assert!(game.play(0, Loc { row: 1, col: 1 }).is_err());
    }

    #[test]
    fn test_check_winner() {
        let mut game = game_with_starter(0);

        game.board = [
            [Cell::X, Cell::X, Cell::O],
            [Cell::X, Cell::O, Cell::X],
            [Cell::X, Cell::X, Cell::O],
        ];
        assert!(game.is_finished());
        assert_eq!(game.winner(), Some(0));

        game.board = [
            [Cell::O, Cell::X, Cell::O],
            [Cell::X, Cell::O, Cell::X],
            [Cell::X, Cell::X, Cell::O],
        ];
        assert!(game.is_finished());
        assert_eq!(game.winner(), Some(1));

        game.board = [
            [Cell::X, Cell::X, Cell::O],
            [Cell::O, Cell::O, Cell::X],
            [Cell::X, Cell::X, Cell::O],
        ];
        assert!(game.is_finished());
        assert_eq!(game.winner(), None);
    }

    #[test]
    fn test_is_finished() {
        let mut game = game_with_starter(0);

        game.board = [
            [Cell::X, Cell::X, Cell::O],
            [Cell::Empty, Cell::Empty, Cell::X],
            [Cell::X, Cell::X, Cell::O],
        ];
        assert!(!game.is_finished());

        game.board = [
            [Cell::X, Cell::X, Cell::O],
            [Cell::O, Cell::O, Cell::X],
            [Cell::X, Cell::X, Cell::O],
        ];
        assert!(game.is_finished());

        game.board = [
            [Cell::Empty, Cell::Empty, Cell::X],
            [Cell::Empty, Cell::X, Cell::Empty],
            [Cell::X, Cell::Empty, Cell::Empty],
        ];
        assert!(game.is_finished());
    }

    #[test]
    fn test_allow_upper_case() {
        let mut game = game_with_starter(0);

        game.command(0, "play A", &players()).unwrap();
        assert_eq!(game.board[0][0], Cell::X);
    }

    #[test]
    fn player_counts_are_exactly_two() {
        assert_eq!(Game::player_counts(), vec![NUM_PLAYERS]);
        assert_eq!(game_with_starter(0).player_count(), NUM_PLAYERS);
        assert!(Game::start(3, 1).is_err());
    }

    #[test]
    fn starter_selection_is_deterministic() {
        let first = Game::start(NUM_PLAYERS, 42).unwrap().0;
        let second = Game::start(NUM_PLAYERS, 42).unwrap().0;

        assert_eq!(first.start_player, second.start_player);
        assert_eq!(first.current_player, second.current_player);
        assert_eq!(first.rng, second.rng);

        let starters: BTreeSet<usize> = (0..64)
            .map(|seed| Game::start(NUM_PLAYERS, seed).unwrap().0.start_player)
            .collect();
        assert_eq!(starters, BTreeSet::from([0, 1]));
    }

    #[test]
    fn every_winning_line_is_detected() {
        let lines = [
            [(0, 0), (0, 1), (0, 2)],
            [(1, 0), (1, 1), (1, 2)],
            [(2, 0), (2, 1), (2, 2)],
            [(0, 0), (1, 0), (2, 0)],
            [(0, 1), (1, 1), (2, 1)],
            [(0, 2), (1, 2), (2, 2)],
            [(0, 0), (1, 1), (2, 2)],
            [(0, 2), (1, 1), (2, 0)],
        ];

        for line in lines {
            let mut game = game_with_starter(0);
            for (row, col) in line {
                game.board[row][col] = Cell::X;
            }
            assert_eq!(game.winner(), Some(0), "line {line:?}");
        }
    }

    #[test]
    fn draw_status_and_points_are_tied() {
        let mut game = game_with_starter(0);
        game.board = [
            [Cell::X, Cell::X, Cell::O],
            [Cell::O, Cell::O, Cell::X],
            [Cell::X, Cell::X, Cell::O],
        ];

        assert_eq!(game.winner(), None);
        assert_eq!(
            game.status(),
            Status::Finished {
                placings: vec![1, 1],
                stats: vec![],
            }
        );
        assert_eq!(game.points(), vec![0.0, 0.0]);
    }

    #[test]
    fn winner_status_and_points_credit_the_winner() {
        let mut game = game_with_starter(0);
        game.board[0] = [Cell::X, Cell::X, Cell::X];

        assert_eq!(
            game.status(),
            Status::Finished {
                placings: vec![1, 2],
                stats: vec![],
            }
        );
        assert_eq!(game.points(), vec![1.0, 0.0]);
    }

    #[test]
    fn command_is_available_only_to_current_player() {
        let mut game = game_with_starter(0);

        assert!(game.command_parser(0).is_some());
        assert!(game.command_parser(1).is_none());
        game.board[0] = [Cell::X, Cell::X, Cell::X];
        assert!(game.command_parser(0).is_none());
        assert!(game.command_parser(1).is_none());
    }

    #[test]
    fn wrong_player_cannot_play() {
        let mut game = game_with_starter(0);

        assert!(game.play(1, loc('a')).is_err());
        assert_eq!(game.board[0][0], Cell::Empty);
        assert_eq!(game.current_player, 0);
    }

    #[test]
    fn finished_game_rejects_play() {
        let mut game = game_with_starter(0);
        game.board[0] = [Cell::X, Cell::X, Cell::X];

        assert!(game.play(0, loc('d')).is_err());
        assert!(game.command(0, "play d", &players()).is_err());
        assert_eq!(game.board[1][0], Cell::Empty);
    }

    #[test]
    fn command_preserves_remaining_input() {
        let mut game = game_with_starter(0);

        let response = game.command(0, "play a then", &players()).unwrap();
        assert_eq!(response.remaining_input, " then");
        assert!(!response.logs.is_empty());
        assert!(response.can_undo);
    }

    #[test]
    fn command_driven_winning_move_finishes_and_advances() {
        let mut game = game_with_starter(0);
        game.board = [
            [Cell::X, Cell::X, Cell::Empty],
            [Cell::O, Cell::O, Cell::Empty],
            [Cell::Empty, Cell::Empty, Cell::Empty],
        ];

        let response = game.command(0, "play c next", &players()).unwrap();

        assert!(game.is_finished());
        assert_eq!(game.current_player, 1);
        assert!(!response.logs.is_empty());
        assert!(response.can_undo);
        assert_eq!(response.remaining_input, " next");
    }

    #[test]
    fn exact_render_markup_matches_the_old_board() {
        let mut game = game_with_starter(0);
        game.board = [
            [Cell::X, Cell::O, Cell::Empty],
            [Cell::Empty, Cell::X, Cell::Empty],
            [Cell::O, Cell::Empty, Cell::X],
        ];

        assert_eq!(
            brdgme_markup::to_string(&game.pub_state().render()),
            concat!(
                "{{b}}x{{/b}}{{fg grey}}|{{/fg}}{{b}}o{{/b}}",
                "{{fg grey}}|{{/fg}}{{fg blue}}c{{/fg}}\n",
                "{{fg blue}}d{{/fg}}{{fg grey}}|{{/fg}}",
                "{{b}}x{{/b}}{{fg grey}}|{{/fg}}",
                "{{fg blue}}f{{/fg}}\n",
                "{{b}}o{{/b}}{{fg grey}}|{{/fg}}",
                "{{fg blue}}h{{/fg}}{{fg grey}}|{{/fg}}",
                "{{b}}x{{/b}}",
                "\n{{player 0}} is X, {{player 1}} is O",
            )
        );
    }

    #[test]
    fn game_and_render_states_round_trip_through_json() {
        let mut game = game_with_starter(1);
        game.play(1, loc('e')).unwrap();

        let game_json = serde_json::to_string(&game).unwrap();
        let decoded_game: Game = serde_json::from_str(&game_json).unwrap();
        assert_eq!(decoded_game, game);

        let public = game.pub_state();
        let public_json = serde_json::to_string(&public).unwrap();
        let decoded_public: PubState = serde_json::from_str(&public_json).unwrap();
        assert_eq!(decoded_public, public);

        let player = game.player_state(0);
        let player_json = serde_json::to_string(&player).unwrap();
        let decoded_player: PlayerState = serde_json::from_str(&player_json).unwrap();
        assert_eq!(decoded_player, player);
    }

    #[test]
    fn states_capture_visible_game_data() {
        let game = game_with_starter(1);
        let public = game.pub_state();

        assert_eq!(public.players, game.players);
        assert_eq!(public.current_player, game.current_player);
        assert_eq!(public.start_player, game.start_player);
        assert_eq!(public.board, game.board);
        assert_eq!(game.player_state(0), PlayerState { public, player: 0 });
    }

    #[test]
    fn locations_are_row_major_lowercase_letters() {
        let locations = all_locations();

        assert_eq!(locations.len(), BOARD_SIZE * BOARD_SIZE);
        assert_eq!(
            locations
                .iter()
                .map(ToString::to_string)
                .collect::<String>(),
            "abcdefghi"
        );
        assert_eq!(locations[0], Loc { row: 0, col: 0 });
        assert_eq!(locations[8], Loc { row: 2, col: 2 });
    }

    #[test]
    fn player_one_starter_gets_credit_for_x_win() {
        let mut game = game_with_starter(1);
        game.board[0] = [Cell::X, Cell::X, Cell::X];

        assert_eq!(game.winner(), Some(1));
        assert_eq!(game.placings(), vec![2, 1]);
        assert_eq!(game.points(), vec![0.0, 1.0]);
    }
}
