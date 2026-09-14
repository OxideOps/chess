---
name: svelte-ui
description: How to write or change UI in web/ (SvelteKit 2, Svelte 5 runes, TypeScript, the chess-core WASM boundary) — conventions, where things live, and the gotchas already hit in this repo. Use before adding or editing any component, route, store, style, or test in web/.
---

# Svelte UI in this repo

## Where things live

- `src/routes/` — pages. `+layout.ts` sets `ssr = false`, `prerender = true`, and awaits
  `initChess()`, so every page can use the WASM synchronously. `+error.svelte` is the 404.
- `src/lib/chess/` — the only code that touches the WASM. `wasm.ts` loads it and types the
  helpers; `game.svelte.ts` is `GameStore`, a runes class holding a WASM `Game` and a
  `GameView` snapshot; `squares.ts` is board geometry; `status.ts` renders the status line.
- `src/lib/engine/` — `worker.ts` (Stockfish in a Web Worker) and `analysis.svelte.ts`
  (`Analyser`: request a FEN, read `lines`; injectable engine for tests).
- `src/lib/auth/` — `session.svelte.ts` is the `Session` store (`session.user`, loaded once
  by the root layout from `GET /api/me`; `ensure()` makes a guest on demand; `signup`,
  `login`, `logout`; injectable fetch for tests). `next.ts` validates `?next=` return paths
  and types them as `ResolvedPathname`, which is what the `no-navigation-without-resolve`
  lint rule accepts for an `href` or `goto()` built from a string. `providers.ts` lists the
  OAuth providers (`GET /api/auth/providers`) and builds their start URLs; those links are
  plain `<a rel="external">` (full-page navigation to an API URL, which the same rule allows).
- `src/lib/puzzles/` — `session.svelte.ts` (`PuzzleSession`: loads a puzzle, plays the setup
  move and the replies, judges each move with `judgePuzzle` from the WASM, reports the first
  try; injectable fetch and delay for tests).
- `src/lib/online/` — `client.svelte.ts` (`OnlineGame`: the server's game mirrored on the
  client over the WebSocket; injectable socket and clock for tests), `clock.ts` (formatting,
  time controls), `invites.ts` (the game link), `listing.ts` (pure helpers for the my-games
  rows). Moves are applied locally first (same rules as the server) and sent; a `Rejected`
  or a gap in plies closes the socket, and the reconnect's `Sync` puts things right.
- `src/lib/components/` — `Board` (props `playAs` and `onmove` for online play),
  `PromotionPicker`, `EvalBar`, `EnginePanel`, `ImportPanel`, `MoveList`, `Controls`,
  `Clock` (side + player name), `GameSidebar` (everything beside an online board: status,
  invite, join, draw offers, controls), `AuthForm` (signup and login), `Nav` (links plus the
  account area). Each has its own scoped `<style>`; shared
  layout classes (`.sidebar`, `.status`) and the colour variables are in `src/app.css`.
- `src/lib/generated/` — TypeScript types generated from Rust by ts-rs. Never edit; run
  `corepack pnpm gen:types` after changing Rust types and commit the output.
- `static/pieces/cburnett/` (CC BY-SA) and `static/engine/` (Stockfish.js, GPL) are served as-is.
  The engine binaries are not committed: `scripts/fetch-engine.mjs` (run by `build:wasm`, so by
  `dev` and `build`) downloads both lite builds from the stockfish.js release and checks their
  SHA-256; bump the tag and hashes there to upgrade. `src/lib/engine/build.ts` picks the
  multi-threaded build when the page is cross-origin isolated (the Rust server and the Vite
  dev server send COOP/COEP; `preview` does too).

## Conventions

- Chess logic lives in Rust. Components read `game.view` and call `GameStore` methods
  (`play`, `playHere`, `goToPly`, …). If the UI needs a new fact (e.g. `movetext`, `pgn`),
  add it to `chess_core::Game` with a test, add it to `GameView` in `chess-core-wasm`, then
  `corepack pnpm gen:types && corepack pnpm build:wasm`.
- State is Svelte 5 runes. Stores are classes in `.svelte.ts` files with `$state` fields
  (`GameStore`, `Analyser`); components use `$props()`, `$derived`, `$bindable()`.
- Props are typed with an `interface Props`; callbacks are plain function props (`onpick`),
  not events.
- Plain CSS, scoped per component, colours only through the variables in `app.css`.
- Layout works down to phone width (Playwright's `phone` project checks it). Pages built
  around a board put `board-page` on their wrapper; the board and anything that should match
  it use `width: var(--board-size)`, and the page sets `--board-beside` / `--board-around`
  for what sits next to or above and below the board (eval bar, clocks). Anything tappable
  gets `min-height: var(--tap)` (44px on touch screens, 0 otherwise). A flex item holding
  unbreakable text (engine lines) needs `min-width: 0`, or it widens the page on phones.
- Interactive things are `<button type="button">` (the board squares included) so no a11y
  suppressions are needed. Don't add `svelte-ignore` comments; fix the markup.
- Tests: `*.svelte.spec.ts` run in real Chromium (Vitest browser mode) and may use the WASM
  after `await initChess()`; `*.spec.ts` run in Node (pure TS only); `e2e/*.e2e.ts` run
  Playwright against the production build served by the Rust server (`cargo run -p server`
  in `playwright.config.ts`), so they cover the engine, the API and the game sockets.
- Dynamic routes (`/game/[id]`) set `prerender = false`; the server and the fallback shell
  handle them. `ts-rs` maps `u64` to `bigint`: annotate millisecond fields with
  `#[ts(type = "number")]`.

## Gotchas that already cost a cycle

- The service worker (`src/service-worker.ts`) serves the prerendered pages and built assets
  from its cache. After a rebuild, an open tab keeps the old version until every tab of the
  site closes (no `skipWaiting`, so an old page never loads a half-new app); in Chrome,
  DevTools → Application → Service workers → "Update on reload" skips that while testing.
- Engine files must be answered with a newly built `Response`, not the cached one: the
  Stockfish loader reads its role from the URL fragment (`#…wasm,worker` for its threads), a
  worker's URL is its response's URL, and cached responses have no fragment. Getting this
  wrong makes every thread spawn threads, forever.
- The offline fallback for other routes is `404.html`, not `/`: prerendered pages use
  relative asset paths (`./_app/…`), so they only work at their own URL.

- A `$derived` that calls a `GameStore` method (not a `$state` read) won't recompute when the
  game changes. Read something reactive first, e.g. `void view.fen;` — see `destinations` in
  `Board.svelte` — or put the value on `GameView`.
- wasm-bindgen types JSON-shaped returns as `any`. Cast once in `src/lib/chess/wasm.ts` with
  the generated type; nothing else should cast.
- `serde_wasm_bindgen::Serializer::json_compatible()` is what makes `None` become `null`,
  matching the ts-rs types (`T | null`). Keep it.
- The Node test project can't load the WASM (`fetch` of a file URL); anything touching it
  goes in a `.svelte.spec.ts`.
- ESLint scans `static/` unless ignored (it is, in `eslint.config.js`), and Prettier ignores
  `static/` and `src/lib/generated/` via `.prettierignore`.
- `pnpm` is not on PATH on the developer's machine; use `corepack pnpm`. Playwright's web
  server command therefore uses `npm run`, which exists wherever Node does.
- Stale `vite preview` servers keep old builds alive on the port and produce 404s for
  `_app/immutable/...`; kill by port (`lsof -ti :4173 | xargs kill`).

## Verify

```sh
cd web && corepack pnpm format >/dev/null && corepack pnpm lint && corepack pnpm check \
  && corepack pnpm test:unit && corepack pnpm test:e2e
```

Then run it and click through the change (see the `run-app` skill). A passing `check` is not
proof that a component renders or that a handler fires.
