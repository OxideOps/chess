---
name: check
description: Run the same checks as CI locally (fmt, clippy native + wasm, tests, dx build) and report results. Use before committing or opening a PR, or when asked "does CI pass?".
---

# Local CI

Run from the repo root, in this order, and stop at the first failure:

```sh
cargo fmt --all                                    # fixes formatting in place
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p app --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
(cd crates/app && dx build)
(cd web && corepack pnpm format >/dev/null && corepack pnpm lint && corepack pnpm check \
   && corepack pnpm test:unit && corepack pnpm build && corepack pnpm test:e2e)
```

Notes:

- `cargo fmt --all` (not `--check`) is intentional locally: formatting is never worth a
  round-trip. Mention in the report if it changed files.
- The wasm clippy pass catches web-only code; the native pass catches tests and `chess-core`.
  Both must be clean because CI runs both with `-D warnings`.
- `dx build` needs the Dioxus CLI (see `README.md`). If it isn't installed, say so rather than
  skipping silently; the other four checks still count.
- `web/` needs `corepack pnpm install` once and `corepack pnpm browsers` once (Chromium for
  Vitest browser mode and Playwright). `pnpm` is not on PATH here; go through `corepack`.
  `format` (not `--check`) is intentional, as with `cargo fmt`. `test:e2e` builds and
  previews the site itself.
- Report the outcome plainly: which steps passed, and the first error verbatim if one failed.
  Do not describe a failing step as done.
