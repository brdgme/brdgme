use std::fmt::Debug;
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Result};
use rand::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

use brdgme_cmd::api;
use brdgme_cmd::requester;
use brdgme_game::{command, Gamer};

pub fn fuzz<F, R>(new_requester: F)
where
    F: Fn() -> R + Send + 'static,
    R: requester::Requester + 'static,
{
    let mut exit_txs: Vec<Sender<()>> = vec![];
    let new_requester = Arc::new(Mutex::new(new_requester));
    let (step_tx, step_rx) = channel();

    for _ in 0..num_cpus::get() {
        let (exit_tx, exit_rx) = channel();
        let step_tx = step_tx.clone();
        let new_requester = new_requester.clone();
        exit_txs.push(exit_tx);
        thread::spawn(move || {
            let client = new_requester.lock().unwrap()();
            let mut fuzzer = Fuzzer::try_new(Box::new(client)).expect("expected to create fuzzer");
            loop {
                step_tx
                    .send(fuzzer.next().expect("failed to get something from fuzzer"))
                    .expect("failed to send fuzz step");
                match exit_rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {}
                }
            }
        });
    }

    let mut tally = FuzzTally::default();
    let mut last_output_at = SystemTime::now();
    let output_interval = Duration::from_secs(1);

    loop {
        let now = SystemTime::now();
        if now
            .duration_since(last_output_at)
            .expect("failed to get duration")
            > output_interval
        {
            eprintln!("{}", tally.render());
            last_output_at = now;
        }
        match step_rx.recv().expect("failed to get step") {
            FuzzStep::Created => tally.started += 1,
            FuzzStep::Finished => tally.finished += 1,
            FuzzStep::CommandOk => tally.commands += 1,
            FuzzStep::UserError { .. } => {
                tally.commands += 1;
                tally.invalid_input += 1;
            }
            FuzzStep::Error {
                game,
                command,
                error,
            } => {
                println!(
                    "\nError detected: {}\n\nCommand: {}\n\nGame: {:?}",
                    error,
                    command.unwrap_or_else(|| "none".to_string()),
                    game
                );
                break;
            }
        }
    }

    for tx in exit_txs {
        tx.send(()).unwrap();
    }
}

pub fn fuzz_gamer<G>()
where
    G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static,
{
    fuzz(requester::gamer::new::<G>)
}

#[derive(Default)]
struct FuzzTally {
    started: usize,
    finished: usize,
    commands: usize,
    invalid_input: usize,
}

impl FuzzTally {
    fn render(&self) -> String {
        format!(
            "Games started: {}   Games finished: {}   Commands: {}   Commands failed: {}",
            self.started, self.finished, self.commands, self.invalid_input
        )
    }
}

struct Fuzzer {
    client: Box<dyn requester::Requester>,
    player_counts: Vec<usize>,
    names: Vec<String>,
    game: Option<FuzzGame>,
    rng: ThreadRng,
}

impl Fuzzer {
    fn try_new(mut client: Box<dyn requester::Requester>) -> Result<Self> {
        let player_counts = match client.request(&api::Request::PlayerCounts)? {
            api::Response::PlayerCounts { player_counts } => player_counts,
            v => {
                return Err(anyhow!(
                    "invalid response to player counts request: {:?}",
                    v
                ))
            }
        };
        Ok(Fuzzer {
            client,
            player_counts,
            names: vec![],
            game: None,
            rng: thread_rng(),
        })
    }

    fn new_game(&mut self) -> Result<()> {
        let players = *self
            .player_counts
            .choose(&mut self.rng)
            .ok_or_else(|| anyhow!("could not get player counts from {:?}", self.player_counts))?;
        self.names = names(players);
        match self.client.request(&api::Request::New { players })? {
            api::Response::New {
                game,
                player_renders,
                ..
            } => {
                self.game = Some(FuzzGame {
                    game,
                    player_renders,
                });
                Ok(())
            }
            v => Err(anyhow!("invalid response for new game: {:?}", v)),
        }
    }

    fn command(&mut self) -> Result<CommandResponse> {
        let (player, command_spec, state) = match self.game {
            Some(FuzzGame {
                game:
                    api::GameResponse {
                        ref state,
                        status: brdgme_game::Status::Active { ref whose_turn, .. },
                        ..
                    },
                ref player_renders,
            }) => {
                let player = *whose_turn.choose(&mut self.rng).ok_or_else(|| {
                    anyhow!("unable to pick active turn player from: {:?}", whose_turn)
                })?;
                if player_renders.len() <= player {
                    return Err(anyhow!(
                        "there is no player_render for player {} in {:?}",
                        player,
                        player_renders
                    ));
                }
                let player_render = &player_renders[player];
                if player_render.command_spec.is_none() {
                    return Err(anyhow!("player {}'s command_spec is None", player));
                }
                (player, player_render.clone().command_spec.unwrap(), state)
            }
            Some(FuzzGame {
                game:
                    api::GameResponse {
                        status: brdgme_game::Status::Finished { .. },
                        ..
                    },
                ..
            }) => return Err(anyhow!("the game is already finished")),
            None => return Err(anyhow!("there isn't a game")),
        };
        exec_rand_command(
            &mut (*self.client),
            state.to_string(),
            player,
            self.names.clone(),
            &command_spec,
            &mut self.rng,
        )
    }
}

#[derive(Debug)]
enum FuzzStep {
    Created,
    CommandOk,
    UserError {
        #[allow(dead_code)]
        message: String,
    },
    Finished,
    Error {
        game: Option<FuzzGame>,
        command: Option<String>,
        error: String,
    },
}

impl Iterator for Fuzzer {
    type Item = FuzzStep;

    fn next(&mut self) -> Option<Self::Item> {
        match self.game {
            Some(_) => match self.command() {
                Ok(CommandResponse::Ok(FuzzGame {
                    game:
                        api::GameResponse {
                            status: brdgme_game::Status::Finished { .. },
                            ..
                        },
                    ..
                })) => {
                    self.game = None;
                    Some(FuzzStep::Finished)
                }
                Ok(CommandResponse::Ok(game)) => {
                    self.game = Some(game);
                    Some(FuzzStep::CommandOk)
                }
                Ok(CommandResponse::UserError { message }) => Some(FuzzStep::UserError { message }),
                Err(e) => Some(FuzzStep::Error {
                    game: self.game.clone(),
                    command: None,
                    error: e.to_string(),
                }),
            },
            None => match self.new_game() {
                Ok(()) => Some(FuzzStep::Created),
                Err(e) => Some(FuzzStep::Error {
                    game: None,
                    command: None,
                    error: e.to_string(),
                }),
            },
        }
    }
}

fn names(players: usize) -> Vec<String> {
    (0..players).map(|p| format!("player{}", p)).collect()
}

#[derive(Clone, Debug)]
struct FuzzGame {
    game: api::GameResponse,
    player_renders: Vec<api::PlayerRender>,
}

enum CommandResponse {
    Ok(FuzzGame),
    UserError { message: String },
}

fn exec_rand_command(
    client: &mut (impl requester::Requester + ?Sized),
    game: String,
    player: usize,
    names: Vec<String>,
    command_spec: &command::Spec,
    rng: &mut ThreadRng,
) -> Result<CommandResponse> {
    exec_command(
        client,
        rand_command(command_spec, &names, rng),
        game,
        player,
        names,
    )
}

fn exec_command(
    client: &mut (impl requester::Requester + ?Sized),
    command: String,
    game: String,
    player: usize,
    names: Vec<String>,
) -> Result<CommandResponse> {
    match client.request(&api::Request::Play {
        command,
        game,
        names,
        player,
    })? {
        api::Response::Play {
            ref remaining_input,
            ..
        } if !remaining_input.trim().is_empty() => Ok(CommandResponse::UserError {
            message: "did not parse all input".to_string(),
        }),
        api::Response::Play {
            game,
            player_renders,
            ..
        } => Ok(CommandResponse::Ok(FuzzGame {
            game,
            player_renders,
        })),
        api::Response::UserError { message } => Ok(CommandResponse::UserError { message }),
        v => Err(anyhow!(format!("{:?}", v))),
    }
}

fn rand_command(command_spec: &command::Spec, players: &[String], rng: &mut ThreadRng) -> String {
    brdgme_rand_bot::spec_to_command(command_spec, command_spec, players, rng).join("")
}
