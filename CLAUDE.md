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
  PWA (manifest and icons in `web/static`); it never touches `/api`. Moves make a sound
  (`src/lib/sound/`): synthesised with Web Audio rather than played from files, so there is
  nothing to license or download — `voice.ts` builds every cue from a wooden knock and a
  tone, `cue.ts` reads what a move did out of its SAN, and `Sounds.attach` hooks
  `GameStore.onmove`, which only the `play` methods fire, so loading a PGN or stepping
  through history stays silent. Every board attaches, so play, drills, puzzles and online
  games all sound the same. The nav has the on/off switch (remembered per browser).
  When a game of yours becomes ready while you are in another tab, the site comes and finds
  you (`src/lib/notify/`): the `ready` cue, the tab's title, and — only when the tab is
  hidden and permission was given — a notification, shown by the service worker because on
  iOS an installed PWA has no other way, and clicking it goes to the board. Permission is
  asked for once, from the click that offers a game: that is the moment it is worth
  something, and a browser shows the prompt for nothing but a gesture. Nothing is pushed —
  the page is open and holding the lobby socket, so there is no push server, no VAPID keys
  and no subscriptions to store.
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
  `lobby.rs` the seek list
  (`/api/lobby/ws`: a socket that sends every open seek on connect and again whenever the
  list changes; one seek per connection, posted, cancelled or accepted by the messages in
  `protocol`). A seek is an offer, not a game: the game is created — with both seats filled
  and the colours drawn — only when someone accepts, and the accept removes the seek under
  the lock first, so two people clicking at once cannot both get it. Each side is told the
  game, the colour they drew and who they are playing, which is what the notification that
  fetches them back to the tab says. Seeks live in memory and
  **die with the connection that posted them**, deliberately: a seek whose author has closed
  their laptop is one nobody can play. Rated seeks refuse guests, as rated games do
  elsewhere), `oauth.rs` sign-in with Lichess/Google (`/api/auth/{provider}/start` → provider →
  `/callback`; `state` + PKCE verifier in a 10-minute cookie; `identities(provider, subject)`
  → `users`, with the provider's label for the account; started while signed in to an account it
  links instead, and refuses an identity that is someone else's; a built-in fake provider behind
  `--fake-oauth` for dev and tests, in-process, no network), `account.rs` the account's own
  sign-in methods (`/api/me/account` lists identities and whether there is a password,
  `DELETE /api/me/identities/{provider}/{subject}` refuses to remove the last way in,
  `PUT /api/me/password` sets or changes it — changing needs the current one — and signs out the
  other sessions), `email.rs` an optional email address and password resets
  (`PUT`/`DELETE /api/me/email`; the address stays pending until its mailed link is followed at
  `POST /api/auth/verify-email`, and only then is it `users.email`, so an unverified address can
  never get a reset; a change tells the old address; `POST /api/auth/forgot-password` answers
  `202` at once for any address and looks up and mails afterwards, so neither the reply nor its
  timing says who has an account; `POST /api/auth/reset-password` spends the link and signs every
  session out; links are 256-bit tokens stored as SHA-256 in `email_tokens`, deleted when used,
  verify 24 h, reset 1 h; mail rate-limited per recipient and per client), `mail.rs` the `Mailer`
  (SMTP through `lettre` with `--smtp-url`/`--mail-from`, or `--fake-mail` writing to the log
  and to `--fake-mail-dir`; without either there is no email at all, `GET /api/auth/mail` says
  so and the UI hides it), `Config` in `lib.rs` for the deployment flags (`--secure-cookies`,
  `--trust-proxy`, `--allowed-origins`, `--public-url`, the provider ids/secrets, the mailer), `rating.rs`
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

## Deploying

`Dockerfile` builds the whole site as one image (the WASM, the client build and the server
binary, in three stages); `compose.yaml` runs that image against a Postgres locally
(`docker compose up --build`, http://localhost:8080); `fly.toml` and
`.github/workflows/deploy.yml` deploy it: that workflow runs when CI on main *finishes*
green (`workflow_run`), not when the push lands, so a red commit never reaches the site,
and it stays inert until the repo has a `FLY_APP` variable. `docs/deploy.md` is the runbook — read it before changing any of that.
Two constraints there are load-bearing: **one instance only** (a game in progress lives in
that process's memory, written through to Postgres and reloaded on access, so two processes
would each hold their own copy), and nothing in front may strip COOP/COEP or the
multi-threaded engine silently stops running. Setting `CHESS_E2E_URL` points the Playwright
suite at a running deployment instead of one it starts itself; the tests needing
`--fake-oauth`, `--fake-coach` or `--fake-mail` are excluded from that, since a deployment
has none of them. Playwright's server writes its mail to `$TMPDIR/chess-e2e-mail`
(`web/e2e/mail.ts` reads the links back).

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
