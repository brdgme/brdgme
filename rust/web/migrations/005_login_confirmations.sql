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

-- Deploy note: the migrate Job runs at ArgoCD sync-wave 1 and the web
-- Deployment updates at sync-wave 2, but old-image pods keep serving
-- traffic until their rollout completes - so there's a brief (~30-60s)
-- window where old pods still `SELECT`ing users.login_confirmation /
-- login_confirmation_at hard-error against this schema. Accepted: the app
-- self-heals once the rollout finishes, and beta traffic is low. See the
-- beta-period checklist in
-- docs/superpowers/plans/2026-07-08-16-production-cutover-validation.md.
ALTER TABLE users
    DROP COLUMN login_confirmation,
    DROP COLUMN login_confirmation_at;
