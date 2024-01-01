DROP TABLE IF EXISTS email_verification_tokens;

ALTER TABLE accounts DROP COLUMN IF EXISTS email_verified;

DROP EXTENSION IF EXISTS "uuid-ossp";
