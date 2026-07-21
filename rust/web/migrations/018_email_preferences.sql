ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS invite_emails_enabled boolean NOT NULL DEFAULT true;
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS reminder_emails_enabled boolean NOT NULL DEFAULT true;
