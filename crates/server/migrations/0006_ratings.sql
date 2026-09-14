-- Glicko-2 ratings per user and category (see src/rating.rs).
CREATE TABLE ratings (
    user_id      TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    category     TEXT NOT NULL CHECK (category IN ('bullet', 'blitz', 'rapid', 'classical')),
    rating       DOUBLE PRECISION NOT NULL,
    deviation    DOUBLE PRECISION NOT NULL,
    volatility   DOUBLE PRECISION NOT NULL,
    games        INTEGER NOT NULL DEFAULT 0,
    last_game_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, category)
);

-- Whether a game is rated; each side's rating when they sat down (shown
-- during the game); and what the game did to it. `rating_applied` makes the
-- update happen exactly once, whatever ends the game and however often.
ALTER TABLE games
    ADD COLUMN rated BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN white_rating INTEGER,
    ADD COLUMN white_provisional BOOLEAN,
    ADD COLUMN black_rating INTEGER,
    ADD COLUMN black_provisional BOOLEAN,
    ADD COLUMN white_rating_diff INTEGER,
    ADD COLUMN black_rating_diff INTEGER,
    ADD COLUMN rating_applied BOOLEAN NOT NULL DEFAULT false;
