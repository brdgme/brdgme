-- #47 Concede with bot replacement & end game.
--
-- A conceded human keeps user_id (preserving the player name/link) and gains
-- game_bot_id (the replacement bot). Relax the XOR check to allow both.
ALTER TABLE game_players DROP CONSTRAINT game_players_user_or_bot;
ALTER TABLE game_players ADD CONSTRAINT game_players_user_or_bot CHECK (
    user_id IS NOT NULL OR game_bot_id IS NOT NULL
);

-- ranked_placing: placing used for ELO/Form (concede/elimination lose first).
-- left_at: when a player conceded or was eliminated (orders ranked placings).
ALTER TABLE game_players ADD COLUMN ranked_placing integer;
ALTER TABLE game_players ADD COLUMN left_at timestamp without time zone;

-- Admin flag: bots eligible to replace a conceding/slow human.
ALTER TABLE bots ADD COLUMN can_replace_humans boolean NOT NULL DEFAULT false;
