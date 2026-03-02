-- Add missing fields to game_players that were present in the legacy schema.
ALTER TABLE public.game_players
    ADD COLUMN IF NOT EXISTS last_turn_at timestamp without time zone,
    ADD COLUMN IF NOT EXISTS is_eliminated boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS is_read boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS points real,
    ADD COLUMN IF NOT EXISTS undo_game_state text,
    ADD COLUMN IF NOT EXISTS rating_change integer;
