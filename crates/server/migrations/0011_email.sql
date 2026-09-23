-- An optional email address per account, for password resets.
--
-- `users.email` only ever holds a *verified* address: one being added or
-- changed to waits in `email_tokens` until its link is followed, so a reset
-- can only go to an address that has proved it belongs to the account.
ALTER TABLE users ADD COLUMN email TEXT;
ALTER TABLE users ADD CONSTRAINT users_email_needs_username
    CHECK (email IS NULL OR username IS NOT NULL);
CREATE UNIQUE INDEX users_email_lower ON users (lower(email));

-- Links sent by email: `verify` confirms `email` for the account, `reset`
-- sets a new password. Only a SHA-256 of the token is kept (a reset link is
-- as good as the password), and following a link deletes its row, so each
-- works once.
CREATE TABLE email_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    purpose    TEXT NOT NULL CHECK (purpose IN ('verify', 'reset')),
    email      TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX email_tokens_user_id ON email_tokens (user_id, purpose);
CREATE INDEX email_tokens_expires_at ON email_tokens (expires_at);
