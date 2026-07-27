CREATE TABLE login_email_sends (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email text NOT NULL,
    sent_at timestamp NOT NULL DEFAULT now()
);

CREATE INDEX idx_login_email_sends_sent_at ON login_email_sends (sent_at);
