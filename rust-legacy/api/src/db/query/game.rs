use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;
use failure::{Error, ResultExt};

use db::models::*;

pub fn update_chat_id(
    game_id: &Uuid,
    chat_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<Game>, Error> {
    use db::schema::games;

    Ok(diesel::update(games::table.find(game_id))
        .set(games::chat_id.eq(chat_id))
        .get_result(conn)
        .optional()
        .context("error updating chat_id for game")?)
}

pub fn update_restarted_game_id(
    game_id: &Uuid,
    restarted_game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<Game>, Error> {
    use db::schema::games;

    Ok(diesel::update(games::table.find(game_id))
        .set(games::restarted_game_id.eq(restarted_game_id))
        .get_result(conn)
        .optional()
        .context("error updating restarted_game_id for game")?)
}
