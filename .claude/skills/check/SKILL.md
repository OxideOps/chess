---
name: check
description: Run the same checks as CI locally (cargo fmt, clippy native + wasm + ts feature, cargo test, then the web lint/check/tests/build) and report results. Use before committing or opening a PR, or when asked "does CI pass?".
---

# Local CI

Run from the repo root, in this order, and stop at the first failure:

```sh
cargo fmt --all                                    # fixes formatting in place
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p chess-core-wasm --target wasm32-unknown-unknown -- -D warnings
cargo clippy --workspace --all-targets --features chess-core-wasm/ts,server/ts -- -D warnings
TEST_DATABASE_URL=postgres://$USER@localhost/chess_test cargo test --workspace
sqlx migrate run --source crates/server/migrations -D postgres://$USER@localhost/chess_test \
  && cargo sqlx prepare --check --workspace -D postgres://$USER@localhost/chess_test
(cd web && corepack pnpm gen:types && git diff --exit-code -- src/lib/generated \
   && corepack pnpm build:wasm && corepack pnpm format >/dev/null && corepack pnpm lint \
   && corepack pnpm check && corepack pnpm test:unit && corepack pnpm build && corepack pnpm test:e2e)
```

Notes:

- `cargo fmt --all` and `pnpm format` (not `--check`) are intentional locally: formatting is
  never worth a round-trip. Mention in the report if they changed files.
- The wasm clippy pass catches the wrapper crate; the native pass catches tests and
  `chess-core`; the `ts` feature pass catches the ts-rs derives. All three run in CI with
  `-D warnings`.
- `git diff --exit-code -- src/lib/generated` fails when Rust types changed but the generated
  TypeScript wasn't committed. Commit the regenerated files; don't hand-edit them.
- `cargo sqlx prepare --check` fails when a `sqlx::query!` changed but `.sqlx/` wasn't
  regenerated: run it without `--check` and commit the result. Builds themselves use the
  cache (`SQLX_OFFLINE=true` in CI; locally either the cache or `DATABASE_URL` works).
  If `--check` reports a query file differing when you didn't touch it (or `Option<i64> as
  u64` errors in `db.rs`), sqlx's plan-based nullability inference changed with the size of
  `chess_test`. `AS "col!"` does not fix the cache (it stores the raw inference); avoid LEFT
  JOINs in `query!` and use scalar subselects instead, as `Db::load` does. To reproduce CI:
  `createdb chess_ci_check`, migrate into it, `cargo sqlx prepare --check` against it, drop it.
- The database tests need a Postgres they can write to. The local one is Homebrew's
  postgresql@17 with a `chess_test` database (`createdb chess_test` once); the URL must name
  the user (`$USER@localhost`), sqlx connects as "anonymous" otherwise. Without
  `TEST_DATABASE_URL` they print "skipping" and pass, which is not the same as passing.
- `web/` needs `wasm-pack` (`cargo install wasm-pack --locked`), `corepack pnpm install` once
  and `corepack pnpm browsers` once (Chromium for Vitest browser mode and Playwright). `pnpm`
  is not on PATH here; go through `corepack`. `test:e2e` builds the site, starts the Rust
  server on the `chess_test` database (online play needs accounts) and runs the real
  Stockfish, so it takes ~30 s. The server command imports the fixture puzzles first
  (idempotent). It has two Playwright projects: `desktop` (everything but
  `phone.e2e.ts`) and `phone` (a Pixel 7 sized touch screen, `phone.e2e.ts` only).
- If `test:e2e` fails with 404s for `_app/immutable/...`, a stale preview server is holding
  the port (it shows up as `vite.js preview`, not `vite preview`): `lsof -ti :4173 | xargs kill`.
- Report the outcome plainly: which steps passed, and the first error verbatim if one failed.
  Do not describe a failing step as done.
