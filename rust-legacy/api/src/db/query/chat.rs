use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;
use chrono::{NaiveDateTime, Utc};
use failure::{Error, ResultExt};

use db::models::*;

pub fn create(conn: &PgConnection) -> Result<Chat, Error> {
    use db::schema::chats;

    Ok(diesel::insert_into(chats::table)
        .values(&NewChat { id: None })
        .get_result(conn)
        .context("error creating chat")?)
}

pub fn add_users(
    chat_id: Uuid,
    user_ids: &[Uuid],
    conn: &PgConnection,
) -> Result<Vec<ChatUser>, Error> {
    use db::schema::chat_users;

    if user_ids.is_empty() {
        return Ok(vec![]);
    }

    Ok(diesel::insert_into(chat_users::table)
        .values(&user_ids
            .iter()
            .map(|&user_id| {
                NewChatUser {
                    chat_id,
                    user_id,
                    last_read_at: None,
                }
            })
            .collect::<Vec<NewChatUser>>())
        .get_results(conn)
        .context("error adding users to chat")?)
}

pub fn create_message(
    chat_user_id: Uuid,
    message: &str,
    conn: &PgConnection,
) -> Result<ChatMessage, Error> {
    use db::schema::chat_messages;

    Ok(diesel::insert_into(chat_messages::table)
        .values(&NewChatMessage {
            chat_user_id,
            message,
        })
        .get_result(conn)
        .context("error creating chat message")?)
}

pub fn find(id: &Uuid, conn: &PgConnection) -> Result<Chat, Error> {
    use db::schema::chats;

    Ok(chats::table
        .find(id)
        .get_result(conn)
        .context("error finding chat")?)
}

pub fn find_users_by_chat(chat_id: &Uuid, conn: &PgConnection) -> Result<Vec<ChatUser>, Error> {
    use db::schema::chat_users;

    Ok(chat_users::table
        .filter(chat_users::chat_id.eq(chat_id))
        .get_results(conn)
        .context("error finding chat users for chat")?)
}

pub fn find_messages_by_chat(
    chat_id: &Uuid,
    conn: &PgConnection,
) -> Result<Vec<ChatMessage>, Error> {
    use db::schema::{chat_messages, chat_users};

    Ok(chat_messages::table
        .inner_join(chat_users::table)
        .filter(chat_users::chat_id.eq(chat_id))
        .get_results::<(ChatMessage, ChatUser)>(conn)
        .map(|rows| rows.into_iter().map(|row| row.0).collect())
        .context("error finding chat users for chat")?)
}

pub fn update_user_last_read_at(
    chat_user_id: &Uuid,
    at: NaiveDateTime,
    conn: &PgConnection,
) -> Result<Option<ChatUser>, Error> {
    use db::schema::chat_users;

    Ok(diesel::update(chat_users::table.find(chat_user_id))
        .set(chat_users::last_read_at.eq(at))
        .get_result(conn)
        .optional()
        .context("error updating chat user last read at")?)
}

pub fn update_user_last_read_at_now(
    chat_user_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<ChatUser>, Error> {
    update_user_last_read_at(chat_user_id, Utc::now().naive_utc(), conn)
}

#[derive(Clone)]
pub struct ChatExtended {
    pub chat: Chat,
    pub chat_users: Vec<ChatUser>,
    pub chat_messages: Vec<ChatMessage>,
}

impl ChatExtended {
    pub fn into_public(self) -> PublicChatExtended {
        PublicChatExtended {
            chat: self.chat,
            chat_users: self.chat_users.into_iter().collect(),
            chat_messages: self.chat_messages.into_iter().collect(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct PublicChatExtended {
    pub chat: PublicChat,
    pub chat_users: Vec<PublicChatUser>,
    pub chat_messages: Vec<PublicChatMessage>,
}

pub fn find_extended(id: &Uuid, conn: &PgConnection) -> Result<ChatExtended, Error> {
    Ok(ChatExtended {
        chat: find(id, conn)?,
        chat_users: find_users_by_chat(id, conn)?,
        chat_messages: find_messages_by_chat(id, conn)?,
    })
}

#[cfg(test)]
mod tests {
    use db::query::*;
    use super::*;

    #[test]
    #[ignore]
    fn create_works() {
        with_db(|conn| {
            create(conn).expect("expected to create a chat");
        });
    }

    #[test]
    #[ignore]
    fn add_users_works() {
        with_db(|conn| {
            let user1 = create_user_by_name("blah", conn).expect("expected to create a user");
            let user2 = create_user_by_name("egg", conn).expect("expected to create a user");
            let user3 = create_user_by_name("bacon", conn).expect("expected to create a user");
            let chat = create(conn).expect("expected to create a chat");
            add_users(chat.id, &[user1.id, user2.id, user3.id], conn)
                .expect("expected to add users to chat");
        });
    }

    #[test]
    #[ignore]
    fn create_message_works() {
        with_db(|conn| {
            let user = create_user_by_name("blah", conn).expect("expected to create a user");
            let chat = create(conn).expect("expected to create a chat");
            let chat_users =
                add_users(chat.id, &[user.id], conn).expect("expected to add user to chat");
            create_message(chat_users[0].id, "this is the message", conn)
                .expect("expected to create a chat message");
        });
    }

    #[test]
    #[ignore]
    fn find_extended_works() {
        with_db(|conn| {
            let user = create_user_by_name("blah", conn).expect("expected to create a user");
            let chat = create(conn).expect("expected to create a chat");
            let chat_users =
                add_users(chat.id, &[user.id], conn).expect("expected to add user to chat");
            create_message(chat_users[0].id, "this is the message", conn)
                .expect("expected to create a chat message");
            find(&chat.id, conn).expect("expected to find chat extended");
        });
    }
}
