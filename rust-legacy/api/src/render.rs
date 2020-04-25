use failure::{Error, ResultExt};

use brdgme_markup as markup;

use std::str::FromStr;

use db::models::{GamePlayerTypeUser, PublicGamePlayerTypeUser};
use db::color;

pub fn public_game_players_to_markup_players(
    game_players: &[PublicGamePlayerTypeUser],
) -> Result<Vec<markup::Player>, Error> {
    game_players
        .iter()
        .map(|gpu| {
            Ok(markup::Player {
                color: color::Color::from_str(&gpu.game_player.color)?.into(),
                name: gpu.user.name.to_owned(),
            })
        })
        .collect()
}

pub fn game_players_to_markup_players(
    game_players: &[GamePlayerTypeUser],
) -> Result<Vec<markup::Player>, Error> {
    public_game_players_to_markup_players(&game_players
        .iter()
        .map(|gptu| gptu.to_owned().into_public())
        .collect::<Vec<PublicGamePlayerTypeUser>>())
}

pub fn markup_html(template: &str, players: &[markup::Player]) -> Result<String, Error> {
    Ok(markup::html(&markup::transform(
        &markup::from_string(template)
            .context("failed to parse template")?
            .0,
        players,
    )))
}
