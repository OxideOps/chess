-- `aborted` is a result too: no winner, the game was called off because a
-- player left before both sides had moved. 0001 only allowed the other three,
-- so aborted games failed to save.
ALTER TABLE games DROP CONSTRAINT games_result_check;
ALTER TABLE games ADD CONSTRAINT games_result_check
    CHECK (result IN ('white_wins', 'black_wins', 'draw', 'aborted'));
