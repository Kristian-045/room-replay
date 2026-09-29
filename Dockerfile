FROM node:22-bookworm-slim AS frontend
WORKDIR /build/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run check && npm run build

FROM rust:1.98-bookworm AS backend
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 568 dvr \
    && useradd --uid 568 --gid 568 --no-create-home dvr \
    && mkdir -p /data /app/web \
    && chown 568:568 /data
COPY --from=backend /build/target/release/cesnet-dvr /usr/local/bin/cesnet-dvr
COPY --from=frontend /build/web/dist/ /app/web/
ENV DVR_BIND=0.0.0.0:3000 DVR_DATA_DIR=/data DVR_WEB_DIR=/app/web RUST_LOG=info
USER 568:568
WORKDIR /app
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:3000/api/state > /dev/null || exit 1
CMD ["cesnet-dvr"]
