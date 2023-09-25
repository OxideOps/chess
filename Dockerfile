#Dockerfile for setting up a docker for our CI
FROM catthehacker/ubuntu:rust-latest-dev
LABEL org.opencontainers.image.source=https://github.com/OxideOps/oxide-chess
WORKDIR /app
ENV SERVER_FN_OVERRIDE_KEY=y
COPY setup.sh .
RUN ./setup.sh --docker
ENV PATH="/root/.cargo/bin:/root/.nvm/versions/node/v20.7.0/bin:${PATH}"
