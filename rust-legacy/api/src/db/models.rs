use uuid::Uuid;
use chrono::NaiveDateTime;
use failure::{Error, ResultExt};

use brdgme_markup as markup;

use db::schema::*;

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
    pub login_confirmation: Option<String>,
    pub login_confirmation_at: Option<NaiveDateTime>,
}

impl User {
    pub fn into_public(self) -> PublicUser {
        PublicUser {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            name: self.name,
            pref_colors: self.pref_colors,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicUser {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub pref_colors: &'a [&'a str],
    pub login_confirmation: Option<&'a str>,
    pub login_confirmation_at: Option<NaiveDateTime>,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(User)]
pub struct UserEmail {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Uuid,
    pub email: String,
    pub is_primary: bool,
}

#[derive(Insertable)]
#[table_name = "user_emails"]
pub struct NewUserEmail<'a> {
    pub user_id: Uuid,
    pub email: &'a str,
    pub is_primary: bool,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(User)]
pub struct UserAuthToken {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Uuid,
}

#[derive(Insertable)]
#[table_name = "user_auth_tokens"]
pub struct NewUserAuthToken {
    pub user_id: Uuid,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub player_counts: Vec<i32>,
    pub weight: f32,
}

pub type PublicGameType = GameType;

#[derive(Insertable)]
#[table_name = "game_types"]
pub struct NewGameType<'a> {
    pub name: &'a str,
    pub player_counts: Vec<i32>,
    pub weight: f32,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
#[belongs_to(GameType)]
pub struct GameVersion {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub name: String,
    pub uri: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

impl GameVersion {
    pub fn into_public(self) -> PublicGameVersion {
        PublicGameVersion {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            game_type_id: self.game_type_id,
            name: self.name,
            is_public: self.is_public,
            is_deprecated: self.is_deprecated,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicGameVersion {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub name: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Insertable)]
#[table_name = "game_versions"]
pub struct NewGameVersion<'a> {
    pub game_type_id: Uuid,
    pub name: &'a str,
    pub uri: &'a str,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameVersionType {
    pub game_version: GameVersion,
    pub game_type: GameType,
}

impl GameVersionType {
    pub fn into_public(self) -> PublicGameVersionType {
        PublicGameVersionType {
            game_version: self.game_version.into_public(),
            game_type: self.game_type,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicGameVersionType {
    pub game_version: PublicGameVersion,
    pub game_type: PublicGameType,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(GameVersion)]
pub struct Game {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub finished_at: Option<NaiveDateTime>,
    pub game_state: String,
    pub chat_id: Option<Uuid>,
    pub restarted_game_id: Option<Uuid>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct PublicGame {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub finished_at: Option<NaiveDateTime>,
    pub chat_id: Option<Uuid>,
    pub restarted_game_id: Option<Uuid>,
}

impl Game {
    pub fn into_public(self) -> PublicGame {
        PublicGame {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            game_version_id: self.game_version_id,
            is_finished: self.is_finished,
            finished_at: self.finished_at,
            chat_id: self.chat_id,
            restarted_game_id: self.restarted_game_id,
        }
    }
}

#[derive(Insertable)]
#[table_name = "games"]
pub struct NewGame<'a> {
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub game_state: &'a str,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
#[belongs_to(Game)]
#[belongs_to(User)]
pub struct GamePlayer {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: NaiveDateTime,
    pub last_turn_at: NaiveDateTime,
    pub is_eliminated: bool,
    pub is_read: bool,
    pub points: Option<f32>,
    pub undo_game_state: Option<String>,
    pub place: Option<i32>,
    pub rating_change: Option<i32>,
}

impl GamePlayer {
    pub fn into_public(self) -> PublicGamePlayer {
        PublicGamePlayer {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            game_id: self.game_id,
            user_id: self.user_id,
            position: self.position,
            color: self.color,
            has_accepted: self.has_accepted,
            is_turn: self.is_turn,
            is_turn_at: self.is_turn_at,
            last_turn_at: self.last_turn_at,
            is_eliminated: self.is_eliminated,
            is_read: self.is_read,
            points: self.points,
            can_undo: self.undo_game_state.is_some(),
            place: self.place,
            rating_change: self.rating_change,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PublicGamePlayer {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: NaiveDateTime,
    pub last_turn_at: NaiveDateTime,
    pub is_eliminated: bool,
    pub is_read: bool,
    pub points: Option<f32>,
    pub can_undo: bool,
    pub place: Option<i32>,
    pub rating_change: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "game_players"]
pub struct NewGamePlayer<'a> {
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: &'a str,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: NaiveDateTime,
    pub last_turn_at: NaiveDateTime,
    pub is_eliminated: bool,
    pub is_read: bool,
    pub points: Option<f32>,
    pub undo_game_state: Option<String>,
    pub place: Option<i32>,
    pub rating_change: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GamePlayerTypeUser {
    pub game_player: GamePlayer,
    pub user: User,
    pub game_type_user: GameTypeUser,
}

impl GamePlayerTypeUser {
    pub fn into_public(self) -> PublicGamePlayerTypeUser {
        PublicGamePlayerTypeUser {
            game_player: self.game_player.into_public(),
            user: self.user.into_public(),
            game_type_user: self.game_type_user,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicGamePlayerTypeUser {
    pub game_player: PublicGamePlayer,
    pub user: PublicUser,
    pub game_type_user: PublicGameTypeUser,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
#[belongs_to(Game)]
pub struct GameLog {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub body: String,
    pub is_public: bool,
    pub logged_at: NaiveDateTime,
}

pub type PublicGameLog = GameLog;

#[derive(Serialize, Deserialize, Clone)]
pub struct RenderedGameLog {
    game_log: PublicGameLog,
    html: String,
}

impl GameLog {
    fn render(&self, players: &[markup::Player]) -> Result<String, Error> {
        let (parsed, _) = markup::from_string(&self.body).context("error parsing log body")?;
        Ok(markup::html(&markup::transform(&parsed, players)))
    }

    pub fn into_rendered(self, players: &[markup::Player]) -> Result<RenderedGameLog, Error> {
        let html = self.render(players)?;
        Ok(RenderedGameLog {
            game_log: self,
            html: html,
        })
    }
}

#[derive(Insertable)]
#[table_name = "game_logs"]
pub struct NewGameLog<'a> {
    pub game_id: Uuid,
    pub body: &'a str,
    pub is_public: bool,
    pub logged_at: NaiveDateTime,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(GameLog)]
#[belongs_to(GamePlayer)]
pub struct GameLogTarget {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_log_id: Uuid,
    pub game_player_id: Uuid,
}

#[derive(Insertable)]
#[table_name = "game_log_targets"]
pub struct NewGameLogTarget {
    pub game_log_id: Uuid,
    pub game_player_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(GameType)]
#[belongs_to(User)]
pub struct GameTypeUser {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub user_id: Uuid,
    pub last_game_finished_at: Option<NaiveDateTime>,
    pub rating: i32,
    pub peak_rating: i32,
}

pub type PublicGameTypeUser = GameTypeUser;

#[derive(Insertable)]
#[table_name = "game_type_users"]
pub struct NewGameTypeUser {
    pub game_type_id: Uuid,
    pub user_id: Uuid,
    pub last_game_finished_at: Option<NaiveDateTime>,
    pub rating: Option<i32>,
    pub peak_rating: Option<i32>,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(User, foreign_key = "target_user_id")]
pub struct Friend {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub source_user_id: Uuid,
    pub target_user_id: Uuid,
    pub has_accepted: Option<bool>,
}

#[derive(Insertable)]
#[table_name = "friends"]
pub struct NewFriend {
    pub source_user_id: Uuid,
    pub target_user_id: Uuid,
    pub has_accepted: Option<bool>,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize)]
pub struct Chat {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub type PublicChat = Chat;

#[derive(Insertable)]
#[table_name = "chats"]
pub struct NewChat {
    pub id: Option<Uuid>, // Can't use an empty struct
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize)]
#[belongs_to(ChatUser)]
pub struct ChatMessage {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub chat_user_id: Uuid,
    pub message: String,
}

pub type PublicChatMessage = ChatMessage;

#[derive(Insertable)]
#[table_name = "chat_messages"]
pub struct NewChatMessage<'a> {
    pub chat_user_id: Uuid,
    pub message: &'a str,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize)]
#[belongs_to(Chat)]
#[belongs_to(User)]
pub struct ChatUser {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub last_read_at: NaiveDateTime,
}

pub type PublicChatUser = ChatUser;

#[derive(Insertable)]
#[table_name = "chat_users"]
pub struct NewChatUser {
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub last_read_at: Option<NaiveDateTime>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{self, Connection};
    use diesel::prelude::*;
    use db::color::Color;
    use db::{schema, CONN};

    #[test]
    #[ignore]
    fn insert_user_works() {
        let conn = &*CONN.w.get().unwrap();
        conn.begin_test_transaction().unwrap();
        diesel::insert_into(schema::users::table)
            .values(&NewUser {
                name: "blah",
                pref_colors: &[&Color::Green.to_string()],
                login_confirmation: None,
                login_confirmation_at: None,
            })
            .get_result::<User>(conn)
            .expect("Error inserting user");
    }
}
