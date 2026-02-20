FROM rust:bookworm AS builder
WORKDIR /usr/src/factorion-bot
COPY . .
RUN cd factorion-bot-reddit && cargo install --path .

FROM debian:bookworm-slim
ARG VERSION=latest

LABEL org.opencontainers.image.version="${VERSION}"
LABEL org.opencontainers.image.title="factorion-bot"
LABEL org.opencontainers.image.description="A Reddit bot that calculates factorials"
LABEL org.opencontainers.image.url="https://github.com/tolik518/factorion-bot"
LABEL org.opencontainers.image.source="https://github.com/tolik518/factorion-bot"
LABEL org.opencontainers.image.licenses="MIT"

RUN apt-get update && apt install -y openssl ca-certificates curl
WORKDIR /usr/factorion
COPY --from=builder /usr/local/cargo/bin/factorion-bot-reddit /usr/bin/factorion-bot
