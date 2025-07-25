FROM rust:bookworm AS builder
WORKDIR /usr/src/factorion-bot
COPY . .
RUN cd factorion-bot-reddit && cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt install -y openssl ca-certificates curl
WORKDIR /usr/factorion
COPY --from=builder /usr/local/cargo/bin/factorion-bot-reddit /usr/bin/factorion-bot
