# Deploying

The whole site is one container: the Rust server with the client build baked
in. `Dockerfile` builds it, `compose.yaml` runs it here, `fly.toml` and
`.github/workflows/deploy.yml` run it on Fly.

## Run the deployment locally first

This is the same image the host runs, against a Postgres, over plain http:

```sh
docker compose up --build        # http://localhost:8080
docker compose down              # add -v to throw the database away too
```

Worth doing before shipping anything, and when something behaves differently
deployed than it does under `pnpm dev`.

The end-to-end tests can be pointed at a running deployment instead of
building and starting their own server:

```sh
cd web
CHESS_E2E_URL=http://localhost:8080 corepack pnpm exec playwright test --project=desktop \
  e2e/accounts.e2e.ts e2e/online.e2e.ts e2e/ratings.e2e.ts e2e/board.e2e.ts \
  e2e/routes.e2e.ts e2e/pwa.e2e.ts
```

That covers what a deployment has to get right: two accounts sign up, play a
rated game and both ratings move; a guest keeps their games after signing up;
the PWA's manifest and icons resolve; the 404 fallback and clean URLs work.
The tests left out are the ones that need `--fake-oauth` or `--fake-coach`,
which a real deployment must never have.

Against the real site, use the public URL — and remember it writes to the
real database: those runs leave signed-up accounts and played games behind.

## One machine, on purpose

A game in progress lives in the server's memory: the room, the seats, the
count of open sockets per seat, and the broadcast channel that fans a move
out to everyone watching. It is written through to Postgres after every
change and loaded back on first access, so it survives a restart — but two
processes would each hold their own copy of the same game and neither would
see the other's moves.

**Run exactly one instance.** `fly.toml` says so (`min_machines_running = 1`,
no auto-stop, one machine). Scaling out means moving live game state out of
process memory first; it is not a config change.

Restarting is fine. Players reconnect, the socket's `Sync` catches them up,
and the away countdown gives a disconnected player time to come back.

## First deploy

Everything below is done once, by hand, by someone with the accounts.

1. **Create the app and its database.** `fly launch --no-deploy` (it will
   read `fly.toml`), then create a Postgres and attach it, which sets
   `DATABASE_URL` as a secret. Check what the provider currently offers for
   managed Postgres and what it costs before choosing — that choice is also
   the backup story, below.
2. **Set the name and URL in one place.** If the app is not `chess`, change
   `app`, `CHESS_PUBLIC_URL`, `CHESS_ALLOWED_ORIGINS` and
   `CHESS_LICHESS_CLIENT_ID` in `fly.toml` together. `CHESS_PUBLIC_URL` is
   what OAuth redirects are built from and `CHESS_ALLOWED_ORIGINS` is what
   game sockets are accepted from, so a stale value breaks sign-in or play
   without breaking the front page.
3. **Google sign-in.** Create an OAuth client in the Google Cloud console
   with `<public url>/api/auth/google/callback` as an authorised redirect
   URL, then:
   ```sh
   fly secrets set CHESS_GOOGLE_CLIENT_ID=... CHESS_GOOGLE_CLIENT_SECRET=...
   ```
   Lichess needs nothing: `CHESS_LICHESS_CLIENT_ID` in `fly.toml` is just a
   name shown on their consent screen.
4. **Deploy.** `fly deploy`. Migrations run as the machine starts, so a fresh
   database needs no separate step.
5. **Import the puzzles.** The 46 in `crates/server/tests/fixtures` are a
   test fixture, not a puzzle set. Download the real
   [Lichess puzzle database](https://database.lichess.org/#puzzles),
   decompress it, and run the import against the deployment's database — it
   is a few million rows, so run it from a machine with the file:
   ```sh
   fly proxy 5432 -a <postgres app> &
   DATABASE_URL=postgres://...@localhost:5432/chess \
     cargo run --release -p server -- import-puzzles lichess_db_puzzle.csv
   ```
   Importing again is safe; it updates what it has already seen.
6. **Deploy on merge.** Set the repository variables `FLY_APP` and
   `CHESS_PUBLIC_URL`, and the secret `FLY_API_TOKEN` (`fly tokens create
   deploy`). Until `FLY_APP` is set the deploy workflow does nothing, so
   nothing goes red before it is wanted.

## Check a deploy

- `https://…/healthz` says `ok`.
- The engine is multi-threaded: open the console on any page and check
  `crossOriginIsolated` is `true`. If it is `false` the proxy has dropped
  the COOP/COEP headers and every visitor silently gets the slow
  single-threaded Stockfish. The analysis page shows "N threads" when the
  threaded build is running.
- Sign in with Lichess and with Google both come back to the site signed in.
- Two browsers (two profiles — the session is a cookie) can play a rated
  game to the end and both ratings move.
- Assets come back precompressed: `curl -sI -H 'Accept-Encoding: br'
  <url>/_app/immutable/...` has `content-encoding: br`.

## Updating

Merging to main deploys. The service worker caches the shell per deploy, so
a tab that was open when the deploy landed keeps running the old version and
shows the update banner; the engine build is kept across deploys rather than
re-downloaded. Worth watching on the first real deploy that a tab open across
a deploy offers the banner and reloads cleanly.

## When a deploy is bad

Migrations apply on start and are not reversible, so "roll the database back"
is not the plan. Roll the *code* back — `fly releases` and
`fly deploy --image <previous>`, or revert the commit and let the workflow
deploy — and only ever add migrations that an older build can live with
(a new nullable column is fine; dropping one an older build still selects is
not). If a migration is the problem, fix forward with another migration.

## Backups

Accounts, games and ratings are real data the moment someone else plays.
Whatever the database provider offers, turn it on, and **restore it once into
a scratch database** — a backup nobody has restored is not a backup. Write
down here what was set up and when it was last tested, so the next person
doesn't have to guess.

## Secrets

`DATABASE_URL`, `CHESS_GOOGLE_CLIENT_SECRET` and (if the coach is ever turned
on) `CHESS_ANTHROPIC_API_KEY` come from the host's secret store. None of them
belong in `fly.toml`, the repo, or a shell history. `--fake-oauth` and
`--fake-coach` are for development and tests and must never be set on a
deployment: the fake OAuth provider signs anyone in as anyone.

The coach is off unless `CHESS_ANTHROPIC_API_KEY` is set, and costs nothing
while it is off. See #57 before turning it on.

## Another host

Nothing above is Fly-specific except `fly.toml` and the workflow. Any host
that runs a container and gives it a Postgres works; it needs to:

- run **one** instance (see above),
- pass WebSocket upgrades through to `/api/games/…`,
- **not strip `Cross-Origin-Opener-Policy` or `Cross-Origin-Embedder-Policy`**,
  or the multi-threaded engine silently stops running,
- set `X-Forwarded-For` (with `CHESS_TRUST_PROXY=1`), terminate TLS (with
  `CHESS_SECURE_COOKIES=1`), and have `CHESS_PUBLIC_URL` and
  `CHESS_ALLOWED_ORIGINS` match the public URL.

The image listens on `CHESS_BIND` (`0.0.0.0:8080` by default) and serves the
client from `CHESS_STATIC_DIR` (`/app/web/build`).
