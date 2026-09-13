# Chess

A chess site that is good at teaching you chess and lets you play other people, built in Rust.

This is the second attempt. The first (`OxideOps/chess-v1`, archived) taught us what not to do:
a hand-written rules engine, a forked UI framework, and a server that trusted clients. This one
uses a proven rules library, tracks Dioxus releases without forking, and will keep the server in
charge of every game.

## Status

Early scaffolding. What works today:

- `chess-core`: full rules via [shakmaty](https://github.com/niklasf/shakmaty), game history with
  navigation, FEN in/out, SAN/UCI, promotion handling, repetition detection, and a first draft of
  the WebSocket protocol. Tested.
- `app`: a local two-player board in the browser with legal-move hints, promotion picker,
  move list, history navigation (buttons and arrow keys), flip, and a FEN readout.

## Roadmap

1. ~~Workspace, rules crate, board component~~
2. Analysis board: FEN/PGN import, Stockfish in the browser (WASM), eval bar, best-move arrows
3. Server (axum + Postgres) that owns games: validation, clocks, reconnects, persistence
4. Accounts and sessions
5. Ratings, then puzzles (Lichess's CC0 puzzle database), then lessons and an AI coach that
   explains engine analysis in plain language

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

Desktop (needs the platform's webview dev packages on Linux; nothing extra on macOS/Windows):

```sh
dx serve --platform desktop
```

Checks, same as CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p app --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
```

## Layout

```
crates/
  chess-core/   rules, Game (history + navigation), client/server protocol — no UI, no I/O
  app/          Dioxus client (web by default; desktop via --platform desktop)
docs/
  dioxus-0.7.md quick reference for the Dioxus version in use
```

## Licenses

Code is MIT (see `LICENSE`). Piece images in `crates/app/assets/pieces/cburnett` are by
Colin M.L. Burnett, licensed [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/).
