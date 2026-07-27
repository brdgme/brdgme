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
        command::Spec::OneOf(ref options) => options
            .choose(rng)
            .map(|o| spec_to_command(o, spec, players, rng))
            .unwrap_or_default(),
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
                if i != 0
                    && let Some(d) = delim
                {
                    parts.extend(spec_to_command(d, ctx, players, rng));
                }
                parts.extend(spec_to_command(spec, ctx, players, rng));
            }
            parts
        }
        command::Spec::Doc { ref spec, .. } => spec_to_command(spec, ctx, players, rng),
        command::Spec::Player => players
            .choose(rng)
            .map(|p| vec![p.to_owned()])
            .unwrap_or_default(),
        command::Spec::Space => vec![" ".to_string()],
    }
}

fn commands(command_spec: &command::Spec, players: &[String]) -> Vec<BotCommand> {
    let mut rng = rand::rng();
    vec![
        spec_to_command(command_spec, command_spec, players, &mut rng)
            .join("")
            .into(),
    ]
}

/// Reads a `bot_cli::Request` from `input` and writes generated commands to
/// `output`. Only `command_spec` and `players` are used - RandBot doesn't need
/// game state, so it works with arbitrary games.
pub fn cli<I, O>(input: I, output: &mut O)
where
    I: Read,
    O: Write,
{
    let request = serde_json::from_reader::<_, bot_cli::Request>(input)
        .expect("failed to parse bot request JSON from input");
    writeln!(
        output,
        "{}",
        serde_json::to_string(&commands(&request.command_spec, &request.players))
            .expect("failed to encode bot commands as JSON")
    )
    .expect("failed to write bot commands to output");
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

#[cfg(test)]
mod tests {
    use super::*;
    use brdgme_game::command::Spec;

    #[test]
    fn empty_oneof_yields_no_tokens_instead_of_panicking() {
        let mut rng = rand::rng();
        let spec = Spec::OneOf(vec![]);
        assert_eq!(
            Vec::<String>::new(),
            spec_to_command(&spec, &spec, &["a".to_string()], &mut rng)
        );
    }

    #[test]
    fn player_spec_with_no_players_yields_no_tokens() {
        let mut rng = rand::rng();
        let spec = Spec::Player;
        assert_eq!(
            Vec::<String>::new(),
            spec_to_command(&spec, &spec, &[], &mut rng)
        );
    }

    #[test]
    fn space_tokens_join_without_double_spaces() {
        let players = vec!["mick".to_string()];
        let spec = Spec::Chain(vec![
            Spec::Token("roll".to_string()),
            Spec::Space,
            Spec::Token("2".to_string()),
        ]);
        let bots = commands(&spec, &players);
        assert_eq!(vec!["roll 2".to_string()], bots[0].commands);
    }

    #[test]
    fn cli_writes_command_json_for_valid_request() {
        let req = bot_cli::Request {
            player: 0,
            player_state: "{}".to_string(),
            players: vec!["a".to_string()],
            command_spec: Spec::Token("go".to_string()),
            game_id: None,
        };
        let input = serde_json::to_vec(&req).unwrap();
        let mut out: Vec<u8> = vec![];
        cli(input.as_slice(), &mut out);
        let cmds: Vec<brdgme_game::bot::BotCommand> = serde_json::from_slice(&out).unwrap();
        assert_eq!(vec!["go".to_string()], cmds[0].commands);
    }
}
