-- 22b: per-player reply token (Reply-To: g-{token}@play.brdg.me). Populated
-- lazily on first notification send, not backfilled here.
ALTER TABLE public.game_players
    ADD COLUMN IF NOT EXISTS email_token text;
CREATE UNIQUE INDEX IF NOT EXISTS idx_game_players_email_token
    ON public.game_players(email_token) WHERE email_token IS NOT NULL;

-- 22b: account-wide turn-email opt-out (legacy used a Mongo Unsubscribed flag).
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS turn_emails_enabled boolean NOT NULL DEFAULT true;

-- 22b: last time the user was active on the web (for active-web suppression of
-- turn emails); NULL = never active.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS last_seen_at timestamp without time zone;

-- 22c: one reminder per turn; reset to NULL on every is_turn transition.
ALTER TABLE public.game_players
    ADD COLUMN IF NOT EXISTS turn_reminder_sent_at timestamp without time zone;

-- 22d: address verification; backfill existing rows as verified (they predate
-- the feature and were login-used).
ALTER TABLE public.user_emails
    ADD COLUMN IF NOT EXISTS verified_at timestamp without time zone;
UPDATE public.user_emails SET verified_at = now() AT TIME ZONE 'utc'
    WHERE verified_at IS NULL;

-- 22d: exactly one primary per user (enforce the invariant in the DB).
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_emails_one_primary
    ON public.user_emails(user_id) WHERE is_primary;

-- 22b: webhook idempotency (Resend retries on non-2xx; dedupe on svix-id).
CREATE TABLE IF NOT EXISTS public.processed_webhook_events (
    event_id text PRIMARY KEY,
    processed_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_processed_webhook_events_processed_at
    ON public.processed_webhook_events(processed_at);
