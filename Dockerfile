FROM rust:bookworm AS builder
WORKDIR /usr/src/factorion-bot
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt install -y openssl ca-certificates curl
WORKDIR /usr/factorion
COPY --from=builder /usr/local/cargo/bin/factorion-bot /usr/bin/factorion-bot
CMD ["./run.sh"]
