-- #30 friends (spec 2026-07-08-30-friends-design.md D1/D4/D7)

-- One row per direction, and one row per unordered pair: "are we friends"
-- is a single-row lookup and A->B / B->A duplicates are impossible.
CREATE UNIQUE INDEX friends_source_target_key
    ON public.friends (source_user_id, target_user_id);
CREATE UNIQUE INDEX friends_pair_key
    ON public.friends (LEAST(source_user_id, target_user_id),
                       GREATEST(source_user_id, target_user_id));

-- D4 invite policy. Deliberately NOT added to the User model struct - read
-- via db::get_invite_policy only (see plan Global Constraints).
ALTER TABLE public.users
    ADD COLUMN invite_policy text NOT NULL DEFAULT 'open'
        CHECK (invite_policy IN ('open', 'friends', 'none'));

-- D7 blocks: directed, independent of friendship, both directions may exist.
CREATE TABLE public.blocks (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone
        DEFAULT timezone('utc'::text, now()) NOT NULL,
    blocker_user_id uuid NOT NULL REFERENCES public.users(id),
    blocked_user_id uuid NOT NULL REFERENCES public.users(id),
    CONSTRAINT blocks_pkey PRIMARY KEY (id),
    CONSTRAINT blocks_blocker_blocked_key
        UNIQUE (blocker_user_id, blocked_user_id),
    CONSTRAINT blocks_check CHECK (blocker_user_id <> blocked_user_id)
);
CREATE INDEX idx_blocks_blocker_user_id ON public.blocks (blocker_user_id);
CREATE INDEX idx_blocks_blocked_user_id ON public.blocks (blocked_user_id);
