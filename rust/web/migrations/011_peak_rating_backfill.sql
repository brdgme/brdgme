-- #29 peak_rating backfill. game_type_users.peak_rating is historically
-- wrong: legacy code never maintained it (default 1200), and current code
-- (since #12) only ever raises it on new games. True rating history is
-- reconstructible as 1200 + running cumulative sum of game_players
-- rating_change per (user_id, game_type_id), ordered by games.finished_at
-- then games.id (matches rating_series() in rust/web/src/stats/queries.rs);
-- true peak is GREATEST(1200, max running value).
--
-- Data-only, idempotent: peak_rating can only ever be under-recorded, never
-- over-recorded, so this UPDATE only ever raises it (GREATEST), and the
-- strict `<` guard makes re-running this migration file a no-op once applied.
WITH series AS (
    SELECT
        gp.user_id,
        gv.game_type_id,
        1200 + sum(gp.rating_change) OVER (
            PARTITION BY gp.user_id, gv.game_type_id
            ORDER BY g.finished_at, g.id
        ) AS running_rating
    FROM game_players gp
    JOIN games g ON g.id = gp.game_id
    JOIN game_versions gv ON gv.id = g.game_version_id
    WHERE gp.user_id IS NOT NULL
      AND gp.rating_change IS NOT NULL
      AND g.finished_at IS NOT NULL
),
peaks AS (
    SELECT user_id, game_type_id, GREATEST(1200, max(running_rating)) AS peak
    FROM series
    GROUP BY user_id, game_type_id
)
UPDATE game_type_users gtu
SET peak_rating = GREATEST(gtu.peak_rating, p.peak)
FROM peaks p
WHERE gtu.user_id = p.user_id
  AND gtu.game_type_id = p.game_type_id
  AND gtu.peak_rating < p.peak;
