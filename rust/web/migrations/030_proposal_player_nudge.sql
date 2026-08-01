-- R-19 / F-144: per-invitee invite-nudge dedup. The nudge sweep sends per
-- invitee but deduped per proposal (game_proposals.nudged_at, migration 016),
-- so a single transient no-send re-nudged the whole roster every tick. Add a
-- per-invitee marker so each invitee is nudged at most once. game_proposals.
-- nudged_at is left dormant (no longer read by the sweep).
ALTER TABLE public.game_proposal_players ADD COLUMN IF NOT EXISTS nudged_at timestamp;
