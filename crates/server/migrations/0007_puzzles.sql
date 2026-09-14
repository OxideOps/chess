-- Puzzles from the Lichess puzzle database (CC0), loaded by
-- `chess-server import-puzzles`. `moves` is UCI, space-separated; the first
-- is the opponent's move that sets the puzzle up.
CREATE TABLE puzzles (
    id         TEXT PRIMARY KEY,
    fen        TEXT NOT NULL,
    moves      TEXT NOT NULL,
    rating     INTEGER NOT NULL,
    deviation  INTEGER NOT NULL,
    popularity INTEGER NOT NULL,
    plays      INTEGER NOT NULL,
    themes     TEXT NOT NULL DEFAULT ''
);

CREATE INDEX puzzles_rating ON puzzles (rating);

-- Each user's puzzle rating (Glicko-2, like game ratings, against the
-- puzzle's own rating). Guests have one too, for as long as they exist.
CREATE TABLE puzzle_ratings (
    user_id    TEXT PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    rating     DOUBLE PRECISION NOT NULL,
    deviation  DOUBLE PRECISION NOT NULL,
    volatility DOUBLE PRECISION NOT NULL,
    attempts   INTEGER NOT NULL DEFAULT 0,
    last_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Only a user's first try at a puzzle counts, and tried puzzles aren't
-- served to them again.
CREATE TABLE puzzle_attempts (
    user_id   TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    puzzle_id TEXT NOT NULL REFERENCES puzzles (id) ON DELETE CASCADE,
    solved    BOOLEAN NOT NULL,
    at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, puzzle_id)
);
