ALTER TABLE game_players
ADD COLUMN is_winner BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE game_players
SET is_winner = TRUE
WHERE place = 1;

ALTER TABLE game_players
ALTER COLUMN is_winner SET NOT NULL;

ALTER TABLE game_players
DROP COLUMN IF EXISTS place;
