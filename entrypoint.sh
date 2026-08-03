#!/bin/bash
set -e

export TAWAI_HOME=${TAWAI_HOME:-/home/tawai}
export TAWAI_SERVER_HOST=${TAWAI_SERVER_HOST:-0.0.0.0}
export TAWAI_SERVER_PORT=${TAWAI_SERVER_PORT:-8080}
export TAWAI_SERVER_MASTER_KEY=${TAWAI_SERVER_MASTER_KEY:-}
export TAWAI_DATABASE_URL=${TAWAI_DATABASE_URL:-}

# Rust treats an empty (but set) env var as a value, so unset these so the
# server falls back to its defaults (SQLite DB path, optional AcoustID).
[ -z "$TAWAI_DATABASE_URL" ] && unset TAWAI_DATABASE_URL || true

CURRENT_UID=$(id -u)
CURRENT_GID=$(id -g)

if [ -z "$TAWAI_SERVER_MASTER_KEY" ]; then
    echo "ERROR: TAWAI_SERVER_MASTER_KEY is not set!" >&2
    echo "Generate one with: openssl rand -hex 32" >&2
    exit 1
fi

if ! echo "$TAWAI_SERVER_MASTER_KEY" | grep -Eq '^[a-fA-F0-9]{64}$'; then
    echo "ERROR: TAWAI_SERVER_MASTER_KEY must be exactly 64 hex characters!" >&2
    exit 1
fi

is_root() { [ "$CURRENT_UID" -eq 0 ]; }

RUN_AS=""
if is_root; then
    if [ -n "$PUID" ]; then
        GID=${PGID:-$PUID}
        addgroup -g "$GID" tawai 2>/dev/null || true
        adduser -u "$PUID" -G tawai -D -s /bin/bash tawai 2>/dev/null || true
        RUN_AS="su-exec tawai"
    elif [ -n "$PGID" ] && [ "$PGID" -ne 0 ]; then
        addgroup -g "$PGID" tawai 2>/dev/null || true
    fi
fi

if ! is_root; then
    echo "Running as non-root user (UID=$CURRENT_UID, GID=$CURRENT_GID)"
fi

mkdir -p "$TAWAI_HOME/config"
mkdir -p "$TAWAI_HOME/downloads"
mkdir -p "$TAWAI_HOME/logs"
mkdir -p "$TAWAI_HOME/data"
if is_root; then
    chown -R tawai:tawai "$TAWAI_HOME" || { echo "ERROR: Failed to chown $TAWAI_HOME" >&2; exit 1; }
    chown -R tawai:tawai /var/lib/nginx /var/log/nginx /var/cache/nginx /etc/nginx || { echo "ERROR: Failed to chown nginx directories" >&2; exit 1; }
fi

if [ -n "$TZ" ]; then
    ln -sf /usr/share/zoneinfo/"$TZ" /etc/localtime 2>/dev/null || true
fi

envsubst '${TAWAI_SERVER_PORT}' < /etc/nginx/nginx.conf.template > /etc/nginx/nginx.conf

cd /app

echo "Starting Tawai API server on ${TAWAI_SERVER_HOST}:${TAWAI_SERVER_PORT}..."
$RUN_AS /usr/local/bin/tawai-server &
SERVER_PID=$!

sleep 2

echo "Serving UI and Proxy on port 3000..."
$RUN_AS nginx -g "daemon off;" &
NGINX_PID=$!

echo "Tawai is ready!"
echo "URL: http://localhost:3000"

trap "kill $SERVER_PID $NGINX_PID 2>/dev/null" EXIT
wait
