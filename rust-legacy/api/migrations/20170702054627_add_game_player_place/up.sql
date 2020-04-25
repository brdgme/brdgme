ALTER TABLE game_players
ADD COLUMN place INT;

UPDATE game_players AS gp
SET place = CASE WHEN gp.is_winner = TRUE THEN 1
                 WHEN gp.is_winner = FALSE THEN 2
            END
FROM games AS g
WHERE gp.game_id = g.id
AND g.is_finished = TRUE;

ALTER TABLE game_players
DROP COLUMN IF EXISTS is_winner;