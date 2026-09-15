---
name: run-app
description: Start the SvelteKit client (web/) locally, verify it in the browser, and stop it. Use when asked to run, serve, or screenshot the app, or to confirm a UI change works for real rather than just compiling.
---

# Run the web client

## Start

From `web/`, in the background (it never exits on its own):

```sh
cd web && corepack pnpm dev --port 5173 --host 127.0.0.1
```

`pnpm dev` first builds `chess-core-wasm` with wasm-pack (dev profile, a few seconds), then
Vite serves with hot reload. Wait for readiness with a poll rather than a fixed sleep:

```sh
for i in $(seq 1 60); do curl -s -o /dev/null --max-time 2 http://127.0.0.1:5173/ && break; sleep 1; done
```

After changing Rust code, run `corepack pnpm build:wasm --dev` (Vite picks up the new
`src/lib/wasm`); after changing Rust types, also `corepack pnpm gen:types`.

Online play (`/online`, `/game/[id]`) needs the Rust server and a database (accounts);
Vite proxies `/api` and the game sockets to it:
`DATABASE_URL=postgres://$USER@localhost/chess_test cargo run -p server` (listens on 8080;
it needs `web/build` to exist, so run `corepack pnpm build` once, or pass any dir with a
`404.html`). Two browsers = two browser contexts/profiles, since the session is a cookie.

For the production build instead: `corepack pnpm build && cargo run -p server -- --static-dir
build --bind 127.0.0.1:4173` (that is exactly what `pnpm test:e2e` starts).

## Verify in the browser

Prefer actually exercising the board over trusting a compile. The fastest reliable way is a
Playwright script run from `web/` (so it can resolve `playwright`), reading state from the DOM:

- Squares are `button[data-square="e2"]`; click one, then the destination.
- Play smoke test: `e2 e4, e7 e5, g1 f3`; `.move-list button.move` reads `e4 e5 Nf3`,
  `.status` reads "Black to move", `.fen input` has the FEN. Focus `.board` and press
  ArrowLeft: status flips to "White to move".
- Promotion: FEN `k7/4P3/8/8/8/8/8/K7 w - - 0 1` via the analysis import, then `e7 e8` opens
  the picker (`button[title=knight]` etc).
- Analysis (`/analysis`): within a few seconds `.engine .name` reads "Stockfish 18 Lite WASM"
  and `.engine .threads` reads "N threads" (N = min(8, cores − 1); absent when N is 1 or the
  page isn't cross-origin isolated, in which case the single-threaded build is running),
  `.engine .summary` reads `Depth N…`, there are 3 `.engine .lines li`, one
  `.board .arrows line`, and `[data-testid=eval-bar] .white` has a height other than 50%.
  Clicking a line plays its first move; moving while in history starts a variation, shown in
  `.move-list .variation` and in `#export-pgn` as `1. e4 (1. d4) …`; "Promote variation" and
  "Delete from here" edit the tree, Shift+↑/↓ switch alternatives.
  A checkmate FEN shows "Idle" and no lines.
- Online (`/online`): pick a time control, "Create game" (a guest session is created for
  you), copy `[data-testid=invite-link]` into a second browser context, click "Join as
  Black"; `[data-testid=game-status]` reads "Your move" / "Waiting for your opponent";
  (in `pnpm dev` the Vite proxy rewrites `Origin`/`Host` to the Rust server's, since it
  refuses game sockets from other origins; a 403 on the socket means that rewrite is gone)
  `[aria-label="White clock"]` gets `.active` once both have moved and shows the player's
  name ("Guest" until they sign up; "Open seat" while nobody holds Black).
- Accounts: the nav's right side reads "Log in · Sign up" (nobody), "Guest · Log in · Sign
  up" (a guest) or the username plus "Log out". `/signup` and `/login` return to `?next=`;
  `/games` lists "You (White) vs …" rows linking to the games.
- OAuth: start the server with `--fake-oauth` (Playwright does) and `/login` shows "Continue
  with Fake provider", which opens a page with a "Sign in as" box and Continue/Cancel; no
  network. Real providers need `CHESS_LICHESS_CLIENT_ID` / the Google pair and, in `pnpm dev`,
  `CHESS_PUBLIC_URL=http://localhost:5173` so the callback comes back through the Vite proxy.
- `e2e/*.e2e.ts` already cover all of the above (the online test drives two pages against
  the real server); `corepack pnpm test:e2e` is often the quickest "does it work" answer.

- Coach: start the server with `--fake-coach` (Playwright does) or a real
  `CHESS_ANTHROPIC_API_KEY`; on `/analysis` a signed-in account gets "Explain this position"
  once the engine reaches depth 16, guests a sign-up hint, and nothing shows without a coach.
  Moves in an answer are `.move-ref` buttons: hovering draws the move, clicking plays its line
  and shows `[data-testid=coach-away]` with "Back to the explained position".
  To judge answer quality, `cargo run -p server --example coach_eval` (real API, ~12¢;
  `-- --dry-run` prints the prompts for free).
- Lessons (`/lessons`): six drills; `/lessons/back-rank-mate` is won by a1→a8 ("Checkmate!",
  then "Next: …" and "✓ Done" on the list). In the others Stockfish answers after
  "Stockfish is thinking…"; `[data-testid=drill-status]` reads "Your move · N moves left".
  A blunder (Qh2-e5+ in `/lessons/queen-mate`) shows `[data-testid=mistake]` with Stockfish's
  better move and, for an account with a coach, "Why was that a mistake?".
- Puzzles (`/puzzles`) need puzzles in the database: `cargo run -p server -- import-puzzles
  crates/server/tests/fixtures/puzzles.csv` (Playwright does this at start). The status line
  reads "Find the best move for White/Black.", then "Correct! Find the next move.", "Solved!"
  or "Not quite: the move was …". `/api/puzzles/next` shows the served puzzle's solution.
- Phone: `corepack pnpm exec playwright test --project phone` runs the layout checks at Pixel
  7 size. To look, a Playwright script with `devices['Pixel 7']` and `page.screenshot` beats
  resizing the developer's Chrome window.
- Offline: after one visit `/`, `/analysis` and a game link load with the network off (the
  `pwa.e2e.ts` test does exactly this). A stale page after a rebuild is the service worker
  keeping the old version until the tabs close; see the `svelte-ui` skill.

With the Chrome tools instead: open the URL in a new tab, screenshot, check the console.
Click coordinates are in *screenshot* pixels, not DOM pixels: multiply DOM coordinates by
`screenshotWidth / window.innerWidth` (~0.59 on the developer's 2560px window). The
developer's Chrome runs Dark Reader, which repaints the light squares near-black; read state
from the DOM rather than trusting colours.

## Stop

```sh
lsof -ti :5173 | xargs kill      # dev server
lsof -ti :4173 | xargs kill      # preview/server, if started
lsof -ti :8080 | xargs kill      # the Rust server, if started
```

Kill by port: the processes show up as `vite.js dev` / `vite.js preview`, so a `pkill -f
"vite dev"` misses them. Close any browser tabs you opened.
