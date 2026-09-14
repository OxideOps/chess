-- Sign-in with an OAuth provider: (provider, subject) names one user. Such
-- users have a username but no password, so the users check that tied the
-- two together is relaxed: a password still needs a username.
ALTER TABLE users DROP CONSTRAINT users_check1;
ALTER TABLE users ADD CONSTRAINT users_password_needs_username
    CHECK (password_hash IS NULL OR username IS NOT NULL);

CREATE TABLE identities (
    provider   TEXT NOT NULL,
    subject    TEXT NOT NULL,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (provider, subject)
);

CREATE INDEX identities_user_id ON identities (user_id);
