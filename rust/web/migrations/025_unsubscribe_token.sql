-- WP-58 / RFC 8058: per-user secret token for the one-click unsubscribe
-- link. Populated lazily on first unsubscribe email, not backfilled.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS unsubscribe_token text;
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_unsubscribe_token
    ON public.users(unsubscribe_token) WHERE unsubscribe_token IS NOT NULL;
