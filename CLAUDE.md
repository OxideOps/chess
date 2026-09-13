# Chess (v2)

A chess site for learning and playing. Rust for the rules and (soon) the server, SvelteKit for
the client. Web only: phones get the same site as a responsive PWA, and there is no desktop
build. Native shells are off the table unless a concrete reason (store listing) appears.
Successor to the archived `OxideOps/chess-v1`.

## Layout

- `crates/chess-core` — rules (via `shakmaty`), `Game` history/navigation, PGN reader, UCI
  engine-output parsing, and the client↔server `protocol` types. No UI, no I/O, must compile
  for `wasm32`. Everything chess-related that both the client and the server need goes here,
  with tests.
- `crates/chess-core-wasm` — wasm-bindgen wrapper around `chess-core` for the client; the only
  crate that knows about JavaScript. Data in, data out, no chess logic: `Game::view()` returns
  one `GameView` snapshot per render. View and protocol types derive ts-rs behind the `ts`
  feature; `pnpm gen:types` writes them to `web/src/lib/generated` (committed, CI checks
  freshness). `pnpm build:wasm` runs wasm-pack into `web/src/lib/wasm` (gitignored).
- `web` — the SvelteKit client. pnpm via corepack, TS strict, plain CSS with the variables in
  `web/src/app.css`. Static SPA: `ssr = false`, every route prerendered as a shell plus a
  `404.html` fallback; the root layout awaits `initChess()` so pages use the WASM
  synchronously. Stockfish (GPL, `web/static/engine`, never linked in) runs in a Web Worker
  behind `src/lib/engine/analysis.svelte.ts`.
- (planned) `crates/server` — axum + sqlx + Postgres. Owns games, clocks, and move validation,
  and serves `web/build`.

## Working on the UI

**Read the `svelte-ui` skill before adding or editing anything in `web/`.** The short version:

- Keep chess logic out of components. They read `GameStore.view`
  (`web/src/lib/chess/game.svelte.ts`) and never decide what is legal. If the UI needs a new
  fact, add it to `chess_core::Game` with a test, expose it in `chess-core-wasm`, run
  `pnpm gen:types` and `pnpm build:wasm`.
- `pnpm dev` from `web/` builds the WASM then serves with hot reload. `pnpm` is reached
  through `corepack pnpm` on machines where it isn't on PATH.

## Skills

Project skills live in `.claude/skills/`: `run-app` (start/verify/stop the client),
`svelte-ui` (conventions and gotchas for `web/`), `check` (local CI). Add a skill when a
workflow has non-obvious steps we'd otherwise rediscover; keep one-line facts in this file
instead.

## Conventions

- Toolchain is pinned in `rust-toolchain.toml`; Node in `web/.nvmrc`; pnpm in
  `web/package.json` (`packageManager`). Never fork dependencies.
- CI runs `cargo fmt --check`, `cargo clippy -D warnings` (native, wasm32, and with the `ts`
  feature), `cargo test`, and for `web/`: binding freshness, `pnpm lint`, `check`,
  `test:unit`, `build`, `test:e2e`. Run the same locally before opening a PR (the `check`
  skill does all of it).
- PRs that finish an issue say `Closes #N` in the description.
- Piece images are Colin M.L. Burnett's `cburnett` set (CC BY-SA 3.0). Any new assets need a
  license compatible with MIT noted in `README.md`. Stockfish.js is GPLv3 and stays a separate
  worker program.
