-- Bot players for LLM-backed AI opponents.
--
-- game_bots: one row per bot player slot per game. Bots are not users.
-- game_players: user_id made nullable; game_bot_id added as the alternative.
-- A CHECK constraint enforces that exactly one of user_id / game_bot_id is set.

CREATE TABLE game_bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_id UUID NOT NULL REFERENCES games(id),
    name TEXT NOT NULL,
    difficulty TEXT NOT NULL CHECK (difficulty IN ('easy', 'medium', 'hard')),
    personality TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (game_id, name)
);

ALTER TABLE game_players
    ALTER COLUMN user_id DROP NOT NULL,
    ADD COLUMN game_bot_id UUID REFERENCES game_bots(id),
    ADD CONSTRAINT game_players_user_or_bot CHECK (
        (user_id IS NOT NULL) != (game_bot_id IS NOT NULL)
    );
