-- Adopt the new 8-entry player colour palette (docs/authoring/THEMING.md).
--
-- The public.color enum created in 001 is unused (users.pref_colors and
-- game_players.color are both text), so it can simply be dropped and
-- recreated with the new palette order rather than migrated in place.
--
-- The stored text values on users.pref_colors and game_players.color do
-- need migrating though: 'Amber' and 'BlueGrey' no longer exist as slots.
-- Per THEMING.md's player-palette slot inheritance, Amber's slot is now
-- named Orange, and BlueGrey's slot is now named Cyan, so old rows are
-- remapped to the new names for the same slots.

DROP TYPE IF EXISTS public.color;
CREATE TYPE public.color AS ENUM (
    'Green',
    'Red',
    'Blue',
    'Orange',
    'Purple',
    'Brown',
    'Cyan',
    'Pink'
);

UPDATE public.users
SET pref_colors = array_replace(array_replace(pref_colors, 'Amber', 'Orange'), 'BlueGrey', 'Cyan')
WHERE 'Amber' = ANY(pref_colors) OR 'BlueGrey' = ANY(pref_colors);

UPDATE public.game_players SET color = 'Orange' WHERE color = 'Amber';
UPDATE public.game_players SET color = 'Cyan' WHERE color = 'BlueGrey';
