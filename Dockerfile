#Dockerfile for setting up a docker for our CI

FROM catthehacker/ubuntu:rust-latest
LABEL org.opencontainers.image.source=https://github.com/OxideOps/chess
WORKDIR /app
ENV SERVER_FN_OVERRIDE_KEY=y
COPY setup.sh .
RUN ./setup.sh --docker
