-- Per-user theme preference (profile sync for the theme picker). NULL means
-- "system" (no explicit preference stored); values are the theme slugs the
-- web layer defines (see web/src/theme.rs THEME_SLUGS), not enforced at the
-- DB level since the set is a web-layer concern, not shared with other
-- consumers of this table.
ALTER TABLE public.users ADD COLUMN theme text;
