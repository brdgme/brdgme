use serde_json;

use brdgme_color;
use term_size;

use std::borrow::Cow;
use std::fs::File;
use std::io::prelude::*;
use std::io::{stdin, stdout};
use std::iter::repeat;

use brdgme_color::{player_color, Style};
use brdgme_game::command::doc;
use brdgme_game::Status;
use brdgme_markup::{self, ansi, from_lines, to_lines, transform, Node, Player, TNode};

use crate::api::{CliLog, GameResponse, Request, Response};
use crate::requester::Requester;

pub fn repl<T>(client: &mut T)
where
    T: Requester,
{
    print!("{}", Style::default().ansi());
    let mut player_names: Vec<String> = vec![];
    loop {
        let player = prompt(format!(
            "Enter player {} (or blank to finish)",
            player_names.len() + 1
        ));
        if player == "" {
            break;
        }
        player_names.push(player);
    }
    let players = player_names
        .iter()
        .enumerate()
        .map(|(i, pn)| Player {
            name: pn.to_string(),
            color: player_color(i).to_owned(),
        })
        .collect::<Vec<Player>>();
    let (mut game, logs, mut public_render, mut player_renders) = match client
        .request(&Request::New {
            players: players.len(),
        })
        .unwrap()
    {
        Response::New {
            game,
            logs,
            public_render,
            player_renders,
        } => (game, logs, public_render, player_renders),
        Response::UserError { message } | Response::SystemError { message } => panic!(message),
        _ => panic!("wrong reponse"),
    };
    output_nl();
    output_logs(logs, &players);
    let mut undo_stack: Vec<GameResponse> = vec![game.clone()];
    loop {
        match game.status.clone() {
            Status::Finished { placings, .. } => {
                output_nl();
                match placings.as_slice() {
                    placings if placings.is_empty() => {
                        println!("The game is over, there are no winners")
                    }
                    placings => println!(
                        "The game is over, placings: {}",
                        placings
                            .iter()
                            .enumerate()
                            .filter_map(|(player, placing)| players
                                .get(player)
                                .map(|p| format!("{} ({})", p.name, placing)))
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                }
                output_nl();
                output_markup(&public_render.render, &players);
                return;
            }
            Status::Active { ref whose_turn, .. } => {
                output_nl();
                if whose_turn.is_empty() {
                    output_nodes(&[Node::text("no player's turn, exiting")], &players);
                    return;
                }
                let current_player = whose_turn[0];
                output_markup(&player_renders[current_player].render, &players);
                println!();
                if let Some(ref spec) = player_renders[current_player].command_spec {
                    output_nl();
                    output_nodes(&doc::render(&spec.doc()), &players);
                }
                println!();
                let input = prompt(ansi(&transform(&[Node::Player(current_player)], &players)));
                match input.as_ref() {
                    ":dump" | ":d" => println!("{:#?}", game),
                    ":json" => println!("{}", serde_json::ser::to_string_pretty(&game).unwrap()),
                    ":save" => {
                        let mut file = File::create("game.json").expect("could not create file");
                        write!(
                            file,
                            "{}",
                            serde_json::ser::to_string_pretty(&game)
                                .expect("could not get game JSON")
                        ).expect("could not write to file");
                    }
                    ":load" => {
                        let file = File::open("game.json").expect("could not open file");
                        game = serde_json::from_reader(file).expect("could not read file JSON");
                    }
                    ":undo" | ":u" => {
                        if let Some(u) = undo_stack.pop() {
                            game = u;
                        } else {
                            output_nodes(
                                &[Node::Bold(vec![Node::Fg(
                                    brdgme_color::RED.into(),
                                    vec![Node::text("No undos available")],
                                )])],
                                &players,
                            );
                        }
                    }
                    ":quit" | ":q" => return,
                    _ => match client
                        .request(&Request::Play {
                            player: current_player,
                            command: input,
                            names: player_names.clone(),
                            game: game.state.clone(),
                        })
                        .unwrap()
                    {
                        Response::Play {
                            game: new_game,
                            logs,
                            remaining_input,
                            public_render: new_public_render,
                            player_renders: new_player_renders,
                            ..
                        } => {
                            if remaining_input.trim() != "" {
                                output_nl();
                                output_error(format!("Unexpected: '{}'", remaining_input));
                                continue;
                            }
                            undo_stack.push(game);
                            game = new_game;
                            public_render = new_public_render;
                            player_renders = new_player_renders;
                            output_nl();
                            output_logs(logs, &players);
                        }
                        Response::SystemError { message } => {
                            output_nl();
                            panic!(message);
                        }
                        Response::UserError { message } => {
                            output_nl();
                            output_error(message);
                        }
                        _ => panic!("unexpected response"),
                    },
                }
            }
        }
    }
}

fn output_logs(logs: Vec<CliLog>, players: &[Player]) {
    for l in logs {
        let (content, _) = brdgme_markup::from_string(&l.content).unwrap();
        let mut l_line = vec![Node::Bold(vec![Node::text(format!("{}", l.at))])];
        l_line.push(Node::text(" - "));
        l_line.extend(content);
        output_nodes(&l_line, players);
    }
}

fn output_nodes(nodes: &[Node], players: &[Player]) {
    let (term_w, _) = term_size::dimensions().unwrap_or_default();
    print!(
        "{}",
        ansi(&from_lines(&to_lines(&transform(nodes, players))
            .iter()
            .map(|l| {
                let l_len = TNode::len(l);
                let mut l = l.to_owned();
                if l_len < term_w {
                    l.push(TNode::Bg(
                        *Style::default().bg,
                        vec![TNode::Text(repeat(" ").take(term_w - l_len).collect())],
                    ));
                }
                l
            })
            .collect::<Vec<Vec<TNode>>>()))
    );
}

fn output_error<I: Into<String>>(s: I) {
    output_nodes(
        &[Node::Bold(vec![Node::Fg(
            brdgme_color::RED.into(),
            vec![Node::text(s)],
        )])],
        &[],
    );
}

fn output_markup(markup: &str, players: &[Player]) {
    output_nodes(&brdgme_markup::from_string(markup).unwrap().0, players)
}

fn output_nl() {
    output_markup("", &[]);
}

fn prompt<'a, T>(s: T) -> String
where
    T: Into<Cow<'a, str>>,
{
    print!("{}: \x1b[K", s.into());
    stdout().flush().unwrap();
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_owned()
}
