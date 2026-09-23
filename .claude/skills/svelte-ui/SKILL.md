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
  `account.ts` backs `/account` (the nav's username links there; a guest's goes to `/games`):
  load the account, disconnect an identity, set the password, and which providers are left to
  connect; `refusal.ts` turns a `{ "error": … }` reply into an `AuthError`. `email.ts` is the
  email address and password reset calls (`mailEnabled()` decides whether `/account`, `/signup`
  and `/login` show any of it); the links land on `/verify-email` (spends the token on load) and
  `/reset-password` (a new-password form), both reading `?token=`; `/forgot-password` asks for a
  link and says the same thing whatever the address.
- `src/lib/coach/` — `coach.svelte.ts` (`Coach`: whether the server has one, explanations per
  position; injectable fetch). `CoachPanel` shows it under the engine on `/analysis` and hides
  itself when the server has no coach. `CoachAnswer` renders an `Explanation`'s `parts`: the
  server marks the moves an answer names with their UCI path from the explained position, so
  the client never parses chess out of prose. Hover/focus/tap previews the move as an arrow;
  on `/analysis` a click plays the path as a variation (`playHereUci`) and the panel keeps
  the answer up with "Back to the explained position". Keep the `{#each}` in `CoachAnswer`
  free of whitespace between parts, or Svelte renders spaces around every move.
  `CoachFollowUps` sits under an answer: `Coach.followUp(key, question, probe)` posts on the
  answer's `thread`; on a `probe` reply it runs `Prober.probe` (its own Stockfish, started on
  first use, 1.5 s) and asks again with the line. One follow-up at a time (`Coach.following`).
- `src/lib/lessons/` — `drill.svelte.ts` (`DrillSession`: the student's moves, `assessDrill`
  from the WASM after each, the engine's replies through an `OpponentLike`, and `mistake`:
  the engine's score before and after a student move, compared with `isMistake`), `progress.ts`
  (completed lessons in localStorage: a guest's progress, and for an account an outbox of what
  the server hasn't taken yet), `progress.svelte.ts` (`LessonProgress`, `lessonProgress`: the
  finished lessons from the right source — localStorage for guests, `/api/lessons/completed`
  for an account, merging the browser's copy in on each `load()` and then forgetting it; a
  completion is written locally first, so a failed write is retried on the next load; guests
  never touch the server; injectable fetch and account for tests). `src/lib/engine/opponent.svelte.ts` is Stockfish
  playing a side (`search(fen, moves)` → its best move, score and line after `go movetime`).
- `src/lib/sound/` — `cue.ts` (pure: a move's SAN and whether the game ended → which sound),
  `voice.ts` (`WebAudioVoice`: the sounds themselves, synthesised; its constructor takes the
  audio context, so the tests render each cue through an `OfflineAudioContext` and check it
  isn't silence), `sounds.svelte.ts` (`Sounds`: the on/off switch in `localStorage`, an
  injectable `Voice` for tests, and `attach(game)` which sets `GameStore.onmove`). `Board`
  attaches on mount, so every mode gets sound from having a board — don't wire it per page.
  A new sound means a new `Cue`, a case in `voice.ts`, and a line in the audible-cues test.
- `src/lib/review/` — `analysis.svelte.ts` (`GameAnalysis`: Stockfish through every position
  of a finished game, one at a time at `REVIEW_DEPTH` via `Opponent({ depth })`; `stop()`
  terminates the worker, `start()` resumes; each position is saved to localStorage as it is
  done, so a reopened review searches nothing twice; `positionsOf(pgn)`), `links.ts` (query
  strings for the review and `/analysis?pgn=…&ply=N&orientation=…`). The route is
  `/games/[id]/review?side=white`; `reviewGame` from the WASM picks the swings.
- `src/lib/puzzles/` — `session.svelte.ts` (`PuzzleSession`: loads a puzzle — `next()`, of
  `theme` if set, or `daily(date?)` — plays the setup move and the replies, judges each move
  with `judgePuzzle` from the WASM, reports the try and keeps the server's `streak`;
  injectable fetch and delay for tests), `themes.ts` (Lichess theme keys → names),
  `daily.ts`. `components/PuzzleView` is the board and sidebar both puzzle pages share;
  `/puzzles` adds the theme picker (kept in `?theme=`), `/puzzles/daily` the date and link.
- `src/lib/online/` — `lobby.svelte.ts` (`Lobby`: the seek list over `/api/lobby/ws`, same
  injectable-socket shape as `OnlineGame`. The server sends the whole list on every change,
  so there is nothing to merge; `mine` is our own seek, `others` is what is worth clicking.
  The socket *is* the seek, so `/online` holds it open and calls `dispose()` when the page
  goes. A signed-out visitor's socket is anonymous: after `session.ensure()` makes a guest,
  call `reauthenticate()` before acting, or the server still doesn't know who we are —
  anything sent meanwhile is queued and goes out on the new socket),
  `client.svelte.ts` (`OnlineGame`: the server's game mirrored on the
  client over the WebSocket; injectable socket and clock for tests), `clock.ts` (formatting,
  time controls), `invites.ts` (the game link), `listing.ts` (pure helpers for the my-games
  rows). Moves are applied locally first (same rules as the server) and sent; a `Rejected`
  or a gap in plies closes the socket, and the reconnect's `Sync` puts things right.
- `src/lib/components/` — `Board` (props `playAs` and `onmove` for online play; moves by click
  or by drag: pointer events on the squares, `<svelte:window>` for move/up, `squareAt` from
  `squares.ts` for the drop square, a 4 px threshold so a press that doesn't travel stays a
  click, and a swallowed click after a drop; `.square.movable` sets `touch-action: none` so a
  finger on a piece drags instead of scrolling — `phone.e2e.ts` checks that with real touch
  input over the DevTools protocol, since synthetic events can't),
  `PromotionPicker`, `EvalBar`, `EnginePanel`, `ImportPanel`, `MoveList` (lays out
  `view.tree`, the PGN-shaped tokens of the move tree, with `src/lib/chess/movelist.ts`:
  main-line rows plus variation blocks; `button.move` is a main-line cell, `button.var-move`
  a variation move), `Controls`,
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
- Plain CSS, scoped per component. Colours, font sizes, radii and shadows come only from the
  variables in `app.css`; never write a hex value or a rem font size in a component. Text the
  game itself produced (moves, clocks, evaluations, ratings, FENs) is set in `--font-mono`
  with `tabular-nums`; everything else is `--font`. Buttons are `class="btn"`, and the one
  action a screen is asking for is `class="btn primary"` — one per screen. Pages that aren't
  built around a board open with a `.page-header` (an `h1` and one line on what the page is
  for). Form controls are styled globally in `app.css`, so a bare `<input>` is already right.
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

- Vitest 5.0.0 corrupts SvelteKit's `define` values in browser tests (`resolve('/')` becomes
  `""#/`). `vite.config.ts` has a small `vitest500BrowserDefines` plugin on the client test
  project that applies the upstream fix; delete it once Vitest is past 5.0.0 and the Nav
  component tests still pass.

- The service worker (`src/service-worker.ts`) serves the prerendered pages and built assets
  from its cache. After a rebuild, an open tab keeps the old version (no automatic
  `skipWaiting`, so an old page never loads a half-new app) and shows the `UpdateBanner`;
  its Reload asks the waiting worker to take over (`src/lib/pwa/updates.svelte.ts`). In
  Chrome, DevTools → Application → Service workers → "Update on reload" skips all that while
  testing. `e2e/update.e2e.ts` simulates a deploy on its own server over a copy of `build`.
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
