# Chess

A chess site that is good at teaching you chess and lets you play other people. The rules and
the server are Rust; the client is SvelteKit, with the Rust rules compiled to WebAssembly. It
is a web app, on phones too (responsive, installable as a PWA). There is no desktop or native
mobile build, and none planned unless a store listing becomes worth the cost.

This is the second attempt. The first (`OxideOps/chess-v1`, archived) taught us what not to do:
a hand-written rules engine, a forked UI framework, and a server that trusted clients. This one
uses a proven rules library, an off-the-shelf UI stack, and will keep the server in charge of
every game.

## Status

Early scaffolding. What works today:

- `chess-core`: full rules via [shakmaty](https://github.com/niklasf/shakmaty), game history with
  navigation, FEN in/out, SAN/UCI, promotion handling, repetition detection, and a first draft of
  the WebSocket protocol. Tested.
- `chess-core` also reads PGN (tags, comments, variations skipped, NAGs) and parses UCI engine
  output (`info` lines, scores, principal variations).
- `server`: an axum binary that serves the client build (clean URLs, 404 fallback,
  precompressed assets, cross-origin isolation headers) and hosts games in memory: create a
  game over HTTP, play it over a WebSocket. The server validates every move with `chess-core`,
  runs the clocks and flags on time, and handles resignation and draw offers. With
  `DATABASE_URL` set, every change is written through to Postgres and games survive restarts
  (clocks included); without it they live in memory.
- `server`: accounts. Guests are created on demand (no signup needed to play), can upgrade to
  a username + password (argon2id) and keep their games; sessions are server-side rows behind
  an `HttpOnly` cookie. Games have seats: the creator is White, the first person to join is
  Black, everyone else spectates; `GET /api/me/games` lists yours.
- `web`: online play (`/online`): pick a time control, create a game, send the link; the
  opponent clicks "Join as Black". The game page shows both clocks counting down with the
  players' names, your side only, draw offers, resign, and the result; reconnecting resumes;
  anyone else spectates. Online play needs the database (`DATABASE_URL`); the Vite dev proxy
  and the e2e suite start the server on `chess_test`.
- `web`: accounts. The nav shows who you are (a name, or *Guest*); `/signup` and `/login`
  take you back where you were (`?next=`); a guest who signs up keeps their games; `/games`
  lists your games with the result from your side.
- `web`: a local two-player board with legal-move hints, promotion picker, move list, history
  navigation (buttons and arrow keys), flip, and a FEN readout.
- `web`: an analysis board (`/analysis`) with Stockfish 18 running in a Web Worker, an eval
  bar, the top three lines (click one to play it), a best-move arrow, FEN/PGN import, PGN
  export, and playing from any point in the history.

## Roadmap

1. ~~Workspace, rules crate, board component~~
2. ~~Analysis board: FEN/PGN import, Stockfish in the browser (WASM), eval bar, best-move arrows~~
   (still open: [multi-threaded engine](https://github.com/OxideOps/chess/issues/2),
   [variations](https://github.com/OxideOps/chess/issues/3),
   [responsive layout and PWA](https://github.com/OxideOps/chess/issues/9))
3. [Server](https://github.com/OxideOps/chess/issues/5) (axum + Postgres) that owns games:
   validation, clocks, reconnects, persistence (done; accounts are the next roadmap item)
4. [Accounts and sessions](https://github.com/OxideOps/chess/issues/6)
5. [Ratings, puzzles, lessons, AI coach](https://github.com/OxideOps/chess/issues/7): ratings,
   then puzzles (Lichess's CC0 puzzle database), then lessons and an AI coach that explains
   engine analysis in plain language

Open work is tracked in [GitHub issues](https://github.com/OxideOps/chess/issues).

## Development

Requirements:

- Rust via [rustup](https://rustup.rs) — the toolchain and `wasm32` target come from
  `rust-toolchain.toml` automatically — plus `cargo install wasm-pack --locked`.
- Node 22 and pnpm (`corepack enable` picks up the pinned version).

Run the client with hot reload (see `web/README.md` for the rest of the commands):

```sh
cd web
pnpm install && pnpm browsers   # first time
pnpm dev                        # builds chess-core to WASM, then serves
```

Serve the production build the way the real server will (after `pnpm build` in `web/`):

```sh
cargo run -p server                 # http://127.0.0.1:8080, --bind and --static-dir to change
DATABASE_URL=postgres://$USER@localhost/chess cargo run -p server   # …and keep games in Postgres
```

The server applies its own migrations (`crates/server/migrations`) on start. The database
tests need a database they may write to: `createdb chess_test` and
`TEST_DATABASE_URL=postgres://$USER@localhost/chess_test cargo test -p server`; they skip
when the variable is unset. (sqlx needs the user in the URL; it doesn't default to yours.)

Queries are checked at compile time (`sqlx::query!`) against the committed `.sqlx` cache, so
building needs no database. After changing a query or a migration, refresh the cache:

```sh
sqlx migrate run --source crates/server/migrations -D postgres://$USER@localhost/chess_test
cargo sqlx prepare --workspace -D postgres://$USER@localhost/chess_test   # commit .sqlx/
```

(`cargo install sqlx-cli --no-default-features --features postgres,rustls` once.) CI fails if
the cache is stale.

For online play in development, run the server next to `pnpm dev` (Vite proxies `/api` to
it): `cargo run -p server` in another terminal, then open http://localhost:5173/online in two
browsers. The wire protocol is in `crates/chess-core/src/protocol.rs`; to drive it by hand:

```sh
curl -s -c jar -X POST localhost:8080/api/auth/guest            # a session cookie
curl -s -b jar -X POST localhost:8080/api/games -H 'content-type: application/json' \
     -d '{"initial_ms": 300000, "increment_ms": 2000}'            # → {"id":"…"}, you are White
# a second cookie jar POSTs /api/games/<id>/join to take Black; then connect a WebSocket
# to /api/games/<id>/ws with the cookie and send {"type":"move","uci":"e2e4"}
```

Checks, same as CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p chess-core-wasm --target wasm32-unknown-unknown -- -D warnings
cargo clippy --workspace --all-targets --features chess-core-wasm/ts -- -D warnings
cargo test --workspace
(cd web && pnpm gen:types && pnpm build:wasm && pnpm lint && pnpm check && pnpm test && pnpm build)
```

`pnpm gen:types` regenerates `web/src/lib/generated` (TypeScript for the protocol and view
types, via ts-rs); CI fails if the committed files are stale.

## Layout

```
crates/
  chess-core/       rules, Game (history + navigation), PGN, UCI parsing, client/server protocol — no UI, no I/O
  chess-core-wasm/  wasm-bindgen wrapper around chess-core for web/ (built by `pnpm build:wasm`)
  server/           axum binary: serves web/build today, will own games (roadmap 3)
web/                SvelteKit client (pnpm; static SPA, prerendered shells)
  src/lib/chess/    WASM loader and the reactive GameStore
  src/lib/engine/   Stockfish worker and the Analyser store
  src/lib/generated/  TypeScript types generated from Rust (ts-rs)
  static/engine/    Stockfish.js build loaded as a Web Worker (not linked into the app)
```

## Licenses

Code is MIT (see `LICENSE`). Piece images in `web/static/pieces/cburnett` are by Colin M.L.
Burnett, licensed [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/).

The engine in `web/static/engine` is [Stockfish.js](https://github.com/nmrugg/stockfish.js)
(Stockfish 18, lite single-threaded build), GPLv3 — see `COPYING.txt` there. It runs as a
separate Web Worker program that the app talks to over UCI text; it is not linked into the
MIT-licensed code. Redistributing the site means redistributing that build under the GPL.
