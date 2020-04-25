table! {
    chat_messages (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        chat_user_id -> Uuid,
        message -> Text,
    }
}

table! {
    chats (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    chat_users (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        chat_id -> Uuid,
        user_id -> Uuid,
        last_read_at -> Timestamp,
    }
}

table! {
    friends (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        source_user_id -> Uuid,
        target_user_id -> Uuid,
        has_accepted -> Nullable<Bool>,
    }
}

table! {
    game_logs (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_id -> Uuid,
        body -> Text,
        is_public -> Bool,
        logged_at -> Timestamp,
    }
}

table! {
    game_log_targets (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_log_id -> Uuid,
        game_player_id -> Uuid,
    }
}

table! {
    game_players (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_id -> Uuid,
        user_id -> Uuid,
        position -> Int4,
        color -> Text,
        has_accepted -> Bool,
        is_turn -> Bool,
        is_turn_at -> Timestamp,
        last_turn_at -> Timestamp,
        is_eliminated -> Bool,
        is_read -> Bool,
        points -> Nullable<Float4>,
        undo_game_state -> Nullable<Text>,
        place -> Nullable<Int4>,
        rating_change -> Nullable<Int4>,
    }
}

table! {
    games (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_version_id -> Uuid,
        is_finished -> Bool,
        finished_at -> Nullable<Timestamp>,
        game_state -> Text,
        chat_id -> Nullable<Uuid>,
        restarted_game_id -> Nullable<Uuid>,
    }
}

table! {
    game_types (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> Text,
        player_counts -> Array<Int4>,
        weight -> Float4,
    }
}

table! {
    game_type_users (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_type_id -> Uuid,
        user_id -> Uuid,
        last_game_finished_at -> Nullable<Timestamp>,
        rating -> Int4,
        peak_rating -> Int4,
    }
}

table! {
    game_versions (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        game_type_id -> Uuid,
        name -> Text,
        uri -> Text,
        is_public -> Bool,
        is_deprecated -> Bool,
    }
}

table! {
    user_auth_tokens (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        user_id -> Uuid,
    }
}

table! {
    user_emails (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        user_id -> Uuid,
        email -> Text,
        is_primary -> Bool,
    }
}

table! {
    users (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> Text,
        pref_colors -> Array<Text>,
        login_confirmation -> Nullable<Text>,
        login_confirmation_at -> Nullable<Timestamp>,
    }
}

joinable!(chat_messages -> chat_users (chat_user_id));
joinable!(chat_users -> chats (chat_id));
joinable!(chat_users -> users (user_id));
joinable!(game_log_targets -> game_logs (game_log_id));
joinable!(game_log_targets -> game_players (game_player_id));
joinable!(game_logs -> games (game_id));
joinable!(game_players -> games (game_id));
joinable!(game_players -> users (user_id));
joinable!(game_type_users -> game_types (game_type_id));
joinable!(game_type_users -> users (user_id));
joinable!(game_versions -> game_types (game_type_id));
joinable!(games -> chats (chat_id));
joinable!(games -> game_versions (game_version_id));
joinable!(user_auth_tokens -> users (user_id));
joinable!(user_emails -> users (user_id));

allow_tables_to_appear_in_same_query!(
    chat_messages,
    chats,
    chat_users,
    friends,
    game_logs,
    game_log_targets,
    game_players,
    games,
    game_types,
    game_type_users,
    game_versions,
    user_auth_tokens,
    user_emails,
    users,
);
