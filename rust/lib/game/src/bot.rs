use std::io::Write;

use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

use ::log::trace;

use crate::command::Spec as CommandSpec;
use crate::errors::GameError;
use crate::game::Gamer;

const BOT_COMMAND_QUALITY_DEFAULT: u8 = 128;

#[derive(Serialize, Deserialize)]
pub struct BotCommand {
    pub quality: u8,
    pub commands: Vec<String>,
}

impl Default for BotCommand {
    fn default() -> Self {
        BotCommand {
            quality: BOT_COMMAND_QUALITY_DEFAULT,
            commands: vec![],
        }
    }
}

impl<I> From<I> for BotCommand
where
    I: Into<String>,
{
    fn from(s: I) -> Self {
        BotCommand {
            commands: vec![s.into()],
            ..BotCommand::default()
        }
    }
}

pub trait Botter<T: Gamer> {
    fn commands(
        &mut self,
        player: usize,
        player_state: &T::PlayerState,
        players: &[String],
        command_spec: &CommandSpec,
        game_id: Option<String>,
    ) -> Vec<BotCommand>;
}

pub struct Fuzzer<G: Gamer, B: Botter<G>> {
    game: Option<G>,
    player_counts: Vec<usize>,
    player_names: Vec<String>,
    player_count: usize,
    bot: B,
    rng: ThreadRng,
    game_count: usize,
    command_count: usize,
    invalid_input_count: usize,
}

impl<G: Gamer, B: Botter<G>> Fuzzer<G, B> {
    pub fn new(bot: B) -> Self {
        let player_counts = G::player_counts();
        Self {
            game: None,
            player_names: (0..player_counts.iter().max().cloned().unwrap_or(0))
                .map(|c| format!("{}", c))
                .collect(),
            player_counts,
            player_count: 0,
            bot,
            rng: thread_rng(),
            game_count: 0,
            command_count: 0,
            invalid_input_count: 0,
        }
    }

    pub fn status(&self) -> String {
        format!(
            "Games: {}\tCommands: {}\tInvalid inputs: {}",
            self.game_count, self.command_count, self.invalid_input_count
        )
    }

    pub fn fuzz<O>(&mut self, out: &mut O)
    where
        O: Write,
    {
        let mut last_status = chrono::Utc::now().timestamp();
        loop {
            self.next();
            let now = chrono::Utc::now().timestamp();
            if now - last_status > 1 {
                last_status = now;
                writeln!(out, "{}", self.status()).unwrap();
            }
        }
    }
}

impl<G: Gamer, B: Botter<G>> Iterator for Fuzzer<G, B> {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        if self.game.as_ref().map(|g| g.is_finished()).unwrap_or(true) {
            self.game_count += 1;
            self.player_count = *self
                .player_counts
                .choose(&mut self.rng)
                .expect("no player counts for game type");
            self.game = Some(
                G::start(self.player_count)
                    .expect("failed to create new game")
                    .0,
            );
        } else if let Some(ref mut game) = self.game {
            let player = *game
                .whose_turn()
                .choose(&mut self.rng)
                .expect("is nobody's turn");
            let player_state = game.player_state(player);
            let command_spec = game.command_spec(player).expect("expected a command spec");
            let bot_commands = self.bot.commands(
                player,
                &player_state,
                &self.player_names[..self.player_count],
                &command_spec,
                Some(format!("{}", self.game_count)),
            );
            let input = bot_commands
                .choose(&mut self.rng)
                .expect("bot returned no commands")
                .to_owned();
            if input.commands.is_empty() {
                panic!("BotCommand with no commands was returned by bot")
            }
            let cmd = &input.commands[0];
            let cmd_res = game.command(player, cmd, &self.player_names);
            self.command_count += 1;
            match cmd_res {
                Ok(..) => {}
                Err(GameError::InvalidInput { message }) => {
                    self.invalid_input_count += 1;
                    trace!("invalid input '{}' for player {}: {}", cmd, player, message)
                }
                _ => panic!(
                    "error running command '{}' for player {}, {:?}",
                    cmd, player, cmd_res
                ),
            }
        }
        Some(())
    }
}
