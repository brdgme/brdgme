ALTER TABLE public.game_players ADD COLUMN IF NOT EXISTS rating_before integer;

WITH ordered AS (
    SELECT
        gp.id AS game_player_id,
        1200 + COALESCE(
            sum(gp.rating_change) OVER (
                PARTITION BY gp.user_id, gv.game_type_id
                ORDER BY g.finished_at, g.id
                ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
            ),
            0
        ) AS rating_before
    FROM game_players gp
    JOIN games g ON g.id = gp.game_id
    JOIN game_versions gv ON gv.id = g.game_version_id
    WHERE gp.user_id IS NOT NULL
      AND gp.rating_change IS NOT NULL
      AND gp.rating_before IS NULL
      AND g.finished_at IS NOT NULL
)
UPDATE game_players gp
SET rating_before = ordered.rating_before
FROM ordered
WHERE gp.id = ordered.game_player_id;
