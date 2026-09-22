-- Themes become an array, so the next-puzzle query can filter on one with
-- the GIN index (`themes @> ARRAY['fork']`). They were one space-separated
-- string, straight from the Lichess CSV.
ALTER TABLE puzzles ALTER COLUMN themes DROP DEFAULT;
ALTER TABLE puzzles ALTER COLUMN themes TYPE TEXT[]
    USING CASE WHEN btrim(themes) = '' THEN '{}'::text[]
               ELSE regexp_split_to_array(btrim(themes), '\s+') END;
ALTER TABLE puzzles ALTER COLUMN themes SET DEFAULT '{}';
CREATE INDEX puzzles_themes ON puzzles USING GIN (themes);

-- Puzzles solved in a row, and the most ever. Only counted tries (a user's
-- first at each puzzle) move them, like the rating beside them.
ALTER TABLE puzzle_ratings
    ADD COLUMN streak      INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN best_streak INTEGER NOT NULL DEFAULT 0;

-- The daily puzzle: picked deterministically from the date the first time
-- anyone asks for that day, then remembered, so a day's link keeps showing
-- the same puzzle even after a re-import changes the pool.
CREATE TABLE daily_puzzles (
    day       DATE PRIMARY KEY,
    puzzle_id TEXT NOT NULL REFERENCES puzzles (id) ON DELETE CASCADE
);
