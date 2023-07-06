FROM rust
LABEL org.opencontainers.image.source=https://github.com/Event-Horizon-Technologies/chess
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y libjavascriptcoregtk-4.0-dev libgtk-3-dev libsoup2.4-dev libwebkit2gtk-4.0-dev
RUN rustup target add wasm32-unknown-unknown
RUN rustup component add rustfmt
RUN cargo install --locked trunk
