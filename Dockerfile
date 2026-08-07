# Stage 1: Build Flutter web
FROM ghcr.io/cirruslabs/flutter:latest AS flutter-build

WORKDIR /app

# Install Rust for rinf gen
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

COPY pubspec*.yaml ./
RUN flutter pub get

COPY lib ./lib
COPY native ./native
COPY web ./web
COPY assets ./assets
COPY analysis_options.yaml ./

RUN cargo install rinf_cli --version 8.10.0 && rinf gen
RUN flutter build web --wasm

# Bake the version into the assets so the status endpoint reports it
COPY pubspec.yaml ./assets/docs/pubspec.yaml

# Stage 2: Build Rust server
FROM rust:alpine AS rust-build

WORKDIR /app

RUN apk add --no-cache \
    pkgconf \
    openssl-dev \
    perl \
    make \
    curl

COPY Cargo.toml Cargo.lock ./
COPY native ./native
COPY assets ./assets

RUN cargo build --release -p tawai-server && \
    strip /app/target/release/tawai-server

# Stage 3: Final image
FROM alpine:latest

WORKDIR /app

RUN apk add --no-cache \
    bash \
    nginx \
    ca-certificates \
    gettext \
    su-exec \
    curl \
    gcompat \
    tzdata \
    ffmpeg

RUN addgroup -g 1000 tawai && \
    adduser -u 1000 -G tawai -D -s /bin/bash tawai && \
    mkdir -p /home/tawai/config /home/tawai/downloads /home/tawai/logs /home/tawai/data && \
    mkdir -p /var/lib/nginx /var/log/nginx /var/cache/nginx /etc/nginx && \
    chown -R tawai:tawai /home/tawai /var/lib/nginx /var/log/nginx /var/cache/nginx /etc/nginx

COPY --from=flutter-build --chown=tawai:tawai /app/build/web ./web
COPY --from=flutter-build --chown=tawai:tawai /app/assets ./assets
COPY --from=rust-build --chown=tawai:tawai /app/target/release/tawai-server /usr/local/bin/tawai-server

COPY --chown=tawai:tawai nginx.conf /etc/nginx/nginx.conf.template

COPY --chown=tawai:tawai entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

HEALTHCHECK --interval=60s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f -s http://localhost:8181/api/tawai/system/status || exit 1

    EXPOSE 3000 8181

USER tawai
ENTRYPOINT ["/entrypoint.sh"]
