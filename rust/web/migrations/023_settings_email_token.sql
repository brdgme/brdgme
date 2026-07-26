-- WP-56 / D-1: per-user secret token for the settings reply address
-- (s-{token}@brdg.me). Populated lazily on first settings email, not
-- backfilled.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS settings_email_token text;
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_settings_email_token
    ON public.users(settings_email_token) WHERE settings_email_token IS NOT NULL;
