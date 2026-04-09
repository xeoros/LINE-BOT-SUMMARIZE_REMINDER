FROM rust:1.88-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY sql ./sql
COPY config ./config
COPY admin ./admin

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/line_bot_summarize /usr/local/bin/line_bot_summarize
COPY config ./config
COPY admin ./admin

ENV PORT=8080
ENV LOG_DIR=/tmp/logs

EXPOSE 8080

CMD ["line_bot_summarize"]
