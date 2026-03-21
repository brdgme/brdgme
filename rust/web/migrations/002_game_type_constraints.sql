-- Add unique constraint on game_types.name required for operator upserts.
-- game_versions_game_type_id_name_key already exists in the production schema (covered by 001).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'game_types_name_key') THEN
        ALTER TABLE game_types ADD CONSTRAINT game_types_name_key UNIQUE (name);
    END IF;
END $$;
