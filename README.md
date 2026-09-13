# Chess

A chess site that is good at teaching you chess and lets you play other people, built in Rust.
It is a web app, on phones too (responsive, installable as a PWA). There is no desktop or
native mobile build, and none planned unless a store listing becomes worth the cost.

This is the second attempt. The first (`OxideOps/chess-v1`, archived) taught us what not to do:
a hand-written rules engine, a forked UI framework, and a server that trusted clients. This one
uses a proven rules library, tracks Dioxus releases without forking, and will keep the server in
charge of every game.

## Status

Early scaffolding. What works today:

- `chess-core`: full rules via [shakmaty](https://github.com/niklasf/shakmaty), game history with
  navigation, FEN in/out, SAN/UCI, promotion handling, repetition detection, and a first draft of
  the WebSocket protocol. Tested.
- `chess-core` also reads PGN (tags, comments, variations skipped, NAGs) and parses UCI engine
  output (`info` lines, scores, principal variations).
- `app`: a local two-player board in the browser with legal-move hints, promotion picker,
  move list, history navigation (buttons and arrow keys), flip, and a FEN readout.
- `app`: an analysis board (`/analysis`) with Stockfish 18 running in a Web Worker, an eval
  bar, the top three lines (click one to play it), a best-move arrow, FEN/PGN import, PGN
  export, and playing from any point in the history.

## Roadmap

1. ~~Workspace, rules crate, board component~~
2. ~~Analysis board: FEN/PGN import, Stockfish in the browser (WASM), eval bar, best-move arrows~~
   (still open: [multi-threaded engine](https://github.com/OxideOps/chess/issues/2),
   [variations](https://github.com/OxideOps/chess/issues/3),
   [responsive layout and PWA](https://github.com/OxideOps/chess/issues/9))
3. [Server](https://github.com/OxideOps/chess/issues/5) (axum + Postgres) that owns games:
   validation, clocks, reconnects, persistence
4. [Accounts and sessions](https://github.com/OxideOps/chess/issues/6)
5. [Ratings, puzzles, lessons, AI coach](https://github.com/OxideOps/chess/issues/7): ratings,
   then puzzles (Lichess's CC0 puzzle database), then lessons and an AI coach that explains
   engine analysis in plain language

Open work is tracked in [GitHub issues](https://github.com/OxideOps/chess/issues).

## Frontend migration

The Dioxus client in `crates/app` is being replaced by a SvelteKit app in `web/`
([plan](https://github.com/OxideOps/chess/issues/12)). Rust stays for `chess-core` and the
server. Until the port is complete, `crates/app` is the working app and `web/` is a shell.

## Development

Requirements:

- Rust via [rustup](https://rustup.rs) — the toolchain and `wasm32` target come from
  `rust-toolchain.toml` automatically.
- The Dioxus CLI: `cargo install dioxus-cli --version 0.7.10 --locked`
  (or `cargo binstall dioxus-cli@0.7.10` for a prebuilt binary).

Run the web client with hot reload:

```sh
cd crates/app
dx serve
```

Run the SvelteKit client (Node 22, pnpm via `corepack enable`, and
`cargo install wasm-pack --locked`; see `web/README.md`):

```sh
cd web
pnpm install && pnpm browsers   # first time
pnpm dev                        # builds chess-core to WASM, then serves
```

Checks, same as CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p app --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
(cd web && pnpm gen:types && pnpm build:wasm && pnpm lint && pnpm check && pnpm test && pnpm build)
```

`pnpm gen:types` regenerates `web/src/lib/generated` (TypeScript for the protocol and view
types, via ts-rs); CI fails if the committed files are stale.

## Layout

```
crates/
  chess-core/   rules, Game (history + navigation), PGN, UCI parsing, client/server protocol — no UI, no I/O
  chess-core-wasm/  wasm-bindgen wrapper around chess-core for web/ (built by `pnpm build:wasm`)
  app/          Dioxus web client (being replaced by web/; type-checks on the host, ships as WASM)
web/            SvelteKit client (pnpm; static SPA, prerendered shells)
    assets/engine/  Stockfish.js build loaded as a Web Worker (not linked into the app)
docs/
  dioxus-0.7.md quick reference for the Dioxus version in use
```

## Licenses

Code is MIT (see `LICENSE`). Piece images in `crates/app/assets/pieces/cburnett` are by
Colin M.L. Burnett, licensed [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/).

The engine in `crates/app/assets/engine` is [Stockfish.js](https://github.com/nmrugg/stockfish.js)
(Stockfish 18, lite single-threaded build), GPLv3 — see `COPYING.txt` there. It runs as a
separate Web Worker program that the app talks to over UCI text; it is not linked into the
MIT-licensed binary. Redistributing the site means redistributing that build under the GPL.
