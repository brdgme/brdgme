-- WP-50 / D-09: canonicalize stored email addresses (trim + lowercase) and
-- enforce it with a lower() unique index. Boundary code now canonicalizes on
-- write; this migrates the existing rows to match.

-- 1. login_confirmations.email IS the primary key, so lowercasing could
--    collide there. Rows are 1-hour ephemeral codes the app already GCs
--    opportunistically; purging costs at most a re-request.
DELETE FROM login_confirmations;

-- 2. Abort on case-duplicates, naming them. Two stored addresses differing
--    only by case become one when lowercased, and the index below would fail.
--    Collapsing them would merge two accounts - no migration can do that
--    deterministically, so surface the risk once, deliberately (D-09).
--    Operator pre-flight:
--      SELECT lower(btrim(email)) AS canonical, array_agg(email), array_agg(user_id)
--      FROM public.user_emails GROUP BY 1 HAVING count(*) > 1;
DO $$
DECLARE dups text;
BEGIN
    SELECT string_agg(k, ', ' ORDER BY k) INTO dups
    FROM (SELECT lower(btrim(email)) AS k FROM public.user_emails
          GROUP BY 1 HAVING count(*) > 1) d;
    IF dups IS NOT NULL THEN
        RAISE EXCEPTION
          'migration 026: case-duplicate addresses must be merged by hand first: %', dups;
    END IF;
END $$;

-- 3. Lowercase + trim stored rows, then enforce. The existing
--    user_emails_email_key is left in place (redundant once rows are canonical
--    but harmless; dropping it is separate risk).
UPDATE public.user_emails SET email = lower(btrim(email)) WHERE email <> lower(btrim(email));
CREATE UNIQUE INDEX IF NOT EXISTS user_emails_email_lower_key ON public.user_emails (lower(email));
