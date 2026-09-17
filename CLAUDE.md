# Chess (v2)

A chess site for learning and playing. Rust for the rules and (soon) the server, SvelteKit for
the client. Web only: phones get the same site as a responsive PWA, and there is no desktop
build. Native shells are off the table unless a concrete reason (store listing) appears.
Successor to the archived `OxideOps/chess-v1`.

## Layout

- `crates/chess-core` — rules (via `shakmaty`), `Game` (a move tree with variations; its
  linear API walks the current line, so code that never branches sees a plain game), PGN
  reader with nested variations, UCI
  engine-output parsing, puzzle checking (`puzzle.rs`: judges the solver's move, any mate
  counts), the lesson drills and their win/loss rules (`lesson.rs`), board facts for the
  coach (`facts.rs`: pieces, material, attacked/defended pieces, pins, where each king can go,
  the pawn structure and how far each side's pieces are out, and what each move of a line does,
  in words — with the material after a capture and, after a check, what can legally reply), and the client↔server
  `protocol` types. No UI, no I/O, must compile
  for `wasm32`. Everything chess-related that both the client and the server need goes here,
  with tests.
- `crates/chess-core-wasm` — wasm-bindgen wrapper around `chess-core` for the client; the only
  crate that knows about JavaScript. Data in, data out, no chess logic: `Game::view()` returns
  one `GameView` snapshot per render. View and protocol types derive ts-rs behind the `ts`
  feature; `pnpm gen:types` writes them to `web/src/lib/generated` (committed, CI checks
  freshness). `pnpm build:wasm` runs wasm-pack into `web/src/lib/wasm` (gitignored).
- `web` — the SvelteKit client. pnpm via corepack, TS strict, plain CSS with the variables in
  `web/src/app.css`, which is the whole visual vocabulary: a warm charcoal palette with brass
  as the one accent (the board keeps its own green for the last move, the selected square and
  the arrows), IBM Plex Sans for the interface and IBM Plex Mono for everything the game
  wrote — moves, clocks, evaluations, ratings, FENs — a type scale, and the shared `.panel`,
  `.btn`/`.btn.primary` and `.page-header` classes. Components use those variables and never
  hard-code a colour or a size. `web/src/lib/components/Logo.svelte` is the mark (a rook in
  the board's right angles); `web/scripts/gen-icons.mjs` renders the app icons from it. Static SPA: `ssr = false`, every route prerendered as a shell plus a
  `404.html` fallback; the root layout awaits `initChess()` so pages use the WASM
  synchronously. Stockfish (GPL, `web/static/engine`, never linked in, fetched by
  `scripts/fetch-engine.mjs` with pinned SHA-256s rather than committed) runs in a Web Worker
  behind `src/lib/engine/analysis.svelte.ts`; the multi-threaded build when the page is
  cross-origin isolated, the single-threaded one otherwise (and as a fallback if the threaded
  one fails to start). Phones get the same site: `.board-page` in `app.css` sizes the board
  to the viewport, `--tap` sizes touch targets. `src/service-worker.ts` makes it an offline
  PWA (manifest and icons in `web/static`); it never touches `/api`.
- `crates/server` — axum binary (`chess-server`). Serves `web/build` with clean URLs for the
  prerendered pages, `404.html` as the fallback, precompressed assets, and the cross-origin
  isolation headers (COOP/COEP) that multi-threaded Stockfish needs. Hosts games:
  `room.rs` is the pure game state (rules via `chess-core`, clocks, draw offers, timeouts,
  the away countdown and abandonment; takes `now` as a parameter so it is unit-tested without
  a runtime; the registry counts each seat's open sockets and tells it who is present), `games.rs` the
  in-memory registry and the `/api/games` HTTP + WebSocket endpoints (the creator holds
  White, `join` fills the Black seat, sockets authenticate from the session cookie,
  `/api/me/games` lists the caller's games; the upgrade is refused for a foreign `Origin`,
  see `origin.rs`), `auth.rs` users + sessions (guests, argon2id passwords, DB sessions in an
  `HttpOnly` cookie; `CurrentUser`/`RequireUser`/`ClientIp` extractors; rate limits from
  `limit.rs` on login/signup/guest; an hourly sweep of expired sessions and idle guests),
  `oauth.rs` sign-in with Lichess/Google (`/api/auth/{provider}/start` → provider →
  `/callback`; `state` + PKCE verifier in a 10-minute cookie; `identities(provider, subject)`
  → `users`; a built-in fake provider behind `--fake-oauth` for dev and tests, in-process,
  no network), `Config` in `lib.rs` for the deployment flags (`--secure-cookies`,
  `--trust-proxy`, `--allowed-origins`, `--public-url`, the provider ids/secrets), `rating.rs`
  pure Glicko-2 (checked against Glickman's worked example), `players.rs` the profile endpoint,
  `puzzles.rs` the Lichess puzzle import (`chess-server import-puzzles`), next-puzzle and
  attempt endpoints (only the first try at a puzzle rates), `coach.rs` the coach (Claude
  Messages API over `reqwest`; `/api/coach/explain` for a position and `/api/coach/mistake`
  for a drill mistake, both with prompts built from the engine's lines in SAN and the board
  facts from `chess_core::facts`, so the model reads the board instead of picturing it;
  `Prompt::check` flags answers that mention moves or pieces the prompt never showed; a flagged
  answer gets one correction turn (the first answer echoed back unchanged, thinking blocks and
  all, then the problems) and the cleaner of the two is served; both are logged;
  `Prompt::parts` marks the moves an answer names with their path from the explained position
  (`Explanation.parts`, rendered clickable by the client); follow-up questions
  (`/api/coach/followup`, `Coach::follow_up`) carry on the answer's conversation, kept server-side
  in memory by thread id (`Explanation.thread`, owned by the user, 1 h, 5 questions, one at a
  time, lost on restart) so the model's turns can't be forged; a question naming a legal move
  the lines don't start with gets `FollowUpReply::Probe` back and the client sends Stockfish's
  line for it (`src/lib/coach/probe.ts`) before the model answers;
  `"fallbacks": "default"` re-runs a request Opus 5's cyber classifier wrongly declines (the
  word "exact" in a prompt set it off); per-user
  limit, answer cache, `--fake-coach` offline stand-in; tests run against a mock API;
  `cargo run -p server --example coach_eval` asks the real API about the fixed positions in
  `tests/fixtures/coach_eval.json` (29 of them: openings, tactics from the puzzle fixture,
  endgame studies, drill and game mistakes; `cargo run -p server --example coach_cases`
  rebuilds the file from a local Stockfish), checks the answers, and grades each one with a second
  model call against the same facts (accuracy/clarity/usefulness out of 5, every unsupported
  claim quoted; `CHESS_JUDGE_MODEL`, `--no-judge`). Runs are saved under `target/coach-eval/`
  and `--compare <run.json>` puts an earlier one beside it: that is how a prompt change is
  judged, so run it before and after one),
  `db.rs` the
  Postgres layer (sqlx 0.9, `query!` macros checked against the committed `.sqlx` offline
  cache, so builds need no database; one row per game holding the room's `Snapshot`;
  migrations embedded and applied on start). After changing SQL or migrations run
  `cargo sqlx prepare --workspace -D $TEST_DATABASE_URL` (migrate first) and commit `.sqlx`. The
  registry writes through after every change and loads games not in memory on first access;
  when a rated game ends it calls `Db::apply_ratings`, which updates both players once (a flag
  on the game row, set in the same transaction, makes repeats no-ops);
  `DATABASE_URL` is optional. Static tests use `tower::ServiceExt::oneshot`; socket tests run
  a real listener with `tokio-tungstenite`; persistence tests need `TEST_DATABASE_URL`
  (local: `postgres://$USER@localhost/chess_test`; CI runs a postgres service) and skip without it;
  CI also curls the real binary.

## Working on the UI

**Read the `svelte-ui` skill before adding or editing anything in `web/`.** The short version:

- Keep chess logic out of components. They read `GameStore.view`
  (`web/src/lib/chess/game.svelte.ts`) and never decide what is legal. If the UI needs a new
  fact, add it to `chess_core::Game` with a test, expose it in `chess-core-wasm`, run
  `pnpm gen:types` and `pnpm build:wasm`.
- `pnpm dev` from `web/` builds the WASM then serves with hot reload. `pnpm` is reached
  through `corepack pnpm` on machines where it isn't on PATH.

## Skills

Project skills live in `.claude/skills/`: `run-app` (start/verify/stop the client),
`svelte-ui` (conventions and gotchas for `web/`), `check` (local CI). Add a skill when a
workflow has non-obvious steps we'd otherwise rediscover; keep one-line facts in this file
instead.

## Conventions

- Toolchain is pinned in `rust-toolchain.toml`; Node in `web/.nvmrc`; pnpm in
  `web/package.json` (`packageManager`). Never fork dependencies.
- CI runs `cargo fmt --check`, `cargo clippy -D warnings` (native, wasm32, and with the `ts`
  feature), `cargo test`, and for `web/`: binding freshness, `pnpm lint`, `check`,
  `test:unit`, `build`, `test:e2e`. Run the same locally before opening a PR (the `check`
  skill does all of it).
- PRs that finish an issue say `Closes #N` in the description.
- Piece images are Colin M.L. Burnett's `cburnett` set (CC BY-SA 3.0). Any new assets need a
  license compatible with MIT noted in `README.md`. Stockfish.js is GPLv3 and stays a separate
  worker program.
