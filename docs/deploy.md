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
  e2e/accounts.e2e.ts e2e/online.e2e.ts e2e/notify.e2e.ts e2e/ratings.e2e.ts \
  e2e/board.e2e.ts e2e/routes.e2e.ts e2e/pwa.e2e.ts
```

That covers what a deployment has to get right: two accounts sign up, play a
rated game and both ratings move; a guest keeps their games after signing up;
a seek taken while the poster is in another tab raises a notification from the
service worker; the PWA's manifest and icons resolve; the 404 fallback and clean URLs work.
The tests left out are the ones that need `--fake-oauth` or `--fake-coach`,
which a real deployment must never have.

A deployment also keeps the real rate limits (10 signups a minute per
address), which the local suite raises with `--rate-limit-scale 10`. The
subset above signs up 7 accounts, under that, but running it twice within a
minute, or adding files that sign up, can get "too many attempts": wait a
minute and rerun.

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

1. **Create the app and its database.** `fly apps create <name>`, then a
   Postgres, attached to the app (which sets `DATABASE_URL` as a secret).

   **The database is the whole bill.** Fly's *managed* Postgres (`fly mpg
   create`) starts at $38/month on its cheapest plan; its *unmanaged* one
   (`fly postgres create --vm-size shared-cpu-1x --volume-size 1
   --initial-cluster-size 1`) is a 256 MB machine and a 1 GB volume, which
   is small change or free depending on the organization's allowance. What
   is deployed today is the unmanaged one — see **Backups**, because that
   choice is also the backup story.
2. **Set the name and URL in one place.** If the app is not `chess`, change
   `app`, `CHESS_PUBLIC_URL`, `CHESS_ALLOWED_ORIGINS` and
   `CHESS_LICHESS_CLIENT_ID` in `fly.toml` together. `CHESS_PUBLIC_URL` is
   what OAuth redirects are built from and `CHESS_ALLOWED_ORIGINS` is what
   game sockets are accepted from, so a stale value breaks sign-in or play
   without breaking the front page.
3. **Google sign-in.** The server already speaks Google; it only needs a
   client id and secret, and shows the button when it has them (see
   **Sign in with Google** below). Lichess needs nothing:
   `CHESS_LICHESS_CLIENT_ID` in `fly.toml` is just a name shown on their
   consent screen.
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
   nothing goes red before it is wanted. Deploys follow a green CI on main —
   see **Updating**. Nothing stops a direct push to main from deploying
   itself: CI still has to pass first, but nobody reviewed it. Branch
   protection on `main` is the way to require a pull request, and this
   repository does not have it.

## Sign in with Google

Nothing in the code is missing: `oauth.rs` has the Google provider, the login
page shows a button for every provider the server is configured with, and
`GET /api/auth/providers` says which those are. What it needs is a client id
and secret from a Google Cloud project, which is a console click-through
someone with the Google account does by hand, once.

In the console (Google has renamed these pages more than once — the names
below are what the things *are*, not necessarily today's menu labels):

1. **A project.** Any project will do; a new one named for the site is
   tidiest.
2. **The consent screen** (now under "Google Auth Platform" → Branding and
   Audience). App name is what the sign-in page says the user is signing in
   to, so make it the site's name; the support and developer contact are the
   account's own email.
   - **Audience: External.** Internal only exists for Workspace
     organisations.
   - Leaving it in **Testing** means only the listed test users can sign in,
     so add anyone who should be able to. **Publishing** it lifts that, and
     needs no Google review as long as the scopes stay the three below — they
     are all non-sensitive. An unverified app shows an extra "Google hasn't
     verified this app" screen; that is cosmetic.
3. **Scopes:** `openid`, `.../auth/userinfo.email`,
   `.../auth/userinfo.profile`. The server asks for exactly these
   (`Provider::scope`) and reads `sub` and `email` from the userinfo
   endpoint. Do not add more: anything sensitive drags in verification.
4. **An OAuth client** of type **Web application**.
   - **Authorised redirect URI:** `<public url>/api/auth/google/callback` —
     for this deployment, `https://chess-oxideops.fly.dev/api/auth/google/callback`.
     It must match what the server sends character for character, and the
     server builds it from `CHESS_PUBLIC_URL`, so those two move together.
     Google rejects a mismatch with `redirect_uri_mismatch` before the user
     sees anything.
   - Authorised JavaScript origins are for browser-side flows; this one is
     server-side, so leave them empty.
   - Add `http://localhost:8080/api/auth/google/callback` here too if the
     same client should work against `docker compose up` (Google allows
     plain http for `localhost` only).

Then hand the two values to the server. The secret belongs in Fly's secret
store and nowhere else — not in `fly.toml`, not in a shell history that gets
pasted somewhere:

```sh
fly secrets set --stage CHESS_GOOGLE_CLIENT_ID=... CHESS_GOOGLE_CLIENT_SECRET=... -a chess-oxideops
```

`--stage` holds them until the next deploy instead of restarting the machine
there and then, which matters because that machine is holding live games.
Drop `--stage` to apply immediately, and expect a restart.

To check it took, without signing in:

```sh
curl -s https://chess-oxideops.fly.dev/api/auth/providers
# [{"id":"lichess",...},{"id":"google","name":"Google"}]
```

If Google is missing the server did not get both values: `CHESS_GOOGLE_CLIENT_ID`
`requires` `CHESS_GOOGLE_CLIENT_SECRET` in the CLI, and one without the other
is refused at startup.

**One caveat to know before telling people to use it.** Signing in with
Google while signed out always means "the Google account's user", which is a
*different* user from the Lichess one, with its own rating and history — the
code only links a new provider to an existing account when you are already
signed in when you use it. Until there is a way to connect a second method
from a settings page, one person using both buttons ends up as two players.

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

Merging to main deploys — once CI on main is green, not before. The deploy
workflow is triggered by CI *finishing* (`workflow_run`), not by the push, so
a commit whose tests fail never reaches the site, and it checks out the exact
commit CI passed on. Expect roughly CI's five minutes and then the deploy's
three.

Two things follow from that. A red CI on main means no deploy and a quiet
failure: nothing goes wrong, the site simply stays where it was, so check the
Actions tab rather than assuming the deploy is slow. And if two merges land
close together, the deploys happen in whatever order the CI runs finish; they
cannot overlap (the `concurrency` group serialises them), but the second one
to finish is what ends up live. With one person merging this is theory.

`workflow_dispatch` still deploys whatever main is now, with no CI gate — for
getting the site back when that is what is needed.

The service worker caches the shell per deploy, so a tab that was open when
the deploy landed keeps running the old version and shows the update banner;
the engine build is kept across deploys rather than re-downloaded. Worth
watching on the first real deploy that a tab open across a deploy offers the
banner and reloads cleanly.

## When a deploy is bad

Migrations apply on start and are not reversible, so "roll the database back"
is not the plan. Roll the *code* back — `fly releases` and
`fly deploy --image <previous>`, or revert the commit and let the workflow
deploy — and only ever add migrations that an older build can live with
(a new nullable column is fine; dropping one an older build still selects is
not). If a migration is the problem, fix forward with another migration.

## Backups

**There are none right now.** The deployment uses Fly's unmanaged Postgres,
which Fly neither supports nor backs up: recovery is the operator's job.
That is a deliberate trade for a site nobody depends on yet, and it stops
being acceptable the moment someone else's games and rating live in there.

Before that day, either move to a managed Postgres (`fly mpg create`, then
attach it and `fly secrets unset DATABASE_URL` from the old one — the app
needs no change, only the secret), or take dumps on a schedule:

```sh
fly proxy 5432 -a chess-pg &
pg_dump "postgres://…@localhost:5432/chess_oxideops" > chess-$(date +%F).sql
```

Either way, **restore it once into a scratch database** — a backup nobody
has restored is not a backup. Write down here what was set up and when it
was last tested, so the next person doesn't have to guess.

## Secrets

`DATABASE_URL`, `CHESS_GOOGLE_CLIENT_SECRET` and (if the coach is ever turned
on) `CHESS_ANTHROPIC_API_KEY` come from the host's secret store. None of them
belong in `fly.toml`, the repo, or a shell history. `--fake-oauth` and
`--fake-coach` are for development and tests and must never be set on a
deployment: the fake OAuth provider signs anyone in as anyone. Nor should
`--rate-limit-scale` (`CHESS_RATE_LIMIT_SCALE`), which multiplies the signup,
login and guest limits for the end-to-end tests; leave it at its default of 1.

The coach is off unless `CHESS_ANTHROPIC_API_KEY` is set, and costs nothing
while it is off. See #57 before turning it on.

## Another host

Nothing above is Fly-specific except `fly.toml` and the workflow. Any host
that runs a container and gives it a Postgres works; it needs to:

- run **one** instance (see above),
- pass WebSocket upgrades through to `/api/games/…`,
- **not strip `Cross-Origin-Opener-Policy` or `Cross-Origin-Embedder-Policy`**,
  or the multi-threaded engine silently stops running,
- set `X-Forwarded-For` (with `CHESS_TRUST_PROXY=true`), terminate TLS (with
  `CHESS_SECURE_COOKIES=true`), and have `CHESS_PUBLIC_URL` and
  `CHESS_ALLOWED_ORIGINS` match the public URL.

The image listens on `CHESS_BIND` (`0.0.0.0:8080` by default) and serves the
client from `CHESS_STATIC_DIR` (`/app/web/build`).

Note that booleans are passed to the binary as `true`/`false`; `1` is
refused (`invalid value '1' for '--secure-cookies'`) and the container will
crash-loop on start.

## What it needs to run

One 256 MB machine is enough, which is what is deployed: the engine runs in
the visitor's browser, so this process only validates moves, keeps clocks
and relays messages between sockets. Idle WebSocket connections are cheap.
If it ever runs out of memory, `fly scale memory 512` is the first move —
but check the logs for an OOM first rather than assuming.
