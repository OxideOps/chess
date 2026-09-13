# web

The SvelteKit client. This replaces `crates/app` (Dioxus) step by step; see the migration
issue in the repo for the plan. Chess logic stays in `crates/chess-core`, reached through a
WASM wrapper (phase 1).

```sh
corepack enable            # once; makes the pinned pnpm available
cargo install wasm-pack --locked   # once; builds chess-core to WASM
pnpm install
pnpm browsers              # once; Chromium for component and end-to-end tests
pnpm dev                   # builds the WASM (dev profile), then http://localhost:5173

pnpm gen:types             # after changing protocol or view types in Rust (commit the output)
pnpm build:wasm            # after changing chess-core or the wrapper (dev/test don't do it for you)
pnpm lint && pnpm check && pnpm test && pnpm build   # what CI runs
```

`src/lib/chess/` is the only place that touches the WASM: `wasm.ts` loads it and types the
helpers, `game.svelte.ts` wraps a game as reactive state. Components read `GameStore.view`.

Component tests (`*.svelte.spec.ts`) run in a real Chromium through Vitest browser mode;
plain unit tests (`*.spec.ts`) run in Node; end-to-end tests live in `e2e/` and run with
Playwright against the production build.
