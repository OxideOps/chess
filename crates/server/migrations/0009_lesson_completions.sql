-- The lessons each account has finished, so progress follows the account
-- rather than the browser. `lesson_id` is a drill's stable id from
-- `chess_core::lesson` (not its place in the list). Guests keep theirs in
-- the browser; the client merges it in when they sign up or sign in.
CREATE TABLE lesson_completions (
    user_id     TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    lesson_id   TEXT NOT NULL,
    finished_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, lesson_id)
);
