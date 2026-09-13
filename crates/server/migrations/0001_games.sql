-- One row per game: the room's snapshot, rewritten on every change.
-- Moves are the whole game as space-separated UCI; a game is small, and a
-- full rewrite keeps the write path to a single statement.
CREATE TABLE games (
    id            TEXT PRIMARY KEY,
    white_token   TEXT NOT NULL,
    black_token   TEXT NOT NULL,
    initial_ms    BIGINT NOT NULL,
    increment_ms  BIGINT NOT NULL,
    moves         TEXT NOT NULL DEFAULT '',
    white_ms      BIGINT NOT NULL,
    black_ms      BIGINT NOT NULL,
    -- Unix milliseconds at which the side to move's clock started running,
    -- NULL when the clocks aren't running (before ply 2, or after the end).
    clock_since_unix_ms BIGINT,
    draw_offer    TEXT CHECK (draw_offer IN ('white', 'black')),
    result        TEXT CHECK (result IN ('white_wins', 'black_wins', 'draw')),
    reason        TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
