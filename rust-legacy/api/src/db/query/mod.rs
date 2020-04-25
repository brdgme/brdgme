use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;
use rand::{self, Rng};
use chrono::{Duration, Utc};
use failure::{Error, ResultExt};

use brdgme_cmd::cli::CliLog;

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::usize::MAX as USIZE_MAX;
use std::cmp::Ordering;

use db::models::*;
use db::color::{self, Color};

#[cfg(test)]
use db::CONN;

pub mod chat;
pub mod game;

lazy_static! {
    static ref CONFIRMATION_EXPIRY: Duration = Duration::minutes(30);
    static ref TOKEN_EXPIRY: Duration = Duration::days(30);
    static ref FINISHED_GAME_RELEVANCE: Duration = Duration::days(3);
}

pub fn create_user_by_name(name: &str, conn: &PgConnection) -> Result<User, Error> {
    use db::schema::users;
    Ok(diesel::insert_into(users::table)
        .values(&NewUser {
            name: name,
            pref_colors: &[],
            login_confirmation: None,
            login_confirmation_at: None,
        })
        .get_result(conn)
        .context("error creating user")?)
}

pub fn find_user(find_id: &Uuid, conn: &PgConnection) -> Result<Option<User>, Error> {
    use db::schema::users;

    Ok(users::table
        .find(find_id)
        .first(conn)
        .optional()
        .context("error finding user")?)
}

pub fn find_user_by_email(
    by_email: &str,
    conn: &PgConnection,
) -> Result<Option<(UserEmail, User)>, Error> {
    use db::schema::{user_emails, users};

    Ok(user_emails::table
        .filter(user_emails::email.eq(by_email))
        .limit(1)
        .inner_join(users::table)
        .first::<(UserEmail, User)>(conn)
        .optional()
        .context("error finding user")?)
}

pub fn find_or_create_user_by_email(
    email: &str,
    conn: &PgConnection,
) -> Result<(UserEmail, User), Error> {
    if let Some(v) = find_user_by_email(email, conn)? {
        return Ok(v);
    }
    create_user_by_email(email, conn)
}

pub fn create_user_by_email(email: &str, conn: &PgConnection) -> Result<(UserEmail, User), Error> {
    conn.transaction(|| {
        let u = create_user_by_name(email, conn)?;
        let ue = create_user_email(
            &NewUserEmail {
                user_id: u.id,
                email: email,
                is_primary: true,
            },
            conn,
        )?;
        Ok((ue, u))
    })
}

pub fn create_user_email(ue: &NewUserEmail, conn: &PgConnection) -> Result<UserEmail, Error> {
    use db::schema::user_emails;
    Ok(diesel::insert_into(user_emails::table)
        .values(ue)
        .get_result(conn)
        .context("error creating user email")?)
}

fn rand_code() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}{:05}",
        (rng.gen::<usize>() % 9) + 1,
        rng.gen::<usize>() % 100000
    )
}

pub fn generate_user_login_confirmation(
    user_id: &Uuid,
    conn: &PgConnection,
) -> Result<String, Error> {
    use db::schema::users;

    let code = rand_code();
    diesel::update(users::table.find(user_id))
        .set((
            users::login_confirmation.eq(&code),
            users::login_confirmation_at.eq(Utc::now().naive_utc()),
        ))
        .execute(conn)?;
    Ok(code)
}

pub fn user_login_request(email: &str, conn: &PgConnection) -> Result<String, Error> {
    conn.transaction(|| {
        let (_, user) = find_or_create_user_by_email(email, conn)?;

        let confirmation = match (user.login_confirmation, user.login_confirmation_at) {
            (Some(ref uc), Some(at)) if at + *CONFIRMATION_EXPIRY > Utc::now().naive_utc() => {
                uc.to_owned()
            }
            _ => generate_user_login_confirmation(&user.id, conn)?,
        };
        Ok(confirmation)
    })
}

pub fn user_login_confirm(
    email: &str,
    confirmation: &str,
    conn: &PgConnection,
) -> Result<Option<UserAuthToken>, Error> {
    let user = match find_user_by_email(email, conn)? {
        Some((_, u)) => u,
        None => return Ok(None),
    };
    Ok(
        match (user.login_confirmation, user.login_confirmation_at) {
            (Some(ref uc), Some(at))
                if at + *CONFIRMATION_EXPIRY > Utc::now().naive_utc() && uc == confirmation =>
            {
                Some(create_auth_token(&user.id, conn)?)
            }
            _ => None,
        },
    )
}

pub fn create_auth_token(for_user_id: &Uuid, conn: &PgConnection) -> Result<UserAuthToken, Error> {
    use db::schema::user_auth_tokens;

    Ok(diesel::insert_into(user_auth_tokens::table)
        .values(&NewUserAuthToken {
            user_id: *for_user_id,
        })
        .get_result::<UserAuthToken>(conn)
        .context("error creating auth token")?)
}

pub fn authenticate(search_token: &Uuid, conn: &PgConnection) -> Result<Option<User>, Error> {
    use db::schema::{user_auth_tokens, users};

    let uat: UserAuthToken = match user_auth_tokens::table
        .find(search_token)
        .filter(user_auth_tokens::created_at.gt(Utc::now().naive_utc() - *TOKEN_EXPIRY))
        .first(conn)
        .optional()?
    {
        Some(v) => v,
        None => return Ok(None),
    };

    Ok(Some(users::table
        .find(uat.user_id)
        .first(conn)
        .context("error finding user")?))
}

pub fn find_valid_user_auth_tokens_for_users(
    user_ids: &[Uuid],
    conn: &PgConnection,
) -> Result<Vec<UserAuthToken>, Error> {
    use db::schema::user_auth_tokens;

    Ok(user_auth_tokens::table
        .filter(user_auth_tokens::user_id.eq_any(user_ids))
        .filter(user_auth_tokens::created_at.gt(Utc::now().naive_utc() - *TOKEN_EXPIRY))
        .get_results(conn)
        .context("error finding user auth tokens for user")?)
}

pub fn find_game(id: &Uuid, conn: &PgConnection) -> Result<Game, Error> {
    use db::schema::games;

    Ok(games::table
        .find(id)
        .first(conn)
        .context("error finding game")?)
}

pub fn find_game_version(id: &Uuid, conn: &PgConnection) -> Result<Option<GameVersion>, Error> {
    use db::schema::game_versions;

    Ok(game_versions::table
        .find(id)
        .first(conn)
        .optional()
        .context("error finding game version")?)
}

pub fn find_game_with_version(
    id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<(Game, GameVersion)>, Error> {
    use db::schema::{game_versions, games};

    Ok(games::table
        .find(id)
        .inner_join(game_versions::table)
        .first(conn)
        .optional()
        .context("error finding game")?)
}

#[derive(Clone)]
pub struct GameExtended {
    pub game: Game,
    pub game_type: GameType,
    pub game_version: GameVersion,
    pub game_players: Vec<GamePlayerTypeUser>,
    pub chat: Option<chat::ChatExtended>,
}

impl GameExtended {
    pub fn into_public(self) -> PublicGameExtended {
        PublicGameExtended {
            game: self.game.into_public(),
            game_type: self.game_type,
            game_version: self.game_version.into_public(),
            game_player: None,
            game_players: self.game_players
                .into_iter()
                .map(|gptu| gptu.into_public())
                .collect(),
            chat: self.chat.map(|c| c.into_public()),
        }
    }

    pub fn into_public_for_user(self, user_id: &Uuid) -> PublicGameExtended {
        let mut p = self.into_public();
        p.game_player = p.game_players
            .iter()
            .find(|gp| gp.user.id == *user_id)
            .map(|gp| gp.game_player.clone());
        p
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct PublicGameExtended {
    pub game: PublicGame,
    pub game_type: PublicGameType,
    pub game_version: PublicGameVersion,
    pub game_player: Option<PublicGamePlayer>,
    pub game_players: Vec<PublicGamePlayerTypeUser>,
    pub chat: Option<chat::PublicChatExtended>,
}

pub fn find_active_games_for_user(
    id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<GameExtended>, Error> {
    use db::schema::{game_players, game_types, game_versions, games};

    Ok(games::table
        .inner_join(game_players::table)
        .filter(game_players::user_id.eq(id))
        .filter(
            games::is_finished.eq(false).or(games::updated_at
                .gt(Utc::now().naive_utc() - *FINISHED_GAME_RELEVANCE)
                .and(game_players::is_read.eq(false))),
        )
        .get_results::<(Game, GamePlayer)>(conn)?
        .iter()
        .map(|&(ref game, _)| {
            let game_version: GameVersion = game_versions::table
                .find(game.game_version_id)
                .get_result(conn)?;
            let game_type: GameType = game_types::table
                .find(game_version.game_type_id)
                .get_result(conn)?;
            let players = find_game_player_type_users_by_game(&game.id, conn)?;
            Ok(GameExtended {
                game: game.clone(),
                game_type: game_type,
                game_version: game_version,
                game_players: players,
                chat: game.chat_id.map(|chat_id| {
                    chat::find_extended(&chat_id, conn).expect("error finding chat for game")
                }),
            })
        })
        .collect::<Result<Vec<GameExtended>, Error>>()?)
}

pub fn find_game_extended(id: &Uuid, conn: &PgConnection) -> Result<GameExtended, Error> {
    use db::schema::{game_types, game_versions, games};

    let (game, game_version) = games::table
        .find(id)
        .inner_join(game_versions::table)
        .get_result::<(Game, GameVersion)>(conn)?;
    let game_type: GameType = game_types::table
        .find(game_version.game_type_id)
        .get_result(conn)?;
    let players = find_game_player_type_users_by_game(&game.id, conn)?;
    Ok(GameExtended {
        game: game.clone(),
        game_type: game_type,
        game_version: game_version,
        game_players: players,
        chat: game.chat_id.map(|chat_id| {
            chat::find_extended(&chat_id, conn).expect("error finding chat for game")
        }),
    })
}

pub struct CreatedGame {
    pub game: Game,
    pub opponents: Vec<(UserEmail, User)>,
    pub players: Vec<GamePlayer>,
}
pub struct CreateGameOpts<'a> {
    pub new_game: &'a NewGame<'a>,
    pub whose_turn: &'a [usize],
    pub eliminated: &'a [usize],
    pub placings: &'a [usize],
    pub points: &'a [f32],
    pub creator_id: &'a Uuid,
    pub opponent_ids: &'a [Uuid],
    pub opponent_emails: &'a [String],
    pub chat_id: Option<Uuid>,
}
pub fn create_game_with_users(
    opts: &CreateGameOpts,
    conn: &PgConnection,
) -> Result<CreatedGame, Error> {
    // We get the timestamp for now before logs are created to make sure players can read them.
    let now = Utc::now().naive_utc();
    conn.transaction(|| {
        // Find or create users.
        let creator = find_user(opts.creator_id, conn)
            .context("could not find creator")?
            .ok_or_else::<Error, _>(|| format_err!("could not find creator"))?;
        let opponents = create_game_users(opts.opponent_ids, opts.opponent_emails, conn)
            .context("could not create game users")?;
        let mut users: Vec<User> = opponents.iter().map(|&(_, ref u)| u.clone()).collect();
        users.push(creator);

        // Randomise the users so player order is random.
        let mut rnd = rand::thread_rng();
        rnd.shuffle(&mut users);

        // Assign colors to each player using preferences.
        let color_prefs = users
            .iter()
            .map(|u| Color::from_strings(&u.pref_colors))
            .collect::<Result<Vec<Vec<Color>>, Error>>()?;
        let player_colors = color::choose(&HashSet::from_iter(color::COLORS.iter()), &color_prefs);

        // Create game record.
        let mut game_record =
            create_game(opts.new_game, conn).context("could not create new game")?;

        // Create chat if needed
        let chat_id = if let Some(chat_id) = opts.chat_id {
            chat_id
        } else {
            let chat = chat::create(conn).context("error creating chat for new game")?;
            chat::add_users(
                chat.id,
                users.iter().map(|u| u.id).collect::<Vec<Uuid>>().as_ref(),
                conn,
            ).context("error adding users to new chat")?;
            chat.id
        };
        game::update_chat_id(&game_record.id, &chat_id, conn)
            .context("error updating chat_id for newly created game")?;
        game_record.chat_id = Some(chat_id);

        // Find or create game type user records.
        let game_version = find_game_version(&opts.new_game.game_version_id, conn)?
            .ok_or_else::<Error, _>(|| format_err!("could not find game version"))?;
        let mut game_type_users: Vec<GameTypeUser> = vec![];
        for user in &users {
            game_type_users.push(find_or_create_game_type_user(
                &game_version.game_type_id,
                &user.id,
                conn,
            )?);
        }

        // Create a player record for each user.
        let mut players: Vec<GamePlayer> = vec![];
        for (pos, user) in users.iter().enumerate() {
            players.push(create_game_player(
                &NewGamePlayer {
                    game_id: game_record.id,
                    user_id: user.id,
                    position: pos as i32,
                    color: &player_colors[pos].to_string(),
                    has_accepted: user.id == *opts.creator_id,
                    is_turn: opts.whose_turn.contains(&pos),
                    is_turn_at: now,
                    last_turn_at: now,
                    is_eliminated: opts.eliminated.contains(&pos),
                    is_read: false,
                    points: opts.points.get(pos).cloned(),
                    undo_game_state: None,
                    place: opts.placings.get(pos).map(|p| *p as i32),
                    rating_change: None,
                },
                conn,
            ).context("could not create game player")?);
        }
        Ok(CreatedGame {
            game: game_record,
            opponents: opponents,
            players: players,
        })
    })
}

pub fn find_or_create_game_type_user(
    game_type_id: &Uuid,
    user_id: &Uuid,
    conn: &PgConnection,
) -> Result<GameTypeUser, Error> {
    if let Some(gtu) = find_game_type_user_by_game_type_and_user(game_type_id, user_id, conn)? {
        return Ok(gtu);
    }
    create_game_type_user(
        &NewGameTypeUser {
            game_type_id: game_type_id.to_owned(),
            user_id: user_id.to_owned(),
            last_game_finished_at: None,
            rating: None,
            peak_rating: None,
        },
        conn,
    )
}

pub fn find_game_type_user_by_game_type_and_user(
    game_type_id: &Uuid,
    user_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<GameTypeUser>, Error> {
    use db::schema::game_type_users;
    Ok(game_type_users::table
        .filter(game_type_users::game_type_id.eq(game_type_id))
        .filter(game_type_users::user_id.eq(user_id))
        .get_result(conn)
        .optional()
        .context("error finding game type user")?)
}

pub fn create_game_type_user(
    gtu: &NewGameTypeUser,
    conn: &PgConnection,
) -> Result<GameTypeUser, Error> {
    use db::schema::game_type_users;
    Ok(diesel::insert_into(game_type_users::table)
        .values(gtu)
        .get_result(conn)
        .context("error inserting new game type user")?)
}

pub fn player_can_undo_set_undo_game_state(
    game_id: &Uuid,
    game_player_id: &Uuid,
    game_state: &str,
    conn: &PgConnection,
) -> Result<(), Error> {
    use db::schema::game_players;
    conn.transaction(|| {
        diesel::update(
            game_players::table
                .find(game_player_id)
                .filter(game_players::undo_game_state.is_null()),
        ).set(game_players::undo_game_state.eq(game_state))
            .execute(conn)
            .context("error updating game player undo_game_state to game_state")?;
        diesel::update(
            game_players::table
                .filter(game_players::game_id.eq(game_id))
                .filter(game_players::id.ne(game_player_id)),
        ).set(game_players::undo_game_state.eq(None::<String>))
            .execute(conn)
            .context("error update game players undo_game_state to None")?;
        Ok(())
    })
}

pub fn player_cannot_undo_set_undo_game_state(
    game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<GamePlayer>, Error> {
    use db::schema::game_players;
    Ok(
        diesel::update(game_players::table.filter(game_players::game_id.eq(game_id)))
            .set(game_players::undo_game_state.eq(None::<String>))
            .get_results(conn)
            .context("error updating game players undo_game_state to None")?,
    )
}


pub struct UpdatedGame {
    pub game: Option<Game>,
    pub whose_turn: Vec<GamePlayer>,
    pub eliminated: Vec<GamePlayer>,
    pub placings: Vec<GamePlayer>,
    pub is_read: Vec<GamePlayer>,
    pub game_type_users: Vec<GameTypeUser>,
}
pub fn update_game_command_success(
    game_id: &Uuid,
    game_player_id: &Uuid,
    update: &NewGame,
    undo_game_state: Option<&str>,
    whose_turn: &[usize],
    eliminated: &[usize],
    placings: &[usize],
    points: &[f32],
    conn: &PgConnection,
) -> Result<UpdatedGame, Error> {
    conn.transaction(|| {
        if let Some(game_state) = undo_game_state {
            player_can_undo_set_undo_game_state(game_id, game_player_id, game_state, conn)?;
        } else {
            player_cannot_undo_set_undo_game_state(game_id, conn)?;
        }
        update_game_points(game_id, points, conn)?;
        let (placings, game_type_users) = update_game_placings(game_id, placings, conn)?;
        Ok(UpdatedGame {
            game: update_game(game_id, update, conn)?,
            whose_turn: update_game_whose_turn(game_id, whose_turn, conn)?,
            eliminated: update_game_eliminated(game_id, eliminated, conn)?,
            placings,
            is_read: update_game_is_read(game_id, &[*game_player_id], conn)?,
            game_type_users,
        })
    })
}

pub fn concede_game(
    game_id: &Uuid,
    game_player_id: &Uuid,
    conn: &PgConnection,
) -> Result<UpdatedGame, Error> {
    conn.transaction(|| {
        let game_players = find_game_players_by_game(game_id, conn)
            .context("unable to find game players for concede")?;
        let placings: Vec<usize> = game_players
            .iter()
            .map(|gp| if gp.id != *game_player_id { 1 } else { 2 })
            .collect();
        let (placings, game_type_users) = update_game_placings(game_id, &placings, conn)?;
        Ok(UpdatedGame {
            game: update_game_is_finished(game_id, true, conn)?,
            whose_turn: update_game_whose_turn(game_id, &[], conn)?,
            eliminated: vec![],
            placings,
            is_read: update_game_is_read(game_id, &[*game_player_id], conn)?,
            game_type_users,
        })
    })
}

pub fn update_game_is_finished(
    game_id: &Uuid,
    is_finished: bool,
    conn: &PgConnection,
) -> Result<Option<Game>, Error> {
    use db::schema::games;
    Ok(diesel::update(games::table.find(game_id))
        .set(games::is_finished.eq(is_finished))
        .get_result(conn)
        .optional()
        .context("error updating game is_finished")?)
}

pub fn find_player_count_by_game(game_id: &Uuid, conn: &PgConnection) -> Result<i64, Error> {
    use diesel::dsl::count;
    use db::schema::game_players;

    Ok(game_players::table
        .select(count(game_players::id))
        .filter(game_players::game_id.eq(game_id))
        .get_result(conn)
        .context("error getting player count")?)
}

fn to_i32_vec(from: &[usize]) -> Vec<i32> {
    from.iter().map(|p| *p as i32).collect::<Vec<i32>>()
}

pub fn update_game(
    update_id: &Uuid,
    update: &NewGame,
    conn: &PgConnection,
) -> Result<Option<Game>, Error> {
    use db::schema::games;
    Ok(diesel::update(games::table.find(update_id))
        .set((
            games::game_version_id.eq(update.game_version_id),
            games::is_finished.eq(update.is_finished),
            games::game_state.eq(update.game_state),
        ))
        .get_result(conn)
        .optional()
        .context("error updating game")?)
}

pub fn mark_game_read(
    id: &Uuid,
    user_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<GamePlayer>, Error> {
    use db::schema::game_players;
    Ok(diesel::update(
        game_players::table
            .filter(game_players::game_id.eq(id))
            .filter(game_players::user_id.eq(user_id)),
    ).set((game_players::is_read.eq(true),))
        .get_result(conn)
        .optional()
        .context("error marking game as read")?)
}

pub fn update_game_whose_turn(
    id: &Uuid,
    positions: &[usize],
    conn: &PgConnection,
) -> Result<Vec<GamePlayer>, Error> {
    use db::schema::game_players;

    Ok(
        diesel::update(game_players::table.filter(game_players::game_id.eq(id)))
            .set(game_players::is_turn.eq(game_players::position.eq_any(to_i32_vec(positions))))
            .get_results(conn)
            .context("error updating game players")?,
    )
}

pub fn update_game_points(
    id: &Uuid,
    points: &[f32],
    conn: &PgConnection,
) -> Result<Vec<GamePlayer>, Error> {
    Ok(points
        .iter()
        .enumerate()
        .filter_map(|(pos, pts)| {
            update_game_points_for_position(id, pos as i32, *pts, conn).unwrap()
        })
        .collect())
}

pub fn update_game_points_for_position(
    id: &Uuid,
    position: i32,
    points: f32,
    conn: &PgConnection,
) -> Result<Option<GamePlayer>, Error> {
    use db::schema::game_players;

    Ok(diesel::update(
        game_players::table
            .filter(game_players::game_id.eq(id))
            .filter(game_players::position.eq(position)),
    ).set(game_players::points.eq(points))
        .get_result(conn)
        .optional()
        .context("error updating game player points")?)
}

pub fn update_game_eliminated(
    id: &Uuid,
    positions: &[usize],
    conn: &PgConnection,
) -> Result<Vec<GamePlayer>, Error> {
    use db::schema::game_players;

    Ok(
        diesel::update(game_players::table.filter(game_players::game_id.eq(id)))
            .set(
                game_players::is_eliminated
                    .eq(game_players::position.eq_any(to_i32_vec(positions))),
            )
            .get_results(conn)
            .context("error updating game players")?,
    )
}

pub fn update_game_placings(
    game_id: &Uuid,
    placings: &[usize],
    conn: &PgConnection,
) -> Result<(Vec<GamePlayer>, Vec<GameTypeUser>), Error> {
    let placings: Vec<usize> = placings.to_owned();

    conn.transaction(|| {
        let game = find_game(game_id, conn)?;
        let game_version = find_game_version(&game.game_version_id, conn)?
            .ok_or_else::<Error, _>(|| format_err!("could not find game version for game"))?;
        let game_players = find_game_players_by_game(game_id, conn)?;
        let mut rating_changes: HashMap<usize, i32> = HashMap::new();

        let mut updated_game_type_users: Vec<GameTypeUser> = vec![];

        // We only update ratings if placings are provided and there haven't been any rating changes
        // yet.
        let update_ratings: bool = !placings.is_empty()
            && game_players
                .iter()
                .find(|gp| gp.rating_change.is_some())
                .is_none();
        if update_ratings {
            // We grab existing game type users up front.
            let game_player_type_users: Vec<(&GamePlayer, GameTypeUser)> = game_players
                .iter()
                .map(|gp| {
                    rating_changes.insert(gp.position as usize, 0);
                    Ok((
                        gp,
                        find_or_create_game_type_user(
                            &game_version.game_type_id,
                            &gp.user_id,
                            conn,
                        )?,
                    ))
                })
                .collect::<Result<Vec<(&GamePlayer, GameTypeUser)>, Error>>()?;

            // Iterate and calculate an adjustment against each opponent.
            for (a_index, &(a_gp, ref a_gtu)) in game_player_type_users
                .iter()
                .take(game_player_type_users.len() - 1)
                .enumerate()
            {
                for &(b_gp, ref b_gtu) in game_player_type_users.iter().skip(a_index + 1) {
                    let a_score: f32 = match placings
                        .get(a_gp.position as usize)
                        .cloned()
                        .unwrap_or(USIZE_MAX)
                        .cmp(&placings
                            .get(b_gp.position as usize)
                            .cloned()
                            .unwrap_or(USIZE_MAX))
                    {
                        Ordering::Less => 1.0,
                        Ordering::Equal => 0.5,
                        Ordering::Greater => 0.0,
                    };
                    let rating_change = elo_rating_change(a_gtu.rating, b_gtu.rating, a_score);
                    *rating_changes.entry(a_gp.position as usize).or_insert(0) += rating_change;
                    *rating_changes.entry(b_gp.position as usize).or_insert(0) -= rating_change;
                }
            }

            // Save the adjusted scores back to the game type users
            for &(gp, ref gtu) in &game_player_type_users {
                let rating_change = rating_changes
                    .get(&(gp.position as usize))
                    .cloned()
                    .unwrap_or(0);
                if rating_change == 0 {
                    continue;
                }
                if let Some(ugtu) =
                    update_game_type_user_rating(&gtu.id, gtu.rating + rating_change, conn)?
                {
                    updated_game_type_users.push(ugtu)
                }
            }
        }

        Ok((
            placings
                .iter()
                .enumerate()
                .filter_map(|(pos, place)| {
                    update_game_player_result(
                        game_id,
                        pos,
                        *place,
                        rating_changes.get(&pos).cloned(),
                        conn,
                    ).unwrap()
                })
                .collect(),
            updated_game_type_users,
        ))
    })
}

fn update_game_type_user_rating(
    id: &Uuid,
    rating: i32,
    conn: &PgConnection,
) -> Result<Option<GameTypeUser>, Error> {
    use db::schema::game_type_users;

    Ok(diesel::update(game_type_users::table.find(id))
        .set(game_type_users::rating.eq(rating))
        .get_result(conn)
        .optional()
        .context("unable to update game type user rating")?)
}

const ELO_K: f32 = 32.0;
fn elo_rating_change(a_rating: i32, b_rating: i32, a_score: f32) -> i32 {
    let a_expected = elo_expected_score(a_rating, b_rating);
    (ELO_K * (a_score - a_expected)).round() as i32
}

fn elo_transformed_rating(rating: i32) -> f32 {
    10f32.powf(rating as f32 / 400.0)
}

fn elo_expected_score(a_rating: i32, b_rating: i32) -> f32 {
    let a_trans = elo_transformed_rating(a_rating);
    let b_trans = elo_transformed_rating(b_rating);
    a_trans / (a_trans + b_trans)
}

pub fn update_game_player_result(
    game_id: &Uuid,
    position: usize,
    place: usize,
    rating_change: Option<i32>,
    conn: &PgConnection,
) -> Result<Option<GamePlayer>, Error> {
    use db::schema::game_players;

    Ok(diesel::update(
        game_players::table
            .filter(game_players::game_id.eq(game_id))
            .filter(game_players::position.eq(position as i32)),
    ).set((
        game_players::place.eq(place as i32),
        game_players::rating_change.eq(rating_change),
    ))
        .get_result(conn)
        .optional()
        .context("error updating place for game player")?)
}

pub fn update_game_is_read(
    id: &Uuid,
    game_player_ids: &[Uuid],
    conn: &PgConnection,
) -> Result<Vec<GamePlayer>, Error> {
    use db::schema::game_players;

    Ok(
        diesel::update(game_players::table.filter(game_players::game_id.eq(id)))
            .set(game_players::is_read.eq(game_players::id.eq_any(game_player_ids)))
            .get_results(conn)
            .context("error updating game players")?,
    )
}

pub fn create_game_logs_from_cli(
    game_id: &Uuid,
    logs: Vec<CliLog>,
    conn: &PgConnection,
) -> Result<Vec<CreatedGameLog>, Error> {
    conn.transaction(|| {
        let mut player_id_by_position: HashMap<usize, Uuid> = HashMap::new();
        for p in find_game_players_by_game(game_id, conn)? {
            player_id_by_position.insert(p.position as usize, p.id);
        }
        let mut created: Vec<CreatedGameLog> = vec![];
        for l in logs {
            let mut player_to: Vec<Uuid> = vec![];
            for t in l.to {
                player_to.push(
                    player_id_by_position
                        .get(&t)
                        .ok_or_else::<Error, _>(|| {
                            format_err!("no player with that position exists")
                        })?
                        .to_owned(),
                );
            }
            created.push(create_game_log(
                &NewGameLog {
                    game_id: *game_id,
                    body: &l.content,
                    is_public: l.public,
                    logged_at: l.at,
                },
                &player_to,
                conn,
            )?);
        }
        Ok(created)
    })
}

pub fn find_game_players_by_game(
    game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<GamePlayer>, Error> {
    use db::schema::game_players;

    Ok(game_players::table
        .filter(game_players::game_id.eq(game_id))
        .order(game_players::position)
        .get_results(conn)
        .context("error finding players")?)
}

pub fn find_game_player_by_user_and_game(
    user_id: &Uuid,
    game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<GamePlayer>, Error> {
    use db::schema::game_players;

    Ok(game_players::table
        .filter(game_players::user_id.eq(user_id))
        .filter(game_players::game_id.eq(game_id))
        .order(game_players::position)
        .get_result(conn)
        .optional()
        .context("error finding player")?)
}

pub fn find_game_player_type_users_by_game(
    game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<GamePlayerTypeUser>, Error> {
    use db::schema::{game_players, game_versions, games, users};

    let (game_version, _) = game_versions::table
        .inner_join(games::table)
        .filter(games::id.eq(game_id))
        .get_result::<(GameVersion, Game)>(conn)
        .context("error finding game version")?;

    game_players::table
        .filter(game_players::game_id.eq(game_id))
        .order(game_players::position)
        .inner_join(users::table)
        .get_results::<(GamePlayer, User)>(conn)
        .context("error finding game players")?
        .into_iter()
        .map(|(gp, u)| {
            let gtu = find_or_create_game_type_user(&game_version.game_type_id, &u.id, conn)?;
            Ok(GamePlayerTypeUser {
                game_player: gp,
                user: u,
                game_type_user: gtu,
            })
        })
        .collect()
}

pub fn find_game_players_with_user_by_game(
    game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<(GamePlayer, User)>, Error> {
    use db::schema::{game_players, users};

    Ok(game_players::table
        .filter(game_players::game_id.eq(game_id))
        .order(game_players::position)
        .inner_join(users::table)
        .get_results(conn)
        .context("error finding game players")?)
}

#[derive(Debug, PartialEq, Clone)]
pub struct CreatedGameLog {
    pub game_log: GameLog,
    pub targets: Vec<GameLogTarget>,
}
pub fn create_game_log(
    log: &NewGameLog,
    to: &[Uuid],
    conn: &PgConnection,
) -> Result<CreatedGameLog, Error> {
    use db::schema::game_logs;
    conn.transaction(|| {
        let created_log: GameLog = diesel::insert_into(game_logs::table)
            .values(log)
            .get_result(conn)?;
        let clid = created_log.id;
        Ok(CreatedGameLog {
            game_log: created_log,
            targets: create_game_log_targets(&clid, to, conn)?,
        })
    })
}

pub fn create_game_log_targets(
    log_id: &Uuid,
    player_ids: &[Uuid],
    conn: &PgConnection,
) -> Result<Vec<GameLogTarget>, Error> {
    conn.transaction(|| {
        let mut created = vec![];
        for id in player_ids {
            created.push(create_game_log_target(
                &NewGameLogTarget {
                    game_log_id: *log_id,
                    game_player_id: *id,
                },
                conn,
            )?);
        }
        Ok(created)
    })
}

pub fn create_game_log_target(
    new_target: &NewGameLogTarget,
    conn: &PgConnection,
) -> Result<GameLogTarget, Error> {
    use db::schema::game_log_targets;

    Ok(diesel::insert_into(game_log_targets::table)
        .values(new_target)
        .get_result(conn)
        .context("error inserting game log target")?)
}

pub fn create_game_users(
    ids: &[Uuid],
    emails: &[String],
    conn: &PgConnection,
) -> Result<Vec<(UserEmail, User)>, Error> {
    conn.transaction(|| {
        let mut users: Vec<(UserEmail, User)> = vec![];
        for id in ids.iter() {
            users.push(find_user_with_primary_email(id, conn)?
                .ok_or_else::<Error, _>(|| format_err!("unable to find user"))?);
        }
        for email in emails.iter() {
            users.push(match find_user_with_primary_email_by_email(email, conn)? {
                Some(ube) => ube,
                None => create_user_by_email(email, conn)?,
            });
        }
        Ok(users)
    })
}

pub fn find_user_with_primary_email(
    find_user_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<(UserEmail, User)>, Error> {
    use db::schema::{user_emails, users};

    Ok(user_emails::table
        .filter(user_emails::user_id.eq(find_user_id))
        .filter(user_emails::is_primary.eq(true))
        .inner_join(users::table)
        .first(conn)
        .optional()
        .context("error finding user")?)
}

pub fn find_user_with_primary_email_by_email(
    search_email: &str,
    conn: &PgConnection,
) -> Result<Option<(UserEmail, User)>, Error> {
    use db::schema::{user_emails, users};

    Ok(match user_emails::table
        .filter(user_emails::email.eq(search_email))
        .first::<UserEmail>(conn)
        .optional()?
    {
        Some(ue) => Some(user_emails::table
            .filter(user_emails::user_id.eq(ue.user_id))
            .filter(user_emails::is_primary.eq(true))
            .inner_join(users::table)
            .first(conn)?),
        None => return Ok(None),
    })
}

pub fn create_game(new_game: &NewGame, conn: &PgConnection) -> Result<Game, Error> {
    use db::schema::games;

    Ok(diesel::insert_into(games::table)
        .values(new_game)
        .get_result(conn)
        .context("error inserting game")?)
}

pub fn create_game_version(
    new_game_version: &NewGameVersion,
    conn: &PgConnection,
) -> Result<GameVersion, Error> {
    use db::schema::game_versions;

    Ok(diesel::insert_into(game_versions::table)
        .values(new_game_version)
        .get_result(conn)
        .context("error inserting game version")?)
}

pub fn create_game_type(
    new_game_type: &NewGameType,
    conn: &PgConnection,
) -> Result<GameType, Error> {
    use db::schema::game_types;

    Ok(diesel::insert_into(game_types::table)
        .values(new_game_type)
        .get_result(conn)
        .context("error inserting game type")?)
}

pub fn create_game_players(
    players: &[NewGamePlayer],
    conn: &PgConnection,
) -> Result<Vec<GamePlayer>, Error> {
    conn.transaction(|| {
        let mut created: Vec<GamePlayer> = vec![];
        for p in players.iter() {
            created.push(create_game_player(p, conn)?);
        }
        Ok(created)
    })
}

pub fn create_game_player(
    player: &NewGamePlayer,
    conn: &PgConnection,
) -> Result<GamePlayer, Error> {
    use db::schema::game_players;

    Ok(diesel::insert_into(game_players::table)
        .values(player)
        .get_result(conn)
        .context("error inserting game player")?)
}

pub fn public_game_versions(conn: &PgConnection) -> Result<Vec<GameVersionType>, Error> {
    use db::schema::{game_types, game_versions};

    Ok(game_versions::table
        .filter(game_versions::is_public.eq(true))
        .filter(game_versions::is_deprecated.eq(false))
        .inner_join(game_types::table)
        .get_results::<(GameVersion, GameType)>(conn)
        .context("error finding game versions")?
        .into_iter()
        .map(|(game_version, game_type)| {
            GameVersionType {
                game_version,
                game_type,
            }
        })
        .collect())
}

pub fn find_public_game_logs_for_game(
    game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<GameLog>, Error> {
    use db::schema::game_logs;

    Ok(game_logs::table
        .filter(game_logs::game_id.eq(game_id))
        .filter(game_logs::is_public.eq(true))
        .order(game_logs::logged_at)
        .get_results(conn)
        .context("error finding game logs")?)
}

pub fn find_game_logs_for_player(
    game_player_id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<GameLog>, Error> {
    use db::schema::{game_log_targets, game_logs, game_players};

    let game_player: GamePlayer = game_players::table.find(game_player_id).get_result(conn)?;
    Ok(game_logs::table
        .left_outer_join(game_log_targets::table)
        .filter(game_logs::game_id.eq(game_player.game_id))
        .filter(
            game_logs::is_public
                .eq(true)
                .or(game_log_targets::game_player_id.eq(game_player_id)),
        )
        .order(game_logs::logged_at)
        .get_results::<(GameLog, Option<GameLogTarget>)>(conn)
        .context("error finding game logs")?
        .iter()
        .map(|&(ref gl, _)| gl.clone())
        .collect())
}

#[cfg(test)]
fn with_db<F>(closure: F)
where
    F: Fn(&PgConnection),
{
    let conn = &CONN.w.get().unwrap();
    conn.test_transaction::<_, Error, _>(|| {
        closure(conn);
        Ok(())
    });
}

#[cfg(test)]
fn create_test_game(players: usize, conn: &PgConnection) -> GameExtended {
    let mut users = vec![];
    for p in 0..players {
        users.push(
            create_user_by_email(&format!("{}", p), conn)
                .expect("expected to create user by name")
                .1,
        );
    }
    let game_type = create_game_type(
        &NewGameType {
            name: "Test Game",
            player_counts: vec![players as i32],
            weight: 1.52,
        },
        conn,
    ).expect("expected to create game type");
    let game_version = create_game_version(
        &NewGameVersion {
            game_type_id: game_type.id,
            uri: "https://example.com/test-game-1",
            name: "v1",
            is_public: true,
            is_deprecated: false,
        },
        conn,
    ).expect("expected to create game version");
    let created_game = create_game_with_users(
        &CreateGameOpts {
            new_game: &NewGame {
                game_version_id: game_version.id,
                is_finished: false,
                game_state: "",
            },
            whose_turn: &[0],
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id: &users[0].id,
            opponent_ids: users
                .iter()
                .skip(1)
                .map(|p| p.id)
                .collect::<Vec<Uuid>>()
                .as_ref(),
            opponent_emails: &[],
            chat_id: None,
        },
        conn,
    ).expect("expected to create game");
    find_game_extended(&created_game.game.id, conn).expect("expected to find game")
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::color::Color;

    #[test]
    fn rand_code_works() {
        for _ in 1..100000 {
            let n: usize = rand_code().parse().unwrap();
            assert!(n > 99999, "n <= 99999");
            assert!(n < 1000000, "n >= 1000000");
        }
    }
    #[test]
    #[ignore]
    fn create_user_by_name_works() {
        with_db(|conn| {
            create_user_by_name("beefsack", conn).unwrap();
        });
    }

    #[test]
    #[ignore]
    fn find_user_works() {
        with_db(|conn| {
            assert_eq!(find_user(&Uuid::new_v4(), conn).unwrap(), None);
            let u = create_user_by_name("beefsack", conn).unwrap();
            assert!(find_user(&u.id, conn).unwrap().is_some());
        });
    }

    #[test]
    #[ignore]
    fn create_user_email_works() {
        with_db(|conn| {
            assert_eq!(find_user(&Uuid::new_v4(), conn).unwrap(), None);
            let u = create_user_by_name("beefsack", conn).unwrap();
            assert!(
                create_user_email(
                    &NewUserEmail {
                        user_id: u.id,
                        email: "beefsack@gmail.com",
                        is_primary: true,
                    },
                    conn,
                ).is_ok()
            );
        });
    }

    #[test]
    #[ignore]
    fn login_works() {
        with_db(|conn| {
            let confirmation = user_login_request("beefsack@gmail.com", conn).unwrap();
            let uat = user_login_confirm("beefsack@gmail.com", &confirmation, conn)
                .expect("error confirming auth")
                .expect("invalid confirm code");
            assert!(authenticate(&uat.id, conn).unwrap().is_some());
        });
    }

    #[test]
    #[ignore]
    fn find_user_with_primary_email_works() {
        with_db(|conn| {
            let (user_email, user) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            let (_, found_user) = find_user_with_primary_email(&user.id, conn)
                .unwrap()
                .unwrap();
            assert_eq!(user.id, found_user.id);
            assert_eq!("beefsack@gmail.com", user_email.email);
        });
    }

    #[test]
    #[ignore]
    fn find_user_with_primary_email_by_email_works() {
        with_db(|conn| {
            let (user_email, user) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            create_user_email(
                &NewUserEmail {
                    user_id: user.id,
                    email: "beefsack+two@gmail.com",
                    is_primary: false,
                },
                conn,
            ).expect("error creating user email");
            let (_, found_user) =
                find_user_with_primary_email_by_email("beefsack+two@gmail.com", conn)
                    .expect("error finding user")
                    .expect("user doesn't exist");
            assert_eq!(user.id, found_user.id);
            assert_eq!("beefsack@gmail.com", user_email.email);
        });
    }

    #[test]
    #[ignore]
    fn create_game_works() {
        with_db(|conn| {
            let game_type = create_game_type(
                &NewGameType {
                    name: "Lost Cities",
                    player_counts: vec![2],
                    weight: 1.52,
                },
                conn,
            ).unwrap();
            let game_version = create_game_version(
                &NewGameVersion {
                    game_type_id: game_type.id,
                    uri: "https://example.com/lost-cities-1",
                    name: "v1",
                    is_public: true,
                    is_deprecated: false,
                },
                conn,
            ).unwrap();
            assert!(
                create_game(
                    &NewGame {
                        game_version_id: game_version.id,
                        is_finished: false,
                        game_state: "blah",
                    },
                    conn,
                ).is_ok()
            );
        });
    }

    #[test]
    #[ignore]
    fn create_players_works() {
        with_db(|conn| {
            let (_, p1) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            let (_, p2) = create_user_by_email("beefsack+two@gmail.com", conn).unwrap();
            let game_type = create_game_type(
                &NewGameType {
                    name: "Lost Cities",
                    player_counts: vec![2],
                    weight: 1.52,
                },
                conn,
            ).unwrap();
            let game_version = create_game_version(
                &NewGameVersion {
                    game_type_id: game_type.id,
                    uri: "https://example.com/lost-cities-1",
                    name: "v1",
                    is_public: true,
                    is_deprecated: false,
                },
                conn,
            ).unwrap();
            let game = create_game(
                &NewGame {
                    game_version_id: game_version.id,
                    is_finished: false,
                    game_state: "egg",
                },
                conn,
            ).unwrap();
            create_game_players(
                &[
                    NewGamePlayer {
                        game_id: game.id,
                        user_id: p1.id,
                        position: 0,
                        color: &Color::Green.to_string(),
                        has_accepted: true,
                        is_turn: false,
                        is_turn_at: Utc::now().naive_utc(),
                        last_turn_at: Utc::now().naive_utc(),
                        is_eliminated: false,
                        place: None,
                        is_read: false,
                        points: None,
                        undo_game_state: None,
                        rating_change: None,
                    },
                    NewGamePlayer {
                        game_id: game.id,
                        user_id: p2.id,
                        position: 1,
                        color: &Color::Red.to_string(),
                        has_accepted: false,
                        is_turn: true,
                        is_turn_at: Utc::now().naive_utc(),
                        last_turn_at: Utc::now().naive_utc(),
                        is_eliminated: false,
                        place: None,
                        is_read: false,
                        points: Some(1.5),
                        undo_game_state: None,
                        rating_change: None,
                    },
                ],
                conn,
            ).unwrap();
        });
    }

    #[test]
    #[ignore]
    fn player_can_undo_set_undo_game_state_works() {
        with_db(|conn| {
            let (_, p1) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            let (_, p2) = create_user_by_email("beefsack+two@gmail.com", conn).unwrap();
            let game_type = create_game_type(
                &NewGameType {
                    name: "Lost Cities",
                    player_counts: vec![2],
                    weight: 1.52,
                },
                conn,
            ).unwrap();
            let game_version = create_game_version(
                &NewGameVersion {
                    game_type_id: game_type.id,
                    uri: "https://example.com/lost-cities-1",
                    name: "v1",
                    is_public: true,
                    is_deprecated: false,
                },
                conn,
            ).unwrap();
            let game = create_game(
                &NewGame {
                    game_version_id: game_version.id,
                    is_finished: false,
                    game_state: "egg",
                },
                conn,
            ).unwrap();
            player_can_undo_set_undo_game_state(&game.id, &p1.id, "{}", conn)
                .expect("failed to update player game undo state");
        });
    }

    #[test]
    #[ignore]
    fn player_cannot_undo_set_undo_game_state_works() {
        with_db(|conn| {
            let (_, p1) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            let (_, p2) = create_user_by_email("beefsack+two@gmail.com", conn).unwrap();
            let game_type = create_game_type(
                &NewGameType {
                    name: "Lost Cities",
                    player_counts: vec![2],
                    weight: 1.52,
                },
                conn,
            ).unwrap();
            let game_version = create_game_version(
                &NewGameVersion {
                    game_type_id: game_type.id,
                    uri: "https://example.com/lost-cities-1",
                    name: "v1",
                    is_public: true,
                    is_deprecated: false,
                },
                conn,
            ).unwrap();
            let game = create_game(
                &NewGame {
                    game_version_id: game_version.id,
                    is_finished: false,
                    game_state: "egg",
                },
                conn,
            ).unwrap();
            player_cannot_undo_set_undo_game_state(&game.id, conn)
                .expect("failed to update player game undo state");
        });
    }

    #[test]
    fn elo_rating_change_works() {
        assert_eq!(elo_rating_change(1184, 1200, 0.0), -15i32);
        assert_eq!(elo_rating_change(2400, 2000, 0.0), -29i32);
        assert_eq!(elo_rating_change(2400, 2000, 1.0), 3i32);
        assert_eq!(elo_rating_change(2400, 2000, 0.5), -13i32);
    }

    #[test]
    #[ignore]
    fn update_game_placings_works() {
        with_db(|conn| {
            let game_extended = create_test_game(5, conn);
            update_game_placings(&game_extended.game.id, &[1, 3, 2, 5, 4], conn)
                .expect("expected to update game placings");
            let updated_game_extended = find_game_extended(&game_extended.game.id, conn)
                .expect("expected to find game again");
            assert_eq!(
                updated_game_extended
                    .game_players
                    .iter()
                    .map(|gptu| gptu.game_player.place)
                    .collect::<Vec<Option<i32>>>(),
                vec![Some(1), Some(3), Some(2), Some(5), Some(4)]
            );

            let ratings: Vec<i32> = updated_game_extended
                .game_players
                .iter()
                .map(|gptu| gptu.game_type_user.rating)
                .collect();
            assert_eq!(ratings, vec![1264, 1200, 1232, 1136, 1168]);
        });
    }
}
