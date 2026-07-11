-- Login confirmations, keyed by email rather than a user row (D2 in
-- docs/superpowers/specs/2026-07-08-28-abuse-protection-design.md): no user
-- row is created until a login code is confirmed, closing the user-table
-- spam / Resend-quota-burn hole where any unknown email auto-created a user.
--
-- Rows are short-lived operational state (1 hour code validity) and are
-- deleted opportunistically by the application: expired rows on upsert in
-- login(), the row itself on successful confirm. No cron/job needed.

CREATE TABLE login_confirmations (
    email TEXT PRIMARY KEY,
    code CHAR(6) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempts INT NOT NULL DEFAULT 0,
    sent_count INT NOT NULL DEFAULT 0,
    last_sent_at TIMESTAMPTZ
);

ALTER TABLE users
    DROP COLUMN login_confirmation,
    DROP COLUMN login_confirmation_at;
