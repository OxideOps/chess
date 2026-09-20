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
  (clocks included); without it they live in memory. A player who disconnects has 60 s to come
  back (`CHESS_ABANDON_AFTER_SECS`) while their opponent is there: then the game is aborted
  if both sides hadn't moved yet, or lost by abandonment. Both players see the countdown;
  reconnecting cancels it.
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
- `server`: hardening. Logins are rate limited per username and per client address, signups
  and guest creation per address (429 with `Retry-After`); expired sessions and guests who
  never played are swept hourly; game sockets refuse cross-origin upgrades. Behind a reverse
  proxy set `CHESS_TRUST_PROXY=true` (client addresses from `X-Forwarded-For`),
  `CHESS_SECURE_COOKIES=true` over https, and `CHESS_ALLOWED_ORIGINS=https://your.host` if the
  proxy rewrites `Host`.
- `server` + `web`: ratings. Games are rated or casual (rated needs an account on both sides;
  aborted games never count). Glicko-2 per category (bullet, blitz, rapid, classical, by
  initial time + 40 × increment), one update per game, deviation growing back while idle,
  "?" while provisional. Ratings show next to names on the clocks with the change at the end,
  in the games list, and on profile pages (`/players/<name>`).
- `server` + `web`: puzzles (`/puzzles`) from the Lichess puzzle database. You get one near
  your puzzle rating that you haven't tried, the opponent's setup and reply moves play
  themselves, and any checkmate counts as a solution. A wrong move names the right one, with
  "Show solution" to play out the rest. Your first try at each puzzle moves your puzzle rating
  (Glicko-2 against the puzzle's rating), guests included; profiles show it.
- `server` + `web`: a coach on the analysis board. "Explain this position" asks Claude to talk
  through Stockfish's best lines in plain language; the server turns the lines into SAN and
  words, spells out the board (what attacks what, pins, where the kings can go) and each move
  of the best line, and tells the model to explain them, not to invent its own variations.
  Moves in an answer show on the board and play their line; follow-up questions carry the
  conversation on, and a move the engine's lines didn't cover gets a Stockfish look first.
  `cargo run -p server --example coach_eval` checks it against a fixed set of positions.
  Accounts only, 30 fresh explanations per user per hour, answers cached per position. Set
  `CHESS_ANTHROPIC_API_KEY` to turn it on (`CHESS_COACH_MODEL`, default `claude-opus-5`;
  `CHESS_COACH_PER_HOUR`); `--fake-coach` gives an offline stand-in for development.
- `web`: lessons (`/lessons`): six short drills against Stockfish from set positions, easiest
  first: a back-rank mate in one, mating with two rooks, with king and queen and with king and
  rook, winning a king and pawn ending, and holding one to a draw. Each has a move budget and
  a short explanation of the technique; chess-core judges when a drill is won or lost
  (stalemate, a lost queen, a promoted pawn, running out of moves). Completed lessons are
  remembered in the browser. A move that throws away the win (or the draw) is pointed out with
  Stockfish's better move, judged from the engine's own searches (a drop in winning chances,
  `Score::is_mistake`); accounts can ask the coach why.
- `server` + `web`: sign in with Lichess or Google. `CHESS_LICHESS_CLIENT_ID=<any name>`
  turns on Lichess (a public client: PKCE, no secret, no registration);
  `CHESS_GOOGLE_CLIENT_ID` + `CHESS_GOOGLE_CLIENT_SECRET` turn on Google (register
  `<public url>/api/auth/google/callback` in the Cloud console — `docs/deploy.md` has the
  whole click-through); `CHESS_PUBLIC_URL` is where
  browsers reach the server (defaults to the request's host). Provider accounts become users
  named after them (a suffix if the name is taken), a signed-in guest is upgraded in place, a
  signed-in account gets the provider linked. `--fake-oauth` adds a built-in provider that
  signs in as any name you type, for development and the e2e suite; never in production.
- `web`: a local two-player board with legal-move hints, promotion picker, move list, history
  navigation (buttons and arrow keys), flip, and a FEN readout.
- `web`: phones get the same site. Below 700px the panels stack under the board, the board is
  sized to the screen (width and height), and buttons are thumb-sized on touch screens. It
  installs as an app (manifest, icons, an "Install app" button where the browser offers it)
  and works offline after one visit: a service worker caches the app shell per deploy and the
  engine build the browser runs (kept across deploys). After a deploy, open pages show "A new
  version of the site is available · Reload" (checked when the tab comes back into view and
  every half hour); nothing reloads by itself, and the banner waits during an online game.
  If the multi-threaded engine can't start (e.g. a phone that won't give it
  the shared memory), the single-threaded one takes its place.
- `web`: an analysis board (`/analysis`) with Stockfish 18 running in a Web Worker (the
  multi-threaded build with one thread per spare core when the page is cross-origin
  isolated, which the server arranges; single-threaded otherwise), an eval
  bar, the top three lines (click one to play it), a best-move arrow, FEN/PGN import, PGN
  export, and variations: a move played from earlier in the game starts a variation instead of
  replacing what came after. The move list shows variations (nested ones too) under the move
  they branch from; click any move to go there, promote a variation or delete from a move,
  and switch between alternatives with Shift+↑/↓. PGN import and export keep them.

## Roadmap

1. ~~Workspace, rules crate, board component~~
2. ~~Analysis board: FEN/PGN import with variations, Stockfish in the browser
   (multi-threaded when possible), eval bar, best-move arrows; phones and offline (PWA)~~
3. ~~Server (axum + Postgres) that owns games: validation, clocks, reconnects and abandonment,
   persistence~~
4. ~~Accounts and sessions: guests, passwords, Lichess and Google sign-in~~
5. ~~Ratings, puzzles, lessons, AI coach~~: Glicko-2 ratings, puzzles from Lichess's CC0
   database, drills against Stockfish, and a coach that explains engine analysis in plain
   language

Open work is tracked in [GitHub issues](https://github.com/OxideOps/chess/issues).

## Deploying

The whole site is one container: the Rust server with the client build baked in. Run that
container here, exactly as a host would, with

```sh
docker compose up --build        # http://localhost:8080
```

[`docs/deploy.md`](docs/deploy.md) is the runbook: the first deploy, the secrets and the OAuth
client, importing the real puzzle database, checking a deploy, rolling a bad one back, and
backups. Two things there are easy to get wrong and quiet when they are: the site must run as
**one instance** (a game in progress lives in that process's memory), and whatever sits in
front of it must not strip the COOP/COEP headers, or every visitor silently drops to the
single-threaded engine.

Merging to main deploys, once the repository has the `FLY_APP` variable and `FLY_API_TOKEN`
secret; until then the workflow does nothing.

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

Puzzles have to be imported once. For development the test fixture (46 real puzzles) is
enough; for a real deployment load the Lichess database. By default the import keeps popular,
often-played puzzles with a settled rating, at most 10 000 per 100 rating points, so every
level is covered (see `--help` for the filters):

```sh
DATABASE_URL=… cargo run -p server -- import-puzzles crates/server/tests/fixtures/puzzles.csv
curl -O https://database.lichess.org/lichess_db_puzzle.csv.zst
zstd -dc lichess_db_puzzle.csv.zst | DATABASE_URL=… cargo run --release -p server -- import-puzzles -
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
  server/           axum binary: serves web/build, owns games, accounts, ratings, puzzles, the coach
web/                SvelteKit client (pnpm; static SPA, prerendered shells)
  src/lib/chess/    WASM loader and the reactive GameStore
  src/lib/engine/   Stockfish worker and the Analyser store
  src/lib/generated/  TypeScript types generated from Rust (ts-rs)
  static/engine/    Stockfish.js build loaded as a Web Worker (not linked into the app)
Dockerfile          the whole site as one image (server + client build)
compose.yaml        that image plus a Postgres, for running the deployment locally
fly.toml            the deployment's configuration
docs/deploy.md      how to deploy it, and what to check afterwards
```

## Licenses

Code is MIT (see `LICENSE`). Piece images in `web/static/pieces/cburnett` are by Colin M.L.
Burnett, licensed [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/). The app
icons (`web/static/icons`, `web/src/lib/assets/favicon.svg`) are the site's own mark — a rook
drawn in the board's right angles — and are MIT with the rest of the code;
`web/scripts/gen-icons.mjs` renders them from the same shape as
`web/src/lib/components/Logo.svelte`.

The interface is set in [IBM Plex](https://github.com/IBM/plex) Sans and Mono, licensed
[OFL 1.1](https://github.com/IBM/plex/blob/master/LICENSE.txt). `web/static/fonts` holds the
latin subsets as woff2, self-hosted so the offline PWA has its type too.

Puzzles come from the [Lichess puzzle database](https://database.lichess.org/#puzzles), which
is CC0 (public domain); thanks to Lichess and its players for it. The test fixture
`crates/server/tests/fixtures/puzzles.csv` is a sample of it.

The engine in `web/static/engine` is [Stockfish.js](https://github.com/nmrugg/stockfish.js)
(Stockfish 18, the lite multi-threaded and lite single-threaded builds from the v18.0.0
release), GPLv3 — see `COPYING.txt` there. It runs as a separate Web Worker program that the
app talks to over UCI text; it is not linked into the MIT-licensed code. Redistributing the
site means redistributing those builds under the GPL. The multi-threaded build runs when the
page is cross-origin isolated (the Rust server and the Vite dev server send the COOP/COEP
headers), with one search thread per spare core, up to 8; otherwise the single-threaded build
is used.
