use std::io::{Read, Write};

use rand::prelude::*;

use brdgme_cmd::bot_cli;
use brdgme_game::Gamer;
use brdgme_game::bot::{BotCommand, Botter, Fuzzer};
use brdgme_game::command;

pub struct RandBot;

fn bounded_i32(v: i32, min: i32, max: i32) -> i32 {
    assert!(min <= max);
    let mut v = i64::from(v);
    let min64 = i64::from(min);
    let max64 = i64::from(max);
    let range_size = max64 - min64 + 1;
    if v < min64 {
        v += range_size * ((min64 - v) / range_size + 1);
    }
    (min64 + (v - min64) % range_size) as i32
}

pub fn spec_to_command(
    spec: &command::Spec,
    ctx: &command::Spec,
    players: &[String],
    rng: &mut ThreadRng,
) -> Vec<String> {
    match *spec {
        command::Spec::Int { min, max } => {
            if min.is_some() && max.is_some() && min > max {
                panic!("invalid Int spec\nSpec: {:?}\nContext: {:?}", spec, ctx)
            }
            vec![format!(
                "{}",
                bounded_i32(
                    rng.random(),
                    min.unwrap_or(i32::MIN),
                    max.unwrap_or(i32::MAX)
                )
            )]
        }
        command::Spec::Token(ref token) => vec![token.to_owned()],
        command::Spec::Enum { ref values, .. } => values
            .choose(rng)
            .map(|v| vec![v.to_owned()])
            .unwrap_or_else(Vec::new),
        command::Spec::OneOf(ref options) => {
            spec_to_command(options.choose(rng).unwrap(), spec, players, rng)
        }
        command::Spec::Chain(ref chain) => chain
            .iter()
            .flat_map(|c| spec_to_command(c, ctx, players, rng))
            .collect(),
        command::Spec::Opt(ref spec) => {
            if rng.random() {
                spec_to_command(spec, ctx, players, rng)
            } else {
                vec![]
            }
        }
        command::Spec::Many {
            ref spec,
            min,
            max,
            ref delim,
        } => {
            let min = min.unwrap_or(0) as i32;
            let max = max.unwrap_or(3) as i32;
            let n = bounded_i32(rng.random(), min, max);
            let mut parts: Vec<String> = vec![];
            for i in 0..n {
                if i != 0 {
                    if let Some(d) = delim {
                        parts.extend(spec_to_command(d, ctx, players, rng));
                    }
                }
                parts.extend(spec_to_command(spec, ctx, players, rng));
            }
            parts
        }
        command::Spec::Doc { ref spec, .. } => spec_to_command(spec, ctx, players, rng),
        command::Spec::Player => vec![players.choose(rng).unwrap().to_owned()],
        command::Spec::Space => vec![" ".to_string()],
    }
}

fn commands(command_spec: &command::Spec, players: &[String]) -> Vec<BotCommand> {
    let mut rng = rand::rng();
    vec![
        spec_to_command(command_spec, command_spec, players, &mut rng)
            .join(" ")
            .into(),
    ]
}

// / Most bots just want to use `brdgme_cmd::bot_cli`, however because RandBot
// doesn't care about game / state, we implement a more simplified version of
// the CLI here. This allows the bot to be used / with arbitrary games as long
// as the command spec is generated.
pub fn cli<I, O>(input: I, output: &mut O)
where
    I: Read,
    O: Write,
{
    let request = serde_json::from_reader::<_, bot_cli::Request>(input).unwrap();
    writeln!(
        output,
        "{}",
        serde_json::to_string(&commands(&request.command_spec, &request.players)).unwrap()
    )
    .unwrap();
}

impl<T: Gamer> Botter<T> for RandBot {
    fn commands(
        &mut self,
        _player: usize,
        _player_state: &T::PlayerState,
        players: &[String],
        command_spec: &command::Spec,
        _game_id: Option<String>,
    ) -> Vec<BotCommand> {
        commands(command_spec, players)
    }
}

pub fn fuzz<G, O>(out: &mut O)
where
    G: Gamer,
    O: Write,
{
    Fuzzer::<G, _>::new(RandBot {}).fuzz(out);
}
