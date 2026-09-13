# web

The SvelteKit client. This replaces `crates/app` (Dioxus) step by step; see the migration
issue in the repo for the plan. Chess logic stays in `crates/chess-core`, reached through a
WASM wrapper (phase 1).

```sh
corepack enable            # once; makes the pinned pnpm available
pnpm install
pnpm browsers              # once; Chromium for component and end-to-end tests
pnpm dev                   # http://localhost:5173

pnpm lint && pnpm check && pnpm test && pnpm build   # what CI runs
```

Component tests (`*.svelte.spec.ts`) run in a real Chromium through Vitest browser mode;
plain unit tests (`*.spec.ts`) run in Node; end-to-end tests live in `e2e/` and run with
Playwright against the production build.
