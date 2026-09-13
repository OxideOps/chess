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
cargo clippy --workspace --all-targets --features chess-core-wasm/ts -- -D warnings
cargo test --workspace
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
- `web/` needs `wasm-pack` (`cargo install wasm-pack --locked`), `corepack pnpm install` once
  and `corepack pnpm browsers` once (Chromium for Vitest browser mode and Playwright). `pnpm`
  is not on PATH here; go through `corepack`. `test:e2e` builds and previews the site itself
  and runs the real Stockfish, so it takes ~10 s.
- If `test:e2e` fails with 404s for `_app/immutable/...`, a stale preview server is holding
  the port (it shows up as `vite.js preview`, not `vite preview`): `lsof -ti :4173 | xargs kill`.
- Report the outcome plainly: which steps passed, and the first error verbatim if one failed.
  Do not describe a failing step as done.
