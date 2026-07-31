ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS settings_token_expires_at timestamptz,
    ADD COLUMN IF NOT EXISTS settings_token_used_at timestamptz;
