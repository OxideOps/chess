# The whole site in one image: the Rust server, with the client build it
# serves baked in. Built in three stages so each half is cached separately.
#
# The build needs network: `fetch-engine.mjs` pulls the Stockfish builds from
# their GitHub release (checking pinned SHA-256s) rather than the repo.
#
#   docker build -t chess .
#   docker run --rm -p 8080:8080 -e DATABASE_URL=... chess
#
# See docs/deploy.md.

ARG RUST_VERSION=1.94.1
ARG NODE_VERSION=22
ARG DEBIAN=trixie

# The WASM the client calls into, built with the same toolchain as the server.
FROM rust:${RUST_VERSION}-${DEBIAN} AS wasm
ARG WASM_PACK_VERSION=0.13.1
WORKDIR /src
RUN rustup target add wasm32-unknown-unknown \
	&& cargo install wasm-pack --locked --version ${WASM_PACK_VERSION}
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
RUN wasm-pack build crates/chess-core-wasm --target web \
	--out-dir /wasm --out-name chess_core --no-pack --release

# The client. Vite is run directly: the `build` script would rebuild the WASM,
# which the stage above already did.
FROM node:${NODE_VERSION}-${DEBIAN}-slim AS web
WORKDIR /web
RUN corepack enable
COPY web/package.json web/pnpm-lock.yaml ./
RUN corepack pnpm install --frozen-lockfile
COPY web .
COPY --from=wasm /wasm src/lib/wasm
RUN node scripts/fetch-engine.mjs && corepack pnpm exec vite build

# The server. SQLX_OFFLINE makes `query!` check against the committed .sqlx
# cache, so the build needs no database.
FROM rust:${RUST_VERSION}-${DEBIAN} AS server
ENV SQLX_OFFLINE=true
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
COPY .sqlx .sqlx
RUN cargo build --release --locked -p server --bin chess-server

FROM debian:${DEBIAN}-slim
# TLS roots: the server calls Lichess, Google and (when it has a key) Anthropic.
RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates \
	&& rm -rf /var/lib/apt/lists/*
# Nothing here needs root.
RUN useradd --system --create-home --uid 10001 chess
WORKDIR /app
COPY --from=server /src/target/release/chess-server /usr/local/bin/chess-server
COPY --from=web /web/build web/build
USER chess
# Listen on every interface: the host's proxy reaches the container from outside.
ENV CHESS_BIND=0.0.0.0:8080 CHESS_STATIC_DIR=/app/web/build
EXPOSE 8080
ENTRYPOINT ["chess-server"]
