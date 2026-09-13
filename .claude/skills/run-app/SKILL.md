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

For the production build instead: `corepack pnpm build && corepack pnpm preview --port 4173`.

## Verify in the browser

Prefer actually exercising the board over trusting a compile. The fastest reliable way is a
Playwright script run from `web/` (so it can resolve `playwright`), reading state from the DOM:

- Squares are `button[data-square="e2"]`; click one, then the destination.
- Play smoke test: `e2 e4, e7 e5, g1 f3`; `.move-list button.move` reads `e4 e5 Nf3`,
  `.status` reads "Black to move", `.fen input` has the FEN. Focus `.board` and press
  ArrowLeft: status flips to "White to move".
- Promotion: FEN `k7/4P3/8/8/8/8/8/K7 w - - 0 1` via the analysis import, then `e7 e8` opens
  the picker (`button[title=knight]` etc).
- Analysis (`/analysis`): within a few seconds `.engine .name` reads "Stockfish 18 Lite WASM",
  `.engine .summary` reads `Depth N…`, there are 3 `.engine .lines li`, one
  `.board .arrows line`, and `[data-testid=eval-bar] .white` has a height other than 50%.
  Clicking a line plays its first move; moving while in history truncates (`#export-pgn`).
  A checkmate FEN shows "Idle" and no lines.
- `e2e/*.e2e.ts` already cover all of the above; `corepack pnpm test:e2e` is often the
  quickest "does it work" answer.

With the Chrome tools instead: open the URL in a new tab, screenshot, check the console.
Click coordinates are in *screenshot* pixels, not DOM pixels: multiply DOM coordinates by
`screenshotWidth / window.innerWidth` (~0.59 on the developer's 2560px window). The
developer's Chrome runs Dark Reader, which repaints the light squares near-black; read state
from the DOM rather than trusting colours.

## Stop

```sh
lsof -ti :5173 | xargs kill      # dev server
lsof -ti :4173 | xargs kill      # preview, if started
```

Kill by port: the processes show up as `vite.js dev` / `vite.js preview`, so a `pkill -f
"vite dev"` misses them. Close any browser tabs you opened.
