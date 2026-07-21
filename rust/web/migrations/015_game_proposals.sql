-- #24 game invites: pre-game proposals. A proposal tracks an open game offer
-- (owner + invited players/bots) that starts a real game once accepted.
-- game_proposal_players.bot_name = BotSlot.name (display name);
-- game_proposal_players.bot_difficulty = BotSlot.bot_name (easy/medium/hard).
CREATE TABLE IF NOT EXISTS public.game_proposals (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    created_at timestamp NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
    updated_at timestamp NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
    game_version_id uuid NOT NULL REFERENCES public.game_versions(id),
    owner_user_id uuid NOT NULL REFERENCES public.users(id),
    restarted_game_id uuid REFERENCES public.games(id),
    status text NOT NULL DEFAULT 'open'
        CHECK (status IN ('open','started','cancelled')),
    started_game_id uuid REFERENCES public.games(id)
);
CREATE INDEX IF NOT EXISTS idx_game_proposals_owner ON public.game_proposals(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_game_proposals_status ON public.game_proposals(status);

CREATE TABLE IF NOT EXISTS public.game_proposal_players (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    created_at timestamp NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
    updated_at timestamp NOT NULL DEFAULT (now() AT TIME ZONE 'utc'),
    proposal_id uuid NOT NULL REFERENCES public.game_proposals(id) ON DELETE CASCADE,
    "position" integer NOT NULL,
    user_id uuid REFERENCES public.users(id),
    bot_name text,
    bot_difficulty text,
    response text NOT NULL DEFAULT 'pending'
        CHECK (response IN ('pending','accepted','declined')),
    responded_at timestamp,
    email_token text,
    CONSTRAINT game_proposal_players_user_or_bot
        CHECK ((user_id IS NOT NULL) != (bot_name IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS idx_game_proposal_players_proposal
    ON public.game_proposal_players(proposal_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_game_proposal_players_email_token
    ON public.game_proposal_players(email_token) WHERE email_token IS NOT NULL;
