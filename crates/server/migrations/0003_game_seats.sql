-- Games belong to users now: seats instead of secret tokens.
ALTER TABLE games
    DROP COLUMN white_token,
    DROP COLUMN black_token,
    ADD COLUMN white_user_id TEXT REFERENCES users (id),
    ADD COLUMN black_user_id TEXT REFERENCES users (id);

CREATE INDEX games_white_user_id ON games (white_user_id);
CREATE INDEX games_black_user_id ON games (black_user_id);
