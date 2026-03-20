-- Add unique constraints required for operator upserts.
ALTER TABLE game_types ADD CONSTRAINT game_types_name_key UNIQUE (name);
ALTER TABLE game_versions ADD CONSTRAINT game_versions_game_type_id_name_key UNIQUE (game_type_id, name);
