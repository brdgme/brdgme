use std::borrow::Cow;
use std::fs::File;
use std::io::prelude::*;
use std::io::{stdin, stdout};

use brdgme_color::{LIGHT, Style};
use brdgme_game::Status;
use brdgme_game::command::doc;
use brdgme_markup::{self, Node, Player, TNode, ansi, from_lines, to_lines, transform};

use crate::api::{CliLog, GameResponse, PlayerRender, PubRender, Request, Response};
use crate::requester::Requester;

pub fn repl<T>(client: &mut T)
where
    T: Requester,
{
    if let Err(message) = run(client) {
        output_error(message);
    }
}

fn run<T: Requester>(client: &mut T) -> Result<(), String> {
    print!("{}", Style::default().ansi());
    let mut player_names: Vec<String> = vec![];
    loop {
        let Some(player) = prompt(format!(
            "Enter player {} (or blank to finish)",
            player_names.len() + 1
        )) else {
            return Ok(());
        };
        if player.is_empty() {
            break;
        }
        player_names.push(player);
    }
    let players = player_names
        .iter()
        .enumerate()
        .map(|(i, pn)| Player {
            name: pn.to_string(),
            color: LIGHT.player_color(i),
        })
        .collect::<Vec<Player>>();
    let (mut game, logs, mut public_render, mut player_renders) =
        match client.request(&Request::New {
            players: players.len(),
            seed: None,
        }) {
            Ok(Response::New {
                game,
                logs,
                public_render,
                player_renders,
                ..
            }) => (game, logs, public_render, player_renders),
            Ok(resp) => {
                output_nl();
                output_error(response_error_message(&resp, "new game request"));
                return Ok(());
            }
            Err(e) => return Err(e.to_string()),
        };
    output_nl();
    output_logs(logs, &players);
    let mut undo_stack: Vec<GameResponse> = vec![];
    loop {
        match game.status.clone() {
            Status::Finished { placings, .. } => {
                output_nl();
                match placings.as_slice() {
                    [] => {
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
                return Ok(());
            }
            Status::Active { ref whose_turn, .. } => {
                output_nl();
                if whose_turn.is_empty() {
                    output_nodes(&[Node::text("no player's turn, exiting")], &players);
                    return Ok(());
                }
                let current_player = whose_turn[0];
                output_markup(&player_renders[current_player].render, &players);
                println!();
                if let Some(ref spec) = player_renders[current_player].command_spec {
                    output_nl();
                    output_nodes(&doc::render(&spec.doc()), &players);
                }
                println!();
                let Some(input) =
                    prompt(ansi(&transform(&[Node::Player(current_player)], &players)))
                else {
                    return Ok(());
                };
                match input.as_ref() {
                    ":dump" | ":d" => println!("{:#?}", game),
                    ":json" => match serde_json::ser::to_string_pretty(&game) {
                        Ok(json) => println!("{}", json),
                        Err(e) => output_error(format!("could not serialize game: {}", e)),
                    },
                    ":save" => {
                        let json = match serde_json::ser::to_string_pretty(&game) {
                            Ok(json) => json,
                            Err(e) => {
                                output_error(format!("could not get game JSON: {}", e));
                                continue;
                            }
                        };
                        let mut file = match File::create("game.json") {
                            Ok(file) => file,
                            Err(e) => {
                                output_error(format!("could not create file: {}", e));
                                continue;
                            }
                        };
                        if let Err(e) = write!(file, "{}", json) {
                            output_error(format!("could not write to file: {}", e));
                        }
                    }
                    ":load" => {
                        let file = match File::open("game.json") {
                            Ok(file) => file,
                            Err(e) => {
                                output_error(format!("could not open file: {}", e));
                                continue;
                            }
                        };
                        match serde_json::from_reader(file) {
                            Ok(new_game) => {
                                game = new_game;
                                if let Err(message) = refresh_renders(
                                    client,
                                    &game,
                                    &mut public_render,
                                    &mut player_renders,
                                ) {
                                    output_error(message);
                                }
                            }
                            Err(e) => output_error(format!("could not read file JSON: {}", e)),
                        }
                    }
                    ":undo" | ":u" => {
                        if let Some(u) = undo_stack.pop() {
                            game = u;
                            if let Err(message) = refresh_renders(
                                client,
                                &game,
                                &mut public_render,
                                &mut player_renders,
                            ) {
                                output_error(message);
                            }
                        } else {
                            output_nodes(
                                &[Node::Bold(vec![Node::Fg(
                                    brdgme_color::NamedColor::Red.into(),
                                    vec![Node::text("No undos available")],
                                )])],
                                &players,
                            );
                        }
                    }
                    ":quit" | ":q" => return Ok(()),
                    _ => match client.request(&Request::Play {
                        player: current_player,
                        command: input,
                        names: player_names.clone(),
                        game: game.state.clone(),
                    }) {
                        Ok(Response::Play {
                            game: new_game,
                            logs,
                            remaining_input,
                            public_render: new_public_render,
                            player_renders: new_player_renders,
                            ..
                        }) => {
                            if !remaining_input.trim().is_empty() {
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
                        Ok(resp) => {
                            output_nl();
                            output_error(response_error_message(&resp, "play request"));
                        }
                        Err(e) => return Err(e.to_string()),
                    },
                }
            }
        }
    }
}

fn refresh_renders<T: Requester>(
    client: &mut T,
    game: &GameResponse,
    public_render: &mut PubRender,
    player_renders: &mut Vec<PlayerRender>,
) -> Result<(), String> {
    let response = client
        .request(&Request::Status {
            game: game.state.clone(),
        })
        .map_err(|e| e.to_string())?;
    match response {
        Response::Status {
            public_render: new_public_render,
            player_renders: new_player_renders,
            ..
        } => {
            *public_render = new_public_render;
            *player_renders = new_player_renders;
            Ok(())
        }
        resp => Err(response_error_message(&resp, "status request")),
    }
}

/// Renders the user-facing message for a non-success REPL response. `context`
/// names the request that produced it for the unexpected-response case. Used by
/// the new-game, play and status handlers so a normal `Response::UserError` (or
/// any other non-success variant) prints a message instead of panicking.
fn response_error_message(response: &Response, context: &str) -> String {
    match response {
        Response::UserError { message } | Response::SystemError { message } => message.clone(),
        r => format!("unexpected response to {}: {:?}", context, r),
    }
}

fn output_logs(logs: Vec<CliLog>, players: &[Player]) {
    for l in logs {
        let content = match brdgme_markup::from_string(&l.content) {
            Ok(nodes) => nodes,
            Err(_) => vec![Node::text(&l.content)],
        };
        let mut l_line = vec![Node::Bold(vec![Node::text(format!("{}", l.at))])];
        l_line.push(Node::text(" - "));
        l_line.extend(content);
        output_nodes(&l_line, players);
    }
}

fn output_nodes(nodes: &[Node], players: &[Player]) {
    let term_w = terminal_size::terminal_size().map_or(0, |(w, _)| w.0 as usize);
    print!(
        "{}",
        ansi(&from_lines(
            &to_lines(&transform(nodes, players))
                .iter()
                .map(|l| {
                    let l_len = TNode::len(l);
                    let mut l = l.to_owned();
                    if l_len < term_w {
                        l.push(TNode::Bg(
                            *Style::default().bg,
                            vec![TNode::Text(" ".repeat(term_w - l_len))],
                        ));
                    }
                    l
                })
                .collect::<Vec<Vec<TNode>>>()
        ))
    );
}

fn output_error<I: Into<String>>(s: I) {
    output_nodes(
        &[Node::Bold(vec![Node::Fg(
            brdgme_color::NamedColor::Red.into(),
            vec![Node::text(s)],
        )])],
        &[],
    );
}

fn output_markup(markup: &str, players: &[Player]) {
    let nodes = match brdgme_markup::from_string(markup) {
        Ok(nodes) => nodes,
        Err(_) => vec![Node::text(markup.to_string())],
    };
    output_nodes(&nodes, players)
}

fn output_nl() {
    output_markup("", &[]);
}

fn prompt<'a, T>(s: T) -> Option<String>
where
    T: Into<Cow<'a, str>>,
{
    print!("{}: \x1b[K", s.into());
    if stdout().flush().is_err() {
        return None;
    }
    let mut input = String::new();
    match stdin().read_line(&mut input) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(input.trim().to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_error_response_produces_message_without_panicking() {
        let message = response_error_message(
            &Response::UserError {
                message: "no more moves".to_string(),
            },
            "new game request",
        );
        assert!(message.contains("no more moves"), "got: {}", message);
    }
}
