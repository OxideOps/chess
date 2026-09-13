# Chess (v2)

A chess site for learning and playing, built in Rust. Web only (Dioxus → WASM): phones get the
same site as a responsive PWA, and there is no desktop build. Native shells are off the table
unless a concrete reason (store listing) appears. Successor to the archived `OxideOps/chess-v1`.

## Layout

- `crates/chess-core` — rules (via `shakmaty`), `Game` history/navigation, and the client↔server
  `protocol` types. No UI, no I/O, must compile for `wasm32`. Everything chess-related that both
  the client and the server need goes here, with tests.
- `crates/app` — the Dioxus web client. It also type-checks on the host so `cargo clippy
  --workspace` and `cargo test --workspace` work, but it only ships as WASM. `src/engine/` runs
  Stockfish (a GPL Web Worker vendored in `assets/engine/`, never linked in) and exposes it as
  the `use_analysis` hook; the UCI text protocol itself is parsed in `chess_core::engine`.
- (planned) `crates/server` — axum + sqlx + Postgres. Owns games, clocks, and move validation.

## Working on the UI

Dioxus 0.7 is very different from older versions (`cx`, `Scope`, `use_state`, `use_shared_state`
are gone). **Read `docs/dioxus-0.7.md` before writing or editing components.** Don't rely on
memory of older Dioxus APIs.

- Run the client with `dx serve` from `crates/app` (hot reloads).
- Never hold a signal `.read()`/`.write()` guard across an `await` or while calling another
  signal write (see `crates/app/clippy.toml`).
- Keep chess logic out of components: if a component needs a new fact about the game, add a
  method to `chess_core::Game` (with a test) and call it.

## Skills

Project skills live in `.claude/skills/`: `run-app` (start/verify/stop the client), `dioxus-ui`
(read before touching `crates/app`), `check` (local CI). Add a skill when a workflow has
non-obvious steps we'd otherwise rediscover; keep one-line facts in this file instead.

## Conventions

- Toolchain is pinned in `rust-toolchain.toml`. Dioxus is pinned to an exact minor; upgrade it
  deliberately, on its own PR. Never fork dependencies.
- CI runs `cargo fmt --check`, `cargo clippy -D warnings` (native and wasm), `cargo test`, and
  `dx build`. Run the same locally before opening a PR.
- Piece images are Colin M.L. Burnett's `cburnett` set (CC BY-SA 3.0). Any new assets need a
  license compatible with MIT noted in `README.md`.
