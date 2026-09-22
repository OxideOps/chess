-- What the provider calls the account ("dillon" on Lichess, an email at
-- Google), so the account page can say which one is connected. Refreshed at
-- every sign-in; rows linked before this column existed get it the next time
-- they are used.
ALTER TABLE identities ADD COLUMN label TEXT;
