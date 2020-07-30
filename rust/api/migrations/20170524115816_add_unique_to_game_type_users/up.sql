ALTER TABLE game_type_users
ADD UNIQUE (game_type_id, user_id);