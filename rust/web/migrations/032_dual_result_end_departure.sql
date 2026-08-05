-- DRM-01a / dual-result model: additive nullable game end and player departure
-- metadata, plus legacy initialization for unfinished games.
--
-- end_reason / departure_reason are checked text and informational only (never
-- ranking inputs). Columns are nullable and NOT linked to left_at, so legacy
-- nulls and old-pod rollout writes (left_at set, both departure fields null)
-- remain valid. Completed historical results and ratings are untouched.

ALTER TABLE public.games
    ADD COLUMN IF NOT EXISTS end_reason text,
    ADD CONSTRAINT games_end_reason_chk CHECK (
        end_reason IS NULL OR end_reason IN ('game_service', 'concession_forfeit', 'last_human_stop')
    );

ALTER TABLE public.game_players
    ADD COLUMN IF NOT EXISTS departure_reason text,
    ADD COLUMN IF NOT EXISTS departure_sequence integer,
    ADD CONSTRAINT game_players_departure_reason_chk CHECK (
        departure_reason IS NULL OR departure_reason IN ('conceded', 'timeout_replaced', 'eliminated', 'unknown_legacy')
    ),
    ADD CONSTRAINT game_players_departure_together_chk CHECK (
        (departure_reason IS NULL) = (departure_sequence IS NULL)
    ),
    ADD CONSTRAINT game_players_departure_positive_chk CHECK (
        departure_sequence IS NULL OR departure_sequence > 0
    );

-- Legacy initialization: only human (user_id IS NOT NULL) rows that left
-- (left_at IS NOT NULL) in an unfinished game (is_finished = false) get
-- 'unknown_legacy' and a per-game dense sequence over left_at, so equal
-- timestamps tie. Completed games, placings, ratings, and bot rows are
-- left alone.
WITH legacy_departures AS (
    SELECT
        gp.id,
        dense_rank() OVER (
            PARTITION BY gp.game_id
            ORDER BY gp.left_at
        ) AS departure_sequence
    FROM public.game_players gp
    JOIN public.games g ON g.id = gp.game_id
    WHERE gp.user_id IS NOT NULL
      AND gp.left_at IS NOT NULL
      AND NOT g.is_finished
)
UPDATE public.game_players gp
SET departure_reason = 'unknown_legacy',
    departure_sequence = ld.departure_sequence
FROM legacy_departures ld
WHERE gp.id = ld.id;
