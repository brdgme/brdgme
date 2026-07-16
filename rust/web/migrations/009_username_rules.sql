-- D2/D3 of docs/superpowers/specs/2026-07-11-35-user-settings-design.md:
-- usernames must match ^[a-zA-Z0-9_-]{1,16}$ and be unique
-- case-insensitively. One-off: sanitize any existing names that violate
-- that (strip disallowed chars, truncate to 16, fall back to 'player' if
-- empty), then resolve case-insensitive duplicates by keeping the name on
-- the earliest-created holder and appending a random 4-digit suffix to
-- the others, then add the unique index.
DO $$
DECLARE
    u record;
    candidate text;
BEGIN
    UPDATE users
    SET name = CASE
        WHEN left(regexp_replace(name, '[^a-zA-Z0-9_-]', '', 'g'), 16) = '' THEN 'player'
        ELSE left(regexp_replace(name, '[^a-zA-Z0-9_-]', '', 'g'), 16)
    END
    WHERE name !~ '^[a-zA-Z0-9_-]{1,16}$';

    FOR u IN
        SELECT us.id, left(us.name, 12) AS base FROM users us
        WHERE EXISTS (
            SELECT 1 FROM users other
            WHERE other.id <> us.id
              AND lower(other.name) = lower(us.name)
              AND (other.created_at < us.created_at
                   OR (other.created_at = us.created_at AND other.id < us.id))
        )
        ORDER BY us.created_at, us.id
    LOOP
        LOOP
            candidate := u.base || lpad(floor(random() * 10000)::int::text, 4, '0');
            EXIT WHEN NOT EXISTS (
                SELECT 1 FROM users WHERE lower(name) = lower(candidate)
            );
        END LOOP;
        UPDATE users SET name = candidate, updated_at = NOW() WHERE id = u.id;
    END LOOP;
END $$;

CREATE UNIQUE INDEX users_name_lower_key ON public.users (lower(name));
