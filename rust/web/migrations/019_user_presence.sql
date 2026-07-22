-- Presence ping: last time the logged-in user's browser pinged the server
-- (client pings every ~5 min while any page is open). NULL = never pinged.
-- Distinct from last_seen_at (request middleware) - this is an explicit ping.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS last_active_at timestamptz;
