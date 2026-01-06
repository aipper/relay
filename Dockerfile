# syntax=docker/dockerfile:1

FROM rust:1 AS builder
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY relay-cli/Cargo.toml relay-cli/Cargo.toml
COPY relay-cli/src relay-cli/src
COPY hostd/Cargo.toml hostd/Cargo.toml
COPY hostd/src hostd/src
COPY protocol/Cargo.toml protocol/Cargo.toml
COPY protocol/src protocol/src
COPY server/Cargo.toml server/Cargo.toml
COPY server/src server/src

RUN cargo build --release -p relay-server

FROM oven/bun:1 AS web-builder
WORKDIR /web
COPY web/package.json web/bun.lock web/index.html web/tsconfig.json web/vite.config.ts ./
COPY web/src ./src
RUN bun install --frozen-lockfile
RUN bun run build

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /src/target/release/relay-server /app/relay-server
COPY --from=web-builder /web/dist /app/web/dist
COPY docker/entrypoint.sh /entrypoint.sh

RUN chmod +x /entrypoint.sh && \
    useradd -r -u 10001 -g nogroup relay && \
    mkdir -p /data /app/web/dist && \
    chown -R relay:nogroup /data /app

ENV DATABASE_URL="sqlite:/data/server.db"
ENV BIND_ADDR="0.0.0.0:8787"
ENV WEB_DIST_DIR="/app/web/dist"

VOLUME ["/data"]
EXPOSE 8787

USER relay
ENTRYPOINT ["/entrypoint.sh"]
