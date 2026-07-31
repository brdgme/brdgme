-- R-07 / F-125: enforce canonical storage and align the unique index with the
-- backfill expression used in 026 (lower(btrim(email))). Migration 026 is
-- immutable; this adds the missing CHECK and replaces the lower(email) index
-- (which disagreed with the btrim backfill) with one on the same expression.

ALTER TABLE public.user_emails
    ADD CONSTRAINT user_emails_email_canonical_chk
    CHECK (email = lower(btrim(email)));

DROP INDEX IF EXISTS public.user_emails_email_lower_key;
CREATE UNIQUE INDEX user_emails_email_canonical_key
    ON public.user_emails (lower(btrim(email)));
