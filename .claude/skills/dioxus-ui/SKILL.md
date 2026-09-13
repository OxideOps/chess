---
name: dioxus-ui
description: How to write or change UI in crates/app with Dioxus 0.7 — required reading, project conventions, and the compile-time gotchas already hit in this repo. Use before adding or editing any component, view, asset, or CSS in crates/app.
---

# Dioxus 0.7 UI in this repo

## Before writing code

1. Read `docs/dioxus-0.7.md`. Dioxus 0.7 replaced the whole API (`cx`, `Scope`, `use_state`,
   `use_shared_state`, `render!`, `cx.render` are gone; signals, `asset!`, `document::*` are
   in). Memory of older Dioxus is actively misleading.
2. If you need an idiom the doc doesn't cover, the source of truth is the official template
   for this version, not the internet:
   `git clone --depth 1 -b v0.7 https://github.com/DioxusLabs/dioxus-template` (read the
   `Jumpstart/` and `Workspace/` sub-templates). `dx new` itself needs an interactive TTY and
   hangs otherwise, so don't script it.
3. Check the installed CLI matches `Cargo.toml` (`dx --version` vs the `dioxus` version).

## Conventions

- Chess logic lives in `chess-core`. If a component needs a new fact about the game, add a
  tested method to `chess_core::Game` and call it; don't reimplement rules in the UI.
- Components take `Signal<Game>` (it's `Copy`, cheap to pass) and plain `Copy` values like
  `Color` for props. Views compose components; components don't know about routes.
- Precompute per-render data into plain structs (see `SquareView` in
  `src/components/board.rs`) and iterate those in `rsx!`, instead of doing lookups inside the
  markup. Drop any `.read()` guard before building the `rsx!`.
- One stylesheet per component, loaded with `document::Stylesheet { href: asset!(...) }` at the
  top of that component's `rsx!`. Colors are CSS variables defined in `assets/main.css`.
- Multiple `class:` attributes on one element merge; use `class: if cond { "x" }` for
  conditional classes rather than string formatting.
- Piece images: `piece_asset(piece)` in `board.rs`. `asset!` needs a string literal per file,
  so new asset sets need their own match arm list.

## Gotchas that already cost a compile cycle

- A helper closure that reads a signal (`game.read()`) must be `move`, or it borrows `game`
  and blocks the later `game.write()` in the same handler (E0502). Signals are `Copy`, so
  `move` is free.
- To mutate a signal from inside a nested closure, rebind it first: `let mut game = game;`.
- Log with `dioxus::logger::tracing::{info, warn}`; `tracing` is not a direct dependency.
- `shakmaty::UciMove` and `Move` are `Copy` — don't `.clone()` them (clippy denies it).
- Never hold a `.read()`/`.write()` guard across an `await`; `crates/app/clippy.toml` flags it.
- A `Result<&T, E>` from a `game.write()` call borrows the guard; map it to an owned value
  (`.map(|_| ())`) in the same statement.

## Verify

```sh
cargo clippy -p app --target wasm32-unknown-unknown -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings
cd crates/app && dx build
```

Then run it and click through the change (see the `run-app` skill). Compiling is not proof
that a Dioxus component renders or that an event handler fires.
