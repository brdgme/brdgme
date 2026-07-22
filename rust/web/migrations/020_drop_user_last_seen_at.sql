-- Drop the dead request-middleware presence column. Web presence is now
-- users.last_active_at (migration 019), pinged explicitly by the client.
ALTER TABLE public.users
    DROP COLUMN IF EXISTS last_seen_at;
