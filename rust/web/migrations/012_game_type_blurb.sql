-- #44 new game page (spec 2026-07-19-new-game-page-design.md): short
-- 1-2 sentence description per game type, upserted by the operator from
-- the GameVersion CRD alongside weight.
ALTER TABLE public.game_types
    ADD COLUMN blurb text NOT NULL DEFAULT '';
