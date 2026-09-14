-- Accounts. Guests have neither username nor password; they can set both
-- later and keep the same id.
CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    username      TEXT,
    password_hash TEXT,
    is_guest      BOOLEAN NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (is_guest = (username IS NULL)),
    CHECK ((username IS NULL) = (password_hash IS NULL))
);

-- Usernames are unique regardless of case; the stored spelling is the user's.
CREATE UNIQUE INDEX users_username_lower ON users (lower(username));

-- Server-side sessions: the id is the secret in the cookie.
CREATE TABLE sessions (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX sessions_user_id ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);
