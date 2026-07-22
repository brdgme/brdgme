-- Unit B (R4) per-user game visibility. Mirrors the invite_policy pattern
-- from 010_friends.sql: deliberately NOT a field on the User model struct -
-- read/written via db::get_game_visibility / db::set_game_visibility only.
-- Indexed for fast bulk filtering (the public index gate scans many users).
-- Repeatable pattern: future privacy settings follow column + CHECK + index.
ALTER TABLE public.users
    ADD COLUMN game_visibility text NOT NULL DEFAULT 'public'
        CHECK (game_visibility IN ('public', 'friends', 'private'));

CREATE INDEX idx_users_game_visibility ON public.users (game_visibility);
