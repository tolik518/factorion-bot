FROM rust:bookworm AS builder
WORKDIR /usr/src/factorion-bot
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt install -y openssl ca-certificates curl
WORKDIR /usr/factorion
COPY --from=builder /usr/local/cargo/bin/factorion-bot /usr/factorion/factorion-bot
COPY --from=builder /usr/src/factorion-bot/.env /usr/factorion/.env
COPY --from=builder /usr/src/factorion-bot/run.sh /usr/factorion/run.sh
RUN chmod +x /usr/factorion/run.sh
CMD ["./run.sh"]
